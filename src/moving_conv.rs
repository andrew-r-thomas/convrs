use std::thread;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::upconv::UPConv;

/*
TODO
- for now we're just gonna write this, but we probably want to
  change the api design to just allow for this to be done in user
  space, since its unique, and good design anyway,

  see: https://www.youtube.com/watch?t=ZQ5_u8Lgvyk
*/

pub struct MovingConv {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    input_buff: Vec<f32>,
    output_buff: Vec<f32>,
    filter_buff: Vec<f32>,
    cycle_count: usize,
    filter_cycle_count: usize,
    partition: Vec<(usize, usize)>,
    channels: usize,
}

struct SegmentHandle {
    offset: usize,
    avail: usize,
    rt_prod: Producer<f32>,
    rt_cons: Consumer<f32>,
    filter_prod: Producer<f32>,
    partition: (usize, usize),
}

impl MovingConv {
    pub fn new(channels: usize, partition: &[(usize, usize)]) -> Self {
        let fdls = &["signal", "filter"];
        let rt_segment = UPConv::new(partition[0].0, channels, partition[0].1, fdls);

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            let mut offset_samples = partition[0].0 * partition[0].1;
            for i in 1..partition.len() {
                let p = partition[i];
                // TODO figure out the correct ringbuf length based on the offset
                let (rt_prod, mut seg_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);
                let (mut seg_prod, rt_cons) = RingBuffer::<f32>::new(p.0 * channels * 1000);

                let (filter_prod, mut filter_cons) =
                    RingBuffer::<f32>::new((p.0 + 1) * p.1 * channels * 2);

                let mut upconv = UPConv::new(p.0, channels, p.1, fdls);

                thread::spawn(move || {
                    // TODO a raw loop i feel like is a really bad idea here
                    // so consider this pseudocode for now
                    loop {
                        if !filter_cons.is_empty() {
                            match filter_cons.read_chunk(p.0 * channels) {
                                Ok(r) => {
                                    let (s1, s2) = r.as_slices();

                                    upconv.push_chunk(
                                        "filter",
                                        [s1, s2].concat().chunks_exact(p.0),
                                        false,
                                    );

                                    r.commit_all();
                                }
                                Err(_) => todo!(),
                            }
                        }
                        if !seg_cons.is_empty() {
                            match seg_cons.read_chunk(p.0 * channels) {
                                Ok(r) => {
                                    let (s1, s2) = r.as_slices();

                                    upconv.push_chunk(
                                        "signal",
                                        [s1, s2].concat().chunks_exact(p.0),
                                        true,
                                    );
                                    let out = upconv.process("signal", "filter");

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
                    avail: p.0 / partition[0].0,
                    offset: offset_samples / partition[0].0,
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
        let filter_buff = vec![0.0; partition.last().unwrap().0 * channels];

        Self {
            rt_segment,
            input_buff,
            output_buff,
            non_rt_segments,
            cycle_count: 0,
            filter_cycle_count: 0,
            partition: Vec::from(partition),
            filter_buff,
            channels,
        }
    }

    // for now we will just say that a filter chunk has to be a block size
    // but we might change this
    // so for now, filter_chunk is block_size * channels
    pub fn push_filter_chunk(&mut self, filter_chunk: &[f32]) {
        self.filter_cycle_count += 1;
        let block_size = self.partition.first().unwrap().0;
        let filter_channel_len = self.partition.last().unwrap().0;

        for (filter_channel, chunk_channel) in self
            .filter_buff
            .chunks_exact_mut(filter_channel_len)
            .zip(filter_chunk.chunks_exact(block_size))
        {
            filter_channel.copy_within(0..filter_channel.len() - block_size, block_size);

            filter_channel[0..block_size].copy_from_slice(chunk_channel);
        }

        for segment in &mut self.non_rt_segments {
            if self.filter_cycle_count % segment.avail == 0 {
                match segment
                    .filter_prod
                    .write_chunk(segment.partition.0 * self.channels)
                {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;
                        let s1_len = s1.len();

                        for filter_channel in self.filter_buff.chunks_exact(filter_channel_len) {
                            let to_write = &filter_channel[0..segment.partition.0];
                            if s1_idx + segment.partition.0 < s1.len() {
                                s1[s1_idx..s1_idx + segment.partition.0].copy_from_slice(to_write);

                                s1_idx += segment.partition.0;
                            } else if s1_idx < s1.len() {
                                s1[s1_idx..s1_len].copy_from_slice(&to_write[0..s1_len - s1_idx]);
                                s2[0..segment.partition.0 - (s1.len() - s1_idx)].copy_from_slice(
                                    &to_write[s1.len() - s1_idx..segment.partition.0],
                                );

                                s2_idx += segment.partition.0 - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                s2[s2_idx..s2_idx + segment.partition.0].copy_from_slice(to_write);

                                s2_idx += segment.partition.0;
                            }
                        }

                        w.commit_all();
                    }
                    Err(_) => todo!(),
                }
            }
        }

        self.rt_segment.push_chunk(
            "filter",
            self.filter_buff
                .chunks_exact(filter_channel_len)
                .map(|b| &b[0..block_size]),
            true,
        );
    }

    pub fn process_block<'block>(
        &mut self,
        channel_blocks: impl Iterator<Item = &'block [f32]>,
    ) -> impl Iterator<Item = &[f32]> {
        // TODO reset this after big blocks, otherwise were gonna run out of space for usize
        self.cycle_count += 1;
        let buff_len = self.partition.last().unwrap().0;
        let block_size = self.partition.first().unwrap().0;

        for ((in_channel, out_channel), block) in self
            .input_buff
            .chunks_exact_mut(buff_len)
            .zip(self.output_buff.chunks_exact_mut(buff_len * 2))
            .zip(channel_blocks)
        {
            in_channel.copy_within(block_size..buff_len, 0);
            in_channel[buff_len - block_size..buff_len].copy_from_slice(block);

            out_channel.copy_within(block_size..buff_len * 2, 0);
            out_channel[buff_len - block_size..buff_len].fill(0.0);
        }

        for segment in &mut self.non_rt_segments {
            // first we check if its time to send and recieve a new block
            if self.cycle_count % segment.avail == 0 {
                match segment
                    .rt_prod
                    .write_chunk(segment.partition.0 * self.channels)
                {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;
                        let s1_len = s1.len();

                        for in_channel in self.input_buff.chunks_exact(buff_len) {
                            let to_write = &in_channel[buff_len - segment.partition.0..buff_len];
                            if s1_idx + segment.partition.0 < s1.len() {
                                s1[s1_idx..s1_idx + segment.partition.0].copy_from_slice(to_write);

                                s1_idx += segment.partition.0;
                            } else if s1_idx < s1.len() {
                                s1[s1_idx..s1_len].copy_from_slice(&to_write[0..s1_len - s1_idx]);
                                s2[0..segment.partition.0 - (s1.len() - s1_idx)].copy_from_slice(
                                    &to_write[s1.len() - s1_idx..segment.partition.0],
                                );

                                s2_idx += segment.partition.0 - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                s2[s2_idx..s2_idx + segment.partition.0].copy_from_slice(to_write);

                                s2_idx += segment.partition.0;
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
                    .read_chunk(segment.partition.0 * self.channels)
                {
                    Ok(r) => {
                        let (s1, s2) = r.as_slices();

                        let mut s1_idx = 0;
                        let mut s2_idx = 0;

                        for out_channel in self.output_buff.chunks_exact_mut(buff_len * 2) {
                            let to_write =
                                &mut out_channel[block_size..segment.partition.0 + block_size];

                            if s1_idx + segment.partition.0 < s1.len() {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1_idx + segment.partition.0])
                                {
                                    *o += s / (segment.partition.0 / block_size) as f32;
                                }

                                s1_idx += segment.partition.0;
                            } else if s1_idx < s1.len() {
                                for (o, s) in to_write[0..s1.len() - s1_idx]
                                    .iter_mut()
                                    .zip(&s1[s1_idx..s1.len()])
                                {
                                    *o += s / (segment.partition.0 / block_size) as f32;
                                }

                                for (o, s) in to_write[s1.len() - s1_idx..segment.partition.0]
                                    .iter_mut()
                                    .zip(&s2[s2_idx..segment.partition.0 - (s1.len() - s1_idx)])
                                {
                                    *o += s / (segment.partition.0 / block_size) as f32;
                                }

                                s2_idx += segment.partition.0 - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                for (o, s) in to_write
                                    .iter_mut()
                                    .zip(&s2[s2_idx..s2_idx + segment.partition.0])
                                {
                                    *o += s / (segment.partition.0 / block_size) as f32;
                                }

                                s2_idx += segment.partition.0;
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
            .chunks_exact(buff_len)
            .map(|i| &i[buff_len - block_size..buff_len]);

        self.rt_segment.push_chunk("signal", map, true);
        let rt_out = self.rt_segment.process("signal", "filter");
        for (new, out) in rt_out
            .chunks_exact(block_size)
            .zip(&mut self.output_buff.chunks_exact_mut(buff_len * 2))
        {
            for (o, n) in out[0..block_size].iter_mut().zip(new) {
                *o += *n;
            }
        }

        self.output_buff
            .chunks_exact(buff_len * 2)
            .map(move |o| &o[0..block_size])
    }
}
