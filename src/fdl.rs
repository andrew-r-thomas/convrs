use realfft::num_complex::Complex;

pub struct Fdl {
    buffer: Vec<Complex<f32>>,
    block_size: usize,
    num_blocks: usize,
    channels: usize,
}

impl Fdl {
    pub fn new(
        starting_buffer: Option<&[Complex<f32>]>,
        block_size: usize,
        num_blocks: usize,
        channels: usize,
    ) -> Self {
        let buffer = match starting_buffer {
            Some(b) => Vec::from(b),
            None => vec![Complex { re: 0.0, im: 0.0 }; (block_size + 1) * num_blocks * channels],
        };

        Self {
            buffer,
            block_size,
            num_blocks,
            channels,
        }
    }

    pub fn set_buffer(&mut self, new_buffer: &[Complex<f32>]) {
        self.buffer.copy_from_slice(new_buffer);
    }

    pub fn push_block(&mut self, block: &[Complex<f32>], channel: usize) {
        let start = (self.block_size + 1) * self.num_blocks * channel;
        let end = start + ((self.block_size + 1) * self.num_blocks);
        let channel_buff = &mut self.buffer[start..end];

        channel_buff.copy_within(
            0..channel_buff.len() - (self.block_size + 1),
            self.block_size + 1,
        );

        channel_buff[0..self.block_size + 1].copy_from_slice(block);
    }

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
