use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use std::sync::Arc;

use crate::envelopes::{linear_envelope, FadeDirection};

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    input_buffs: Vec<Vec<f32>>,
    input_fft_buff: Vec<f32>,
    output_buffs: Vec<Vec<f32>>,
    output_fft_buff: Vec<f32>,
    block_size: usize,
    filter: Vec<Vec<Complex<f32>>>,
    // TODO lol this type just makes me not feel very good about life
    fdls: Vec<Vec<Vec<Complex<f32>>>>,
    accumulation_buffer: Vec<Complex<f32>>,
    new_spectrum_buff: Vec<Complex<f32>>,
    channels: usize,
    needs_fade: bool,
    old_filter: Vec<Vec<Complex<f32>>>,
}

impl UPConv {
    pub fn new(
        block_size: usize,
        max_filter_size: usize,
        starting_filter: &[f32],
        channels: usize,
    ) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let mut input_fft_buff = fft.make_input_vec();
        let output_fft_buff = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let mut new_spectrum_buff = fft.make_output_vec();

        let input_buffs = vec![vec![0.0; block_size * 2]; channels];
        let output_buffs = vec![vec![0.0; block_size]; channels];

        let p = max_filter_size.div_ceil(block_size);
        let mut filter = vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; p];

        let fdls = vec![vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; p]; channels];

        let filter_iter = starting_filter.chunks_exact(block_size);
        for (chunk, filter_buff) in filter_iter.zip(&mut filter) {
            input_fft_buff.fill(0.0);
            input_fft_buff[0..chunk.len()].copy_from_slice(chunk);

            fft.process_with_scratch(&mut input_fft_buff, &mut new_spectrum_buff, &mut [])
                .unwrap();

            filter_buff.copy_from_slice(&new_spectrum_buff);
            new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
        }

        Self {
            fft,
            ifft,
            block_size,
            input_buffs,
            input_fft_buff,
            output_buffs,
            output_fft_buff,
            filter,
            fdls,
            accumulation_buffer,
            new_spectrum_buff,
            channels,
            needs_fade: false,
            old_filter: vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; p],
        }
    }

    pub fn update_filter(&mut self, new_filter: &[f32]) {
        let filter_iter = new_filter.chunks_exact(self.block_size);

        self.old_filter
            .iter_mut()
            .for_each(|o| o.fill(Complex { re: 0.0, im: 0.0 }));

        for (filter, old) in self.filter.iter().zip(&mut self.old_filter) {
            old.copy_from_slice(filter);
        }

        self.filter
            .iter_mut()
            .for_each(|n| n.fill(Complex { re: 0.0, im: 0.0 }));

        for (chunk, filter_buff) in filter_iter.zip(&mut self.filter) {
            self.input_fft_buff.fill(0.0);
            self.input_fft_buff[0..chunk.len()].copy_from_slice(chunk);

            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            filter_buff.copy_from_slice(&self.new_spectrum_buff);
            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
        }

        self.needs_fade = true;
    }

    /// block is a slice of channel slices, as opposed to a slice of sample slices,
    /// so there will be one block size slice of samples per channel in block
    pub fn process_block<'blocks>(
        &mut self,
        channel_blocks: impl IntoIterator<Item = &'blocks [f32]>,
    ) -> impl IntoIterator<Item = &[f32]> {
        let mut blocks = channel_blocks.into_iter();
        // move the inputs over by one block and add the new block on the end
        for i in 0..self.channels {
            let buff = &mut self.input_buffs[i];
            let block = blocks.next().unwrap();
            let fdl = &mut self.fdls[i];
            assert!(self.filter.len() == fdl.len());
            let out = &mut self.output_buffs[i];

            buff.copy_within(self.block_size..self.block_size * 2, 0);
            buff[self.block_size..self.block_size * 2].copy_from_slice(block);
            self.input_fft_buff[0..self.block_size * 2]
                .copy_from_slice(&buff[0..self.block_size * 2]);

            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            let fdl_len = fdl.len();
            fdl.get_mut(fdl_len - 1)
                .unwrap()
                .copy_from_slice(&self.new_spectrum_buff);
            fdl.rotate_right(1);

            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
            self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });

            for (filter_block, fdl_block) in self.filter.iter().zip(&*fdl) {
                for i in 0..self.block_size + 1 {
                    self.accumulation_buffer[i] += filter_block[i] * fdl_block[i];
                }
            }

            self.ifft
                .process_with_scratch(
                    &mut self.accumulation_buffer,
                    &mut self.output_fft_buff,
                    &mut [],
                )
                .unwrap();

            out.copy_from_slice(&self.output_fft_buff[self.block_size..self.block_size * 2]);
            self.output_fft_buff.fill(0.0);

            if self.needs_fade {
                for (filter_block, fdl_block) in self.old_filter.iter().zip(&*fdl) {
                    self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });
                    for i in 0..self.block_size + 1 {
                        self.accumulation_buffer[i] += filter_block[i] * fdl_block[i];
                    }
                }

                self.ifft
                    .process_with_scratch(
                        &mut self.accumulation_buffer,
                        &mut self.output_fft_buff,
                        &mut [],
                    )
                    .unwrap();

                let mut j = 0;
                for (i, o) in out
                    .iter_mut()
                    .zip(&self.output_fft_buff[self.block_size..self.block_size * 2])
                {
                    *i *= linear_envelope(j, self.block_size, FadeDirection::FadeIn);
                    *i += o * linear_envelope(j, self.block_size, FadeDirection::FadeOut);
                    j += 1;
                }

                self.needs_fade = false;
            }
        }

        self.output_buffs.iter().map(|o| o.as_slice())
    }
}
