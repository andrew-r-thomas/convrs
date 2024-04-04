use crate::upconv::UPConv;
use hound::{self, WavReader};

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

    let upconv = UPConv::new(128, 48000);
}
