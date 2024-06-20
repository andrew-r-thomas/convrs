use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use std::collections::HashMap;
use std::sync::Arc;

use crate::fdl::Fdl;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
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
        channels: usize,
        num_blocks: usize,
        fdl_keys: &[&'static str],
    ) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_fft_buff = fft.make_input_vec();
        let output_fft_buff = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let new_spectrum_buff = fft.make_output_vec();

        let output_buff = vec![0.0; block_size * channels];

        let mut fdls = HashMap::new();
        for fdl_key in fdl_keys {
            fdls.insert(*fdl_key, Fdl::new(block_size, num_blocks, channels));
        }

        Self {
            fft,
            ifft,
            block_size,
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

    pub fn push_chunk<'push_chunk>(
        &mut self,
        fdl_key: &'static str,
        chunk: impl Iterator<Item = &'push_chunk [f32]>,
        sliding: bool,
    ) {
        let fdl = self.fdls.get_mut(fdl_key).unwrap();
        for (chunk_channel, channel) in chunk.zip(0..self.channels) {
            fdl.push_block(
                chunk_channel,
                &self.fft,
                &mut self.input_fft_buff,
                sliding,
                channel,
            );
        }
    }

    pub fn process(&mut self, fdl_key_1: &'static str, fdl_key_2: &'static str) -> &[f32] {
        let fdl_1 = self.fdls.get(fdl_key_1).unwrap();
        let fdl_2 = self.fdls.get(fdl_key_2).unwrap();

        for (out_channel, channel) in self
            .output_buff
            .chunks_exact_mut(self.block_size)
            .zip(0..self.channels)
        {
            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
            self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });
            self.output_fft_buff.fill(0.0);

            fdl_1.mul_blocks(fdl_2, &mut self.accumulation_buffer, channel);

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
