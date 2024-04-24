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
    // NOTE we just do stereo filter for now,
    // might add arbitrary filter lengths in the future
    // the Conv class manages mono filters by simply copying them
    // into a stereo filter
    filter: Vec<Vec<Complex<f32>>>,
    // TODO lol this type just makes me not feel very good about life
    fdls: Vec<Vec<Vec<Complex<f32>>>>,
    accumulation_buffer: Vec<Complex<f32>>,
    new_spectrum_buff: Vec<Complex<f32>>,
    channels: usize,
    old_filter: (usize, Vec<Vec<Complex<f32>>>),
    fade_len: usize,
}

impl UPConv {
    pub fn new(
        block_size: usize,
        starting_filter: Vec<Vec<Complex<f32>>>,
        channels: usize,
        fade_len: usize,
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
            old_filter: (fade_len, old_filter),
            fade_len,
        }
    }

    pub fn update_filter<'filter>(
        &mut self,
        new_filter: impl IntoIterator<Item = &'filter [Complex<f32>]>,
    ) {
        // TODO maybe nice to have some more asserts here or we make a new type

        for ((new, current), old) in new_filter
            .into_iter()
            .zip(&mut self.filter)
            .zip(&mut self.old_filter.1)
        {
            old.copy_from_slice(current);
            current.copy_from_slice(new);
        }

        self.old_filter.0 = 0;
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

            if self.old_filter.0 <= self.fade_len {
                let old = &self.old_filter.1[i];
                self.accumulation_buffer.fill(Complex { re: 0.0, im: 0.0 });

                for (filter_block, fdl_block) in
                    // NOTE the &* makes me nervous
                    old.chunks(self.block_size + 1).zip(&*fdl)
                {
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

                for (o, f) in out
                    .iter_mut()
                    .zip(&self.output_fft_buff[self.block_size..self.block_size * 2])
                {
                    let f_in = (self.old_filter.0 / self.fade_len) as f32;
                    let f_out = (1 - (self.old_filter.0 / self.fade_len)) as f32;
                    *o *= f_in;
                    // then we mix add it with something weighted towards the beginning
                    *o += f * f_out;
                    // so its a weighted average, we multiply our end by some thing
                    // which favors the end
                }
            }
        }

        self.old_filter.0 += 1;
        self.output_buffs.iter().map(|o| o.as_slice())
    }
}
