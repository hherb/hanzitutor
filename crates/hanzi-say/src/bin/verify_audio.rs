//! Check that a clip says what its text says.
//!
//! ```text
//! scripts/with-cargo-env.sh cargo run --release -p hanzi-say --bin verify-audio -- \
//!     --manifest public/audio/no7z/manifest.json \
//!     --corpus data/phrases/no7z-sentences.jsonl \
//!     --readings data/phrases/readings.tsv \
//!     --asr .tmp-asr/<sensevoice dir>
//! ```
//!
//! ## Why this exists
//!
//! Every other check on a clip is about the *signal*: is it there, does it
//! decode, is it loud enough, does it clip, is it the right length. None of them
//! can tell you **what was said**, and that turned out to matter. Sweeping the
//! first batch through a recogniser found clips that were perfect by every
//! measurement and were saying the wrong words — MeloTTS corrupts the first
//! syllable of some phrases, so 请进，请坐。 was heard as 笔迹你做. A learner
//! repeating that is being taught the wrong thing.
//!
//! ## What it decides
//!
//! The judgement is [`hanzi_say::judge`], which compares the two texts
//! phonetically rather than by counting characters — see that module for why a
//! character-error rate alone is not enough. This program only supplies the
//! recogniser and the report.
//!
//! ## Why it is a separate program
//!
//! The recogniser is a 240 MB download. `synthesize-audio` must stay runnable
//! from a checkout with nothing but the TTS weights, so verification is here,
//! opt-in, and pointed at a model directory by the caller.

use std::collections::HashMap;
use std::path::PathBuf;

use hanzi_say::judge;
use hanzi_say::phonetics::Readings;
use hanzi_say::{AudioManifest, sentences};

struct Args {
    manifest: PathBuf,
    corpus: Vec<PathBuf>,
    /// `character<TAB>reading` lines; see `judge::readings_from_lines`.
    readings: PathBuf,
    asr: PathBuf,
    /// Root the manifest's clip paths are relative to.
    ///
    /// Defaults to `public`, which is where Vite serves them from. Made a flag
    /// because the same judgement has to run over a *different* corpus — the
    /// reference audio a comparison engine produced — and that will not live
    /// under `public/`.
    base: PathBuf,
    /// Report every clip, not only the failures.
    all: bool,
}

fn usage() -> String {
    "usage: verify-audio --manifest <manifest.json> --corpus <sentences.jsonl> \
     --readings <tai.tsv> --asr <sensevoice dir> [--all]\n\
     \n\
     \x20 --manifest  the manifest of clips to check\n\
     \x20 --corpus    the sentence file the clips were made from (repeatable)\n\
     \x20 --readings  character<TAB>pinyin lines, for the phonetic comparison\n\
     \x20 --asr       directory holding model.int8.onnx and tokens.txt\n\
     \x20 --base      root the clip paths are relative to (default public)\n\
     \x20 --all       list every clip, not only those that fail\n"
        .to_string()
}

fn parse_args() -> Result<Args, String> {
    let mut manifest = None;
    let mut corpus = Vec::new();
    let mut readings = None;
    let mut asr = None;
    let mut base = PathBuf::from("public");
    let mut all = false;

    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        let mut value = |name: &str| {
            argv.next()
                .ok_or_else(|| format!("{name} needs a value\n{}", usage()))
        };
        match arg.as_str() {
            "--manifest" => manifest = Some(PathBuf::from(value("--manifest")?)),
            "--corpus" => corpus.push(PathBuf::from(value("--corpus")?)),
            "--readings" => readings = Some(PathBuf::from(value("--readings")?)),
            "--asr" => asr = Some(PathBuf::from(value("--asr")?)),
            "--base" => base = PathBuf::from(value("--base")?),
            "--all" => all = true,
            "-h" | "--help" => return Err(usage()),
            other => return Err(format!("unexpected argument {other:?}\n{}", usage())),
        }
    }

    Ok(Args {
        manifest: manifest.ok_or_else(|| format!("--manifest is required\n{}", usage()))?,
        corpus: if corpus.is_empty() {
            return Err(format!("at least one --corpus is required\n{}", usage()));
        } else {
            corpus
        },
        readings: readings.ok_or_else(|| format!("--readings is required\n{}", usage()))?,
        asr: asr.ok_or_else(|| format!("--asr is required\n{}", usage()))?,
        base,
        all,
    })
}

/// The text each clip id was made from.
fn texts(corpora: &[PathBuf]) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    for path in corpora {
        for phrase in sentences::load(path).map_err(|e| e.to_string())? {
            map.insert(phrase.id, phrase.text);
        }
    }
    Ok(map)
}

/// Why a clip could not be read.
#[derive(Debug)]
enum ClipError {
    /// `ffmpeg` itself could not be run. Every clip would fail the same way, so
    /// this ends the run rather than being counted once per clip.
    NoDecoder(String),
    /// This clip did not decode.
    Undecodable(String),
}

/// Read a clip as 16 kHz mono — what the recogniser expects.
fn read_clip(path: &std::path::Path) -> Result<Vec<f32>, ClipError> {
    let output = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "f32le", "-ac", "1", "-ar", "16000", "-"])
        .output()
        .map_err(|e| {
            ClipError::NoDecoder(format!("could not run ffmpeg (is it installed?): {e}"))
        })?;
    if !output.status.success() {
        return Err(ClipError::Undecodable(format!(
            "ffmpeg failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    if output.stdout.is_empty() {
        return Err(ClipError::Undecodable(format!(
            "{} decoded to no audio",
            path.display()
        )));
    }
    Ok(output
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// What a run found, clip by clip.
///
/// Every clip in the manifest ends up in exactly one bucket. The first version
/// counted only the clips it managed to judge and skipped the rest with a line
/// on stderr, so a wrong `--base` — or no `ffmpeg` — reported "0 clips
/// checked, 0 failed" and exited 0: a check that passed by checking nothing.
#[derive(Debug, Default)]
struct Tally {
    /// Phrases the manifest lists.
    listed: usize,
    /// Clips the recogniser heard and the judge ruled on.
    checked: usize,
    /// Of those, the ones that say the wrong thing.
    wrong: usize,
    /// Clips the manifest names that are not on disk.
    missing: Vec<String>,
    /// Clips on disk that would not decode.
    undecodable: Vec<String>,
    /// Phrases with no text in the corpus, so nothing to judge them against.
    no_text: Vec<String>,
}

impl Tally {
    fn failures(&self) -> usize {
        self.wrong + self.missing.len() + self.undecodable.len() + self.no_text.len()
    }

    /// True only when something was checked and nothing failed.
    fn passed(&self) -> bool {
        self.checked > 0 && self.failures() == 0
    }

    /// The closing report: one line of totals, and a line of explanation for
    /// anything that is not a clean pass.
    fn summary(&self, base: &std::path::Path) -> String {
        let mut out = format!(
            "{} phrase(s) in the manifest: {} checked, {} failed — {} said the wrong \
             thing, {} missing, {} undecodable, {} not in the corpus",
            self.listed,
            self.checked,
            self.failures(),
            self.wrong,
            self.missing.len(),
            self.undecodable.len(),
            self.no_text.len(),
        );
        if self.checked == 0 {
            out.push_str(&format!(
                "\nFAILED: nothing was checked. Clips are looked for under {} — is \
                 --base right, and is --corpus the file this manifest was made from?",
                base.display()
            ));
        }
        for (label, ids) in [
            ("missing", &self.missing),
            ("undecodable", &self.undecodable),
            ("not in the corpus", &self.no_text),
        ] {
            if ids.is_empty() {
                continue;
            }
            let shown: Vec<&str> = ids.iter().take(10).map(String::as_str).collect();
            let more = ids.len().saturating_sub(shown.len());
            out.push_str(&format!("\n{label}: {}", shown.join(", ")));
            if more > 0 {
                out.push_str(&format!(" … and {more} more"));
            }
        }
        out
    }
}

fn run(args: &Args) -> Result<bool, Box<dyn std::error::Error>> {
    let manifest: AudioManifest = serde_json::from_str(&std::fs::read_to_string(&args.manifest)?)?;
    let texts = texts(&args.corpus).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let readings: Readings = judge::readings_from_lines(
        std::fs::read_to_string(&args.readings)?
            .lines()
            .map(str::to_string),
    );
    if readings.is_empty() {
        return Err(format!(
            "{} has no usable readings — expected `character<TAB>pinyin` lines",
            args.readings.display()
        )
        .into());
    }

    let model = args.asr.join("model.int8.onnx");
    let tokens = args.asr.join("tokens.txt");
    if !model.is_file() || !tokens.is_file() {
        return Err(format!(
            "no recogniser at {} — expected model.int8.onnx and tokens.txt",
            args.asr.display()
        )
        .into());
    }

    let config = sherpa_onnx::OfflineRecognizerConfig {
        model_config: sherpa_onnx::OfflineModelConfig {
            sense_voice: sherpa_onnx::OfflineSenseVoiceModelConfig {
                model: Some(model.to_string_lossy().into_owned()),
                language: Some("zh".to_string()),
                use_itn: true,
            },
            tokens: Some(tokens.to_string_lossy().into_owned()),
            num_threads: 2,
            ..Default::default()
        },
        ..Default::default()
    };
    let recogniser = sherpa_onnx::OfflineRecognizer::create(&config)
        .ok_or("the recogniser could not be loaded")?;

    // A character the readings cannot speak for is reported once, not per clip:
    // it weakens every verdict for that phrase and is the caller's to fix.
    let mut unreadable: std::collections::BTreeSet<char> = std::collections::BTreeSet::new();

    // The normal take only: the slow one is the same audio at a different rate,
    // so a defect appears in both and checking one halves the work.
    let mut tally = Tally {
        listed: manifest.phrases.len(),
        ..Tally::default()
    };

    for phrase in &manifest.phrases {
        let Some(expected) = texts.get(&phrase.id) else {
            println!(
                "  FAIL {:<12} not in the corpus — nothing to judge it against",
                phrase.id
            );
            tally.no_text.push(phrase.id.clone());
            continue;
        };
        unreadable.extend(judge::unknown_characters(expected, &readings));

        let clip = args.base.join(&phrase.audio.normal);
        if !clip.is_file() {
            println!("  FAIL {:<12} missing: {}", phrase.id, clip.display());
            tally.missing.push(phrase.id.clone());
            continue;
        }
        let samples = match read_clip(&clip) {
            Ok(s) => s,
            Err(ClipError::NoDecoder(e)) => return Err(e.into()),
            Err(ClipError::Undecodable(e)) => {
                println!("  FAIL {:<12} undecodable: {e}", phrase.id);
                tally.undecodable.push(phrase.id.clone());
                continue;
            }
        };
        let stream = recogniser.create_stream();
        stream.accept_waveform(16_000, &samples);
        recogniser.decode(&stream);
        let heard = judge::han_only(
            &stream
                .get_result()
                .map(|r| r.text)
                .unwrap_or_default(),
        );

        let verdict = judge::judge(expected, &heard, &readings);
        tally.checked += 1;

        if verdict.is_failure() {
            tally.wrong += 1;
            println!(
                "  FAIL {:<12} want {:<16} heard {:<16} {}",
                phrase.id,
                judge::han_only(expected),
                heard,
                verdict.reason()
            );
        } else if args.all {
            println!(
                "  ok   {:<12} want {:<16} heard {:<16}",
                phrase.id,
                judge::han_only(expected),
                heard
            );
        }
    }

    println!("\n{}", tally.summary(&args.base));
    if !unreadable.is_empty() {
        let shown: String = unreadable.iter().take(30).collect();
        println!(
            "note: {} character(s) have no reading, so those positions fall back to \
             the error rate: {shown}",
            unreadable.len()
        );
    }
    Ok(tally.passed())
}

fn main() -> std::process::ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(message) => {
            eprintln!("{message}");
            return std::process::ExitCode::FAILURE;
        }
    };
    match run(&args) {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(n: usize, prefix: &str) -> Vec<String> {
        (0..n).map(|i| format!("{prefix}{i}")).collect()
    }

    #[test]
    fn nothing_checked_is_a_failure() {
        // The issue's case: a wrong --base, so every clip is missing and the
        // old report read "0 clips checked, 0 failed" with exit 0.
        let tally = Tally {
            listed: 3,
            missing: ids(3, "p"),
            ..Tally::default()
        };
        assert!(!tally.passed());
        let summary = tally.summary(std::path::Path::new("wrong/base"));
        assert!(summary.contains("nothing was checked"), "{summary}");
        assert!(summary.contains("wrong/base"), "{summary}");
        assert!(summary.contains("3 missing"), "{summary}");

        // An empty manifest checks nothing either.
        assert!(!Tally::default().passed());
    }

    #[test]
    fn a_missing_or_undecodable_clip_fails_the_run() {
        let ok = Tally {
            listed: 2,
            checked: 2,
            ..Tally::default()
        };
        assert!(ok.passed());

        for tally in [
            Tally {
                missing: ids(1, "m"),
                ..Tally {
                    listed: 3,
                    checked: 2,
                    ..Tally::default()
                }
            },
            Tally {
                undecodable: ids(1, "u"),
                ..Tally {
                    listed: 3,
                    checked: 2,
                    ..Tally::default()
                }
            },
            Tally {
                no_text: ids(1, "t"),
                ..Tally {
                    listed: 3,
                    checked: 2,
                    ..Tally::default()
                }
            },
            Tally {
                wrong: 1,
                ..Tally {
                    listed: 2,
                    checked: 2,
                    ..Tally::default()
                }
            },
        ] {
            assert_eq!(tally.failures(), 1, "{tally:?}");
            assert!(!tally.passed(), "{tally:?} should fail");
        }
    }

    #[test]
    fn the_summary_accounts_for_every_clip() {
        let tally = Tally {
            listed: 30,
            checked: 10,
            wrong: 2,
            missing: ids(12, "m"),
            undecodable: ids(5, "u"),
            no_text: ids(3, "t"),
        };
        let summary = tally.summary(std::path::Path::new("public"));
        assert!(
            summary.starts_with(
                "30 phrase(s) in the manifest: 10 checked, 22 failed — 2 said the wrong thing, \
                 12 missing, 5 undecodable, 3 not in the corpus"
            ),
            "{summary}"
        );
        // Ids are listed, capped at ten per kind.
        assert!(summary.contains("missing: m0, m1"), "{summary}");
        assert!(summary.contains("… and 2 more"), "{summary}");
        assert!(summary.contains("undecodable: u0"), "{summary}");
        assert!(!summary.contains("nothing was checked"));
    }

    #[test]
    fn a_clip_that_is_not_audio_is_undecodable_not_fatal() {
        // Passes with or without ffmpeg installed, so the suite never needs it:
        // with it, garbage is `Undecodable` and counted; without it, the error is
        // the run-ending `NoDecoder`. Either way it is never silently skipped.
        let dir = std::env::temp_dir().join(format!("hanzi-say-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("bad.mp3");
        std::fs::write(&clip, b"not audio").unwrap();
        match read_clip(&clip) {
            Err(ClipError::Undecodable(_)) | Err(ClipError::NoDecoder(_)) => {}
            Ok(samples) => panic!("garbage decoded to {} samples", samples.len()),
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
