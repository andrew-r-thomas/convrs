#[cfg(test)]
mod tests {

    use hound::{WavSpec, WavWriter};

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

        let mut first_left = UPConv::new(128, filter_samples.len());
        let mut first_right = UPConv::new(128, filter_samples.len());
        let mut second_left = UPConv::new(512, filter_samples.len());
        let mut second_right = UPConv::new(512, filter_samples.len());

        let first = &filter_samples[0..4096];
        let second = &filter_samples[4096..];

        first_left.set_filter(&first);
        first_right.set_filter(&first);
        second_left.set_filter(&second);
        second_right.set_filter(&second);

        let spec = WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = WavWriter::create(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/scratch/7.wav",
            spec,
        )
        .unwrap();

        let mut left_out = vec![0.0; signal_samples_left.len() + 4096];
        let mut right_out = vec![0.0; signal_samples_right.len() + 4096];

        let mut i = 0;
        for chunk in signal_samples_left.chunks_exact(128) {
            let out = first_left.process_block(chunk);
            for j in 0..128 {
                left_out[j + i] += out[j];
            }
            i += 128;
        }

        i = 0;
        for chunk in signal_samples_right.chunks_exact(128) {
            let out = first_right.process_block(chunk);
            for j in 0..128 {
                right_out[j + i] += out[j];
            }
            i += 128;
        }

        i = 4096;
        for chunk in signal_samples_left.chunks_exact(512) {
            let out = second_left.process_block(chunk);
            for j in 0..512 {
                left_out[j + i] += out[j] / (512 / 128) as f32;
            }
            i += 512;
        }

        i = 4096;
        for chunk in signal_samples_right.chunks_exact(512) {
            let out = second_right.process_block(chunk);
            for j in 0..512 {
                right_out[j + i] += out[j] / (512 / 128) as f32;
            }
            i += 512;
        }

        for (l, r) in left_out.iter().zip(&right_out) {
            writer.write_sample(*l).unwrap();
            writer.write_sample(*r).unwrap();
        }

        writer.finalize().unwrap();
    }
}
