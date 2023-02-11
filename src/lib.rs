use nih_plug::prelude::*;
use std::sync::{Arc};
/// The time it takes for the peak meter to decay by 12 dB after switching to complete silence.
const PEAK_METER_DECAY_MS: f64 = 150.0;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
pub struct NihSampler {
    params: Arc<NihSamplerParams>,

    /// Needed to normalize the peak meter's response based on the sample rate.
    peak_meter_decay_weight: f32,
    /// The current data for the peak meter. This is stored as an [`Arc`] so we can share it between
    /// the GUI and the audio processing parts. If you have more state to share, then it's a good
    /// idea to put all of that in a struct behind a single `Arc`.
    ///
    /// This is stored as voltage gain.
    pub playing_samples: Vec<PlayingSample>,
}

#[derive(Params)]
struct NihSamplerParams {
    #[id = "gain"]
    pub gain: FloatParam,
}

impl Default for NihSampler {
    fn default() -> Self {
        Self {
            params: Arc::new(NihSamplerParams::default()),
            playing_samples: vec![],
            peak_meter_decay_weight: 1.0,
        }
    }
}

impl Default for NihSamplerParams {
    fn default() -> Self {
        Self {
            gain: FloatParam::new(
                "Gain",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-30.0),
                    max: util::db_to_gain(30.0),
                    factor: FloatRange::gain_skew_factor(-30.0, 30.0),
                },
            )
            .with_smoother(SmoothingStyle::Logarithmic(50.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_gain_to_db(2))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),
        }
    }
}

impl Plugin for NihSampler {
    const NAME: &'static str = "Sampler Demo";
    const VENDOR: &'static str = "Moist Plugins GmbH";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "info@example.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const DEFAULT_INPUT_CHANNELS: u32 = 2;
    const DEFAULT_OUTPUT_CHANNELS: u32 = 2;
    
    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn accepts_bus_config(&self, config: &BusConfig) -> bool {
        // This can output to any number of channels, but it doesn't take any audio inputs
        config.num_input_channels == 0 && config.num_output_channels > 0
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // After `PEAK_METER_DECAY_MS` milliseconds of pure silence, the peak meter's value should
        // have dropped by 12 dB
        self.peak_meter_decay_weight = 0.25f64
            .powf((buffer_config.sample_rate as f64 * PEAK_METER_DECAY_MS / 1000.0).recip())
            as f32;

        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();
        for (sample_id, channel_samples) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn {
                        timing,
                        voice_id,
                        channel,
                        note,
                        velocity,
                    } => {
                        self.playing_samples
                            .push(PlayingSample::new());
                    }
                    _ => (),
                }

                next_event = context.next_event();
            }

            for sample in channel_samples {
                for playing_sample in &mut self.playing_samples {
                    *sample += playing_sample.get_next_sample();
                }

                self.playing_samples.retain(|e| !e.should_be_removed());
            }
        }

        ProcessStatus::Normal
    }
}


pub struct PlayingSample {
    data: Vec<f32>,
    current_sample_index: usize,
}

const INPUT_SAMPLE: &[u8] = include_bytes!("sample.wav");


pub fn load_wav() -> Vec<f32> {
    let mut reader = hound::WavReader::new(INPUT_SAMPLE).unwrap();
    let spec = reader.spec();
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.unwrap_or_default())
            .collect::<Vec<_>>(),

        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.unwrap_or_default() as f32 * 256.0 / i32::MAX as f32)
            .collect::<Vec<_>>(),
    };

    samples
}

impl PlayingSample {
    pub fn new() -> Self {
        Self {
            data: load_wav(),
            current_sample_index: 0,
        }
    }

    pub fn get_next_sample(&mut self) -> f32 {
        let sample = self.data[self.current_sample_index];
        self.current_sample_index += 1;
        sample
    }

    pub fn should_be_removed(&self) -> bool {
        self.current_sample_index >= self.data.len()
    }
}

impl ClapPlugin for NihSampler {
    const CLAP_ID: &'static str = "com.moist-plugins-gmbh.gain-gui-vizia";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A smoothed gain parameter example plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for NihSampler {
    const VST3_CLASS_ID: [u8; 16] = *b"GainGuiVIIIZIAAA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Drum,
        Vst3SubCategory::Sampler,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(NihSampler);
nih_export_vst3!(NihSampler);