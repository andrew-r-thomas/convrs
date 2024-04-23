use nih_plug::nih_log;
use realfft::num_complex::Complex;
use std::thread;

use crate::upconv::UPConv;
use rtrb::{Consumer, Producer, RingBuffer};

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

// TODO think about where we want to store partition info
// right now its in a couple of places
struct SegmentHandle {
    block_size: usize,
    offset: usize,
    avail: usize,
    rt_prods: Vec<Producer<f32>>,
    rt_conss: Vec<Consumer<f32>>,
    filter_prods: Vec<Producer<Complex<f32>>>,
    message_prod: Producer<Message>,
    partition: (usize, usize),
}

enum Message {
    ProcessBlock,
    NewFilter,
}

impl Conv {
    pub fn new(
        block_size: usize,
        starting_filter: &[&[f32]],
        partition: &[(usize, usize)],
        channels: usize,
    ) -> Self {
        assert!(starting_filter.len() == channels);
        let mut filter_index = 0;

        let first_filter = vec![];
        for i in 0..channels {
            first_filter.push(&starting_filter[i][0..(partition[0].0 * partition[0].1)])
        }

        let rt_segment = UPConv::new(partition[0].0, &first_filter, channels, 4, partition[0].1);

        filter_index += partition[0].0 * partition[0].1;

        let mut non_rt_segments = vec![];
        if partition.len() > 1 {
            let mut offset_samples = partition[0].0 * partition[0].1;
            for i in 1..partition.len() {
                let p = partition[i];
                // TODO figure out the correct ringbuf length based on the offset
                let mut rt_prods = vec![];
                let mut rt_conss = vec![];
                let mut seg_conss = vec![];
                let mut seg_prods = vec![];
                let (filter_prod, mut filter_cons) =
                    RingBuffer::<Complex<f32>>::new((p.0 + 1) * p.1 * 2);
                let (message_prod, mut message_cons) = RingBuffer::<Message>::new(10);
                for _ in 0..channels {
                    let (rt_prod, seg_cons) = RingBuffer::<f32>::new(p.0 * 1000 * channels);
                    let (seg_prod, rt_cons) = RingBuffer::<f32>::new(p.0 * 1000 * channels);

                    rt_prods.push(rt_prod);
                    rt_conss.push(rt_cons);
                    seg_conss.push(seg_cons);
                    seg_prods.push(seg_prod);
                }

                let mut upconv = UPConv::new(
                    p.0,
                    &starting_filter
                        [filter_index..(p.0 * p.1 + filter_index).min(starting_filter.len())],
                    channels,
                    4,
                    p.1,
                );

                filter_index = (filter_index + (p.0 * p.1)).min(starting_filter.len());

                thread::spawn(move || {
                    // TODO a raw loop i feel like is a really bad idea here
                    // so consider this pseudocode for now
                    let mut channels_buff = vec![vec![0.0; p.0]; channels];

                    loop {
                        match message_cons.pop() {
                            Ok(m) => match m {
                                Message::NewFilter => match filter_cons.read_chunk(p.0 * p.1) {
                                    Ok(r) => {
                                        let (s1, s2) = r.as_slices();
                                        upconv.update_filter(&[s1, s2].concat());
                                        r.commit_all();
                                    }
                                    Err(_) => panic!(),
                                },
                                Message::ProcessBlock => {
                                    for (seg_cons, channel_buff) in
                                        seg_conss.iter_mut().zip(&mut channels_buff)
                                    {
                                        match seg_cons.read_chunk(p.0) {
                                            Ok(r) => {
                                                let (s1, s2) = r.as_slices();
                                                let total = [s1, s2].concat();
                                                channel_buff.copy_from_slice(total.as_slice());
                                                r.commit_all();
                                            }
                                            Err(e) => {
                                                // TODO logging
                                                nih_log!("{}", e);
                                                panic!();
                                            }
                                        }
                                    }

                                    let out = upconv
                                        .process_block(channels_buff.iter().map(|c| c.as_slice()));

                                    for (seg_prod, o) in seg_prods.iter_mut().zip(out) {
                                        match seg_prod.write_chunk(p.0) {
                                            Ok(mut w) => {
                                                let (s1, s2) = w.as_mut_slices();
                                                s1.copy_from_slice(&o[0..s1.len()]);
                                                s2.copy_from_slice(
                                                    &o[s1.len()..s1.len() + s2.len()],
                                                );
                                                w.commit_all();
                                            }
                                            Err(e) => {
                                                // TODO logging
                                                nih_log!(
                                            "error writing buff in segment with partition: {:?}",
                                            p
                                        );
                                                nih_log!("{}", e);
                                                panic!();
                                            }
                                        }
                                    }
                                }
                            },
                            Err(_) => {}
                        }
                    }
                });

                non_rt_segments.push(SegmentHandle {
                    avail: p.0 / block_size,
                    offset: offset_samples / block_size,
                    block_size: p.0,
                    rt_prods,
                    rt_conss,
                    filter_prod,
                    partition: p,
                    message_prod,
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

    pub fn update_filter<'filter>(
        &mut self,
        new_filter: impl IntoIterator<Item = &'filter [Complex<f32>]>,
    ) {
        let mut filter_iter = new_filter.into_iter();
        self.rt_segment.update_filter(filter_iter.next().unwrap());

        // we're relying on the ordering being correct here
        for (seg, filter_chunk) in self.non_rt_segments.iter_mut().zip(filter_iter) {
            match seg
                .filter_prod
                .write_chunk((seg.partition.0 + 1) * seg.partition.1)
            {
                Ok(mut w) => {
                    let (s1, s2) = w.as_mut_slices();
                    s1.fill(Complex { re: 0.0, im: 0.0 });
                    s2.fill(Complex { re: 0.0, im: 0.0 });

                    let s1_len = s1.len();
                    let s2_len = s2.len();

                    for (s, f) in s1
                        .iter_mut()
                        .zip(&filter_chunk[0..s1_len.min(filter_chunk.len())])
                    {
                        *s = *f;
                    }

                    for (s, f) in s2.iter_mut().zip(
                        &filter_chunk[s1_len.min(filter_chunk.len())
                            ..(s1_len + s2_len).min(filter_chunk.len())],
                    ) {
                        *s = *f;
                    }

                    w.commit_all();
                }
                Err(_) => {
                    todo!()
                }
            }

            match seg.message_prod.push(Message::NewFilter) {
                Ok(_) => {}
                Err(_) => panic!(),
            }
        }
    }

    pub fn process_block<'block>(
        &mut self,
        channel_blocks: impl IntoIterator<Item = &'block [f32]>,
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
                for (rt_prod, input) in segment.rt_prods.iter_mut().zip(&mut self.input_buffs) {
                    match rt_prod.write_chunk(segment.block_size) {
                        Ok(mut w) => {
                            let (s1, s2) = w.as_mut_slices();
                            let s1_len = s1.len();
                            let s2_len = s2.len();

                            s1.copy_from_slice(
                                &input[self.buff_len - s1_len - s2_len..self.buff_len - s2_len],
                            );
                            s2.copy_from_slice(&input[self.buff_len - s2_len..self.buff_len]);

                            w.commit_all();
                        }
                        Err(e) => {
                            // TODO logging
                            nih_log!(
                                "error writing to segment with block size: {:?}",
                                segment.block_size
                            );
                            nih_log!("{}", e);
                            panic!();
                        }
                    }
                }

                match segment.message_prod.push(Message::ProcessBlock) {
                    Ok(_) => {}
                    Err(_) => panic!(),
                }
            }

            if self.cycle_count >= segment.offset
                && (self.cycle_count - segment.offset) % segment.avail == 0
            {
                for (rt_cons, out) in segment.rt_conss.iter_mut().zip(&mut self.output_buffs) {
                    match rt_cons.read_chunk(segment.block_size) {
                        Ok(r) => {
                            let (s1, s2) = r.as_slices();
                            let s1_len = s1.len();
                            let s2_len = s2.len();
                            for (o, s) in out[self.block_size..self.block_size + s1_len]
                                .iter_mut()
                                .zip(s1)
                            {
                                *o += s / (segment.block_size / self.block_size) as f32;
                            }
                            for (o, s) in out
                                [self.block_size + s1_len..self.block_size + s1_len + s2_len]
                                .iter_mut()
                                .zip(s2)
                            {
                                *o += s / (segment.block_size / self.block_size) as f32;
                            }

                            r.commit_all();
                        }
                        Err(e) => {
                            // TODO  , again, good logging setup
                            nih_log!(
                                "error reading from segment with block size: {:?}",
                                segment.block_size
                            );
                            nih_log!("{}", e);
                            panic!();
                        }
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
