use realfft::{num_complex::Complex, RealFftPlanner};

/// this function is not real time safe
pub fn process_filter(filter: &[f32], partition: &[(usize, usize)]) -> Vec<Vec<Complex<f32>>> {
    // TODO consider taking ffts as arguments
    let mut planner = RealFftPlanner::<f32>::new();
    let ffts = partition.iter().map(|p| planner.plan_fft_forward(p.0 * 2));

    let mut out = vec![];
    let mut filter_index = 0;
    for (part, fft) in partition.iter().zip(ffts) {
        let filter_chunk = &filter
            [filter_index..(filter_index.min(filter.len()) + (part.0 * part.1)).min(filter.len())];

        for chunk in filter_chunk.chunks(part.0) {
            let mut fft_in = fft.make_input_vec();
            fft_in.fill(0.0);
            fft_in[0..chunk.len()].copy_from_slice(chunk);

            let mut fft_out = fft.make_output_vec();

            fft.process(&mut fft_in, &mut fft_out).unwrap();

            out.push(fft_out);
        }

        filter_index += part.0 * part.1;
    }
    out
}
