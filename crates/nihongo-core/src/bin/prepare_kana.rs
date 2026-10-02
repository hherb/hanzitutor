//! Turn the upstream AnimCJK kana material into the compact artifact the app
//! embeds.
//!
//! Three inputs, all fetched by `scripts/fetch-data.sh`:
//!
//! * `graphicsJaKana.txt` — one JSON object per kana, with the SVG outline of
//!   every drawing segment and a centre-line for it (LGPL-3.0-or-later);
//! * `svgsJaKana/<code point>.svg` — the same characters as SVG. Only the element
//!   **ids** are read, to recover which drawing segments make up one taught
//!   stroke; the graphics file has already lost that grouping (see
//!   `nihongo_core::kana`). LGPL-3.0-or-later;
//! * `kvgJa/<code point>.svg` — KanjiVG, which is *not* bundled and *not* an
//!   input to the artifact. It is read only to check the answer: its per-stroke
//!   paths are an independent statement of how many strokes each kana is written
//!   with, so the grouped count is verified against a second project rather than
//!   against this code's own assumptions. CC BY-SA 3.0.
//!
//! Run: `pnpm run prepare-kana` (add `--features nihongo-core/prepare` to a
//! plain `cargo run`).

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use nihongo_core::kana::{merge_strokes, segment_to_stroke};
use nihongo_core::{Artifact, Kana, KanaDataset, Script, ARTIFACT_MAGIC};
use serde::Deserialize;

/// One line of `graphicsJaKana.txt`, in Make Me a Hanzi's format.
#[derive(Deserialize)]
struct GraphicsLine {
    character: String,
    strokes: Vec<String>,
    medians: Vec<Vec<[f32; 2]>>,
}

/// How many strokes KanjiVG says a character has.
///
/// Its stroke elements are `id="kvg:<code point>-s<n>"`, one per stroke.
fn kvg_stroke_count(svg: &str) -> usize {
    let mut seen = BTreeSet::new();
    let mut rest = svg;

    while let Some(pos) = rest.find("id=\"kvg:") {
        let after = &rest[pos + "id=\"kvg:".len()..];
        let hex: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
        let tail = &after[hex.len()..];
        if let Some(numbered) = tail.strip_prefix("-s") {
            let digits: String = numbered.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(n) = digits.parse::<u32>() {
                seen.insert(n);
            }
        }
        rest = &after[hex.len().max(1)..];
    }

    seen.len()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-kana: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/crates/nihongo-core")
        .to_path_buf();

    let mut raw_dir = repo_root.join("data/raw");
    let mut out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/kana.bin.gz");
    // The cross-check is a second project's answer to "how many strokes is
    // this?", so an artifact built without it is an artifact built on this
    // code's own assumptions. That has to be asked for deliberately.
    let mut allow_unchecked = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw_dir = PathBuf::from(args.next().ok_or("--raw needs a directory")?),
            "--out" => out_path = PathBuf::from(args.next().ok_or("--out needs a file")?),
            "--allow-unchecked" => allow_unchecked = true,
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    let graphics_path = raw_dir.join("graphicsJaKana.txt");
    if !graphics_path.exists() {
        return Err(format!(
            "missing {} — run scripts/fetch-data.sh first",
            graphics_path.display()
        )
        .into());
    }
    let svg_dir = raw_dir.join("svgsJaKana");
    let kvg_dir = raw_dir.join("kvgJa");

    let mut kana: Vec<Kana> = Vec::new();
    let mut rows = 0usize;
    let mut checked_against_kvg = 0usize;
    let mut split_characters = 0usize;
    let mut segments_folded = 0usize;
    let mut missing_svgs: Vec<String> = Vec::new();

    for (line_no, line) in BufReader::new(File::open(&graphics_path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: GraphicsLine = serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", graphics_path.display(), line_no + 1))?;
        rows += 1;

        let mut chars = row.character.chars();
        let (Some(ch), None) = (chars.next(), chars.next()) else {
            return Err(format!("{}: expected one character, got {:?}", graphics_path.display(), row.character).into());
        };
        let cp = ch as u32;
        let script = Script::of(ch)
            .ok_or_else(|| format!("{ch:?} (U+{cp:04X}) is not a kana this app teaches"))?;

        // The grouping only exists in the SVG, so the SVG is required.
        let svg_path = svg_dir.join(format!("{cp}.svg"));
        if !svg_path.exists() {
            missing_svgs.push(format!("{} (U+{cp:04X})", svg_path.display()));
            continue;
        }
        let svg = std::fs::read_to_string(&svg_path)?;
        let grouping = segment_to_stroke(&svg, cp);

        if grouping.len() != row.strokes.len() {
            return Err(format!(
                "{ch} (U+{cp:04X}): the SVG names {} drawing segments but the graphics file has {} strokes — \
                 the two disagree and merging would be guesswork",
                grouping.len(),
                row.strokes.len()
            )
            .into());
        }

        let taught = grouping.last().map_or(0, |&s| s as usize + 1);
        if taught < row.strokes.len() {
            split_characters += 1;
            segments_folded += row.strokes.len() - taught;
        }

        let (outlines, medians) = merge_strokes(&row.strokes, &row.medians, &grouping)
            .map_err(|e| format!("{ch} (U+{cp:04X}): {e}"))?;

        // Cross-check against KanjiVG where it is available. This is the check
        // that makes the merge trustworthy: it is a different project's answer.
        let kvg_path = kvg_dir.join(format!("{cp:05x}.svg"));
        if kvg_path.exists() {
            let counted = kvg_stroke_count(&std::fs::read_to_string(&kvg_path)?);
            if counted > 0 {
                if counted != outlines.len() {
                    return Err(format!(
                        "{ch} (U+{cp:04X}): merged to {} strokes but KanjiVG says {counted}",
                        outlines.len()
                    )
                    .into());
                }
                checked_against_kvg += 1;
            }
        }

        kana.push(Kana {
            ch,
            script,
            stroke_count: (outlines.len()).min(u8::MAX as usize) as u8,
            outlines,
            medians,
        });
    }

    if !missing_svgs.is_empty() {
        return Err(format!(
            "{} kana have no SVG, so their stroke grouping cannot be recovered:\n  {}",
            missing_svgs.len(),
            missing_svgs.join("\n  ")
        )
        .into());
    }

    if !allow_unchecked && checked_against_kvg != rows {
        return Err(format!(
            "only {checked_against_kvg} of {rows} kana could be checked against KanjiVG, so the \
             merged stroke counts rest on this code's own assumptions. Run scripts/fetch-data.sh \
             to fetch kvgJa/, or pass --allow-unchecked to accept that."
        )
        .into());
    }

    let dataset = KanaDataset::from_kana_teachable(kana);
    let hiragana = dataset.of_script(Script::Hiragana).count();
    let katakana = dataset.of_script(Script::Katakana).count();
    let graded = dataset.len();
    let total_strokes: usize = dataset.kana().iter().map(|k| k.stroke_count as usize).sum();
    let avg_strokes = total_strokes as f64 / graded.max(1) as f64;

    // Sort by code point before writing, so the artifact is byte-for-byte
    // reproducible.
    let mut payload_kana: Vec<Kana> = dataset.kana().to_vec();
    payload_kana.sort_by_key(|k| k.ch as u32);

    let payload = postcard::to_allocvec(&Artifact::new(payload_kana))?;
    let mut raw = Vec::with_capacity(payload.len() + ARTIFACT_MAGIC.len());
    raw.extend_from_slice(ARTIFACT_MAGIC);
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

    println!("prepare-kana: wrote {}", out_path.display());
    println!("  graphics rows          {rows}");
    println!("  kana                   {graded} ({hiragana} hiragana, {katakana} katakana)");
    println!("  total strokes          {total_strokes} (average {avg_strokes:.2})");
    println!("  split characters       {split_characters} ({segments_folded} segments folded back)");
    println!("  checked against KanjiVG {checked_against_kvg}");
    println!(
        "  artifact               {:.0} KB compressed from {:.0} KB ({:.0}%)",
        artifact_bytes as f64 / 1e3,
        raw.len() as f64 / 1e3,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    Ok(())
}
