//! Tone scoring: what the learner's pitch actually did, against the tone the
//! character asks for.
//!
//! ## Why this is not speech recognition
//!
//! A Mandarin tone is an F0 contour, and a contour is not in the recogniser's
//! output. A Chinese ASR model carries a strong language-model prior and is
//! built to be robust to exactly the errors a learner makes: say `shì` where
//! `sì` was wanted and a good recogniser still emits the character you were
//! aiming for, because that is what the context makes likely. There is no
//! non-neural substitute for recognising *what* was said, but there is for
//! judging *how* it was said — measure the pitch and compare its shape. That is
//! this module, and it needs no model, no download and no network, which is why
//! it is the half of the feature that can ship first.
//!
//! See `docs/research/ASR_TTS_CLAUDE_RESEARCH.md` §6 for the full argument, and
//! §6.3 for what this deliberately is not: there is no forced alignment and no
//! phone-level diagnosis here, so the interface must not imply either.
//!
//! ## What is compared: shape, not height
//!
//! The learner's F0 is converted to semitones relative to *their own* median for
//! the syllable, which is the only speaker reference available from a single
//! syllable, and then mean-centred. The four canonical tones are centred the
//! same way ([`tone_template`]), so the comparison is purely about shape: flat,
//! rising, dipping, falling.
//!
//! That choice has one consequence worth stating plainly rather than hiding.
//! Tone 1 (`55`, high level) and a tone 3 realised as a low level are *both*
//! flat, and one syllable cannot tell them apart without knowing the speaker's
//! register — which is why [`ToneConfig::flat_tone`] accepts a flat contour for
//! either. Tone 3 in careful speech dips, and a dip is evidence for tone 3 and
//! against tone 1. This is the honest position: it under-claims rather than
//! guessing, and it is the caveat §6.2 of the research raises.
//!
//! ## Everything here is pure
//!
//! Samples in, numbers out. No device, no file, no async. Capture lives in the
//! app (`src-tauri/src/capture.rs`); this crate stays free of platform
//! dependencies so the scoring can be tested against synthetic contours, which
//! is the only way to test it at all.

use serde::{Deserialize, Serialize};

use crate::grade::Grade;

/// The rate everything is analysed at.
///
/// Chosen to match what the speech models in the research (SenseVoice,
/// Zipformer) expect, so that a later ASR stage can reuse the same buffer rather
/// than resampling twice. Nothing in *this* module needs 16 kHz specifically.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Display span, in semitones, either side of centre.
///
/// Used to turn a centred contour into 0..1 for drawing. Both the learner's
/// contour and the expected shape go through the same mapping, so the two lines
/// in the interface are comparable by construction.
pub const DISPLAY_SPAN_ST: f32 = 5.0;

/// How forgiving the score is, in units of normalised shape distance.
///
/// A mean per-point error of this much scores `1/e`, about 37. Calibrated
/// against the synthetic contours in the tests below, where a tone matched to
/// its own template sits near 0.01, a plausible near-miss near 0.8, and a
/// clearly wrong tone past 1.5. Named because it is a judgement rather than a
/// measurement, and it should be re-tuned against real recordings rather than
/// buried in a formula.
const SCORE_DECAY_ST: f32 = 1.2;

/// What a level contour scores when a level tone was asked for.
///
/// Not 100, and deliberately: `1` and a flat `3` are indistinguishable from one
/// syllable, so the honest ceiling is "good" rather than "excellent".
const FLAT_MATCH_SCORE: f32 = 85.0;

/// What a level contour scores when the character asked for movement.
const FLAT_OFF_TARGET_SCORE: f32 = 30.0;

/// A contour flatter than this counts as "level", in semitones peak-to-peak.
///
/// Natural declination over a single syllable, and the wobble in a steady
/// phonation, are both well under this.
const FLAT_ST: f32 = 1.2;

/// How much closer one tone's template must be before the difference is called
/// real rather than noise, in the same normalised units as the score. Below
/// this the two shapes are not distinguishable and the verdict is `Uncertain`.
const DECIDE_MARGIN: f32 = 0.25;

/// How much periodic voice a segment must hold before a tone can be read from
/// it. Shorter than this and it was a click, not a syllable.
///
/// One judgement, written once, in milliseconds, and turned into a frame count by
/// [`min_voiced_frames`]. It used to be two constants — this and a
/// `MIN_VOICED_FRAMES = 6` beside it — and they drifted apart as the hop changed:
/// six frames is 25 ms at this hop, so the frame guard was several times weaker
/// than the duration guard it was supposed to stand beside, and neither was
/// derived from the hop it is measured with.
///
/// **30 ms, and the road here — 90 → 60 → 50 → 30 — is the point.**
///
/// A fourth tone is the shortest tone Mandarin has, and 是 is a long `sh` followed
/// by a short vowel. What this is compared against is not the vowel's length,
/// though: a frame counts only when its whole analysis window is periodic (see
/// [`track_pitch`]), so the `voiced_ms` measured from a syllable runs short of the
/// sound by the window's edges.
///
/// The first two steps were argued from synthetic contours, and the second was
/// still wrong. What settled it was a real voice: five rounds of a natural-rate 是
/// through `records_from_the_real_microphone`, after the window was fixed, measured
/// **41, 47, 69, 78 ms — and one outlier at 6 ms.** A gate at 50 ms sat in the
/// middle of that spread: it refused two perfectly good attempts, one of them by a
/// single frame, and the 6 ms outlier was going to be refused at any threshold
/// worth having.
///
/// **That is the rule this should have been set by from the beginning: a gate
/// placed inside the distribution of correct attempts will refuse correct
/// attempts, however carefully the number is argued for.** It belongs *below* the
/// distribution, where its only job is rejecting what is not speech — a click, a
/// lip smack, a chair creak. 30 ms is ten frames at this hop, and those frames
/// overlap: their windows cover about 53 ms of signal between them, against the
/// one or two frames a transient manages.
///
/// **Confirmed on the next run**: seven more rounds of a natural-rate 是 scored
/// seven times, margins +17 to +67 ms, nothing refused. The distribution has since
/// been 15–31 frames, so 30 ms leaves the whole of it clear — and a gate at the
/// old 50 ms would have refused two of those seven.
///
/// Public because it is the number to hold a real voice against: the
/// `records_from_the_real_microphone` test in `src-tauri` prints the voicing it
/// measured beside this floor.
pub const MIN_VOICED_MS: u32 = 30;

/// A floor under [`min_voiced_frames`], in frames.
///
/// A hop fine enough to divide the floor down to one frame would let a single
/// voiced frame pass as a syllable, so the derived count never goes below this
/// however the track was built.
const MIN_VOICED_FLOOR_FRAMES: usize = 4;

/// [`MIN_VOICED_MS`] as a number of frames, for the hop this track was built at.
///
/// The one place the duration floor becomes a frame count, so that
/// [`contour_in`] and the syllable splitter ask the same question of the same
/// track. The splitter needs it as much as the scorer does: a boundary placed so
/// that it cuts out a segment the scorer then refuses is a syllable the learner
/// said and the app threw away, which reads as a bug rather than as a property of
/// their speech.
pub fn min_voiced_frames(track: &PitchTrack) -> usize {
    let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;
    let by_duration = (MIN_VOICED_MS as f32 / hop_ms).ceil() as usize;
    MIN_VOICED_FLOOR_FRAMES.max(by_duration)
}

/// Points the contour is resampled to before comparison.
///
/// Fixed, so that a slow syllable and a fast one compare as the same shape —
/// which is also what makes a per-point score meaningful.
const CONTOUR_POINTS: usize = 12;

// ---------------------------------------------------------------------------
// Pitch tracking
// ---------------------------------------------------------------------------

/// One analysis frame of the pitch track.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchFrame {
    /// Estimated fundamental, or `0.0` when the frame is unvoiced.
    pub hz: f32,
    /// False for silence and for anything the estimator could not find a period
    /// in — a fricative, a click, breath noise.
    pub voiced: bool,
    /// Root-mean-square level, used to gate on loudness as well as periodicity.
    pub rms: f32,
}

/// Knobs for [`track_pitch`]. The frame size and hop are derived from the rate
/// and these bounds rather than configured, so they cannot contradict them.
#[derive(Clone, Copy, Debug)]
pub struct PitchConfig {
    /// Lowest fundamental to look for.
    ///
    /// **This is also what sets the analysis window**, because YIN needs two
    /// periods at the longest lag it searches: `window = 2 · sr / f_min`. So it is
    /// not only "how low a voice can be tracked", it is "how much pitch movement
    /// can be seen inside one frame" — and 60 Hz bought the first at the cost of
    /// the second.
    ///
    /// At 60 Hz the window is 33 ms, and a **fourth tone said at a normal rate does
    /// not hold a period across 33 ms**: it falls far enough that the difference
    /// function never dips under YIN's threshold, the frames come back unvoiced,
    /// and the syllable is refused for having no voice in it. Measured on a
    /// synthetic 是 with an 80 ms vowel, 60 Hz found **11 voiced frames and scored
    /// nothing**; at 80 Hz, where the window is 25 ms, the same signal gave
    /// **22 frames and 69 ms of voicing** — enough to judge. A tone 4 is the
    /// shortest and fastest-moving tone Mandarin has, so it is the case that
    /// suffered.
    ///
    /// 80 Hz rather than lower or higher, from the same measurements: a 90 Hz male
    /// voice still tracks correctly at 80 (84 ms of voicing on a 100 ms vowel), and
    /// **stops tracking at 100** (0 ms) — the search range has to stay under the
    /// lowest voice this will meet, and 90 Hz is about as low as speech goes.
    pub f_min: f32,
    /// Highest fundamental to look for. Above a child's or a high female voice,
    /// and above the range any Mandarin tone uses.
    pub f_max: f32,
    /// YIN's absolute threshold on the cumulative mean normalised difference.
    /// Lower is stricter; 0.15 is the value the original paper suggests.
    pub threshold: f32,
    /// Frames quieter than this are unvoiced without running the estimator.
    pub silence_rms: f32,
}

impl Default for PitchConfig {
    fn default() -> Self {
        Self {
            f_min: 80.0,
            f_max: 500.0,
            threshold: 0.15,
            silence_rms: 0.008,
        }
    }
}

/// A pitch track: the frames, and the timing needed to interpret them.
#[derive(Clone, Debug)]
pub struct PitchTrack {
    pub frames: Vec<PitchFrame>,
    /// Samples between frame starts.
    pub hop: usize,
    pub sample_rate: u32,
}

impl PitchTrack {
    /// Sample index each frame starts at.
    pub fn frame_start(&self, index: usize) -> usize {
        index * self.hop
    }

    /// Milliseconds from the start of the recording to a frame.
    pub fn frame_ms(&self, index: usize) -> f32 {
        (self.frame_start(index) as f32) * 1000.0 / (self.sample_rate as f32)
    }
}

/// Linear resampling, for bringing device capture to [`TARGET_SAMPLE_RATE`].
///
/// Deliberately the simplest thing that works, because the only consumer is an
/// F0 estimator that cares about periodicity below 500 Hz and nothing else. It
/// is *not* a band-limited converter and must not be used for audio anyone will
/// listen to: a later ASR stage needs a real resampler, and the research notes
/// `rubato` for it. Aliasing here folds high-frequency hiss down into the pass
/// band, which is a cosmetic problem for pitch and would be an audible one for
/// speech.
pub fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz || input.is_empty() {
        return input.to_vec();
    }
    let ratio = to_hz as f64 / from_hz as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let frac = (src - i0 as f64) as f32;
        let a = input.get(i0).copied().unwrap_or(0.0);
        let b = input.get(i0 + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

/// Estimate F0 frame by frame with YIN.
///
/// YIN rather than autocorrelation because autocorrelation's octave errors are
/// exactly the failure that ruins a tone score, and YIN's cumulative mean
/// normalised difference is what removes them. The implementation follows the
/// original paper (de Cheveigné & Kawahara, 2002): difference function,
/// cumulative mean normalisation, absolute threshold, then parabolic
/// interpolation for a sub-sample period.
///
/// Written here rather than pulled in as a crate on purpose. The algorithm is
/// about eighty lines, it is the one piece of signal processing this project
/// needs, and a dependency that is small, self-contained and unmaintained (the
/// `pitch-detection` crate the research suggests) is still a dependency to
/// vendor, licence and keep working. Owning it also makes it testable against
/// signals whose true F0 is known by construction, which is what the tests below
/// do.
pub fn track_pitch(samples: &[f32], sample_rate: u32, cfg: &PitchConfig) -> PitchTrack {
    let sr = sample_rate as f32;
    let tau_min = (sr / cfg.f_max).floor().max(2.0) as usize;
    // Window is two periods of the lowest frequency we look for: the shortest
    // window in which a period at `f_min` is still visible at the largest lag.
    let tau_max = (sr / cfg.f_min).ceil() as usize;
    let window = (tau_max * 2).max(tau_min * 4);
    // Eight windows per window, not four.
    //
    // The hop is what sets a contour's resolution in *time*, and a short syllable
    // is the case that needs it: at four, the whole of a conversational 是 is
    // about a dozen frames — barely more points than `CONTOUR_POINTS` — so
    // `normalise_shape` was interpolating more of the contour than it measured.
    // Overlapping the windows twice as far costs a factor of two in the YIN inner
    // loop, and a recording is capped at ten seconds, so the price is paid in
    // milliseconds on a phone.
    let hop = (window / 8).max(1);

    let mut frames = Vec::new();
    if samples.len() < window {
        return PitchTrack {
            frames,
            hop,
            sample_rate,
        };
    }

    let mut start = 0;
    while start + window <= samples.len() {
        frames.push(analyse_frame(
            &samples[start..start + window],
            sample_rate,
            tau_min,
            tau_max,
            cfg,
        ));
        start += hop;
    }

    PitchTrack {
        frames,
        hop,
        sample_rate,
    }
}

fn analyse_frame(
    frame: &[f32],
    sample_rate: u32,
    tau_min: usize,
    tau_max: usize,
    cfg: &PitchConfig,
) -> PitchFrame {
    let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
    if rms < cfg.silence_rms {
        return PitchFrame {
            hz: 0.0,
            voiced: false,
            rms,
        };
    }

    // The difference function, and its cumulative mean normalisation, in one
    // pass: `running` is the sum of d(1..=tau) that the normaliser divides by.
    let max_tau = tau_max.min(frame.len() - 2);
    let mut diff = vec![0.0f32; max_tau + 1];
    for tau in 1..=max_tau {
        let mut sum = 0.0;
        for j in 0..frame.len() - tau {
            let d = frame[j] - frame[j + tau];
            sum += d * d;
        }
        diff[tau] = sum;
    }

    let mut running = 0.0;
    let mut normalised = vec![1.0f32; max_tau + 1];
    for tau in 1..=max_tau {
        running += diff[tau];
        normalised[tau] = if running > 0.0 {
            diff[tau] * tau as f32 / running
        } else {
            1.0
        };
    }

    // First lag that dips under the threshold, then walk to the bottom of that
    // dip: the first minimum is the fundamental, a later one is an octave down.
    let mut tau = tau_min.max(1);
    while tau <= max_tau {
        if normalised[tau] < cfg.threshold {
            while tau < max_tau && normalised[tau + 1] < normalised[tau] {
                tau += 1;
            }
            let refined = parabolic_min(&normalised, tau);
            let period = if refined > 0.0 {
                refined
            } else {
                tau as f32
            };
            return PitchFrame {
                hz: sample_rate as f32 / period,
                voiced: true,
                rms,
            };
        }
        tau += 1;
    }

    PitchFrame {
        hz: 0.0,
        voiced: false,
        rms,
    }
}

/// Sub-sample position of the minimum, from the three samples around it.
fn parabolic_min(values: &[f32], tau: usize) -> f32 {
    if tau == 0 || tau + 1 >= values.len() {
        return tau as f32;
    }
    let (a, b, c) = (values[tau - 1], values[tau], values[tau + 1]);
    let denom = 2.0 * (2.0 * b - c - a);
    if denom.abs() < f32::EPSILON {
        return tau as f32;
    }
    let offset = (c - a) / denom;
    if offset.abs() > 1.0 {
        return tau as f32;
    }
    tau as f32 + offset
}

// ---------------------------------------------------------------------------
// Contours
// ---------------------------------------------------------------------------

/// A learner's syllable, reduced to the thing that is actually compared: a
/// shape in semitones, centred on its own mean.
#[derive(Clone, Debug)]
pub struct Contour {
    /// Semitone offsets from the syllable's own mean, `CONTOUR_POINTS` long.
    pub shape: Vec<f32>,
    /// Peak-to-peak span of the shape, in semitones. Small means level.
    pub range_st: f32,
    /// Median F0 of the voiced frames, in Hz. Reported, not scored.
    pub median_hz: f32,
    /// Milliseconds of voiced speech found.
    pub voiced_ms: u32,
    /// Milliseconds from the first voiced frame to the last.
    pub span_ms: u32,
}

impl Contour {
    /// True when the shape is level rather than rising, dipping or falling.
    pub fn is_flat(&self) -> bool {
        self.range_st < FLAT_ST
    }

    /// The shape mapped to 0..1 for drawing, with the same mapping the
    /// templates use so the two are comparable.
    pub fn display(&self) -> Vec<f32> {
        display(&self.shape)
    }
}

/// Map a centred semitone shape to 0..=1 for drawing.
pub fn display(shape: &[f32]) -> Vec<f32> {
    shape
        .iter()
        .map(|s| (0.5 + s / (2.0 * DISPLAY_SPAN_ST)).clamp(0.0, 1.0))
        .collect()
}

/// The first and last frame that carry voice at all.
///
/// Deliberately generous: this is the *outer* bound of an utterance, before the
/// per-syllable loudness gate, and it is what a word's segmentation divides up.
pub fn voiced_span(track: &PitchTrack) -> Option<(usize, usize)> {
    let first = track.frames.iter().position(|f| f.voiced && f.hz > 0.0)?;
    let last = track.frames.iter().rposition(|f| f.voiced && f.hz > 0.0)?;
    Some((first, last))
}

/// Reduce a pitch track to one contour, or `None` when there is not enough voice
/// in it to judge.
pub fn contour(track: &PitchTrack) -> Option<Contour> {
    let (first, last) = voiced_span(track)?;
    contour_in(track, first, last)
}

/// Reduce the frames in `first..=last` to a contour.
///
/// The loudness gate is measured against the peak **inside this range**, not
/// against the whole recording. That is what makes it work for a word: a syllable
/// spoken quietly, next to a loud one, is still judged against its own level
/// rather than being discarded as silence.
///
/// The gate is deliberately strict. A tutor that scores noise is worse than one
/// that admits it could not hear, because the learner has no way to tell the
/// difference between a bad score and a bad measurement.
pub fn contour_in(track: &PitchTrack, first: usize, last: usize) -> Option<Contour> {
    if track.frames.is_empty() || first > last || last >= track.frames.len() {
        return None;
    }

    let voiced: Vec<&PitchFrame> = track.frames[first..=last]
        .iter()
        .filter(|f| f.voiced && f.hz > 0.0)
        .collect();
    if voiced.is_empty() {
        return None;
    }

    // Loudness gate relative to this range's own peak, on top of the absolute
    // one: a quiet syllable is still scorable, a recording with a cough in it is
    // not.
    let peak = voiced.iter().map(|f| f.rms).fold(0.0f32, f32::max);
    let floor = peak * 0.08;

    let indices: Vec<usize> = (first..=last)
        .filter(|i| {
            let f = &track.frames[*i];
            f.voiced && f.hz > 0.0 && f.rms >= floor
        })
        .collect();

    let minimum = min_voiced_frames(track);
    if indices.len() < minimum {
        return None;
    }

    let first = *indices.first()?;
    let last = *indices.last()?;
    let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;
    let voiced_ms = (indices.len() as f32 * hop_ms).round() as u32;
    let span_ms = ((last - first + 1) as f32 * hop_ms).round() as u32;
    if voiced_ms < MIN_VOICED_MS {
        return None;
    }

    // Fill the span, interpolating the log of F0 across unvoiced gaps so a
    // glottal stop in the middle of a syllable does not read as a cliff. Log
    // domain because pitch is perceived multiplicatively.
    let mut log_f0 = Vec::with_capacity(last - first + 1);
    let mut gaps: Vec<Option<f32>> = Vec::with_capacity(last - first + 1);
    for i in first..=last {
        let f = &track.frames[i];
        gaps.push(if f.voiced && f.hz > 0.0 && f.rms >= floor {
            Some(f.hz.log2())
        } else {
            None
        });
    }
    // Drop the frames the estimator got wrong before anything is built from
    // them. Done before the gap fill, so a dropped frame becomes one of the gaps
    // that fill already knows how to handle.
    reject_outliers(&mut gaps);
    interpolate_gaps(&mut gaps);
    for v in gaps.iter().flatten() {
        log_f0.push(*v);
    }
    if log_f0.len() < minimum {
        return None;
    }

    let mut sorted = log_f0.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_log = sorted[sorted.len() / 2];
    let median_hz = median_log.exp2();

    // Semitones from this syllable's own median: the only speaker reference a
    // single syllable offers. Perceptually linear, so "two semitones up" means
    // the same thing for every voice.
    let semitones: Vec<f32> = log_f0.iter().map(|l| (*l - median_log) * 12.0).collect();
    let shape = normalise_shape(&semitones, CONTOUR_POINTS);

    let min = shape.iter().copied().fold(f32::INFINITY, f32::min);
    let max = shape.iter().copied().fold(f32::NEG_INFINITY, f32::max);

    Some(Contour {
        range_st: max - min,
        shape,
        median_hz,
        voiced_ms,
        span_ms,
    })
}

/// How far a frame may sit from its neighbours before it is read as a
/// mis-measurement rather than as pitch, in semitones.
///
/// Within one syllable the pitch moves smoothly. A fourth tone falls its whole
/// twelve semitones over 150–300 ms, which is well under **one** semitone between
/// frames twelve milliseconds apart, so a frame several semitones from its
/// neighbours is not something a voice did.
const OUTLIER_ST: f32 = 4.0;

/// Half-width, in frames, of the window the local median is taken over. Four
/// frames is about 12 ms, over which a real contour moves by a fraction of a
/// semitone.
const OUTLIER_WINDOW: usize = 4;

/// Drop frames that sit implausibly far from their neighbours.
///
/// **Why this is not cosmetic.** `range_st` is `max - min` over the contour, so a
/// single wrong frame sets it — and [`Contour::is_flat`], which decides whether a
/// tone is settled by the flatness rule or handed to the shape comparison, is
/// decided by `range_st`. A hand test on 中国人 measured segments of 166 ms
/// reporting **27.4 semitones**, more than two octaves, with the raw track jumping
/// between 83 and 441 Hz and 4 to 12 discontinuities per segment. One loud bad
/// frame turned a level tone 1 into "heard falling". A synthetic probe built from
/// that shape reproduces it: the segments with jumps are exactly the segments with
/// impossible ranges, and the ones with no jumps have ranges of 1 to 5 semitones.
///
/// The frame is dropped rather than replaced, so it becomes a gap and
/// [`interpolate_gaps`] fills it from its neighbours — which is the value it
/// should have had. A *consistent* octave error is deliberately left alone: the
/// contour has its mean removed before comparison, so a track that sits an octave
/// out throughout has the right shape, and only the jumps do damage.
fn reject_outliers(values: &mut [Option<f32>]) {
    let measured = values.to_vec();
    for (index, value) in measured.iter().enumerate() {
        let Some(value) = *value else { continue };
        let from = index.saturating_sub(OUTLIER_WINDOW);
        let to = (index + OUTLIER_WINDOW + 1).min(measured.len());
        let mut neighbours: Vec<f32> = measured[from..to].iter().flatten().copied().collect();
        if neighbours.len() < 3 {
            continue;
        }
        neighbours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = neighbours[neighbours.len() / 2];
        if (value - median).abs() * 12.0 > OUTLIER_ST {
            values[index] = None;
        }
    }
}

/// Fill `None`s by linear interpolation in whatever domain the values are in,
/// holding the edge values flat where a gap runs to an end.
fn interpolate_gaps(values: &mut [Option<f32>]) {
    let known: Vec<usize> = values
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_some())
        .map(|(i, _)| i)
        .collect();
    if known.len() < 2 {
        return;
    }
    for pair in known.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if b == a + 1 {
            continue;
        }
        let (va, vb) = (values[a].unwrap(), values[b].unwrap());
        let steps = (b - a) as f32;
        for (k, i) in (a + 1..b).enumerate() {
            let t = (k + 1) as f32 / steps;
            values[i] = Some(va + (vb - va) * t);
        }
    }
    let first = known[0];
    let last = *known.last().unwrap();
    let (vf, vl) = (values[first].unwrap(), values[last].unwrap());
    for v in values.iter_mut().take(first) {
        *v = Some(vf);
    }
    for v in values.iter_mut().skip(last + 1) {
        *v = Some(vl);
    }
}

/// Resample to a fixed number of points and remove the mean, so what remains is
/// shape alone.
fn normalise_shape(values: &[f32], points: usize) -> Vec<f32> {
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    let centred: Vec<f32> = values.iter().map(|v| v - mean).collect();
    let n = centred.len();
    if n == points {
        return centred;
    }
    (0..points)
        .map(|i| {
            let pos = if points == 1 {
                0.0
            } else {
                i as f32 * (n - 1) as f32 / (points - 1) as f32
            };
            let i0 = pos.floor() as usize;
            let frac = pos - i0 as f32;
            let a = centred[i0.min(n - 1)];
            let b = centred[(i0 + 1).min(n - 1)];
            a + (b - a) * frac
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The canonical tones
// ---------------------------------------------------------------------------

/// Name of a tone, for the interface to use in a sentence.
pub fn tone_name(tone: u8) -> &'static str {
    match tone {
        1 => "high level",
        2 => "rising",
        3 => "dipping",
        4 => "falling",
        5 => "neutral",
        _ => "unknown",
    }
}

/// What a neutral-tone judgement can and cannot see, said to the learner every
/// time one is scored.
///
/// This is a real limit rather than a hedge. Every contour has its mean removed
/// before comparison, because the speaker's register is not knowable, and the
/// score is about shape rather than duration. A neutral tone is short and takes
/// its pitch from the syllable before it — those are the two things a listener
/// hears, and neither is judged here. What *is* judged is that it was level, and
/// saying so is better than the alternative this replaced, which was to refuse to
/// score neutral tones at all.
pub const NEUTRAL_LIMIT: &str =
    "A neutral tone is short and takes its pitch from the syllable before it, so this \
     judges that it was level — not how high or how long it was.";

/// Whether a tone can be judged at all: the four full tones, and neutral.
///
/// Neutral was once refused here, on exactly the grounds [`NEUTRAL_LIMIT`]
/// describes. That is defensible and it was wrong in practice: it left 的 — the
/// most frequent character in the language — as the one character in the course
/// whose tone the app would not even look at.
pub fn is_scorable(tone: u8) -> bool {
    matches!(tone, 1..=5)
}

/// The canonical shape of a tone, as a centred semitone contour.
///
/// Built from the classical five-level scale — tone 1 `55`, tone 2 `35`, tone 3
/// `214`, tone 4 `51` — with one level taken as two semitones, which puts the
/// widest tone (4) at eight semitones and the whole scale at about a minor
/// sixth. The absolute levels are discarded: the mean is removed, because the
/// comparison is about shape (see the module note).
pub fn tone_template(tone: u8) -> Vec<f32> {
    let anchors: &[(f32, f32)] = match tone {
        1 => &[(0.0, 4.0), (1.0, 4.0)],
        2 => &[(0.0, 0.0), (1.0, 4.0)],
        3 => &[(0.0, -2.0), (0.5, -4.0), (1.0, 2.0)],
        4 => &[(0.0, 4.0), (1.0, -4.0)],
        _ => &[(0.0, 0.0), (1.0, 0.0)],
    };
    let raw: Vec<f32> = (0..CONTOUR_POINTS)
        .map(|i| {
            let t = i as f32 / (CONTOUR_POINTS - 1) as f32;
            sample_anchors(anchors, t)
        })
        .collect();
    let mean = raw.iter().sum::<f32>() / raw.len() as f32;
    raw.into_iter().map(|v| v - mean).collect()
}

fn sample_anchors(anchors: &[(f32, f32)], t: f32) -> f32 {
    if t <= anchors[0].0 {
        return anchors[0].1;
    }
    for pair in anchors.windows(2) {
        let (t0, v0) = pair[0];
        let (t1, v1) = pair[1];
        if t <= t1 {
            let span = t1 - t0;
            if span <= f32::EPSILON {
                return v1;
            }
            return v0 + (v1 - v0) * (t - t0) / span;
        }
    }
    anchors[anchors.len() - 1].1
}

/// Scale a shape to unit RMS, so that comparison is about the *direction and
/// shape* of the movement rather than how far the pitch moved.
///
/// This step is not cosmetic. Tone 2 (`35`) genuinely moves half as far as tone
/// 4 (`51`) — four semitones against eight — so with amplitudes left in, a
/// falling contour sits closer to a rising template than to a level one, and the
/// classifier will call a flat syllable "rising" when a fall was asked for.
/// Normalising removes the amplitude difference and leaves the direction, which
/// is the part a learner can act on. A shape with no movement at all is returned
/// unchanged: there is nothing to scale, and it is handled as a level tone
/// before any distance is consulted.
fn normalise_rms(shape: &[f32]) -> Vec<f32> {
    let rms = (shape.iter().map(|v| v * v).sum::<f32>() / shape.len() as f32).sqrt();
    if rms < 1e-3 {
        return shape.to_vec();
    }
    shape.iter().map(|v| v / rms).collect()
}

/// Mean per-point cost of the cheapest warping path between two contours.
///
/// Dynamic time warping rather than a straight difference because the position
/// of a tone-3 dip inside the syllable varies far more between speakers than the
/// depth of the dip does, and penalising that would be penalising the wrong
/// thing. Both inputs are already time-normalised to the same length, so the
/// warping is a small correction rather than a substitute for resampling.
///
/// Normalised by the longer length, which slightly over-states the average cost
/// of a very warped path. Consistent, which is what the score needs; it is tuned
/// as one number ([`SCORE_DECAY_ST`]) rather than interpreted absolutely.
///
/// Feed this *normalised* shapes ([`normalise_rms`]). On raw semitones the
/// warping is too free: it can slide a rise onto its own mirror image, which is
/// the one confusion that matters most here.
pub fn dtw_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return f32::INFINITY;
    }
    let mut prev = vec![f32::INFINITY; b.len() + 1];
    let mut cur = vec![f32::INFINITY; b.len() + 1];
    prev[0] = 0.0;
    for i in 1..=a.len() {
        cur[0] = f32::INFINITY;
        for j in 1..=b.len() {
            let cost = (a[i - 1] - b[j - 1]).abs();
            let best = prev[j].min(cur[j - 1]).min(prev[j - 1]);
            cur[j] = cost + best;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()] / a.len().max(b.len()) as f32
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// What the attempt is judged to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneVerdict {
    /// The expected tone is the one the contour looks most like.
    Match,
    /// Another tone's shape is clearly closer than the expected one.
    OffTarget,
    /// Not enough evidence to say, and it says so rather than guessing.
    Uncertain,
}

/// The result of scoring one spoken syllable.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneAttempt {
    /// The tone the character called for, 1..=4.
    pub expected_tone: u8,
    /// The tone the contour looks most like, when there was a contour at all.
    pub heard_tone: Option<u8>,
    /// 0..=100, from the shape distance to the expected template. Same scale and
    /// same bands as a handwriting score, so the interface can treat them alike.
    pub score: f32,
    pub grade: Grade,
    pub verdict: ToneVerdict,
    /// One plain sentence for the learner. The interface shows this rather than
    /// inventing its own wording, so there is one place the judgement is worded.
    pub detail: String,
    /// The learner's contour, 0..=1 for drawing, low pitch at 0.
    pub contour: Vec<f32>,
    /// The expected tone's canonical shape, on the same 0..=1 scale, so the two
    /// lines can be drawn over each other.
    pub reference: Vec<f32>,
    pub median_hz: f32,
    /// Peak-to-peak span of the contour, in semitones.
    pub range_semitones: f32,
    pub voiced_ms: u32,
    pub span_ms: u32,
}

impl ToneAttempt {
    /// An attempt that could not be judged, with the reason in `detail`.
    ///
    /// Score is 0 and the grade is `Poor`, which are never shown: the interface
    /// renders [`ToneVerdict::Uncertain`] as its own state. The fields exist so
    /// the shape of the message is the same either way, which keeps the
    /// TypeScript side honest.
    fn unheard(expected_tone: u8, detail: impl Into<String>) -> Self {
        Self {
            expected_tone,
            heard_tone: None,
            score: 0.0,
            grade: Grade::Poor,
            verdict: ToneVerdict::Uncertain,
            detail: detail.into(),
            contour: Vec::new(),
            reference: display(&tone_template(expected_tone)),
            median_hz: 0.0,
            range_semitones: 0.0,
            voiced_ms: 0,
            span_ms: 0,
        }
    }
}

/// The tone an attempt could not be judged against, and why.
///
/// Only not-a-tone (`0`) lands here now. Neutral used to, and no longer does:
/// it is judged on being level, which is the part of it that can be seen from
/// one syllable — see [`NEUTRAL_LIMIT`] and [`is_scorable`].
fn unsupported(expected_tone: u8) -> ToneAttempt {
    ToneAttempt::unheard(
        expected_tone,
        "this reading does not name a tone that can be judged",
    )
}

/// Score one contour against the tone it should have carried.
///
/// Split out from [`analyze`] so that a word can score each of its syllables with
/// exactly the same rules as a single character, which is the only way the two
/// can be trusted to agree.
fn score_contour(contour: &Contour, expected_tone: u8) -> ToneAttempt {
    // A level syllable is settled by its flatness rather than by a distance, for
    // two reasons that both matter. A level tone has no shape for a shape
    // comparison to work with; and normalising a near-flat contour to unit RMS
    // would amplify its own measurement noise into what looks like a large
    // movement, so a *correct* level tone would be scored badly for having been
    // measured imperfectly. Judged by the rule instead, and given a fixed honest
    // score: level is right for tone 1, for a tone 3 realised flat and for a
    // neutral tone, and it is plainly not a rising or a falling tone.
    let (verdict, score, heard_tone, mut detail) = if contour.is_flat() {
        if matches!(expected_tone, 1 | 3) {
            (
                ToneVerdict::Match,
                FLAT_MATCH_SCORE,
                Some(1),
                format!(
                    "A level tone — right for {}. Tone 3 is often flat like this in speech, so \
                     this counts whether you were aiming for 1 or 3.",
                    tone_name(expected_tone)
                ),
            )
        } else if expected_tone == 5 {
            (
                ToneVerdict::Match,
                FLAT_MATCH_SCORE,
                Some(5),
                format!("A level tone, which is what a neutral tone is. {NEUTRAL_LIMIT}"),
            )
        } else {
            (
                ToneVerdict::OffTarget,
                FLAT_OFF_TARGET_SCORE,
                Some(1),
                format!(
                    "That was level, and the character asks for tone {} ({}).",
                    expected_tone,
                    tone_name(expected_tone)
                ),
            )
        }
    } else {
        let observed = normalise_rms(&contour.shape);
        let distances: Vec<(u8, f32)> = (1..=4)
            .map(|t| (t, dtw_distance(&observed, &normalise_rms(&tone_template(t)))))
            .collect();
        let (heard, best) = distances
            .iter()
            .copied()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .expect("four tones");

        if expected_tone == 5 {
            // The pitch moved, and a neutral tone is level, so this is wrong
            // whatever it moved like. Nothing is measured against a template
            // here because a neutral tone cannot have one — its height comes
            // from the syllable before it — which is why this is answered by the
            // rule rather than by a distance, in the same band as a level
            // contour offered for a moving tone. The tone it *did* move like is
            // still named, because "that was a rising tone where a neutral one
            // was asked for" is something a learner can act on.
            (
                ToneVerdict::OffTarget,
                FLAT_OFF_TARGET_SCORE,
                Some(heard),
                format!(
                    "Heard {}, but this asks for a neutral tone. {NEUTRAL_LIMIT}",
                    tone_name(heard)
                ),
            )
        } else {
            let expected_distance = distances
                .iter()
                .find(|(t, _)| *t == expected_tone)
                .map(|(_, d)| *d)
                .expect("the expected tone is one of the four");
            let score = 100.0 * (-expected_distance / SCORE_DECAY_ST).exp();

            if heard == expected_tone {
                (
                    ToneVerdict::Match,
                    score,
                    Some(heard),
                    format!(
                        "That is {} (tone {}), which is what the character asks for.",
                        tone_name(expected_tone),
                        expected_tone
                    ),
                )
            } else if (expected_distance - best).abs() < DECIDE_MARGIN {
                (
                    ToneVerdict::Uncertain,
                    score,
                    Some(heard),
                    format!(
                        "Between tone {} ({}) and tone {} ({}). Say it again, a little longer.",
                        expected_tone,
                        tone_name(expected_tone),
                        heard,
                        tone_name(heard)
                    ),
                )
            } else {
                (
                    ToneVerdict::OffTarget,
                    score,
                    Some(heard),
                    format!(
                        "Heard {}, but the character asks for tone {} ({}).",
                        tone_name(heard),
                        expected_tone,
                        tone_name(expected_tone)
                    ),
                )
            }
        }
    };

    // The score is about direction, not distance travelled (see
    // `normalise_rms`), so a matching but barely-moving tone still scores well.
    // Say so rather than letting the number imply the movement was convincing.
    // Only for a *moving* tone: a flat one is not shallow, it is level, and the
    // wording above already covers it.
    if verdict == ToneVerdict::Match && !contour.is_flat() && contour.range_st < 2.0 {
        detail.push_str(
            " The movement was shallow — let the pitch travel further next time for a clearer tone.",
        );
    }

    ToneAttempt {
        expected_tone,
        heard_tone,
        score,
        grade: Grade::from_score(score),
        verdict,
        detail,
        contour: contour.display(),
        reference: display(&tone_template(expected_tone)),
        median_hz: contour.median_hz,
        range_semitones: contour.range_st,
        voiced_ms: contour.voiced_ms,
        span_ms: contour.span_ms,
    }
}

// ---------------------------------------------------------------------------
// Splitting one recording into syllables
// ---------------------------------------------------------------------------

/// Cost of putting a boundary on a frame that carries voice.
///
/// Larger than any plausible RMS, so a boundary is only ever placed inside voiced
/// speech when there is no unvoiced frame available at all. A consonant between
/// two syllables — the `x` of `xuéxí`, the `h` of `nǐhǎo` — is unvoiced and is
/// therefore the natural place to cut, which is exactly what a listener hears as
/// the syllable break.
const VOICED_CUT_PENALTY: f32 = 1.0;

/// Per-frame cost of cutting between two frames.
///
/// Unvoiced frames are free (they are the syllable breaks); voiced frames cost
/// [`VOICED_CUT_PENALTY`] plus their loudness, so among several unvoiced frames
/// the quietest wins and among voiced ones the quietest wins too.
fn cut_cost(track: &PitchTrack) -> Vec<f32> {
    track
        .frames
        .iter()
        .map(|f| if f.voiced { VOICED_CUT_PENALTY + f.rms } else { f.rms })
        .collect()
}

/// Choose where to divide `first..=last` into `count` syllables.
///
/// Returns the `count - 1` boundary frames, ascending, or `None` when the span
/// cannot hold that many syllables — see the two floors below.
///
/// `min_len` is [`min_voiced_frames`] at the call site, and so is `min_voiced`.
/// They are separate arguments because they say different things and only happen
/// to have the same value: **how long a segment must be, and how much voice it
/// must contain.**
///
/// A shortest-path search rather than picking the `count - 1` cheapest frames
/// outright, because those may all sit in one gap. Cost is the sum of the frame
/// costs the cuts land on.
///
/// ## The voice floor is the one that matters, and it was missing
///
/// A frame-length floor alone does not stop the search from **cutting twice in the
/// same silence**, and that is exactly what it did on a real voice. A long quiet
/// stretch is the cheapest place in a recording to put a boundary, so with two
/// boundaries to place the search put both of them inside it, `min_len` apart —
/// the minimum the rule allowed. The learner then got a middle "syllable" of
/// 31 ms holding no voice at all, reported as "I could not hear syllable 2
/// clearly", and a third segment holding two syllables' worth of pitch: a hand
/// test on 中国人 measured one segment at 750 ms with a 26-semitone range. Six
/// rounds of 是不是 out of eight and four of 中国人 out of seven did this.
///
/// **A syllable has a vowel in it.** Requiring each segment to hold `min_voiced`
/// frames of voice expresses that, and it makes cutting twice in a silence
/// impossible — the segment between the two cuts would have no voice in it. What
/// survives is a boundary at each *end* of the silence, which is what a listener
/// hears as the syllable break.
fn find_boundaries(
    track: &PitchTrack,
    cost: &[f32],
    first: usize,
    last: usize,
    count: usize,
    min_len: usize,
    min_voiced: usize,
) -> Option<Vec<usize>> {
    let cuts = count.checked_sub(1)?;
    if cuts == 0 {
        return Some(Vec::new());
    }
    if last + 1 < first + count * min_len {
        return None;
    }

    // `voiced_before[i]` is the number of voiced frames before frame `i`, so the
    // voice in a segment is one subtraction. The raw flag, not the loudness-gated
    // count `contour_in` ends up using: the gate is measured against a segment's
    // own peak, which is not known until the split is chosen.
    let mut voiced_before = vec![0usize; track.frames.len() + 1];
    for (index, frame) in track.frames.iter().enumerate() {
        voiced_before[index + 1] =
            voiced_before[index] + usize::from(frame.voiced && frame.hz > 0.0);
    }
    let voice_in = |from: usize, to: usize| voiced_before[to] - voiced_before[from];

    // Cheap necessary condition, so a hopeless span is refused before the search.
    if voice_in(first, last + 1) < count * min_voiced {
        return None;
    }

    // `best[j][i]` is the cheapest placement of `j` boundaries with the `j`-th at
    // frame `i`; row 0 is the virtual start at `first`, costing nothing. A
    // boundary numbered `j` (1-based) has at least `j` syllables behind it and
    // `count - j` ahead, which is where its window comes from.
    const INF: f32 = f32::INFINITY;
    let mut best = vec![vec![INF; cost.len()]; cuts + 1];
    let mut from = vec![vec![None; cost.len()]; cuts + 1];
    best[0][first] = 0.0;

    for j in 1..=cuts {
        let lo = first + j * min_len;
        let hi = last + 1 - (count - j) * min_len;
        if lo > hi {
            return None;
        }

        // `winner` is the running minimum of the previous row over every legal
        // predecessor. Both constraints cut the legal set down to a *prefix* of
        // the row — a frame too close is too close for every later one too, and a
        // frame with too little voice behind it is too early for every later one
        // too — so the set only grows as `i` grows and one pass is still enough.
        let mut winner = INF;
        let mut winner_at = None;
        let mut previous = first;
        for i in lo..=hi {
            let far_enough = i.saturating_sub(min_len);
            // The predecessor must leave a whole syllable — voice included —
            // between itself and this boundary.
            let enough_voice = voiced_before[i].saturating_sub(min_voiced);
            while previous <= far_enough && voiced_before[previous] <= enough_voice {
                if best[j - 1][previous] < winner {
                    winner = best[j - 1][previous];
                    winner_at = Some(previous);
                }
                previous += 1;
            }
            if let Some(at) = winner_at {
                best[j][i] = winner + cost[i];
                from[j][i] = Some(at);
            }
        }
    }

    // The last boundary also has to leave a whole syllable after it, voice
    // included. This is the constraint the naive "pick the cheapest frames"
    // approach cannot express.
    let lo = first + cuts * min_len;
    let hi = last + 1 - min_len;
    if lo > hi {
        return None;
    }
    let mut winner = INF;
    let mut at = None;
    for (i, value) in best[cuts].iter().enumerate().skip(lo).take(hi - lo + 1) {
        if voice_in(i, last + 1) < min_voiced {
            continue;
        }
        if *value < winner {
            winner = *value;
            at = Some(i);
        }
    }
    let mut at = at?;

    let mut boundaries = Vec::with_capacity(cuts);
    for j in (1..=cuts).rev() {
        boundaries.push(at);
        at = from[j][at]?;
    }
    boundaries.reverse();
    Some(boundaries)
}

// ---------------------------------------------------------------------------
// Scoring a whole utterance
// ---------------------------------------------------------------------------

/// One syllable's worth of a report, with what was asked for.
///
/// The engine works in tones only; the app pairs these with the characters and
/// readings from `crate::pinyin`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyllableReport {
    /// Which syllable this is, counting from 1, for the wording.
    pub position: usize,
    /// The tone asked for. `5` means nothing was scored for this syllable.
    pub expected_tone: u8,
    pub attempt: ToneAttempt,
}

/// The result of scoring one recording against a sequence of tones.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToneReport {
    /// One entry per syllable asked for, in order, always the same length as the
    /// tones that were requested.
    pub syllables: Vec<SyllableReport>,
    /// The tones that were scored against.
    pub expected: Vec<u8>,
    pub verdict: ToneVerdict,
    /// Mean of the scores of the syllables that could be scored. `0` when none
    /// could.
    pub score: f32,
    pub grade: Grade,
    /// One plain sentence for the learner, worded here.
    pub detail: String,
    /// Where the syllables were divided, in milliseconds **from the first voiced
    /// frame** — one entry per boundary, empty for a single syllable.
    ///
    /// The baseline is the start of speech rather than the start of the recording,
    /// because that is the only one that can be read against [`Self::voiced_ms`]:
    /// a learner holds the button before speaking, so an absolute offset is mostly
    /// the length of their own pause.
    pub boundaries_ms: Vec<u32>,
    pub voiced_ms: u32,
    pub span_ms: u32,
    pub median_hz: f32,
}

impl ToneReport {
    fn silent(expected: Vec<u8>, detail: impl Into<String>) -> Self {
        let syllables = expected
            .iter()
            .enumerate()
            .map(|(i, tone)| SyllableReport {
                position: i + 1,
                expected_tone: *tone,
                attempt: if (1..=4).contains(tone) {
                    ToneAttempt::unheard(
                        *tone,
                        "I could not hear enough voice to judge. Hold the button, then say it.",
                    )
                } else {
                    unsupported(*tone)
                },
            })
            .collect();
        Self {
            syllables,
            expected,
            verdict: ToneVerdict::Uncertain,
            score: 0.0,
            grade: Grade::Poor,
            detail: detail.into(),
            boundaries_ms: Vec::new(),
            voiced_ms: 0,
            span_ms: 0,
            median_hz: 0.0,
        }
    }
}

/// Score a whole utterance against the tones it should carry.
///
/// One tone per expected syllable, in order. A tone of `5` (neutral) is carried
/// through and reported but not scored — see [`unsupported`].
///
/// With one tone this is the single-character case and the whole voiced span is
/// one syllable. With several, the recording is divided first: the syllable
/// boundaries are found from the *unvoiced* frames, which is where a consonant
/// between two syllables sits. That is a heuristic, and the honest failure mode
/// is the one below — when the span is too short to hold the syllables asked for,
/// nothing is scored rather than something being scored wrongly.
pub fn analyze(samples: &[f32], sample_rate: u32, expected: &[u8]) -> ToneReport {
    if expected.is_empty() {
        return ToneReport::silent(Vec::new(), "There was no tone to listen for.");
    }

    let resampled = resample(samples, sample_rate, TARGET_SAMPLE_RATE);
    let track = track_pitch(&resampled, TARGET_SAMPLE_RATE, &PitchConfig::default());

    let Some((first, last)) = voiced_span(&track) else {
        return ToneReport::silent(
            expected.to_vec(),
            "I could not hear enough voice to judge. Hold the button, then say it.",
        );
    };

    let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;
    // For a word these two differ, and the difference is the point: the span
    // covers the consonants between syllables, while only the voiced frames are
    // pitch. Reporting one number for both would hide a word that was mostly
    // silence.
    let voiced_frames = track.frames[first..=last]
        .iter()
        .filter(|f| f.voiced && f.hz > 0.0)
        .count();
    let voiced_ms = (voiced_frames as f32 * hop_ms).round() as u32;
    let span_ms = ((last + 1 - first) as f32 * hop_ms).round() as u32;
    let median_hz = median_hz_of(&track, first, last).unwrap_or(0.0);

    // One syllable: the whole span, exactly as a single character was scored
    // before words existed.
    if expected.len() == 1 {
        let Some(contour) = contour_in(&track, first, last) else {
            return ToneReport::silent(
                expected.to_vec(),
                "I could not hear enough voice to judge. Hold the button, then say the syllable.",
            );
        };
        let attempt = if is_scorable(expected[0]) {
            score_contour(&contour, expected[0])
        } else {
            unsupported(expected[0])
        };
        return ToneReport {
            syllables: vec![SyllableReport {
                position: 1,
                expected_tone: expected[0],
                attempt,
            }],
            expected: expected.to_vec(),
            verdict: ToneVerdict::Uncertain,
            score: 0.0,
            grade: Grade::Poor,
            detail: String::new(),
            boundaries_ms: Vec::new(),
            voiced_ms: contour.voiced_ms,
            span_ms: contour.span_ms,
            median_hz: contour.median_hz,
        }
        .finish();
    }

    let cost = cut_cost(&track);
    // One floor in two forms: a segment has to be at least this long, and has to
    // hold at least this much voice in it. See `find_boundaries` for why the
    // second is not implied by the first.
    let syllable_floor = min_voiced_frames(&track);
    let Some(boundaries) = find_boundaries(
        &track,
        &cost,
        first,
        last,
        expected.len(),
        syllable_floor,
        syllable_floor,
    ) else {
        let mut report = ToneReport::silent(
            expected.to_vec(),
            format!(
                "I could not separate {} syllables in that. Say them one after another, a \
                 little more slowly.",
                expected.len()
            ),
        );
        report.voiced_ms = voiced_ms;
        report.span_ms = span_ms;
        report.median_hz = median_hz;
        return report;
    };

    // Turn the boundaries into frame ranges. The frame a boundary lands on starts
    // the next syllable: it is an unvoiced consonant frame, so the per-syllable
    // loudness gate drops it from whichever side it is on.
    let mut edges: Vec<(usize, usize)> = Vec::with_capacity(expected.len());
    let mut previous = first;
    for boundary in boundaries.iter() {
        edges.push((previous, boundary.saturating_sub(1).max(previous)));
        previous = *boundary;
    }
    edges.push((previous, last));

    let mut syllables = Vec::with_capacity(expected.len());

    for (index, (from, to)) in edges.iter().enumerate() {
        let expected_tone = expected[index];
        let attempt = if is_scorable(expected_tone) {
            match contour_in(&track, *from, *to) {
                Some(contour) => score_contour(&contour, expected_tone),
                None => ToneAttempt::unheard(
                    expected_tone,
                    format!(
                        "I could not hear syllable {} clearly. Say the whole word again.",
                        index + 1
                    ),
                ),
            }
        } else {
            unsupported(expected_tone)
        };
        syllables.push(SyllableReport {
            position: index + 1,
            expected_tone,
            attempt,
        });
    }

    ToneReport {
        syllables,
        expected: expected.to_vec(),
        verdict: ToneVerdict::Uncertain,
        score: 0.0,
        grade: Grade::Poor,
        detail: String::new(),
        boundaries_ms: boundaries
            .iter()
            // Relative to the first voiced frame, not to the start of the
            // recording. Absolute offsets include however long the learner held
            // the button before speaking, which turned "split 163 ms into the
            // word" into "split at 1463 ms" beside a 308 ms utterance — a working
            // split that read as a broken one.
            .map(|b| ((*b - first) as f32 * hop_ms).round() as u32)
            .collect(),
        voiced_ms,
        span_ms,
        median_hz,
    }
    .finish()
}

impl ToneReport {
    /// Fill in the aggregate from the syllables, and word the summary.
    ///
    /// The aggregate ignores a syllable that was never scoreable (the neutral
    /// tone), so a word containing one can still be judged on the rest.
    fn finish(mut self) -> Self {
        let scored: Vec<&SyllableReport> = self
            .syllables
            .iter()
            .filter(|s| (1..=4).contains(&s.expected_tone))
            .collect();

        if scored.is_empty() {
            self.verdict = ToneVerdict::Uncertain;
            self.score = 0.0;
            self.grade = Grade::from_score(0.0);
            if self.detail.is_empty() {
                self.detail = "Nothing in this could be scored.".into();
            }
            return self;
        }

        let off: Vec<&SyllableReport> = scored
            .iter()
            .copied()
            .filter(|s| s.attempt.verdict == ToneVerdict::OffTarget)
            .collect();
        let unsure: Vec<&SyllableReport> = scored
            .iter()
            .copied()
            .filter(|s| s.attempt.verdict == ToneVerdict::Uncertain)
            .collect();

        self.verdict = if !off.is_empty() {
            ToneVerdict::OffTarget
        } else if !unsure.is_empty() {
            ToneVerdict::Uncertain
        } else {
            ToneVerdict::Match
        };

        self.score = scored.iter().map(|s| s.attempt.score).sum::<f32>() / scored.len() as f32;
        self.grade = Grade::from_score(self.score);

        if self.detail.is_empty() {
            self.detail = self.summary(&scored, &off, &unsure);
        }
        self
    }

    fn summary(
        &self,
        scored: &[&SyllableReport],
        off: &[&SyllableReport],
        unsure: &[&SyllableReport],
    ) -> String {
        if self.syllables.len() == 1 {
            return self.syllables[0].attempt.detail.clone();
        }
        let total = scored.len();
        if off.is_empty() && unsure.is_empty() {
            return format!("All {total} tones are right.");
        }
        let right = total - off.len() - unsure.len();
        let worst = off.first().or_else(|| unsure.first()).expect("one of them");
        let label = if off.is_empty() { "unclear" } else { "wrong" };
        format!(
            "{right} of {total} tones are right; syllable {} (tone {}) was {label}.",
            worst.position, worst.expected_tone
        )
    }
}

/// Median F0 over a frame range, in Hz.
fn median_hz_of(track: &PitchTrack, first: usize, last: usize) -> Option<f32> {
    let mut values: Vec<f32> = track.frames[first..=last]
        .iter()
        .filter(|f| f.voiced && f.hz > 0.0)
        .map(|f| f.hz.log2())
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(values[values.len() / 2].exp2())
}

/// Score one spoken syllable against the tone it should have carried.
///
/// `samples` is mono, at `sample_rate`, nominally -1..=1. Any rate works; it is
/// resampled to [`TARGET_SAMPLE_RATE`] first.
///
/// A tone of `5` (neutral) or `0` (not a tone) is not scored and comes back as
/// [`ToneVerdict::Uncertain`]. For a word, use [`analyze`].
pub fn analyze_tone(samples: &[f32], sample_rate: u32, expected_tone: u8) -> ToneAttempt {
    analyze(samples, sample_rate, &[expected_tone])
        .syllables
        .into_iter()
        .next()
        .map(|s| s.attempt)
        .unwrap_or_else(|| unsupported(expected_tone))
}

// ---------------------------------------------------------------------------
// Reading a tone out of pinyin
// ---------------------------------------------------------------------------
//
// Moved to `crate::pinyin`. Splitting a *word's* reading into syllables is what
// scoring a word needs, and it belongs with the sandhi rules rather than with
// the signal processing — `pinyin::tone_target` is the entry point now.

#[cfg(test)]
mod tests {
    use super::*;

    /// A voice with jitter in it, which is the difference between this suite and
    /// a real one.
    ///
    /// [`say`] and [`say_at`] are perfect tones with three steady harmonics, and
    /// YIN finds a period in those almost anywhere — so a test built on them
    /// measures what the estimator does to a *signal generator*, not to a person.
    /// Every short-syllable test in this file passed while a real 是 was being
    /// refused on a phone. This adds the cycle-to-cycle variation real phonation
    /// has, which is what makes a window that is too long fail.
    ///
    /// Deterministic, so a failure is reproducible.
    fn say_rough(base: f32, len: usize, semitones: &[(f32, f32)], pad_ms: u32) -> Vec<f32> {
        let sr = TARGET_SAMPLE_RATE as f32;
        let pad = (sr * pad_ms as f32 / 1000.0) as usize;
        let mut out = vec![0.0f32; pad];
        let mut phase = 0.0f32;
        let mut state = 4242u32;
        for i in 0..len {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let jitter = ((state >> 9) as f32 / 4_194_304.0 - 1.0) * 0.02;
            let t = i as f32 / len as f32;
            let st = sample_anchors(semitones, t);
            let hz = base * (2.0f32).powf(st / 12.0) * (1.0 + jitter);
            out.push(
                0.35 * (phase * std::f32::consts::TAU).sin()
                    + 0.12 * (phase * std::f32::consts::TAU * 2.0).sin()
                    + 0.05 * (phase * std::f32::consts::TAU * 3.0).sin(),
            );
            phase += hz / sr;
        }
        out.extend(vec![0.0f32; pad]);
        out
    }

    /// A sine wave whose frequency follows a contour, with silence either side.
    ///
    /// The true F0 is known by construction, which is the only reason a pitch
    /// estimator can be tested at all.
    fn say(len: usize, semitones: &[(f32, f32)], pad_ms: u32) -> Vec<f32> {
        say_at(180.0, len, semitones, pad_ms)
    }

    /// The same, at a chosen fundamental — a low male voice is the case
    /// [`PitchConfig::f_min`] has to stay under, so it needs to be sayable.
    fn say_at(base: f32, len: usize, semitones: &[(f32, f32)], pad_ms: u32) -> Vec<f32> {
        let sr = TARGET_SAMPLE_RATE as f32;
        let pad = (sr * pad_ms as f32 / 1000.0) as usize;
        let mut out = vec![0.0f32; pad];
        let mut phase = 0.0f32;
        for i in 0..len {
            let t = i as f32 / len as f32;
            let st = sample_anchors(semitones, t);
            let hz = base * (st / 12.0).exp2();
            // A couple of harmonics, so this is not a pure tone that flatters
            // the estimator.
            let sample = 0.35 * (phase * std::f32::consts::TAU).sin()
                + 0.12 * (phase * std::f32::consts::TAU * 2.0).sin()
                + 0.05 * (phase * std::f32::consts::TAU * 3.0).sin();
            out.push(sample);
            phase += hz / sr;
        }
        out.extend(vec![0.0f32; pad]);
        out
    }

    fn voiced_hz(samples: &[f32]) -> Vec<f32> {
        track_pitch(samples, TARGET_SAMPLE_RATE, &PitchConfig::default())
            .frames
            .iter()
            .filter(|f| f.voiced)
            .map(|f| f.hz)
            .collect()
    }

    #[test]
    fn resample_keeps_length_and_level() {
        let input: Vec<f32> = (0..4800).map(|_| 0.5).collect();
        let out = resample(&input, 48_000, 16_000);
        assert_eq!(out.len(), 1600);
        assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
        // Same rate is a copy, and an empty input stays empty.
        assert_eq!(resample(&input, 16_000, 16_000).len(), 4800);
        assert!(resample(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn yin_finds_a_steady_tone() {
        let samples = say(8000, &[(0.0, 0.0), (1.0, 0.0)], 60);
        let hz = voiced_hz(&samples);
        assert!(hz.len() > 20, "expected voiced frames, got {}", hz.len());
        let median = hz[hz.len() / 2];
        assert!(
            (median - 180.0).abs() < 6.0,
            "expected about 180 Hz, got {median}"
        );
    }

    #[test]
    fn yin_reports_silence_as_unvoiced() {
        let quiet = vec![0.0f32; 8000];
        let track = track_pitch(&quiet, TARGET_SAMPLE_RATE, &PitchConfig::default());
        assert!(!track.frames.is_empty(), "silence still produces frames");
        assert!(track.frames.iter().all(|f| !f.voiced));
    }

    #[test]
    fn yin_reports_noise_as_unvoiced() {
        // Deterministic white noise from a small LCG — no RNG dependency, and
        // the same signal every run. Aperiodic, so there is no period to find.
        //
        // Not an alternating signal, which is the tempting choice and is wrong:
        // `+a, -a, +a, …` is a perfect square wave at half the sample rate, so
        // it is genuinely periodic and calling it voiced would be correct.
        let mut state = 12345u32;
        let noise: Vec<f32> = (0..8000)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect();
        let track = track_pitch(&noise, TARGET_SAMPLE_RATE, &PitchConfig::default());
        let voiced = track.frames.iter().filter(|f| f.voiced).count();
        assert_eq!(voiced, 0, "aperiodic input must not be called voiced");
    }

    #[test]
    fn yin_tracks_a_falling_glide() {
        let samples = say(8000, &[(0.0, 4.0), (1.0, -4.0)], 60);
        let hz = voiced_hz(&samples);
        assert!(hz.len() > 20);
        let first = hz[2];
        let last = hz[hz.len() - 3];
        assert!(
            first > last * 1.3,
            "expected a clear fall, got {first} Hz then {last} Hz"
        );
    }

    #[test]
    fn templates_have_the_shapes_their_names_claim() {
        let t1 = tone_template(1);
        assert!(t1.iter().all(|v| v.abs() < 1e-4), "tone 1 is level");

        let t2 = tone_template(2);
        assert!(t2[t2.len() - 1] > t2[0] + 3.0, "tone 2 rises");

        let t4 = tone_template(4);
        assert!(t4[0] > t4[t4.len() - 1] + 6.0, "tone 4 falls");

        let t3 = tone_template(3);
        let min = t3.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            min < t3[0] - 1.5 && min < t3[t3.len() - 1] - 1.5,
            "tone 3 dips below both ends: {t3:?}"
        );
    }

    #[test]
    fn dtw_separates_shapes_once_amplitude_is_normalised() {
        // The property this locks is the one that was wrong on the first
        // attempt: compared as raw semitones, a falling contour sat closer to a
        // *rising* template than to a level one, because DTW could slide the
        // fall onto its own mirror while the amplitudes differed by a factor of
        // two. Normalising removes that, and a fall must then be nearest its own
        // template and furthest from a rise.
        let fall = normalise_rms(&tone_template(4));
        let rise = normalise_rms(&tone_template(2));
        let dip = normalise_rms(&tone_template(3));

        assert!(dtw_distance(&fall, &fall) < 1e-5);
        assert!(
            dtw_distance(&fall, &rise) > dtw_distance(&fall, &dip),
            "a fall is nearer a dip, which shares its first half, than a rise"
        );
        assert!(
            dtw_distance(&fall, &rise) > 1.0,
            "a fall and a rise must be clearly apart, got {}",
            dtw_distance(&fall, &rise)
        );
    }

    #[test]
    fn a_rising_syllable_matches_tone_two() {
        let samples = say(8000, &[(0.0, -2.5), (1.0, 2.5)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 2);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(2));
        assert!(attempt.score > 60.0, "score was {}", attempt.score);
        assert_eq!(attempt.contour.len(), CONTOUR_POINTS);
        assert_eq!(attempt.reference.len(), CONTOUR_POINTS);
    }

    #[test]
    fn a_falling_syllable_is_off_target_for_tone_two() {
        let samples = say(8000, &[(0.0, 4.0), (1.0, -4.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 2);
        assert_eq!(attempt.verdict, ToneVerdict::OffTarget, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(4));
        assert!(attempt.detail.contains("falling"), "{}", attempt.detail);
        // The same recording against its own tone scores well.
        let good = analyze_tone(&samples, TARGET_SAMPLE_RATE, 4);
        assert_eq!(good.verdict, ToneVerdict::Match);
        assert!(
            good.score > attempt.score,
            "a fall should score better against tone 4 than tone 2"
        );
    }

    #[test]
    fn a_dipping_syllable_is_evidence_for_tone_three() {
        let samples = say(8000, &[(0.0, -2.0), (0.5, -4.0), (1.0, 2.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 3);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(3));
    }

    #[test]
    fn a_level_syllable_is_accepted_for_tone_one_and_tone_three() {
        // Tone 3 is usually realised flat and low in running speech. Scoring it
        // against the textbook dip would flag correct speech as wrong, so a
        // level contour has to count for both.
        let samples = say(8000, &[(0.0, 0.0), (1.0, 0.0)], 50);
        for tone in [1u8, 3] {
            let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, tone);
            assert_eq!(
                attempt.verdict,
                ToneVerdict::Match,
                "tone {tone}: {}",
                attempt.detail
            );
        }
        // But a level syllable is not a rising or a falling one.
        for tone in [2u8, 4] {
            let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, tone);
            assert_eq!(
                attempt.verdict,
                ToneVerdict::OffTarget,
                "tone {tone}: {}",
                attempt.detail
            );
        }
    }

    #[test]
    fn a_shape_that_matches_no_tone_is_uncertain() {
        // An arch — pitch up then down, the inverse of a tone-3 dip — is not
        // close to any of the four. The honest answer is that, rather than
        // whichever template happens to be nearest. This is the branch that
        // keeps "not sure" reachable at all.
        let samples = say(8000, &[(0.0, -2.0), (0.5, 4.0), (1.0, -2.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 1);
        assert_eq!(attempt.verdict, ToneVerdict::Uncertain, "{}", attempt.detail);
        assert!(attempt.detail.contains("Between"), "{}", attempt.detail);
    }

    #[test]
    fn the_score_separates_a_right_tone_from_a_wrong_one() {
        // A score that does not separate these two is decoration. The bands are
        // `Grade`'s, shared with handwriting, so a tone and a stroke mean the
        // same thing when they say "good".
        let rise = say(8000, &[(0.0, -2.5), (1.0, 2.5)], 50);

        let matched = analyze_tone(&rise, TARGET_SAMPLE_RATE, 2);
        assert!(
            matched.score >= 75.0,
            "a matched tone should be Good or better, got {}",
            matched.score
        );
        assert!(matches!(matched.grade, Grade::Good | Grade::Excellent));

        let wrong = analyze_tone(&rise, TARGET_SAMPLE_RATE, 4);
        assert!(
            wrong.score < 55.0,
            "a wrong tone should be Poor, got {}",
            wrong.score
        );
        assert_eq!(wrong.grade, Grade::Poor);
    }

    #[test]
    fn a_shallow_but_correct_movement_says_so() {
        // A tone 4 that only falls a little: the right direction and not much
        // travel. The score is about direction (see `normalise_rms`), so this
        // matches — and the wording says the movement was small rather than
        // letting a good number imply it was convincing.
        let samples = say(8000, &[(0.0, 1.6), (1.0, -0.2)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 4);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert!(attempt.detail.contains("shallow"), "{}", attempt.detail);
    }

    /// A fricative: deterministic aperiodic noise, which is what a consonant
    /// between two syllables is and what the segmentation is meant to find.
    fn fricative(ms: u32, sr: f32) -> Vec<f32> {
        let len = (sr * ms as f32 / 1000.0) as usize;
        let mut state = 999u32;
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 8_388_608.0 - 1.0) * 0.05
            })
            .collect()
    }

    /// A word: one pitch contour per syllable, separated by a fricative.
    ///
    /// The leading and trailing silence is real — a learner presses the button
    /// before speaking and releases after — and its position is what fixes where
    /// the fricatives land, which the boundary tests assert against.
    fn say_word(syllables: &[(f32, f32)], per_ms: u32, gap_ms: u32) -> Vec<f32> {
        let even: Vec<(f32, f32, u32)> = syllables
            .iter()
            .map(|(from, to)| (*from, *to, per_ms))
            .collect();
        say_word_at(&even, gap_ms)
    }

    /// The same, with each syllable given its own length.
    ///
    /// Real words are not made of equal syllables: the middle one of a
    /// three-syllable word is usually the shortest and the most reduced, which is
    /// exactly the case the scoring floor used to throw away. A helper that can
    /// only build even words cannot express the failure.
    fn say_word_at(syllables: &[(f32, f32, u32)], gap_ms: u32) -> Vec<f32> {
        let sr = TARGET_SAMPLE_RATE as f32;
        let mut out = vec![0.0f32; (sr * 0.05) as usize];
        for (index, (from, to, per_ms)) in syllables.iter().enumerate() {
            if index > 0 {
                out.extend(fricative(gap_ms, sr));
            }
            out.extend(tone(*from, *to, *per_ms));
        }
        out.extend(vec![0.0f32; (sr * 0.05) as usize]);
        out
    }

    /// A word whose syllables are separated by *silence* of a given length rather
    /// than by a fricative — a learner pausing between them.
    ///
    /// The distinction matters: silence is the cheapest thing in a recording to
    /// put a boundary on, so it is what provokes the splitter into cutting twice
    /// in the same gap.
    fn say_word_pausing(syllables: &[(f32, f32, u32)], pause_ms: u32) -> Vec<f32> {
        let sr = TARGET_SAMPLE_RATE as f32;
        let mut out = vec![0.0f32; (sr * 0.05) as usize];
        for (index, (from, to, per_ms)) in syllables.iter().enumerate() {
            if index > 0 {
                out.extend(vec![0.0f32; (sr * pause_ms as f32 / 1000.0) as usize]);
            }
            out.extend(tone(*from, *to, *per_ms));
        }
        out.extend(vec![0.0f32; (sr * 0.05) as usize]);
        out
    }

    /// One voiced syllable: a contour from `from` to `to` semitones over `per_ms`.
    fn tone(from: f32, to: f32, per_ms: u32) -> Vec<f32> {
        let sr = TARGET_SAMPLE_RATE as f32;
        let len = (sr * per_ms as f32 / 1000.0) as usize;
        let mut out = Vec::with_capacity(len);
        let mut phase = 0.0f32;
        for i in 0..len {
            let t = i as f32 / len as f32;
            let st = from + (to - from) * t;
            let hz = 180.0 * (2.0f32).powf(st / 12.0);
            out.push(0.35 * (phase * std::f32::consts::TAU).sin());
            phase += hz / sr;
        }
        out
    }

    /// The splitter must not put two boundaries in the same silence.
    ///
    /// **This is the failure a real voice found and no synthetic test did.** A long
    /// quiet stretch is the cheapest place in a recording to put a boundary, so
    /// with two boundaries to place, the search put both of them inside it,
    /// `min_len` apart — the least the frame-length rule allowed. The learner got a
    /// middle "syllable" of 31 ms holding no voice, reported as "I could not hear
    /// syllable 2 clearly", and a third segment holding two syllables' worth of
    /// pitch. Six rounds of 是不是 out of eight, and four of 中国人 out of seven,
    /// did exactly this.
    ///
    /// What stops it is requiring voice in every segment: two cuts inside one
    /// silence leave a segment with none in it, and a syllable has a vowel.
    #[test]
    fn a_silence_between_two_syllables_is_not_cut_twice() {
        let samples = say_word_pausing(
            &[(-2.0, 2.0, 180), (1.0, 4.0, 140), (1.0, 1.0, 220)],
            260,
        );
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[2, 2, 1]);

        assert_eq!(
            report.boundaries_ms.len(),
            2,
            "the word must still be divided: {}",
            report.detail
        );
        let gap = report.boundaries_ms[1] - report.boundaries_ms[0];
        assert!(
            gap > 100,
            "the boundaries are {gap} ms apart, so the middle syllable is a sliver: {:?}",
            report.boundaries_ms
        );
        for syllable in &report.syllables {
            assert!(
                syllable.attempt.voiced_ms > 0,
                "every syllable must have voice in it, and syllable {} has none: {}",
                syllable.position,
                syllable.attempt.detail
            );
        }
    }

    #[test]
    fn a_two_syllable_word_is_split_at_the_consonant_between_them() {
        // Rising, rising: 学习, tones 2 + 2.
        let samples = say_word(&[(-2.5, 2.5), (-2.5, 2.5)], 220, 60);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[2, 2]);

        assert_eq!(report.syllables.len(), 2);
        assert_eq!(report.boundaries_ms.len(), 1, "one boundary for two syllables");
        // Measured from the first voiced frame, so the first syllable's own
        // length (220 ms) is the expectation — not the 50 ms of leading silence
        // the recording also holds.
        let boundary = report.boundaries_ms[0];
        assert!(
            boundary.abs_diff(220) < 30,
            "expected the split after the first 220 ms syllable, got {boundary} ms"
        );
        assert!(
            boundary < report.voiced_ms,
            "the split must fall inside the voiced speech: {boundary} ms of {} ms",
            report.voiced_ms
        );
        assert_eq!(report.verdict, ToneVerdict::Match, "{}", report.detail);
        assert!(
            report.syllables.iter().all(|s| s.attempt.verdict == ToneVerdict::Match),
            "{:?}",
            report.detail
        );
        assert!(report.score > 75.0, "score was {}", report.score);
        assert_eq!(report.detail, "All 2 tones are right.");
        assert_eq!(report.grade, Grade::from_score(report.score));
    }

    #[test]
    fn a_word_reports_which_syllable_was_wrong() {
        // The first syllable rises as asked, the second falls instead: 学习 with
        // the second syllable said as a fourth tone.
        let samples = say_word(&[(-2.5, 2.5), (4.0, -4.0)], 220, 60);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[2, 2]);

        assert_eq!(report.syllables[0].attempt.verdict, ToneVerdict::Match);
        assert_eq!(
            report.syllables[1].attempt.verdict,
            ToneVerdict::OffTarget,
            "{}",
            report.syllables[1].attempt.detail
        );
        assert_eq!(report.syllables[1].attempt.heard_tone, Some(4));
        assert_eq!(report.verdict, ToneVerdict::OffTarget);
        assert!(
            report.detail.contains("1 of 2") && report.detail.contains("syllable 2"),
            "the summary must name the syllable: {}",
            report.detail
        );
        // The aggregate is the mean of the two, so it sits between them.
        let mean = (report.syllables[0].attempt.score + report.syllables[1].attempt.score) / 2.0;
        assert!((report.score - mean).abs() < 0.01);
    }

    #[test]
    fn three_syllables_are_divided_twice() {
        let samples = say_word(&[(-2.5, 2.5), (4.0, -4.0), (0.0, 0.0)], 200, 60);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[2, 4, 1]);

        assert_eq!(report.syllables.len(), 3);
        assert_eq!(report.boundaries_ms.len(), 2, "two boundaries for three syllables");
        assert!(
            report.boundaries_ms[0] < report.boundaries_ms[1],
            "boundaries must ascend: {:?}",
            report.boundaries_ms
        );
        assert_eq!(report.verdict, ToneVerdict::Match, "{}", report.detail);
    }

    /// The reported failure this fixes: 是 at a conversational rate.
    ///
    /// A fourth tone is the shortest tone Mandarin has, and 是 is a long `sh`
    /// followed by a vowel of roughly 80–150 ms. The floor used to be 90 ms of
    /// *measured* voicing, and a frame is only counted when its whole 33 ms
    /// window is periodic, so a vowel this short never reached it: the learner
    /// was told "I could not hear enough voice to judge" about a syllable they
    /// had plainly said, and speaking slowly — which stretches the vowel — was
    /// the only way through. See `MIN_VOICED_MS`.
    #[test]
    fn a_short_syllable_at_a_speaking_rate_is_scored_rather_than_refused() {
        let samples = say_word_at(&[(4.0, -4.0, 80)], 60);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 4);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(4));
    }

    /// The shape a fourth tone actually has, and why the window had to shrink.
    ///
    /// Every other test here falls *evenly*, and an even fall was measured
    /// perfectly well by the 33 ms window that `f_min = 60` bought — which is why
    /// they all passed while an ordinary 是 was being refused. A real tone 4 is
    /// `51`: it stays high and then drops hard, so inside one 33 ms window the
    /// pitch moves far enough that no lag in it holds a period. YIN returned
    /// unvoiced for every frame, and the syllable came back as "I could not hear
    /// enough voice to judge" rather than as a wrong tone. At 80 Hz the window is
    /// 25 ms and the fall is visible inside it.
    ///
    /// **This test is the one that fails if the window goes back.** It is the
    /// regression the hand test on a real voice found; the even-fall tests above
    /// it cannot see it.
    #[test]
    fn a_short_syllable_with_a_fast_fall_is_heard_at_all() {
        // 1280 samples is 80 ms: an ordinary 是, not a careful one. 200 Hz and a
        // 4 → 2 → −6 semitone shape is what a real fourth tone looks like — high,
        // then a hard drop — and `say_rough` because a perfect tone does not
        // reproduce the failure.
        let samples = say_rough(200.0, 1280, &[(0.0, 4.0), (0.5, 2.0), (1.0, -6.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 4);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(4));
    }

    /// Raising `f_min` to see a fast fall has a floor of its own.
    ///
    /// This is what keeps the window from shrinking further: `f_min` is the
    /// lowest fundamental the lag search covers, and a voice below it is tracked
    /// at the wrong octave or not at all. 90 Hz is about as low as speech goes —
    /// it tracks correctly at 80 Hz and collapses at 100, which is why the window
    /// is 25 ms rather than the 20 ms that would suit a fast fall even better.
    #[test]
    fn a_low_voice_is_still_tracked() {
        let samples = say_at(90.0, 2560, &[(0.0, 0.0), (1.0, 0.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 1);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert!(
            (attempt.median_hz - 90.0).abs() < 6.0,
            "expected about 90 Hz, got {:.0}",
            attempt.median_hz
        );
    }

    /// A single mis-measured frame must not decide what a syllable sounds like.
    ///
    /// **This is the shape the hand test found**, built directly from its numbers:
    /// segments of 166 ms reporting 27.4 semitones, with the raw track jumping
    /// between 83 and 441 Hz and 4 to 12 discontinuities in a segment. Across six
    /// rounds of 中国人 the segments with jumps were exactly the segments with
    /// impossible ranges, and the one segment with no jumps in every round had a
    /// range of 1 to 5 semitones.
    ///
    /// `range_st` is `max - min` over the contour and `Contour::is_flat` is
    /// decided by it, so one bad frame was enough to turn a level tone 1 into
    /// "heard falling" — which is what the learner was told, repeatedly.
    #[test]
    fn one_frame_at_the_wrong_frequency_does_not_set_the_range() {
        let mut frames: Vec<PitchFrame> = (0..80)
            .map(|_| PitchFrame {
                hz: 180.0,
                voiced: true,
                rms: 0.1,
            })
            .collect();
        // Two frames at three times the frequency, which is what the real tracks
        // did: 83–441 Hz and 133–433 Hz around a median near 145.
        frames[30].hz = 540.0;
        frames[58].hz = 540.0;
        let track = PitchTrack {
            frames,
            hop: 50,
            sample_rate: TARGET_SAMPLE_RATE,
        };

        let contour = contour_in(&track, 0, 79).expect("80 frames of clear voice");
        assert!(
            contour.range_st < 2.0,
            "one bad frame set the range to {:.1} st",
            contour.range_st
        );
        assert!(
            contour.is_flat(),
            "a level tone must still read as level: {:.1} st",
            contour.range_st
        );
        assert!(
            (contour.median_hz - 180.0).abs() < 1.0,
            "the median must ignore them too, got {:.0} Hz",
            contour.median_hz
        );
    }

    /// The other half of the same report: the middle of a three-syllable word.
    ///
    /// 180 / 80 / 220 ms is the shape of a word like 对不起 — the middle syllable
    /// is the shortest and the most reduced, so it was the first to fall under
    /// the floor. It came back as "I could not hear syllable 2 clearly" while the
    /// split itself was correct, which is what made it look like a segmentation
    /// bug rather than a measurement floor.
    #[test]
    fn a_three_syllable_word_whose_middle_is_short_is_scored_throughout() {
        let samples = say_word_at(&[(-2.0, 2.0, 180), (1.0, 4.0, 80), (1.0, 1.0, 220)], 50);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[2, 2, 1]);

        assert_eq!(report.boundaries_ms.len(), 2, "the word must still be divided");
        let scored = report
            .syllables
            .iter()
            .filter(|s| s.attempt.verdict == ToneVerdict::Match)
            .count();
        assert_eq!(scored, 3, "every syllable should be scored: {}", report.detail);
    }

    /// The splitter and the scorer answer to the same floor.
    ///
    /// They used to be a frame count and a millisecond figure with nothing tying
    /// them together, so the splitter could cut out a segment it was satisfied
    /// with and the scorer then refuse it — a syllable the learner said, that the
    /// app had just decided was long enough, reported as inaudible.
    #[test]
    fn the_splitter_will_not_cut_out_a_segment_the_scorer_would_refuse() {
        let track = track_pitch(
            &say(8000, &[(0.0, 0.0), (1.0, 0.0)], 50),
            TARGET_SAMPLE_RATE,
            &PitchConfig::default(),
        );
        let floor = min_voiced_frames(&track);
        let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;

        assert!(floor >= MIN_VOICED_FLOOR_FRAMES, "{floor} frames");
        assert!(
            floor as f32 * hop_ms >= MIN_VOICED_MS as f32,
            "{floor} frames at {hop_ms:.2} ms is under the {MIN_VOICED_MS} ms floor"
        );
    }

    /// The floor came down, but not past a transient.
    ///
    /// This is the guard the lowering had to keep: 40 ms of perfect tone is not a
    /// syllable, and 160 ms of it is. The window between the two is where the
    /// judgement lives, and it is deliberately not asserted to the millisecond —
    /// see the note on `MIN_VOICED_MS` about calibrating against a real voice.
    #[test]
    fn the_floor_still_refuses_something_too_short_to_be_a_syllable() {
        // Deliberately far from the floor rather than near it. A 40 ms tone would
        // have been the more obvious fixture and a worse test: it lands within a
        // few milliseconds of the gate, so it would break on any nudge and would
        // not say which side was wrong. The floor's job is to tell a *transient*
        // from a syllable, and that is what is asserted.
        let transient = say(320, &[(0.0, 0.0), (1.0, 0.0)], 0);
        assert_eq!(
            analyze_tone(&transient, TARGET_SAMPLE_RATE, 1).verdict,
            ToneVerdict::Uncertain,
            "20 ms of tone must not be scored as a syllable"
        );

        let spoken = say(2560, &[(0.0, 0.0), (1.0, 0.0)], 0);
        assert_eq!(
            analyze_tone(&spoken, TARGET_SAMPLE_RATE, 1).verdict,
            ToneVerdict::Match,
            "160 ms of tone is a syllable and must be scored"
        );
    }

    #[test]
    fn a_word_that_was_not_said_as_separate_syllables_is_refused() {
        // One short syllable against two expected: there is nowhere to put a
        // boundary, so the honest answer is that, not an invented split.
        let samples = say(600, &[(0.0, 0.0), (1.0, 0.0)], 50);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[1, 1]);

        assert_eq!(report.verdict, ToneVerdict::Uncertain);
        assert!(report.boundaries_ms.is_empty());
        assert!(
            report.detail.contains("could not separate"),
            "{}",
            report.detail
        );
        assert!(report.voiced_ms > 0, "it should still report what it heard");
    }

    #[test]
    fn a_neutral_syllable_is_scored_for_being_level() {
        // 妈妈: tone 1 then the neutral tone. Both are level here, so both match
        // — and the neutral one says what was and was not judged.
        let samples = say_word(&[(2.0, 2.0), (-1.0, -1.0)], 220, 60);
        let report = analyze(&samples, TARGET_SAMPLE_RATE, &[1, 5]);

        assert_eq!(report.syllables.len(), 2);
        assert_eq!(report.syllables[1].attempt.verdict, ToneVerdict::Match);
        assert_eq!(report.syllables[1].attempt.heard_tone, Some(5));
        assert!(
            report.syllables[1].attempt.detail.contains("neutral"),
            "{}",
            report.syllables[1].attempt.detail
        );
        assert_eq!(report.verdict, ToneVerdict::Match, "{}", report.detail);
    }

    #[test]
    fn a_neutral_tone_asked_for_and_given_a_moving_tone_is_wrong() {
        // The one thing a neutral tone cannot be is a clear movement, and that
        // is answered by a rule rather than by a distance — there is no neutral
        // template to measure against, because its height comes from the
        // syllable before it.
        let samples = say(8000, &[(0.0, -3.0), (1.0, 3.0)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 5);
        assert_eq!(attempt.verdict, ToneVerdict::OffTarget, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(2), "a rising contour was heard");
        assert!(attempt.detail.contains("neutral"), "{}", attempt.detail);
        assert!((0.0..=100.0).contains(&attempt.score));
    }

    #[test]
    fn a_word_never_scores_more_syllables_than_it_was_asked_for() {
        // Whatever the recording holds, the report has one entry per expected
        // tone, so the interface can zip it against the characters.
        for expected in [vec![1], vec![1, 1], vec![1, 2, 4], vec![4, 4, 4, 4]] {
            let shape = vec![(0.0f32, 0.0f32); expected.len()];
            let samples = say_word(&shape, 200, 60);
            let report = analyze(&samples, TARGET_SAMPLE_RATE, &expected);
            assert_eq!(report.syllables.len(), expected.len(), "{expected:?}");
            assert_eq!(report.expected, expected);
            assert_eq!(report.boundaries_ms.len(), expected.len() - 1, "{expected:?}");
            // Boundaries share a baseline with `voiced_ms`, so every one of them
            // has to fall inside the speech. A boundary that is reported beyond
            // the end of the utterance means the two are measured from different
            // places, which is a display bug that looks like a scoring bug.
            for boundary in &report.boundaries_ms {
                assert!(
                    *boundary < report.voiced_ms,
                    "{expected:?}: boundary {boundary} ms is outside {} ms of speech",
                    report.voiced_ms
                );
            }
        }
    }

    #[test]
    fn silence_is_uncertain_rather_than_wrong() {
        let samples = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 1);
        assert_eq!(attempt.verdict, ToneVerdict::Uncertain);
        assert_eq!(attempt.heard_tone, None);
        assert!(attempt.contour.is_empty());
        assert!(attempt.detail.contains("could not hear"), "{}", attempt.detail);
    }

    #[test]
    fn a_click_is_uncertain_rather_than_wrong() {
        // 20 ms of tone: real voice, but not a syllable.
        let samples = say(320, &[(0.0, 0.0), (1.0, 0.0)], 0);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 1);
        assert_eq!(attempt.verdict, ToneVerdict::Uncertain, "{}", attempt.detail);
    }

    #[test]
    fn a_tone_that_is_not_a_tone_is_refused_rather_than_guessed() {
        // 0 is "no tone number at all", which is a fact about the reading rather
        // than about the recording, so it is refused whatever was heard. Neutral
        // (5) used to be refused alongside it and is now judged — see
        // `a_neutral_syllable_is_scored_for_being_level`.
        let attempt = analyze_tone(&vec![0.1f32; 16_000], TARGET_SAMPLE_RATE, 0);
        assert_eq!(attempt.verdict, ToneVerdict::Uncertain);
        assert!(
            attempt.detail.contains("can be judged"),
            "{}",
            attempt.detail
        );
    }

    #[test]
    fn scores_stay_in_range_and_map_onto_the_shared_bands() {
        let samples = say(8000, &[(0.0, -2.5), (1.0, 2.5)], 50);
        let attempt = analyze_tone(&samples, TARGET_SAMPLE_RATE, 2);
        assert!((0.0..=100.0).contains(&attempt.score));
        assert_eq!(attempt.grade, Grade::from_score(attempt.score));
        assert!(attempt
            .contour
            .iter()
            .chain(attempt.reference.iter())
            .all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn capture_rate_does_not_change_the_verdict() {
        // The same syllable as a device would hand it over, at 48 kHz.
        let at_16 = say(8000, &[(0.0, 4.0), (1.0, -4.0)], 50);
        let at_48 = resample(&at_16, TARGET_SAMPLE_RATE, 48_000);
        let attempt = analyze_tone(&at_48, 48_000, 4);
        assert_eq!(attempt.verdict, ToneVerdict::Match, "{}", attempt.detail);
        assert_eq!(attempt.heard_tone, Some(4));
    }


}

