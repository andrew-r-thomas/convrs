use converb::upconv::UPConv;
use hound::{self, WavReader, WavSpec, WavWriter};

pub fn main() {
    let mut signal_reader =
        match WavReader::open("/Users/andrewthomas/dev/diy/convrs/converb/test_sounds/piano.wav") {
            Ok(r) => r,
            Err(e) => {
                println!("signal reader error: {}", e);
                return;
            }
        };

    let signal_bits = signal_reader.spec().bits_per_sample;
    let mut signal_left_samples: Vec<f32> = Vec::with_capacity(signal_reader.len() as usize / 2);
    let mut signal_right_samples: Vec<f32> = Vec::with_capacity(signal_reader.len() as usize / 2);
    match signal_reader.spec().sample_format {
        hound::SampleFormat::Float => {
            let mut i = 0;
            for s in signal_reader.samples::<f32>() {
                if i % 2 == 0 {
                    signal_left_samples.push(s.unwrap());
                } else {
                    signal_right_samples.push(s.unwrap());
                }

                i += 1;
            }
        }
        hound::SampleFormat::Int => match signal_bits {
            8 => {
                let mut i = 0;
                for s in signal_reader.samples::<i8>() {
                    if i % 2 == 0 {
                        signal_left_samples.push(s.unwrap() as f32 / i8::MAX as f32);
                    } else {
                        signal_right_samples.push(s.unwrap() as f32 / i8::MAX as f32);
                    }

                    i += 1;
                }
            }
            16 => {
                let mut i = 0;
                for s in signal_reader.samples::<i16>() {
                    if i % 2 == 0 {
                        signal_left_samples.push(s.unwrap() as f32 / i16::MAX as f32);
                    } else {
                        signal_right_samples.push(s.unwrap() as f32 / i16::MAX as f32);
                    }

                    i += 1;
                }
            }
            24 => {
                let mut i = 0;
                for s in signal_reader.samples::<i32>() {
                    if i % 2 == 0 {
                        signal_left_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    } else {
                        signal_right_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }

                    i += 1;
                }
            }
            32 => {
                let mut i = 0;
                for s in signal_reader.samples::<i32>() {
                    if i % 2 == 0 {
                        signal_left_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    } else {
                        signal_right_samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }

                    i += 1;
                }
            }
            _ => {
                println!("invalid reader format");
                return;
            }
        },
    };

    let mut filter_reader =
        match WavReader::open("/Users/andrewthomas/dev/diy/convrs/converb/IRs/shortsweet.wav") {
            Ok(r) => r,
            Err(e) => {
                println!("filter reader error: {}", e);
                return;
            }
        };

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
            _ => {
                println!("invalid filter reader format");
                return;
            }
        },
    };

    let mut left_upconv = UPConv::new(128, 48000);
    left_upconv.set_filter(&filter_samples);
    let mut right_upconv = UPConv::new(128, 48000);
    right_upconv.set_filter(&filter_samples);

    println!("filter rate: {}", filter_reader.spec().sample_rate);
    println!("signal rate: {}", signal_reader.spec().sample_rate);

    let spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create("test_out.wav", spec).unwrap();

    for (left_chunk, right_chunk) in signal_left_samples
        .chunks_exact_mut(128)
        .zip(signal_right_samples.chunks_exact_mut(128))
    {
        let left_out = left_upconv.process_block(left_chunk);
        let right_out = right_upconv.process_block(right_chunk);
        for (l, r) in left_out.iter().zip(right_out) {
            writer.write_sample(*l).unwrap();
            writer.write_sample(*r).unwrap();
        }
    }

    writer.finalize().unwrap();
}
