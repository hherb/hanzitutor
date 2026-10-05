//! Turn the passages written in `data/passages/` into the artifact the reading
//! screen draws.
//!
//! # What this is for, and why the analyser is here rather than in the app
//!
//! Japanese has no spaces, so a passage has to be *segmented* before a learner can
//! tap a word — and the reading over each word comes from knowing what the word is,
//! not from its characters. That is morphological analysis, and it needs a
//! dictionary: **UniDic, 134 MB of it**, which no desktop application should ship
//! and no learner should download.
//!
//! So the analysis happens **here, at build time**, and the artifact carries the
//! result: the app renders tokens and their readings and never tokenises anything.
//! `scripts/fetch-unidic.sh` fetches and builds the dictionary into `.lindera/`
//! (gitignored, ~500 MB unpacked), which is why this binary is behind the
//! `tokenize` feature and why nothing in the shipped app depends on it.
//!
//! # What it refuses to write
//!
//! A passage containing a **kanji the vocabulary does not teach**. Such a passage
//! asks a learner to read a character they have no card for and cannot tap, so it
//! is an error naming every offending token rather than a warning — the same
//! discipline `prepare-kanji` and `prepare-words` use for their own inputs. Kana,
//! particles and punctuation are free: the constraint is on kanji.
//!
//! Run: `pnpm run prepare-passages` (which needs `scripts/fetch-unidic.sh` to have
//! been run once, and `pnpm run prepare-words` for the vocabulary it checks
//! against).

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nihongo_core::passages::{
    parse_passage, Passage, PassageToken, PassagesArtifact, PassageDataset, PassagesSource,
    PASSAGES_ARTIFACT_MAGIC,
};
use nihongo_core::segment::analyse;
use nihongo_core::WordDataset;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

/// The analyser this pipeline segments with, and the dictionary it uses.
///
/// Constants rather than values read from the dependency at run time, because the
/// artifact records them and half a version is worse than none: `LINDERA_VERSION`
/// must track the `lindera` requirement in `Cargo.toml`, and `UNIDIC_VERSION` is
/// the archive `lindera-unidic`'s build script fetches — whose name and **MD5**
/// are in that crate's `build.rs`, repeated in `scripts/fetch-unidic.sh` so that a
/// change to it shows up in a diff here rather than only inside a dependency.
const LINDERA_VERSION: &str = "6.2";
const UNIDIC_VERSION: &str = "unidic-mecab-2.1.2";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-passages: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // The dictionary cache is the *repository's*, not the crate's: `.lindera/` sits
    // beside `crates/`, because it belongs to the build rather than to this crate.
    let repo_root = crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/crates/nihongo-core")
        .to_path_buf();
    let mut passages_dir = crate_dir.join("data/passages");
    let mut out_path = crate_dir.join("data/passages.bin.gz");
    let mut words_path = crate_dir.join("data/words.bin.gz");
    let mut dictionary: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--passages" => {
                passages_dir = PathBuf::from(args.next().ok_or("--passages needs a directory")?)
            }
            "--out" => out_path = PathBuf::from(args.next().ok_or("--out needs a file")?),
            "--words" => words_path = PathBuf::from(args.next().ok_or("--words needs a file")?),
            "--dictionary" => {
                dictionary = Some(PathBuf::from(args.next().ok_or("--dictionary needs a path")?))
            }
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    if !words_path.exists() {
        return Err(format!(
            "missing {} — run `pnpm run prepare-words` first: the passages are held to the \
             vocabulary it holds",
            words_path.display()
        )
        .into());
    }
    let words = WordDataset::from_gzip_bytes(&std::fs::read(&words_path)?)
        .map_err(|e| format!("{}: {e}", words_path.display()))?;

    let dictionary_dir = match dictionary {
        Some(path) => path,
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

    let mut files: Vec<PathBuf> = std::fs::read_dir(&passages_dir)
        .map_err(|e| format!("{}: {e}", passages_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no passages in {}", passages_dir.display()).into());
    }

    let mut passages: Vec<Passage> = Vec::new();
    let mut tokens = 0usize;
    let mut linked = 0usize;
    let mut unknown_kanji: Vec<String> = Vec::new();

    for file in &files {
        let fallback_key = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("passage")
            .to_string();
        let source = std::fs::read_to_string(file)
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let skeleton = parse_passage(&fallback_key, &source)
            .map_err(|e| format!("{}: {e}", file.display()))?;

        let mut lines: Vec<Vec<PassageToken>> = Vec::new();
        for line in &skeleton.lines {
            // `parse_passage` hands each line back as one unsegmented token; the
            // analyser is what turns it into words.
            let text = line
                .first()
                .map(|token| token.surface.clone())
                .unwrap_or_default();
            let analysed = analyse(&segmenter, &text, &words)
                .map_err(|e| format!("{}: {e}", file.display()))?;

            let mut out_line: Vec<PassageToken> = Vec::new();
            for token in analysed {
                let has_kanji = token.has_kanji();
                if has_kanji && token.word.is_none() {
                    unknown_kanji.push(format!("{} (in {})", token.surface, skeleton.title));
                }
                if has_kanji && token.rt.is_none() {
                    return Err(format!(
                        "{}: {:?} has a kanji but the analyser gave no reading for it",
                        file.display(),
                        token.surface
                    )
                    .into());
                }
                tokens += 1;
                if token.word.is_some() {
                    linked += 1;
                }
                out_line.push(PassageToken {
                    surface: token.surface,
                    rt: token.rt,
                    word: token.word,
                });
            }
            if !out_line.is_empty() {
                lines.push(out_line);
            }
        }

        passages.push(Passage {
            key: skeleton.key,
            title: skeleton.title,
            gloss: skeleton.gloss,
            lines,
        });
    }

    if !unknown_kanji.is_empty() {
        return Err(format!(
            "{} kanji-bearing tokens are not in the vocabulary, so a learner could neither read \
             them nor tap them:\n  {}\nEither the passage should use words the course teaches, or \
             the vocabulary is missing a word that belongs in it.",
            unknown_kanji.len(),
            unknown_kanji.join("\n  ")
        )
        .into());
    }

    let source = PassagesSource {
        analyser: format!("lindera {LINDERA_VERSION}"),
        dictionary: UNIDIC_VERSION.to_string(),
    };
    let dataset = PassageDataset::from_passages(passages, source.clone());
    let payload = postcard::to_allocvec(&PassagesArtifact::new(dataset.passages().to_vec(), source))?;
    let mut raw = Vec::with_capacity(payload.len() + PASSAGES_ARTIFACT_MAGIC.len());
    raw.extend_from_slice(PASSAGES_ARTIFACT_MAGIC);
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

    let lines: usize = dataset.passages().iter().map(|p| p.lines.len()).sum();
    let mut per_passage: Vec<String> = dataset
        .passages()
        .iter()
        .map(|p| format!("{}:{}", p.key, p.lines.len()))
        .collect();
    per_passage.sort();
    let artifact_bytes = std::fs::metadata(&out_path)?.len();

    println!("prepare-passages: wrote {}", out_path.display());
    println!(
        "  analyser               {} with {}",
        dataset.source().analyser,
        dataset.source().dictionary
    );
    println!("  passages               {}", dataset.len());
    println!("  lines                  {lines}");
    println!("  tokens                 {tokens} ({linked} linked to a vocabulary word)");
    println!("  vocabulary             every kanji-bearing token is a word the course teaches");
    println!(
        "  artifact               {:.0} KB compressed from {:.0} KB",
        artifact_bytes as f64 / 1e3,
        raw.len() as f64 / 1e3
    );
    println!("  keys                   {}", per_passage.join(" "));
    Ok(())
}

/// Find the built dictionary under `<root>/.lindera/`.
///
/// The dictionary sits at `.lindera/<crate version>-fmt<format version>/lindera-unidic`,
/// where the last component is `lindera-unidic`'s own `FetchParams::output_dir` and
/// the first is its cache key. This looks for it rather than hard-coding the key,
/// which would rot at the next upgrade — the same way `scripts/fetch-unidic.sh`
/// finds it — and checks for `metadata.json` so a half-written directory left by
/// an interrupted build is not mistaken for one.
fn find_dictionary(root: &Path) -> Option<PathBuf> {
    let cache = root.join(".lindera");
    for entry in std::fs::read_dir(cache).ok()? {
        let candidate = entry.ok()?.path().join("lindera-unidic");
        if candidate.join("metadata.json").is_file() {
            return Some(candidate);
        }
    }
    None
}
