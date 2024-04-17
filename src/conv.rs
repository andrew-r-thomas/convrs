use std::thread;

use crate::upconv::UPConv;
use rtrb::{Consumer, Producer, RingBuffer};
// use crate::{partition_table::PARTITIONS_1_128, upconv::UPConv};

// TODO move channels to here
// TODO sample rate conversions for filters
pub struct Conv {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    buff_len: usize,
    input_buffs: Vec<Vec<f32>>,
    output_buffs: Vec<Vec<f32>>,
    cycle_count: usize,
    block_size: usize,
    channels: usize,
}

struct SegmentHandle {
    block_size: usize,
    offset: usize,
    avail: usize,
    rt_prod: Producer<f32>,
    rt_cons: Consumer<f32>,
}

impl<'blocks> Conv {
    pub fn new(block_size: usize, filter: &[f32], channels: usize) -> Self {
        // TODO make this not hard coded
        // our filter len is 206400
        // our partition len total is 212736
        let partition = &[(128, 22), (1024, 21), (8192, 20)];
        let mut filter_index = 0;

        let mut rt_segment = UPConv::new(partition[0].0, partition[0].1 * partition[0].0, channels);
        rt_segment.set_filter(&filter[0..(partition[0].0 * partition[0].1)]);
        filter_index += partition[0].0 * partition[0].1;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            let mut offset_samples = partition[0].0 * partition[0].1;
            for p in &partition[1..] {
                // TODO figure out the correct ringbuf length based on the offset
                // first ring buffer for us so send new blocks
                // to the worker thread
                let (rt_prod, mut seg_cons) = RingBuffer::<f32>::new(p.0 * 1000 * channels);
                // then a ring buffer for us to send result blocks
                // back to the real time thread
                let (mut seg_prod, rt_cons) = RingBuffer::<f32>::new(p.0 * 1000 * channels);

                let mut upconv = UPConv::new(p.0, p.0 * p.1, channels);
                upconv.set_filter(
                    &filter[filter_index..(p.0 * p.1 + filter_index).min(filter.len())],
                );
                filter_index = (filter_index + (p.0 * p.1)).min(filter.len());

                thread::spawn(move || {
                    // TODO a raw loop i feel like is a really bad idea here
                    // so consider this pseudocode for now
                    loop {
                        if !seg_cons.is_empty() {
                            match seg_cons.read_chunk(p.0 * channels) {
                                Ok(r) => {
                                    // TODO
                                    // ok right now we are just righting the channels in a row
                                    // and repacking things to suit needs
                                    // definitely needs a refactor but should work probably
                                    let (s1, s2) = r.as_slices();
                                    let total = [s1, s2].concat();
                                    let slice = (0..channels).map(|i| &total[i..i + p.0]);
                                    let out = upconv.process_block(slice);
                                    r.commit_all();

                                    match seg_prod.write_chunk(p.0 * channels) {
                                        Ok(mut w) => {
                                            let (s1, s2) = w.as_mut_slices();
                                            let o: Vec<f32> =
                                                out.into_iter().flatten().map(|x| *x).collect();
                                            s1.copy_from_slice(&o[0..s1.len()]);
                                            s2.copy_from_slice(&o[s1.len()..s1.len() + s2.len()]);
                                            w.commit_all();
                                        }
                                        Err(e) => {
                                            // TODO logging
                                            println!(
                                                "error writing buff in segment with partition: {:?}",
                                                p
                                            );
                                            println!("{}", e);
                                            panic!();
                                        }
                                    }
                                }
                                Err(e) => {
                                    // TODO logging
                                    println!("{}", e);
                                    panic!();
                                }
                            }
                        }
                    }
                });

                non_rt_segments.push(SegmentHandle {
                    // TODO hard coding
                    avail: p.0 / 128,
                    offset: offset_samples / 128,
                    block_size: p.0,
                    rt_prod,
                    rt_cons,
                });

                offset_samples += p.0 * p.1;
            }
        }

        // TODO this might be more buffer than we need,
        // we might need just the last block size plus the main block size
        let input_buffs =
            vec![
                vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * 2];
                channels
            ];
        let output_buffs =
            vec![
                vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * 2];
                channels
            ];

        let buff_len = input_buffs.first().unwrap().len();

        Self {
            rt_segment,
            input_buffs,
            output_buffs,
            non_rt_segments,
            cycle_count: 0,
            block_size,
            buff_len,
            channels,
        }
    }

    pub fn set_filter() {
        todo!()
    }

    pub fn process_block(
        &mut self,
        channel_blocks: impl IntoIterator<Item = &'blocks [f32]>,
    ) -> impl IntoIterator<Item = &[f32]> {
        self.cycle_count += 1;
        let mut blocks = channel_blocks.into_iter();

        for i in 0..self.channels {
            let buff = &mut self.input_buffs[i];
            let out = &mut self.output_buffs[i];
            let block = blocks.next().unwrap();
            buff.copy_within(self.block_size..self.buff_len, 0);
            buff[self.buff_len - self.block_size..self.buff_len].copy_from_slice(block);

            out.copy_within(self.block_size..self.buff_len, 0);
            out[self.buff_len - self.block_size..self.buff_len].fill(0.0);
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
                        let s1_len = s1.len();
                        let mut i = 0;
                        for channel in 0..self.channels {
                            let to_write = &self.input_buffs[channel]
                                [self.buff_len - segment.block_size..self.buff_len];

                            for w in to_write {
                                if i < s1_len {
                                    s1[i] = *w;
                                } else {
                                    s2[i] = *w;
                                }

                                i += 1;
                            }
                        }

                        w.commit_all();
                    }
                    Err(e) => {
                        // TODO logging
                        println!(
                            "error writing to segment with block size: {:?}",
                            segment.block_size
                        );
                        println!("{}", e);
                        panic!();
                    }
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
                        let s1_len = s1.len();
                        let mut i = 0;
                        for channel in 0..self.channels {
                            let to_read = &mut self.output_buffs[channel]
                                [self.block_size..self.block_size + segment.block_size];

                            for r in to_read {
                                if i < s1_len {
                                    *r += s1[i] / (segment.block_size / self.block_size) as f32;
                                } else {
                                    *r += s2[i] / (segment.block_size / self.block_size) as f32;
                                }
                                i += 1;
                            }
                        }

                        r.commit_all();
                    }
                    Err(e) => {
                        // TODO  , again, good logging setup
                        println!(
                            "error reading from segment with block size: {:?}",
                            segment.block_size
                        );
                        println!("{}", e);
                        panic!();
                    }
                }
            }
        }

        let map = &mut self
            .input_buffs
            .iter()
            .map(|i| &i[self.buff_len - self.block_size..self.buff_len]);

        let rt_out = self.rt_segment.process_block(map);
        for (new, out) in rt_out.into_iter().zip(&mut self.output_buffs) {
            for (o, n) in out.iter_mut().zip(new) {
                *o += *n;
            }
        }

        self.output_buffs.iter().map(|o| o.as_slice())
    }
}
