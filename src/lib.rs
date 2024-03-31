use realfft::{ComplexToReal, RealToComplex};
use std::sync::Arc;

pub struct UPConv {
    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,
}

impl UPConv {
    pub fn new() -> Self {}
}
