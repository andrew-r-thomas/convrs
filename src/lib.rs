use num::{Float, Zero};
use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, FftNum, RealToComplex};
use std::sync::Arc;

pub struct UPConv<T: Float + FftNum, const N: usize> {
    fft: Arc<dyn RealToComplex<T>>,
    ifft: Arc<dyn ComplexToReal<T>>,
    input_buffer: Vec<T>,
    output_buffer: Vec<T>,
    block_size: usize,
    filter: Vec<Vec<Complex<T>>>,
    fdl: Vec<Vec<Complex<T>>>,
    comp_buff: Vec<Complex<T>>,
}

impl<T: Float + FftNum, const N: usize> UPConv<T, N> {
    pub fn new(block_size: usize, max_filter_size: usize) -> Self {
        let mut planner = RealFftPlanner::<T>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_buffer = fft.make_input_vec();
        let output_buffer = ifft.make_output_vec();
        let comp_buff = ifft.make_input_vec();

        let p = max_filter_size.div_ceil(block_size);
        let filter = vec![Vec::with_capacity(block_size + 1); p];

        let fdl = vec![Vec::with_capacity(block_size + 1); p];

        Self {
            fft,
            ifft,
            block_size,
            input_buffer,
            output_buffer,
            filter,
            fdl,
            comp_buff,
        }
    }

    // NOTE
    // right now this takes in complex values,
    // which means the conversion should be done by the user,
    // this might not be the ideal way to do this
    pub fn set_filter(&mut self, new_filter: &[Complex<T>]) {
        let mut filter_iter = new_filter.chunks_exact(self.block_size);

        for p in &mut self.filter {
            match filter_iter.next() {
                Some(c) => p.copy_from_slice(c),
                None => {
                    p.fill(Complex {
                        re: Zero::zero(),
                        im: Zero::zero(),
                    });
                    let r = filter_iter.remainder();
                    p[0..r.len()].copy_from_slice(r);
                }
            };
        }
    }

    pub fn process_block(&mut self, block: &mut [T]) -> &[T] {
        self.input_buffer.copy_within(self.block_size.., 0);
        self.input_buffer[self.block_size..].copy_from_slice(block);

        let fdl_len = self.fdl.len();

        self.fdl.rotate_left(fdl_len - 1);

        self.fft
            .process_with_scratch(&mut self.input_buffer, &mut self.fdl[0], &mut [])
            .unwrap();

        self.multiply_blocks();

        self.ifft
            .process_with_scratch(&mut self.comp_buff, &mut self.output_buffer, &mut [])
            .unwrap();

        &self.output_buffer[self.block_size..]
    }

    fn multiply_blocks(&mut self) {
        for (filter_block, fdl_block) in self.filter.iter().zip(&self.fdl) {
            for i in 0..self.block_size + 1 {
                self.comp_buff[i] = self.comp_buff[i] + (filter_block[i] * fdl_block[i]);
            }
        }
    }
}
