use convrs::{conv::Conv, helpers::process_filter, upconv::UPConv};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use realfft::num_complex::Complex;

fn main() {
    let mut filter_1_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short.wav").unwrap();
    let mut filter_2_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short2.wav").unwrap();
    let mut input_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/in/c3sine.wav").unwrap();

    let output_spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut output_writer = WavWriter::create(
        "/Users/andrewthomas/dev/diy/convrs/test_sounds/out/uhh_out_c3sine.wav",
        output_spec,
    )
    .unwrap();

    // TODO this is assuming we should use i32
    let filter_1: Vec<f32> = filter_1_reader
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();
    let filter_2: Vec<f32> = filter_2_reader
        .samples::<i32>()
        .map(|s| s.unwrap() as f32 / i32::MAX as f32)
        .collect();
    let input: Vec<f32> = input_reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    println!("input spec: {:?}", input_reader.spec());

    let mut conv = UPConv::new(128, filter_1.len().max(filter_2.len()), &filter_1, 2);
    let partition = &[(
        128,
        (filter_1.len().div_ceil(128)).max(filter_2.len().div_ceil(128)),
    )];

    let filter_spectrum_1: Vec<Complex<f32>> = process_filter(&filter_1, partition)
        .into_iter()
        .flatten()
        .collect();

    let filter_spectrum_2: Vec<Complex<f32>> = process_filter(&filter_2, partition)
        .into_iter()
        .flatten()
        .collect();
    println!("filter 1 len: {}", filter_spectrum_1.len());
    println!("filter 2 len: {}", filter_spectrum_2.len());

    let mut on_1 = true;
    for (chunk, i) in input.chunks_exact(128 * 2).zip(0..) {
        if i % 100 == 0 {
            if on_1 {
                conv.update_filter(&filter_spectrum_2);
                on_1 = false;
            } else {
                conv.update_filter(&filter_spectrum_1);
                on_1 = true;
            }
        }
        let mut left = vec![];
        let mut right = vec![];

        for i in 0..(128 * 2) {
            if i % 2 == 0 {
                left.push(chunk[i]);
            } else {
                right.push(chunk[i]);
            }
        }

        let channels = [left.as_slice(), right.as_slice()];

        let mut out = conv.process_block(channels).into_iter();

        let left_out = out.next().unwrap();
        let right_out = out.next().unwrap();

        for (l, r) in left_out.iter().zip(right_out) {
            output_writer.write_sample(*l).unwrap();
            output_writer.write_sample(*r).unwrap();
        }
    }

    match output_writer.finalize() {
        Ok(_) => {}
        Err(e) => println!("error finalizing: {}", e),
    }
}
