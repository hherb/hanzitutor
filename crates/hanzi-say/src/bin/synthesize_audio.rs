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
//!
//! ## Running it again
//!
//! A run can be stopped and started again, and can be given a subset
//! (`--level`, `--limit`, one `--in` of several) without losing the rest. The
//! rules are [`hanzi_say::resume`]'s: a clip is encoded to `.tmp` and renamed,
//! so a file at its final name is finished; a phrase is reused only when the
//! manifest records it under the key this run would compute — text, engine,
//! model, voice and encoder settings — so changing any of them re-synthesises
//! it; and the manifest on disk is merged into rather than replaced.

use std::path::{Path, PathBuf};
use std::process::Command;

use hanzi_say::resume;
use hanzi_say::sentences::{self, Speed};
use hanzi_say::{AudioPaths, ManifestPhrase, Model, SayError};

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

/// Encode a WAV to mono MP3 with `ffmpeg`, at `dest`.
///
/// `dest` is the *partial* path ([`resume::partial_path`]), never the clip's
/// final name: `ffmpeg` writes as it goes, so a run stopped mid-encode would
/// otherwise leave a truncated MP3 where a finished one is expected. The format
/// is named with `-f mp3` because the `.tmp` extension does not say it.
///
/// A failure here is reported rather than ignored: a phrase with no clip would
/// otherwise reach the manifest as a path that does not exist, and the frontend
/// would fail at playback on a real device instead of here.
fn encode_mp3(wav: &Path, dest: &Path) -> Result<(), String> {
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
            "-f",
            "mp3",
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

/// The name the manifest records for the model that speaks.
///
/// For MeloTTS, the name the link points at, not the link: `.melo-tts/current`
/// says nothing about which weights produced the audio, and a manifest exists to
/// answer exactly that. For CosyVoice, its model directory — not the MeloTTS
/// one, which an earlier version recorded whichever engine ran.
fn model_id(args: &Args) -> String {
    let name_of = |path: &Path| path.file_name().map(|n| n.to_string_lossy().into_owned());
    match args.engine {
        Engine::MeloTts => std::fs::read_link(&args.model)
            .ok()
            .and_then(|target| name_of(&target))
            .or_else(|| name_of(&args.model))
            .unwrap_or_default(),
        Engine::CosyVoice => args
            .cosyvoice_model
            .as_deref()
            .and_then(name_of)
            .unwrap_or_else(|| "cosyvoice".to_string()),
    }
}

/// Everything except the text that decides what a clip sounds like, in the
/// order [`resume::synthesis_key`] hashes it.
///
/// The engine, the model, the speeds and the encoder settings, and for
/// CosyVoice the reference clip it clones (by content, so replacing the file
/// under the same name counts) and that clip's transcript. Anything the code
/// itself changes is covered by [`resume::KEY_VERSION`].
fn voice_parts(args: &Args, model: &str) -> Result<Vec<String>, String> {
    let speeds: Vec<String> = Speed::all()
        .iter()
        .map(|s| s.multiplier().to_string())
        .collect();
    let mut parts = vec![
        resume::KEY_VERSION.to_string(),
        match args.engine {
            Engine::MeloTts => "melotts".to_string(),
            Engine::CosyVoice => "cosyvoice".to_string(),
        },
        model.to_string(),
        speeds.join(","),
        MP3_BITRATE.to_string(),
        MP3_RATE.to_string(),
    ];
    if args.engine == Engine::CosyVoice {
        let prompt = args
            .prompt_wav
            .as_ref()
            .ok_or("--engine cosyvoice needs --prompt-wav")?;
        let bytes = std::fs::read(prompt).map_err(|e| format!("{}: {e}", prompt.display()))?;
        parts.push(resume::digest(&bytes));
        parts.push(args.prompt_text.clone());
    }
    Ok(parts)
}

/// The key one phrase's clips are recorded under.
fn phrase_key(voice: &[String], text: &str) -> String {
    let mut parts: Vec<&str> = voice.iter().map(String::as_str).collect();
    parts.push(text);
    resume::synthesis_key(&parts)
}

/// The clip URLs a phrase's entry should name, and the files they are under
/// `out`, normal first.
fn clip_paths(args: &Args, id: &str) -> (AudioPaths, [PathBuf; 2]) {
    let stem = sentences::clip_stem(&args.source, id);
    let rel = |speed: Speed| format!("{stem}{}.mp3", speed.suffix());
    let audio = AudioPaths {
        normal: format!("audio/{}", rel(Speed::Normal)),
        slow: format!("audio/{}", rel(Speed::Slow)),
    };
    let files = [
        args.out.join(rel(Speed::Normal)),
        args.out.join(rel(Speed::Slow)),
    ];
    (audio, files)
}

fn size_of(paths: &[PathBuf]) -> std::io::Result<u64> {
    paths
        .iter()
        .map(|p| std::fs::metadata(p).map(|m| m.len()))
        .sum()
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
    // Every id in the inputs, in file order, before any filter: the merged
    // manifest is written in this order whichever subset this run produced.
    let order: Vec<String> = phrases.iter().map(|p| p.id.clone()).collect();
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

    let out_dir = args.out.join(&args.source);
    let manifest_path = out_dir.join("manifest.json");

    // What an interrupted run left half-written. Nothing at a final name can be
    // partial — encoding goes to `.tmp` and is renamed — so these are the only
    // files that can be, and they are of no use to anyone.
    let partials = resume::remove_partials(&out_dir)?;
    if !partials.is_empty() {
        println!(
            "removed {} partial file(s) left by an interrupted run",
            partials.len()
        );
    }

    let model_name = model_id(args);
    let voice = voice_parts(args, &model_name)?;
    let speeds: Vec<f32> = Speed::all().iter().map(|s| s.multiplier()).collect();
    let mut merged = resume::Merged::new(
        resume::read_manifest(&manifest_path)?,
        &args.source,
        &model_name,
        speeds,
    )?;

    // Which phrases already have clips made from exactly what this run would
    // use, and which need synthesising. Decided before any engine is loaded, so
    // a run with nothing to do needs no model.
    let mut todo: Vec<(sentences::Phrase, String)> = Vec::new();
    let (mut reused, mut adopted) = (0usize, 0usize);
    for phrase in &phrases {
        let key = phrase_key(&voice, &phrase.text);
        let (audio, files) = clip_paths(args, &phrase.id);
        let entry = merged.entry(&phrase.id);
        let present =
            entry.is_some_and(|e| e.audio == audio) && resume::clips_present(&args.out, &audio);
        let verdict = resume::reuse(
            entry,
            &phrase.text,
            &key,
            &model_name,
            merged.previous_model(),
            present,
        );
        let Some(verdict) = verdict else {
            todo.push((phrase.clone(), key));
            continue;
        };
        match verdict {
            resume::Reuse::Keyed => reused += 1,
            resume::Reuse::Legacy => adopted += 1,
        }
        // The clips stand; the rest of the entry is refreshed from the corpus,
        // since a corrected gloss or reading does not change the audio.
        merged.insert(ManifestPhrase {
            id: phrase.id.clone(),
            level: phrase.level.clone(),
            text: phrase.text.clone(),
            pinyin: phrase.pinyin.clone(),
            translation: phrase.translation.clone(),
            audio,
            bytes: size_of(&files)?,
            model: Some(model_name.clone()),
            key: Some(key),
        });
    }
    println!(
        "{} phrase(s) to synthesise; {reused} already made from the same text and \
         model{}",
        todo.len(),
        if adopted > 0 {
            format!(", {adopted} adopted from a manifest written before keys")
        } else {
            String::new()
        }
    );
    // Clips that exist and are about to be overwritten deserve a line of their
    // own: they are committed, and a model or text change replaces them all.
    let replacing = todo
        .iter()
        .filter(|(p, _)| merged.entry(&p.id).is_some())
        .count();
    if replacing > 0 {
        println!(
            "{replacing} of them replace existing clips whose text, model or settings \
             differ (the manifest on disk records model {:?}; this run uses {model_name:?})",
            merged.previous_model().unwrap_or("none")
        );
    }

    // Written after every phrase, so a run stopped part-way loses only the
    // phrase in flight — and written atomically, so a stop mid-write cannot
    // leave a truncated manifest that the next run would refuse.
    let present = |e: &ManifestPhrase| resume::clips_present(&args.out, &e.audio);
    let write_manifest =
        |merged: &resume::Merged| -> Result<resume::Built, Box<dyn std::error::Error>> {
            let built = merged.build(&order, present);
            std::fs::create_dir_all(&out_dir)?;
            resume::write_atomically(
                &manifest_path,
                (serde_json::to_string_pretty(&built.manifest)? + "\n").as_bytes(),
            )?;
            Ok(built)
        };

    // Phrases the engine could not say, and why. Reported at the end and left
    // out of the manifest, so the app never offers a clip that does not exist.
    let mut skipped_phrases: Vec<(String, String)> = Vec::new();
    let mut encoded = 0usize;

    if !todo.is_empty() {
        // MeloTTS is loaded once here; CosyVoice is run once below, over every
        // phrase that needs it, because both are far too slow to start per phrase.
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

        // CosyVoice runs over the whole list at once, before the per-phrase loop
        // — and only over the phrases that need it, so a resumed run does not pay
        // for the ones already done.
        let mut say_wavs = None;
        if args.engine == Engine::CosyVoice {
            println!("synthesising {} phrases with CosyVoice…", todo.len());
            let wanted: Vec<sentences::Phrase> = todo.iter().map(|(p, _)| p.clone()).collect();
            say_wavs = Some(
                cosyvoice_wavs(args, &wanted, &scratch)
                    .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?,
            );
        }

        for (n, (phrase, key)) in todo.iter().enumerate() {
            let (audio, files) = clip_paths(args, &phrase.id);
            let partial: Vec<PathBuf> = files.iter().map(|f| resume::partial_path(f)).collect();
            let discard = || {
                for p in &partial {
                    let _ = std::fs::remove_file(p);
                }
            };

            // Both speeds are encoded to their partial paths first and renamed
            // into place together, so a phrase is never left with one new take
            // and one old one.
            let mut failure = None;
            for (speed, dest) in Speed::all().into_iter().zip(&partial) {
                // Whichever engine is in use produces a WAV at a known path; the
                // encode below is identical either way, which is what keeps the
                // two comparable.
                // A phrase the model cannot say is **skipped, not fatal**. One bad
                // input ended a 1,185-sentence run at item 156 before this: the
                // silence guard did its job and then took the whole batch down
                // with it, discarding every clip after it. A run over a corpus is
                // not a transaction — it should finish, report what it could not
                // say, and let the manifest record the rest.
                let wav = match &say_wavs {
                    Some(wavs) => match wavs.get(&format!("{}{}", phrase.id, speed.suffix())) {
                        Some(p) => p.clone(),
                        None => {
                            failure = Some("no audio produced".to_string());
                            break;
                        }
                    },
                    None => {
                        let path = scratch.join(format!(
                            "{}-{}{}.wav",
                            args.source,
                            phrase.id.replace('/', "-"),
                            speed.suffix()
                        ));
                        let model = model.as_ref().ok_or("no engine initialised")?;
                        match model.write_wav(&phrase.text, speed.multiplier(), &path) {
                            Ok(_) => path,
                            Err(e) => {
                                failure = Some(e.to_string());
                                break;
                            }
                        }
                    }
                };
                let encoded_ok = encode_mp3(&wav, dest);
                let _ = std::fs::remove_file(&wav);
                if let Err(e) = encoded_ok {
                    // Not a phrase-level problem: `ffmpeg` missing or broken
                    // fails every phrase alike, so stop. The manifest written
                    // so far already records everything finished.
                    discard();
                    return Err(e.into());
                }
            }

            if let Some(why) = failure {
                discard();
                // If either speed failed, the phrase is not practisable — a
                // half-present phrase is worse than an absent one, because it
                // looks playable. An entry from an earlier run is left as it
                // was: its clips are untouched and still say what it records.
                let note = if merged.entry(&phrase.id).is_some() {
                    format!("{why} (the previous clips are kept)")
                } else {
                    why
                };
                skipped_phrases.push((phrase.id.clone(), note));
                continue;
            }

            for (tmp, dest) in partial.iter().zip(&files) {
                std::fs::rename(tmp, dest)
                    .map_err(|e| format!("could not move {} into place: {e}", tmp.display()))?;
            }
            let bytes = size_of(&files)?;
            encoded += files.len();
            merged.insert(ManifestPhrase {
                id: phrase.id.clone(),
                level: phrase.level.clone(),
                text: phrase.text.clone(),
                pinyin: phrase.pinyin.clone(),
                translation: phrase.translation.clone(),
                audio,
                bytes,
                model: Some(model_name.clone()),
                key: Some(key.clone()),
            });
            write_manifest(&merged)?;
            println!("  [{}/{}] {}  ({bytes} B)", n + 1, todo.len(), phrase.text);
        }

        let _ = std::fs::remove_dir(&scratch);
    }

    let built = write_manifest(&merged)?;

    println!(
        "\n{encoded} clip(s) written; {reused} phrase(s) reused{}",
        if adopted > 0 {
            format!(", {adopted} adopted")
        } else {
            String::new()
        }
    );
    if !skipped_phrases.is_empty() {
        println!(
            "\n{} phrase(s) the engine could not say, omitted from the manifest:",
            skipped_phrases.len()
        );
        print_ids(
            skipped_phrases
                .iter()
                .map(|(id, why)| format!("{id}: {why}")),
        );
    }
    if !built.dropped_missing.is_empty() {
        println!(
            "\n{} manifest entr(ies) dropped because a clip they name is gone from disk:",
            built.dropped_missing.len()
        );
        print_ids(built.dropped_missing.iter().cloned());
    }
    if !built.kept_outside.is_empty() {
        println!(
            "\n{} manifest entr(ies) kept from an earlier run although their phrase is \
             not in these input files (remove them by hand if they were retired):",
            built.kept_outside.len()
        );
        print_ids(built.kept_outside.iter().cloned());
    }
    let total_bytes: u64 = built.manifest.phrases.iter().map(|p| p.bytes).sum();
    println!(
        "\n{} phrases in the manifest, {:.2} MB total\nmanifest {}",
        built.manifest.phrases.len(),
        total_bytes as f64 / 1e6,
        manifest_path.display()
    );
    Ok(())
}

/// Print a list of report lines, the first twenty and a count of the rest.
fn print_ids(lines: impl ExactSizeIterator<Item = String>) {
    let total = lines.len();
    for line in lines.take(20) {
        println!("  {line}");
    }
    if total > 20 {
        println!("  … and {} more", total - 20);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn args(engine: Engine) -> Args {
        Args {
            source: "no7z".to_string(),
            inputs: vec![PathBuf::from("in.jsonl")],
            limit: 0,
            levels: Vec::new(),
            out: PathBuf::from("public/audio"),
            model: PathBuf::from("/nonexistent/vits-melo-tts-zh_en"),
            engine,
            cosyvoice_repo: None,
            cosyvoice_model: Some(PathBuf::from("/models/Fun-CosyVoice3-0.5B")),
            prompt_wav: None,
            prompt_text: "prompt".to_string(),
            cosyvoice_python: "python3".to_string(),
        }
    }

    #[test]
    fn the_key_follows_the_text_and_the_model() {
        let a = args(Engine::MeloTts);
        let voice = voice_parts(&a, &model_id(&a)).unwrap();
        let key = phrase_key(&voice, "你好。");
        assert_eq!(key, phrase_key(&voice, "你好。"));
        assert_ne!(
            key,
            phrase_key(&voice, "您好。"),
            "changed text must not reuse the clip"
        );

        let other = voice_parts(&a, "another-model").unwrap();
        assert_ne!(
            key,
            phrase_key(&other, "你好。"),
            "a changed model must not reuse the clip"
        );
    }

    #[test]
    fn the_key_covers_the_engine_and_the_cosyvoice_prompt() {
        let melo = args(Engine::MeloTts);
        let melo_voice = voice_parts(&melo, "m").unwrap();

        let dir = std::env::temp_dir().join(format!("hanzi-say-synth-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let prompt = dir.join("prompt.wav");
        std::fs::write(&prompt, b"one voice").unwrap();
        let mut cosy = args(Engine::CosyVoice);
        cosy.prompt_wav = Some(prompt.clone());
        let cosy_voice = voice_parts(&cosy, "m").unwrap();
        assert_ne!(
            phrase_key(&melo_voice, "你好"),
            phrase_key(&cosy_voice, "你好")
        );

        // A different reference clip under the same name is a different voice.
        std::fs::write(&prompt, b"another voice").unwrap();
        let recloned = voice_parts(&cosy, "m").unwrap();
        assert_ne!(
            phrase_key(&cosy_voice, "你好"),
            phrase_key(&recloned, "你好")
        );

        // And so is a different transcript for it.
        cosy.prompt_text = "other".to_string();
        assert_ne!(
            phrase_key(&recloned, "你好"),
            phrase_key(&voice_parts(&cosy, "m").unwrap(), "你好")
        );
        std::fs::remove_dir_all(&dir).unwrap();

        // CosyVoice without a prompt is refused before anything runs.
        assert!(voice_parts(&args(Engine::CosyVoice), "m").is_err());
    }

    #[test]
    fn the_model_recorded_is_the_engine_that_spoke() {
        assert_eq!(model_id(&args(Engine::MeloTts)), "vits-melo-tts-zh_en");
        assert_eq!(model_id(&args(Engine::CosyVoice)), "Fun-CosyVoice3-0.5B");
    }

    /// A corpus of two phrases at levels 1 and 3, with finished clips and a
    /// manifest naming them, in a fresh directory. No model, and no `ffmpeg`.
    fn corpus(name: &str, keyed: bool) -> (Args, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hanzi-say-run-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("out/no7z")).unwrap();
        let input = dir.join("in.json");
        std::fs::write(
            &input,
            r#"[{"id":"hsk1-0001","hsk":1,"chinese":"你好。"},{"id":"hsk3-0001","hsk":3,"chinese":"我学中文。"}]"#,
        )
        .unwrap();
        let mut a = args(Engine::MeloTts);
        a.inputs = vec![input];
        a.out = dir.join("out");
        a.model = dir.join("vits-melo-tts-zh_en"); // absent: loading it fails

        let voice = voice_parts(&a, &model_id(&a)).unwrap();
        let mut phrases = Vec::new();
        for (id, level, text) in [
            ("hsk1-0001", "1", "你好。"),
            ("hsk3-0001", "3", "我学中文。"),
        ] {
            let (audio, files) = clip_paths(&a, id);
            for f in &files {
                std::fs::write(f, b"a finished clip").unwrap();
            }
            phrases.push(ManifestPhrase {
                id: id.to_string(),
                level: level.to_string(),
                text: text.to_string(),
                pinyin: String::new(),
                translation: String::new(),
                audio,
                bytes: 30,
                model: None,
                key: keyed.then(|| phrase_key(&voice, text)),
            });
        }
        let manifest = hanzi_say::AudioManifest {
            source: "no7z".to_string(),
            model: "vits-melo-tts-zh_en".to_string(),
            speeds: vec![1.0, 0.7],
            phrases,
        };
        std::fs::write(
            dir.join("out/no7z/manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        (a, dir)
    }

    fn manifest_at(dir: &Path) -> hanzi_say::AudioManifest {
        resume::read_manifest(&dir.join("out/no7z/manifest.json"))
            .unwrap()
            .unwrap()
    }

    #[test]
    fn a_one_level_run_keeps_every_other_phrase() {
        // The truncation in the issue: `--level 3` after a full run used to
        // write a manifest of level 3 alone.
        let (mut a, dir) = corpus("one-level", true);
        a.levels = vec!["3".to_string()];
        std::fs::write(dir.join("out/no7z/hsk3-0001.mp3.tmp"), b"half a clip").unwrap();
        run(&a).expect("nothing to synthesise, so no model is needed");

        let m = manifest_at(&dir);
        let ids: Vec<&str> = m.phrases.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["hsk1-0001", "hsk3-0001"]);
        assert!(
            !dir.join("out/no7z/hsk3-0001.mp3.tmp").exists(),
            "the partial is cleaned up"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_changed_phrase_is_synthesised_rather_than_trusted() {
        // A finished clip is on disk, which the old code kept on its size alone;
        // the phrase's text has changed, so this run must try to synthesise —
        // which, with no model present, is an error naming the fetch script.
        let (a, dir) = corpus("changed", true);
        std::fs::write(
            &a.inputs[0],
            r#"[{"id":"hsk1-0001","hsk":1,"chinese":"你好。"},{"id":"hsk3-0001","hsk":3,"chinese":"我学汉语。"}]"#,
        )
        .unwrap();
        let err = run(&a)
            .expect_err("a stale clip must not be reused")
            .to_string();
        assert!(err.contains("fetch-tts.sh"), "{err}");
        // Nothing was written over the manifest on the way.
        assert_eq!(manifest_at(&dir).phrases[1].text, "我学中文。");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_manifest_from_before_keys_is_adopted_and_keyed() {
        let (a, dir) = corpus("legacy", false);
        run(&a).expect("same text and model: adopted, nothing to synthesise");
        for p in manifest_at(&dir).phrases {
            assert!(p.key.is_some(), "{} should now carry its key", p.id);
            assert_eq!(p.model.as_deref(), Some("vits-melo-tts-zh_en"));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_entry_whose_clip_is_gone_is_dropped() {
        let (mut a, dir) = corpus("gone", true);
        a.levels = vec!["3".to_string()];
        std::fs::remove_file(dir.join("out/no7z/hsk1-0001_slow.mp3")).unwrap();
        run(&a).unwrap();
        let ids: Vec<String> = manifest_at(&dir)
            .phrases
            .into_iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, ["hsk3-0001"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clip_paths_keep_the_names_already_shipped() {
        let (audio, files) = clip_paths(&args(Engine::MeloTts), "hsk1-0001");
        assert_eq!(audio.normal, "audio/no7z/hsk1-0001.mp3");
        assert_eq!(audio.slow, "audio/no7z/hsk1-0001_slow.mp3");
        assert_eq!(files[0], PathBuf::from("public/audio/no7z/hsk1-0001.mp3"));
        assert_eq!(
            files[1],
            PathBuf::from("public/audio/no7z/hsk1-0001_slow.mp3")
        );
        // And the manifest's URL maps back to the same file.
        assert_eq!(
            resume::clip_file(Path::new("public/audio"), &audio.slow),
            files[1]
        );
        // The partial is beside it, with the suffix `remove_partials` clears.
        assert_eq!(
            resume::partial_path(&files[0]),
            PathBuf::from("public/audio/no7z/hsk1-0001.mp3.tmp")
        );
    }
}
