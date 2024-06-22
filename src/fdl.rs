use std::sync::Arc;

use realfft::{num_complex::Complex, RealToComplex};

// TODO i would really love const generics here
pub struct Fdl {
    buffer: Vec<Complex<f32>>,
    input_buff: Vec<f32>,
    block_size: usize,
    num_blocks: usize,
    channels: usize,
}

impl Fdl {
    pub fn new(block_size: usize, num_blocks: usize, channels: usize) -> Self {
        let buffer = vec![Complex { re: 0.0, im: 0.0 }; (block_size + 1) * num_blocks * channels];
        let input_buff = vec![0.0; block_size * 2 * channels];

        Self {
            buffer,
            input_buff,
            block_size,
            num_blocks,
            channels,
        }
    }

    pub fn set_buffer(&mut self, new_buffer: &[Complex<f32>]) {
        self.buffer.copy_from_slice(new_buffer);
    }

    pub fn push_block(
        &mut self,
        block: &[f32],
        fft: &Arc<dyn RealToComplex<f32>>,
        fft_in_buff: &mut [f32],
        sliding: bool,
        // top: bool,
        channel: usize,
    ) {
        let in_start = self.block_size * 2 * channel;
        let in_end = in_start + (self.block_size * 2);
        let buff_start = (self.block_size + 1) * self.num_blocks * channel;
        let buff_end = buff_start + ((self.block_size + 1) * self.num_blocks);

        let channel_buff = &mut self.buffer[buff_start..buff_end];
        let channel_in = &mut self.input_buff[in_start..in_end];

        match sliding {
            true => {
                channel_in.copy_within(self.block_size..self.block_size * 2, 0);
                channel_in[self.block_size..self.block_size * 2].copy_from_slice(block);
            }
            false => {
                channel_in.fill(0.0);
                channel_in[0..self.block_size].copy_from_slice(block)
            }
        }

        channel_buff.copy_within(
            0..channel_buff.len() - (self.block_size + 1),
            self.block_size + 1,
        );

        fft_in_buff.copy_from_slice(&channel_in);

        // let out = match top {
        //     true => {}
        //     false => {
        //         &mut channel_buff[0..self.block_size + 1]
        //     }
        // }

        fft.process_with_scratch(
            fft_in_buff,
            &mut channel_buff[0..self.block_size + 1],
            &mut [],
        )
        .unwrap();
    }

    // TODO simd
    pub fn mul_blocks(&self, other: &Self, accum_buff: &mut [Complex<f32>], channel: usize) {
        let start = (self.block_size + 1) * self.num_blocks * channel;
        let end = start + ((self.block_size + 1) * self.num_blocks);
        let self_channel = &self.buffer[start..end];
        let other_channel = other.get_channel(channel);

        for (self_block, other_block) in self_channel
            .chunks_exact(self.block_size + 1)
            .zip(other_channel.chunks_exact(self.block_size + 1))
        {
            for ((self_sample, other_sample), accum_sample) in
                // TODO i dont like &mut *
                self_block.iter().zip(other_block).zip(&mut *accum_buff)
            {
                *accum_sample += self_sample * other_sample;
            }
        }
    }

    pub fn get_channel(&self, channel: usize) -> &[Complex<f32>] {
        let start = (self.block_size + 1) * self.num_blocks * channel;
        let end = start + ((self.block_size + 1) * self.num_blocks);

        &self.buffer[start..end]
    }
}
