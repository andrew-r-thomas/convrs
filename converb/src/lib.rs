use convrs::{conv::Conv, upconv::UPConv};

use nih_plug::prelude::*;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Converb {
    params: Arc<ConverbParams>,
    left_upconv: Conv,
    right_upconv: Conv,
}

#[derive(Params)]
struct ConverbParams {
    /// The parameter's ID is used to identify the parameter in the wrappred plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for Converb {
    fn default() -> Self {
        let mut reader = match hound::WavReader::open(
            "/Users/andrewthomas/dev/diy/convrs/test_sounds/IRs/realylong.wav",
        ) {
            Ok(r) => r,
            Err(e) => {
                nih_log!("{}", e);
                panic!()
            }
        };

        let bits = reader.spec().bits_per_sample;
        let mut samples: Vec<f32> = Vec::with_capacity(reader.len() as usize);
        match reader.spec().sample_format {
            hound::SampleFormat::Float => {
                for s in reader.samples::<f32>() {
                    samples.push(s.unwrap());
                }
            }
            hound::SampleFormat::Int => match bits {
                8 => {
                    for s in reader.samples::<i8>() {
                        samples.push(s.unwrap() as f32 / i8::MAX as f32);
                    }
                }
                16 => {
                    for s in reader.samples::<i16>() {
                        samples.push(s.unwrap() as f32 / i16::MAX as f32);
                    }
                }
                24 => {
                    for s in reader.samples::<i32>() {
                        samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                32 => {
                    for s in reader.samples::<i32>() {
                        samples.push(s.unwrap() as f32 / i32::MAX as f32);
                    }
                }
                _ => panic!(),
            },
        };

        let left_upconv = Conv::new(128, &samples);
        let right_upconv = Conv::new(128, &samples);

        Self {
            params: Arc::new(ConverbParams::default()),
            left_upconv,
            right_upconv,
        }
    }
}

impl Default for ConverbParams {
    fn default() -> Self {
        Self {
            // This gain is stored as linear gain. NIH-plug comes with useful conversion functions
            // to treat these kinds of parameters as if we were dealing with decibels. Storing this
            // as decibels is easier to work with, but requires a conversion for every sample.
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    // This makes the range appear as if it was linear when displaying the values as
                    // decibels
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            // Because the gain parameter is stored as linear gain instead of storing the value as
            // decibels, we need logarithmic smoothing
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            // There are many predefined formatters we can use here. If the gain was stored as
            // decibels instead of as a linear gain value, we could have also used the
            // `.with_step_size(0.1)` function to get internal rounding.
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
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
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for (_size, block) in buffer.iter_blocks(128) {
            let mut channels = block.into_iter();
            let left_channel = channels.next().unwrap();
            let right_channel = channels.next().unwrap();
            let left_out = self.left_upconv.process_block(left_channel);
            let right_out = self.right_upconv.process_block(right_channel);

            left_channel.copy_from_slice(left_out);
            right_channel.copy_from_slice(right_out);
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
