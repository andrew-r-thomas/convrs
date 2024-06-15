use std::thread;

use realfft::num_complex::Complex;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::upconv::UPConv;

/*
NOTE

questions:
- does the filter block size for pushing need to be a certain size

    so the whole reason that we're doing this is because we can't do all
    these ffts every frame, so we should think about the sizing for the filter
    to be exactly the same as the signal

    so we definitely dont want to be ffting more than a segments block size
    of filter at a time

    which i think means the updates to the filter will have to come in the rt segments
    block size chunks at a time

    this doesnt mean that a frame of the game will have to only be one rt segment
    block size large though, because we will get through many blocks of audio
    per frame of graphics

    for example, say we are rendering at 30fps, and our block size is 128
    and our sample rate is 48000. one frame is rendered every 1/30 seconds
    which would be 12.5 128 sample blocks per frame

    this means our game size could cover as much as 1600 samples
    which is very large for the averages IR conversion, and very short for the
    spiral thing

    so for a 512 by 512 board, we would want to get one sample from ~163 pixels

- if its flexible^, how do we want to handle pushing
- for dynamically changing the size of the reverb,
  we will want a way to dynamically change how much we are convolving
  and basically not compute some amount of tail end at will.
  this will need to go across segments as well
*/

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
    filter_prod: Producer<Complex<f32>>,
    partition: (usize, usize),
}

impl MovingConv {
    pub fn new(channels: usize, partition: &[(usize, usize)]) -> Self {
        let rt_segment = UPConv::new(partition[0].0, None, channels, partition[0].1);

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

                let mut upconv = UPConv::new(p.0, None, channels, p.1);

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
        let filter_buff =
            vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * channels];

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

        for (filter_channel, chunk_channel) in self
            .filter_buff
            .chunks_exact_mut(self.partition.last().unwrap().0 * self.partition.last().unwrap().1)
            .zip(filter_chunk.chunks_exact(self.partition.first().unwrap().0))
        {}
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
                                s1[s1_idx..segment.partition.0].copy_from_slice(to_write);

                                s1_idx += segment.partition.0;
                            } else if s1_idx < s1.len() {
                                s1[s1_idx..s1_len].copy_from_slice(&to_write[0..s1_len - s1_idx]);
                                s2[0..segment.partition.0 - (s1.len() - s1_idx)].copy_from_slice(
                                    &to_write[s1.len() - s1_idx..segment.partition.0],
                                );

                                s2_idx += segment.partition.0 - (s1.len() - s1_idx);
                                s1_idx = s1.len();
                            } else {
                                s2[s2_idx..segment.partition.0].copy_from_slice(to_write);

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

        let rt_out = self.rt_segment.process_block(map);
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
