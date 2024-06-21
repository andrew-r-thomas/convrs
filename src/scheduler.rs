use std::{collections::HashMap, thread};

use itertools::FilterMapOk;
use realfft::num_complex::Complex;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::upconv::UPConv;

pub struct Scheduler {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    in_buffs: HashMap<&'static str, (Vec<f32>, usize)>,
    partition: Vec<(usize, usize)>,
    out_buff: Vec<f32>,
    process_counter: usize,
    block_size: usize,
    channels: usize,
}

struct SegmentHandle {
    real_prods: HashMap<&'static str, Producer<f32>>,
    complex_prods: HashMap<&'static str, Producer<Complex<f32>>>,
    // TODO this will be an arbitrary length instead of a tuple eventually
    // for multiple signals/filters at once
    process_prod: Producer<(&'static str, &'static str)>,
    out_cons: Consumer<f32>,
    block_size: usize,
    num_blocks: usize,
    offset: usize,
    avail: usize,
}

pub struct FdlConfig {
    pub name: &'static str,
    pub complex_ringbuff: bool,
    pub real_ringbuff: bool,
    pub moving: bool,
}

// TODO you could consider passing already made upconvs to this, and just have it
// move them to different threads and handle the comms
// makes it a bit more modular
// could also pass the timestamps and stuff rather than the partition
impl Scheduler {
    pub fn new(partition: &[(usize, usize)], fdl_config: &[FdlConfig], channels: usize) -> Self {
        let rt_segment = UPConv::new(
            partition.first().unwrap().0,
            channels,
            partition.first().unwrap().1,
            fdl_config.iter().map(|f| f.name),
        );

        let mut in_buffs = HashMap::new();
        for fdl in fdl_config {
            if fdl.real_ringbuff {
                in_buffs.insert(
                    fdl.name,
                    (vec![0.0; partition.last().unwrap().0 * channels], 0),
                );
            }
        }

        let mut non_rt_segments = Vec::with_capacity(partition.len() - 1);
        let mut offset_samples = partition[0].0 * partition[0].1;

        for part in partition.get(1..).unwrap_or(&[]) {
            let p = *part;
            let mut real_prods = HashMap::new();
            let mut complex_prods = HashMap::new();
            let mut complex_conss = vec![];
            let mut real_conss = vec![];

            for fdl in fdl_config {
                if fdl.complex_ringbuff {
                    let (complex_prod, complex_cons) =
                        RingBuffer::<Complex<f32>>::new(p.0 * p.1 * channels * 2);
                    complex_prods.insert(fdl.name, complex_prod);
                    complex_conss.push((fdl.name, complex_cons));
                }
                if fdl.real_ringbuff {
                    // TODO make this the correct ringbuff size based on offset etc
                    let (real_prod, real_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);
                    real_prods.insert(fdl.name, real_prod);
                    real_conss.push((fdl.name, real_cons, fdl.moving));
                }
            }

            // TODO make this the correct ringbuff size based on offset etc
            let (mut out_prod, out_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);
            let (process_prod, mut process_cons) =
                RingBuffer::<(&'static str, &'static str)>::new(2);
            let mut upconv = UPConv::new(p.0, channels, p.1, fdl_config.iter().map(|f| f.name));

            thread::spawn(move || loop {
                for (fdl_name, complex_cons) in &mut complex_conss {
                    if !complex_cons.is_empty() {
                        match complex_cons.read_chunk((p.0 + 1) * p.1 * channels) {
                            Ok(r) => {
                                let (s1, s2) = r.as_slices();

                                // TODO maybe have a buff already made for perf
                                upconv.set_fdl_buff(&[s1, s2].concat(), &fdl_name);

                                r.commit_all();
                            }
                            Err(_) => todo!(),
                        }
                    }
                }
                for (fdl_name, real_cons, moving) in &mut real_conss {
                    if !real_cons.is_empty() {
                        match real_cons.read_chunk(p.0 * channels) {
                            Ok(r) => {
                                let (s1, s2) = r.as_slices();

                                // TODO again, maybe a premade buff for perf
                                upconv.push_chunk(
                                    &fdl_name,
                                    [s1, s2].concat().chunks_exact(p.0),
                                    *moving,
                                );

                                r.commit_all();
                            }
                            Err(_) => todo!(),
                        }
                    }
                }
                if !process_cons.is_empty() {
                    let (fdl_1, fdl_2) = process_cons.pop().unwrap();
                    let out = upconv.process(fdl_1, fdl_2);
                    match out_prod.write_chunk(p.0 * channels) {
                        Ok(mut w) => {
                            let (s1, s2) = w.as_mut_slices();
                            s1.copy_from_slice(&out[0..s1.len()]);
                            s2.copy_from_slice(&out[s1.len()..s1.len() + s2.len()]);
                            w.commit_all();
                        }
                        Err(_) => todo!(),
                    }
                }
            });

            non_rt_segments.push(SegmentHandle {
                real_prods,
                complex_prods,
                out_cons,
                block_size: p.0,
                num_blocks: p.1,
                avail: p.0 / partition.first().unwrap().0,
                offset: offset_samples / partition.first().unwrap().0,
                process_prod,
            });

            offset_samples += p.0 * p.1;
        }

        let out_buff = vec![0.0; partition.last().unwrap().0 * 2 * channels];

        Self {
            rt_segment,
            in_buffs,
            out_buff,
            partition: Vec::from(partition),
            channels,
            non_rt_segments,
            block_size: partition.first().unwrap().0,
            process_counter: 0,
        }
    }

    pub fn set(&mut self, fdl_key: &'static str, data: &[Complex<f32>]) {
        let mut index = 0;
        let first_part = self.partition.first().unwrap();
        let first = &data[0..(first_part.0 + 1) * first_part.1 * self.channels];

        self.rt_segment.set_fdl_buff(first, fdl_key);
        index += (first_part.0 + 1) * first_part.1 * self.channels;

        for seg in &mut self.non_rt_segments {
            let chunk = &data[index..index + (seg.block_size + 1) * seg.num_blocks * self.channels];

            let complex_prod = seg.complex_prods.get_mut(fdl_key).unwrap();

            match complex_prod.write_chunk((seg.block_size + 1) * seg.num_blocks * self.channels) {
                Ok(mut w) => {
                    let (s1, s2) = w.as_mut_slices();

                    s1.copy_from_slice(&chunk[0..s1.len()]);
                    s2.copy_from_slice(&chunk[s1.len()..s1.len() + s2.len()]);

                    w.commit_all();
                }
                Err(_) => todo!(),
            }

            index += (seg.block_size + 1) * seg.num_blocks * self.channels;
        }
    }

    pub fn push<'push>(
        &mut self,
        input: impl Iterator<Item = &'push [f32]>,
        fdl_key: &'static str,
        sliding: bool,
    ) {
        let fdl_in = self.in_buffs.get_mut(fdl_key).unwrap();
        let fdl_len = fdl_in.0.len() / self.channels;

        for (fdl_in_channel, input_channel) in fdl_in.0.chunks_exact_mut(fdl_len).zip(input) {
            fdl_in_channel.copy_within(0..fdl_len - self.block_size, self.block_size);
            fdl_in_channel[0..self.block_size].copy_from_slice(input_channel);
        }

        fdl_in.1 += 1;

        for seg in &mut self.non_rt_segments {
            if fdl_in.1 % seg.avail == 0 {
                let fdl_prod = seg.real_prods.get_mut(fdl_key).unwrap();
                match fdl_prod.write_chunk(seg.block_size * self.channels) {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;
                        let s1_len = s1.len();

                        for fdl_in_channel in fdl_in.0.chunks_exact(fdl_len) {
                            let to_write = &fdl_in_channel[fdl_len - seg.block_size..fdl_len];
                            if s1_idx + seg.block_size < s1.len() {
                                s1[s1_idx..s1_idx + seg.block_size].copy_from_slice(to_write);

                                s1_idx += seg.block_size;
                            } else if s1_idx < s1.len() {
                                s1[s1_idx..s1_len].copy_from_slice(&to_write[0..s1_len - s1_idx]);
                                s2[0..seg.block_size - (s1.len() - s1_idx)]
                                    .copy_from_slice(&to_write[s1.len() - s1_idx..seg.block_size]);

                                s2_idx += seg.block_size - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                s2[s2_idx..s2_idx + seg.block_size].copy_from_slice(to_write);

                                s2_idx += seg.block_size;
                            }
                        }

                        w.commit_all();
                    }
                    Err(_) => todo!(),
                }
            }
        }

        self.rt_segment.push_chunk(
            fdl_key,
            fdl_in
                .0
                .chunks_exact(fdl_len)
                .map(|f| &f[0..self.block_size]),
            // TODO i dont really like that were passing this as an argument
            sliding,
        );

        // TODO logic for reseting counter after largest size so that
        // we dont run out of room in usize
    }

    // TODO this will get weird with more than two fdls
    pub fn process(
        &mut self,
        fdl_1: &'static str,
        fdl_2: &'static str,
    ) -> impl Iterator<Item = &[f32]> {
        // TODO i dont really like this process counter business, want to make sure its correct conceptually
        self.process_counter += 1;
        let out_len = self.out_buff.len();
        for out_channel in self.out_buff.chunks_exact_mut(out_len / self.channels) {
            let out_channel_len = out_channel.len();
            out_channel.copy_within(self.block_size..out_channel_len, 0);
            out_channel[out_channel_len - self.block_size..out_channel_len].fill(0.0);
        }
        for seg in &mut self.non_rt_segments {
            if self.process_counter % seg.avail == 0 {
                seg.process_prod.push((fdl_1, fdl_2)).unwrap();
            }
            if self.process_counter >= seg.offset
                && (self.process_counter - seg.offset) % seg.avail == 0
            {
                match seg.out_cons.read_chunk(seg.block_size * self.channels) {
                    Ok(r) => {
                        let (s1, s2) = r.as_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;

                        for out_channel in self.out_buff.chunks_exact_mut(out_len / self.channels) {
                            let to_write =
                                &mut out_channel[self.block_size..seg.block_size + self.block_size];

                            if s1_idx + seg.block_size < s1.len() {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1_idx + seg.block_size])
                                {
                                    *o += s / (seg.block_size / self.block_size) as f32;
                                }

                                s1_idx += seg.block_size;
                            } else if s1_idx < s1.len() {
                                for (o, s) in to_write[0..s1.len() - s1_idx]
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1.len()])
                                {
                                    *o += s / (seg.block_size / self.block_size) as f32;
                                }

                                for (o, s) in to_write[s1.len() - s1_idx..seg.block_size]
                                    .iter_mut()
                                    .zip(&s2[s2_idx..seg.block_size - (s1.len() - s1_idx)])
                                {
                                    *o += s / (seg.block_size / self.block_size) as f32;
                                }

                                s2_idx += seg.block_size - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s2[s2_idx..s2_idx + seg.block_size])
                                {
                                    *o += s / (seg.block_size / self.block_size) as f32;
                                }

                                s2_idx += seg.block_size;
                            }
                        }

                        r.commit_all();
                    }
                    Err(_) => todo!(),
                }
            }
        }

        let rt_out = self.rt_segment.process(fdl_1, fdl_2);
        for (new, out) in rt_out
            .chunks_exact(self.block_size)
            .zip(&mut self.out_buff.chunks_exact_mut(out_len / self.channels))
        {
            for (o, n) in out[0..self.block_size].iter_mut().zip(new) {
                *o += *n;
            }
        }

        self.out_buff
            .chunks_exact(self.out_buff.len() / self.channels)
            .map(|o| &o[0..self.block_size])
    }
}
