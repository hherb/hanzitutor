//! Synthesise the pronunciation clips for a graded phrase list.
//!
//! ```text
//! scripts/with-cargo-env.sh cargo run --release -p hanzi-say --bin synthesize-audio -- \
//!     --source no7z --in data/slices/no7z_hsk12_slice.json
//! ```
//!
//! Reads one or more sentence files, writes one MP3 per phrase per speed under
//! `public/audio/<source>/`, and a `manifest.json` beside them that says what was
//! produced and by which model.
//!
//! ## Why this is a Rust binary and not a script
//!
//! The app speaks phrases too — in time, a phrase with no bundled clip — and it
//! does that through the same `sherpa-onnx` crate this links. So the clips a
//! learner hears at build time and the audio the app produces at run time come
//! from one implementation rather than two that drift. A Python synthesis
//! pipeline would have been a second one to keep in step.
//!
//! ## What it deliberately does not do
//!
//! It never fetches the model: `scripts/fetch-tts.sh` does, and pins the digests
//! while it is there. A build that silently acquired weights over the network
//! would be the thing this repository's whole posture is arranged against.
//!
//! MP3 encoding is delegated to `ffmpeg`, which produces the store assets
//! already, rather than linking an encoder for the one format that ships.

use std::path::PathBuf;
use std::process::Command;

use hanzi_say::sentences::{self, Speed};
use hanzi_say::{AudioManifest, AudioPaths, ManifestPhrase, Model, SayError};

/// Where the model lives, relative to the repository root.
const DEFAULT_MODEL: &str = ".melo-tts/current";
/// Where clips are written, relative to the repository root. Under `public/`,
/// because Vite copies that directory verbatim into the build and only emits
/// assets from `src/` that something imports.
const DEFAULT_OUT: &str = "public/audio";
/// Mono MP3, at the shape the corpus this app takes its sentences from uses.
///
/// **The parameters here matter more than they look.** A first version encoded
/// at 32 kbps while leaving the model's 44.1 kHz rate alone, which is 0.73 bits
/// per sample — and it sounded it: a crackle and a hollowness over every phrase,
/// reported by ear before any measurement caught it. MeloTTS emits 44.1 kHz,
/// but speech carries almost nothing above 11 kHz, so MP3 at that rate is
/// spending its bits encoding hiss.
///
/// Resampling to 24 kHz costs nothing audible and changes the arithmetic
/// completely: 64 kbps mono at 24 kHz is 2.7 bits per sample, nearly four times
/// the budget, and it is what the reference clips for these sentences use
/// (measured: 77 kbps at 24 kHz). `-ar` is the part that was missing.
const MP3_BITRATE: &str = "64k";
/// Output sample rate. See [`MP3_BITRATE`] for why this and not the model's own.
const MP3_RATE: &str = "24000";

struct Args {
    source: String,
    inputs: Vec<PathBuf>,
    limit: usize,
    /// Keep only phrases whose level is one of these. Empty means all of them.
    levels: Vec<String>,
    out: PathBuf,
    model: PathBuf,
    /// Which engine speaks. See [`Engine`].
    engine: Engine,
    /// A CosyVoice checkout and its model, for `--engine cosyvoice`.
    cosyvoice_repo: Option<PathBuf>,
    cosyvoice_model: Option<PathBuf>,
    /// Reference clip for zero-shot cloning, and its transcript.
    prompt_wav: Option<PathBuf>,
    prompt_text: String,
    /// Interpreter for the CosyVoice script.
    ///
    /// Configurable because CosyVoice cannot live in the same environment as
    /// anything else: it pins `torch==2.3.1`, and installing that over a newer
    /// torch breaks one or the other. So it has its own virtualenv, and the
    /// caller names the interpreter inside it.
    cosyvoice_python: String,
}

/// The engine that produces the audio.
///
/// Two, because they are good at different things. MeloTTS runs in this process
/// through `sherpa-onnx` and is what the app also uses on device; CosyVoice 3
/// runs out of process in Python and is better at the phrases MeloTTS
/// mispronounces, but is 5 GB and cannot ship. See
/// `docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md`.
#[derive(Clone, Copy, PartialEq)]
enum Engine {
    MeloTts,
    /// Shell out to `scripts/cosyvoice-say.py`. Needs `--cosyvoice-repo`,
    /// `--cosyvoice-model` and `--prompt-wav`.
    CosyVoice,
}

fn usage() -> String {
    format!(
        "usage: synthesize-audio --source <corpus> --in <file> [--in <file>...] [options]\n\
         \n\
         \x20 --source <name>   corpus name; the output subdirectory and id namespace\n\
         \x20 --in <file>       sentence file to synthesise (repeatable; JSON array or JSONL)\n\
         \x20 --level <n>       keep only this level (repeatable; e.g. --level 1 --level 2)\n\
         \x20 --limit <n>       synthesise at most n phrases (0 = all)\n\
         \x20 --out <dir>       output root (default {DEFAULT_OUT})\n\
         \x20 --model <dir>     MeloTTS model directory (default {DEFAULT_MODEL})\n\
         \x20 --engine <name>   melotts (default) or cosyvoice\n\
         \x20 --cosyvoice-repo <dir>   a CosyVoice checkout, for --engine cosyvoice\n\
         \x20 --cosyvoice-model <dir>  its model directory\n\
         \x20 --prompt-wav <file>      reference clip for zero-shot cloning\n\
         \x20 --prompt-text <text>     that clip's transcript\n\
         \x20 --cosyvoice-python <exe> interpreter for the CosyVoice venv (default python3)\n"
    )
}

fn parse_args() -> Result<Args, String> {
    let mut source = None;
    let mut inputs = Vec::new();
    let mut limit = 0usize;
    let mut levels: Vec<String> = Vec::new();
    let mut out = PathBuf::from(DEFAULT_OUT);
    let mut model = PathBuf::from(DEFAULT_MODEL);
    let mut engine = Engine::MeloTts;
    let mut cosyvoice_repo: Option<PathBuf> = None;
    let mut cosyvoice_model: Option<PathBuf> = None;
    let mut prompt_wav: Option<PathBuf> = None;
    let mut prompt_text = "You are a helpful assistant.<|endofprompt|>".to_string();
    let mut cosyvoice_python = "python3".to_string();

    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        let mut value = |name: &str| {
            argv.next()
                .ok_or_else(|| format!("{name} needs a value\n{}", usage()))
        };
        match arg.as_str() {
            "--source" => source = Some(value("--source")?),
            "--in" => inputs.push(PathBuf::from(value("--in")?)),
            "--level" => levels.push(value("--level")?),
            "--limit" => {
                limit = value("--limit")?
                    .parse()
                    .map_err(|e| format!("--limit: {e}"))?
            }
            "--out" => out = PathBuf::from(value("--out")?),
            "--model" => model = PathBuf::from(value("--model")?),
            "--engine" => {
                let name = value("--engine")?;
                engine = match name.as_str() {
                    "melotts" => Engine::MeloTts,
                    "cosyvoice" => Engine::CosyVoice,
                    other => return Err(format!("unknown engine {other:?}\n{}", usage())),
                };
            }
            "--cosyvoice-repo" => cosyvoice_repo = Some(PathBuf::from(value("--cosyvoice-repo")?)),
            "--cosyvoice-model" => cosyvoice_model = Some(PathBuf::from(value("--cosyvoice-model")?)),
            "--prompt-wav" => prompt_wav = Some(PathBuf::from(value("--prompt-wav")?)),
            "--prompt-text" => prompt_text = value("--prompt-text")?,
            "--cosyvoice-python" => cosyvoice_python = value("--cosyvoice-python")?,
            "-h" | "--help" => return Err(usage()),
            other => return Err(format!("unexpected argument {other:?}\n{}", usage())),
        }
    }

    Ok(Args {
        source: source.ok_or_else(|| format!("--source is required\n{}", usage()))?,
        inputs: if inputs.is_empty() {
            return Err(format!("at least one --in is required\n{}", usage()));
        } else {
            inputs
        },
        limit,
        levels,
        out,
        model,
        engine,
        cosyvoice_repo,
        cosyvoice_model,
        prompt_wav,
        prompt_text,
        cosyvoice_python,
    })
}

/// Encode a WAV to mono MP3 with `ffmpeg`.
///
/// A failure here is reported rather than ignored: a phrase with no clip would
/// otherwise reach the manifest as a path that does not exist, and the frontend
/// would fail at playback on a real device instead of here.
fn encode_mp3(wav: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let output = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(wav)
        .args([
            "-codec:a",
            "libmp3lame",
            "-b:a",
            MP3_BITRATE,
            "-ar",
            MP3_RATE,
            "-ac",
            "1",
        ])
        .arg(dest)
        .output()
        .map_err(|e| format!("could not run ffmpeg (is it installed?): {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "ffmpeg failed for {}: {}",
            dest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}


/// Synthesise every phrase with CosyVoice 3, in one subprocess.
///
/// Batching is the reason this is a pre-pass rather than a per-phrase call: the
/// model takes tens of seconds to load, and doing that 1,638 times would make the
/// build absurd. The script writes a WAV per phrase; this then encodes each to
/// MP3, which is the same tail the MeloTTS path uses.
///
/// Returns the WAV for each phrase id, in the order given.
fn cosyvoice_wavs(
    args: &Args,
    phrases: &[sentences::Phrase],
    scratch: &std::path::Path,
) -> Result<std::collections::HashMap<String, PathBuf>, String> {
    let repo = args
        .cosyvoice_repo
        .as_ref()
        .ok_or("--engine cosyvoice needs --cosyvoice-repo")?;
    let model = args
        .cosyvoice_model
        .as_ref()
        .ok_or("--engine cosyvoice needs --cosyvoice-model")?;
    let prompt = args
        .prompt_wav
        .as_ref()
        .ok_or("--engine cosyvoice needs --prompt-wav")?;

    // The script's request format: one JSON object per line. Written to the
    // scratch directory so it is visible when something goes wrong.
    let list = scratch.join("cosyvoice-requests.jsonl");
    let mut body = String::new();
    for (speed, suffix) in Speed::all().iter().map(|s| (s.multiplier(), s.suffix())) {
        for phrase in phrases {
            let request = serde_json::json!({
                "id": format!("{}{suffix}", phrase.id.replace('/', "-")),
                "text": phrase.text,
                "speed": speed,
                // Inpainting is left to a later pass: the caller decides which
                // syllable to force, from the acoustic evidence, and this only
                // applies what it is given.
                "syllables": serde_json::Value::Null,
            });
            body.push_str(&request.to_string());
            body.push('\n');
        }
    }
    std::fs::write(&list, body).map_err(|e| format!("could not write {}: {e}", list.display()))?;

    let script = PathBuf::from("scripts/cosyvoice-say.py");
    let out_dir = scratch.join("cosyvoice");
    let status = Command::new(&args.cosyvoice_python)
        .arg(&script)
        .arg("--repo").arg(repo)
        .arg("--model-dir").arg(model)
        .arg("--list").arg(&list)
        .arg("--out-dir").arg(&out_dir)
        .arg("--prompt-wav").arg(prompt)
        .arg("--prompt-text").arg(&args.prompt_text)
        .status()
        .map_err(|e| format!("could not run {}: {e}", script.display()))?;
    if !status.success() {
        return Err(format!("{} exited with {status}", script.display()));
    }

    let mut wavs = std::collections::HashMap::new();
    for phrase in phrases {
        for suffix in ["", "_slow"] {
            let id = format!("{}{suffix}", phrase.id.replace('/', "-"));
            let wav = out_dir.join(format!("{id}.wav"));
            if !wav.is_file() {
                return Err(format!("{} did not produce {}", script.display(), wav.display()));
            }
            wavs.insert(format!("{}{suffix}", phrase.id), wav);
        }
    }
    Ok(wavs)
}

fn run(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut phrases = Vec::new();
    for input in &args.inputs {
        let mut read = sentences::load(input)?;
        println!("read {} phrases from {}", read.len(), input.display());
        phrases.append(&mut read);
    }
    if !args.levels.is_empty() {
        let before = phrases.len();
        phrases.retain(|p| args.levels.iter().any(|l| l == &p.level));
        println!(
            "kept {} of {before} phrases at level {}",
            phrases.len(),
            args.levels.join(", ")
        );
    }
    if args.limit > 0 && phrases.len() > args.limit {
        phrases.truncate(args.limit);
    }
    if phrases.is_empty() {
        return Err("no phrases to synthesise".into());
    }

    // MeloTTS is loaded once here; CosyVoice is run once below, over every
    // phrase, because both are far too slow to start per phrase.
    let model = if args.engine == Engine::MeloTts {
        let m = Model::load(&args.model).map_err(|e: SayError| e.to_string())?;
        println!("model {} at {} Hz", m.dir().display(), m.sample_rate());
        Some(m)
    } else {
        println!("engine: cosyvoice (out of process)");
        None
    };

    // A scratch directory for the intermediate WAVs. `ffmpeg` takes its input
    // from a file path rather than a pipe here, so that a failure names the
    // phrase it happened on.
    let scratch = std::env::temp_dir().join(format!("hanzi-say-{}", std::process::id()));
    std::fs::create_dir_all(&scratch)?;

    // CosyVoice runs over the whole list at once, before the per-phrase loop.
    let mut say_wavs = None;
    if args.engine == Engine::CosyVoice {
        println!("synthesising {} phrases with CosyVoice…", phrases.len());
        say_wavs = Some(
            cosyvoice_wavs(args, &phrases, &scratch)
                .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
        );
    }

    let mut manifest_phrases = Vec::with_capacity(phrases.len());
    let mut skipped = 0usize;
    let mut encoded = 0usize;
    // Phrases the engine could not say, and why. Reported at the end and left
    // out of the manifest, so the app never offers a clip that does not exist.
    let mut skipped_phrases: Vec<(String, String)> = Vec::new();
    let mut total_bytes = 0u64;

    for (n, phrase) in phrases.iter().enumerate() {
        let stem = sentences::clip_stem(&args.source, &phrase.id);
        let mut paths = AudioPaths {
            normal: String::new(),
            slow: String::new(),
        };
        let mut bytes = 0u64;

        for speed in Speed::all() {
            let rel = format!("{stem}{}.mp3", speed.suffix());
            let dest = args.out.join(&rel);
            // Resumable: an MP3 already written by an earlier pass is kept, so a
            // run stopped part-way costs the phrase in flight rather than the
            // whole corpus. The size is checked, not just the name, because a
            // truncated file is not a finished one.
            if std::fs::metadata(&dest).map(|m| m.len() > 1024).unwrap_or(false) {
                bytes += std::fs::metadata(&dest)?.len();
                match speed {
                    Speed::Normal => paths.normal = format!("audio/{rel}"),
                    Speed::Slow => paths.slow = format!("audio/{rel}"),
                }
                skipped += 1;
                continue;
            }
            // Whichever engine is in use produces a WAV at a known path; the
            // encode below is identical either way, which is what keeps the two
            // comparable.
            // A phrase the model cannot say is **skipped, not fatal**. One bad
            // input ended a 1,185-sentence run at item 156 before this: the
            // silence guard did its job and then took the whole batch down with
            // it, discarding every clip after it. A run over a corpus is not a
            // transaction — it should finish, report what it could not say, and
            // let the manifest record the rest.
            let wav = match &say_wavs {
                Some(wavs) => match wavs.get(&format!("{}{}", phrase.id, speed.suffix())) {
                    Some(p) => p.clone(),
                    None => {
                        skipped_phrases.push((phrase.id.clone(), "no audio produced".to_string()));
                        continue;
                    }
                },
                None => {
                    let path = scratch
                        .join(format!("{}-{}.wav", args.source, phrase.id.replace('/', "-")));
                    let model = model.as_ref().ok_or("no engine initialised")?;
                    match model.write_wav(&phrase.text, speed.multiplier(), &path) {
                        Ok(_) => path,
                        Err(e) => {
                            skipped_phrases.push((phrase.id.clone(), e.to_string()));
                            continue;
                        }
                    }
                }
            };
            encode_mp3(&wav, &dest).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            let _ = std::fs::remove_file(&wav);

            bytes += std::fs::metadata(&dest)?.len();
            encoded += 1;
            match speed {
                Speed::Normal => paths.normal = format!("audio/{rel}"),
                Speed::Slow => paths.slow = format!("audio/{rel}"),
            }
        }

        total_bytes += bytes;
        println!(
            "  [{}/{}] {}  ({bytes} B)",
            n + 1,
            phrases.len(),
            phrase.text
        );
        // If either speed failed, the phrase is not practisable and does not
        // belong in the manifest at all — a half-present phrase is worse than an
        // absent one, because it looks playable.
        if paths.normal.is_empty() || paths.slow.is_empty() {
            continue;
        }
        manifest_phrases.push(ManifestPhrase {
            id: phrase.id.clone(),
            level: phrase.level.clone(),
            text: phrase.text.clone(),
            pinyin: phrase.pinyin.clone(),
            translation: phrase.translation.clone(),
            audio: paths,
            bytes,
        });
    }

    let _ = std::fs::remove_dir(&scratch);

    let manifest = AudioManifest {
        source: args.source.clone(),
        // The name the link points at, not the link: `.melo-tts/current` says
        // nothing about which weights produced the audio, and a manifest exists
        // to answer exactly that.
        model: std::fs::read_link(&args.model)
            .ok()
            .and_then(|target| target.file_name().map(|n| n.to_string_lossy().into_owned()))
            .or_else(|| {
                args.model
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
            })
            .unwrap_or_default(),
        speeds: Speed::all().iter().map(|s| s.multiplier()).collect(),
        phrases: manifest_phrases,
    };

    let manifest_path = args.out.join(&args.source).join("manifest.json");
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)? + "\n")?;

    if skipped > 0 {
        println!("resumed: {skipped} clips already present, {encoded} written");
    }
    if !skipped_phrases.is_empty() {
        println!(
            "\n{} phrase(s) the engine could not say, omitted from the manifest:",
            skipped_phrases.len()
        );
        for (id, why) in skipped_phrases.iter().take(20) {
            println!("  {id}: {why}");
        }
        if skipped_phrases.len() > 20 {
            println!("  … and {} more", skipped_phrases.len() - 20);
        }
    }
    println!(
        "\n{} phrases, {:.2} MB total\nmanifest {}",
        manifest.phrases.len(),
        total_bytes as f64 / 1e6,
        manifest_path.display()
    );
    Ok(())
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
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
