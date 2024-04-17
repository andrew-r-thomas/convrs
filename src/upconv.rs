use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use std::sync::Arc;

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
    p: usize,
}

impl<'blocks> UPConv {
    pub fn new(block_size: usize, max_filter_size: usize, channels: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_fft_buff = fft.make_input_vec();
        let output_fft_buff = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let new_spectrum_buff = fft.make_output_vec();

        let mut input_buffs = vec![];
        for _ in 0..channels {
            input_buffs.push(fft.make_input_vec());
        }

        let output_buffs = vec![vec![0.0; block_size]; channels];

        let p = max_filter_size.div_ceil(block_size);
        let filter = Vec::with_capacity(p);

        let fdls = vec![vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; p]; channels];

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
            p,
        }
    }

    // TODO
    // need to figure out how to make sure this is either real time safe
    // or not real time safe and clearly stated, or sectioned off
    pub fn set_filter(&mut self, new_filter: &[f32]) {
        let filter_iter = new_filter.chunks_exact(self.block_size);
        // TODO see if we can do this in a real time safe way
        let mut real_vec = self.fft.make_input_vec();
        let mut comp_vec = self.fft.make_output_vec();

        self.filter.clear();

        for chunk in filter_iter {
            real_vec.fill(0.0);
            real_vec[0..chunk.len()].copy_from_slice(chunk);

            self.fft
                .process_with_scratch(&mut real_vec, &mut comp_vec, &mut [])
                .unwrap();

            let mut out = Vec::with_capacity(self.block_size + 1);
            out.clone_from(&comp_vec);

            self.filter.push(out);
        }
        let filter_len = self.filter.len();

        self.filter.extend(vec![
            vec![Complex { re: 0.0, im: 0.0 }; self.block_size + 1];
            self.p - filter_len
        ]);
    }

    /// block is a slice of channel slices, as opposed to a slice of sample slices,
    /// so there will be one block size slice of samples per channel in block
    pub fn process_block(
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

            for (filter_block, fdl_block) in self.filter.iter().zip(fdl) {
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
        }

        self.output_buffs.iter().map(|o| o.as_slice())
    }
}
