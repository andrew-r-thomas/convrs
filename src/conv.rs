use std::thread;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::{partition_table::PARTITIONS_1_128, upconv::UPConv};

pub struct Conv {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    input_buff: Vec<f32>,
    output_buff: Vec<f32>,
    cycle_count: usize,
    block_size: usize,
}

struct SegmentHandle {
    cycles_per_block: usize,
    block_size: usize,
    rt_prod: Producer<f32>,
    rt_cons: Consumer<f32>,
}

impl Conv {
    pub fn new(block_size: usize, filter_len: usize, filter: &[f32]) -> Self {
        // TODO might need some rounding here
        // let partition = PARTITIONS_1_128[filter_len / block_size];
        let partition = &[(128, 22), (1024, 21), (8192, 8)];
        let mut filter_index = 0;

        let mut rt_segment = UPConv::new(partition[0].0, partition[0].1 * partition[0].0);
        rt_segment.set_filter(&filter[0..(partition[0].0 * partition[0].1)]);
        filter_index += partition[0].0 * partition[0].1;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            for p in &partition[1..] {
                // first ring buffer for us so send new blocks
                // to the worker thread
                let (mut rt_prod, mut seg_cons) = RingBuffer::<f32>::new(p.0 * 2);
                // then a ring buffer for us to send result blocks
                // back to the real time thread
                let (mut seg_prod, rt_cons) = RingBuffer::<f32>::new(p.0 * 2);

                // fill both of our ring buffers with 0s
                // so we can pop whenever we push
                match rt_prod.write_chunk(p.0) {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();
                        s1.fill(0.0);
                        s2.fill(0.0);
                        w.commit_all();
                    }
                    Err(e) => {
                        // TODO make a better logging system
                        // base it off of the nih log logging traits
                        // so that you can use it with that logger easily
                        println!("{}", e);
                        panic!();
                    }
                }
                match seg_prod.write_chunk(p.0) {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();
                        s1.fill(0.0);
                        s2.fill(0.0);
                        w.commit_all();
                    }
                    Err(e) => {
                        // TODO make a better logging system
                        // base it off of the nih log logging traits
                        // so that you can use it with that logger easily
                        println!("{}", e);
                        panic!();
                    }
                }
                let mut upconv = UPConv::new(p.0, p.0 * p.1);
                upconv.set_filter(&filter[filter_index..(p.0 * p.1).min(filter.len())]);
                filter_index += p.0 * p.1;

                thread::spawn(move || {
                    // TODO a raw loop i feel like is a really bad idea here
                    // so consider this pseudocode for now
                    // we are also over working bc we fill everything
                    // with zeros first, but it makes the code a lot more
                    // simple so it could be worth it at least for now
                    loop {
                        if !seg_cons.is_empty() {
                            match seg_cons.read_chunk(p.0) {
                                Ok(r) => {
                                    let out = {
                                        let (s1, s2) = r.as_slices();
                                        upconv.process_block([s1, s2].concat().as_mut_slice())
                                    };
                                    r.commit_all();

                                    match seg_prod.write_chunk(p.0) {
                                        Ok(mut w) => {
                                            let (s1, s2) = w.as_mut_slices();
                                            // TODO change all the other ones of these to copies
                                            s1.copy_from_slice(&out[0..s1.len()]);
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
                    cycles_per_block: p.0 / block_size,
                    block_size: p.0,
                    rt_prod,
                    rt_cons,
                });
            }
        }

        let mut input_buff: Vec<f32> =
            vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1];
        let mut output_buff: Vec<f32> =
            vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1];

        input_buff.fill(0.0);
        output_buff.fill(0.0);

        Self {
            rt_segment,
            input_buff,
            output_buff,
            non_rt_segments,
            cycle_count: 0,
            block_size,
        }
    }

    pub fn set_filter() {
        todo!()
    }

    pub fn process_block(&mut self, block: &mut [f32]) -> &[f32] {
        self.cycle_count += 1;

        let mid = self.input_buff.len() - self.block_size;
        self.input_buff.rotate_left(mid);
        self.input_buff[0..self.block_size].copy_from_slice(block);

        for segment in &mut self.non_rt_segments {
            // first we check if its time to send and recieve a new block
            if self.cycle_count % segment.cycles_per_block == 0 {
                match segment.rt_cons.read_chunk(segment.block_size) {
                    Ok(r) => {
                        let (s1, s2) = r.as_slices();
                        let s1_len = s1.len();
                        let s2_len = s2.len();
                        for (f, s) in s1.iter().zip(&mut self.output_buff[0..s1_len]) {
                            *s += *f;
                        }
                        for (f, s) in s2.iter().zip(&mut self.output_buff[0..s2_len]) {
                            *s += f;
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

                match segment.rt_prod.write_chunk(segment.block_size) {
                    Ok(mut w) => {
                        let (s1, s2) = w.as_mut_slices();
                        let s1_len = s1.len();
                        let s2_len = s2.len();
                        for (f, s) in s1.iter_mut().zip(&self.input_buff[0..s1_len]) {
                            *f = *s;
                        }
                        for (f, s) in s2.iter_mut().zip(&self.input_buff[0..s2_len]) {
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
        }

        let rt_out = self.rt_segment.process_block(block);
        for (n, o) in rt_out.iter().zip(&mut self.output_buff[0..self.block_size]) {
            *o += *n;
        }

        &self.output_buff[0..self.block_size]
    }
}
