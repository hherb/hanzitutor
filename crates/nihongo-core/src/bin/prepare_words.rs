//! Turn the upstream Japanese dictionary material into the compact vocabulary
//! artifact the app embeds.
//!
//! Four inputs, all fetched by `scripts/fetch-data.sh`, and the split between
//! them is the part worth reading:
//!
//! * `jmdict-eng.json` — EDRDG's JMdict (CC BY-SA 4.0), republished as one JSON
//!   document by `scriptin/jmdict-simplified`. The authority for a word's written
//!   form, its **own** reading (never composed from its characters), its English
//!   glosses, and which forms are common.
//! * `JMdict_e.gz` — EDRDG's own XML, read for **the priority markers and
//!   nothing else**: the `nf01`–`nf48` rank, and the `ichi1`/`ichi2` markers that
//!   name the most-common-word corpus. The JSON reformatting drops all of them,
//!   which was measured rather than assumed; the XML is scanned for `<ent_seq>`
//!   and `<ke_pri>`/`<re_pri>` and nothing else is parsed.
//! * `JmdictFurigana.json` — [JmdictFurigana](https://github.com/Doublevil/JmdictFurigana)
//!   (MIT), which puts each reading over the characters it belongs to. Its file
//!   starts with a **UTF-8 BOM**, which `serde_json` rejects with a message about
//!   line 1 column 1, so it is read through [`read_json_stripping_bom`].
//! * `crates/nihongo-core/data/kanji.bin.gz` — **the committed kanji artifact**,
//!   not an upstream file. It is the membership test: a word is carried only when
//!   every one of its kanji is a character that artifact holds, so the vocabulary
//!   course can never offer a word the board cannot draw. Run `prepare-kanji`
//!   before this if that file is missing.
//!
//! Run: `pnpm run prepare-words` (add `--features nihongo-core/prepare` to a
//! plain `cargo run`).

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use nihongo_core::words::{
    band_for, is_kanji, Ruby, Word, WordsArtifact, WordDataset, WordsSource,
    WORDS_ARTIFACT_MAGIC,
};
use nihongo_core::KanjiDataset;
use serde::Deserialize;

/// The JMdict entry, in the shape `scriptin/jmdict-simplified` publishes.
#[derive(Deserialize)]
struct Jmdict {
    version: String,
    #[serde(rename = "dictDate")]
    dict_date: String,
    words: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    #[serde(default)]
    kanji: Vec<Form>,
    #[serde(default)]
    kana: Vec<KanaForm>,
    #[serde(default)]
    sense: Vec<Sense>,
}

#[derive(Deserialize)]
struct Form {
    text: String,
    #[serde(default)]
    common: bool,
    #[serde(default)]
    tags: Vec<String>,
}

/// The tags that mark a written form as one nobody actually writes.
///
/// EDRDG marks the *entry* common but can still tag the spelling: `ateji` is a
/// kanji spelling chosen for its sound (型録 for カタログ), and `ik`/`oK`/`rK`/`sK`
/// are irregular, outdated, rare and search-only kanji forms. 173 of the entries
/// this pipeline would otherwise carry have a common form carrying one of these,
/// and teaching them would teach 彼処 for あそこ.
///
/// It is not a cosmetic filter: the words it removes are words whose ordinary
/// spelling is kana, and a *kanji* course that spells them with kanji is teaching
/// a spelling its learner will never meet.
const RARE_SPELLINGS: [&str; 7] = ["ateji", "ik", "oK", "rK", "sK", "io", "gikun"];

impl Form {
    /// Whether this is a spelling worth teaching.
    fn is_teachable(&self) -> bool {
        self.common
            && !self
                .tags
                .iter()
                .any(|tag| RARE_SPELLINGS.contains(&tag.as_str()))
    }
}

#[derive(Deserialize)]
struct KanaForm {
    text: String,
    #[serde(default)]
    common: bool,
    #[serde(default)]
    applies_to_kanji: Vec<String>,
}

#[derive(Deserialize)]
struct Sense {
    #[serde(default)]
    gloss: Vec<Gloss>,
}

#[derive(Deserialize)]
struct Gloss {
    lang: String,
    text: String,
}

/// One JmdictFurigana entry. The JSON is a bare array of these.
#[derive(Deserialize)]
struct FuriganaEntry {
    text: String,
    reading: String,
    #[serde(default)]
    furigana: Vec<FuriganaPart>,
}

#[derive(Deserialize)]
struct FuriganaPart {
    ruby: String,
    #[serde(default)]
    rt: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-words: {err}");
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

    let mut raw_dir = repo_root.join("data/raw");
    let mut out_path = crate_dir.join("data/words.bin.gz");
    let mut kanji_path = crate_dir.join("data/kanji.bin.gz");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--raw" => raw_dir = PathBuf::from(args.next().ok_or("--raw needs a directory")?),
            "--out" => out_path = PathBuf::from(args.next().ok_or("--out needs a file")?),
            "--kanji" => kanji_path = PathBuf::from(args.next().ok_or("--kanji needs a file")?),
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    let jmdict_path = raw_dir.join("jmdict-eng.json");
    let xml_path = raw_dir.join("JMdict_e.gz");
    let furigana_path = raw_dir.join("JmdictFurigana.json");
    for path in [&jmdict_path, &xml_path, &furigana_path] {
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-data.sh first",
                path.display()
            )
            .into());
        }
    }
    if !kanji_path.exists() {
        return Err(format!(
            "missing {} — run `pnpm run prepare-kanji` first: the words are chosen by the \
             characters the kanji artifact holds, so that a word this course offers is always \
             one the board can draw",
            kanji_path.display()
        )
        .into());
    }

    // ---- The kanji artifact decides which words are teachable ----------------
    let kanji = KanjiDataset::from_gzip_bytes(&std::fs::read(&kanji_path)?)
        .map_err(|e| format!("{}: {e}", kanji_path.display()))?;

    // ---- EDRDG's priority markers, from the XML the JSON reformatting drops
    let (priorities, nf_source_date) = read_priorities(&xml_path)?;

    // ---- JMdict: written forms, readings, glosses ---------------------------
    let document: Jmdict = serde_json::from_reader(BufReader::new(File::open(&jmdict_path)?))
        .map_err(|e| format!("{}: {e}", jmdict_path.display()))?;
    let source = WordsSource {
        jmdict_version: document.version.clone(),
        jmdict_date: document.dict_date.clone(),
        nf_source_date,
        furigana_release: FURIGANA_RELEASE.to_string(),
    };

    // ---- JmdictFurigana: the alignment --------------------------------------
    let furigana: HashMap<(String, String), Vec<Ruby>> =
        serde_json::from_str::<Vec<FuriganaEntry>>(&read_json_stripping_bom(&furigana_path)?)
            .map_err(|e| format!("{}: {e}", furigana_path.display()))?
            .into_iter()
            .map(|entry| {
                (
                    (entry.text, entry.reading),
                    entry
                        .furigana
                        .into_iter()
                        .map(|part| Ruby::new(part.ruby, part.rt))
                        .collect(),
                )
            })
            .collect();

    // ---- Join them, and say what was left out and why ------------------------
    let mut words: Vec<Word> = Vec::new();
    let mut seen: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    let mut ranked = 0usize;
    let mut common_but_unranked = 0usize;
    let mut not_common = 0usize;
    let mut kana_only = 0usize;
    let mut no_common_spelling = 0usize;
    let mut outside_the_kanji_set = 0usize;
    let mut without_furigana: Vec<String> = Vec::new();

    for entry in &document.words {
        // Two of EDRDG's signals, and either is enough: the `nf` rank, or the
        // `ichi` marker. See `Priority::is_common` for why the rank alone drops
        // 行く and 本.
        let priority = priorities.get(&entry.id).copied().unwrap_or_default();
        if !priority.is_common() {
            not_common += 1;
            continue;
        }
        if priority.nf.is_some() {
            ranked += 1;
        } else {
            common_but_unranked += 1;
        }

        // The written form: the first one EDRDG marks common **and** does not tag
        // as a rare or ateji spelling. There is deliberately no fallback to an
        // unmarked form — 7,160 entries of this dictionary are common words whose
        // kanji spelling is *not* what people write (あそこ, おにぎり, さっき), and
        // teaching those as kanji words would be worse than leaving them to the
        // kana course.
        let Some(form) = entry.kanji.iter().find(|form| form.is_teachable()) else {
            if entry.kanji.is_empty() {
                kana_only += 1;
            } else {
                no_common_spelling += 1;
            }
            continue;
        };
        let text = &form.text;

        // Having a form in JMdict's `kanji` list is not the same as containing a
        // kanji: full-width numerals like `１０００` live there too. A word with no
        // kanji character teaches no character, so it is not this course's, and it
        // must be excluded before the band is computed — an empty grade list would
        // otherwise drop it into band 1.
        let kanji_chars: Vec<char> = text.chars().filter(|c| is_kanji(*c)).collect();
        if kanji_chars.is_empty() {
            kana_only += 1;
            continue;
        }

        // The membership test. A single kanji outside the artifact is enough to
        // leave the word out, and it becomes teachable the day that set is added.
        if kanji_chars.iter().any(|c| kanji.get(*c).is_none()) {
            outside_the_kanji_set += 1;
            continue;
        }

        // The reading: one of the entry's own kana forms that applies to this
        // written form, common first. **Chosen, never composed** — that is the
        // whole point of carrying a reading per word.
        let reading = entry
            .kana
            .iter()
            .filter(|kana| applies(kana, text))
            .min_by_key(|kana| !kana.common)
            .or_else(|| entry.kana.first())
            .map(|kana| kana.text.clone());
        let Some(reading) = reading else {
            return Err(format!("JMdict entry {} ({text}) has no kana reading", entry.id).into());
        };

        if !seen.insert((text.clone(), reading.clone())) {
            continue;
        }

        let grades: Vec<u8> = kanji_chars
            .iter()
            .filter_map(|c| kanji.get(*c).map(|k| k.grade))
            .collect();
        let band = band_for(grades);

        let aligned = furigana
            .get(&(text.clone(), reading.clone()))
            .cloned()
            .unwrap_or_default();
        if aligned.is_empty() {
            without_furigana.push(format!("{text} ({reading})"));
        }

        let meaning = entry
            .sense
            .first()
            .map(|sense| {
                sense
                    .gloss
                    .iter()
                    .filter(|gloss| gloss.lang == "eng")
                    .map(|gloss| gloss.text.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_default();

        words.push(Word {
            text: text.clone(),
            reading,
            furigana: aligned,
            meaning,
            band,
            nf: priority.nf,
        });
    }

    if words.is_empty() {
        return Err("no words survived the join — has an upstream file changed shape?".into());
    }

    let dataset = WordDataset::from_words(words, source.clone());
    let band_counts = dataset.band_counts();
    let total: usize = band_counts.iter().map(|(_, count)| count).sum();
    let with_furigana = dataset.words().iter().filter(|w| w.has_furigana()).count();
    let total_nf: usize = dataset
        .words()
        .iter()
        .filter_map(|w| w.nf)
        .map(usize::from)
        .sum();
    let ranked_words = dataset.words().iter().filter(|w| w.nf.is_some()).count();

    let payload = postcard::to_allocvec(&WordsArtifact::new(dataset.words().to_vec(), source))?;
    let mut raw = Vec::with_capacity(payload.len() + WORDS_ARTIFACT_MAGIC.len());
    raw.extend_from_slice(WORDS_ARTIFACT_MAGIC);
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
    let bands: Vec<String> = band_counts
        .iter()
        .map(|(band, count)| format!("b{band}:{count}"))
        .collect();

    println!("prepare-words: wrote {}", out_path.display());
    println!(
        "  jmdict                 {} ({})",
        dataset.source().jmdict_version,
        dataset.source().jmdict_date
    );
    println!(
        "  priority markers       from JMdict_e created {}, {} entries ranked and {} marked \
         common without a rank",
        dataset.source().nf_source_date, ranked, common_but_unranked
    );
    println!("  furigana               {}", dataset.source().furigana_release);
    println!(
        "  considered             {} of {} JMdict entries ({not_common} not marked common)",
        ranked + common_but_unranked,
        document.words.len()
    );
    println!("  words                  {total} in {bands:?}");
    println!(
        "  left out               {kana_only} with no kanji, {no_common_spelling} whose kanji \
         spelling is not the common one, {outside_the_kanji_set} with a kanji the artifact does \
         not hold"
    );
    println!(
        "  furigana               {with_furigana} of {total} ({} without, gathered rather \
         than invented)",
        without_furigana.len()
    );
    println!(
        "  mean nf rank           {:.1} over the {ranked_words} ranked words",
        total_nf as f64 / ranked_words.max(1) as f64
    );
    println!(
        "  artifact               {:.0} KB compressed from {:.0} KB ({:.0}%)",
        artifact_bytes as f64 / 1e3,
        raw.len() as f64 / 1e3,
        100.0 * artifact_bytes as f64 / raw.len() as f64
    );
    if !without_furigana.is_empty() {
        let shown: Vec<&str> = without_furigana.iter().take(10).map(String::as_str).collect();
        println!(
            "  no alignment for       {}{}",
            shown.join(", "),
            if without_furigana.len() > shown.len() { ", …" } else { "" }
        );
    }
    Ok(())
}

/// The JmdictFurigana release the alignments are taken from.
///
/// A constant rather than an argument because it is half of a pinned pair: the
/// fetch script downloads this release and the artifact records it, so a rebuild
/// cannot silently pick up a different one.
const FURIGANA_RELEASE: &str = "2.3.1+2026-09-25";

/// Whether a kana form is a reading of `text`.
///
/// JMdict says this with `appliesToKanji`, which is either `["*"]` or a list of
/// the written forms the reading belongs to. An empty list is not a promise of
/// anything, so it is treated as applying.
fn applies(kana: &KanaForm, text: &str) -> bool {
    kana.applies_to_kanji.is_empty()
        || kana
            .applies_to_kanji
            .iter()
            .any(|applies_to| applies_to == "*" || applies_to == text)
}

/// What EDRDG says about one entry's commonness.
#[derive(Clone, Copy, Debug, Default)]
struct Priority {
    /// The `nf01`–`nf48` frequency block, smaller being more frequent.
    nf: Option<u8>,
    /// Whether the entry carries `ichi1`/`ichi2`, EDRDG's marker for the words of
    /// its most-common-word corpus. Some of those carry no `nf` rank at all —
    /// 行く and 本 among them — which is why membership is this `or` and not the
    /// rank alone.
    ichi: bool,
}

impl Priority {
    /// Whether EDRDG considers the entry worth teaching.
    ///
    /// `spec1`/`spec2` (specialist vocabulary) and `gai1` (loanwords) are
    /// deliberately not part of this: they would add about 1,200 words of
    /// technical and foreign terminology to a beginner's course.
    fn is_common(self) -> bool {
        self.nf.is_some() || self.ichi
    }
}

/// Read EDRDG's priority markers out of `JMdict_e.gz`, with the file's own
/// creation date.
///
/// A targeted scan rather than an XML parser, and that is a deliberate choice:
/// the only things wanted are the entry's `ent_seq` and the tokens in its
/// `ke_pri`/`re_pri` elements, all of which are ASCII words with no entities and
/// no nesting to speak of. Pulling in an XML dependency to read two fields would
/// be the tail wagging the dog. The join key is `ent_seq`, which is the `id` the
/// JSON reformatting publishes, so the two files meet exactly.
///
/// The entry with the **best** (smallest) nf wins when a word has several, which
/// is what "this word is in the 500-word block `nf`" means for the word as a
/// whole rather than for one of its spellings.
fn read_priorities(
    path: &PathBuf,
) -> Result<(HashMap<String, Priority>, String), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(flate2::read::GzDecoder::new(file));

    let mut priorities: HashMap<String, Priority> = HashMap::new();
    let mut current: Option<String> = None;
    let mut created = String::new();
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        if created.is_empty() {
            if let Some(rest) = line.split("JMdict created:").nth(1) {
                created = rest
                    .trim()
                    .trim_start_matches("--")
                    .trim()
                    .trim_end_matches("-->")
                    .trim()
                    .to_string();
            }
        }
        if let Some(rest) = line.split("<ent_seq>").nth(1) {
            if let Some(id) = rest.split("</ent_seq>").next() {
                current = Some(id.trim().to_string());
            }
            continue;
        }
        for token in priority_tokens(&line) {
            if let Some(id) = &current {
                let entry = priorities.entry(id.clone()).or_default();
                if let Some(number) = token.strip_prefix("nf") {
                    if let Ok(rank) = number.parse::<u8>() {
                        if rank >= 1 {
                            entry.nf = Some(entry.nf.map_or(rank, |best| best.min(rank)));
                        }
                    }
                } else if token == "ichi1" || token == "ichi2" {
                    entry.ichi = true;
                }
            }
        }
    }

    if priorities.is_empty() {
        return Err(format!(
            "{} yielded no priority markers — is it EDRDG's JMdict_e, and has the format \
             changed?",
            path.display()
        )
        .into());
    }
    Ok((priorities, created))
}

/// The tokens in one line's `<ke_pri>`/`<re_pri>` elements.
fn priority_tokens(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for element in ["ke_pri", "re_pri"] {
        let open = format!("<{element}>");
        let close = format!("</{element}>");
        let mut rest = line;
        while let Some(start) = rest.find(&open) {
            let after = &rest[start + open.len()..];
            let Some(end) = after.find(&close) else { break };
            out.push(after[..end].trim());
            rest = &after[end + close.len()..];
        }
    }
    out
}

/// Read a UTF-8 file, dropping a byte-order mark if it has one.
///
/// JmdictFurigana's JSON starts with one, and `serde_json` reports that as
/// "expected value at line 1 column 1" — a message that says nothing about a BOM
/// and sends the reader looking for a syntax error that is not there.
fn read_json_stripping_bom(path: &PathBuf) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    Ok(String::from_utf8(body.to_vec())?)
}
