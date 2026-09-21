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

/// Read a clip as 16 kHz mono — what the recogniser expects.
fn read_clip(path: &std::path::Path) -> Result<Vec<f32>, String> {
    let output = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "f32le", "-ac", "1", "-ar", "16000", "-"])
        .output()
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output
        .stdout
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
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
    let mut checked = 0usize;
    let mut failures = 0usize;

    for phrase in &manifest.phrases {
        let Some(expected) = texts.get(&phrase.id) else {
            continue;
        };
        unreadable.extend(judge::unknown_characters(expected, &readings));

        let clip = args.base.join(&phrase.audio.normal);
        let samples = match read_clip(&clip) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {e}");
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
        checked += 1;

        if verdict.is_failure() {
            failures += 1;
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

    println!("\n{checked} clips checked, {failures} failed");
    if !unreadable.is_empty() {
        let shown: String = unreadable.iter().take(30).collect();
        println!(
            "note: {} character(s) have no reading, so those positions fall back to \
             the error rate: {shown}",
            unreadable.len()
        );
    }
    Ok(failures == 0)
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
