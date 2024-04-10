use realfft::{num_complex::Complex, RealFftPlanner};

pub fn straight_fft_conv(signal: Vec<f32>, filter: &Vec<f32>) -> Vec<f32> {
    let mut planner = RealFftPlanner::<f32>::new();
    let fft_len = (filter.len() * 2).max(signal.len() * 2);
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    let mut fft_input_buff = fft.make_input_vec();
    let mut signal_spectrum = fft.make_output_vec();
    let mut filter_spectrum = fft.make_output_vec();

    let mut ifft_input_buff = ifft.make_input_vec();
    let mut ifft_output_buff = ifft.make_output_vec();

    fft_input_buff.fill(0.0);
    signal_spectrum.fill(Complex { re: 0.0, im: 0.0 });
    filter_spectrum.fill(Complex { re: 0.0, im: 0.0 });
    ifft_input_buff.fill(Complex { re: 0.0, im: 0.0 });
    ifft_output_buff.fill(0.0);

    for i in 0..signal.len() {
        fft_input_buff[i] += signal[i];
    }

    fft.process(&mut fft_input_buff, &mut signal_spectrum)
        .unwrap();

    fft_input_buff.fill(0.0);
    for i in 0..filter.len() {
        fft_input_buff[i] += filter[i];
    }

    fft.process(&mut fft_input_buff, &mut filter_spectrum)
        .unwrap();

    for i in 0..ifft_input_buff.len() {
        ifft_input_buff[i] += signal_spectrum[i] * filter_spectrum[i];
    }

    ifft.process(&mut ifft_input_buff, &mut ifft_output_buff)
        .unwrap();

    ifft_output_buff
}
