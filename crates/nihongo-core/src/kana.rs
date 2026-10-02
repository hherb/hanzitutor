//! The kana dataset: hiragana and katakana, with the stroke geometry needed to
//! animate them and to grade a handwritten attempt at them.
//!
//! # Where the geometry comes from, and the one thing that has to be fixed
//!
//! AnimCJK publishes `graphicsJaKana.txt` in Make Me a Hanzi's format — one JSON
//! object per character, carrying the SVG outline of every stroke **and** a
//! centre-line ("median") for it — but with one wrinkle that matters here. To
//! animate a kana whose stroke crosses itself (あ, ぬ, る …), AnimCJK splits that
//! stroke into several drawing segments, and the graphics file stores one entry
//! per *segment*. So `graphicsJaKana.txt` gives あ four strokes where three are
//! taught.
//!
//! Taking that at face value would mis-grade handwriting: the grader would look
//! for four reference strokes, mark a correct three-stroke あ as missing a
//! stroke, and score the order wrongly. [`segment_to_stroke`] reads the split
//! back out of the SVG element ids (where `z12354d3a` and `z12354d3b` are two
//! segments of taught stroke 3) and [`merge_strokes`] folds them back together.
//!
//! Two independent checks pin this down, and both are tests:
//!
//! * every one of the 177 kana's merged stroke count equals KanjiVG's stroke
//!   count for the same character — KanjiVG being a separate project whose
//!   per-stroke paths are the stroke-order reference this is measured against;
//! * the geometry survives the merge — the kept centre-line still spans the
//!   stroke, and the merged outlines still sit inside the design box.

use hanzi_core::Point;
use serde::{Deserialize, Serialize};

/// Magic bytes at the head of a kana artifact, checked before decoding.
///
/// `postcard` is not self-describing, so a stale or foreign artifact would
/// otherwise decode into nonsense. The trailing digits are the format version.
pub const ARTIFACT_MAGIC: &[u8; 8] = b"KANAD001";

/// Which kana syllabary a character belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Script {
    Hiragana,
    Katakana,
}

impl Script {
    pub const fn name(self) -> &'static str {
        match self {
            Script::Hiragana => "hiragana",
            Script::Katakana => "katakana",
        }
    }

    /// The script a code point belongs to, or `None` if it is neither.
    ///
    /// `ー` (U+30FC, the prolonged sound mark) counts as katakana: it lives in
    /// the katakana block, and although it is used after both scripts it is
    /// taught with katakana.
    pub const fn of(ch: char) -> Option<Self> {
        match ch as u32 {
            0x3041..=0x3096 => Some(Script::Hiragana),
            0x30A1..=0x30FA | 0x30FC => Some(Script::Katakana),
            _ => None,
        }
    }

    /// Parse the name the webview sends back, so the interface can talk about a
    /// script without a second enum to keep in step.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "hiragana" => Some(Script::Hiragana),
            "katakana" => Some(Script::Katakana),
            _ => None,
        }
    }

    /// The other script, for a screen that offers "show this in katakana".
    pub const fn other(self) -> Self {
        match self {
            Script::Hiragana => Script::Katakana,
            Script::Katakana => Script::Hiragana,
        }
    }
}

/// One kana, with everything needed both to draw it and to grade it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kana {
    pub ch: char,
    pub script: Script,
    /// How many strokes this kana is **taught** with. This is the count the
    /// grader must see, and it is not always the number of drawing segments in
    /// the upstream graphics file — see the module docs.
    pub stroke_count: u8,
    /// SVG path data in **font space**, one entry per taught stroke, in stroke
    /// order. Render within `scale(1, -1) translate(0, -900)` over a 1024×1024
    /// box, exactly as the Chinese outlines are rendered.
    pub outlines: Vec<String>,
    /// Stroke centre-lines in **display space**, in stroke order. Compared
    /// against a user's strokes by [`hanzi_core::grade`].
    pub medians: Vec<Vec<Point>>,
}

impl Kana {
    /// The reference to grade an attempt against.
    pub fn reference_medians(&self) -> &[Vec<Point>] {
        &self.medians
    }

    /// True when this kana can be practised: it has geometry to grade against.
    pub fn is_practisable(&self) -> bool {
        !self.medians.is_empty() && self.medians.len() == self.outlines.len()
    }
}

/// The decoded payload of the shipped kana artifact.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Artifact {
    pub kana: Vec<Kana>,
}

impl Artifact {
    pub fn new(kana: Vec<Kana>) -> Self {
        Self { kana }
    }
}

/// An indexed collection of kana, ordered by code point.
///
/// Code-point order is the gojūon order for the base kana (あ い う え お,
/// か き く け こ …), which is the order they are taught in, so it is also the
/// order the collection is stored in.
#[derive(Clone, Debug, Default)]
pub struct KanaDataset {
    kana: Vec<Kana>,
}

impl KanaDataset {
    pub fn from_kana(mut kana: Vec<Kana>) -> Self {
        kana.sort_by_key(|k| k.ch as u32);
        Self { kana }
    }

    /// Drop any kana that cannot be practised, so the app can never offer
    /// something the board cannot grade.
    pub fn from_kana_teachable(kana: Vec<Kana>) -> Self {
        Self::from_kana(kana.into_iter().filter(Kana::is_practisable).collect())
    }

    /// Decode an artifact produced by `prepare-kana`: gzip-compressed, magic
    /// prefixed, then a `postcard`-encoded [`Artifact`].
    pub fn from_gzip_bytes(bytes: &[u8]) -> std::io::Result<Self> {
        use std::io::Read;

        let mut decoder = flate2::read::GzDecoder::new(bytes);
        let mut raw = Vec::new();
        decoder.read_to_end(&mut raw)?;

        if raw.len() < ARTIFACT_MAGIC.len() || &raw[..ARTIFACT_MAGIC.len()] != ARTIFACT_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "not a kana dataset artifact (bad or outdated magic); re-run `prepare-kana`",
            ));
        }
        let artifact: Artifact = postcard::from_bytes(&raw[ARTIFACT_MAGIC.len()..]).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("corrupt kana dataset artifact: {e}"),
            )
        })?;
        Ok(Self::from_kana(artifact.kana))
    }

    pub fn kana(&self) -> &[Kana] {
        &self.kana
    }

    pub fn len(&self) -> usize {
        self.kana.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kana.is_empty()
    }

    pub fn get(&self, ch: char) -> Option<&Kana> {
        self.kana.iter().find(|k| k.ch == ch)
    }

    /// Every kana of one script, in gojūon order.
    pub fn of_script(&self, script: Script) -> impl Iterator<Item = &Kana> {
        self.kana.iter().filter(move |k| k.script == script)
    }
}

/// Why a merge could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeError {
    /// The three arrays must describe the same set of drawing segments.
    LengthMismatch { strokes: usize, medians: usize, segments: usize },
    /// The graphics file had no outline for a character that had medians, or
    /// the other way round.
    Empty,
    /// Segment stroke indices must be non-decreasing and start at zero. A file
    /// where they are not is not grouped the way this code assumes, and
    /// guessing would mis-grade silently.
    NotContiguous { at: usize, found: u8, expected: u8 },
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::LengthMismatch { strokes, medians, segments } => write!(
                f,
                "stroke grouping describes {segments} segments but there are {strokes} outlines and {medians} medians"
            ),
            MergeError::Empty => write!(f, "no strokes to merge"),
            MergeError::NotContiguous { at, found, expected } => write!(
                f,
                "segment {at} belongs to stroke {found} but stroke {expected} was expected"
            ),
        }
    }
}

impl std::error::Error for MergeError {}

/// Read, out of an AnimCJK kana SVG, which taught stroke each drawing segment
/// belongs to. The returned indices are zero-based.
///
/// AnimCJK names an outline element `z<code point>d<stroke><segment>`, so
/// `z12354d1`, `z12354d2`, `z12354d3a`, `z12354d3b` is あ: two whole strokes
/// followed by one stroke drawn in two segments. That gives `[0, 1, 2, 2]` —
/// three taught strokes.
///
/// Only the `d` (outline) elements are counted; the `c` elements are the clip
/// paths the animation uses and describe the same segments again.
pub fn segment_to_stroke(svg: &str, cp: u32) -> Vec<u8> {
    let prefix = format!("id=\"z{cp}d");
    let mut out = Vec::new();
    let mut rest = svg;

    while let Some(pos) = rest.find(&prefix) {
        let after = &rest[pos + prefix.len()..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u32>() {
            if n >= 1 {
                out.push((n - 1).min(u8::MAX as u32) as u8);
            }
        }
        rest = &after[digits.len().max(1)..];
    }

    out
}

/// Fold AnimCJK's drawing segments back into the strokes a kana is taught with.
///
/// `segment_stroke[i]` is the taught stroke that segment `i` belongs to, as
/// returned by [`segment_to_stroke`]. Segments of one stroke are contiguous.
///
/// * **Outlines** are concatenated. The segments of a split stroke are genuine
///   complementary pieces of the shape — the animation clips each to its own
///   region — so joining the path data reconstructs the whole stroke. Two
///   subpaths in one `d` string is valid SVG.
/// * **Medians** keep the first segment's centre-line and discard the rest. The
///   later ones are displaced copies of the same path, moved out of the design
///   box so that their dash animation does not show; the first is the one in the
///   box that follows the stroke. Measured over all 25 affected kana, the first
///   centre-line spans 73–96% of its stroke's outline width and 92–98% of its
///   height, which is what a centre-line inside a round-capped outline should do.
pub fn merge_strokes(
    strokes: &[String],
    medians: &[Vec<[f32; 2]>],
    segment_stroke: &[u8],
) -> Result<(Vec<String>, Vec<Vec<Point>>), MergeError> {
    if strokes.is_empty() || medians.is_empty() {
        return Err(MergeError::Empty);
    }
    if strokes.len() != medians.len() || strokes.len() != segment_stroke.len() {
        return Err(MergeError::LengthMismatch {
            strokes: strokes.len(),
            medians: medians.len(),
            segments: segment_stroke.len(),
        });
    }
    // Stroke indices must run 0, 0, 1, 1, 1, 2 … — each stroke's segments
    // contiguous, no stroke skipped, nothing out of order. Anything else means
    // the grouping is not the shape this code assumes.
    let mut previous = 0u8;
    for (at, &found) in segment_stroke.iter().enumerate() {
        let expected = if at == 0 {
            0
        } else if found == previous {
            previous
        } else {
            previous + 1
        };
        if found != expected {
            return Err(MergeError::NotContiguous { at, found, expected });
        }
        previous = found;
    }

    let taught = segment_stroke[segment_stroke.len() - 1] as usize + 1;
    let mut outlines = Vec::with_capacity(taught);
    let mut merged = Vec::with_capacity(taught);

    let mut i = 0usize;
    while i < segment_stroke.len() {
        let stroke = segment_stroke[i];
        let start = i;
        while i < segment_stroke.len() && segment_stroke[i] == stroke {
            i += 1;
        }

        outlines.push(strokes[start..i].join(" "));
        merged.push(
            medians[start]
                .iter()
                .map(|[x, y]| Point::from_font(*x, *y))
                .collect(),
        );
    }

    Ok((outlines, merged))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_of_covers_the_kana_blocks_and_nothing_else() {
        assert_eq!(Script::of('あ'), Some(Script::Hiragana));
        assert_eq!(Script::of('ん'), Some(Script::Hiragana));
        assert_eq!(Script::of('ア'), Some(Script::Katakana));
        assert_eq!(Script::of('ー'), Some(Script::Katakana));
        assert_eq!(Script::of('一'), None);
        assert_eq!(Script::of('a'), None);
    }

    /// あ is drawn in four segments — `d1 d2 d3a d3b` — because its loop crosses
    /// itself. The whole point of this module is that it comes out as three.
    #[test]
    fn segments_are_read_out_of_the_svg_ids() {
        let svg = r##"<svg id="z12354"><path id="z12354d1" d="M0,0"/>
            <path id="z12354d2" d="M0,0"/>
            <path id="z12354d3a" d="M0,0"/>
            <path id="z12354d3b" d="M0,0"/>
            <clipPath id="z12354c1"><use href="#z12354d1"/></clipPath>
            <clipPath id="z12354c3a"><use href="#z12354d3a"/></clipPath></svg>"##;
        assert_eq!(segment_to_stroke(svg, 12354), vec![0, 1, 2, 2]);
    }

    /// The clip paths repeat the same segments, and the root element carries the
    /// bare code point. Neither may be mistaken for an outline.
    #[test]
    fn clip_paths_and_the_root_id_are_not_counted() {
        let svg = r#"<svg id="z12354"><clipPath id="z12354c1"/><clipPath id="z12354c2"/></svg>"#;
        assert_eq!(segment_to_stroke(svg, 12354), Vec::<u8>::new());
    }

    #[test]
    fn a_kana_with_no_split_passes_through_unchanged() {
        let strokes = vec!["M1,1".to_string(), "M2,2".to_string()];
        let medians = vec![
            vec![[0.0, 900.0]],
            vec![[100.0, 800.0]],
        ];
        let (out, med) =
            merge_strokes(&strokes, &medians, &[0, 1]).expect("contiguous grouping merges");
        assert_eq!(out, strokes);
        // font space is y-up: y = 900 is the top, so it becomes display y = 0.
        assert_eq!(med[0][0], Point::new(0.0, 0.0));
        assert_eq!(med[1][0], Point::new(100.0, 100.0));
    }

    #[test]
    fn a_split_stroke_joins_its_outlines_and_keeps_one_median() {
        let strokes = vec![
            "M1,1".to_string(),
            "M2,2".to_string(),
            "M3,3".to_string(),
        ];
        let medians = vec![
            vec![[0.0, 100.0]],
            vec![[10.0, 100.0]],
            vec![[-900.0, 100.0]], // displaced copy of the same segment
        ];
        let (out, med) =
            merge_strokes(&strokes, &medians, &[0, 1, 1]).expect("merges");

        assert_eq!(out.len(), 2, "three segments describe two taught strokes");
        assert_eq!(out[1], "M2,2 M3,3", "split outlines are concatenated");
        assert_eq!(med.len(), 2);
        assert_eq!(med[1][0].x, 10.0, "the displaced copy is discarded");
    }

    #[test]
    fn a_three_way_split_collapses_to_one_stroke() {
        let strokes = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let medians = vec![vec![[0.0, 0.0]], vec![[0.0, 0.0]], vec![[0.0, 0.0]]];
        let (out, med) = merge_strokes(&strokes, &medians, &[0, 0, 0]).expect("merges");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "a b c");
        assert_eq!(med.len(), 1);
    }

    #[test]
    fn mismatched_arrays_are_refused_rather_than_guessed_at() {
        let strokes = vec!["a".to_string()];
        let medians = vec![vec![[0.0, 0.0]]];
        assert_eq!(
            merge_strokes(&strokes, &medians, &[0, 1]),
            Err(MergeError::LengthMismatch { strokes: 1, medians: 1, segments: 2 })
        );
    }

    #[test]
    fn a_non_contiguous_grouping_is_refused() {
        let strokes = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let medians = vec![vec![[0.0, 0.0]]; 3];
        // Stroke 1 is interrupted and resumed: not how AnimCJK writes them.
        assert!(matches!(
            merge_strokes(&strokes, &medians, &[0, 1, 0]),
            Err(MergeError::NotContiguous { .. })
        ));
    }

    #[test]
    fn an_empty_entry_is_refused() {
        assert_eq!(merge_strokes(&[], &[], &[]), Err(MergeError::Empty));
    }

    fn kana(ch: char, strokes: usize) -> Kana {
        Kana {
            ch,
            script: Script::of(ch).expect("kana block"),
            stroke_count: strokes as u8,
            outlines: vec!["M0,0".to_string(); strokes],
            medians: vec![vec![Point::new(0.0, 0.0)]; strokes],
        }
    }

    #[test]
    fn the_dataset_orders_by_code_point_which_is_gojuon_order() {
        let dataset = KanaDataset::from_kana(vec![kana('か', 3), kana('あ', 3), kana('い', 2)]);
        let order: String = dataset.kana().iter().map(|k| k.ch).collect();
        assert_eq!(order, "あいか");
    }

    #[test]
    fn unpractisable_kana_are_dropped_rather_than_offered() {
        let mut broken = kana('あ', 3);
        broken.medians.clear();
        let dataset = KanaDataset::from_kana_teachable(vec![broken, kana('い', 2)]);
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.kana()[0].ch, 'い');
    }

    #[test]
    fn scripts_can_be_listed_separately() {
        let dataset = KanaDataset::from_kana(vec![kana('あ', 3), kana('ア', 2), kana('い', 2)]);
        assert_eq!(dataset.of_script(Script::Hiragana).count(), 2);
        assert_eq!(dataset.of_script(Script::Katakana).count(), 1);
    }

    #[test]
    fn an_artifact_with_the_wrong_magic_is_rejected() {
        use std::io::Write;
        let mut raw = b"WRONGDC1".to_vec();
        raw.extend_from_slice(&postcard::to_allocvec(&Artifact::new(vec![])).unwrap());
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let err = KanaDataset::from_gzip_bytes(&bytes).unwrap_err();
        assert!(err.to_string().contains("magic"), "{err}");
    }

    #[test]
    fn an_artifact_round_trips_through_gzip_and_postcard() {
        use std::io::Write;
        let mut raw = ARTIFACT_MAGIC.to_vec();
        raw.extend_from_slice(
            &postcard::to_allocvec(&Artifact::new(vec![kana('あ', 3)])).unwrap(),
        );
        let mut encoder =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&raw).unwrap();
        let bytes = encoder.finish().unwrap();

        let dataset = KanaDataset::from_gzip_bytes(&bytes).expect("decodes");
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.get('あ').expect("あ is present").stroke_count, 3);
    }
}
