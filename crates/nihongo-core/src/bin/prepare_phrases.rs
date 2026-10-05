//! Turn Tatoeba's Japanese sentences into the graded phrase artifact the Phrases
//! screen draws.
//!
//! # What it does, in order
//!
//! 1. reads the Japanese sentences with their contributors,
//! 2. keeps the ones whose English translation the corpus publishes, since a phrase
//!    with no translation is not one this screen can offer,
//! 3. keeps the ones the corpus **names a contributor for** and has **not tagged**
//!    `not a sentence` or `@possible copyright infringement` — the first because CC
//!    BY 2.0 FR requires the author to be named and the export carries no name for
//!    42.7% of its Japanese sentences, the second because there is no shortage of
//!    sentences that are neither,
//! 4. keeps the ones that are **one short sentence** — a length band, one terminal
//!    punctuation mark, no Latin letters, no brackets or quotation marks, no
//!    whitespace — because a phrase list is read, not parsed,
//! 5. throws away a short, deliberate list of sentences a course should not put in
//!    front of a learner (see [`BANNED`]),
//! 6. segments what is left with UniDic and **keeps only the sentences every
//!    kanji-bearing word of which the course teaches**, which is the invariant
//!    `prepare-passages` enforces by refusing to write and this pipeline enforces by
//!    dropping,
//! 7. levels each sentence on this course's own ladder — its **hardest word** — and
//!    keeps the shortest [`Options::per_band`] of each band, and
//! 8. writes the artifact and the attribution notice the corpus's licence requires.
//!
//! # Why it drops rather than refuses
//!
//! `prepare-passages` **refuses to write** an artifact containing a kanji the
//! vocabulary does not hold, because a passage is written here and can be fixed. An
//! imported sentence cannot be fixed, only kept or thrown away: there are hundreds
//! of thousands of them, and the ones this course cannot read are simply not part of
//! the corpus it ships. That difference is the whole reason a filter needs a *funnel
//! report* rather than an error message — the numbers printed at the end are what
//! say whether the cut was reasonable.
//!
//! # The licence
//!
//! Tatoeba's exports are CC BY 2.0 FR, with a part of the sentences under CC0 1.0;
//! the per-language files carry no per-sentence licence column, so the corpus's own
//! statement is the basis and `jpn_sentences_CC0.tsv` is what separates the CC0 part
//! out. Both licences permit redistribution and adaptation; CC BY requires the
//! author to be named, which is what the attribution file this writes is for. See
//! `scripts/fetch-tatoeba.sh` for the measurement and `LICENSES.md` for the record.
//!
//! Run: `pnpm run prepare-phrases`, after `scripts/fetch-tatoeba.sh` once and
//! `pnpm run prepare-words` for the vocabulary it is held to.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use nihongo_core::phrases::{
    Phrase, PhraseDataset, PhraseToken, PhrasesArtifact, PhrasesSource, PHRASES_ARTIFACT_MAGIC,
};
use nihongo_core::segment::analyse;
use nihongo_core::{is_kanji, WordDataset};

/// The analyser this pipeline segments with, and the dictionary it uses. The same
/// two constants `prepare-passages` records, for the same reason: the artifact
/// states who read the text, and half a version is worse than none.
const LINDERA_VERSION: &str = "6.2";
const UNIDIC_VERSION: &str = "unidic-mecab-2.1.2";

/// The two licences the corpus offers its sentences under.
const CC_BY: &str = "CC BY 2.0 FR";
const CC0: &str = "CC0 1.0";

/// The marker the exports use for a value they do not have.
///
/// A literal two-character `\N`, not an empty field and not `NULL`, and it is the
/// whole reason the attribution rule below can be written at all: a `\N` contributor
/// is *not* an empty string, so a parser that skips empty fields would carry the
/// literal `\N` into the artifact and name a contributor called `\N`.
const MISSING: &str = "\\N";

/// Sentences this pipeline will not ship, whatever else is true of them.
///
/// A hand-written list, and it is a judgement rather than a rule: Tatoeba is
/// user-contributed and contains insults and sexual vocabulary, and a language
/// course is not the place for them. The list is deliberately short — matching on
/// stems would start throwing away ordinary sentences, since 死 is part of words a
/// course may well teach — so it names the few a phrase list genuinely cannot
/// carry. Edit it here; there is nowhere else.
const BANNED: [&str; 9] = [
    "馬鹿", "バカ", "ばか", "死ね", "殺す", "殺し", "セックス", "おっぱい", "ちんこ",
];

/// The community tags that exclude a sentence outright.
///
/// The rest of the corpus's tags — `colloquial`, `proverb`, `weather`, 397 of them —
/// are descriptions this course has no business judging on. These two are different:
/// one says the text is not a sentence at all, and the other is the corpus's own
/// members flagging a sentence whose copyright they are unsure of. Neither is worth
/// arguing with when there are a hundred thousand candidates.
const EXCLUDED_TAGS: [&str; 2] = ["not a sentence", "@possible copyright infringement"];

/// What the cut is, and where it may be changed.
struct Options {
    /// The directory `scripts/fetch-tatoeba.sh` writes the exports into.
    tatoeba: PathBuf,
    words: PathBuf,
    out: PathBuf,
    attribution: PathBuf,
    /// The shortest and longest sentence to keep, in characters.
    min_length: usize,
    max_length: usize,
    /// How many phrases of each band to keep. The shortest are kept first, so a
    /// band's easiest sentences are the ones that survive the cap.
    per_band: usize,
    /// How many phrases of each band to print after the report, for a human
    /// rebuilding the corpus and wanting to see what the cut produced. Zero prints
    /// none, which is what a scripted run wants.
    show: usize,
}

impl Default for Options {
    fn default() -> Self {
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = repo_root();
        Self {
            tatoeba: root.join("data/raw/tatoeba"),
            words: crate_dir.join("data/words.bin.gz"),
            out: crate_dir.join("data/phrases.bin.gz"),
            // The notice lives beside the app that ships it, not in `data/`: it is
            // compiled into the binary by `licences.rs` with `include_str!` and
            // copied into the bundle from there, and a file the app embeds cannot
            // live in a directory nothing else reads.
            attribution: root.join("apps/nihongo-tutor/src-tauri/licences/TATOEBA-phrases.txt"),
            min_length: 6,
            max_length: 32,
            per_band: 200,
            show: 0,
        }
    }
}

/// How many sentences each step of the cut left behind, so the run says what it did
/// rather than only what it kept.
///
/// The named counters are the funnel the report prints; `reasons` is the same drops
/// grouped by the sentence that caused them, which is the half a reader can act on.
#[derive(Default)]
struct Funnel {
    read: usize,
    // The exports' own gates, checked before anything is analysed.
    /// No English translation in the corpus's links.
    no_english: usize,
    /// The export names no contributor, and CC BY requires the author to be named.
    no_author: usize,
    /// The community tagged it `not a sentence` or `@possible copyright infringement`.
    excluded_tag: usize,
    /// The English translation is linked but absent from the English export.
    english_absent: usize,
    /// Not the shape of a phrase.
    shaped_out: usize,
    /// On the do-not-ship list.
    banned_out: usize,
    /// Contains a kanji no taught word contains, caught before segmenting.
    kanji_unknown: usize,
    /// Segmented, but a kanji-bearing token is not a word the course teaches.
    word_unknown: usize,
    /// Segmented, but a kanji-bearing token came back with no reading.
    unreadable: usize,
    /// The analyser itself refused the text.
    analyser_failed: usize,
    /// The analyser found nothing in it.
    empty: usize,
    /// Dropped by the per-band cap.
    capped: usize,
    /// Reached the end of the filter with every kanji covered.
    kanji_covered: usize,
    /// Survived segmentation and the vocabulary check.
    segmented: usize,
    /// Every drop, in the order the reasons were first met.
    reasons: Vec<(&'static str, usize)>,
}

impl Funnel {
    fn drop_for(&mut self, reason: &'static str) {
        if let Some(entry) = self.reasons.iter_mut().find(|(name, _)| *name == reason) {
            entry.1 += 1;
        } else {
            self.reasons.push((reason, 1));
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("prepare-phrases: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut options = Options::default();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tatoeba" => {
                options.tatoeba = PathBuf::from(args.next().ok_or("--tatoeba needs a directory")?)
            }
            "--words" => options.words = PathBuf::from(args.next().ok_or("--words needs a file")?),
            "--out" => options.out = PathBuf::from(args.next().ok_or("--out needs a file")?),
            "--attribution" => {
                options.attribution =
                    PathBuf::from(args.next().ok_or("--attribution needs a file")?)
            }
            "--min-length" => {
                options.min_length = args.next().ok_or("--min-length needs a number")?.parse()?
            }
            "--max-length" => {
                options.max_length = args.next().ok_or("--max-length needs a number")?.parse()?
            }
            "--per-band" => {
                options.per_band = args.next().ok_or("--per-band needs a number")?.parse()?
            }
            "--show" => options.show = args.next().ok_or("--show needs a number")?.parse()?,
            other => return Err(format!("unknown argument {other:?}").into()),
        }
    }

    if !options.words.exists() {
        return Err(format!(
            "missing {} — run `pnpm run prepare-words` first: the phrases are held to the \
             vocabulary it holds",
            options.words.display()
        )
        .into());
    }
    let words = WordDataset::from_gzip_bytes(&std::fs::read(&options.words)?)
        .map_err(|e| format!("{}: {e}", options.words.display()))?;

    let jpn_path = options.tatoeba.join("jpn_sentences_detailed.tsv");
    let links_path = options.tatoeba.join("jpn-eng_links.tsv");
    let eng_path = options.tatoeba.join("eng_sentences.tsv");
    let cc0_path = options.tatoeba.join("jpn_sentences_CC0.tsv");
    let tags_path = options.tatoeba.join("jpn_tags.tsv");
    for (path, archive) in [
        (&jpn_path, "jpn_sentences_detailed.tsv.bz2"),
        (&links_path, "jpn-eng_links.tsv.bz2"),
        (&eng_path, "eng_sentences.tsv.bz2"),
        // Required rather than optional, unlike the CC0 list: a sentence the corpus
        // itself has flagged as possibly infringing must not be shipped because a
        // file happened to be absent, and the exclusion cannot be reconstructed
        // without it.
        (&tags_path, "jpn_tags.tsv.bz2"),
    ] {
        if !path.exists() {
            return Err(format!(
                "missing {} — run scripts/fetch-tatoeba.sh once (it downloads about 34 MB and \
                 unpacks it), which fetches {archive}",
                path.display()
            )
            .into());
        }
    }
    // Not an error: the file is 228 bytes and a checkout that predates it can still
    // build. Saying so is better than silently assuming every sentence is CC BY.
    let mut funnel = Funnel::default();
    let cc0: HashSet<u32> = match read_ids(&cc0_path) {
        Ok(ids) => ids,
        Err(_) => {
            eprintln!(
                "prepare-phrases: {} is missing or unreadable, so every sentence is treated as \
                 {CC_BY} — re-run scripts/fetch-tatoeba.sh to have the CC0 sentences named",
                cc0_path.display()
            );
            HashSet::new()
        }
    };
    let excluded: HashSet<u32> = tagged_with(&tags_path, &EXCLUDED_TAGS)?;

    // ---- The sentences, then the English they are paired with -----------------
    //
    // Read in three passes rather than one, and the order is what keeps the memory
    // bounded: the English export is every English sentence Tatoeba has, and only
    // the few thousand ids a phrase can actually use are worth holding.
    let mut sentences: Vec<(u32, String, String)> = Vec::new();
    let mut by_id: HashSet<u32> = HashSet::new();
    for line in lines(&jpn_path)? {
        let line = line?;
        let columns: Vec<&str> = line.split('\t').collect();
        // id, lang, text, contributor, date added, date modified.
        let (Some(id), Some(text), Some(author)) =
            (columns.first(), columns.get(2), columns.get(3))
        else {
            continue;
        };
        let Ok(id) = id.parse::<u32>() else { continue };
        if !text.is_empty() && by_id.insert(id) {
            sentences.push((id, text.to_string(), author.to_string()));
        }
    }
    funnel.read = sentences.len();

    // The shortest English id per Japanese sentence, so a sentence with several
    // translations takes the first one by id and the result is deterministic.
    let mut english_id: HashMap<u32, u32> = HashMap::new();
    for line in lines(&links_path)? {
        let line = line?;
        let columns: Vec<&str> = line.split('\t').collect();
        let (Some(jpn), Some(eng)) = (columns.first(), columns.get(1)) else {
            continue;
        };
        let (Ok(jpn), Ok(eng)) = (jpn.parse::<u32>(), eng.parse::<u32>()) else {
            continue;
        };
        if !by_id.contains(&jpn) {
            continue;
        }
        english_id
            .entry(jpn)
            .and_modify(|current| {
                if eng < *current {
                    *current = eng;
                }
            })
            .or_insert(eng);
    }

    // ---- The shape of a phrase, before any of it is analysed ------------------
    //
    // Cheap by comparison with segmentation, so it runs first: a quarter of a million
    // sentences in, and only the survivors are worth a dictionary lookup.
    let mut shaped: Vec<(u32, String, String)> = Vec::new();
    for (id, text, author) in sentences.drain(..) {
        if !english_id.contains_key(&id) {
            funnel.no_english += 1;
            funnel.drop_for("no English translation in the corpus");
            continue;
        }
        // The attribution gate, and it comes before anything that costs work: CC BY
        // 2.0 FR requires the author to be named, and the export carries no name for
        // 42.7% of the Japanese sentences it holds. Those are not shipped. There is
        // no shortage — well over a hundred thousand candidates do name one — so the
        // choice is between a corpus with a hole in its attribution and a smaller one
        // without, and the licence is not negotiable.
        if author.trim().is_empty() || author == MISSING {
            funnel.no_author += 1;
            funnel.drop_for("the corpus names no contributor");
            continue;
        }
        if excluded.contains(&id) {
            funnel.excluded_tag += 1;
            funnel.drop_for("tagged `not a sentence` or `@possible copyright infringement`");
            continue;
        }
        if let Err(reason) = shape_of(&text, options.min_length, options.max_length) {
            funnel.shaped_out += 1;
            funnel.drop_for(reason);
            continue;
        }
        if BANNED.iter().any(|word| text.contains(word)) {
            funnel.banned_out += 1;
            funnel.drop_for("on the do-not-ship list");
            continue;
        }
        shaped.push((id, text, author));
    }

    // ---- The English texts of what is left -----------------------------------
    let needed: HashSet<u32> = shaped.iter().map(|(id, _, _)| english_id[id]).collect();
    let mut english: HashMap<u32, String> = HashMap::with_capacity(needed.len());
    for line in lines(&eng_path)? {
        let line = line?;
        let columns: Vec<&str> = line.split('\t').collect();
        let (Some(id), Some(text)) = (columns.first(), columns.get(2)) else {
            continue;
        };
        let Ok(id) = id.parse::<u32>() else { continue };
        if needed.contains(&id) {
            english.entry(id).or_insert_with(|| text.to_string());
        }
    }

    // A sentence whose English is missing from the export cannot carry a
    // translation, so it goes the way of one that never had a link.
    let mut candidates: Vec<(u32, String, String, String)> = Vec::new();
    for (id, text, author) in shaped {
        match english.get(&english_id[&id]) {
            Some(translation) => candidates.push((id, text, author, translation.clone())),
            None => {
                funnel.english_absent += 1;
                funnel.drop_for("English text absent from the export");
            }
        }
    }

    // ---- The kanji the course knows, as a set, before segmenting anything -----
    //
    // A sentence containing a kanji no taught word contains can never satisfy the
    // vocabulary rule, whatever the analyser does with it. That is a set lookup per
    // character, and it removes most of the corpus before the 134 MB dictionary is
    // asked about any of it.
    let mut known_kanji: HashSet<char> = HashSet::new();
    for word in words.words() {
        for ch in word.text.chars() {
            if is_kanji(ch) {
                known_kanji.insert(ch);
            }
        }
    }
    let mut analysable: Vec<(u32, String, String, String)> = Vec::new();
    for (id, text, author, translation) in candidates {
        if text.chars().any(|ch| is_kanji(ch) && !known_kanji.contains(&ch)) {
            funnel.kanji_unknown += 1;
            funnel.drop_for("a kanji no taught word contains");
            continue;
        }
        analysable.push((id, text, author, translation));
    }
    funnel.kanji_covered = analysable.len();

    // ---- The analyser --------------------------------------------------------
    let dictionary_dir = find_dictionary(&repo_root())?.ok_or_else(|| {
        "no UniDic dictionary under .lindera/ — run scripts/fetch-unidic.sh once (it downloads \
         about 134 MB and builds a dictionary from it)"
            .to_string()
    })?;
    let dictionary = load_dictionary(dictionary_dir.to_string_lossy().as_ref()).map_err(|e| {
        format!(
            "could not load the dictionary at {}: {e}",
            dictionary_dir.display()
        )
    })?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

    let mut by_band: BTreeMap<u8, Vec<Phrase>> = BTreeMap::new();
    for (id, text, author, translation) in analysable {
        let tokens = match analyse(&segmenter, &text, &words) {
            Ok(tokens) => tokens,
            Err(err) => {
                eprintln!("prepare-phrases: {id}: {err}");
                funnel.analyser_failed += 1;
                funnel.drop_for("the analyser refused it");
                continue;
            }
        };
        if tokens.is_empty() {
            funnel.empty += 1;
            funnel.drop_for("the analyser found no words in it");
            continue;
        }
        if tokens.iter().any(|token| token.has_kanji() && token.rt.is_none()) {
            funnel.unreadable += 1;
            funnel.drop_for("a kanji with no reading");
            continue;
        }
        if tokens
            .iter()
            .any(|token| token.has_kanji() && token.word.is_none())
        {
            funnel.word_unknown += 1;
            funnel.drop_for("a kanji-bearing word the course does not teach");
            continue;
        }
        funnel.segmented += 1;

        // The band is left at 1 here and set by the same method the artifact test
        // recomputes it with, so the two cannot drift: one rule, one call site.
        let phrase = Phrase {
            id,
            text,
            english: translation,
            author,
            licence: if cc0.contains(&id) { CC0 } else { CC_BY }.to_string(),
            band: 1,
            tokens: tokens
                .into_iter()
                .map(|token| PhraseToken {
                    surface: token.surface,
                    rt: token.rt,
                    word: token.word,
                })
                .collect(),
        };
        let band = phrase.band_from(&words);
        by_band.entry(band).or_default().push(Phrase { band, ..phrase });
    }

    // ---- The cap: the shortest sentences of each band -------------------------
    let mut kept: Vec<Phrase> = Vec::new();
    for (_, mut phrases) in by_band {
        phrases.sort_by(|a, b| {
            a.text
                .chars()
                .count()
                .cmp(&b.text.chars().count())
                .then(a.id.cmp(&b.id))
        });
        if phrases.len() > options.per_band {
            funnel.capped += phrases.len() - options.per_band;
            phrases.truncate(options.per_band);
        }
        kept.extend(phrases);
    }

    let dataset = PhraseDataset::from_phrases(kept, source(&options));
    write_artifact(&options.out, &dataset)?;
    write_attribution(&options.attribution, &dataset)?;

    report(&options, &dataset, &funnel);
    Ok(())
}

/// The rule this run cut the corpus by, as one sentence, written into the artifact.
fn selection_rule(options: &Options) -> String {
    format!(
        "one sentence of {}–{} characters ending in a single full stop, question or \
         exclamation mark, with no Latin letters, brackets, quotation marks or whitespace; an \
         English translation in the corpus; a named contributor, because CC BY 2.0 FR requires \
         the author to be named; not tagged `not a sentence` or `@possible copyright \
         infringement`; and every kanji-bearing word a word the course teaches. The {} shortest \
         of each band are kept.",
        options.min_length, options.max_length, options.per_band
    )
}

fn source(options: &Options) -> PhrasesSource {
    let modified = export_date(&options.tatoeba.join("PROVENANCE.txt"));
    PhrasesSource {
        analyser: format!("lindera {LINDERA_VERSION}"),
        dictionary: UNIDIC_VERSION.to_string(),
        corpus: "Tatoeba".to_string(),
        url: "https://tatoeba.org".to_string(),
        export: format!(
            "Tatoeba per-language exports: jpn_sentences_detailed.tsv, jpn-eng_links.tsv, \
             eng_sentences.tsv{}",
            match modified.as_deref() {
                Some(date) => format!(" (last modified {date})"),
                None => String::new(),
            }
        ),
        licences: format!("{CC_BY}, {CC0}"),
        selection: selection_rule(options),
    }
}

/// The export's own date, from the sidecar `scripts/fetch-tatoeba.sh` writes.
fn export_date(provenance: &Path) -> Option<String> {
    let text = std::fs::read_to_string(provenance).ok()?;
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut columns = line.split('\t');
        let (Some(url), Some(modified)) = (columns.next(), columns.next()) else {
            continue;
        };
        if url.contains("jpn_sentences_detailed") && !modified.is_empty() && modified != "unknown" {
            return Some(modified.to_string());
        }
    }
    None
}

/// Whether a sentence is the shape of a phrase, or why it is not.
///
/// Every rejection is a reason a reader can act on, because the funnel prints them
/// and a rule nobody can see is a rule nobody can argue with.
fn shape_of(text: &str, min_length: usize, max_length: usize) -> Result<(), &'static str> {
    let length = text.chars().count();
    if length < min_length {
        return Err("shorter than the minimum");
    }
    if length > max_length {
        return Err("longer than the maximum");
    }
    if text.chars().any(char::is_whitespace) {
        return Err("contains whitespace");
    }
    if text.chars().any(|ch| ch.is_ascii_alphabetic()) {
        return Err("contains Latin letters");
    }
    // Quotation marks, brackets, colons and the middle dot all need context a single
    // imported sentence does not carry — a phrase list of half-conversations reads as
    // a broken list. The Japanese brackets and quotes are here for the second
    // reason rather than the first: a sentence inside 「」 is quoted speech, and a
    // quoted line with no speaker reads as something missing. They would otherwise
    // be dropped by the terminal-punctuation rule below and be *reported* as
    // malformed, which is a different and less true complaint.
    const PUNCTUATION: [char; 36] = [
        '"', '\'', '`', '(', ')', '[', ']', '{', '}', '<', '>', ':', ';', '/', '\\', '|', '~', '@',
        '#', '$', '%', '^', '&', '*', '\u{300C}', '\u{300D}', '\u{300E}', '\u{300F}', '\u{FF08}',
        '\u{FF09}', '\u{3008}', '\u{3009}', '\u{300A}', '\u{300B}', '\u{3010}', '\u{3011}',
    ];
    if text
        .chars()
        .any(|ch| PUNCTUATION.contains(&ch) || ch == '・')
    {
        return Err("contains brackets, quotes or a middle dot");
    }
    if text.chars().any(|ch| {
        ('\u{FF21}'..='\u{FF3A}').contains(&ch) || ('\u{FF41}'..='\u{FF5A}').contains(&ch)
    }) {
        return Err("contains full-width Latin letters");
    }
    if text
        .chars()
        .any(|ch| ('\u{FF61}'..='\u{FF9F}').contains(&ch))
    {
        return Err("contains half-width katakana");
    }
    if text.chars().any(|ch| !ch.is_ascii() && (ch as u32) > 0xFFFF) {
        return Err("contains a character outside the basic plane");
    }

    // One sentence, and it ends where a sentence ends. `。！？` and their ASCII
    // spellings are all a Tatoeba sentence uses, and a question that ends in `?` is
    // still a phrase.
    const TERMINAL: [char; 5] = ['。', '！', '？', '!', '?'];
    let Some(last) = text.chars().next_back() else {
        return Err("is empty");
    };
    if !TERMINAL.contains(&last) {
        return Err("does not end with a full stop, question or exclamation mark");
    }
    if text[..text.len() - last.len_utf8()]
        .chars()
        .any(|ch| TERMINAL.contains(&ch))
    {
        return Err("holds more than one sentence");
    }
    Ok(())
}

/// The lines of a plain-text export, buffered.
fn lines(path: &Path) -> Result<std::io::Lines<BufReader<File>>, Box<dyn std::error::Error>> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(BufReader::with_capacity(1 << 20, file).lines())
}

/// The ids in a `id<TAB>lang<TAB>text` file, for the CC0 list.
fn read_ids(path: &Path) -> Result<HashSet<u32>, Box<dyn std::error::Error>> {
    let mut ids = HashSet::new();
    for line in lines(path)? {
        let line = line?;
        if let Some(first) = line.split('\t').next() {
            if let Ok(id) = first.parse::<u32>() {
                ids.insert(id);
            }
        }
    }
    Ok(ids)
}

/// The ids in a `id<TAB>tag` file that carry one of `tags`.
fn tagged_with(
    path: &Path,
    tags: &[&str],
) -> Result<HashSet<u32>, Box<dyn std::error::Error>> {
    let mut ids = HashSet::new();
    for line in lines(path)? {
        let line = line?;
        let columns: Vec<&str> = line.split('\t').collect();
        let (Some(id), Some(tag)) = (columns.first(), columns.get(1)) else {
            continue;
        };
        if tags.contains(tag) {
            if let Ok(id) = id.parse::<u32>() {
                ids.insert(id);
            }
        }
    }
    Ok(ids)
}

/// Where the crate lives, so the dictionary can be found relative to it.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/crates/nihongo-core")
        .to_path_buf()
}

/// Find the built dictionary under `<root>/.lindera/`, the same way
/// `prepare-passages` does.
fn find_dictionary(root: &Path) -> Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    let cache = root.join(".lindera");
    let Ok(entries) = std::fs::read_dir(cache) else {
        return Ok(None);
    };
    for entry in entries.flatten() {
        let candidate = entry.path().join("lindera-unidic");
        if candidate.join("metadata.json").is_file() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn write_artifact(path: &Path, dataset: &PhraseDataset) -> Result<(), Box<dyn std::error::Error>> {
    let payload = postcard::to_allocvec(&PhrasesArtifact::new(
        dataset.phrases().to_vec(),
        dataset.source().clone(),
    ))?;
    let mut raw = Vec::with_capacity(payload.len() + PHRASES_ARTIFACT_MAGIC.len());
    raw.extend_from_slice(PHRASES_ARTIFACT_MAGIC);
    raw.extend_from_slice(&payload);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut encoder = flate2::write::GzEncoder::new(
        BufWriter::new(File::create(path)?),
        flate2::Compression::best(),
    );
    encoder.write_all(&raw)?;
    let mut writer = encoder.finish()?;
    writer.flush()?;
    Ok(())
}

/// Write the attribution the corpus's licence requires, beside the app that ships
/// it.
///
/// One line per contributor, with the Tatoeba sentence ids taken from them: that is
/// the "name the author" half of CC BY 2.0 FR, at a size a reader can actually read.
/// The ids are in the artifact too, so every phrase can be traced back to its
/// sentence page.
fn write_attribution(
    path: &Path,
    dataset: &PhraseDataset,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut by_author: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
    for phrase in dataset.phrases() {
        by_author
            .entry(phrase.author.as_str())
            .or_default()
            .push(phrase.id);
    }
    for ids in by_author.values_mut() {
        ids.sort_unstable();
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(File::create(path)?);

    writeln!(out, "Sentences from Tatoeba")?;
    writeln!(out, "=====================")?;
    writeln!(out)?;
    writeln!(out, "Source            https://tatoeba.org")?;
    writeln!(out, "Export            {}", dataset.source().export)?;
    writeln!(
        out,
        "Licence           {CC_BY} — https://creativecommons.org/licenses/by/2.0/fr/"
    )?;
    writeln!(
        out,
        "Also included     {CC0} — https://creativecommons.org/publicdomain/zero/1.0/ for the \
         sentences the corpus lists as CC0"
    )?;
    writeln!(
        out,
        "Segmented with    {} ({}) at build time",
        dataset.source().analyser,
        dataset.source().dictionary
    )?;
    writeln!(out, "Selection         {}", dataset.source().selection)?;
    writeln!(
        out,
        "Shipped           {} Japanese sentences, from {} contributors",
        dataset.len(),
        by_author.len()
    )?;
    writeln!(out)?;
    writeln!(out, "Why this file exists")?;
    writeln!(out, "---------------------")?;
    writeln!(out)?;
    writeln!(
        out,
        "Tatoeba's sentences are contributed under CC BY 2.0 FR, which requires the author of\n\
         each one to be named. Naming them one per line inside the app would be a wall of names\n\
         longer than the phrases themselves, so they are named here: one line per contributor,\n\
         with the Tatoeba sentence ids taken from them. Every phrase in the app also carries its\n\
         own id, so a sentence can be looked up at https://tatoeba.org/en/sentences/show/<id>."
    )?;
    writeln!(out)?;
    writeln!(
        out,
        "The text of each sentence is unchanged. This app adds what the corpus does not have —\n\
         a word segmentation, a reading over every kanji, a level on this course's own ladder\n\
         and the English translation the corpus pairs with it — and leaves out the sentences\n\
         that use a word the course does not teach."
    )?;
    writeln!(out)?;
    writeln!(out, "Attribution")?;
    writeln!(out, "-----------")?;
    writeln!(out)?;
    for (author, ids) in &by_author {
        let ids: Vec<String> = ids.iter().map(u32::to_string).collect();
        writeln!(out, "{author}\t{}", ids.join(" "))?;
    }
    out.flush()?;
    Ok(())
}

fn report(options: &Options, dataset: &PhraseDataset, funnel: &Funnel) {
    let bytes = std::fs::metadata(&options.out)
        .map(|m| m.len())
        .unwrap_or(0);
    let tokens: usize = dataset.phrases().iter().map(|p| p.tokens.len()).sum();
    let linked: usize = dataset
        .phrases()
        .iter()
        .flat_map(|p| p.tokens())
        .filter(|t| t.word.is_some())
        .count();
    let bands: Vec<String> = dataset
        .band_counts()
        .into_iter()
        .map(|(band, count)| format!("{band}:{count}"))
        .collect();

    println!("prepare-phrases: wrote {}", options.out.display());
    println!(
        "  analyser               {} with {}",
        dataset.source().analyser,
        dataset.source().dictionary
    );
    println!("  corpus                 {}", dataset.source().export);
    println!("  sentences read         {}", funnel.read);
    println!("  ...with a translation  {}", funnel.read - funnel.no_english);
    println!(
        "  ...and a named author  {}",
        funnel.read - funnel.no_english - funnel.no_author
    );
    println!(
        "  ...not tagged out      {}",
        funnel.read - funnel.no_english - funnel.no_author - funnel.excluded_tag
    );
    println!(
        "  ...of the right shape  {}",
        funnel.read
            - funnel.no_english
            - funnel.no_author
            - funnel.excluded_tag
            - funnel.shaped_out
            - funnel.banned_out
    );
    println!("  ...kanji all known     {}", funnel.kanji_covered);
    println!("  ...segmented           {}", funnel.segmented);
    println!(
        "  phrases shipped        {} ({} tokens, {linked} linked to a vocabulary word)",
        dataset.len(),
        tokens
    );
    println!("  per band               {}", bands.join(" "));
    if funnel.capped > 0 {
        println!("  dropped by the cap     {}", funnel.capped);
    }
    if !funnel.reasons.is_empty() {
        let mut reasons = funnel.reasons.clone();
        reasons.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        println!("  why sentences went:");
        for (reason, count) in reasons {
            println!("    {count:>8}  {reason}");
        }
    }
    println!(
        "  artifact               {:.0} KB compressed",
        bytes as f64 / 1e3
    );
    println!("  attribution            {}", options.attribution.display());
    if options.show > 0 {
        // For a human rebuilding the corpus: the numbers say the cut was reasonable,
        // and only a sample says whether the sentences are any good.
        println!();
        for (band, count) in dataset.band_counts() {
            if count == 0 {
                continue;
            }
            println!("  band {band} — first {} of {count}:", options.show);
            for phrase in dataset.of_band(band).take(options.show) {
                println!("    {}", phrase.text);
                println!("      {}", phrase.english);
            }
        }
    }
    if dataset.is_empty() {
        println!();
        println!("  No phrases survived. That is a legitimate outcome of the filter rather");
        println!("  than a failure of the run, but it means the artifact is empty and the");
        println!("  screen will have nothing to show: widen --min-length/--max-length, or");
        println!("  raise --per-band, before committing it.");
    }
}
