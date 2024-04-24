use hound::{SampleFormat, WavReader, WavSpec, WavWriter};

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
