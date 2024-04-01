use realfft::RealFftPlanner;
use realfft::{ComplexToReal, RealToComplex};
use std::sync::Arc;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
    fft_size: usize,
}

impl UPConv {
    pub fn new(fft_size: usize) -> Self {
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);

        Self {
            fft,
            ifft,
            fft_size,
        }
    }
}
