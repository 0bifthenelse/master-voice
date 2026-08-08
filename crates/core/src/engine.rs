use crate::config::Config;
use crate::error::Error;
use master_voice_linguistics::lang::Language;
use master_voice_linguistics::overrides::Overrides;
use master_voice_synth::SynthOptions;

pub struct SpeakRequest {
    pub text: String,
    pub language: Option<Language>,
    pub interrupt: bool,
}

pub struct SpeakOutcome {
    pub language: Language,
    pub duration_s: f32,
    pub sample_count: usize,
    pub synth_ms: f64,
}

#[derive(Clone)]
pub struct EngineSettings {
    pub language: Option<Language>,
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub robotic_depth: f32,
}

impl EngineSettings {
    pub fn from_config(config: &Config) -> Self {
        let language = config.language.as_deref().and_then(Language::from_code);
        Self {
            language,
            rate: config.rate.unwrap_or(1.0).clamp(0.5, 2.0),
            pitch: config.pitch.unwrap_or(1.0).clamp(0.5, 1.5),
            volume: config.volume.unwrap_or(1.0).clamp(0.0, 2.0),
            robotic_depth: config.robotic_depth.unwrap_or(0.55).clamp(0.0, 1.0),
        }
    }

    pub fn synth_options(&self) -> SynthOptions {
        SynthOptions {
            rate: self.rate,
            pitch: self.pitch,
            volume: self.volume,
            robotic_depth: self.robotic_depth,
        }
    }
}

pub fn overrides_from_config(config: &Config) -> Overrides {
    match &config.overrides {
        Some(table) => Overrides::from_toml_table(table),
        None => Overrides::default(),
    }
}

pub fn synthesize_text(
    text: &str,
    explicit_language: Option<Language>,
    settings: &EngineSettings,
    overrides: &Overrides,
) -> Result<(Language, master_voice_synth::AudioBuffer, f64), Error> {
    let started = std::time::Instant::now();
    let utterance = master_voice_linguistics::phonemize(
        text,
        explicit_language.or(settings.language),
        overrides,
    )?;
    let buffer = master_voice_synth::synthesize(&utterance.phonemes, &settings.synth_options());
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    Ok((utterance.language, buffer, elapsed))
}

/// Chunked synthesis: the renderer and post-chain state persist across
/// chunks, so consecutive chunks concatenate sample-continuously (Step 7b).
pub struct StreamSynth {
    renderer: master_voice_synth::klatt::Renderer,
    post: master_voice_synth::dsp::PostState,
    opts: SynthOptions,
    first: bool,
}

impl StreamSynth {
    pub fn new(settings: &EngineSettings) -> Self {
        let opts = settings.synth_options();
        Self {
            renderer: master_voice_synth::klatt::Renderer::new(opts.robotic_depth),
            post: master_voice_synth::dsp::PostState::default(),
            opts,
            first: true,
        }
    }

    /// Phonemize + render one chunk of text. `last` marks the final chunk
    /// of the utterance (drives fades and the prosody finality flags).
    pub fn chunk(
        &mut self,
        text: &str,
        language: Option<Language>,
        overrides: &Overrides,
        last: bool,
    ) -> Result<(Language, Vec<f32>, f64), Error> {
        let started = std::time::Instant::now();
        let utterance = master_voice_linguistics::phonemize(text, language, overrides)?;
        let pos = master_voice_synth::ChunkPos {
            first: self.first,
            last,
        };
        let frames =
            master_voice_synth::prosody::build_frames_chunk(&utterance.phonemes, &self.opts, pos);
        let mut samples = self.renderer.render(&frames);
        master_voice_synth::dsp::post_chain(
            &mut samples,
            self.opts.robotic_depth,
            &mut self.post,
            pos,
        );
        master_voice_synth::dsp::apply_volume(&mut samples, self.opts.volume);
        self.first = false;
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        Ok((utterance.language, samples, elapsed))
    }
}

pub fn speak(
    request: &SpeakRequest,
    settings: &EngineSettings,
    overrides: &Overrides,
    controller: &master_voice_audio::PlaybackController,
) -> Result<SpeakOutcome, Error> {
    let (language, buffer, synth_ms) =
        synthesize_text(&request.text, request.language, settings, overrides)?;
    let sample_count = buffer.samples.len();
    let duration_s = sample_count as f32 / buffer.sample_rate as f32;
    let _receiver = controller.push(0, 0, buffer.samples, buffer.sample_rate, request.interrupt)?;
    Ok(SpeakOutcome {
        language,
        duration_s,
        sample_count,
        synth_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_from_default_config() {
        let settings = EngineSettings::from_config(&Config::default());
        assert_eq!(settings.rate, 1.0);
        assert_eq!(settings.robotic_depth, 0.55);
        assert_eq!(settings.language, None);
    }

    #[test]
    fn synthesizes_text_without_audio() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        let (language, buffer, _) = synthesize_text("hello", None, &settings, &overrides).unwrap();
        assert_eq!(language, Language::English);
        assert!(!buffer.samples.is_empty());
        assert!(buffer.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_text_errors() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        assert!(synthesize_text("", None, &settings, &overrides).is_err());
    }

    #[test]
    fn explicit_french_wins() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        let (language, _, _) = synthesize_text(
            "Bonjour le monde",
            Some(Language::French),
            &settings,
            &overrides,
        )
        .unwrap();
        assert_eq!(language, Language::French);
    }
}
