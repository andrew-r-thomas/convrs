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

        let writer_spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/out/piano_out_weirdstacked_realylong.wav",
            writer_spec,
        )
        .unwrap();

        // let mut left_conv = NoThreadConv::new(128, filter_samples.len(), &filter_samples);
        // let mut right_conv = NoThreadConv::new(128, filter_samples.len(), &filter_samples);
        // let mut left_conv = UPConv::new(128, filter_samples.len());
        // let mut right_conv = UPConv::new(128, filter_samples.len());
        // left_conv.set_filter(&filter_samples);
        // right_conv.set_filter(&filter_samples);

        // for (lchunk, rchunk) in signal_samples_left
        //     .chunks_exact_mut(128)
        //     .zip(signal_samples_right.chunks_exact_mut(128))
        // {
        //     let left_out: &[f32] = left_conv.process_block(lchunk);
        //     let right_out: &[f32] = right_conv.process_block(rchunk);

        //     for (l, r) in left_out.iter().zip(right_out) {
        //         writer.write_sample(*l).unwrap();
        //         writer.write_sample(*r).unwrap();
        //     }
        //     // thread::sleep(Duration::from_secs(1));
        // }

        let first = &filter_samples[0..4096];
        let second = &filter_samples[4096..];

        // let left_first = straight_fft_conv(&signal_samples_left, &first);
        // let right_first = straight_fft_conv(&signal_samples_right, &first);

        // let left_second = straight_fft_conv(&signal_samples_left, &second);
        // let right_second = straight_fft_conv(&signal_samples_right, &second);

        let mut left_first = UPConv::new(128, first.len());
        let mut right_first = UPConv::new(128, first.len());
        let mut left_second = UPConv::new(512, second.len());
        let mut right_second = UPConv::new(512, second.len());

        left_first.set_filter(&first);
        right_first.set_filter(&first);
        left_second.set_filter(&second);
        right_second.set_filter(&second);

        let mut left_out = vec![0.0; signal_samples_left.len()];
        let mut right_out = vec![0.0; signal_samples_left.len()];

        let mut i = 0;
        for chunk in signal_samples_left.chunks_exact_mut(128) {
            let out = left_first.process_block(chunk);
            for j in 0..128 {
                left_out[j + i] += out[j];
            }
            i += 128;
        }

        i = 0;
        for chunk in signal_samples_left.chunks_exact_mut(512) {
            let out = left_second.process_block(chunk);
            for j in 0..512 {
                left_out[j + i] += out[j];
            }
            i += 512;
        }

        i = 0;
        for chunk in signal_samples_right.chunks_exact_mut(128) {
            let out = right_first.process_block(chunk);
            for j in 0..128 {
                right_out[j + i] += out[j];
            }
            i += 128;
        }

        i = 0;
        for chunk in signal_samples_right.chunks_exact_mut(512) {
            let out = right_second.process_block(chunk);
            for j in 0..512 {
                right_out[j + i] += out[j];
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
