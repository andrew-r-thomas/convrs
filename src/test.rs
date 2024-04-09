#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::{conv::Conv, non_thread::NoThreadConv, upconv::UPConv};

    #[test]
    fn main_test() {
        let mut filter_reader =
            hound::WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/other.wav")
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
        let mut signal_samples: Vec<f32> = Vec::with_capacity(signal_reader.len() as usize);
        match signal_reader.spec().sample_format {
            hound::SampleFormat::Float => {
                for s in signal_reader.samples::<f32>() {
                    signal_samples.push(s.unwrap());
                }
            }
            hound::SampleFormat::Int => match signal_bits {
                8 => {
                    for s in signal_reader.samples::<i8>() {
                        signal_samples.push(s.unwrap() as f32 / i8::MAX as f32);
                    }
                }
                16 => {
                    for s in signal_reader.samples::<i16>() {
                        signal_samples.push(s.unwrap() as f32 / i16::MAX as f32);
                    }
                }
                24 => {
                    for s in signal_reader.samples::<i32>() {
                        signal_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                32 => {
                    for s in signal_reader.samples::<i32>() {
                        signal_samples.push(s.unwrap() as f32 / i32::MAX as f32);
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
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/out/piano_out_other.wav",
            writer_spec,
        )
        .unwrap();

        let mut left_conv = NoThreadConv::new(128, filter_samples.len(), &filter_samples);
        let mut right_conv = NoThreadConv::new(128, filter_samples.len(), &filter_samples);

        for chunk in signal_samples.chunks_exact(128 * 2) {
            let mut left = vec![];
            let mut right = vec![];

            for i in 0..chunk.len() {
                if i % 2 == 0 {
                    left.push(chunk[i]);
                } else {
                    right.push(chunk[i]);
                }
            }

            let left_out: &[f32] = left_conv.process_block(&mut left);
            let right_out: &[f32] = right_conv.process_block(&mut right);

            for (l, r) in left_out.iter().zip(right_out) {
                writer.write_sample(*l).unwrap();
                writer.write_sample(*r).unwrap();
            }
            // thread::sleep(Duration::from_secs(1));
        }

        writer.finalize().unwrap();
    }
}
