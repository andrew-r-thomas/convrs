use nih_plug::debug::nih_log;
use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use std::sync::Arc;

/*
NOTE
trying to figure out why the filter swapping is
causing the filter to just stop being applied

its not the crossfading, we run into the same issue
even if we get rid of that code, so it's either
something to do with the update filter function here

or somewhere in conv.rs
*/

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
    old_filter: (bool, Vec<Vec<Complex<f32>>>),
}

impl UPConv {
    pub fn new(
        block_size: usize,
        starting_filter: Vec<Vec<Complex<f32>>>,
        channels: usize,
        num_blocks: usize,
    ) -> Self {
        assert!(starting_filter.len() == channels);

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_fft_buff = fft.make_input_vec();
        let output_fft_buff = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let new_spectrum_buff = fft.make_output_vec();

        let input_buffs = vec![vec![0.0; block_size * 2]; channels];
        let output_buffs = vec![vec![0.0; block_size]; channels];

        let old_filter =
            vec![vec![Complex { re: 0.0, im: 0.0 }; (block_size + 1) * num_blocks]; channels];

        let fdls =
            vec![vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; num_blocks]; channels];

        Self {
            fft,
            ifft,
            block_size,
            input_buffs,
            input_fft_buff,
            output_buffs,
            output_fft_buff,
            filter: starting_filter,
            fdls,
            accumulation_buffer,
            new_spectrum_buff,
            channels,
            old_filter: (true, old_filter),
        }
    }

    pub fn update_filter<'filter>(
        &mut self,
        new_filter: impl IntoIterator<Item = &'filter [Complex<f32>]> + ExactSizeIterator,
    ) {
        assert!(new_filter.len() == self.channels);

        for ((new, current), old) in new_filter
            .into_iter()
            .zip(&mut self.filter)
            .zip(&mut self.old_filter.1)
        {
            assert!(new.len() == old.len());
            assert!(current.len() == new.len());

            old.copy_from_slice(current);
            current.copy_from_slice(new);
        }

        self.old_filter.0 = true;
    }

    /// block is a slice of channel slices, as opposed to a slice of sample slices,
    /// so there will be one block size slice of samples per channel in block
    pub fn process_block<'blocks>(
        &mut self,
        channel_blocks: impl IntoIterator<Item = &'blocks [f32]> + ExactSizeIterator,
    ) -> impl IntoIterator<Item = &[f32]> + ExactSizeIterator {
        let mut blocks = channel_blocks.into_iter();
        // move the inputs over by one block and add the new block on the end
        for i in 0..self.channels {
            let buff = &mut self.input_buffs[i];
            let block = blocks.next().unwrap();
            let fdl = &mut self.fdls[i];
            let filter = &self.filter[i];
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

            for (filter_block, fdl_block) in filter.chunks(self.block_size + 1).zip(&*fdl) {
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

            //     if self.old_filter.0 {
            //         nih_log!(
            //             "doing filter swap in process block on segment with block len: {}",
            //             self.block_size
            //         );

            //         let old = &self.old_filter.1[i];
            //         self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });

            //         for (filter_block, fdl_block) in
            //             old.chunks(self.block_size + 1).zip(&*fdl)
            //         {
            //             for i in 0..self.block_size + 1 {
            //                 self.accumulation_buffer[i] += filter_block[i] * fdl_block[i];
            //             }
            //         }

            //         self.ifft
            //             .process_with_scratch(
            //                 &mut self.accumulation_buffer,
            //                 &mut self.output_fft_buff,
            //                 &mut [],
            //             )
            //             .unwrap();

            //         let mut j = 0;
            //         for (o, f) in out
            //             .iter_mut()
            //             .zip(&self.output_fft_buff[self.block_size..self.block_size * 2])
            //         {
            //             let f_in = (j / self.block_size) as f32;
            //             let f_out = 1.0 - f_in;
            //             *o *= f_in;
            //             *o += f * f_out;
            //             j += 1;
            //         }
            //         self.old_filter.0 = false;
            //     }
        }

        self.output_buffs.iter().map(|o| o.as_slice())
    }
}
