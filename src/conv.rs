use realfft::num_complex::Complex;
use std::thread;

use crate::upconv::UPConv;
use rtrb::{Consumer, Producer, RingBuffer};

pub struct Conv {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    buff_len: usize,
    input_buff: Vec<f32>,
    output_buff: Vec<f32>,
    cycle_count: usize,
    block_size: usize,
    partition: Vec<(usize, usize)>,
    channels: usize,
}

struct SegmentHandle {
    block_size: usize,
    offset: usize,
    avail: usize,
    rt_prod: Producer<f32>,
    rt_cons: Consumer<f32>,
    filter_prod: Producer<Complex<f32>>,
    partition: (usize, usize),
}

impl Conv {
    pub fn new(
        block_size: usize,
        starting_filter: &[Complex<f32>],
        partition: &[(usize, usize)],
        channels: usize,
    ) -> Self {
        let mut filter_index = 0;
        let first_part = &starting_filter[0..(partition[0].0 + 1) * partition[0].1 * channels];

        let rt_segment = UPConv::new(partition[0].0, Some(first_part), channels, partition[0].1);

        filter_index += (partition[0].0 + 1) * partition[0].1 * channels;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            let mut offset_samples = partition[0].0 * partition[0].1;
            for i in 1..partition.len() {
                let p = partition[i];
                // TODO figure out the correct ringbuf length based on the offset
                let (rt_prod, mut seg_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);
                let (mut seg_prod, rt_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);

                let (filter_prod, mut filter_cons) =
                    RingBuffer::<Complex<f32>>::new((p.0 + 1) * p.1 * channels * 2);

                let mut upconv = UPConv::new(
                    p.0,
                    Some(
                        &starting_filter[filter_index..filter_index + ((p.0 + 1) * p.1 * channels)],
                    ),
                    channels,
                    p.1,
                );

                filter_index += ((p.0 + 1) * p.1) * channels;

                thread::spawn(move || {
                    // TODO a raw loop i feel like is a really bad idea here
                    // so consider this pseudocode for now
                    loop {
                        if !filter_cons.is_empty() {
                            match filter_cons.read_chunk((p.0 + 1) * p.1 * channels) {
                                Ok(r) => {
                                    let (s1, s2) = r.as_slices();

                                    upconv.set_filter(&[s1, s2].concat());

                                    r.commit_all();
                                }
                                Err(_) => todo!(),
                            }
                        }
                        if !seg_cons.is_empty() {
                            match seg_cons.read_chunk(p.0 * channels) {
                                Ok(r) => {
                                    let (s1, s2) = r.as_slices();

                                    let out =
                                        upconv.process_block([s1, s2].concat().chunks_exact(p.0));

                                    match seg_prod.write_chunk(p.0 * channels) {
                                        Ok(mut w) => {
                                            let (w1, w2) = w.as_mut_slices();

                                            w1.copy_from_slice(&out[0..w1.len()]);
                                            w2.copy_from_slice(&out[w1.len()..w1.len() + w2.len()]);

                                            w.commit_all();
                                        }
                                        Err(_) => todo!(),
                                    }

                                    r.commit_all();
                                }
                                Err(_) => todo!(),
                            }
                        }
                    }
                });

                non_rt_segments.push(SegmentHandle {
                    avail: p.0 / block_size,
                    offset: offset_samples / block_size,
                    block_size: p.0,
                    rt_prod,
                    rt_cons,
                    filter_prod,
                    partition: p,
                });

                offset_samples += p.0 * p.1;
            }
        }

        // TODO this might be more buffer than we need,
        // we might need just the last block size plus the main block size
        let input_buff = vec![0.0; partition.last().unwrap().0 * channels];
        let output_buff = vec![0.0; partition.last().unwrap().0 * 2 * channels];

        let buff_len = partition.last().unwrap().0;

        Self {
            rt_segment,
            input_buff,
            output_buff,
            non_rt_segments,
            cycle_count: 0,
            block_size,
            buff_len,
            partition: Vec::from(partition),
            channels,
        }
    }

    pub fn update_filter(
        &mut self,
        // chunks are on the outside, then channels inside that, then block inside that
        new_filter: &[Complex<f32>],
    ) {
        let mut filter_index = 0;
        let first = &new_filter[0..(self.partition[0].0 + 1) * self.partition[0].1 * self.channels];
        self.rt_segment.set_filter(first);
        filter_index += (self.partition[0].0 + 1) * self.partition[0].1 * self.channels;

        for seg in self.non_rt_segments.iter_mut() {
            let filter_chunk = &new_filter[filter_index
                ..filter_index + ((seg.partition.0 + 1) * seg.partition.1 * self.channels)];
            match seg
                .filter_prod
                .write_chunk((seg.partition.0 + 1) * seg.partition.1 * self.channels)
            {
                Ok(mut w) => {
                    let (s1, s2) = w.as_mut_slices();

                    s1.copy_from_slice(&filter_chunk[0..s1.len()]);
                    s2.copy_from_slice(&filter_chunk[s1.len()..s1.len() + s2.len()]);

                    w.commit_all();
                }
                Err(_) => todo!(),
            }

            filter_index += (seg.partition.0 + 1) * seg.partition.1 * self.channels;
        }
    }

    pub fn process_block<'block>(
        &mut self,
        channel_blocks: impl Iterator<Item = &'block [f32]>,
    ) -> impl Iterator<Item = &[f32]> {
        // TODO reset this after big blocks, otherwise were gonna run out of space for usize
        self.cycle_count += 1;

        for ((in_channel, out_channel), block) in self
            .input_buff
            .chunks_exact_mut(self.buff_len)
            .zip(self.output_buff.chunks_exact_mut(self.buff_len * 2))
            .zip(channel_blocks)
        {
            in_channel.copy_within(self.block_size..self.buff_len, 0);
            in_channel[self.buff_len - self.block_size..self.buff_len].copy_from_slice(block);

            out_channel.copy_within(self.block_size..self.buff_len * 2, 0);
            out_channel[self.buff_len - self.block_size..self.buff_len].fill(0.0);
        }

        for segment in &mut self.non_rt_segments {
            // first we check if its time to send and recieve a new block
            if self.cycle_count % segment.avail == 0 {
                match segment
                    .rt_prod
                    .write_chunk(segment.block_size * self.channels)
                {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;
                        let s1_len = s1.len();

                        for in_channel in self.input_buff.chunks_exact(self.buff_len) {
                            let to_write =
                                &in_channel[self.buff_len - segment.block_size..self.buff_len];
                            if s1_idx + segment.block_size < s1.len() {
                                s1[s1_idx..s1_idx + segment.block_size].copy_from_slice(to_write);

                                s1_idx += segment.block_size;
                            } else if s1_idx < s1.len() {
                                s1[s1_idx..s1_len].copy_from_slice(&to_write[0..s1_len - s1_idx]);
                                s2[0..segment.block_size - (s1.len() - s1_idx)].copy_from_slice(
                                    &to_write[s1.len() - s1_idx..segment.block_size],
                                );

                                s2_idx += segment.block_size - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                s2[s2_idx..s2_idx + segment.block_size].copy_from_slice(to_write);

                                s2_idx += segment.block_size;
                            }
                        }

                        w.commit_all();
                    }
                    Err(_) => todo!(),
                }
            }

            if self.cycle_count >= segment.offset
                && (self.cycle_count - segment.offset) % segment.avail == 0
            {
                match segment
                    .rt_cons
                    .read_chunk(segment.block_size * self.channels)
                {
                    Ok(r) => {
                        let (s1, s2) = r.as_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;

                        for out_channel in self.output_buff.chunks_exact_mut(self.buff_len * 2) {
                            let to_write = &mut out_channel
                                [self.block_size..segment.block_size + self.block_size];

                            if s1_idx + segment.block_size < s1.len() {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1_idx + segment.block_size])
                                {
                                    *o += s / (segment.block_size / self.block_size) as f32;
                                }

                                s1_idx += segment.block_size;
                            } else if s1_idx < s1.len() {
                                for (o, s) in to_write[0..s1.len() - s1_idx]
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1.len()])
                                {
                                    *o += s / (segment.block_size / self.block_size) as f32;
                                }

                                for (o, s) in to_write[s1.len() - s1_idx..segment.block_size]
                                    .iter_mut()
                                    .zip(&s2[s2_idx..segment.block_size - (s1.len() - s1_idx)])
                                {
                                    *o += s / (segment.block_size / self.block_size) as f32;
                                }

                                s2_idx += segment.block_size - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s2[s2_idx..s2_idx + segment.block_size])
                                {
                                    *o += s / (segment.block_size / self.block_size) as f32;
                                }

                                s2_idx += segment.block_size;
                            }
                        }

                        r.commit_all();
                    }
                    Err(_) => todo!(),
                }
            }
        }

        let map = &mut self
            .input_buff
            .chunks_exact(self.buff_len)
            .map(|i| &i[self.buff_len - self.block_size..self.buff_len]);

        let rt_out = self.rt_segment.process_block(map);
        for (new, out) in rt_out
            .chunks_exact(self.block_size)
            .zip(&mut self.output_buff.chunks_exact_mut(self.buff_len * 2))
        {
            for (o, n) in out[0..self.block_size].iter_mut().zip(new) {
                *o += *n;
            }
        }

        self.output_buff
            .chunks_exact(self.buff_len * 2)
            .map(|o| &o[0..self.block_size])
    }
}
