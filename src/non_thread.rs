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
    cycles_per_block: usize,
    block_size: usize,
    pub conv: UPConv,
}

impl NoThreadConv {
    pub fn new(block_size: usize, filter_len: usize, filter: &[f32]) -> Self {
        // TODO might need some rounding here
        // let partition = PARTITIONS_1_128[filter_len / block_size];
        // our filter len is 206400
        // our partition len total is 212864
        let partition = &[(128, 7), (1024, 15), (16384, 12)];
        let mut filter_index = 0;

        let mut rt_segment = UPConv::new(partition[0].0, partition[0].1 * partition[0].0);
        rt_segment.set_filter(&filter[0..(partition[0].0 * partition[0].1)]);
        filter_index += partition[0].0 * partition[0].1;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            for p in &partition[1..] {
                // NOTE ok so, maybe we are getting the doubling up sound because our
                // individual filter guys have wierd fits, like the ends are padded with zeros
                // and so theres a dip at each partition point
                // TODO we might need to do the filter convolution all at once, and give the individual
                // upconvs the spectrum directly
                let mut upconv = UPConv::new(p.0, p.0 * p.1);
                upconv
                    .set_filter(&filter[filter_index..(p.0 * p.1 + filter_index).min(filter_len)]);
                filter_index = (filter_index + (p.0 * p.1)).min(filter_len);
                non_rt_segments.push(SegmentHandle {
                    cycles_per_block: p.0 / block_size,
                    block_size: p.0,
                    conv: upconv,
                })
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

    pub fn process_block(&mut self, block: &mut [f32]) -> &[f32] {
        self.cycle_count += 1;
        self.input_buff
            .copy_within(self.buff_len - self.block_size..self.buff_len, 0);
        self.input_buff[self.buff_len - self.block_size..self.buff_len].copy_from_slice(block);

        self.output_buff
            .copy_within(self.block_size..self.buff_len, 0);
        self.output_buff[self.buff_len - self.block_size..self.buff_len].fill(0.0);

        for segment in &mut self.non_rt_segments {
            // first we check if its time to send and recieve a new block
            if self.cycle_count % segment.cycles_per_block == 0 {
                let out = segment.conv.process_block(
                    &mut self.input_buff[self.buff_len - segment.block_size..self.buff_len],
                );

                for (s, o) in out
                    .iter()
                    .zip(&mut self.output_buff[0..segment.block_size + self.block_size])
                {
                    *o += s;
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
