#[cfg(test)]
mod tests {

    use crate::{
        conv::Conv, non_thread::NoThreadConv, straight_fft_conv::straight_fft_conv, upconv::UPConv,
    };

    #[test]
    fn main_test() {
        let mut filter_reader = hound::WavReader::open(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/realylong.wav",
        )
        .unwrap();
        let mut signal_reader =
            hound::WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/in/piano.wav")
                .unwrap();

        let filter_bits = filter_reader.spec().bits_per_sample;
        let mut filter_samples: Vec<f32> = Vec::with_capacity(filter_reader.len() as usize);
        match filter_reader.spec().sample_format {
            hound::SampleFormat::Float => {
                for s in filter_reader.samples::<f32>() {
                    filter_samples.push(s.unwrap());
                }
            }
            hound::SampleFormat::Int => match filter_bits {
                8 => {
                    for s in filter_reader.samples::<i8>() {
                        filter_samples.push(s.unwrap() as f32 / i8::MAX as f32);
                    }
                }
                16 => {
                    for s in filter_reader.samples::<i16>() {
                        filter_samples.push(s.unwrap() as f32 / i16::MAX as f32);
                    }
                }
                24 => {
                    for s in filter_reader.samples::<i32>() {
                        filter_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                32 => {
                    for s in filter_reader.samples::<i32>() {
                        filter_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                _ => panic!(),
            },
        };
        println!("fitler sample len: {}", filter_samples.len());

        let signal_bits = signal_reader.spec().bits_per_sample;
        let mut signal_samples_left: Vec<f32> =
            Vec::with_capacity(signal_reader.len() as usize / 2);
        let mut signal_samples_right: Vec<f32> =
            Vec::with_capacity(signal_reader.len() as usize / 2);
        match signal_reader.spec().sample_format {
            hound::SampleFormat::Float => {
                let mut i = 0;
                for s in signal_reader.samples::<f32>() {
                    if i % 2 == 0 {
                        signal_samples_left.push(s.unwrap());
                    } else {
                        signal_samples_right.push(s.unwrap());
                    }
                    i += 1;
                }
            }
            hound::SampleFormat::Int => match signal_bits {
                8 => {
                    let mut i = 0;
                    for s in signal_reader.samples::<i8>() {
                        if i % 2 == 0 {
                            signal_samples_left.push(s.unwrap() as f32 / i8::MAX as f32);
                        } else {
                            signal_samples_right.push(s.unwrap() as f32 / i8::MAX as f32);
                        }
                        i += 1;
                    }
                }
                16 => {
                    let mut i = 0;
                    for s in signal_reader.samples::<i16>() {
                        if i % 2 == 0 {
                            signal_samples_left.push(s.unwrap() as f32 / i16::MAX as f32);
                        } else {
                            signal_samples_right.push(s.unwrap() as f32 / i16::MAX as f32);
                        }
                        i += 1;
                    }
                }
                24 => {
                    let mut i = 0;
                    for s in signal_reader.samples::<i32>() {
                        if i % 2 == 0 {
                            signal_samples_left.push(s.unwrap() as f32 / i32::MAX as f32);
                        } else {
                            signal_samples_right.push(s.unwrap() as f32 / i32::MAX as f32);
                        }
                        i += 1;
                    }
                }
                32 => {
                    let mut i = 0;
                    for s in signal_reader.samples::<i32>() {
                        if i % 2 == 0 {
                            signal_samples_left.push(s.unwrap() as f32 / i32::MAX as f32);
                        } else {
                            signal_samples_right.push(s.unwrap() as f32 / i32::MAX as f32);
                        }
                        i += 1;
                    }
                }
                _ => panic!(),
            },
        };

        let mut first = UPConv::new(128, filter_samples.len());
        let mut second = UPConv::new(512, filter_samples.len());
        let mut third = UPConv::new(1024, filter_samples.len());
        let mut fourth = UPConv::new(2048, filter_samples.len());
        let mut fifth = UPConv::new(4096, filter_samples.len());

        first.set_filter(&filter_samples);
        second.set_filter(&filter_samples);
        third.set_filter(&filter_samples);
        fourth.set_filter(&filter_samples);
        fifth.set_filter(&filter_samples);

        let mut first_out: Vec<f32> = vec![];
        let mut second_out: Vec<f32> = vec![];
        let mut third_out: Vec<f32> = vec![];
        let mut fourth_out: Vec<f32> = vec![];
        let mut fifth_out: Vec<f32> = vec![];

        let mut data = vec![
            (&mut first, &mut first_out, 128),
            (&mut second, &mut second_out, 512),
            (&mut third, &mut third_out, 1024),
            (&mut fourth, &mut fourth_out, 2048),
            (&mut fifth, &mut fifth_out, 4096),
        ];

        for (conv, out, size) in &mut data {
            for chunk in signal_samples_left.chunks_exact_mut(*size) {
                let out_chunk = conv.process_block(chunk);
                for i in 0..out_chunk.len() {
                    out.push(out_chunk[i] / (*size / 128) as f32);
                }
            }
        }

        let control = data[0].1.clone();

        for (_, out, size) in &mut data {
            let mut diff: Vec<f32> = vec![];
            for (c, d) in control.iter().zip(out.as_slice()) {
                diff.push(d / c);
            }

            let mut avg = 0.0;
            for d in &diff {
                avg += d;
            }
            avg /= diff.len() as f32;

            println!("size {} diff {:?}", size, avg);
        }
    }
}
