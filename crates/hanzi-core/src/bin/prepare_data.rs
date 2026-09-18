//! Build the compact character artifact that ships with the app.
//!
//! Fuses two upstream datasets into one `postcard` payload:
//!
//! * `graphics.txt` (Make Me a Hanzi) — stroke outlines and stroke-order
//!   centre-lines, converted from font space into display space here so that
//!   the app never has to think about the flipped y axis when grading.
//! * `hanziDB.csv` — frequency rank, pinyin, meaning, radical, HSK level.
//! * `dictionary.txt` (Make Me a Hanzi) — etymology hints, for mnemonics.
//!
//! ```text
//! cargo run -p hanzi-core --features prepare --bin prepare-data -- \
//!     [--raw <dir>] [--out <file>]
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hanzi_core::dataset::{Character, ARTIFACT_MAGIC};
use hanzi_core::geom::Point;
use serde::Deserialize;

/// Geometry: one JSON object per line.
#[derive(Deserialize)]
struct GraphicsLine {
    character: String,
    strokes: Vec<String>,
    medians: Vec<Vec<[f32; 2]>>,
}

#[derive(Deserialize)]
struct Etymology {
    #[serde(default)]
    hint: String,
}

/// Lexical detail: one JSON object per line.
#[derive(Deserialize)]
struct DictionaryLine {
    character: String,
    #[serde(default)]
    definition: String,
    #[serde(default)]
    pinyin: Vec<String>,
    #[serde(default)]
    radical: String,
    #[serde(default)]
    etymology: Option<Etymology>,
}

/// Frequency list row. The upstream header misspells the character column as
/// `charcter`, so the field name matches the file rather than the language.
#[derive(Deserialize)]
struct FrequencyRow {
    frequency_rank: u32,
    charcter: String,
    #[serde(default)]
    pinyin: String,
    #[serde(default)]
    definition: String,
    #[serde(default)]
    radical: String,
    #[serde(default)]
    stroke_count: String,
    #[serde(default)]
    hsk_level: String,
}

/// Lexical data merged from both lexical sources, keyed by character.
#[derive(Default, Clone)]
struct Lexical {
    rank: u32,
    hsk: u8,
    stroke_count: u8,
    radical: char,
    pinyin: Vec<String>,
    definition: String,
    etymology: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-data: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/crates/hanzi-core")
        .to_path_buf();

    let mut raw_dir = repo_root.join("data/raw");
    let mut out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/hanzi.bin.gz");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw_dir = PathBuf::from(args.next().ok_or("--raw needs a directory")?),
            "--out" => out_path = PathBuf::from(args.next().ok_or("--out needs a file")?),
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    let graphics_path = raw_dir.join("graphics.txt");
    let dictionary_path = raw_dir.join("dictionary.txt");
    let frequency_path = raw_dir.join("hanziDB.csv");

    for path in [&graphics_path, &dictionary_path, &frequency_path] {
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-data.sh first",
                path.display()
            )
            .into());
        }
    }

    // 1. Lexical layer, from the lower-precedence source first.
    let mut lexical: HashMap<char, Lexical> = HashMap::new();
    let mut dict_lines = 0usize;
    for (line_no, line) in BufReader::new(File::open(&dictionary_path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: DictionaryLine = serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", dictionary_path.display(), line_no + 1))?;
        let Some(ch) = single_char(&row.character) else {
            continue;
        };
        let entry = lexical.entry(ch).or_default();
        entry.pinyin = row.pinyin.clone();
        entry.definition = row.definition.clone();
        if let Some(etymology) = row.etymology {
            entry.etymology = etymology.hint.trim().to_string();
        }
        entry.radical = single_char(&row.radical).unwrap_or('\0');
        dict_lines += 1;
    }

    // 2. The frequency list takes precedence for rank, HSK level, meaning and
    //    stroke count, since it is the list the curriculum is built from.
    let mut csv_reader = csv::Reader::from_path(&frequency_path)?;
    let mut freq_rows = 0usize;
    for record in csv_reader.deserialize::<FrequencyRow>() {
        let row: FrequencyRow = record?;
        let Some(ch) = single_char(&row.charcter) else {
            continue;
        };
        let entry = lexical.entry(ch).or_default();
        entry.rank = row.frequency_rank;
        entry.hsk = parse_leading_u8(&row.hsk_level);
        entry.stroke_count = parse_leading_u8(&row.stroke_count);
        if let Some(radical) = single_char(&row.radical) {
            entry.radical = radical;
        }
        if !row.definition.trim().is_empty() {
            entry.definition = row.definition.trim().to_string();
        }
        if entry.pinyin.is_empty() {
            entry.pinyin = split_readings(&row.pinyin);
        }
        freq_rows += 1;
    }

    // 3. Geometry, streaming so the 30 MB file never lands in memory whole.
    let mut characters: Vec<Character> = Vec::new();
    let mut skipped = 0usize;
    let mut graphics_lines = 0usize;
    for (line_no, line) in BufReader::new(File::open(&graphics_path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: GraphicsLine = serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", graphics_path.display(), line_no + 1))?;
        graphics_lines += 1;

        let Some(ch) = single_char(&row.character) else {
            skipped += 1;
            continue;
        };
        // Without a centre-line per stroke there is nothing to grade against.
        if row.medians.len() != row.strokes.len() || row.strokes.is_empty() {
            skipped += 1;
            continue;
        }

        let medians: Vec<Vec<Point>> = row
            .medians
            .iter()
            .map(|stroke| {
                stroke
                    .iter()
                    .map(|[x, y]| Point::from_font(*x, *y))
                    .collect()
            })
            .collect();

        let lex = lexical.get(&ch).cloned().unwrap_or_default();
        let stroke_count = if lex.stroke_count > 0 {
            lex.stroke_count
        } else {
            row.strokes.len() as u8
        };

        characters.push(Character {
            ch,
            rank: lex.rank,
            hsk: lex.hsk,
            stroke_count,
            radical: lex.radical,
            pinyin: lex.pinyin,
            definition: lex.definition,
            etymology: lex.etymology,
            outlines: row.strokes,
            medians,
        });
    }

    // 4. Sort by frequency rank, ranked characters first, then by codepoint so
    //    the artifact is byte-for-byte reproducible.
    characters.sort_by_key(|c| {
        let rank = if c.rank == 0 { u32::MAX } else { c.rank };
        (rank, c.ch as u32)
    });

    // 5. Serialise and compress.
    let payload = postcard::to_allocvec(&characters)?;
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
    let ranked = characters.iter().filter(|c| c.rank > 0).count();
    let with_etymology = characters.iter().filter(|c| !c.etymology.is_empty()).count();
    let with_pinyin = characters.iter().filter(|c| !c.pinyin.is_empty()).count();
    let avg_strokes =
        characters.iter().map(|c| c.medians.len()).sum::<usize>() as f64 / characters.len().max(1) as f64;

    println!("prepare-data: wrote {}", out_path.display());
    println!("  frequency rows        {freq_rows}");
    println!("  dictionary rows       {dict_lines}");
    println!("  graphics rows         {graphics_lines} ({skipped} skipped)");
    println!("  characters            {}", characters.len());
    println!("  ranked / teachable    {ranked}");
    println!("  with pinyin           {with_pinyin}");
    println!("  with etymology        {with_etymology}");
    println!("  average strokes       {avg_strokes:.1}");
    println!(
        "  artifact              {:.2} MB compressed from {:.2} MB ({:.0}%)",
        artifact_bytes as f64 / 1e6,
        raw.len() as f64 / 1e6,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    Ok(())
}

/// First `char` of a string, ignoring anything after it.
fn single_char(value: &str) -> Option<char> {
    value.trim().chars().next()
}

/// Parse the leading integer of a possibly-empty or decorated field.
fn parse_leading_u8(value: &str) -> u8 {
    let digits: String = value
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

/// Split a compact pronunciation string such as `"hǎo, hào"` into readings.
fn split_readings(value: &str) -> Vec<String> {
    value
        .split([',', ';', '/'])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_leading_numbers() {
        assert_eq!(parse_leading_u8("8"), 8);
        assert_eq!(parse_leading_u8(""), 0);
        assert_eq!(parse_leading_u8("7-9"), 7);
        assert_eq!(parse_leading_u8("  12  "), 12);
        assert_eq!(parse_leading_u8("n/a"), 0);
    }

    #[test]
    fn splits_compact_readings() {
        assert_eq!(split_readings("hǎo"), vec!["hǎo"]);
        assert_eq!(split_readings("hǎo, hào"), vec!["hǎo", "hào"]);
        assert_eq!(split_readings(" de ; dí "), vec!["de", "dí"]);
        assert!(split_readings("").is_empty());
    }

    #[test]
    fn single_char_ignores_trailing_text() {
        assert_eq!(single_char("好"), Some('好'));
        assert_eq!(single_char(" 一 "), Some('一'));
        assert_eq!(single_char(""), None);
    }
}
