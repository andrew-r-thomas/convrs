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
        let partition = &[(128, 22), (1024, 21), (8192, 23)];
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
                let (rt_prod, mut seg_cons) = RingBuffer::<&[f32]>::new(p.0 * 1000);
                // then a ring buffer for us to send result blocks
                // back to the real time thread
                let (mut seg_prod, rt_cons) = RingBuffer::<&[f32]>::new(p.0 * 1000);

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
                            match seg_cons.read_chunk(channels) {
                                Ok(r) => {
                                    let out = {
                                        let (s1, s2) = r.as_slices();
                                        upconv.process_block([s1, s2].concat())
                                    };
                                    r.commit_all();

                                    match seg_prod.write_chunk(p.0) {
                                        Ok(mut w) => {
                                            let (s1, s2) = w.as_mut_slices();
                                            s1.copy_from_slice(&out.into_iter()[0..s1.len()]);
                                            s2.copy_from_slice(&out[s1.len()..s1.len() + s2.len()]);
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
        let mut input_buffs =
            vec![
                vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * 2];
                channels
            ];
        let mut output_buffs =
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
        }
    }

    pub fn set_filter() {
        todo!()
    }

    pub fn process_block(
        &mut self,
        block: impl IntoIterator<Item = &'blocks [f32]>,
    ) -> impl IntoIterator<Item = &[f32]> {
        self.cycle_count += 1;

        self.input_buff
            .copy_within(self.block_size..self.buff_len, 0);
        self.input_buff[self.buff_len - self.block_size..self.buff_len].copy_from_slice(block);

        self.output_buff
            .copy_within(self.block_size..self.buff_len, 0);
        self.output_buff[self.buff_len - self.block_size..self.buff_len].fill(0.0);

        for segment in &mut self.non_rt_segments {
            // first we check if its time to send and recieve a new block
            if self.cycle_count % segment.avail == 0 {
                match segment.rt_prod.write_chunk(segment.block_size) {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();
                        let s1_len = s1.len();
                        let s2_len = s2.len();
                        for (f, s) in s1.iter_mut().zip(
                            &self.input_buff
                                [self.buff_len - s1_len - s2_len..self.buff_len - s2_len],
                        ) {
                            *f = *s;
                        }
                        for (f, s) in s2
                            .iter_mut()
                            .zip(&self.input_buff[self.buff_len - s2_len..self.buff_len])
                        {
                            *f = *s;
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
                match segment.rt_cons.read_chunk(segment.block_size) {
                    Ok(r) => {
                        let (s1, s2) = r.as_slices();
                        let s1_len = s1.len();
                        let s2_len = s2.len();
                        for (f, s) in s1
                            .iter()
                            .zip(&mut self.output_buff[self.block_size..s1_len + self.block_size])
                        {
                            *s += *f / (segment.block_size / self.block_size) as f32;
                        }
                        for (f, s) in s2.iter().zip(
                            &mut self.output_buff
                                [self.block_size + s1_len..self.block_size + s2_len + s1_len],
                        ) {
                            *s += *f / (segment.block_size / self.block_size) as f32;
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

        let rt_out = self.rt_segment.process_block(
            &mut self
                .input_buffs
                .iter()
                .map(|i| &i[self.buff_len - self.block_size..self.buff_len]),
        );
        for (new, out) in rt_out.into_iter().zip(&self.output_buffs) {
            for (n, o) in new.iter().zip(out) {
                *o += *n;
            }
        }

        self.output_buffs.iter().map(|o| &o[0..self.block_size])
    }
}
