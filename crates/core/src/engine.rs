use crate::config::Config;
use crate::error::Error;
use master_voice_linguistics::lang::Language;
use master_voice_linguistics::overrides::Overrides;
use master_voice_linguistics::phoneme::Boundary;
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
            robotic_depth: config
                .robotic_depth
                .unwrap_or(master_voice_synth::DEFAULT_ROBOTIC_DEPTH)
                .clamp(0.0, 1.0),
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

fn complete_prefix_len(text: &str) -> usize {
    text.char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | ';' | ':' | '\n'))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0)
}

pub struct StreamSynth {
    state: master_voice_synth::SynthState,
    opts: SynthOptions,
    pending: String,
    default_language: Language,
    first: bool,
}

impl StreamSynth {
    pub fn new(settings: &EngineSettings) -> Self {
        Self {
            state: master_voice_synth::SynthState::new(),
            opts: settings.synth_options(),
            pending: String::new(),
            default_language: settings.language.unwrap_or(Language::French),
            first: true,
        }
    }

    pub fn chunk(
        &mut self,
        text: &str,
        language: Option<Language>,
        overrides: &Overrides,
        last: bool,
    ) -> Result<(Language, master_voice_synth::AudioBuffer, f64), Error> {
        let started = std::time::Instant::now();
        self.pending.push_str(text);
        let ready = if last {
            std::mem::take(&mut self.pending)
        } else {
            let tail = self.pending.split_off(complete_prefix_len(&self.pending));
            std::mem::replace(&mut self.pending, tail)
        };
        if ready.trim().is_empty() {
            let elapsed = started.elapsed().as_secs_f64() * 1000.0;
            return Ok((
                language.unwrap_or(self.default_language),
                master_voice_synth::AudioBuffer {
                    samples: Vec::new(),
                    sample_rate: master_voice_synth::SAMPLE_RATE,
                },
                elapsed,
            ));
        }
        let utterance = master_voice_linguistics::phonemize(&ready, language, overrides)?;
        let mut phonemes = utterance.phonemes;
        if !last && !ready.trim_end().ends_with(['.', '!', '?']) {
            if let Some(tail) = phonemes.last_mut() {
                if tail.boundary_after == Boundary::Sentence {
                    tail.boundary_after = Boundary::Word;
                }
            }
        }
        let pos = master_voice_synth::ChunkPos {
            first: self.first,
            last,
        };
        let buffer = master_voice_synth::render_chunk(&mut self.state, &phonemes, &self.opts, pos);
        self.first = false;
        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
        Ok((utterance.language, buffer, elapsed))
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
        assert_eq!(
            settings.robotic_depth,
            master_voice_synth::DEFAULT_ROBOTIC_DEPTH
        );
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
    fn assert_safe(samples: &[f32], label: &str) {
        assert!(!samples.is_empty(), "{label}");
        assert!(
            samples
                .iter()
                .all(|sample| sample.is_finite() && sample.abs() <= 0.95),
            "{label}"
        );
    }

    #[test]
    fn whole_buffer_corpus_obeys_synthesis_headroom() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        for (label, text, language) in [
            (
                "english",
                "HELLO. MASTER VOICE IS ONLINE AND READY.",
                Language::English,
            ),
            (
                "french",
                "BONJOUR. UN ENFANT ENTEND MAINTENANT UNE VOIX SYNTHÉTIQUE CLAIRE.",
                Language::French,
            ),
            ("nasal", "UN BON VIN BLANC, MON AMI.", Language::French),
            ("consonants", "PAPA, TCHAO! SIX CHEZ ZOE.", Language::French),
        ] {
            let (_, buffer, _) =
                synthesize_text(text, Some(language), &settings, &overrides).unwrap();
            assert_safe(&buffer.samples, label);
            let peak = buffer
                .samples
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            let rms = (buffer
                .samples
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
                / buffer.samples.len() as f32)
                .sqrt();
            assert!(peak < 0.95, "{label}: peak={peak}");
            assert!(rms > 0.01, "{label}: rms={rms}");
        }
    }

    #[test]
    fn single_stream_chunk_matches_whole_buffer() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        let (_, whole, _) = synthesize_text(
            "MASTER VOICE IS ONLINE.",
            Some(Language::English),
            &settings,
            &overrides,
        )
        .unwrap();
        let mut stream = StreamSynth::new(&settings);
        let (_, chunk, _) = stream
            .chunk(
                "MASTER VOICE IS ONLINE.",
                Some(Language::English),
                &overrides,
                true,
            )
            .unwrap();
        assert_eq!(whole.samples, chunk.samples);
    }

    #[test]
    fn stream_chunks_are_safe_and_boundaries_are_continuous() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        let mut stream = StreamSynth::new(&settings);
        let chunks = ["MASTER ", "VOICE ", "IS ", "ONLINE."];
        let mut previous: Option<f32> = None;
        for (index, text) in chunks.iter().enumerate() {
            let (_, buffer, _) = stream
                .chunk(
                    text,
                    Some(Language::English),
                    &overrides,
                    index + 1 == chunks.len(),
                )
                .unwrap();
            assert_safe(&buffer.samples, text);
            if let Some(previous) = previous {
                let delta = (buffer.samples[0] - previous).abs();
                assert!(delta < 0.02, "{text}: boundary delta={delta}");
            }
            previous = buffer.samples.last().copied();
        }
    }

    fn stream_all(chunks: &[String], language: Language, settings: &EngineSettings) -> Vec<f32> {
        let overrides = Overrides::default();
        let mut stream = StreamSynth::new(settings);
        let mut out = Vec::new();
        for (index, text) in chunks.iter().enumerate() {
            let (_, buffer, _) = stream
                .chunk(text, Some(language), &overrides, index + 1 == chunks.len())
                .unwrap();
            out.extend(buffer.samples);
        }
        out
    }

    #[test]
    fn character_chunks_match_word_chunks() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        for (text, language) in [
            ("MASTER VOICE IS ONLINE.", Language::English),
            ("HELLO, THIS SYSTEM IS READY.", Language::English),
            ("BONJOUR, LE SYSTÈME EST PRÊT.", Language::French),
        ] {
            let by_word: Vec<String> = text
                .split_inclusive(' ')
                .map(str::to_string)
                .collect::<Vec<String>>();
            let by_char: Vec<String> = text.chars().map(|c| c.to_string()).collect();
            let word_samples = stream_all(&by_word, language, &settings);
            let char_samples = stream_all(&by_char, language, &settings);
            assert!(!word_samples.is_empty(), "{text}: word stream was silent");
            assert_eq!(
                word_samples.len(),
                char_samples.len(),
                "{text}: chunk granularity changed the sample count"
            );
            assert_eq!(
                word_samples, char_samples,
                "{text}: chunk granularity changed the audio"
            );
        }
    }

    #[test]
    fn mid_word_chunks_do_not_spell_letters() {
        let config = Config::default();
        let settings = EngineSettings::from_config(&config);
        let overrides = overrides_from_config(&config);
        let (_, whole, _) = synthesize_text(
            "SYSTEM READY.",
            Some(Language::English),
            &settings,
            &overrides,
        )
        .unwrap();
        let split = ["SY".to_string(), "STEM READY.".to_string()];
        let streamed = stream_all(&split, Language::English, &settings);
        assert_eq!(
            whole.samples, streamed,
            "a mid-word chunk boundary changed the rendered audio"
        );
    }
}
