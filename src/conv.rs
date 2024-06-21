use realfft::num_complex::Complex;

use crate::scheduler::{FdlConfig, Scheduler};

pub struct Conv {
    scheduler: Scheduler,
}

// TODO yeah this is probably too much abstraction
impl Conv {
    pub fn new(partition: &[(usize, usize)], channels: usize) -> Self {
        let fdl_config = &[
            FdlConfig {
                name: "signal",
                complex_ringbuff: false,
                real_ringbuff: true,
                moving: true,
            },
            FdlConfig {
                name: "filter",
                complex_ringbuff: true,
                real_ringbuff: false,
                moving: false,
            },
        ];
        let scheduler = Scheduler::new(partition, fdl_config, channels);

        Self { scheduler }
    }

    pub fn set_filter(
        &mut self,
        // chunks are on the outside, then channels inside that, then block inside that
        new_filter: &[Complex<f32>],
    ) {
        self.scheduler.set("filter", new_filter);
    }

    pub fn process_block<'block>(
        &mut self,
        channel_blocks: impl Iterator<Item = &'block [f32]>,
    ) -> impl Iterator<Item = &[f32]> {
        self.scheduler.push(channel_blocks, "signal", true);

        self.scheduler.process("signal", "filter")
    }
}
