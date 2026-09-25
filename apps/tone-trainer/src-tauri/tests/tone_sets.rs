//! The two things this app claims, tested against the real embedded dataset.
//!
//! Both are claims about *sharing*, which is exactly the kind of thing that
//! breaks silently when a crate boundary moves:
//!
//! 1. the minimal pairs the drill shows are the derivation `hanzi-core` owns, so
//!    they are real minimal pairs — one syllable, one character per tone, and the
//!    members really do differ only in tone;
//! 2. a synthetic syllable of a known tone is judged as that tone through this
//!    app's own scoring path, so the tone scorer is wired up rather than merely
//!    linked.
//!
//! What is *not* tested here is the microphone. That needs hardware and a person,
//! and it lives in the full app as `records_from_the_real_microphone` — the
//! recorder itself is shared, so the instrument there covers this app too.

use hanzi_voice::Recording;
use tone_trainer_lib::Trainer;

/// The app's own startup path, so the dataset really is the embedded one.
///
/// `None` for the data directory: these tests never install a recognition model,
/// and one must not be picked up from the machine's real application data — the
/// tests below assert what happens *without* a model, which is how the app ships.
fn trainer() -> Trainer {
    Trainer::assemble(
        Trainer::prepare().expect("the embedded dataset should decode"),
        None,
    )
}

#[test]
fn the_embedded_dataset_yields_real_minimal_pairs() {
    let sets = trainer().tone_sets(400);
    assert!(
        sets.len() > 100,
        "expected hundreds of tone sets from the full course, got {}",
        sets.len()
    );

    for set in &sets {
        assert!(
            set.members.len() >= 2,
            "{} is not a pair: {} member(s)",
            set.base,
            set.members.len()
        );

        // Every member is a different tone of the *same* syllable. This is the
        // property that makes the drill worth doing, and the one a changed
        // grouping rule would quietly destroy: a set of 奴 `nú` and 女 `nǚ` would
        // drill a vowel contrast while claiming to drill tone.
        let mut tones: Vec<u8> = set.members.iter().map(|m| m.tone).collect();
        tones.sort_unstable();
        tones.dedup();
        assert_eq!(
            tones.len(),
            set.members.len(),
            "{} has two members at the same tone",
            set.base
        );
        for member in &set.members {
            assert!(
                (1..=4).contains(&member.tone),
                "{} carries tone {}, which is not drilled",
                member.reading,
                member.tone
            );
            assert_eq!(
                hanzi_core::pinyin::base(&member.reading),
                set.base,
                "{} does not have the set's syllable {}",
                member.reading,
                set.base
            );
            assert_eq!(
                hanzi_core::pinyin::tone_from_pinyin(&member.reading),
                Some(member.tone),
                "{} disagrees with its own tone number",
                member.reading
            );
        }
    }
}

#[test]
fn the_set_the_full_app_uses_is_here_too() {
    // 妈 mā / 麻 má / 马 mǎ / 骂 mà — the canonical four, and the one the README
    // and the app's own copy name. If the two apps ever derive different sets,
    // this is where it shows.
    let sets = trainer().tone_sets(400);
    let ma = sets
        .iter()
        .find(|set| set.base == "ma")
        .expect("ma is in the course");
    let characters: Vec<char> = ma.members.iter().map(|m| m.ch).collect();
    for expected in ['妈', '麻', '马', '骂'] {
        assert!(
            characters.contains(&expected),
            "ma should hold {expected}, holds {characters:?}"
        );
    }
}

/// A voice whose pitch follows a contour, with silence either side.
///
/// The true F0 is known by construction, which is the only way a pitch estimator
/// can be tested at all. `hanzi-core`'s own suite has this same generator — it is
/// private there, and duplicating twenty lines of signal is a smaller cost than
/// making a test-only helper part of a published API.
///
/// Two harmonics rather than a pure sine, so the test does not flatter the
/// estimator with a signal that has no harmonics to confuse it.
fn say(base_hz: f32, len: usize, anchors: &[(f32, f32)], pad_ms: u32) -> Vec<f32> {
    let sr = 16_000.0f32;
    let pad = (sr * pad_ms as f32 / 1000.0) as usize;
    let mut out = vec![0.0f32; pad];
    let mut phase = 0.0f32;
    for i in 0..len {
        let t = i as f32 / len as f32;
        let st = sample_anchors(anchors, t);
        let hz = base_hz * (st / 12.0).exp2();
        let sample = 0.35 * (phase * std::f32::consts::TAU).sin()
            + 0.12 * (phase * std::f32::consts::TAU * 2.0).sin()
            + 0.05 * (phase * std::f32::consts::TAU * 3.0).sin();
        out.push(sample);
        phase += hz / sr;
    }
    out.extend(vec![0.0f32; pad]);
    out
}

/// Linear interpolation between a tone's anchor points, clamped at the ends.
///
/// The same reading of the five-level scale that `hanzi_core::tone::tone_template`
/// uses, written out here because the template is published as a finished shape
/// and not as the anchors it was built from.
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

/// The canonical contour of one tone, as the anchors `tone_template` is built
/// from — kept here rather than exported from `hanzi-core`, which publishes the
/// template as a shape and not as this list.
fn anchors(tone: u8) -> &'static [(f32, f32)] {
    match tone {
        1 => &[(0.0, 4.0), (1.0, 4.0)],
        2 => &[(0.0, 0.0), (1.0, 4.0)],
        3 => &[(0.0, -2.0), (0.5, -4.0), (1.0, 2.0)],
        _ => &[(0.0, 4.0), (1.0, -4.0)],
    }
}

fn recording(samples: Vec<f32>) -> Recording {
    Recording {
        samples,
        sample_rate: 16_000,
        device: "synthetic".into(),
        truncated: false,
    }
}

#[test]
fn a_synthetic_rising_tone_is_judged_rising() {
    let trainer = trainer();
    // 300 ms of voice, 180 Hz, the shape of tone 2, in the middle of a second of
    // silence — long enough to clear the voicing floor with room to spare.
    let samples = say(180.0, 4_800, anchors(2), 200);
    let result = trainer.score(&recording(samples), "麻", "má");

    assert_eq!(result.syllables.len(), 1);
    let attempt = &result.syllables[0].attempt;
    assert_eq!(
        attempt.verdict,
        hanzi_core::ToneVerdict::Match,
        "a clean tone 2 should match: {}",
        attempt.detail
    );
    assert_eq!(attempt.heard_tone, Some(2));
    assert!(
        attempt.voiced_ms > 100,
        "expected a real measurement, got {} ms",
        attempt.voiced_ms
    );
    // The two lines the chart draws must both be there, or the panel has nothing
    // to show even though the score is right.
    assert!(attempt.contour.len() > 2, "no learner contour was produced");
    assert!(attempt.reference.len() > 2, "no reference shape was produced");
}

#[test]
fn a_synthetic_falling_tone_scores_badly_against_a_rising_one() {
    // The drill's whole premise: saying the wrong tone is visibly wrong. A tone 4
    // offered where tone 2 was asked for must not come back as a match — this is
    // the confusion the app exists to catch.
    let trainer = trainer();
    let samples = say(180.0, 4_800, anchors(4), 200);
    let result = trainer.score(&recording(samples), "麻", "má");

    let attempt = &result.syllables[0].attempt;
    assert_ne!(
        attempt.verdict,
        hanzi_core::ToneVerdict::Match,
        "a falling tone was accepted as rising: {}",
        attempt.detail
    );
    assert_eq!(attempt.heard_tone, Some(4));
}

/// Recognition is the optional half, and its absence must change nothing.
///
/// This is the property the whole feature rests on: the app ships with no model,
/// a learner may use it for years without installing one, and the tone judgement
/// has to be complete and identical either way. `heard` being `None` is how the
/// interface knows to draw no recognition block at all, rather than an empty one.
#[test]
fn without_a_model_nothing_is_recognised_and_nothing_is_reported_as_wrong() {
    let trainer = trainer();
    let samples = say(180.0, 4_800, anchors(2), 200);
    let result = trainer.score(&recording(samples), "麻", "má");

    assert!(
        result.heard.is_none(),
        "no model is installed, so nothing can have been recognised: {:?}",
        result.heard
    );
    assert!(
        result.heard_error.is_none(),
        "a missing model is not an error: {:?}",
        result.heard_error
    );
    // And the tone half is untouched by any of it: this is the same assertion the
    // test above makes, and it has to keep holding with recognition compiled in.
    assert_eq!(
        result.syllables[0].attempt.verdict,
        hanzi_core::ToneVerdict::Match
    );
}

/// Word families exist, hang on a tone contrast, and hold scorable words.
///
/// The drill this app now offers: not just 妈/麻/马/骂 but the words built on them.
#[test]
fn word_families_are_built_on_the_tone_contrasts() {
    let trainer = trainer();
    let families = trainer.word_sets(60);

    assert!(
        !families.is_empty(),
        "the HSK list should yield word families on the tone-set characters"
    );

    let sets = trainer.tone_sets(400);
    let contrasts: Vec<char> = sets
        .iter()
        .filter_map(|set| set.members.first().map(|m| m.ch))
        .collect();
    // The syllable each family is about, which is what the Words search box
    // matches a typed reading against — so it has to be the tone set's own base
    // and not something the family derived for itself.
    let syllables: Vec<&str> = sets.iter().map(|set| set.base.as_str()).collect();

    for family in &families {
        assert!(
            contrasts.contains(&family.key),
            "{} is not a character from any tone set",
            family.key
        );
        assert!(
            syllables.contains(&family.base.as_str()),
            "{} is not the syllable of any tone set",
            family.base
        );
        assert!(!family.words.is_empty(), "{} has no words", family.key);
        assert!(
            family.contrast.contains("tone"),
            "the header should name the contrast: {}",
            family.contrast
        );

        for word in &family.words {
            let syllables = word.text.chars().count();
            assert!(
                (2..=4).contains(&syllables),
                "{} is {syllables} syllables, outside what this app scores",
                word.text
            );
            // One tone per syllable, and the two lists agree on how many.
            assert_eq!(word.spoken.len(), syllables, "{} spoken tones", word.text);
            assert_eq!(word.citation.len(), syllables, "{} citation tones", word.text);
            for tone in word.spoken.iter().chain(word.citation.iter()) {
                assert!((1..=5).contains(tone), "{} has tone {tone}", word.text);
            }
        }
    }
}

/// Sandhi is applied, because scoring the dictionary's tones marks correct speech wrong.
///
/// 你好 is the case that matters: written tone 3 + tone 3, spoken 2 + 3. A learner
/// who says it the way it is actually said must score as correct, so the tones this
/// app scores against have to be the spoken ones — and the interface has to be able
/// to say *why* it is asking for tone 2 where a dictionary prints tone 3.
///
/// Asserted through `score`, not through a word *family*: 你 and 好 have no minimal
/// pair in the course, so neither is a family key and 你好 is not reachable that way.
/// The sandhi rule still applies to it, and this is the path a drill actually takes.
#[test]
fn a_word_is_scored_against_the_tones_it_is_spoken_with() {
    let trainer = trainer();
    // Silence is enough: the goal is what the *target* says, and every syllable
    // comes back with its citation and spoken tone whether or not pitch was found.
    let result = trainer.score(&recording(vec![0.0; 1_600]), "你好", "nǐhǎo");

    assert!(result.tone_scored, "你好 divides cleanly, so it is scored");
    assert!(
        result.sandhi_applied,
        "the interface is told sandhi moved a tone, so it can explain the difference"
    );
    let tones: Vec<(u8, u8)> = result
        .syllables
        .iter()
        .map(|s| (s.citation, s.spoken))
        .collect();
    assert_eq!(
        tones,
        vec![(3, 2), (3, 3)],
        "你好 is written 3 + 3 and spoken 2 + 3"
    );
    assert_eq!(result.syllables.len(), 2, "one entry per syllable");
}

/// A polyphonic word takes its reading from the dictionary, not from its characters.
///
/// 银行 is `yínháng`, not `yínxíng`: the isolated 行 has no context to pick a reading
/// from, so a reading composed from the characters would score the wrong tone. This
/// is the whole reason a word is resolved through the dataset's own entry.
#[test]
fn a_polyphonic_word_takes_the_dictionary_reading() {
    let trainer = trainer();
    let result = trainer.score(&recording(vec![0.0; 1_600]), "银行", "yínháng");

    assert!(result.tone_scored);
    let citation: Vec<u8> = result.syllables.iter().map(|s| s.citation).collect();
    assert_eq!(
        citation,
        vec![2, 2],
        "银行 is yínháng — tone 2 twice, which only the dictionary's reading says"
    );
    // And the readings are the ones that were passed, tone marks included.
    let readings: Vec<&str> = result.syllables.iter().map(|s| s.reading.as_str()).collect();
    assert_eq!(readings, vec!["yín", "háng"]);
}

/// A reading that will not divide one syllable per character is *unmeasured*.
///
/// Not scored zero: zero reads as a perfectly flat attempt, and the interface reads
/// `toneScored` to decide whether to draw a chart at all.
#[test]
fn a_reading_that_cannot_be_divided_is_unmeasured_rather_than_zero() {
    let trainer = trainer();
    // Three characters, one syllable's reading — the counts cannot be lined up.
    let result = trainer.score(&recording(vec![0.0; 1_600]), "中国人", "zhōng");

    assert!(
        !result.tone_scored,
        "a reading that will not divide must not be scored"
    );
    assert_eq!(result.score, 0.0);
    assert!(result.syllables.is_empty(), "nothing was judged, so nothing is reported");
    assert!(
        result.detail.contains("could not be lined up"),
        "and it says why: {}",
        result.detail
    );
}

#[test]
fn silence_is_refused_rather_than_scored() {
    // A tutor that scores noise is worse than one that admits it could not hear,
    // because the learner cannot tell a bad score from a bad measurement.
    let trainer = trainer();
    let result = trainer.score(&recording(vec![0.0; 16_000]), "妈", "mā");

    assert_eq!(result.verdict, hanzi_core::ToneVerdict::Uncertain);
    assert_eq!(result.score, 0.0);
    let attempt = &result.syllables[0].attempt;
    assert!(attempt.contour.is_empty(), "silence produced a contour");
    assert!(
        attempt.detail.contains("could not hear"),
        "the refusal should say so: {}",
        attempt.detail
    );
}
