use std::collections::VecDeque;

use crate::upconv::UPConv;
// use crate::{partition_table::PARTITIONS_1_128, upconv::UPConv};

pub struct NoThreadConv {
    rt_segment: UPConv,
    non_rt_segments: Vec<SegmentHandle>,
    buff_len: usize,
    input_buff: Vec<f32>,
    output_buff: Vec<f32>,
    cycle_count: usize,
    block_size: usize,
}

struct SegmentHandle {
    avail: usize,
    block_size: usize,
    offset: usize,
    buff: VecDeque<f32>,
    pub conv: UPConv,
}

impl NoThreadConv {
    pub fn new(block_size: usize, filter: &[f32]) -> Self {
        // TODO make this not hard coded
        // our filter len is 206400
        // our partition len total is 212736
        let partition = &[(128, 22), (1024, 21), (8192, 23)];
        let mut filter_index = 0;

        let mut rt_segment = UPConv::new(partition[0].0, partition[0].1 * partition[0].0);
        rt_segment.set_filter(&filter[0..(partition[0].0 * partition[0].1)]);
        filter_index += partition[0].0 * partition[0].1;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            let mut offset_samples = partition[0].0 * partition[0].1;
            for p in &partition[1..] {
                let mut upconv = UPConv::new(p.0, p.0 * p.1);
                upconv.set_filter(
                    &filter[filter_index..(p.0 * p.1 + filter_index).min(filter.len())],
                );
                filter_index = (filter_index + (p.0 * p.1)).min(filter.len());
                // TODO fix hard coding to 128
                non_rt_segments.push(SegmentHandle {
                    avail: p.0 / 128,
                    buff: VecDeque::new(),
                    offset: offset_samples / 128,
                    block_size: p.0,
                    conv: upconv,
                });

                offset_samples += p.0 * p.1;
            }
        }

        let mut input_buff: Vec<f32> =
            vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * 2];
        let mut output_buff: Vec<f32> =
            vec![0.0; partition.last().unwrap().0 * partition.last().unwrap().1 * 2];

        input_buff.fill(0.0);
        output_buff.fill(0.0);
        let buff_len = input_buff.len();

        Self {
            rt_segment,
            input_buff,
            buff_len,
            output_buff,
            non_rt_segments,
            cycle_count: 0,
            block_size,
        }
    }

    pub fn set_filter() {
        todo!()
    }

    pub fn process_block(&mut self, block: &[f32]) -> &[f32] {
        self.cycle_count += 1;
        self.input_buff
            .copy_within(self.block_size..self.buff_len, 0);
        self.input_buff[self.buff_len - self.block_size..self.buff_len].copy_from_slice(block);

        self.output_buff
            .copy_within(self.block_size..self.buff_len, 0);
        self.output_buff[self.buff_len - self.block_size..self.buff_len].fill(0.0);

        for segment in &mut self.non_rt_segments {
            // this if statement checks if we have more data to send to the segment
            if self.cycle_count % segment.avail == 0 {
                let out = segment.conv.process_block(
                    &mut self.input_buff[self.buff_len - segment.block_size..self.buff_len],
                );
                segment.buff.extend(out);
            }
            if self.cycle_count >= segment.offset
                && (self.cycle_count - segment.offset) % segment.avail == 0
            {
                for o in
                    // deadline is one block ahead of the current one
                    &mut self.output_buff
                        [self.block_size..self.block_size + segment.block_size]
                {
                    // gain adjustment based on block size
                    let s = segment.buff.pop_front().unwrap();
                    *o += s / (segment.block_size / self.block_size) as f32;
                }
            }
        }

        let rt_out = self
            .rt_segment
            .process_block(&mut self.input_buff[self.buff_len - self.block_size..self.buff_len]);
        for (n, o) in rt_out.iter().zip(&mut self.output_buff[0..self.block_size]) {
            *o += *n;
        }

        &self.output_buff[0..self.block_size]
    }
}
