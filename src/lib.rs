use num::{Float, Zero};
use realfft::RealFftPlanner;
use realfft::{num_complex::Complex, ComplexToReal, FftNum, RealToComplex};
use std::collections::VecDeque;
use std::sync::Arc;

pub struct UPConv<T: FftNum> {
    fft: Arc<dyn RealToComplex<T>>,
    ifft: Arc<dyn ComplexToReal<T>>,
    input_buffer: Vec<T>,
    output_buffer: Vec<T>,
    new_block_buffer: Vec<Complex<T>>,
    block_size: usize,
    filter: Vec<Vec<Complex<T>>>,
    fdl: VecDeque<Vec<Complex<T>>>,
}

impl<T: Float + FftNum> UPConv<T> {
    pub fn new(block_size: usize, max_filter_size: usize) -> Self {
        let mut planner = RealFftPlanner::<T>::new();
        let fft = planner.plan_fft_forward(block_size * 2);
        let ifft = planner.plan_fft_inverse(block_size * 2);

        let input_buffer = fft.make_input_vec();
        let output_buffer = ifft.make_output_vec();

        let p = max_filter_size.div_ceil(block_size);
        let filter = vec![Vec::with_capacity(block_size + 1); p];

        let fdl = VecDeque::from(vec![Vec::with_capacity(block_size + 1); p]);

        let new_block_buffer = Vec::with_capacity(block_size + 1);

        Self {
            fft,
            ifft,
            block_size,
            input_buffer,
            output_buffer,
            filter,
            fdl,
            new_block_buffer,
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

    pub fn process_block(&mut self, block: &mut [T]) {
        self.input_buffer.rotate_left(self.block_size);
        self.input_buffer[self.block_size..].copy_from_slice(block);

        self.new_block_buffer = self.fdl.pop_front().unwrap();

        self.fft
            .process_with_scratch(block, &mut self.new_block_buffer, &mut [])
            .unwrap();

        self.fdl.push_back(self.new_block_buffer);
    }
}
