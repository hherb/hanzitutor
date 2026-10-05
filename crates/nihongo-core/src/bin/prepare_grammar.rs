//! Turn OpenJLPT's grammar dataset into the compact artifact the app embeds.
//!
//! One input tree, fetched by `scripts/fetch-openjlpt.sh` into
//! `data/raw/openjlpt/`, and the five files it holds are the whole story:
//!
//! * `n5.json` … `n1.json` — the published dataset, one file per level, each in
//!   teaching order, each entry carrying its pattern, romaji, meaning, formation,
//!   notes, tags and examples. The **level is not read**: there has been no
//!   official JLPT grammar list since 2010, the corpus's own NOTICE calls its
//!   levels a consensus of preparation materials, and `HANDOVER_NIHONGO.md`
//!   invariant 20 forbids the interface from implying otherwise. The five files
//!   become one course, in the order they give — see `nihongo_core::grammar`.
//! * `meta.json` — the dataset's own version and the counts it claims, which are
//!   checked against what was read rather than trusted: a rebuild from a different
//!   release fails here rather than quietly changing the course's size.
//! * `LICENSE` and `NOTICE.md` — CC BY-SA 4.0 and the attribution the licence
//!   requires. The notice file is read for its first line only, as the statement
//!   that the grammar content is the project's own; the full attribution is a
//!   notice of its own in the app.
//!
//! Run: `pnpm run prepare-grammar`. It needs the **analyser** as well as the raw
//! corpus: the corpus supplies its own furigana markup for every kanji, so the
//! examples as drawn need no analyser at all, but the sentence records which of the
//! course's words it uses, and that link must be UniDic's rather than the markup's
//! — the corpus's furigana marks readings, not word boundaries, and the obvious
//! rule built on it was measured and rejected ([`nihongo_core::grammar`] and
//! [`GrammarExample::words`] carry the numbers). So this binary is behind
//! `tokenize` and needs `scripts/fetch-unidic.sh` to have run once, like
//! `prepare-passages` and `prepare-phrases`.
//!
//! The inputs are therefore the corpus, the **committed words artifact** (the
//! vocabulary a token is looked up in) and the built dictionary.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

use nihongo_core::grammar::{
    parse_furigana, strip_level_references, GrammarArtifact, GrammarDataset, GrammarExample,
    GrammarPoint, GrammarSource, GRAMMAR_ARTIFACT_MAGIC,
};
use nihongo_core::segment::analyse;
use nihongo_core::words::{is_kanji, Ruby};
use nihongo_core::WordDataset;
use serde::Deserialize;

/// The upstream dataset, as published.
const CORPUS: &str = "OpenJLPT";
const CORPUS_URL: &str = "https://github.com/evanclan/OpenJLPT";
const LICENCE: &str = "CC BY-SA 4.0";

/// The analyser this pipeline links the examples' vocabulary with, and the
/// dictionary it uses.
///
/// Constants for the reason `prepare-passages` states them: the artifact records
/// both, and half a version is worse than none. `LINDERA_VERSION` must track the
/// `lindera` requirement in `Cargo.toml`, and `UNIDIC_VERSION` is the archive
/// `lindera-unidic`'s build script fetches — `scripts/fetch-unidic.sh` repeats its
/// name and MD5 so a change to it shows up in a diff here.
const LINDERA_VERSION: &str = "6.2";
const UNIDIC_VERSION: &str = "unidic-mecab-2.1.2";

/// The level files, in the order a course meets them.
///
/// This is the **only** place the level appears, and it is used as a file name and
/// an order rather than as a label: nothing downstream stores or shows it.
const LEVEL_FILES: [&str; 5] = ["n5.json", "n4.json", "n3.json", "n2.json", "n1.json"];

/// One entry of the published dataset, in the shape it publishes.
#[derive(Deserialize)]
struct Entry {
    id: String,
    pattern: String,
    #[serde(default)]
    reading: Option<String>,
    romaji: String,
    meaning: String,
    formation: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    examples: Vec<SourceExample>,
}

#[derive(Deserialize)]
struct SourceExample {
    ja: String,
    en: String,
    #[serde(default)]
    furigana: Option<String>,
}

#[derive(Deserialize)]
struct Meta {
    version: String,
    #[serde(default)]
    counts: Option<Counts>,
}

#[derive(Deserialize)]
struct Counts {
    #[serde(default)]
    grammar: Option<GrammarCounts>,
}

#[derive(Deserialize)]
struct GrammarCounts {
    #[serde(default)]
    total: Option<usize>,
}

/// The selection rule, in one sentence, as the artifact states it.
const SELECTION: &str = "all 526 grammar points the corpus publishes, in the order its five \
                         level files give them; the level labels are dropped and the files are \
                         read as one course";

/// What was done to the text on the way in, as CC BY-SA 4.0 §3(a)(1)(B) requires.
const MODIFICATIONS: &str = "the JLPT level references in the notes were removed; the examples' \
                             furigana markup was parsed into ruby segments; no text was added or \
                             reworded. Each example also records which of the course's own words \
                             its sentence uses, from UniDic's segmentation — an index over the \
                             sentence rather than a change to it, and the sentence is stored and \
                             shown exactly as the corpus wrote it";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-grammar: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/crates/nihongo-core")
        .to_path_buf();

    let mut raw_dir = repo_root.join("data/raw/openjlpt");
    let mut out_path = crate_dir.join("data/grammar.bin.gz");
    let mut words_path = crate_dir.join("data/words.bin.gz");
    let mut dictionary: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw_dir = PathBuf::from(args.next().ok_or("--raw needs a directory")?),
            "--out" => out_path = PathBuf::from(args.next().ok_or("--out needs a file")?),
            "--words" => words_path = PathBuf::from(args.next().ok_or("--words needs a file")?),
            "--dictionary" => {
                dictionary = Some(PathBuf::from(args.next().ok_or("--dictionary needs a path")?))
            }
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    // ---- The vocabulary the examples are linked to, and the analyser --------
    if !words_path.exists() {
        return Err(format!(
            "missing {} — run pnpm run prepare-words first: the artifact decides which of a \
             sentence's words the course teaches, and there is nothing to link to without it",
            words_path.display()
        )
        .into());
    }
    let words = WordDataset::from_gzip_bytes(&std::fs::read(&words_path)?)
        .map_err(|e| format!("{}: {e}", words_path.display()))?;

    let dictionary_dir = match dictionary {
        Some(dir) => dir,
        None => find_dictionary(&repo_root).ok_or_else(|| {
            format!(
                "no UniDic dictionary under {} — run scripts/fetch-unidic.sh once (it downloads \
                 about 134 MB and builds a dictionary from it), or pass --dictionary",
                repo_root.join(".lindera").display()
            )
        })?,
    };
    let dictionary = load_dictionary(dictionary_dir.to_string_lossy().as_ref())
        .map_err(|e| format!("could not load the dictionary at {}: {e}", dictionary_dir.display()))?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let meta_path = raw_dir.join("meta.json");
    for name in ["meta.json", "LICENSE", "NOTICE.md"] {
        let path = raw_dir.join(name);
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-openjlpt.sh first",
                path.display()
            )
            .into());
        }
    }
    for name in LEVEL_FILES {
        let path = raw_dir.join(name);
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-openjlpt.sh first",
                path.display()
            )
            .into());
        }
    }

    // ---- What the dataset says it is ----------------------------------------
    let meta: Meta = serde_json::from_reader(BufReader::new(File::open(&meta_path)?))
        .map_err(|e| format!("{}: {e}", meta_path.display()))?;
    let notice = std::fs::read_to_string(raw_dir.join("NOTICE.md"))?;
    let licence_file = std::fs::read_to_string(raw_dir.join("LICENSE"))?;
    if !licence_file.contains("Attribution-ShareAlike 4.0") {
        return Err(format!(
            "{} no longer states CC BY-SA 4.0 — the licence this artifact is redistributed \
             under. Read it before rebuilding.",
            raw_dir.join("LICENSE").display()
        )
        .into());
    }
    if !notice.contains("CC BY-SA 4.0") {
        return Err(format!(
            "{} no longer states the share-alike licence the grammar content is under",
            raw_dir.join("NOTICE.md").display()
        )
        .into());
    }
    // The whole reason only the grammar is taken from this repository: the levels
    // rest on an unverified upstream grant, while the grammar dataset is the
    // project's own writing. The notice is what says so, and a notice that stops
    // saying it is a licence question rather than a rebuild.
    // Compared with its whitespace collapsed: the sentence is wrapped across lines
    // upstream, and a re-wrap must not fail a licence check.
    let notice_flat = notice.split_whitespace().collect::<Vec<_>>().join(" ");
    if !notice_flat.contains("original content written for the project") {
        return Err(format!(
            "{} no longer states that the grammar dataset is the project's own original \
             content — which is the basis for taking it and nothing else from that \
             repository (invariant 38). Read the notice before rebuilding.",
            raw_dir.join("NOTICE.md").display()
        )
        .into());
    }

    // ---- The five level files, as one course --------------------------------
    let mut points: Vec<GrammarPoint> = Vec::new();
    let mut per_level: Vec<(&str, usize)> = Vec::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    let mut tags_seen: BTreeSet<String> = BTreeSet::new();
    let mut examples_total = 0usize;
    let mut ruby_spans = 0usize;
    let mut examples_without_ruby: Vec<String> = Vec::new();
    let mut notes_changed: Vec<String> = Vec::new();
    let mut readings = 0usize;
    let mut examples_with_words = 0usize;
    let mut word_links = 0usize;
    let mut points_with_words = 0usize;

    for name in LEVEL_FILES {
        let path = raw_dir.join(name);
        let entries: Vec<Entry> = serde_json::from_reader(BufReader::new(File::open(&path)?))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        per_level.push((name, entries.len()));

        for entry in entries {
            if entry.id.trim().is_empty() {
                return Err(format!("{}: an entry has no id ({:?})", path.display(), entry.pattern).into());
            }
            if !seen_ids.insert(entry.id.clone()) {
                return Err(format!(
                    "the id {:?} appears twice; the corpus's ids are the artifact's keys",
                    entry.id
                )
                .into());
            }
            if entry.pattern.trim().is_empty() || entry.romaji.trim().is_empty() {
                return Err(format!("{}: {:?} has no pattern or no romaji", path.display(), entry.id).into());
            }
            if entry.meaning.trim().is_empty() || entry.formation.trim().is_empty() {
                return Err(format!(
                    "{}: {:?} has no meaning or no formation; both are what the card draws",
                    path.display(),
                    entry.id
                )
                .into());
            }
            if entry.pattern.chars().any(is_kanji) && entry.reading.is_none() {
                return Err(format!(
                    "{}: {:?} is written with a kanji and has no reading",
                    path.display(),
                    entry.id
                )
                .into());
            }
            if entry.reading.is_some() {
                readings += 1;
            }
            if entry.tags.is_empty() {
                return Err(format!("{}: {:?} carries no tag", path.display(), entry.id).into());
            }
            for tag in &entry.tags {
                if tag.trim().is_empty() || tag.chars().any(char::is_whitespace) {
                    return Err(format!(
                        "{}: {:?} has the tag {tag:?}, which is not a tag the corpus's schema allows",
                        path.display(),
                        entry.id
                    )
                    .into());
                }
                tags_seen.insert(tag.clone());
            }
            if entry.examples.len() < 2 {
                return Err(format!(
                    "{}: {:?} has {} example(s); a pattern with fewer than two is one the \
                     corpus does not consider taught",
                    path.display(),
                    entry.id,
                    entry.examples.len()
                )
                .into());
            }

            let mut examples = Vec::with_capacity(entry.examples.len());
            for source in entry.examples {
                if source.ja.trim().is_empty() || source.en.trim().is_empty() {
                    return Err(format!(
                        "{}: {:?} has an example with no sentence or no translation",
                        path.display(),
                        entry.id
                    )
                    .into());
                }
                let ruby = match source.furigana.as_deref() {
                    Some(markup) => parse_furigana(&source.ja, markup).map_err(|e| {
                        format!("{}: {:?}: {e}", path.display(), entry.id)
                    })?,
                    None => {
                        // A sentence with no kanji needs no reading, and it still
                        // carries one plain segment: the ruby list then always
                        // reassembles the sentence, which is what lets the screen,
                        // the test and `text_from_ruby` treat every example alike.
                        examples_without_ruby.push(source.ja.clone());
                        vec![Ruby::new(source.ja.clone(), None)]
                    }
                };
                ruby_spans += ruby.iter().filter(|part| part.rt.is_some()).count();
                // Which of the course's words this sentence uses: the analyser's
                // answer, kept in the sentence's own order and each named once.
                // Nothing is refused here — a grammar example is deliberately not
                // held to the vocabulary (invariant 38) — so an example that uses
                // no word the course teaches simply carries none.
                let mut sentence_words: Vec<String> = Vec::new();
                for token in analyse(&segmenter, &source.ja, &words)
                    .map_err(|e| format!("{}: {:?}: {e}", path.display(), entry.id))?
                {
                    if let Some(word) = token.word {
                        if !sentence_words.contains(&word) {
                            sentence_words.push(word);
                        }
                    }
                }
                if !sentence_words.is_empty() {
                    examples_with_words += 1;
                    word_links += sentence_words.len();
                }
                let example = GrammarExample {
                    ja: source.ja,
                    en: source.en,
                    ruby,
                    words: sentence_words,
                };
                // The invariant that replaces the vocabulary rule: a kanji with no
                // reading over it is one the learner cannot read, and a grammar
                // example has no word card to fall back on.
                let uncovered = example.kanji_without_ruby();
                if !uncovered.is_empty() {
                    return Err(format!(
                        "{}: {:?}: {:?} has no reading over {:?}",
                        path.display(),
                        entry.id,
                        example.ja,
                        uncovered.iter().collect::<String>()
                    )
                    .into());
                }
                if example.has_kanji() && example.ruby.is_empty() {
                    return Err(format!(
                        "{}: {:?}: {:?} contains a kanji and carries no furigana at all",
                        path.display(),
                        entry.id,
                        example.ja
                    )
                    .into());
                }
                examples.push(example);
            }
            examples_total += examples.len();

            let notes = entry.notes.map(|note| {
                let stripped = strip_level_references(&note);
                if stripped != note {
                    notes_changed.push(entry.id.clone());
                }
                stripped
            });
            if let Some(note) = &notes {
                if note.trim().is_empty() {
                    return Err(format!(
                        "{}: {:?} has a note that is empty once the level references are removed",
                        path.display(),
                        entry.id
                    )
                    .into());
                }
            }

            points.push(GrammarPoint {
                id: entry.id,
                pattern: entry.pattern,
                reading: entry.reading,
                romaji: entry.romaji,
                meaning: entry.meaning,
                formation: entry.formation,
                notes,
                tags: entry.tags,
                examples,
            });
            if points
                .last()
                .is_some_and(|point: &GrammarPoint| !point.words().is_empty())
            {
                points_with_words += 1;
            }
        }
    }

    // ---- The counts the dataset claims, checked rather than trusted ---------
    let claimed = meta
        .counts
        .as_ref()
        .and_then(|counts| counts.grammar.as_ref())
        .and_then(|grammar| grammar.total);
    if let Some(claimed) = claimed {
        if claimed != points.len() {
            return Err(format!(
                "the corpus's meta.json claims {claimed} grammar points and {} were read — \
                 upstream moved, and the milestone's numbers need re-taking rather than \
                 silently changing",
                points.len()
            )
            .into());
        }
    }
    if points.is_empty() {
        return Err("no grammar points were read; the artifact would teach nothing".into());
    }

    let source = GrammarSource {
        corpus: CORPUS.to_string(),
        url: CORPUS_URL.to_string(),
        version: meta.version.clone(),
        licence: LICENCE.to_string(),
        retrieved: retrieved_date(&raw_dir),
        analyser: format!("lindera {LINDERA_VERSION}"),
        dictionary: UNIDIC_VERSION.to_string(),
        selection: SELECTION.to_string(),
        modifications: MODIFICATIONS.to_string(),
    };
    let dataset = GrammarDataset::from_points(points, source.clone());

    // ---- Write it ----------------------------------------------------------
    let payload = postcard::to_allocvec(&GrammarArtifact::new(
        dataset.points().to_vec(),
        source,
    ))?;
    let mut raw = Vec::with_capacity(payload.len() + GRAMMAR_ARTIFACT_MAGIC.len());
    raw.extend_from_slice(GRAMMAR_ARTIFACT_MAGIC);
    raw.extend_from_slice(&payload);

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut encoder = flate2::write::GzEncoder::new(
        BufWriter::new(File::create(&out_path)?),
        flate2::Compression::best(),
    );
    encoder.write_all(&raw)?;
    let mut writer = encoder.finish()?;
    writer.flush()?;
    let artifact_bytes = std::fs::metadata(&out_path)?.len();

    let per_level: Vec<String> = per_level
        .iter()
        .map(|(name, count)| format!("{}:{count}", name.trim_end_matches(".json")))
        .collect();
    println!("prepare-grammar: wrote {}", out_path.display());
    println!("  corpus                 {CORPUS} {} ({CORPUS_URL})", meta.version);
    println!("  licence                {LICENCE}, share-alike stated in the notice");
    println!("  points                 {} from {per_level:?}", dataset.len());
    println!(
        "  levels                 dropped: the five files are one course, in the order they give"
    );
    println!(
        "  examples               {examples_total} with {ruby_spans} ruby spans ({})",
        if examples_without_ruby.is_empty() {
            "every one annotated".to_string()
        } else {
            format!("{} with no kanji to annotate", examples_without_ruby.len())
        }
    );
    println!("  readings               {readings} patterns carry a kana reading");
    println!("  tags                   {} distinct", tags_seen.len());
    println!(
        "  words                  {points_with_words} of {} patterns have a linked word; \
         {examples_with_words} of {examples_total} examples use one, {word_links} links",
        dataset.len(),
    );
    println!(
        "  analyser               lindera {LINDERA_VERSION} with {UNIDIC_VERSION}"
    );
    println!(
        "  notes                  {} of {} carry one; {} had level references removed",
        dataset.points().iter().filter(|p| p.notes.is_some()).count(),
        dataset.len(),
        notes_changed.len()
    );
    println!(
        "  artifact               {:.0} KB compressed from {:.0} KB ({:.0}%)",
        artifact_bytes as f64 / 1e3,
        raw.len() as f64 / 1e3,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    Ok(())
}

/// The date `scripts/fetch-openjlpt.sh` recorded, or `unknown`.
///
/// Read from the provenance file rather than from the clock, so a rebuild from the
/// same raw tree produces the same artifact byte for byte.
fn retrieved_date(raw_dir: &std::path::Path) -> String {
    let path = raw_dir.join("PROVENANCE.txt");
    let Ok(text) = std::fs::read_to_string(path) else {
        return "unknown".to_string();
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# fetched ") {
            return rest.trim().to_string();
        }
    }
    "unknown".to_string()
}

/// Find the built dictionary under `<root>/.lindera/`.
///
/// The dictionary sits at `.lindera/<crate version>-fmt<format version>/lindera-unidic`,
/// where the last component is `lindera-unidic`'s own `FetchParams::output_dir` and
/// the middle one is a version this crate does not name. So it is found rather than
/// spelled out, as `prepare-passages` and `prepare-phrases` do.
fn find_dictionary(root: &Path) -> Option<PathBuf> {
    let cache = root.join(".lindera");
    let entries = std::fs::read_dir(cache).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("lindera-unidic");
        if candidate.join("metadata.json").is_file() {
            return Some(candidate);
        }
    }
    None
}
