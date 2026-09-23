//! Build the compact dataset artifact that ships with the app.
//!
//! Fuses the upstream datasets into one `postcard` payload:
//!
//! * `graphics.txt` (Make Me a Hanzi) — stroke outlines and stroke-order
//!   centre-lines, converted from font space into display space here so that
//!   the app never has to think about the flipped y axis when grading.
//! * `hanziDB.csv` — frequency rank, pinyin, meaning, radical, HSK level.
//! * `dictionary.txt` (Make Me a Hanzi) — etymology hints, for mnemonics.
//! * `hsk-words.json` (complete-hsk-vocabulary) — the word list: text, reading
//!   and definition for every multi-character word in the HSK 3.0 lists.
//!
//! ```text
//! cargo run -p hanzi-core --features prepare --bin prepare-data -- \
//!     [--raw <dir>] [--out <file>]
//! ```

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use hanzi_core::dataset::{Artifact, Character, Word, ARTIFACT_MAGIC};
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
    /// The IDS string that says what the character is built from, e.g. `⿰讠兑`.
    #[serde(default)]
    decomposition: String,
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

/// One entry of `hsk-words.json`. The file is minified, so the field names are
/// the abbreviations its own README documents.
#[derive(Deserialize)]
struct RawWord {
    /// Simplified headword.
    s: String,
    /// Levels the word appears in, e.g. `["n3", "o2"]`.
    #[serde(default, rename = "l")]
    levels: Vec<String>,
    #[serde(default, rename = "f")]
    forms: Vec<RawWordForm>,
}

/// One reading/traditional-form variant of a word.
#[derive(Deserialize)]
struct RawWordForm {
    #[serde(default, rename = "i")]
    transcriptions: RawTranscriptions,
    #[serde(default, rename = "m")]
    meanings: Vec<String>,
}

#[derive(Deserialize, Default)]
struct RawTranscriptions {
    /// Hanyu Pinyin with tone marks, e.g. `"ài hào"`.
    #[serde(default, rename = "y")]
    pinyin: String,
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
    decomposition: String,
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
    let words_path = raw_dir.join("hsk-words.json");

    for path in [&graphics_path, &dictionary_path, &frequency_path, &words_path] {
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
        // `？` is Make Me a Hanzi's "cannot say", not a decomposition. Storing it
        // would make every screen test for it; storing nothing says the same
        // thing once, here.
        entry.decomposition = match row.decomposition.trim() {
            "" | "？" => String::new(),
            found => found.to_string(),
        };
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
            decomposition: lex.decomposition,
            outlines: row.strokes,
            medians,
        });
    }

    // 4. Words, from the HSK lists. Every character of a word has to be
    //    drawable, or the word could be listed but never practised.
    let drawable: HashSet<char> = characters.iter().map(|c| c.ch).collect();
    let words = build_words(&words_path, &lexical, &drawable)?;

    // 5. Sort by frequency rank, ranked characters first, then by codepoint so
    //    the artifact is byte-for-byte reproducible.
    characters.sort_by_key(|c| {
        let rank = if c.rank == 0 { u32::MAX } else { c.rank };
        (rank, c.ch as u32)
    });

    // 6. Summarise before the collections are moved into the payload.
    let character_count = characters.len();
    let ranked = characters.iter().filter(|c| c.rank > 0).count();
    let with_etymology = characters.iter().filter(|c| !c.etymology.is_empty()).count();
    let with_decomposition = characters
        .iter()
        .filter(|c| !c.decomposition.is_empty())
        .count();
    let with_pinyin = characters.iter().filter(|c| !c.pinyin.is_empty()).count();
    let avg_strokes = characters.iter().map(|c| c.medians.len()).sum::<usize>() as f64
        / characters.len().max(1) as f64;
    let word_count = words.len();
    let words_with_meaning = words.iter().filter(|w| !w.meaning.is_empty()).count();
    let avg_word_chars =
        words.iter().map(|w| w.text.chars().count()).sum::<usize>() as f64 / words.len().max(1) as f64;
    let word_levels: Vec<String> = level_census(&words)
        .into_iter()
        .map(|(level, count)| format!("HSK {level}: {count}"))
        .collect();

    // 7. Serialise and compress.
    let payload = postcard::to_allocvec(&Artifact::new(characters, words))?;
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

    println!("prepare-data: wrote {}", out_path.display());
    println!("  frequency rows        {freq_rows}");
    println!("  dictionary rows       {dict_lines}");
    println!("  graphics rows         {graphics_lines} ({skipped} skipped)");
    println!("  characters            {character_count}");
    println!("  with pinyin           {with_pinyin}");
    println!("  with etymology        {with_etymology}");
    println!("  with decomposition    {with_decomposition}");
    println!("  ranked / teachable    {ranked}");
    println!("  average strokes       {avg_strokes:.1}");
    println!("  words                 {word_count} ({words_with_meaning} with a definition)");
    println!("  average word length   {avg_word_chars:.1} characters");
    println!("  words per level       {}", word_levels.join(", "));
    println!(
        "  artifact              {:.2} MB compressed from {:.2} MB ({:.0}%)",
        artifact_bytes as f64 / 1e6,
        raw.len() as f64 / 1e6,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    Ok(())
}

/// Build the word list from `hsk-words.json`.
///
/// Words the app cannot teach are dropped rather than listed: a word is only
/// kept when every one of its characters is drawable and has a frequency rank,
/// so the word browser can never offer something the board cannot ask for.
fn build_words(
    path: &Path,
    lexical: &HashMap<char, Lexical>,
    drawable: &HashSet<char>,
) -> Result<Vec<Word>, Box<dyn std::error::Error>> {
    let file = BufReader::new(File::open(path)?);
    let raw: Vec<RawWord> = serde_json::from_reader(file)
        .map_err(|e| format!("{}: {e}", path.display()))?;

    let mut words: Vec<Word> = Vec::new();
    let mut kept = HashSet::new();
    let mut single_character = 0usize;
    let mut unteachable = 0usize;
    let mut not_hsk3 = 0usize;

    for entry in &raw {
        let text = entry.s.trim();
        let characters: Vec<char> = text.chars().collect();

        // Single characters are the course's job; see `Word`'s documentation.
        if characters.len() < 2 {
            single_character += 1;
            continue;
        }

        let Some(hsk) = hsk3_level(&entry.levels) else {
            not_hsk3 += 1;
            continue;
        };

        // The word's derived frequency is its rarest character's rank, so it is
        // only defined when every character has one.
        let mut rank = 0u32;
        let mut usable = true;
        for ch in &characters {
            match (drawable.contains(ch), lexical.get(ch).map(|l| l.rank)) {
                (true, Some(rank_of)) if rank_of > 0 => rank = rank.max(rank_of),
                _ => {
                    usable = false;
                    break;
                }
            }
        }
        if !usable {
            unteachable += 1;
            continue;
        }

        let Some(form) = choose_form(&entry.forms) else {
            continue;
        };
        let pinyin: String = form.transcriptions.pinyin.split_whitespace().collect();
        if pinyin.is_empty() {
            continue;
        }
        let meaning = form
            .meanings
            .iter()
            .map(|sense| sense.trim())
            .filter(|sense| !sense.is_empty())
            .collect::<Vec<_>>()
            .join("; ");

        // The file is meant to hold each word once; if it ever does not, the
        // first mention wins so the artifact stays deterministic.
        if !kept.insert(text.to_string()) {
            continue;
        }

        words.push(Word {
            text: text.to_string(),
            pinyin,
            meaning,
            hsk,
            rank,
        });
    }

    println!(
        "prepare-data: words from {}: {} kept, {} single characters, {} not HSK 3.0, \
         {} with an unteachable character",
        path.display(),
        words.len(),
        single_character,
        not_hsk3,
        unteachable,
    );
    Ok(words)
}

/// The HSK 3.0 level of an entry, which is the lowest `n1`..`n7` it appears in.
///
/// The same file also carries the older HSK 2.0 (`o1`..`o6`) and a third set of
/// codes; only the `n` levels are the current lists the app teaches.
fn hsk3_level(levels: &[String]) -> Option<u8> {
    levels
        .iter()
        .filter_map(|level| level.strip_prefix('n'))
        .filter_map(|number| number.parse::<u8>().ok())
        .filter(|level| (1..=7).contains(level))
        .min()
}

/// Choose which of a word's several dictionary forms to teach.
///
/// CC-CEDICT gives a surname or a place name its own form, capitalised, and it
/// is frequently first: 安 is `Ān` "surname An" before `ān` "peaceful", and 都
/// is `Dū` before `dōu`. A learner wants the ordinary word, so a capitalised
/// reading is passed over whenever an ordinary one exists. Whatever is left
/// keeps the dictionary's own order, which is right for the large majority.
fn choose_form(forms: &[RawWordForm]) -> Option<&RawWordForm> {
    let first = forms.first()?;
    if is_proper_noun(&first.transcriptions.pinyin) {
        if let Some(ordinary) = forms
            .iter()
            .find(|f| !is_proper_noun(&f.transcriptions.pinyin) && !f.meanings.is_empty())
        {
            return Some(ordinary);
        }
    }
    Some(first)
}

/// True when a reading is a proper noun: CC-CEDICT capitalises its first letter.
fn is_proper_noun(pinyin: &str) -> bool {
    pinyin
        .trim_start()
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
}

/// How many words sit at each level, lowest first.
fn level_census(words: &[Word]) -> Vec<(u8, usize)> {
    let mut census: Vec<(u8, usize)> = Vec::new();
    let mut sorted: Vec<u8> = words.iter().map(|w| w.hsk).collect();
    sorted.sort_unstable();
    for level in sorted {
        match census.last_mut() {
            Some((last, count)) if *last == level => *count += 1,
            _ => census.push((level, 1)),
        }
    }
    census
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

    // ---- the word list ----------------------------------------------------

    fn form(pinyin: &str, meanings: &[&str]) -> RawWordForm {
        RawWordForm {
            transcriptions: RawTranscriptions {
                pinyin: pinyin.to_string(),
            },
            meanings: meanings.iter().map(|m| m.to_string()).collect(),
        }
    }

    fn levels(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn only_the_current_hsk_levels_count() {
        assert_eq!(hsk3_level(&levels(&["n2"])), Some(2));
        // An entry in several lists takes the lowest current level.
        assert_eq!(hsk3_level(&levels(&["n4", "n2", "o3"])), Some(2));
        // The older 2.0 lists and the mixed codes are not the course.
        assert_eq!(hsk3_level(&levels(&["o1", "t3"])), None);
        assert_eq!(hsk3_level(&[]), None);
        // A level outside 1..=7 is ignored rather than trusted.
        assert_eq!(hsk3_level(&levels(&["n9"])), None);
    }

    #[test]
    fn a_proper_noun_reading_is_passed_over_for_the_ordinary_word() {
        // 安 as the dictionary lists it: the surname first.
        let forms = vec![
            form("Ān", &["surname An"]),
            form("ān", &["calm; peaceful", "safe"]),
        ];
        let chosen = choose_form(&forms).unwrap();
        assert_eq!(chosen.transcriptions.pinyin, "ān");
        assert!(is_proper_noun("Ān"));
        assert!(!is_proper_noun("ān"));
    }

    #[test]
    fn a_word_with_no_ordinary_reading_keeps_the_dictionary_order() {
        let forms = vec![form("Běi jīng", &["Beijing"])];
        assert_eq!(
            choose_form(&forms).unwrap().transcriptions.pinyin,
            "Běi jīng"
        );
    }

    #[test]
    fn an_ordinary_first_reading_is_left_alone() {
        let forms = vec![
            form("hǎo", &["good"]),
            form("hào", &["to be fond of"]),
        ];
        assert_eq!(choose_form(&forms).unwrap().transcriptions.pinyin, "hǎo");
    }

    #[test]
    fn an_empty_form_list_is_not_a_word() {
        assert!(choose_form(&[]).is_none());
    }

    #[test]
    fn the_level_census_counts_every_level_once() {
        let word = |hsk: u8| Word {
            text: "x".into(),
            pinyin: "x".into(),
            meaning: String::new(),
            hsk,
            rank: 1,
        };
        let words = vec![word(1), word(3), word(1), word(3), word(3)];
        assert_eq!(level_census(&words), vec![(1, 2), (3, 3)]);
        assert!(level_census(&[]).is_empty());
    }
}
