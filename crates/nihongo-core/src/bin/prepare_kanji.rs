//! Turn the upstream Japanese kanji material into the compact artifact the app
//! embeds.
//!
//! Four inputs, all fetched by `scripts/fetch-data.sh`, and which one is the
//! authority for what is the part worth reading:
//!
//! * `graphicsJa.txt` — AnimCJK's geometry: the SVG outline of every stroke and
//!   its centre-line, for 7,007 characters, in **Make Me a Hanzi's own font
//!   space** (Arphic Public License). The taught stroke count is this file's
//!   count, and it needs none of the repair the kana file does — AnimCJK splits
//!   a kana stroke that crosses itself, and does not do that to a kanji (see
//!   `nihongo_core::kanji`).
//! * `dictionaryJa.txt` — AnimCJK's dictionary (LGPL-3.0-or-later): the radical
//!   the character is written with and its note, and the IDS decomposition. Its
//!   `on`, `kun` and `definition` fields are deliberately **not** read: readings
//!   and glosses come from EDRDG, which is the attributed source for them, and a
//!   second-hand copy of them would misrecord where the app's text comes from.
//!   The same file's **214 entries whose `set` contains `radical`** are the
//!   radical table, in Kangxi number order, and they are where the head forms on
//!   the radicals screen come from. The number is read from the *position* and
//!   checked against KANJIDIC2's classical number, never parsed out of the
//!   entry's gloss — three of the 214 glosses carry the wrong number, and one of
//!   those fields is not read at all.
//! * `kanjidic2-all.json` — EDRDG's KANJIDIC2 (CC BY-SA 4.0), republished as one
//!   JSON document by `scriptin/jmdict-simplified`. The authority for the
//!   **current** kyōiku grade, the readings with their okurigana, the frequency
//!   rank and the English glosses. Its own `version` and `dictDate` are copied
//!   into the artifact, so what shipped can be checked against the update
//!   obligation LICENSES.md records.
//! * `kvgJa/<code point>.svg` — KanjiVG (CC BY-SA 3.0). **Not** bundled and
//!   **not** an input: the independent second opinion the stroke counts are
//!   checked against, exactly as for the kana. One character, 衷, is a written
//!   exception — see [`KANJIVG_DISAGREES`].
//!
//! Run: `pnpm run prepare-kanji` (add `--features nihongo-core/prepare` to a
//! plain `cargo run`).

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use nihongo_core::kanji::{
    parse_radical, Kanji, KanjiArtifact, KanjiDataset, KanjiSource, Radical, JOYO_COUNT,
    JOYO_GRADES, KANJI_ARTIFACT_MAGIC, RADICAL_COUNT,
};
use nihongo_core::Point;
use serde::Deserialize;

/// The characters whose stroke count KanjiVG does not agree with.
///
/// Each entry is `(character, taught, KanjiVG says)`. The taught count is the
/// geometry's, because that is the count the grader has reference strokes for,
/// and the entry records **both** numbers so the check is two-sided: a change to
/// the geometry, to KanjiVG, or to this table fails the build. There is one:
///
/// * 衷 — 10 strokes by the geometry and by KANJIDIC2, 9 paths in KanjiVG.
///
/// `ROADMAP_NIHONGO.md` records nine such characters (謎 賭 葛 餌 遜 僅 遡 餅 牙).
/// That list is a measurement artefact: those nine are where KANJIDIC2 lists more
/// than one stroke count and the *first* is not the taught one, so comparing
/// against whichever value happens to come first rather than against the taught
/// count invents a disagreement. Both files were re-measured for this pipeline —
/// see `nihongo_core::kanji`'s module docs.
const KANJIVG_DISAGREES: [(char, usize, usize); 1] = [('衷', 10, 9)];

/// The two radical head forms KANJIDIC2 gives no classical radical number for,
/// with the number `dictionaryJa.txt`'s own order assigns them.
///
/// This is a written exception rather than a gap, checked in both directions: a
/// radical with no KANJIDIC2 number that is *not* here fails the build, and so
/// does one of these two turning up with a number, because then the exception is
/// out of date and the check it documents has moved.
///
/// Neither is an error upstream — EDRDG records the other form of those two
/// radicals (戸 for 63, 青 for 174), and AnimCJK's file writes this one — which
/// is exactly why it belongs in a list with its reason rather than in a
/// `continue`.
const RADICALS_KANJIDIC2_CANNOT_NUMBER: [(char, u8); 2] = [('戶', 63), ('靑', 174)];

/// One line of `graphicsJa.txt`, in Make Me a Hanzi's format.
#[derive(Deserialize)]
struct GraphicsLine {
    character: String,
    strokes: Vec<String>,
    medians: Vec<Vec<[f32; 2]>>,
}

/// The parts of KANJIDIC2's document this reads. Everything else — the
/// dictionary references, the query codes, the other languages — is left alone.
#[derive(Deserialize)]
struct Kanjidic2 {
    version: String,
    #[serde(rename = "dictDate")]
    dict_date: String,
    characters: Vec<Kd2Entry>,
}

#[derive(Deserialize)]
struct Kd2Entry {
    literal: String,
    #[serde(default)]
    misc: Option<Kd2Misc>,
    #[serde(default, rename = "readingMeaning")]
    reading_meaning: Option<Kd2ReadingMeaning>,
    #[serde(default)]
    radicals: Vec<Kd2Radical>,
}

impl Kd2Entry {
    fn grade(&self) -> Option<u8> {
        self.misc.as_ref().and_then(|misc| misc.grade)
    }

    fn stroke_counts(&self) -> &[u8] {
        self.misc
            .as_ref()
            .map(|misc| misc.stroke_counts.as_slice())
            .unwrap_or_default()
    }

    /// The classical radical's number, 1–214. The other entries in the list —
    /// `nelson_c` — are a different classification and are ignored.
    fn classical_radical(&self) -> Option<u8> {
        self.radicals
            .iter()
            .find(|radical| radical.kind == "classical")
            .map(|radical| radical.value)
    }

    fn readings(&self, kind: &str) -> Vec<String> {
        self.reading_meaning
            .iter()
            .flat_map(|rm| rm.groups.iter())
            .flat_map(|group| group.readings.iter())
            .filter(|reading| reading.kind == kind)
            .map(|reading| reading.value.clone())
            .collect()
    }

    /// The English glosses, and only those: the `-all` document also carries
    /// French, Spanish and Portuguese meanings, and a screen that mixed them
    /// would be showing three languages in one list.
    fn english_meanings(&self) -> Vec<String> {
        self.reading_meaning
            .iter()
            .flat_map(|rm| rm.groups.iter())
            .flat_map(|group| group.meanings.iter())
            .filter(|meaning| meaning.lang.as_deref() == Some("en"))
            .map(|meaning| meaning.value.clone())
            .collect()
    }

    fn nanori(&self) -> Vec<String> {
        self.reading_meaning
            .iter()
            .flat_map(|rm| rm.nanori.iter())
            .cloned()
            .collect()
    }
}

#[derive(Deserialize)]
struct Kd2Misc {
    #[serde(default)]
    grade: Option<u8>,
    #[serde(default, rename = "strokeCounts")]
    stroke_counts: Vec<u8>,
    #[serde(default)]
    frequency: Option<u16>,
}

#[derive(Deserialize)]
struct Kd2ReadingMeaning {
    #[serde(default)]
    groups: Vec<Kd2Group>,
    #[serde(default)]
    nanori: Vec<String>,
}

#[derive(Deserialize)]
struct Kd2Group {
    #[serde(default)]
    readings: Vec<Kd2Reading>,
    #[serde(default)]
    meanings: Vec<Kd2Meaning>,
}

#[derive(Deserialize)]
struct Kd2Reading {
    #[serde(rename = "type")]
    kind: String,
    value: String,
}

#[derive(Deserialize)]
struct Kd2Meaning {
    value: String,
    #[serde(default)]
    lang: Option<String>,
}

#[derive(Deserialize)]
struct Kd2Radical {
    #[serde(rename = "type")]
    kind: String,
    value: u8,
}

/// One line of `dictionaryJa.txt`: only the fields this pipeline reads.
///
/// `set` is a **list**, not a string, and a character can be in two of them at
/// once — `["g1", "radical"]` is 一 — so the grade is a *member*, not the value.
#[derive(Deserialize)]
struct DictionaryLine {
    character: String,
    #[serde(default)]
    set: Vec<String>,
    #[serde(default)]
    radical: Option<String>,
    #[serde(default)]
    decomposition: Option<String>,
}

impl DictionaryLine {
    /// AnimCJK's grade for this character, as a KANJIDIC2 grade number.
    ///
    /// AnimCJK's `g1`…`g6` are the kyōiku grades and its `g7` is the jōyō
    /// remainder, which is KANJIDIC2's **grade 8** — the numbering differs, and
    /// reading the two as the same number would be wrong for every one of the
    /// 1,110 characters involved. Its `g8` and `g9` are jinmeiyō and hyōgai, and
    /// the other members (`radical`, `gc`, `stroke`, `hiragana`, `katakana`) are
    /// not grades at all, so a character whose only membership is one of those
    /// gets `None` and the caller says so.
    fn grade(&self) -> Option<u8> {
        self.set.iter().find_map(|member| match member.as_str() {
            "g1" => Some(1),
            "g2" => Some(2),
            "g3" => Some(3),
            "g4" => Some(4),
            "g5" => Some(5),
            "g6" => Some(6),
            "g7" => Some(8),
            _ => None,
        })
    }
}

/// How many strokes KanjiVG says a character has.
///
/// Its stroke elements are `id="kvg:<code point>-s<n>"`, one per stroke — the
/// same rule `prepare-kana` counts with.
fn kvg_stroke_count(svg: &str) -> usize {
    let mut seen = std::collections::BTreeSet::new();
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
            eprintln!("prepare-kanji: {err}");
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
    let mut out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/kanji.bin.gz");
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

    let kanjidic2_path = raw_dir.join("kanjidic2-all.json");
    let graphics_path = raw_dir.join("graphicsJa.txt");
    let dictionary_path = raw_dir.join("dictionaryJa.txt");
    for path in [&kanjidic2_path, &graphics_path, &dictionary_path] {
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-data.sh first",
                path.display()
            )
            .into());
        }
    }
    let kvg_dir = raw_dir.join("kvgJa");

    // ---- KANJIDIC2: the curriculum, the readings, the glosses ----------------
    let document: Kanjidic2 = serde_json::from_reader(File::open(&kanjidic2_path)?)
        .map_err(|e| format!("{}: {e}", kanjidic2_path.display()))?;
    let source = KanjiSource {
        kanjidic2_version: document.version.clone(),
        kanjidic2_date: document.dict_date.clone(),
    };

    let mut joyo: Vec<&Kd2Entry> = document
        .characters
        .iter()
        .filter(|entry| entry.grade().is_some_and(|g| JOYO_GRADES.contains(&g)))
        .collect();
    joyo.sort_by_key(|entry| entry.literal.chars().next().map(|c| c as u32));
    if joyo.len() != JOYO_COUNT {
        return Err(format!(
            "KANJIDIC2 {} holds {} characters in the jōyō grades, not {JOYO_COUNT}. The jōyō \
             list is fixed by the Japanese ministry's curriculum, so this is a decision to take \
             and record rather than a rebuild — check the snapshot, then update JOYO_COUNT and \
             the tests together.",
            document.version,
            joyo.len()
        )
        .into());
    }

    // Every KANJIDIC2 entry by its character, for the radical table's check: the
    // radicals include 92 characters that are not jōyō, so `joyo` cannot answer
    // for them.
    let mut kd2_by_char: HashMap<char, &Kd2Entry> = HashMap::new();
    for entry in &document.characters {
        if let Some(ch) = single_char(&entry.literal) {
            kd2_by_char.insert(ch, entry);
        }
    }

    // ---- dictionaryJa.txt: the radical and the decomposition -----------------
    let mut structure: HashMap<char, DictionaryLine> = HashMap::new();
    // The 214 entries tagged `radical`, in file order. Their position *is* the
    // Kangxi number — see `RADICALS_KANJIDIC2_CANNOT_NUMBER` and the check below.
    let mut radical_chars: Vec<char> = Vec::new();
    for (line_no, line) in BufReader::new(File::open(&dictionary_path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: DictionaryLine = serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", dictionary_path.display(), line_no + 1))?;
        if let Some(ch) = single_char(&row.character) {
            if row.set.iter().any(|member| member == "radical") {
                radical_chars.push(ch);
            }
            structure.insert(ch, row);
        }
    }

    // ---- graphicsJa.txt: the geometry ---------------------------------------
    let mut geometry: HashMap<char, GraphicsLine> = HashMap::new();
    let mut graphics_rows = 0usize;
    for (line_no, line) in BufReader::new(File::open(&graphics_path)?).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let row: GraphicsLine = serde_json::from_str(&line)
            .map_err(|e| format!("{}:{}: {e}", graphics_path.display(), line_no + 1))?;
        graphics_rows += 1;
        if let Some(ch) = single_char(&row.character) {
            geometry.insert(ch, row);
        }
    }

    // ---- The 214 Kangxi radicals --------------------------------------------
    //
    // The head forms, in number order, from the entries `dictionaryJa.txt` tags
    // `radical`. Two things are checked rather than assumed: that there are 214 of
    // them, and that their position agrees with KANJIDIC2's classical radical
    // number — which is what makes "position is the number" a measurement instead
    // of a convention.
    if radical_chars.len() != RADICAL_COUNT {
        return Err(format!(
            "dictionaryJa.txt tags {} entries `radical`, not the {RADICAL_COUNT} Kangxi \
             radicals. The radical table's numbering is the file order, so this is a decision to \
             take and record rather than a rebuild.",
            radical_chars.len()
        )
        .into());
    }
    let mut radicals: Vec<Radical> = Vec::with_capacity(RADICAL_COUNT);
    let mut radicals_checked = 0usize;
    let mut cannot_number: Vec<String> = Vec::new();
    let mut missing_radical_geometry: Vec<String> = Vec::new();
    for (index, ch) in radical_chars.iter().enumerate() {
        let number = (index + 1) as u8;
        let ch = *ch;
        match kd2_by_char.get(&ch).and_then(|entry| entry.classical_radical()) {
            Some(says) if says == number => {
                // The exception list is checked in *this* direction too: one of
                // the two radicals KANJIDIC2 could not number turning up with one
                // means the exception is out of date, not that the check passed.
                if let Some((_, recorded)) = RADICALS_KANJIDIC2_CANNOT_NUMBER
                    .iter()
                    .find(|(c, _)| *c == ch)
                {
                    return Err(format!(
                        "{ch} is written down as one of the radicals KANJIDIC2 cannot number \
                         ({recorded}), but it now carries classical radical {says}. Remove it from \
                         RADICALS_KANJIDIC2_CANNOT_NUMBER and record the change."
                    )
                    .into());
                }
                radicals_checked += 1;
            }
            Some(says) => {
                return Err(format!(
                    "{ch} is entry {number} of dictionaryJa.txt's radical list but KANJIDIC2 \
                     calls it classical radical {says}. The file order is the numbering the \
                     table is written with, so one of the two moved — decide which, and record it."
                )
                .into());
            }
            None => match RADICALS_KANJIDIC2_CANNOT_NUMBER.iter().find(|(c, _)| *c == ch) {
                Some((_, recorded)) if *recorded == number => {
                    cannot_number.push(format!("{ch}:{number}"));
                }
                Some((_, recorded)) => {
                    return Err(format!(
                        "{ch} is the written exception for classical radical {recorded} but sits at \
                         position {number} in dictionaryJa.txt's radical list"
                    )
                    .into());
                }
                None => {
                    return Err(format!(
                        "{ch} (radical entry {number}) has no classical radical number in \
                         KANJIDIC2, and is not one of the written exceptions. Either KANJIDIC2 \
                         now numbers it — in which case remove it from \
                         RADICALS_KANJIDIC2_CANNOT_NUMBER — or this is a new disagreement to \
                         decide about."
                    )
                    .into());
                }
            },
        }

        let Some(row) = geometry.get(&ch) else {
            missing_radical_geometry.push(format!("{ch} (radical {number})"));
            continue;
        };
        if row.strokes.is_empty() || row.strokes.len() != row.medians.len() {
            return Err(format!(
                "{ch} (radical {number}): {} outlines and {} centre-lines — they must be one per \
                 stroke, and there must be at least one",
                row.strokes.len(),
                row.medians.len()
            )
            .into());
        }
        radicals.push(Radical {
            number,
            ch,
            stroke_count: row.strokes.len().min(u8::MAX as usize) as u8,
            outlines: row.strokes.clone(),
            medians: row
                .medians
                .iter()
                .map(|stroke| {
                    stroke
                        .iter()
                        .map(|[x, y]| Point::from_font(*x, *y))
                        .collect()
                })
                .collect(),
        });
    }
    if !missing_radical_geometry.is_empty() {
        return Err(format!(
            "{} of the {RADICAL_COUNT} radicals have no geometry in graphicsJa.txt, so the board \
             could not write them:\n  {}",
            missing_radical_geometry.len(),
            missing_radical_geometry.join("\n  ")
        )
        .into());
    }
    // Every entry in the exception list has to have been *seen*, or the list is
    // carrying a radical this file no longer has and the check it documents is
    // not running.
    if cannot_number.len() != RADICALS_KANJIDIC2_CANNOT_NUMBER.len() {
        return Err(format!(
            "{} of the {} written exceptions were found in dictionaryJa.txt's radical list. An \
             exception for a radical the file does not carry is not a check, so either the file \
             changed or RADICALS_KANJIDIC2_CANNOT_NUMBER did.",
            cannot_number.len(),
            RADICALS_KANJIDIC2_CANNOT_NUMBER.len()
        )
        .into());
    }

    // ---- Assemble, checking every character against every source -------------
    let mut kanji: Vec<Kanji> = Vec::with_capacity(joyo.len());
    let mut checked_against_kvg = 0usize;
    let mut dictionary_agrees = 0usize;
    // The 2017 kyōiku additions, told apart from the reassignments: AnimCJK puts
    // them in the jōyō remainder and KANJIDIC2 in grade 4.
    let mut added_in_2017 = 0usize;
    let mut reassigned = 0usize;
    let mut missing_geometry: Vec<String> = Vec::new();
    let mut missing_kvg: Vec<String> = Vec::new();
    let mut ungradeable: Vec<String> = Vec::new();

    for entry in &joyo {
        let ch = single_char(&entry.literal).ok_or_else(|| {
            format!("KANJIDIC2 entry {:?} is not a single character", entry.literal)
        })?;
        let cp = ch as u32;
        let grade = entry.grade().expect("filtered on having a grade");

        // The structure — radical, decomposition, and AnimCJK's own opinion about
        // which grade this character is in.
        let dictionary = structure
            .get(&ch)
            .ok_or_else(|| format!("{ch} (U+{cp:04X}) is jōyō but dictionaryJa.txt does not list it"))?;
        let animcjk_grade = dictionary.grade().ok_or_else(|| {
            format!(
                "{ch} (U+{cp:04X}) is jōyō in KANJIDIC2 but dictionaryJa.txt puts it in {:?}, \
                 which is not one of the jōyō sets",
                dictionary.set
            )
        })?;
        if animcjk_grade == grade {
            dictionary_agrees += 1;
        } else if animcjk_grade == 8 && grade == 4 {
            added_in_2017 += 1;
        } else {
            reassigned += 1;
        }

        let radical_field = dictionary
            .radical
            .as_deref()
            .ok_or_else(|| format!("{ch} (U+{cp:04X}) has no radical in dictionaryJa.txt"))?;
        let (radical, radical_note) = parse_radical(radical_field).ok_or_else(|| {
            format!("{ch} (U+{cp:04X}): {radical_field:?} is not a radical and its note")
        })?;
        let radical_number = entry.classical_radical().ok_or_else(|| {
            format!("{ch} (U+{cp:04X}) has no classical radical number in KANJIDIC2")
        })?;

        // The geometry. Its own count is the taught count — see the module docs
        // and `nihongo_core::kanji` for why no merge is needed here.
        let Some(row) = geometry.get(&ch) else {
            missing_geometry.push(format!("{ch} (U+{cp:04X})"));
            continue;
        };
        if row.strokes.is_empty() || row.strokes.len() != row.medians.len() {
            return Err(format!(
                "{ch} (U+{cp:04X}): {} outlines and {} centre-lines — they must be one per stroke, \
                 and there must be at least one",
                row.strokes.len(),
                row.medians.len()
            )
            .into());
        }
        let taught = row.strokes.len();
        if !entry.stroke_counts().iter().any(|&count| count as usize == taught) {
            return Err(format!(
                "{ch} (U+{cp:04X}): the geometry has {taught} strokes, which is not one of the \
                 counts KANJIDIC2 lists ({:?})",
                entry.stroke_counts()
            )
            .into());
        }

        // The oracle. A written exception is checked *against its recorded value*
        // rather than skipped, so a change on either side is a failure.
        let kvg_path = kvg_dir.join(format!("{cp:05x}.svg"));
        if !kvg_path.exists() {
            missing_kvg.push(format!("{ch} (U+{cp:04X})"));
        } else {
            let counted = kvg_stroke_count(&std::fs::read_to_string(&kvg_path)?);
            if counted == 0 {
                return Err(format!(
                    "{ch} (U+{cp:04X}): {} has no stroke paths to count",
                    kvg_path.display()
                )
                .into());
            }
            match KANJIVG_DISAGREES.iter().find(|(kana, _, _)| *kana == ch) {
                Some((_, recorded_taught, recorded_kvg)) => {
                    if taught != *recorded_taught || counted != *recorded_kvg {
                        return Err(format!(
                            "{ch} (U+{cp:04X}) is a written KanjiVG disagreement: recorded as \
                             {recorded_taught} taught strokes against KanjiVG's {recorded_kvg}, \
                             and now the geometry says {taught} and KanjiVG says {counted} — the \
                             exception list is out of date"
                        )
                        .into());
                    }
                }
                None => {
                    if counted != taught {
                        return Err(format!(
                            "{ch} (U+{cp:04X}): the geometry has {taught} strokes but KanjiVG \
                             says {counted}. Either the geometry changed or this is a new \
                             disagreement to decide about — it must not be a silent pass"
                        )
                        .into());
                    }
                }
            }
            checked_against_kvg += 1;
        }

        kanji.push(Kanji {
            ch,
            grade: entry.grade().expect("filtered on having a grade"),
            stroke_count: (taught).min(u8::MAX as usize) as u8,
            frequency: entry.misc.as_ref().and_then(|misc| misc.frequency),
            radical,
            radical_note,
            radical_number,
            on: entry.readings("ja_on"),
            kun: entry.readings("ja_kun"),
            meanings: entry.english_meanings(),
            nanori: entry.nanori(),
            decomposition: dictionary.decomposition.clone().unwrap_or_default(),
            outlines: row.strokes.clone(),
            // Font space in the file, display space in the artifact — the same
            // conversion the kana pipeline applies.
            medians: row
                .medians
                .iter()
                .map(|stroke| {
                    stroke
                        .iter()
                        .map(|[x, y]| Point::from_font(*x, *y))
                        .collect()
                })
                .collect(),
        });
    }

    if !missing_geometry.is_empty() {
        return Err(format!(
            "{} jōyō characters have no geometry in graphicsJa.txt:\n  {}",
            missing_geometry.len(),
            missing_geometry.join("\n  ")
        )
        .into());
    }
    // A missing oracle is what `--allow-unchecked` exists to accept: the counts
    // then rest on this code's own reading of the geometry, and the artifact will
    // say so on the command line that built it.
    if !missing_kvg.is_empty() && !allow_unchecked {
        return Err(format!(
            "{} jōyō characters have no KanjiVG SVG, so their stroke count could not be checked \
             independently:\n  {}\nRun scripts/fetch-data.sh to fetch kvgJa/, or pass \
             --allow-unchecked to accept that.",
            missing_kvg.len(),
            missing_kvg.join("\n  ")
        )
        .into());
    }
    if !allow_unchecked && checked_against_kvg != joyo.len() {
        return Err(format!(
            "only {checked_against_kvg} of {} jōyō characters could be checked against KanjiVG, so \
             their stroke counts rest on this code's own assumptions. Run scripts/fetch-data.sh \
             to fetch kvgJa/, or pass --allow-unchecked to accept that.",
            joyo.len()
        )
        .into());
    }
    for k in &kanji {
        if !k.is_practisable() {
            ungradeable.push(format!("{} (U+{:04X})", k.ch, k.ch as u32));
        }
    }
    if !ungradeable.is_empty() {
        return Err(format!(
            "{} characters would be written into the artifact without usable geometry, so a \
             course could offer something the board cannot grade:\n  {}",
            ungradeable.len(),
            ungradeable.join("\n  ")
        )
        .into());
    }
    let ungradeable_radicals: Vec<String> = radicals
        .iter()
        .filter(|r| !r.is_practisable())
        .map(|r| format!("{} (radical {})", r.ch, r.number))
        .collect();
    if !ungradeable_radicals.is_empty() {
        return Err(format!(
            "{} radicals would be written without usable geometry:\n  {}",
            ungradeable_radicals.len(),
            ungradeable_radicals.join("\n  ")
        )
        .into());
    }

    let dataset = KanjiDataset::from_parts(kanji, radicals, source.clone());
    if dataset.len() != JOYO_COUNT {
        return Err(format!(
            "assembled {} characters, expected {JOYO_COUNT}",
            dataset.len()
        )
        .into());
    }
    if dataset.radicals().len() != RADICAL_COUNT {
        return Err(format!(
            "assembled {} radicals, expected {RADICAL_COUNT}",
            dataset.radicals().len()
        )
        .into());
    }
    let grade_counts = dataset.grade_counts();
    let kyoiku: usize = grade_counts
        .iter()
        .filter(|(grade, _)| *grade <= 6)
        .map(|(_, count)| count)
        .sum();
    // How much of the Kangxi set the jōyō characters actually use. Measured here
    // so the number in the README, the panel and the tests is one number.
    let families = dataset.radical_families();
    let radicals_in_use = families.iter().filter(|f| !f.characters.is_empty()).count();

    // Sort by code point before writing, so the artifact is byte-for-byte
    // reproducible — `from_kanji` has done that, and this makes it explicit that
    // nothing after it may reorder.
    let mut payload_kanji: Vec<Kanji> = dataset.kanji().to_vec();
    payload_kanji.sort_by_key(|k| k.ch as u32);

    let payload = postcard::to_allocvec(&KanjiArtifact::new(
        payload_kanji,
        dataset.radicals().to_vec(),
        source,
    ))?;
    let mut raw = Vec::with_capacity(payload.len() + KANJI_ARTIFACT_MAGIC.len());
    raw.extend_from_slice(KANJI_ARTIFACT_MAGIC);
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
    let total_strokes: usize = dataset
        .kanji()
        .iter()
        .map(|k| k.stroke_count as usize)
        .sum();
    let avg_strokes = total_strokes as f64 / dataset.len().max(1) as f64;
    let grades: Vec<String> = grade_counts
        .iter()
        .map(|(grade, count)| format!("g{grade}:{count}"))
        .collect();

    println!("prepare-kanji: wrote {}", out_path.display());
    println!(
        "  kanjidic2              {} ({})",
        dataset.source().kanjidic2_version,
        dataset.source().kanjidic2_date
    );
    println!("  graphics rows          {graphics_rows}");
    println!(
        "  kanji                  {} ({kyoiku} kyōiku, {} jōyō remainder)",
        dataset.len(),
        dataset.len() - kyoiku
    );
    println!("  total strokes          {total_strokes} (average {avg_strokes:.2})");
    println!("  grades from KANJIDIC2  {}", grades.join(" "));
    let radical_strokes: usize = dataset
        .radicals()
        .iter()
        .map(|r| r.stroke_count as usize)
        .sum();
    println!(
        "  radicals               {RADICAL_COUNT} ({radicals_in_use} used by jōyō, {} unused), \
         {radical_strokes} strokes",
        RADICAL_COUNT - radicals_in_use
    );
    println!(
        "  radical numbering      {radicals_checked} agree with KANJIDIC2's classical number, {} \
         it cannot number ({})",
        cannot_number.len(),
        cannot_number.join(" ")
    );
    // Every number here is counted, none is asserted: if the two sources ever
    // agree about more or fewer characters, this line says so.
    println!(
        "  dictionaryJa grades    {dictionary_agrees} agree, {} differ ({added_in_2017} added in \
         2017, {reassigned} reassigned)",
        added_in_2017 + reassigned
    );
    let exceptions: Vec<String> = KANJIVG_DISAGREES
        .iter()
        .map(|(ch, taught, kvg)| format!("{ch}: geometry {taught} vs KanjiVG {kvg}"))
        .collect();
    println!(
        "  checked against KanjiVG {checked_against_kvg}{}",
        if exceptions.is_empty() {
            String::new()
        } else {
            format!(" (written disagreements: {})", exceptions.join(", "))
        }
    );
    println!(
        "  artifact               {:.0} KB compressed from {:.0} KB ({:.0}%)",
        artifact_bytes as f64 / 1e3,
        raw.len() as f64 / 1e3,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    Ok(())
}

/// The one character in a JSON-lines `character` field, or `None`.
fn single_char(field: &str) -> Option<char> {
    let mut chars = field.chars();
    let ch = chars.next()?;
    match chars.next() {
        None => Some(ch),
        Some(_) => None,
    }
}
