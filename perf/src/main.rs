use convrs::{helpers::process_filter, upconv::UPConv};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use realfft::num_complex::Complex;

fn main() {
    let mut reader_1 =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/in/c2sine.wav").unwrap();
    let mut reader_2 =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/in/c3sine.wav").unwrap();
    println!("reader 1 spec: {:?}", reader_1.spec());
    println!("reader 2 spec: {:?}", reader_2.spec());

    let samples_1: Vec<f32> = reader_1
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();
    let samples_2: Vec<f32> = reader_2
        .samples::<i16>()
        .map(|s| s.unwrap() as f32 / i16::MAX as f32)
        .collect();

    let spec = WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut writer = WavWriter::create(
        "/Users/andrewthomas/dev/diy/convrs/test_sounds/out/c2sine_to_c3sine_l512.wav",
        spec,
    )
    .unwrap();

    let mut l_out = vec![];
    let mut r_out = vec![];

    let mut on_1 = true;

    let mut i = 0;
    for (s1, s2) in samples_1
        .chunks_exact(N * 2)
        .zip(samples_2.chunks_exact(N * 2))
    {
        let mut l1 = vec![];
        let mut r1 = vec![];
        let mut l2 = vec![];
        let mut r2 = vec![];

        for i in 0..(N * 2) {
            if i % 2 == 0 {
                l1.push(s1[i]);
                l2.push(s2[i]);
            } else {
                r1.push(s1[i]);
                r2.push(s2[i]);
            }
        }

        if i % 10 == 0 {
            let mut j = 0;
            let (oldl, newl, oldr, newr) = match on_1 {
                true => (&l1, &l2, &r1, &r2),
                false => (&l2, &l1, &r2, &r1),
            };

            for (one, two) in oldl.iter().zip(newl) {
                let mut out = 0.0;

                if j < (N - L) / 2 {
                    out += one;
                } else if j < N - ((N - L) / 2) {
                    let f_in = ((j - ((N - L) / 2)) / L) as f32;
                    let f_out = (1 - ((j - ((N - L) / 2)) / L)) as f32;

                    out += (one * f_out) + (two * f_in);
                } else {
                    out += two;
                }

                l_out.push(out);
                j += 1;
            }
            let mut j = 0;
            for (one, two) in oldr.iter().zip(newr) {
                let mut out = 0.0;

                if j < (N - L) / 2 {
                    out += one;
                } else if j < N - ((N - L) / 2) {
                    let f_in = ((j - ((N - L) / 2)) / L) as f32;
                    let f_out = (1 - ((j - ((N - L) / 2)) / L)) as f32;

                    out += (one * f_out) + (two * f_in);
                } else {
                    out += two;
                }

                r_out.push(out);
                j += 1;
            }
            on_1 = !on_1;
        } else {
            match on_1 {
                true => {
                    for l in &l1 {
                        l_out.push(*l);
                    }
                    for r in &r1 {
                        r_out.push(*r);
                    }
                }
                false => {
                    for l in &l2 {
                        l_out.push(*l);
                    }
                    for r in &r2 {
                        r_out.push(*r);
                    }
                }
            }
        }

        i += 1;
    }

    for (l, r) in l_out.iter().zip(r_out) {
        writer.write_sample(*l).unwrap();
        writer.write_sample(r).unwrap();
    }

    writer.finalize().unwrap();
}

const L: usize = 512;
const N: usize = 1024;

pub fn filter_swap() {
    let mut filter_1_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short.wav").unwrap();
    let mut filter_2_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short2.wav").unwrap();
    let mut input_reader =
        WavReader::open("/Users/andrewthomas/dev/diy/convrs/test_sounds/in/piano.wav").unwrap();
    println!("input spec: {:?}", input_reader.spec());
    println!("filter spec: {:?}", filter_1_reader.spec());

    let output_spec = WavSpec {
        channels: 2,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };
    let mut output_writer = WavWriter::create(
        "/Users/andrewthomas/dev/diy/convrs/test_sounds/out/multiblock_8_out_piano.wav",
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

    let mut conv = UPConv::new(
        128,
        &filter_1,
        2,
        8,
        filter_1.len().max(filter_2.len()).div_ceil(128),
    );

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
