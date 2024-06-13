use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
// use std::f32::consts::PI;
use std::sync::Arc;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    input_buff: Vec<f32>,
    input_fft_buff: Vec<f32>,
    output_buff: Vec<f32>,
    output_fft_buff: Vec<f32>,
    filter_fdl: Vec<Complex<f32>>,
    signal_fdl: Vec<Complex<f32>>,
    accumulation_buffer: Vec<Complex<f32>>,
    new_spectrum_buff: Vec<Complex<f32>>,
    block_size: usize,
    _channels: usize,
    num_blocks: usize,
}

impl UPConv {
    pub fn new(
        block_size: usize,
        starting_filter: &[Complex<f32>],
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

        let signal_fdl =
            vec![Complex { re: 0.0, im: 0.0 }; (block_size + 1) * num_blocks * channels];

        Self {
            fft,
            ifft,
            block_size,
            input_buff,
            input_fft_buff,
            output_buff,
            output_fft_buff,
            filter_fdl: Vec::from(starting_filter),
            signal_fdl,
            accumulation_buffer,
            new_spectrum_buff,
            _channels: channels,
            num_blocks,
        }
    }

    // this is block size * channels
    pub fn push_filter_block(&mut self, filter_block: &[f32]) {
        for (channel_block, channel_fdl) in filter_block.chunks_exact(self.block_size).zip(
            self.filter_fdl
                .chunks_exact_mut(self.block_size * self.num_blocks),
        ) {
            self.input_fft_buff.fill(0.0);
            self.input_fft_buff[0..channel_block.len()].copy_from_slice(channel_block);
            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            channel_fdl.copy_within(
                0..channel_fdl.len() - (self.block_size + 1),
                self.block_size + 1,
            );
            channel_fdl[0..self.block_size + 1].copy_from_slice(&self.new_spectrum_buff);
        }
    }

    /// block is a slice of channel slices, as opposed to a slice of sample slices,
    /// so there will be one block size slice of samples per channel in block
    pub fn process_block<'blocks>(
        &mut self,
        channel_blocks: impl Iterator<Item = &'blocks [f32]>,
    ) -> &[f32] {
        // move the inputs over by one block and add the new block on the end
        // iterate over everything by channel
        for ((((in_channel, out_channel), block_channel), fdl_channel), filter_channel) in self
            .input_buff
            .chunks_exact_mut(self.block_size * 2)
            .zip(self.output_buff.chunks_exact_mut(self.block_size))
            .zip(channel_blocks)
            .zip(
                self.signal_fdl
                    .chunks_exact_mut((self.block_size + 1) * self.num_blocks),
            )
            .zip(
                self.filter_fdl
                    .chunks_exact_mut((self.block_size + 1) * self.num_blocks),
            )
        {
            in_channel.copy_within(self.block_size..self.block_size * 2, 0);
            in_channel[self.block_size..self.block_size * 2].copy_from_slice(block_channel);

            self.new_spectrum_buff.fill(Complex { re: 0.0, im: 0.0 });
            self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });
            self.output_fft_buff.fill(0.0);
            self.input_fft_buff.copy_from_slice(&in_channel);

            self.fft
                .process_with_scratch(
                    &mut self.input_fft_buff,
                    &mut self.new_spectrum_buff,
                    &mut [],
                )
                .unwrap();

            fdl_channel.copy_within(
                0..fdl_channel.len() - (self.block_size + 1),
                self.block_size + 1,
            );
            fdl_channel[0..self.block_size + 1].copy_from_slice(&self.new_spectrum_buff);

            for (filter_block, fdl_block) in filter_channel
                .chunks_exact(self.block_size + 1)
                .zip(fdl_channel.chunks_exact(self.block_size + 1))
            {
                for ((filter_sample, fdl_sample), accum_sample) in filter_block
                    .iter()
                    .zip(fdl_block)
                    .zip(&mut self.accumulation_buffer)
                {
                    *accum_sample += filter_sample * fdl_sample;
                }
            }

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
