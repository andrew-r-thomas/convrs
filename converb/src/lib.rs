pub mod editor;

use convrs::{helpers::process_filter, upconv::UPConv};

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use num::Complex;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Converb {
    params: Arc<ConverbParams>,
    conv: UPConv,
    filter_1: Vec<Complex<f32>>,
    filter_2: Vec<Complex<f32>>,
    is_filter_1: bool,
}

#[derive(Params)]
struct ConverbParams {
    #[id = "filter 1"]
    filter_1: BoolParam,

    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Converb {
    fn default() -> Self {
        let mut reader_1 = match hound::WavReader::open(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short.wav",
        ) {
            Ok(r) => r,
            Err(e) => {
                nih_log!("{}", e);
                panic!()
            }
        };

        let bits_1 = reader_1.spec().bits_per_sample;
        let mut samples_1: Vec<f32> = Vec::with_capacity(reader_1.len() as usize);
        match reader_1.spec().sample_format {
            hound::SampleFormat::Float => {
                for s in reader_1.samples::<f32>() {
                    samples_1.push(s.unwrap());
                }
            }
            hound::SampleFormat::Int => match bits_1 {
                8 => {
                    for s in reader_1.samples::<i8>() {
                        samples_1.push(s.unwrap() as f32 / i8::MAX as f32);
                    }
                }
                16 => {
                    for s in reader_1.samples::<i16>() {
                        samples_1.push(s.unwrap() as f32 / i16::MAX as f32);
                    }
                }
                24 => {
                    for s in reader_1.samples::<i32>() {
                        samples_1.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                32 => {
                    for s in reader_1.samples::<i32>() {
                        samples_1.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                _ => panic!(),
            },
        };

        let mut reader_2 = match hound::WavReader::open(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/short2.wav",
        ) {
            Ok(r) => r,
            Err(e) => {
                nih_log!("{}", e);
                panic!()
            }
        };

        let bits_2 = reader_2.spec().bits_per_sample;
        let mut samples_2: Vec<f32> = Vec::with_capacity(reader_2.len() as usize);
        match reader_2.spec().sample_format {
            hound::SampleFormat::Float => {
                for s in reader_2.samples::<f32>() {
                    samples_2.push(s.unwrap());
                }
            }
            hound::SampleFormat::Int => match bits_2 {
                8 => {
                    for s in reader_2.samples::<i8>() {
                        samples_2.push(s.unwrap() as f32 / i8::MAX as f32);
                    }
                }
                16 => {
                    for s in reader_2.samples::<i16>() {
                        samples_2.push(s.unwrap() as f32 / i16::MAX as f32);
                    }
                }
                24 => {
                    for s in reader_2.samples::<i32>() {
                        samples_2.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                32 => {
                    for s in reader_2.samples::<i32>() {
                        samples_2.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                _ => panic!(),
            },
        };

        let conv = UPConv::new(128, samples_1.len().min(samples_2.len()), &samples_1, 2);
        let filter_1_spectrums = process_filter(
            &samples_1,
            &[(
                128,
                (samples_1.len().div_ceil(128)).max(samples_2.len().div_ceil(128)),
            )],
        );
        let filter_2_spectrums = process_filter(
            &samples_2,
            &[(
                128,
                (samples_1.len().div_ceil(128)).max(samples_2.len().div_ceil(128)),
            )],
        );

        Self {
            params: Arc::new(ConverbParams::default()),
            conv,
            filter_1: filter_1_spectrums.into_iter().flatten().collect(),
            filter_2: filter_2_spectrums.into_iter().flatten().collect(),
            is_filter_1: true,
        }
    }
}

impl Default for ConverbParams {
    fn default() -> Self {
        Self {
            filter_1: BoolParam::new("Filter 1", true),
            editor_state: editor::default_state(),
        }
    }
}

impl Plugin for Converb {
    const NAME: &'static str = "Converb";
    const VENDOR: &'static str = "Andrew Thomas";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "andrew.r.j.thomas@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone(), self.params.editor_state.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.params.filter_1.value() != self.is_filter_1 {
            if self.params.filter_1.value() {
                self.conv.update_filter(&self.filter_1);
            } else {
                self.conv.update_filter(&self.filter_2);
            }

            self.is_filter_1 = self.params.filter_1.value();
        }

        for (_size, mut block) in buffer.iter_blocks(128) {
            let map = block.iter_mut().map(|b| &*b);
            let out = self.conv.process_block(map.into_iter());
            for (b, o) in block.iter_mut().zip(out) {
                for (bb, oo) in b.iter_mut().zip(o) {
                    *bb = *oo;
                }
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Converb {
    const CLAP_ID: &'static str = "com.your-domain.converb";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("a simple convolution reverb");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Converb {
    const VST3_CLASS_ID: [u8; 16] = *b"Exactly16Chars!!";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

// nih_export_clap!(Converb);
nih_export_vst3!(Converb);
