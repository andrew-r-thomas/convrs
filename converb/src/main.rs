use converb::upconv::UPConv;
use hound::{self, WavReader, WavSpec, WavWriter};

pub fn main() {
    let mut signal_reader = match WavReader::open("../test_audio/test.wav") {
        Ok(r) => r,
        Err(_) => panic!(),
    };

    let bits = signal_reader.spec().bits_per_sample;
    let mut signal_samples: Vec<f32> = Vec::with_capacity(signal_reader.len() as usize);
    match signal_reader.spec().sample_format {
        hound::SampleFormat::Float => {
            for s in signal_reader.samples::<f32>() {
                signal_samples.push(s.unwrap());
            }
        }
        hound::SampleFormat::Int => match bits {
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

    let mut filter_reader = match WavReader::open("../test_audio/test.wav") {
        Ok(r) => r,
        Err(_) => panic!(),
    };

    let bits = filter_reader.spec().bits_per_sample;
    let mut filter_samples: Vec<f32> = Vec::with_capacity(filter_reader.len() as usize);
    match filter_reader.spec().sample_format {
        hound::SampleFormat::Float => {
            for s in filter_reader.samples::<f32>() {
                filter_samples.push(s.unwrap());
            }
        }
        hound::SampleFormat::Int => match bits {
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

    let mut upconv = UPConv::new(128, 48000);

    let spec = WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = WavWriter::create("test_out.wav", spec).unwrap();

    for sample_chunk in signal_samples.chunks_exact_mut(128) {
        let out = upconv.process_block(sample_chunk);
        for o in out {
            writer.write_sample(*o).unwrap();
        }
    }

    writer.finalize().unwrap();
}
