//! The bookkeeping that lets `synthesize-audio` be stopped and run again.
//!
//! Kept out of the binary so it can be tested without a model or `ffmpeg`.
//!
//! ## What this replaced, and why
//!
//! The first version resumed by trusting **any clip over 1 KB** at its final
//! name and rewrote the manifest from **only the phrases of the current run**.
//! Each was wrong in a way that shipped bad audio quietly:
//!
//! * `ffmpeg` wrote straight to the clip's final name, so a run interrupted
//!   mid-encode left a partial MP3 there, over 1 KB, which the next run kept.
//! * A clip whose phrase text, model or voice had changed since was kept too,
//!   because a file existed — so the manifest paired new text with old audio.
//! * Running `--level 3` after a full run wrote a manifest of level 3 alone.
//!   Every other phrase vanished from the app while its clips stayed on disk.
//!
//! ## The rules now
//!
//! * **A file at a clip's final name is a finished one.** Encoding goes to
//!   `<clip>.mp3.tmp` ([`partial_path`]) and is renamed into place only after
//!   both speeds of the phrase encoded. A `.tmp` left behind is an interrupted
//!   run, and [`remove_partials`] deletes it on start.
//! * **A clip is reused only when it was made from what would be used now.**
//!   Each manifest entry records a [`synthesis_key`] — a digest of the text, the
//!   engine, the model, the voice settings, the speeds and the encoder settings
//!   — and [`reuse`] keeps a phrase only when that key matches and both clips are
//!   present. The key lives in the manifest rather than in a sidecar beside each
//!   clip because `public/audio/` is committed and shipped verbatim, and 3,000
//!   sidecar files would ship with it.
//! * **The manifest is merged, not replaced.** [`Merged`] starts from the
//!   manifest already on disk and replaces only the phrases the run produced.
//!   What happens to the rest is decided in [`Merged::build`]:
//!   - an entry whose clips are **gone from disk** is dropped and reported — it
//!     would be a play button with nothing behind it;
//!   - an entry for a phrase **not in this run's input files** is kept and
//!     reported. A run is often given one sentence file of several, so absence
//!     from the inputs is not evidence the phrase was retired; deleting a
//!     retired phrase is a deliberate edit, not a side effect of a partial run.
//!
//! Manifests written before keys existed have none. Every entry read from disk
//! is first **pinned** to the model its manifest recorded ([`Merged::new`]),
//! because the manifest written back carries this run's model at the top and
//! an entry without its own would silently become this run's output. Then:
//!
//! * a key-less entry whose text matches and whose pinned model is the model
//!   in use now is this tool's own older output, and is adopted
//!   ([`Reuse::Legacy`]) — so a corpus synthesised by this tool is not
//!   re-encoded wholesale the first time the new version runs;
//! * a key-less entry under **another** model, or clip files with **no entry**
//!   at all, were not made by this tool ([`foreign`]). The committed `no7z`
//!   clips are the case: the corpus's own CosyVoice2 recordings, chosen over
//!   MeloTTS on measurement. `synthesize-audio` refuses to replace them unless
//!   given `--replace-foreign-clips`, and says so before touching anything.
//!   A *keyed* entry under another model is this tool's own, and is simply
//!   synthesised again.
//!
//! ## Not handled: two runs at once
//!
//! Two runs over the same corpus at the same time would each merge into the
//! manifest they read at the start, and the later write wins. There is no lock:
//! a lock file left behind by a `Ctrl-C` — the usual way a long run ends early —
//! would block the next run, which is a worse failure than the one it prevents
//! for a tool one developer runs by hand. Run one at a time per corpus.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::{AudioManifest, AudioPaths, ManifestPhrase};

/// Bumped whenever the pipeline changes what a clip sounds like for the same
/// inputs — the normalisation target, the punctuation rewrite, the chunking —
/// so that every clip made the old way is synthesised again.
pub const KEY_VERSION: &str = "hanzi-say-clips/1";

/// FNV-1a, 64 bits. Not a security hash: it only has to be stable across Rust
/// versions and machines, which `std`'s `DefaultHasher` does not promise, and to
/// tell a few thousand inputs apart.
fn fnv1a(bytes: impl IntoIterator<Item = u8>, mut hash: u64) -> u64 {
    for b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// A digest of raw bytes, for an input that is a file (a reference clip).
pub fn digest(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a(bytes.iter().copied(), FNV_OFFSET))
}

/// The key a phrase's clips are recorded under: a digest of everything that
/// decides what they sound like.
///
/// Each part is length-prefixed, so `["ab", "c"]` and `["a", "bc"]` differ. The
/// caller passes [`KEY_VERSION`] first; it is not added here so that the parts
/// a key covers are all visible at the one call site.
pub fn synthesis_key<S: AsRef<str>>(parts: &[S]) -> String {
    let mut hash = FNV_OFFSET;
    for part in parts {
        let part = part.as_ref().as_bytes();
        hash = fnv1a((part.len() as u64).to_le_bytes(), hash);
        hash = fnv1a(part.iter().copied(), hash);
    }
    format!("{hash:016x}")
}

/// Where a file is written before it is renamed into place: `<dest>.tmp`.
pub fn partial_path(dest: &Path) -> PathBuf {
    let mut name = dest.as_os_str().to_owned();
    name.push(".tmp");
    PathBuf::from(name)
}

/// True for a file this tool writes before renaming: a clip or a manifest with
/// `.tmp` on the end. Nothing else in the directory is touched.
fn is_partial(name: &str) -> bool {
    name.ends_with(".mp3.tmp") || name.ends_with(".json.tmp")
}

/// Delete what an interrupted run left half-written in `dir`, returning what was
/// removed. A directory that does not exist yet has nothing in it.
pub fn remove_partials(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut removed = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        if is_partial(&name.to_string_lossy()) && entry.file_type()?.is_file() {
            std::fs::remove_file(entry.path())?;
            removed.push(entry.path());
        }
    }
    removed.sort();
    Ok(removed)
}

/// Write `bytes` to `dest` so that a reader never sees half of it: to
/// [`partial_path`] first, then renamed over `dest`.
pub fn write_atomically(dest: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = partial_path(dest);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, dest)
}

/// Read the manifest already on disk, if there is one.
///
/// A manifest that exists and does not parse is an error rather than "none":
/// treating it as absent would replace every entry in it with this run's subset,
/// which is the truncation this module exists to prevent.
pub fn read_manifest(path: &Path) -> Result<Option<AudioManifest>, String> {
    let body = match std::fs::read_to_string(path) {
        Ok(body) => body,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("could not read {}: {e}", path.display())),
    };
    serde_json::from_str(&body).map(Some).map_err(|e| {
        format!(
            "{} exists but is not a manifest ({e}) — refusing to overwrite it; \
             fix or remove it and run again",
            path.display()
        )
    })
}

/// The file a manifest's root-relative clip URL (`audio/<source>/<id>.mp3`)
/// names, under the output root the clips were written to.
pub fn clip_file(out: &Path, url: &str) -> PathBuf {
    out.join(url.strip_prefix("audio/").unwrap_or(url))
}

/// True when both clips exist and are not empty.
pub fn clips_present(out: &Path, audio: &AudioPaths) -> bool {
    [&audio.normal, &audio.slow].iter().all(|url| {
        !url.is_empty()
            && std::fs::metadata(clip_file(out, url))
                .map(|m| m.is_file() && m.len() > 0)
                .unwrap_or(false)
    })
}

/// Why a phrase's existing clips may be kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reuse {
    /// The entry's key is the key this run would record.
    Keyed,
    /// A manifest from before keys: same text, and the same model recorded.
    Legacy,
}

/// Whether a phrase's existing clips can stand, or it must be synthesised.
///
/// * `entry` — the phrase's entry in the manifest on disk, if it has one. Its
///   `model` is always set once it has been through [`Merged::new`], which
///   pins each entry to the model the manifest it came from recorded;
/// * `text`, `key`, `model` — what this run would synthesise it from;
/// * `present` — whether both clip files exist.
///
/// A clip on disk with no entry is never reused: nothing says what it is.
pub fn reuse(
    entry: Option<&ManifestPhrase>,
    text: &str,
    key: &str,
    model: &str,
    present: bool,
) -> Option<Reuse> {
    let entry = entry?;
    if !present {
        return None;
    }
    match entry.key.as_deref() {
        Some(k) if k == key => Some(Reuse::Keyed),
        Some(_) => None,
        None => {
            (entry.text == text && entry.model.as_deref() == Some(model)).then_some(Reuse::Legacy)
        }
    }
}

/// Why a phrase's existing clips may not be replaced without being asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Foreign {
    /// An entry with no key, recorded under another model: clips this tool did
    /// not make, such as a corpus's own recordings. Carries that model.
    OtherModel(Option<String>),
    /// Clip files at the phrase's names with no manifest entry at all — laid
    /// down by another tool, or by a run that stopped before recording them.
    Unlisted,
}

/// Whether replacing a phrase's clips would overwrite audio this tool did not
/// make, given its entry (pinned by [`Merged::new`]), this run's `model`, and
/// whether any of its clip files is on disk.
///
/// An entry *with* a key is this tool's own, whatever model it names: a changed
/// model is an ordinary re-synthesis. So is a key-less entry under this run's
/// model, which is this tool's output from before keys existed.
pub fn foreign(entry: Option<&ManifestPhrase>, model: &str, on_disk: bool) -> Option<Foreign> {
    match entry {
        Some(e) if e.key.is_none() && e.model.as_deref() != Some(model) => {
            Some(Foreign::OtherModel(e.model.clone()))
        }
        Some(_) => None,
        None if on_disk => Some(Foreign::Unlisted),
        None => None,
    }
}

/// A manifest being merged into: the one on disk, with this run's phrases
/// replacing their entries as they are produced.
#[derive(Debug)]
pub struct Merged {
    source: String,
    model: String,
    speeds: Vec<f32>,
    entries: HashMap<String, ManifestPhrase>,
    /// The ids of the manifest on disk, in its order.
    previous_order: Vec<String>,
    previous_model: Option<String>,
}

/// A manifest ready to write, and what merging decided about the entries this
/// run did not produce.
#[derive(Debug)]
pub struct Built {
    pub manifest: AudioManifest,
    /// Entries left out because a clip they name is no longer on disk.
    pub dropped_missing: Vec<String>,
    /// Entries kept although their phrase is not in this run's inputs.
    pub kept_outside: Vec<String>,
}

impl Merged {
    /// Start from `existing`, the manifest on disk (if any), for a run that
    /// writes `source` with `model` at `speeds`.
    ///
    /// A manifest for a different corpus is refused: merging the two would put
    /// one corpus's clips under the other's licence notice.
    pub fn new(
        existing: Option<AudioManifest>,
        source: &str,
        model: &str,
        speeds: Vec<f32>,
    ) -> Result<Self, String> {
        let mut merged = Self {
            source: source.to_string(),
            model: model.to_string(),
            speeds,
            entries: HashMap::new(),
            previous_order: Vec::new(),
            previous_model: None,
        };
        if let Some(mut existing) = existing {
            if existing.source != source {
                return Err(format!(
                    "the manifest on disk is for {:?}, not {source:?} — refusing to merge them",
                    existing.source
                ));
            }
            // Each entry is pinned to the model its manifest recorded. The top-
            // level `model` written back is this run's, so an entry left
            // without its own would read as this run's output from then on —
            // and the corpus's own recordings would be adopted as MeloTTS.
            for phrase in &mut existing.phrases {
                if phrase.model.is_none() {
                    phrase.model = Some(existing.model.clone());
                }
            }
            merged.previous_model = Some(existing.model);
            for phrase in existing.phrases {
                if !merged.entries.contains_key(&phrase.id) {
                    merged.previous_order.push(phrase.id.clone());
                }
                merged.entries.insert(phrase.id.clone(), phrase);
            }
        }
        Ok(merged)
    }

    /// The entry for `id`, from disk or from this run.
    pub fn entry(&self, id: &str) -> Option<&ManifestPhrase> {
        self.entries.get(id)
    }

    /// The model the manifest on disk recorded, for [`reuse`].
    pub fn previous_model(&self) -> Option<&str> {
        self.previous_model.as_deref()
    }

    /// Record a phrase this run produced or confirmed, replacing any entry it had.
    pub fn insert(&mut self, entry: ManifestPhrase) {
        self.entries.insert(entry.id.clone(), entry);
    }

    /// The merged manifest.
    ///
    /// `order` is every phrase id in this run's input files, in file order —
    /// *before* `--level` or `--limit` narrowed them — so a partial run writes
    /// the same order a full one does. Entries outside it keep their previous
    /// order after it. `present` says whether an entry's clips are on disk.
    pub fn build(&self, order: &[String], present: impl Fn(&ManifestPhrase) -> bool) -> Built {
        let inputs: HashSet<&str> = order.iter().map(String::as_str).collect();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut phrases = Vec::new();
        let mut dropped_missing = Vec::new();
        let mut kept_outside = Vec::new();

        let ids = order.iter().chain(self.previous_order.iter());
        for id in ids {
            if !seen.insert(id.as_str()) {
                continue;
            }
            let Some(entry) = self.entries.get(id) else {
                continue;
            };
            if !present(entry) {
                dropped_missing.push(id.clone());
                continue;
            }
            if !inputs.contains(id.as_str()) {
                kept_outside.push(id.clone());
            }
            phrases.push(entry.clone());
        }

        Built {
            manifest: AudioManifest {
                source: self.source.clone(),
                model: self.model.clone(),
                speeds: self.speeds.clone(),
                phrases,
            },
            dropped_missing,
            kept_outside,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, text: &str, key: Option<&str>) -> ManifestPhrase {
        ManifestPhrase {
            id: id.to_string(),
            level: "1".to_string(),
            text: text.to_string(),
            pinyin: String::new(),
            translation: String::new(),
            audio: AudioPaths {
                normal: format!("audio/src/{id}.mp3"),
                slow: format!("audio/src/{id}_slow.mp3"),
            },
            bytes: 10,
            model: None,
            key: key.map(str::to_string),
        }
    }

    fn manifest(model: &str, phrases: Vec<ManifestPhrase>) -> AudioManifest {
        AudioManifest {
            source: "src".to_string(),
            model: model.to_string(),
            speeds: vec![1.0, 0.7],
            phrases,
        }
    }

    fn ids(m: &AudioManifest) -> Vec<&str> {
        m.phrases.iter().map(|p| p.id.as_str()).collect()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("hanzi-say-resume-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_key_changes_with_every_part() {
        let base = ["v1", "melotts", "model-a", "1.0,0.7", "你好"];
        let key = synthesis_key(&base);
        assert_eq!(key, synthesis_key(&base), "a key must be deterministic");
        assert_eq!(key.len(), 16);
        for i in 0..base.len() {
            let mut changed = base;
            changed[i] = "other";
            assert_ne!(synthesis_key(&changed), key, "part {i} is not in the key");
        }
        // Length-prefixed, so moving a boundary is a different key.
        assert_ne!(synthesis_key(&["ab", "c"]), synthesis_key(&["a", "bc"]));
    }

    #[test]
    fn the_key_is_stable_across_builds() {
        // Pinned, because a key that changed with the toolchain would make every
        // committed clip look stale and be synthesised again.
        assert_eq!(synthesis_key::<&str>(&[]), "cbf29ce484222325");
        assert_eq!(digest(b"a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn a_matching_key_is_reused_and_any_other_is_not() {
        let e = entry("p1", "你好", Some("k1"));
        assert_eq!(reuse(Some(&e), "你好", "k1", "m", true), Some(Reuse::Keyed));
        // Text, model or voice changed: the key differs, and the clip is redone.
        assert_eq!(reuse(Some(&e), "你好", "k2", "m", true), None);
        // The key matches but a clip is gone.
        assert_eq!(reuse(Some(&e), "你好", "k1", "m", false), None);
        // A clip on disk with no entry at all says nothing about what it is.
        assert_eq!(reuse(None, "你好", "k1", "m", true), None);
    }

    #[test]
    fn a_legacy_entry_is_adopted_only_for_the_same_text_and_model() {
        let legacy = |model: Option<&str>| {
            let mut e = entry("p1", "你好", None);
            e.model = model.map(str::to_string);
            e
        };
        let e = legacy(Some("melo"));
        assert_eq!(
            reuse(Some(&e), "你好", "k", "melo", true),
            Some(Reuse::Legacy)
        );
        // The phrase's text has changed since.
        assert_eq!(reuse(Some(&e), "您好", "k", "melo", true), None);
        // A different model made it — the no7z case, the corpus's own audio.
        let theirs = legacy(Some("cosyvoice2-0.5b (dataset's own audio)"));
        assert_eq!(reuse(Some(&theirs), "你好", "k", "melo", true), None);
        // No model recorded at all is not a match.
        assert_eq!(reuse(Some(&legacy(None)), "你好", "k", "melo", true), None);
        assert_eq!(reuse(Some(&e), "你好", "k", "melo", false), None);
    }

    #[test]
    fn only_clips_this_tool_did_not_make_are_foreign() {
        let with = |key: Option<&str>, model: Option<&str>| {
            let mut e = entry("p1", "你好", key);
            e.model = model.map(str::to_string);
            e
        };
        let dataset = "cosyvoice2-0.5b (dataset's own audio)";
        // The corpus's own recordings: no key, another model.
        assert_eq!(
            foreign(Some(&with(None, Some(dataset))), "melo", true),
            Some(Foreign::OtherModel(Some(dataset.to_string())))
        );
        assert_eq!(
            foreign(Some(&with(None, None)), "melo", true),
            Some(Foreign::OtherModel(None))
        );
        // This tool's own, keyed under an older model, or key-less under this one.
        assert_eq!(
            foreign(Some(&with(Some("k"), Some("old"))), "melo", true),
            None
        );
        assert_eq!(foreign(Some(&with(None, Some("melo"))), "melo", true), None);
        // Files with no entry, and nothing at all.
        assert_eq!(foreign(None, "melo", true), Some(Foreign::Unlisted));
        assert_eq!(foreign(None, "melo", false), None);
    }

    #[test]
    fn a_partial_run_merges_instead_of_truncating() {
        // A full run wrote p1..p3; `--level 3` then produced p2 alone.
        let existing = manifest(
            "melo",
            vec![
                entry("p1", "一", Some("a")),
                entry("p2", "二", Some("b")),
                entry("p3", "三", Some("c")),
            ],
        );
        let mut merged = Merged::new(Some(existing), "src", "melo", vec![1.0, 0.7]).unwrap();
        merged.insert(entry("p2", "二二", Some("b2")));
        let order: Vec<String> = ["p1", "p2", "p3"].map(String::from).to_vec();
        let built = merged.build(&order, |_| true);
        assert_eq!(ids(&built.manifest), ["p1", "p2", "p3"]);
        assert_eq!(
            built.manifest.phrases[1].text, "二二",
            "the run's entry replaces the old one"
        );
        assert_eq!(built.manifest.phrases[0].text, "一", "the others are kept");
        assert!(built.dropped_missing.is_empty() && built.kept_outside.is_empty());
    }

    #[test]
    fn the_merged_order_follows_the_inputs_not_the_run() {
        // Level 3 synthesised first, then level 1: the manifest still reads in
        // file order, the order a full run would have written.
        let existing = manifest("melo", vec![entry("p3", "三", Some("c"))]);
        let mut merged = Merged::new(Some(existing), "src", "melo", vec![1.0, 0.7]).unwrap();
        merged.insert(entry("p1", "一", Some("a")));
        let order: Vec<String> = ["p1", "p2", "p3"].map(String::from).to_vec();
        assert_eq!(ids(&merged.build(&order, |_| true).manifest), ["p1", "p3"]);
    }

    #[test]
    fn entries_with_missing_clips_are_dropped_and_others_outside_the_inputs_kept() {
        let existing = manifest(
            "melo",
            vec![
                entry("gone", "一", Some("a")),
                entry("other-file", "二", Some("b")),
                entry("p1", "三", Some("c")),
            ],
        );
        let merged = Merged::new(Some(existing), "src", "melo", vec![1.0, 0.7]).unwrap();
        let built = merged.build(&["p1".to_string()], |e| e.id != "gone");
        assert_eq!(ids(&built.manifest), ["p1", "other-file"]);
        assert_eq!(built.dropped_missing, ["gone"]);
        assert_eq!(built.kept_outside, ["other-file"]);
    }

    #[test]
    fn existing_entries_keep_the_model_they_were_made_with() {
        // The manifest on disk records the dataset's model for itself and none
        // per entry. Once merged, the top-level model is this run's — so each
        // carried-over entry has to say its own, or it would read as this run's.
        let mut own = entry("own", "二", Some("k"));
        own.model = Some("melo".to_string());
        let existing = manifest("dataset", vec![entry("theirs", "一", None), own]);
        let merged = Merged::new(Some(existing), "src", "melo", vec![1.0, 0.7]).unwrap();
        assert_eq!(
            merged.entry("theirs").unwrap().model.as_deref(),
            Some("dataset")
        );
        assert_eq!(merged.entry("own").unwrap().model.as_deref(), Some("melo"));
        let built = merged.build(&[], |_| true);
        assert_eq!(built.manifest.model, "melo");
        assert_eq!(built.manifest.phrases[0].model.as_deref(), Some("dataset"));
        // And it is no longer adoptable as this run's output.
        let theirs = merged.entry("theirs");
        assert_eq!(reuse(theirs, "一", "k2", "melo", true), None);
    }

    #[test]
    fn a_manifest_for_another_corpus_is_refused() {
        let mut other = manifest("melo", vec![]);
        other.source = "harukicoder".to_string();
        assert!(Merged::new(Some(other), "no7z", "melo", vec![1.0]).is_err());
    }

    #[test]
    fn a_fresh_manifest_records_the_run() {
        let mut merged = Merged::new(None, "src", "melo", vec![1.0, 0.7]).unwrap();
        assert_eq!(merged.previous_model(), None);
        merged.insert(entry("p1", "一", Some("a")));
        let built = merged.build(&["p1".to_string()], |_| true);
        assert_eq!(built.manifest.model, "melo");
        assert_eq!(built.manifest.source, "src");
        assert_eq!(ids(&built.manifest), ["p1"]);
    }

    #[test]
    fn keys_survive_a_round_trip_and_old_manifests_still_parse() {
        let mut e = entry("p1", "一", Some("abc"));
        e.model = Some("melo".to_string());
        let m = manifest("melo", vec![e.clone()]);
        let back: AudioManifest =
            serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.phrases[0], e);
        // A manifest from before the fields existed — what is committed today.
        let old = r#"{"source":"src","model":"melo","speeds":[1.0,0.7],"phrases":[
            {"id":"p1","level":"1","text":"一","pinyin":"","translation":"",
             "audio":{"normal":"audio/src/p1.mp3","slow":"audio/src/p1_slow.mp3"},"bytes":10}]}"#;
        let old: AudioManifest = serde_json::from_str(old).unwrap();
        assert_eq!(old.phrases[0].key, None);
        // And an entry without them writes no `null`s for the frontend to read.
        let written = serde_json::to_string(&old).unwrap();
        assert!(
            !written.contains("key") && !written.contains("\"model\":null"),
            "{written}"
        );
    }

    #[test]
    fn a_partial_file_is_cleaned_up_and_nothing_else_is() {
        let dir = scratch("partials");
        for name in [
            "a.mp3",
            "a.mp3.tmp",
            "manifest.json",
            "manifest.json.tmp",
            "README.md",
            "notes.tmp",
        ] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        let removed = remove_partials(&dir).unwrap();
        let names: Vec<String> = removed
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.mp3.tmp", "manifest.json.tmp"]);
        for kept in ["a.mp3", "manifest.json", "README.md", "notes.tmp"] {
            assert!(
                dir.join(kept).exists(),
                "{kept} should not have been touched"
            );
        }
        assert!(remove_partials(&dir.join("absent")).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_atomic_write_leaves_no_partial_behind() {
        let dir = scratch("atomic");
        let dest = dir.join("manifest.json");
        write_atomically(&dest, b"{}").unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"{}");
        assert!(!partial_path(&dest).exists());
        assert_eq!(partial_path(&dest), dir.join("manifest.json.tmp"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unreadable_manifest_is_an_error_not_an_empty_one() {
        let dir = scratch("unreadable");
        let path = dir.join("manifest.json");
        assert_eq!(
            read_manifest(&path).unwrap(),
            None,
            "no file is no manifest"
        );
        std::fs::write(&path, "{ not json").unwrap();
        let err = read_manifest(&path).unwrap_err();
        assert!(err.contains("refusing to overwrite"), "{err}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clips_are_present_only_when_both_files_have_content() {
        let dir = scratch("present");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let e = entry("p1", "一", None);
        assert!(!clips_present(&dir, &e.audio));
        std::fs::write(dir.join("src/p1.mp3"), b"mp3").unwrap();
        assert!(!clips_present(&dir, &e.audio), "the slow take is missing");
        std::fs::write(dir.join("src/p1_slow.mp3"), b"").unwrap();
        assert!(
            !clips_present(&dir, &e.audio),
            "an empty file is not a clip"
        );
        std::fs::write(dir.join("src/p1_slow.mp3"), b"mp3").unwrap();
        assert!(clips_present(&dir, &e.audio));
        assert_eq!(clip_file(&dir, "audio/src/p1.mp3"), dir.join("src/p1.mp3"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
