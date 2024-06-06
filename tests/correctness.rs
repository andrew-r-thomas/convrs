use convrs::{self, conv::Conv, helpers::process_filter};
use hound::{WavReader, WavSpec, WavWriter};
use realfft::{num_complex::Complex, RealFftPlanner};

#[test]
fn correctness() {
    let signal = load_signal();
    let short = load_short();

    let mut control_l = basic_fft_conv(&signal.0, &short[0]);
    let mut control_r = basic_fft_conv(&signal.1, &short[1]);

    // NOTE before, i thought we were using partition that worked,
    // but we got weird click in middle, so for some reason, this is only
    // good with this partition
    // TODO probably want to make other tests with different partitions, but
    // this is good enough for basic correctness
    let partition = &[(128, 22), (1024, 21), (8192, 23)];
    let short_processed = process_filter(short, partition);
    let mut conv = Conv::new(128, short_processed, partition, 2);

    let mut test_l_out = vec![];
    let mut test_r_out = vec![];

    for (l_block, r_block) in signal.0.chunks_exact(128).zip(signal.1.chunks_exact(128)) {
        let mut out = conv.process_block([l_block, r_block]).into_iter();

        let out_l = out.next().unwrap();
        let out_r = out.next().unwrap();

        test_l_out.extend_from_slice(out_l);
        test_r_out.extend_from_slice(out_r);
    }

    // TODO this is little bit heuristic, see if we can find general way to
    // convert gain situation, to make sure that the only difference between the two
    // is the gain, for example use average absolute difference
    let idx = control_l.len() / 2;
    let diff = control_l[idx].abs() / test_l_out[idx].abs();
    for (l, r) in control_l.iter_mut().zip(control_r.iter_mut()) {
        *l /= diff;
        *r /= diff;
    }

    write_to_wav((control_l.as_slice(), control_r.as_slice()), "control.wav");
    write_to_wav((test_l_out.as_slice(), test_r_out.as_slice()), "test.wav");
}

fn basic_fft_conv<'fft_conv>(signal: &[f32], filter: &[f32]) -> Vec<f32> {
    let fft_len = signal.len().max(filter.len()) * 2;

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    let mut fft_input = fft.make_input_vec();
    let mut signal_spectrum = fft.make_output_vec();
    let mut filter_spectrum = ifft.make_input_vec();
    let mut out = ifft.make_output_vec();

    fft_input.fill(0.0);
    signal_spectrum.fill(Complex { re: 0.0, im: 0.0 });
    fft_input[0..signal.len()].copy_from_slice(signal);
    fft.process(&mut fft_input, &mut signal_spectrum).unwrap();

    fft_input.fill(0.0);
    filter_spectrum.fill(Complex { re: 0.0, im: 0.0 });
    fft_input[0..filter.len()].copy_from_slice(filter);
    fft.process(&mut fft_input, &mut filter_spectrum).unwrap();

    for (s, f) in signal_spectrum.iter().zip(&mut filter_spectrum) {
        *f *= s;
    }

    out.fill(0.0);
    ifft.process(&mut filter_spectrum, &mut out).unwrap();

    out[0..fft_len / 2].into()
}

fn write_to_wav(data: (&[f32], &[f32]), filename: &str) {
    let mut root: String = "/Users/andrewthomas/dev/diy/convrs/tests/test_sounds/out/".to_owned();
    root.push_str(filename);

    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create(root, spec).unwrap();

    for (l, r) in data.0.iter().zip(data.1.iter()) {
        writer.write_sample(*l).unwrap();
        writer.write_sample(*r).unwrap();
    }

    writer.finalize().unwrap();
}

fn load_signal() -> (Vec<f32>, Vec<f32>) {
    let mut reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/tests/test_sounds/in/piano.wav")
            .unwrap();

    let signal: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    let mut l_out = vec![];
    let mut r_out = vec![];

    for (s, i) in signal.iter().zip(0..) {
        if i % 2 == 0 {
            l_out.push(*s);
        } else {
            r_out.push(*s);
        }
    }

    (l_out, r_out)
}

fn load_short() -> Vec<Vec<f32>> {
    let mut reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/tests/test_sounds/IRs/short2.wav")
            .unwrap();

    let filter: Vec<f32> = reader
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();

    vec![filter.clone(), filter.clone()]
}
