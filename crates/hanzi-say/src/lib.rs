//! Mandarin speech synthesis, wrapped around the toolkit crate the app already links.
//!
//! There are two callers, and they want the same thing:
//!
//! * `synthesize-audio`, which turns a graded phrase list into the MP3 clips the
//!   app ships, at build time and on a developer's machine;
//! * the app itself, which will speak a phrase it has no clip for.
//!
//! Both go through [`Model`], so a learner hears audio produced by exactly the
//! code that produced the bundled clips — one implementation, not two that drift.
//!
//! ## Why a crate of its own
//!
//! `hanzi-core` is deliberately free of native dependencies so the grading engine
//! builds and tests anywhere; this links `sherpa-onnx`, so it cannot live there.
//! It is also not `src-tauri`'s business: the build-time binary must be runnable
//! without building the desktop app.
//!
//! ## What this does not do
//!
//! It does not fetch the model, and it does not encode MP3. The first is
//! `scripts/fetch-tts.sh`'s job, which pins the weights by digest; the second is
//! `ffmpeg`'s, invoked by the binary. Synthesis produces samples and a sample
//! rate, and that is the whole of this crate's output.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod judge;
pub mod phonetics;
pub mod sentences;

pub use sentences::{Phrase, Speed};

/// Longest run of characters sent to the model in one call.
///
/// The model takes one utterance at a time, and a long passage in a single call
/// audibly flattens across the middle. Graded phrases are far shorter than this,
/// so it only bites on the reader texts.
const MAX_CHUNK: usize = 60;

/// Threads the synthesiser is allowed to use.
///
/// **Fixed rather than taken from the machine, because more is slower here.**
/// On a 16-core host, measured on one phrase at both speeds: 1 thread 2.53 s,
/// 2 threads 2.32 s, 4 threads 2.31 s, 8 threads 2.36 s, 16 threads **3.40 s**.
/// This model is small enough that extra threads spend their time synchronising
/// rather than computing, and asking for one per core made the whole build 45%
/// slower while making the machine unusable for anything else.
///
/// Four is the knee of that curve: within noise of the best, with enough
/// parallelism that a single phrase still finishes promptly on device.
const SYNTHESIS_THREADS: i32 = 4;

/// Why synthesis could not be set up.
#[derive(Debug)]
pub enum SayError {
    /// No model at the path given. The caller is expected to have run the fetch.
    NoModel(PathBuf),
    /// The toolkit refused the configuration — a missing or unreadable file.
    NotLoaded(PathBuf),
    /// The model produced nothing for the text.
    NoAudio(String),
    /// The model returned silence — see [`SILENCE_PEAK`]. Reported separately
    /// from [`Self::NoAudio`] because it is a different failure with a different
    /// cause, and a caller may want to retry one and not the other.
    Silent(String),
    /// The output could not be written.
    Io(String, std::io::Error),
}

impl std::fmt::Display for SayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoModel(p) => write!(
                f,
                "no speech model at {} — run scripts/fetch-tts.sh, which pins its digests",
                p.display()
            ),
            Self::NotLoaded(p) => write!(
                f,
                "the speech model at {} could not be loaded (a missing or unreadable file?)",
                p.display()
            ),
            Self::NoAudio(t) => write!(f, "the model produced no audio for {t:?}"),
            Self::Silent(t) => write!(
                f,
                "the model returned silence for {t:?} — this is a known failure \
                 mode for a few short inputs, and the phrase needs different text \
                 or a different voice"
            ),
            Self::Io(p, why) => write!(f, "could not write {p}: {why}"),
        }
    }
}

impl std::error::Error for SayError {}

/// Peak level every clip is normalised to.
///
/// **The model's own output is very quiet.** Measured across phrases, MeloTTS
/// returns samples peaking around 0.06–0.08 — about −22 dBFS, and roughly a
/// sixth of full scale. Written out unchanged, a clip is audible only with the
/// volume near maximum, and everything audible at that gain is the noise floor
/// and the MP3 encoder's quantisation, which is heard as crackle and a hollow,
/// echoing quality. It is not distortion in the samples; there is none. The
/// samples are simply too small.
///
/// The target is where the corpus this app takes its sentences from sits: its
/// published clips peak between 0.43 and 0.72. Normalising to a fixed 0.7 lands
/// inside that band, loud enough to be heard at an ordinary volume and with
/// enough headroom that the encoder never clips.
const TARGET_PEAK: f32 = 0.7;

/// Peak below which output is treated as the model having produced nothing.
///
/// **This is a real failure mode, not a defensive nicety.** MeloTTS returns
/// digital silence — 0.12 s at a peak of 0.0001 — for some inputs, and
/// 谢谢你！ is one of them, while 谢谢你。 is fine. A silent clip that reaches the
/// app is the worst kind of bug here: the file exists, the manifest lists it,
/// the button plays, and nothing is heard.
///
/// The threshold is far below anything voiced. Measured speech peaks are around
/// 0.03–0.15 before normalisation, so 1e-3 is roughly 30 dB below the quietest
/// real output — low enough never to reject speech, high enough to catch a
/// buffer that is only dither.
const SILENCE_PEAK: f32 = 1e-3;

/// Rewrite punctuation the model mishandles, for synthesis only.
///
/// **A workaround for a measured model defect, not a stylistic choice.** MeloTTS
/// drops what follows an exclamation mark inside a phrase: 谢谢你！不客气。 comes
/// back as 0.96 s of audio with the first clause missing, while 谢谢你。不客气。
/// is 1.57 s and complete. In isolation the same input is worse still — 谢谢你！
/// yields 0.12 s of digital silence.
///
/// A full stop says the same thing to a speech model: the clause has ended and
/// another begins. Nothing here reaches the learner, who still reads the original
/// text with its exclamation. The alternative is a clip that is quietly missing a
/// phrase, which is the failure this replaced.
fn for_speech(text: &str) -> String {
    text.replace(['！', '!'], "。")
}

/// The largest absolute sample, which is what both the silence guard and the
/// normaliser need and neither should compute differently.
fn peak_of(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// Scale `samples` so the loudest of them sits at [`TARGET_PEAK`].
///
/// Applied to the **whole utterance**, across every chunk, rather than per
/// chunk: normalising each piece separately would make one part of a sentence
/// louder than the next, which is a worse artefact than the quietness it fixes.
///
/// Never attenuates. A model that already returned full-scale audio would be
/// left alone rather than turned down, so this cannot make a future model
/// quieter than it was.
fn normalize(samples: &mut [f32]) {
    let peak = peak_of(samples);
    if peak <= 0.0 || peak >= TARGET_PEAK {
        return;
    }
    let gain = TARGET_PEAK / peak;
    for s in samples.iter_mut() {
        *s *= gain;
    }
}

/// 16-bit mono WAV, written by hand.
///
/// Not a dependency: a PCM WAV header is 44 bytes of little-endian integers, and
/// the alternative is a crate in the licence catalogue for something this crate
/// only writes to a temporary file that `ffmpeg` is about to read and discard.
fn wav_bytes(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);

    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM header size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        out.extend_from_slice(&((clamped * 32767.0) as i16).to_le_bytes());
    }
    out
}

/// A loaded model, ready to synthesise.
pub struct Model {
    tts: sherpa_onnx::OfflineTts,
    dir: PathBuf,
}

impl Model {
    /// Load the model in `dir`.
    ///
    /// `dir` is the directory holding `model.int8.onnx` and its companions, as
    /// laid down by `scripts/fetch-tts.sh`. Every file it names is required; a
    /// model that is half-present is the failure this returns rather than one
    /// that surfaces later as mangled audio.
    pub fn load(dir: impl AsRef<Path>) -> Result<Self, SayError> {
        let dir = dir.as_ref().to_path_buf();
        let model = dir.join("model.int8.onnx");
        if !model.is_file() {
            return Err(SayError::NoModel(dir));
        }

        // The four rewrites are what make a sentence read as a sentence rather
        // than a bag of characters: they normalise dates, numbers, phone numbers
        // and heteronyms before the model sees the text.
        let rules = ["date", "number", "phone", "new_heteronym"]
            .map(|name| dir.join(format!("{name}.fst")).to_string_lossy().into_owned())
            .join(",");

        let config = sherpa_onnx::OfflineTtsConfig {
            model: sherpa_onnx::OfflineTtsModelConfig {
                vits: sherpa_onnx::OfflineTtsVitsModelConfig {
                    model: Some(model.to_string_lossy().into_owned()),
                    lexicon: Some(dir.join("lexicon.txt").to_string_lossy().into_owned()),
                    tokens: Some(dir.join("tokens.txt").to_string_lossy().into_owned()),
                    dict_dir: Some(dir.to_string_lossy().into_owned()),
                    ..Default::default()
                },
                num_threads: SYNTHESIS_THREADS,
                ..Default::default()
            },
            rule_fsts: Some(rules),
            max_num_sentences: 1,
            ..Default::default()
        };

        let tts = sherpa_onnx::OfflineTts::create(&config)
            .ok_or_else(|| SayError::NotLoaded(dir.clone()))?;
        Ok(Self { tts, dir })
    }

    /// The model's own output rate. Resampling is the caller's problem, not this
    /// crate's — the build tool passes it to `ffmpeg` and the app to its device.
    pub fn sample_rate(&self) -> u32 {
        self.tts.sample_rate().max(0) as u32
    }

    /// Where this model was loaded from, for a manifest or a log line.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Synthesise `text` into mono samples at [`Self::sample_rate`].
    ///
    /// `speed` is a multiplier: 1.0 is the model's own rate, 0.7 the slow take.
    /// The text is split on sentence punctuation first and the pieces joined, so
    /// a passage keeps its prosody across the seam instead of flattening.
    ///
    /// The **characters** are synthesised, never the pinyin. The model has a
    /// Chinese lexicon, so 汉 comes out with its Mandarin reading; a romanised
    /// string would have it guess at the diacritics, and would also tell the
    /// learner the answer the exercise is asking for. This is the same decision
    /// `src-tauri/src/speech.rs` records for the system synthesiser.
    pub fn synthesize(&self, text: &str, speed: f32) -> Result<Vec<f32>, SayError> {
        let mut out = Vec::new();
        // The model is given a version of the text with the punctuation it
        // mishandles rewritten. See `for_speech`.
        for chunk in chunk_text(&for_speech(text)) {
            let config = sherpa_onnx::GenerationConfig {
                speed,
                ..Default::default()
            };
            let audio = self
                .tts
                .generate_with_config(&chunk, &config, None::<fn(&[f32], f32) -> bool>)
                .ok_or_else(|| SayError::NoAudio(text.to_string()))?;
            out.extend_from_slice(audio.samples());
        }
        if out.is_empty() {
            return Err(SayError::NoAudio(text.to_string()));
        }
        // Checked *before* normalising: normalisation would amplify a silent
        // buffer's dither to full scale and turn an obvious failure into a clip
        // of amplified noise, which is worse than silence because it sounds like
        // something.
        if peak_of(&out) < SILENCE_PEAK {
            return Err(SayError::Silent(text.to_string()));
        }
        normalize(&mut out);
        Ok(out)
    }

    /// Synthesise `text` and write it as a 16-bit mono WAV at `dest`.
    ///
    /// The intermediate WAV exists because encoding to MP3 is `ffmpeg`'s job and
    /// `ffmpeg` reads files or pipes — this keeps the encoder out of the
    /// dependency graph while still producing the format that ships.
    pub fn write_wav(&self, text: &str, speed: f32, dest: impl AsRef<Path>) -> Result<u64, SayError> {
        let dest = dest.as_ref();
        let samples = self.synthesize(text, speed)?;
        let bytes = wav_bytes(&samples, self.sample_rate());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SayError::Io(parent.display().to_string(), e))?;
        }
        std::fs::write(dest, &bytes).map_err(|e| SayError::Io(dest.display().to_string(), e))?;
        Ok(bytes.len() as u64)
    }
}

/// Split text into model-sized utterances.
///
/// **Only splits when it has to.** An earlier version cut on every sentence mark
/// and joined the pieces with no crossfade, which was wrong in two audible ways.
/// Each piece is generated independently, so the model's prosody restarts at the
/// join and the two waveforms rarely meet at the same amplitude — a phrase such
/// as 谢谢你！不客气。 came out as a click between two half-sentences rather than
/// as one thing said. Worse, a short piece is a degenerate input: 谢谢你！ on its
/// own is three characters, and the model produced audio so poor that the app's
/// own recogniser transcribed only the second clause.
///
/// So punctuation is no longer a split point. Text at or under [`MAX_CHUNK`]
/// goes to the model whole, which covers every graded phrase in both corpora —
/// the longest is 18 characters. Splitting is a fallback for a pasted paragraph,
/// and it happens on **spaces** where there are any, because a passage the user
/// typed has them, so the cut lands between words rather than inside one.
///
/// Public because the build tool reports how many pieces a phrase became, and a
/// phrase that split is worth being able to see.
pub fn chunk_text(text: &str) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if trimmed.chars().count() <= MAX_CHUNK {
        return vec![trimmed.to_string()];
    }

    // Over the limit: break on spaces, keeping whole words where the text has
    // any, and hard-wrap only a run that is itself longer than the limit.
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, out: &mut Vec<String>| {
        let piece = current.trim();
        if !piece.is_empty() {
            out.push(piece.to_string());
        }
        current.clear();
    };

    for word in trimmed.split(' ') {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current_len > 0 && current_len + 1 + word_len > MAX_CHUNK {
            flush(&mut current, &mut out);
        }
        if word_len > MAX_CHUNK {
            flush(&mut current, &mut out);
            let chars: Vec<char> = word.chars().collect();
            for window in chars.chunks(MAX_CHUNK) {
                out.push(window.iter().collect());
            }
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    flush(&mut current, &mut out);
    out
}

/// The manifest the build tool writes beside a corpus's clips.
///
/// Serialised to `public/audio/<source>/manifest.json` and read by the frontend,
/// which is why the field names are camelCase: the reader is TypeScript.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioManifest {
    /// The corpus these clips came from, e.g. `no7z`.
    pub source: String,
    /// Model directory name, so a manifest says what produced it.
    pub model: String,
    /// Speeds written, normal first.
    pub speeds: Vec<f32>,
    /// One entry per phrase, in the order they were synthesised.
    pub phrases: Vec<ManifestPhrase>,
}

/// One phrase's clips within an [`AudioManifest`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestPhrase {
    pub id: String,
    /// HSK level, or a reader shelf name for the graded readers.
    pub level: String,
    pub text: String,
    pub pinyin: String,
    pub translation: String,
    /// `normal` and `slow`, as root-relative URLs the webview can play.
    pub audio: AudioPaths,
    /// Total bytes of every clip for this phrase.
    pub bytes: u64,
}

/// The clips written for one phrase.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioPaths {
    pub normal: String,
    pub slow: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_multi_sentence_phrase_is_not_split() {
        // The regression this guards: splitting here produced a click at the join
        // and a degenerate three-character fragment that even the app's own
        // recogniser could not transcribe.
        assert_eq!(chunk_text("谢谢你！不客气。"), vec!["谢谢你！不客气。"]);
        assert_eq!(chunk_text("对不起。没关系。"), vec!["对不起。没关系。"]);
    }

    #[test]
    fn a_short_phrase_is_one_chunk_even_with_punctuation() {
        assert_eq!(chunk_text("你好，很高兴认识你。"), vec!["你好，很高兴认识你。"]);
        assert_eq!(chunk_text("老师，您好！"), vec!["老师，您好！"]);
    }

    #[test]
    fn chunks_split_on_spaces_only_when_too_long() {
        // Long enough to be over `MAX_CHUNK`, with spaces to break on. Built
        // rather than written out so the test cannot drift out of range if the
        // limit changes.
        let unit = "你好世界";
        let long = std::iter::repeat_n(unit, MAX_CHUNK / unit.chars().count() + 4)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(long.chars().count() > MAX_CHUNK, "test input must exceed the limit");

        let chunks = chunk_text(&long);
        assert!(chunks.len() > 1, "a long passage should split");
        for chunk in &chunks {
            assert!(
                chunk.chars().count() <= MAX_CHUNK,
                "chunk over the limit: {chunk:?}"
            );
            // Split on a space, so no chunk begins or ends mid-word.
            assert!(!chunk.starts_with(' ') && !chunk.ends_with(' '));
        }
        // Nothing lost in the split.
        assert_eq!(chunks.join(" ").replace(' ', ""), long.replace(' ', ""));
    }

    #[test]
    fn a_single_word_longer_than_the_limit_is_hard_wrapped() {
        let long = "汉".repeat(MAX_CHUNK + 5);
        let chunks = chunk_text(&long);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), MAX_CHUNK);
        assert_eq!(chunks[1].chars().count(), 5);
    }

    #[test]
    fn chunks_of_blank_text_are_empty() {
        assert!(chunk_text("").is_empty());
        assert!(chunk_text("   ").is_empty());
    }

    #[test]
    fn a_chunk_is_never_blank() {
        // Blank text must never reach the model: an empty utterance is a
        // degenerate input, and this is the guard that keeps one out.
        for text in ["  你好  ", "\t你好\t", "你好 "] {
            let chunks = chunk_text(text);
            assert!(!chunks.is_empty());
            for chunk in chunks {
                assert!(!chunk.trim().is_empty(), "{text:?} produced a blank chunk");
            }
        }
    }

    #[test]
    fn wav_header_is_44_bytes_and_states_its_shape() {
        let wav = wav_bytes(&[0.0, 0.5, -0.5], 44100);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 6);
        // byte rate = rate * channels * bytes per sample
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 88_200);
        // data chunk size
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn normalize_raises_a_quiet_clip_to_the_target_peak() {
        // The real model returns peaks near 0.07; this is what fixes that.
        let mut samples = vec![0.0, 0.0317, -0.0634, 0.01];
        normalize(&mut samples);
        let peak = peak_of(&samples);
        assert!((peak - TARGET_PEAK).abs() < 1e-6, "peak was {peak}");
        // Relative shape is preserved, so it is a gain and not a limiter.
        assert!((samples[2] / samples[1] - -2.0).abs() < 1e-5);
    }

    #[test]
    fn normalize_never_attenuates() {
        let mut samples = vec![0.0, 0.9, -1.0];
        let before = samples.clone();
        normalize(&mut samples);
        assert_eq!(samples, before, "a loud clip must be left alone");
    }

    #[test]
    fn normalize_leaves_silence_alone() {
        let mut samples = vec![0.0, 0.0];
        normalize(&mut samples);
        assert_eq!(samples, vec![0.0, 0.0], "silence has no peak to scale");
    }

    #[test]
    fn the_silence_threshold_separates_dither_from_speech() {
        // `谢谢你！` returns a peak of about 0.0001; the quietest voiced output
        // measured is about 0.03. The threshold has to sit between them, with
        // room on both sides.
        let silent = vec![1e-4f32, -1e-4, 5e-5];
        assert!(peak_of(&silent) < SILENCE_PEAK, "dither must read as silent");

        let quietest_voiced = vec![0.03f32, -0.01, 0.005];
        assert!(
            peak_of(&quietest_voiced) > SILENCE_PEAK,
            "the quietest real speech must not be rejected"
        );
        // An order of magnitude of headroom on each side.
        assert!(SILENCE_PEAK > peak_of(&silent) * 5.0);
        assert!(SILENCE_PEAK * 5.0 < peak_of(&quietest_voiced));
    }

    #[test]
    fn exclamation_marks_are_rewritten_for_speech_only() {
        // The model truncates after an internal ！, so it is given a full stop.
        assert_eq!(for_speech("谢谢你！不客气。"), "谢谢你。不客气。");
        assert_eq!(for_speech("你好!"), "你好。");
        // Nothing else is touched, and a sentence with no ！ is unchanged.
        assert_eq!(for_speech("你好，很高兴认识你。"), "你好，很高兴认识你。");
        assert_eq!(for_speech("请问，你叫什么名字？"), "请问，你叫什么名字？");
    }

    #[test]
    fn silence_is_reported_as_silence_not_as_no_audio() {
        // The two are different failures with different causes, and a caller may
        // want to retry one and not the other.
        let silent = SayError::Silent("谢谢你！".to_string());
        let message = silent.to_string();
        assert!(message.contains("silence"), "{message}");
        assert!(!matches!(silent, SayError::NoAudio(_)));
    }

    #[test]
    fn a_missing_model_says_what_to_run() {
        // Matched rather than `unwrap_err`: a successful load would have to be
        // printed, and `Model` holds a toolkit handle that is not `Debug`.
        let message = match Model::load("/nonexistent/melo") {
            Err(e) => e.to_string(),
            Ok(_) => panic!("loading a nonexistent model should fail"),
        };
        assert!(message.contains("fetch-tts.sh"), "{message}");
    }
}
