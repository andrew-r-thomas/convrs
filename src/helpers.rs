use realfft::{num_complex::Complex, RealFftPlanner};

/// this function is not real time safe
/// outermost vec is segment wise,
/// middle vec is channel wize
/// innermost vec is block wise
pub fn process_filter(
    filter: &[f32],
    mono: bool,
    channels: usize,
    partition: &[(usize, usize)],
) -> Vec<Vec<Vec<Complex<f32>>>> {
    let mut planner = RealFftPlanner::<f32>::new();
    let mut ffts = partition.iter().map(|p| planner.plan_fft_forward(p.0 * 2));

    let mut out = vec![];
    let channel_filters = {
        let mut cf = vec![];
        for i in 0..channels {
            let mut c = vec![];
            for j in 0..filter.len() {
                if !mono && j % channels == i % channels {
                    c.push(filter[j]);
                } else if mono {
                    c.push(filter[j]);
                }
            }
            cf.push(c);
        }
        cf
    };

    let mut filter_index = 0;
    for (part, fft) in partition.iter().zip(&mut ffts) {
        let mut part_vec = vec![];
        for channel_filter in &channel_filters {
            let mut channel_vec = vec![];
            let filter_chunk = &channel_filter[filter_index.min(channel_filter.len())
                ..(filter_index.min(channel_filter.len()) + (part.0 * part.1))
                    .min(channel_filter.len())];

            for chunk in filter_chunk.chunks(part.0) {
                let mut fft_in = fft.make_input_vec();
                fft_in.fill(0.0);
                fft_in[0..chunk.len()].copy_from_slice(chunk);

                let mut fft_out = fft.make_output_vec();

                fft.process(&mut fft_in, &mut fft_out).unwrap();

                channel_vec.extend(fft_out);
            }
            channel_vec.extend(vec![
                Complex { re: 0.0, im: 0.0 };
                ((part.0 + 1) * part.1) - channel_vec.len()
            ]);

            part_vec.push(channel_vec);
        }

        filter_index += part.0 * part.1;
        out.push(part_vec);
    }
    out
}
