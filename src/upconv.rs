use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use std::collections::HashMap;
use std::sync::Arc;

use crate::fdl::Fdl;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    input_buff: Vec<f32>,
    input_fft_buff: Vec<f32>,
    output_buff: Vec<f32>,
    output_fft_buff: Vec<f32>,
    fdls: HashMap<&'static str, Fdl>,
    accumulation_buffer: Vec<Complex<f32>>,
    new_spectrum_buff: Vec<Complex<f32>>,
    block_size: usize,
    channels: usize,
    num_blocks: usize,
}

impl UPConv {
    pub fn new(
        block_size: usize,
        starting_filter: Option<&[Complex<f32>]>,
        channels: usize,
        num_blocks: usize,
    ) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_fft_buff = fft.make_input_vec();
        let output_fft_buff = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let new_spectrum_buff = fft.make_output_vec();

        let input_buff = vec![0.0; block_size * 2 * channels];
        let output_buff = vec![0.0; block_size * channels];

        let filter = Fdl::new(starting_filter, block_size, num_blocks, channels);
        let signal = Fdl::new(None, block_size, num_blocks, channels);

        let fdls = HashMap::from([("signal", filter), ("signal", signal)]);

        Self {
            fft,
            ifft,
            block_size,
            input_buff,
            input_fft_buff,
            output_buff,
            output_fft_buff,
            fdls,
            accumulation_buffer,
            new_spectrum_buff,
            channels,
            num_blocks,
        }
    }

    pub fn set_fdl_buff(&mut self, new_buff: &[Complex<f32>], fdl_key: &'static str) {
        // TODO maybe put channels here
        self.fdls.get_mut(fdl_key).unwrap().set_buffer(new_buff);
    }

    // filter chunk should be p.0 * channels
    pub fn push_filter_chunk<'push_filter_chunk>(
        &mut self,
        filter_chunk: impl Iterator<Item = &'push_filter_chunk [f32]>,
    ) {
        // TODO kinda feel like this maybe should be somewhere else
        for (chunk_channel, channel) in filter_chunk.zip(0..self.channels) {
            self.input_fft_buff.fill(0.0);
            self.input_fft_buff[0..self.block_size].copy_from_slice(chunk_channel);
            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });

            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            self.filter.push_block(&self.new_spectrum_buff, channel);
        }
    }

    /// block is a slice of channel slices, as opposed to a slice of sample slices,
    /// so there will be one block size slice of samples per channel in block
    pub fn process_block<'process_block>(
        &mut self,
        channel_blocks: impl Iterator<Item = &'process_block [f32]>,
    ) -> &[f32] {
        // move the inputs over by one block and add the new block on the end
        // iterate over everything by channel
        for (((in_channel, out_channel), block_channel), channel) in self
            .input_buff
            .chunks_exact_mut(self.block_size * 2)
            .zip(self.output_buff.chunks_exact_mut(self.block_size))
            .zip(channel_blocks)
            .zip(0..self.channels)
        {
            in_channel.copy_within(self.block_size..self.block_size * 2, 0);
            in_channel[self.block_size..self.block_size * 2].copy_from_slice(block_channel);

            self.input_fft_buff.copy_from_slice(&in_channel);
            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            self.signal.push_block(&self.new_spectrum_buff, channel);

            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
            self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });
            self.output_fft_buff.fill(0.0);

            self.signal
                .mul_blocks(&self.filter, &mut self.accumulation_buffer, channel);

            self.ifft
                .process_with_scratch(
                    &mut self.accumulation_buffer,
                    &mut self.output_fft_buff,
                    &mut [],
                )
                .unwrap();

            out_channel
                .copy_from_slice(&self.output_fft_buff[self.block_size..self.block_size * 2]);
        }

        &self.output_buff
    }
}
