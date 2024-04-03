use nih_plug::nih_log;
use realfft::{num_complex::Complex, ComplexToReal, RealToComplex};
use realfft::{FftError, RealFftPlanner};
use std::sync::Arc;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    block_size: usize,
    filter: Vec<Vec<Complex<f32>>>,
    fdl: Vec<Vec<Complex<f32>>>,
    accumulation_buffer: Vec<Complex<f32>>,
    new_spectrum_buff: Vec<Complex<f32>>,
}

impl UPConv {
    pub fn new(block_size: usize, max_filter_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_buffer = fft.make_input_vec();
        let output_buffer = ifft.make_output_vec();
        let accumulation_buffer = ifft.make_input_vec();
        let new_spectrum_buff = fft.make_output_vec();

        let p = max_filter_size.div_ceil(block_size);
        let filter = Vec::with_capacity(p);

        let fdl = vec![vec![Complex { re: 0.0, im: 0.0 }; block_size + 1]; p];

        Self {
            fft,
            ifft,
            block_size,
            input_buffer,
            output_buffer,
            filter,
            fdl,
            accumulation_buffer,
            new_spectrum_buff,
        }
    }

    // NOTE
    // right now this takes in complex values,
    // which means the conversion should be done by the user,
    // this might not be the ideal way to do this
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
    }

    pub fn process_block(&mut self, block: &mut [f32]) -> &[f32] {
        self.input_buffer.copy_within(self.block_size.., 0);
        self.input_buffer[self.block_size..].copy_from_slice(block);

        self.fft
            .process_with_scratch(&mut self.input_buffer, &mut self.new_spectrum_buff, &mut [])
            .unwrap();

        let fdl_len = self.fdl.len();

        self.fdl
            .get_mut(fdl_len - 1)
            .unwrap()
            .copy_from_slice(&self.new_spectrum_buff);

        self.fdl.rotate_right(1);

        self.multiply_blocks();

        match self.ifft.process_with_scratch(
            &mut self.accumulation_buffer,
            &mut self.output_buffer,
            &mut [],
        ) {
            Ok(_) => {}
            Err(e) => match e {
                FftError::InputBuffer(_, _) => nih_log!("upconv ifft error, input buffer"),
                FftError::OutputBuffer(_, _) => nih_log!("upconv ifft error, output buffer"),
                FftError::ScratchBuffer(_, _) => nih_log!("upconv ifft error, scratch buffer"),
                FftError::InputValues(_, _) => nih_log!("upconv ifft error, input values"),
            },
        }

        &self.output_buffer[self.block_size..]
    }

    fn multiply_blocks(&mut self) {
        for (filter_block, fdl_block) in self.filter.iter().zip(&self.fdl) {
            for i in 0..self.block_size + 1 {
                self.accumulation_buffer[i] =
                    self.accumulation_buffer[i] + (filter_block[i] * fdl_block[i]);
            }
        }
    }
}
