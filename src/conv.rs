use crossbeam::channel::bounded;
use std::thread;

use crate::{partition_table::PARTITIONS_1_128, upconv::UPConv};

pub struct Conv {
    rt_segment: UPConv,
    filter_len: usize,
    input_buff: Vec<f32>,
    output_buff: Vec<f32>,
}

impl Conv {
    pub fn new(block_size: usize, filter_len: usize) -> Self {
        let partition = PARTITIONS_1_128[filter_len % block_size];
        let mut workers = vec![];
        let (sender, receiver) = bounded::<WorkerMessage>(10);
        for p in &partition[1..] {
            let r = receiver.clone();
            let worker_handle = thread::spawn(move || {
                r.recv().unwrap();
                let upconv = UPConv::new(p.0, p.0 * p.1);
            });

            workers.push((worker_handle, sender.clone()));
        }

        let rt_segment = UPConv::new(partition[0].0, partition[0].1 * partition[0].0);

        let input_buff =
            Vec::with_capacity(partition.last().unwrap().0 * partition.last().unwrap().1);
        let output_buff =
            Vec::with_capacity(partition.last().unwrap().0 * partition.last().unwrap().1);

        Self {
            rt_segment,
            filter_len,
            input_buff,
            output_buff,
        }
    }

    pub fn set_filter() {}

    pub fn process_block(&mut self, block: &mut [f32]) {
        let rt_out = self.rt_segment.process_block(block);
    }
}

enum WorkerMessage {
    Process,
}
