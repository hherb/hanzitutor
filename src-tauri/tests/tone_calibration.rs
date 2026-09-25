//! The one test that needs real hardware, and the one that needs a person.
//!
//! It lives here rather than in `hanzi-voice` because it drives the *whole*
//! scoring path for a chosen character or word: the target comes from the
//! dataset, which is this app's, not the voice crate's. The recorder and the
//! pitch analysis it exercises are shared, so what this instrument finds
//! reaches both apps.
//!
//! Ignored by default because it needs a microphone and, on macOS, a granted
//! permission — neither of which a test run can arrange. Run it by hand:
//!
//! ```text
//! cargo test -p hanzi-tutor --test tone_calibration -- --ignored --nocapture records_from_the_real_microphone
//! ```
//!
//! `HANZI_TUTOR_TONE_TEST` picks the character or word, `HANZI_TUTOR_TONE_SECS`
//! the hold, and both are described below — which is the original note, moved
//! here with the test.

use hanzi_voice::Recorder;

mod calibration {
    // The test reaches these by bare name, the way it did inside the voice
    // module it was written in: `Recorder` from the shared crate, and `Duration`
    // for the hold between pressing Enter and the recording starting.
    use super::Recorder;
    use std::time::Duration;

    /// The one test that needs real hardware, and the one that needs a person.
    ///
    /// Ignored by default because it needs a microphone and, on macOS, a granted
    /// permission — neither of which a test run can arrange. Run it by hand:
    ///
    /// ```text
    /// cargo test -p hanzi-tutor --lib -- --ignored --nocapture records_from_the_real_microphone
    /// ```
    ///
    /// **It waits for you.** It prints a prompt, and nothing is recorded until you
    /// press Enter; then it says `RECORDING` in as many words, records for a few
    /// seconds, and prints what it measured. Press Enter again for another go, or
    /// `q` to finish. The first version of this started recording the instant the
    /// process did and was over in under two seconds, which is exactly as useful as
    /// it sounds — the report was "no chance to speak before it finishes".
    ///
    /// `--nocapture` is not optional. Without it the harness swallows the prompt
    /// and the test looks like it has hung.
    ///
    /// What it cannot tell you is whether the *permission* was granted: a denied
    /// microphone still opens and still streams, and delivers silence or room
    /// tone. That case shows up as `voiced` staying at zero — which the analyser
    /// reports as "I could not hear enough voice", rather than as a wrong tone.
    /// The printed peak level tells the two apart from a genuinely quiet attempt.
    ///
    /// ## Calibrating the scoring floor with it
    ///
    /// The tones worth trying are the short ones, because they are what the floor
    /// in `hanzi_core::tone` decides about. Set the character and the hold:
    ///
    /// ```text
    /// HANZI_TUTOR_TONE_TEST=是 HANZI_TUTOR_TONE_SECS=3 \
    ///   cargo test -p hanzi-tutor --lib -- --ignored --nocapture records_from_the_real_microphone
    /// ```
    ///
    /// Then read `voicing` against `floor`, which each round prints as a margin.
    /// `voicing` is the number the analyser compares against the floor, and it is
    /// *not* the length of the syllable: a frame counts only when its whole
    /// analysis window holds a period, so the measured figure runs short of the
    /// sound by the window's edges. What is worth having is margin — if a
    /// comfortable 是 lands a long way above the floor, the floor can come down;
    /// if it lands just above, normal-rate speech will fall under it and
    /// `MIN_VOICED_MS` wants revisiting. **Say it normally, not carefully**: the
    /// whole point is what an ordinary attempt measures. Two or three rounds of the
    /// same word is more useful than one.
    #[test]
    #[ignore = "needs a microphone and permission"]
    fn records_from_the_real_microphone() {
        /// One environment value, trimmed, or a fallback when it is unset.
        fn wished_for(key: &str, fallback: &str) -> String {
            std::env::var(key)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| fallback.to_string())
        }

        /// One line from the terminal.
        ///
        /// `None` at end of input, which is what a piped or CI run looks like.
        /// The caller records once and stops rather than waiting on a stdin that
        /// will never produce anything.
        fn read_line() -> Option<String> {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                // A read error is treated as "no terminal" for the same reason:
                // there is nothing to interact with.
                Ok(0) | Err(_) => None,
                Ok(_) => Some(line),
            }
        }

        let wanted = wished_for("HANZI_TUTOR_TONE_TEST", "是");
        let seconds: f32 = wished_for("HANZI_TUTOR_TONE_SECS", "3")
            .parse()
            .expect("HANZI_TUTOR_TONE_SECS must be a number of seconds");

        let recorder = Recorder::default();
        let status = recorder.status();
        println!("microphone: {status:?}");
        assert!(status.available, "no microphone available: {}", status.detail);

        let state = hanzi_tutor_lib::AppState::load(None).expect("the dataset should load");
        let target = state
            .tone_target(&wanted)
            .unwrap_or_else(|| panic!("{wanted} has no scorable reading"));

        let floor_ms = hanzi_core::tone::MIN_VOICED_MS;
        let silence_gate = hanzi_core::tone::PitchConfig::default().silence_rms;

        println!();
        println!("Scoring {wanted}. Nothing is recorded until you press Enter, and each");
        println!("round says RECORDING before it listens. Enter again to repeat, q to stop.");

        let mut interactive = true;
        let mut rounds = 0usize;
        let mut empty = 0usize;

        loop {
            if interactive {
                println!();
                // `println!`, never `print!`, for this prompt. Rust's stdout is
                // line-buffered, so a prompt written without a newline sits in the
                // buffer while `read_line` blocks — the screen shows nothing and
                // the test looks hung. That is the whole failure this test exists
                // to avoid repeating.
                println!(
                    "--- press Enter to record {seconds} s of {wanted}, \
                     or q then Enter to finish ---"
                );
                match read_line() {
                    // No terminal to read from. Record once and stop.
                    None => interactive = false,
                    Some(line) if line.trim().eq_ignore_ascii_case("q") => break,
                    Some(_) => {}
                }
            }

            // The device is opened *before* the prompt to speak rather than after
            // it, so nobody is talking into a microphone that is still opening.
            // That is the order the app's own push-to-talk button uses, and the
            // reason it does not lose the beginning of a syllable.
            if let Err(problem) = recorder.start() {
                println!("could not open the microphone: {problem}");
                break;
            }
            println!(">>> RECORDING — say {wanted} now <<<");
            std::thread::sleep(Duration::from_secs_f32(seconds));
            let recording = recorder.stop().expect("stopping should succeed");
            println!(">>> done <<<");

            rounds += 1;
            println!(
                "captured {} samples at {} Hz from {} (truncated: {})",
                recording.samples.len(),
                recording.sample_rate,
                recording.device,
                recording.truncated
            );
            if recording.samples.is_empty() {
                empty += 1;
                println!("  the stream ran but delivered nothing at all");
            }

            let loudest = recording
                .samples
                .iter()
                .fold(0.0f32, |peak, s| peak.max(s.abs()));
            println!("peak level: {loudest:.4} (the analyser's silence gate is {silence_gate:.4})");
            if loudest < silence_gate {
                println!(
                    "  quieter than that gate, so every frame is unvoiced before the pitch \
                     estimator runs at all. Either nothing was said, or the input is muted — a \
                     terminal that has never been granted the microphone produces exactly this."
                );
            }

            // The instrument reading behind the verdict, which the report itself
            // has no room for: the geometry of the track, the floor it is judged
            // against, and how much voice there was to judge.
            let resampled = hanzi_core::tone::resample(
                &recording.samples,
                recording.sample_rate,
                hanzi_core::tone::TARGET_SAMPLE_RATE,
            );
            let track = hanzi_core::tone::track_pitch(
                &resampled,
                hanzi_core::tone::TARGET_SAMPLE_RATE,
                &hanzi_core::tone::PitchConfig::default(),
            );
            let hop_ms = track.hop as f32 * 1000.0 / track.sample_rate as f32;
            println!(
                "track: {} frames, {} voiced, hop {:.2} ms, window {:.2} ms",
                track.frames.len(),
                track.frames.iter().filter(|f| f.voiced).count(),
                hop_ms,
                hop_ms * 8.0,
            );
            let floor_frames = hanzi_core::tone::min_voiced_frames(&track);
            println!("floor: {floor_frames} frames at this hop ({floor_ms} ms)");

            // The measurement has to come from the track, not from the report.
            //
            // `ToneReport` zeroes `voiced_ms` and `span_ms` when it refuses a
            // syllable, so printing the report's figures here said "voicing 0 ms"
            // about a recording the line above had just counted ten voiced frames
            // in. That is the one number this test exists to print, and it was
            // being read from the wrong place. `contour_in` is exactly what the
            // scorer's floor is applied to, so it is the honest source.
            //
            // For a word this is the *whole utterance* over one span, which is not
            // the figure that decides anything — each syllable is gated on its own
            // segment. It is printed as a sanity check, and the per-syllable
            // numbers below are the ones to read.
            let whole = if target.syllables.len() == 1 {
                "the syllable".to_string()
            } else {
                format!("the whole word ({} syllables)", target.syllables.len())
            };
            let measured = hanzi_core::tone::voiced_span(&track)
                .and_then(|(first, last)| hanzi_core::tone::contour_in(&track, first, last));
            match measured {
                Some(contour) => {
                    let margin = contour.voiced_ms as i64 - floor_ms as i64;
                    println!(
                        "whole span: {whole} — {} ms of voicing in a {} ms span, median {:.0} Hz \
                         (margin {margin:+} ms)",
                        contour.voiced_ms, contour.span_ms, contour.median_hz
                    );
                }
                None => {
                    let gated = track
                        .frames
                        .iter()
                        .filter(|f| f.voiced && f.hz > 0.0)
                        .count();
                    println!(
                        "whole span: {whole} — refused, {gated} voiced frames against \
                         {floor_frames} needed"
                    );
                }
            }

            let result = state.score_tones(&recording, &target);
            println!(
                "verdict {:?}, score {:.1}: {}",
                result.verdict, result.score, result.detail
            );

            // Per syllable, because for a word the whole-utterance figures above
            // are not what decides anything: each syllable is gated on its own
            // segment, and it is the *middle* one that a learner reports going
            // unrecognised. Printing only the verdict would say a word failed
            // without saying which syllable was short of voice, which is the one
            // thing worth knowing.
            let expected = target.syllables.len();
            if result.boundaries_ms.is_empty() && expected == 1 {
                println!("split: none — one syllable");
            } else if result.boundaries_ms.is_empty() {
                // Empty here does not mean "no split was needed": the report is
                // only built with boundaries when the span was long enough to
                // divide, and a refusal leaves this empty however many syllables
                // were asked for. Saying "one syllable" about 是不是 would be a
                // plain lie.
                println!("split: never attempted — the recording was refused before division");
            } else {
                println!(
                    "split after: {} ms (from the start of speech)",
                    result
                        .boundaries_ms
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            for syllable in &result.syllables {
                println!(
                    "  {}. {} ({}, tone {}): {:?} {:.1} — voicing {} ms in a {} ms span, \
                     median {:.0} Hz, range {:.1} st",
                    syllable.position,
                    syllable.ch,
                    syllable.reading,
                    syllable.spoken,
                    syllable.attempt.verdict,
                    syllable.attempt.score,
                    syllable.attempt.voiced_ms,
                    syllable.attempt.span_ms,
                    syllable.attempt.median_hz,
                    syllable.attempt.range_semitones,
                );
                println!("     {}", syllable.attempt.detail);
            }

            // Is a big `range` a real pitch movement, or the track jumping?
            //
            // A hand test on 中国人 reported segments of 188 and 306 ms with ranges
            // of 26.7 and 27.8 semitones — more than two octaves, which no voice
            // does in a fifth of a second, and at durations that are one ordinary
            // syllable. That is a pitch *tracker* question, and the report cannot
            // answer it because it only carries the finished contour.
            //
            // So the raw frames are walked per segment, looking for the step a
            // voice cannot make: adjacent frames are one hop apart, about 3 ms, so
            // the ratio between them is near 1 or the estimator has jumped — most
            // often to half the frequency, which is creak at the end of a syllable
            // and shows up as a whole octave in the range.
            if let Some((first_frame, last_frame)) = hanzi_core::tone::voiced_span(&track) {
                let mut edges = vec![first_frame];
                for boundary in &result.boundaries_ms {
                    edges.push(first_frame + (*boundary as f32 / hop_ms).round() as usize);
                }
                edges.push(last_frame + 1);

                for (index, pair) in edges.windows(2).enumerate() {
                    let from = pair[0].min(track.frames.len());
                    let to = pair[1].min(track.frames.len()).max(from);
                    let mut jumps = 0usize;
                    let mut previous: Option<f32> = None;
                    let mut lowest = f32::MAX;
                    let mut highest = 0.0f32;
                    for frame in &track.frames[from..to] {
                        if !frame.voiced || frame.hz <= 0.0 {
                            continue;
                        }
                        lowest = lowest.min(frame.hz);
                        highest = highest.max(frame.hz);
                        if let Some(p) = previous {
                            if !(0.75..=1.33).contains(&(frame.hz / p)) {
                                jumps += 1;
                            }
                        }
                        previous = Some(frame.hz);
                    }
                    if previous.is_none() {
                        continue;
                    }
                    println!(
                        "     segment {}: {lowest:.0}–{highest:.0} Hz, {jumps} tracking jump(s)",
                        index + 1
                    );
                }
            }

            if !interactive {
                break;
            }
        }

        // The backend check the original assertion existed for: a stream that runs
        // and produces nothing is a broken capture path. A *transient* empty round
        // is not — a learner who pressed Enter and said nothing gets another go —
        // so this only fails when every round was empty.
        if rounds > 0 {
            println!();
            println!("finished after {rounds} round(s).");
            assert!(
                empty < rounds,
                "the stream ran but produced no samples at all, every time"
            );
        }
    }
}
