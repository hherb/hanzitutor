//! Nihongo Tutor's webview-facing surface.
//!
//! The shape is the same as the other two apps': a thin `#[tauri::command]` layer
//! over methods on [`AppState`], so the whole interface can be driven and
//! asserted on from a test without opening a window. `tests/ipc_contract.rs`
//! does exactly that, and it is what stops the JSON the interface reads from
//! drifting away from the JSON this returns.
//!
//! Everything the app teaches is **embedded**: the kana and kanji artifacts are
//! compiled into the binary with `include_bytes!`. There is no network path in
//! this crate at all — no download, no model, no sync — which is why it has no
//! plugin permissions in `capabilities/default.json`. **Pronunciation does not
//! change that**: the voice is the operating system's own, spoken in process, and
//! nothing is fetched to speak a kana.

// The two that share a name with a command are aliased, so that `fn lessons`
// below is the command and `build_lessons` is the course it serves.
pub mod licences;
pub mod store;

use hanzi_voice::{Language, Speaker};
use licences::{AppInfo, LicenceNotice};
use nihongo_core::{
    band_name, confusions_for, grade_kana, grade_name,
    kanji_lessons as build_kanji_lessons, key_of, lessons as build_lessons,
    normalise_to_hiragana, now_iso8601, off_grid, parse_decomposition,
    queue as build_review_queue, reading, split_key, to_kana, to_kana_in, yoon as build_yoon,
    Confusable, ConfusionLog, Decomposition, DrillKind, DrillPair, DueItem, GradeOptions,
    GradeReport, KanaDataset, Kanji, KanjiDataset, Passage, PassageDataset, PassageToken, Phrase,
    PhraseDataset, Point, Row, Ruby, Script, Section, Word, WordDataset, KANJI_LESSON_SIZE, ROWS,
    VOWEL_COLUMNS,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use store::{ConfusionStore, Prefs, PrefsStore, ReviewStore};
use tauri::{Manager, State};

/// The datasets, loaded once and shared by every command, the drill's memory of
/// what this learner gets wrong, the schedule of what comes back, how the
/// interface was left, and the voice.
///
/// All five artifacts are **embedded** — `include_bytes!`, not read from a path —
/// so there is no data directory to find, no file to go missing, and no filesystem
/// permission for the webview to hold. The kanji artifact is the largest at 3.2 MB
/// and carries the course, the radical table and the geometry the board draws a
/// character from; every byte of it is served by a command now.
pub struct AppState {
    kana: KanaDataset,
    kanji: KanjiDataset,
    words: WordDataset,
    passages: PassageDataset,
    /// The graded phrases: sentences imported from Tatoeba, segmented and levelled
    /// at build time like the passages and held to the same vocabulary. The
    /// segmentation, the reading over every kanji and the band are all in the
    /// artifact, so the app renders them and nothing here tokenises anything.
    phrases: PhraseDataset,
    /// Behind a `Mutex` because Tauri hands every command a shared `&AppState`
    /// and recording an answer is a write. Contention is nil: one learner, one
    /// window, and a lock held for the microseconds a JSON write takes.
    drill: Mutex<ConfusionStore>,
    /// The review schedule, in this app's own file beside the drill's — never a
    /// shared one. See [`store`] and `HANDOVER_NIHONGO.md` invariant 15.
    review: Mutex<ReviewStore>,
    /// Which half of the app is open, in its own file beside the other two. A
    /// preference must never be able to corrupt a schedule, which is why it is a
    /// third file rather than a field in `review.json`.
    prefs: Mutex<PrefsStore>,
    /// The system synthesiser, told once and for all that this app speaks
    /// **Japanese**. Behind an `Arc` because the warm-up thread outlives the
    /// closure that builds the state, and because every command that speaks shares
    /// one voice rather than racing to resolve a list of its own.
    ///
    /// No learner data is here and none is written: the voice is the machine's,
    /// and which one is in use is the machine's answer. That is the opposite of
    /// the three stores above, which are this app's own files — see
    /// `HANDOVER_NIHONGO.md` invariant 15.
    speaker: Arc<Speaker>,
}

impl AppState {
    /// Load the committed artifact, with the learner's own records kept in memory.
    ///
    /// This is what a test wants and what the app falls back to when the platform
    /// will not name a data directory.
    pub fn load() -> Self {
        Self::with_stores(
            ConfusionStore::in_memory(),
            ReviewStore::in_memory(),
            PrefsStore::in_memory(),
        )
    }

    /// Load the committed artifact and the learner's records from `dir`.
    pub fn load_at(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self::with_stores(
            ConfusionStore::at_dir(&dir),
            ReviewStore::at_dir(&dir),
            PrefsStore::at_dir(&dir),
        )
    }

    fn with_stores(drill: ConfusionStore, review: ReviewStore, prefs: PrefsStore) -> Self {
        macro_rules! artifact {
            ($name:literal) => {
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../crates/nihongo-core/data/",
                    $name
                ))
            };
        }
        Self {
            kana: KanaDataset::from_gzip_bytes(artifact!("kana.bin.gz"))
                .expect("the committed kana artifact decodes"),
            kanji: KanjiDataset::from_gzip_bytes(artifact!("kanji.bin.gz"))
                .expect("the committed kanji artifact decodes"),
            words: WordDataset::from_gzip_bytes(artifact!("words.bin.gz"))
                .expect("the committed words artifact decodes"),
            passages: PassageDataset::from_gzip_bytes(artifact!("passages.bin.gz"))
                .expect("the committed passages artifact decodes"),
            phrases: PhraseDataset::from_gzip_bytes(artifact!("phrases.bin.gz"))
                .expect("the committed phrases artifact decodes"),
            drill: Mutex::new(drill),
            review: Mutex::new(review),
            prefs: Mutex::new(prefs),
            speaker: Arc::new(Speaker::new(Language::Japanese)),
        }
    }

    /// Which half of the app to open, and where the learner left it.
    pub fn prefs(&self) -> Prefs {
        self.prefs
            .lock()
            .expect("the preference store is not poisoned")
            .prefs()
    }

    /// Remember the half of the app the learner moved to.
    ///
    /// The answer comes back as a warning rather than an error, exactly as a
    /// graded attempt's write failure does: the learner did move, this session
    /// knows where they are, and what was lost is only that the next start will
    /// not.
    pub fn set_section(&self, section: Section) -> Option<String> {
        self.prefs
            .lock()
            .expect("the preference store is not poisoned")
            .set_section(section)
            .err()
            .map(|err| format!("the section could not be remembered: {err}"))
    }

    /// The speaker, shared rather than borrowed.
    ///
    /// An `Arc` clone because the warm-up thread outlives the `setup` closure
    /// that builds the state, while `AppState` itself belongs to Tauri from the
    /// moment it is managed.
    pub fn speaker_handle(&self) -> Arc<Speaker> {
        Arc::clone(&self.speaker)
    }

    /// Speak `text` with the system's Japanese voice.
    ///
    /// Thin on purpose: the text is whatever the caller was looking at — a kana,
    /// or a word's own reading — and no reading is composed here. Returning before
    /// the sound finishes is [`Speaker::speak`]'s own contract, so the button
    /// never blocks on the utterance.
    pub fn speak(&self, text: &str) -> Result<(), String> {
        self.speaker.speak(text)
    }

    /// Cut off whatever is being said.
    pub fn stop_speaking(&self) {
        self.speaker.stop();
    }

    /// The voice pronunciation will use, or `None` when the machine has no
    /// Japanese voice installed.
    ///
    /// A description rather than a voice: the interface needs to know whether a
    /// "Hear it" button can do anything, and, when it cannot, that the reason is
    /// a missing system voice rather than a broken app. There is deliberately no
    /// voice-*choosing* screen — one language, one automatic choice — which is the
    /// same decision `apps/tone-trainer` records.
    pub fn voice_status(&self) -> Option<String> {
        self.speaker.status()
    }

    pub fn dataset(&self) -> &KanaDataset {
        &self.kana
    }

    /// The kanji, and the 214-radical table they are classified under.
    pub fn kanji_dataset(&self) -> &KanjiDataset {
        &self.kanji
    }

    /// The learner's record, for a caller that wants to show or assert on it.
    pub fn log(&self) -> ConfusionLog {
        self.drill
            .lock()
            .expect("the drill's store is not poisoned")
            .log()
            .clone()
    }

    /// What is in the course, in numbers.
    pub fn stats(&self) -> DatasetStats {
        let hiragana = self.kana.of_script(Script::Hiragana).count();
        let katakana = self.kana.of_script(Script::Katakana).count();
        let lesson_count = build_lessons(&self.kana, Script::Hiragana).len()
            + build_lessons(&self.kana, Script::Katakana).len();
        let kanji_lessons = build_kanji_lessons(&self.kanji, KANJI_LESSON_SIZE).len();
        DatasetStats {
            kana: self.kana.len(),
            hiragana,
            katakana,
            lessons: lesson_count,
            words: self.words.len(),
            passages: self.passages.len(),
            phrases: self.phrases.len(),
            strokes: self
                .kana
                .kana()
                .iter()
                .map(|k| k.stroke_count as usize)
                .sum(),
            kanji: self.kanji.len(),
            kyoiku: self
                .kanji
                .kanji()
                .iter()
                .filter(|k| k.is_kyoiku())
                .count(),
            radicals: self.kanji.radicals().len(),
            kanji_lessons,
        }
    }

    /// The course for one script, in teaching order.
    pub fn lessons(&self, script: Script) -> Vec<LessonView> {
        build_lessons(&self.kana, script)
            .into_iter()
            .map(|lesson| LessonView {
                count: lesson.kana.len(),
                key: lesson.key,
                title: lesson.title,
                kana: lesson.kana,
                voiced: lesson.voiced,
            })
            .collect()
    }

    /// The whole kana chart for one script: the gojūon grid row by row, with the
    /// holes each row has, and the groups that are not on the grid at all.
    ///
    /// The grid is `ROWS` — the same table the course is built from — and the
    /// off-grid groups come from the same function the course's own extra lessons
    /// do, so the chart cannot show a kana the course does not teach or miss one
    /// it does.
    pub fn chart(&self, script: Script) -> ChartView {
        ChartView {
            // Echoed back so a screen can tell an answer to its own question from
            // one about the script that was up a moment ago.
            script: script.name().to_string(),
            vowels: VOWEL_COLUMNS.iter().map(char::to_string).collect(),
            rows: ROWS
                .iter()
                .map(|row: &Row| ChartRowView {
                    sound: row.sound.to_string(),
                    voiced: row.voiced,
                    cells: row
                        .scripted_cells(script)
                        .iter()
                        .map(|cell| cell.map(|ch| ch.to_string()))
                        .collect(),
                })
                .collect(),
            off_grid: off_grid(&self.kana, script)
                .into_iter()
                .map(|lesson| ChartGroupView {
                    key: lesson.key,
                    title: lesson.title,
                    kana: lesson.kana.iter().map(char::to_string).collect(),
                })
                .collect(),
        }
    }

    /// One kana, with everything a practice screen needs: the geometry it draws
    /// and grades against, how it is read, and what it is confused with.
    pub fn kana(&self, ch: char) -> Result<KanaView, String> {
        let kana = self
            .kana
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not in the kana set", ch as u32))?;
        let reading = reading(ch).ok_or_else(|| format!("{ch} has no reading"))?;
        Ok(KanaView {
            ch: kana.ch,
            script: kana.script.name().to_string(),
            stroke_count: kana.stroke_count,
            practisable: kana.is_practisable(),
            romaji: reading.spellings().map(str::to_string).collect(),
            hepburn: reading.hepburn.first().copied().unwrap_or_default().to_string(),
            silent: reading.is_silent(),
            outlines: kana.outlines.clone(),
            medians: kana.medians.clone(),
            confusions: self.confusions(ch),
        })
    }

    /// Grade a handwritten attempt against whatever the app can draw.
    ///
    /// Three things can be written on the board, and they share the one engine:
    /// a **kana**, a **jōyō kanji**, and a **radical head form** — 手 is all three
    /// lists' business, but 亅 is only a radical, and a learner who is shown it
    /// should be able to write it. The lookup is in that order and the geometry
    /// comes from whichever holds the character; `hanzi-core`'s grader never looks
    /// at what the character *is*, which is why one command can serve all three.
    ///
    /// **A kana goes through [`nihongo_core::grade_kana`]**, which accepts a hand
    /// that joined adjacent strokes — さ drawn in two, き in three — and the
    /// answer says which strokes were joined (see [`GradedCharacter::joined`]).
    /// **A kanji and a radical do not**: they keep the strict taught stroke count
    /// and order and go to the shared engine directly, so a kanji drawn with two
    /// strokes joined is refused with the taught count. Combining strokes is
    /// accepted for kana only, by decision — kanji stroke order is the thing the
    /// course teaches. See `nihongo_core::variants`.
    pub fn grade(
        &self,
        ch: char,
        strokes: &[Vec<Point>],
        options: &GradeOptions,
    ) -> Result<GradedCharacter, String> {
        if let Some(kana) = self.kana.get(ch) {
            if !kana.is_practisable() {
                return Err(format!("{ch} has no stroke geometry to grade against"));
            }
            let graded = grade_kana(kana, strokes, options);
            return Ok(GradedCharacter {
                report: graded.report,
                joined: graded.joined,
            });
        }
        if let Some(kanji) = self.kanji.get(ch) {
            if !kanji.is_practisable() {
                return Err(format!("{ch} has no stroke geometry to grade against"));
            }
            return Ok(GradedCharacter {
                report: nihongo_core::grade_with_outlines(
                    kanji.reference_medians(),
                    &kanji.outlines,
                    strokes,
                    options,
                ),
                joined: Vec::new(),
            });
        }
        if let Some(radical) = self.kanji.radicals().iter().find(|r| r.ch == ch) {
            if !radical.is_practisable() {
                return Err(format!("{ch} has no stroke geometry to grade against"));
            }
            return Ok(GradedCharacter {
                report: nihongo_core::grade_with_outlines(
                    &radical.medians,
                    &radical.outlines,
                    strokes,
                    options,
                ),
                joined: Vec::new(),
            });
        }
        Err(format!(
            "{ch} (U+{:04X}) is not a kana, a jōyō kanji, or a radical, so there is no stroke \
             order to grade against",
            ch as u32
        ))
    }

    /// Grade a handwritten attempt **and offer it to the review schedule**.
    ///
    /// This is what the interface calls. Grading alone is [`Self::grade`], which
    /// stays pure so a test can grade without a learner's file being involved.
    ///
    /// The schedule is only advanced when the attempt is a *review*: the character
    /// is new, or its due date has passed. Writing あ five times in one sitting
    /// must not multiply SM-2's interval five times, so a later attempt inside the
    /// interval is graded and not rescheduled — see [`store::ReviewStore`].
    pub fn grade_and_schedule(
        &self,
        ch: char,
        strokes: &[Vec<Point>],
        options: &GradeOptions,
    ) -> Result<GradedAttempt, String> {
        let GradedCharacter { report, joined } = self.grade(ch, strokes, options)?;
        let mut store = self
            .review
            .lock()
            .expect("the review store is not poisoned");
        // A schedule that will not be written is a standing condition, not a
        // per-attempt one; it is said once and repeated on every grade rather than
        // hidden, because the learner is losing work either way.
        let session = store.warning().map(str::to_string);
        let (scheduled, next_due, warning) = match store.record_attempt(ch, report.overall) {
            Ok(outcome) => (
                outcome.counted,
                Some(outcome.due),
                outcome.warning.or(session),
            ),
            // The attempt could not be counted at all. The grade still stands: a
            // verdict the learner can read is worth more than an error that hides
            // it, and the score they just earned did happen.
            Err(err) => (false, None, Some(err).or(session)),
        };
        Ok(GradedAttempt {
            report,
            joined,
            scheduled,
            next_due,
            warning,
        })
    }

    /// What the board has taught and what is due, in one half of the app.
    ///
    /// `limit` caps the items returned, not the count: a learner with two hundred
    /// due characters is told there are two hundred, and shown the most overdue
    /// ones. Taking only the first few is a decision for the screen, so the full
    /// number travels beside them.
    ///
    /// `section` is which course is asking. The schedule behind it is one file and
    /// one set of cards (invariant 15) — the split is in what a screen offers, not
    /// in what is stored — so the kana screen is not shown a due kanji and the
    /// character screen carries the head forms with it. `None` is the whole
    /// schedule, for a caller that is not one of the two screens.
    pub fn review_queue(&self, limit: usize, section: Option<Section>) -> ReviewQueueView {
        let store = self
            .review
            .lock()
            .expect("the review store is not poisoned");
        let now = now_iso8601();
        let queue = build_review_queue(store.store(), &self.kana, &self.kanji, &now, section);
        ReviewQueueView {
            items: queue.items.into_iter().take(limit).collect(),
            cards: queue.cards,
            due: queue.due,
            next_due: queue.next_due,
            warning: store.warning().map(str::to_string),
        }
    }

    /// The kana a given kana is confused with, from the classic set.
    pub fn confusions(&self, ch: char) -> Vec<ConfusionView> {
        confusions_for(&self.kana, ch)
            .into_iter()
            .map(|pair: Confusable| {
                let other = if pair.a == ch { pair.b } else { pair.a };
                ConfusionView {
                    ch: other,
                    tell: pair.tell.to_string(),
                }
            })
            .collect()
    }

    /// The pairs one exercise can actually ask: every spelling written with kana
    /// the app holds, and every prompt non-empty.
    ///
    /// A pair that fails this is skipped rather than asked with an empty prompt or
    /// a character the board cannot draw, which is why the pool is filtered here
    /// instead of being assumed complete. It is also what the app validates an
    /// answer against, so a key that is not askable is a key that is not recorded.
    pub fn askable_pairs(&self, kind: DrillKind) -> Vec<DrillPair> {
        kind.pairs()
            .into_iter()
            .filter(|pair| self.pair_is_askable(pair))
            .collect()
    }

    /// The pair the drill should ask about next, weighted towards the ones this
    /// learner gets wrong.
    ///
    /// Two rolls, because two independent things are being decided: which pair,
    /// and which of the pair is the one being asked for. Both are supplied rather
    /// than drawn here so that the whole decision is deterministic in a test; the
    /// command above passes two rolls from the clock.
    pub fn next_drill_question_with_rolls(
        &self,
        kind: DrillKind,
        pair_roll: f64,
        side_roll: f64,
    ) -> Option<DrillQuestion> {
        let askable = self.askable_pairs(kind);
        let log = self
            .drill
            .lock()
            .expect("the drill's store is not poisoned");
        let pair = log.log().pick_by(&askable, DrillPair::key, pair_roll)?.clone();
        drop(log);

        // Which way round to ask. Both sides are askable, so this cannot fail; it
        // returns the question rather than panicking all the same, because a kana
        // dataset is data and data is allowed to be wrong.
        let [first, second] = &pair.sides;
        let (target, other) = if side_roll < 0.5 { (first, second) } else { (second, first) };

        Some(DrillQuestion {
            kind: kind.name().to_string(),
            pair: pair.key(),
            ch: target.spelling.clone(),
            hepburn: target.prompt.clone(),
            // Exactly the pair's own two spellings, so that every answer says
            // something unambiguous about *this* pair: which of these two shapes
            // is the reading. The wider four-option question the drill used to ask
            // tested more at once and taught less — a miss could not be attributed
            // to a pair, which is precisely what has to be remembered.
            options: vec![target.spelling.clone(), other.spelling.clone()],
            tell: pair.tell.clone(),
        })
    }

    /// The next question, with the randomness supplied by the clock.
    pub fn next_drill_question(&self, kind: DrillKind) -> Option<DrillQuestion> {
        self.next_drill_question_with_rolls(kind, roll(), roll())
    }

    /// Record what the learner answered, and say what the pair's record now is.
    ///
    /// The caller says which pair it was asked about, which spelling was wanted
    /// and which was picked; **correctness is decided here**, not sent by the
    /// interface. A client that could post `correct: true` would be a client that
    /// could lie to itself, and the file is meant to be worth reading.
    ///
    /// The pair is looked up in the pools the app can actually ask rather than in
    /// a list of its own, so a key that names a pair the learner could not have
    /// been asked about is refused instead of written into the file for good.
    pub fn record_drill_answer(
        &self,
        pair: &str,
        target: &str,
        picked: &str,
    ) -> Result<DrillTally, String> {
        // The caller may name the pair either way round — the file is written in
        // one canonical order, and a caller should not have to know which — so
        // the key is canonicalised before it is looked up. A key that is not two
        // spellings between one bar stays as it is and matches nothing, which is
        // how it is refused.
        let wanted = match split_key(pair) {
            Some((a, b)) => key_of(&a, &b),
            None => pair.to_string(),
        };
        let found = DrillKind::ALL
            .iter()
            .flat_map(|kind| self.askable_pairs(*kind))
            .find(|candidate| candidate.key() == wanted)
            .ok_or_else(|| {
                format!(
                    "{pair:?} is not one of the pairs this drill asks about, so there is nothing \
                     to record"
                )
            })?;
        if !found.holds(target) {
            return Err(format!("{target} is not in {}", found.key()));
        }
        if !found.holds(picked) {
            return Err(format!(
                "{picked} is neither of the two answers {} offers",
                found.key()
            ));
        }
        let key = found.key();
        let tally = self
            .drill
            .lock()
            .expect("the drill's store is not poisoned")
            .record(&key, picked == target)
            .map_err(|err| format!("the answer was counted but could not be saved: {err}"))?;
        Ok(DrillTally {
            pair: key,
            asked: tally.asked,
            correct: tally.correct,
            wrong: tally.wrong,
            weight: tally.weight(),
        })
    }

    /// Whether a pair can be asked at all: every character either spelling is
    /// written with is a kana the app holds, and neither side is prompted with
    /// nothing.
    ///
    /// Checking the *characters* rather than the spelling is what lets one rule
    /// serve a kana and a digraph: きゃ is two characters the board can draw as
    /// two kana, and it is not one character the board cannot.
    fn pair_is_askable(&self, pair: &DrillPair) -> bool {
        pair.sides
            .iter()
            .all(|side| !side.prompt.is_empty() && !side.spelling.is_empty())
            && pair.characters().all(|ch| self.kana.get(ch).is_some())
    }

    /// The yōon digraphs for one script — the pairs that make one mora.
    pub fn yoon(&self, script: Script) -> Vec<YoonView> {
        build_yoon(script)
            .into_iter()
            .map(|y| YoonView {
                key: y.key,
                display: y.display,
                hepburn: y.hepburn,
                kunrei: y.kunrei,
                kana: y.kana,
            })
            .collect()
    }

    /// The kanji course: every grade in teaching order, each sliced into lessons.
    ///
    /// Whole rather than paged, unlike a band of words: this is 2,136 characters
    /// and their lesson keys — tens of kilobytes of JSON — where a band is nearly
    /// five thousand words with readings, glosses and furigana each.
    pub fn kanji_lessons(&self) -> Vec<KanjiLessonView> {
        build_kanji_lessons(&self.kanji, KANJI_LESSON_SIZE)
            .into_iter()
            .map(|lesson| KanjiLessonView {
                key: lesson.key,
                title: lesson.title,
                grade: lesson.grade,
                grade_name: grade_name(lesson.grade).to_string(),
                count: lesson.kanji.len(),
                kanji: lesson.kanji,
            })
            .collect()
    }

    /// One kanji, with everything a screen draws and says about it: the geometry
    /// the board writes, the readings and glosses, the radical, and the IDS
    /// components.
    pub fn kanji(&self, ch: char) -> Result<KanjiView, String> {
        let kanji = self
            .kanji
            .get(ch)
            .ok_or_else(|| format!("{ch} (U+{:04X}) is not one of the jōyō kanji", ch as u32))?;
        Ok(self.kanji_view(kanji))
    }

    fn kanji_view(&self, kanji: &Kanji) -> KanjiView {
        let head = self.kanji.radical(kanji.radical_number);
        KanjiView {
            ch: kanji.ch,
            grade: kanji.grade,
            grade_name: grade_name(kanji.grade).to_string(),
            stroke_count: kanji.stroke_count,
            frequency: kanji.frequency,
            on: kanji.on.clone(),
            kun: kanji.kun.clone(),
            meanings: kanji.meanings.clone(),
            nanori: kanji.nanori.clone(),
            radical: RadicalRefView {
                number: kanji.radical_number,
                // The head form where the table has it, and the shape written
                // inside the character otherwise — which happens only for a
                // dataset built without a table.
                ch: head.map(|r| r.ch).unwrap_or(kanji.radical),
                form: kanji.radical,
                note: kanji.radical_note.clone(),
                stroke_count: head.map(|r| r.stroke_count).unwrap_or(kanji.stroke_count),
                characters: self
                    .kanji
                    .kanji()
                    .iter()
                    .filter(|k| k.radical_number == kanji.radical_number)
                    .count(),
            },
            // The components come from AnimCJK's IDS string, and a component the
            // course does not hold is still named — it is what the character is
            // made of, whether or not the board can write it.
            decomposition: parse_decomposition(&kanji.decomposition, |ch| {
                self.kanji.get(ch).is_some()
            }),
            practisable: kanji.is_practisable(),
            outlines: kanji.outlines.clone(),
            medians: kanji.medians.clone(),
        }
    }

    /// The 214 radicals, each with the characters the course classifies under it.
    ///
    /// All 214, in number order, including the sixteen no jōyō character uses: the
    /// panel sorts them, and a radical that is missing from the list is a fact
    /// about the set that would be invisible.
    pub fn radicals(&self) -> Vec<RadicalFamilyView> {
        self.kanji
            .radical_families()
            .into_iter()
            .map(|family| RadicalFamilyView {
                number: family.number,
                ch: family.ch,
                stroke_count: family.stroke_count,
                characters: family.characters,
            })
            .collect()
    }

    /// One radical, with the geometry the board writes it with and its family.
    pub fn radical(&self, number: u8) -> Result<RadicalView, String> {
        let family = self.kanji.radical_family(number).ok_or_else(|| {
            format!("{number} is not a Kangxi radical; they are numbered 1 to 214")
        })?;
        let radical = self
            .kanji
            .radical(number)
            .expect("the family came from the table");
        Ok(RadicalView {
            number: radical.number,
            ch: radical.ch,
            stroke_count: radical.stroke_count,
            characters: family.characters,
            outlines: radical.outlines.clone(),
            medians: radical.medians.clone(),
        })
    }

    /// The ladder, one entry per band, with the counts a screen shows.
    ///
    /// The names come from `nihongo_core::band_name` rather than from this
    /// interface, because the ladder is *this project's* derivation and the
    /// interface has to say so: there has been no official JLPT list since 2010,
    /// and a band labelled as one would be a lie about the data.
    pub fn word_bands(&self) -> Vec<BandView> {
        self.words
            .band_counts()
            .into_iter()
            .map(|(band, words)| BandView {
                band,
                name: band_name(band).to_string(),
                words,
            })
            .collect()
    }

    /// One page of a band's words, in course order.
    ///
    /// Paged rather than whole because band 7 holds nearly five thousand words and
    /// the interface shows a list: a single payload would be a megabyte of JSON to
    /// draw sixty rows.
    pub fn words_in_band(&self, band: u8, offset: usize, limit: usize) -> WordPage {
        let total = self.words.of_band(band).count();
        let words = self
            .words
            .of_band(band)
            .skip(offset)
            .take(limit.clamp(1, MAX_WORD_PAGE))
            .map(word_view)
            .collect();
        WordPage {
            band,
            total,
            offset,
            words,
        }
    }

    /// One page of the words a character is written in, in course order.
    ///
    /// What a character's card lists, and the other half of "a character arrives
    /// through the words that use it": the vocabulary is ordered by band and then
    /// by EDRDG's frequency, so the first page is the words whose other characters
    /// the course teaches first. Paged for a measured reason — 一 is written in
    /// 223 of the 16,073 words and 人 in 218 — where a card that shipped them all
    /// would draw a list nobody scrolls.
    ///
    /// A jōyō character no word uses — 57 of the 2,136 — answers with an empty
    /// list and a total of nothing, which the card states in words: that is a fact
    /// about the vocabulary, not a hole in the screen. A character **outside** the
    /// jōyō set is an error, because the artifact holds no word it could appear in
    /// (invariant 21), and an empty list there would make a typo look like a gap in
    /// the data.
    pub fn words_of_kanji(
        &self,
        ch: char,
        offset: usize,
        limit: usize,
    ) -> Result<WordsOfKanji, String> {
        if self.kanji.get(ch).is_none() {
            return Err(format!(
                "{ch} (U+{:04X}) is not one of the jōyō kanji",
                ch as u32
            ));
        }
        let words = self
            .words
            .of_kanji(ch)
            .skip(offset)
            .take(limit.clamp(1, MAX_WORD_PAGE))
            .map(word_view)
            .collect();
        Ok(WordsOfKanji {
            ch,
            total: self.words.of_kanji(ch).count(),
            offset,
            words,
        })
    }

    /// One page of the words this course teaches that are **read** `reading`, in
    /// course order.
    ///
    /// What the Start screen argues with, and the argument is the artifact's
    /// rather than the interface's: はし is 橋, 箸 and 端 here, and かみ is five
    /// words, so a learner who has the reading and not the character has several
    /// true answers and no way to choose between them. The readings drawn are the
    /// dictionary's own (invariant 21); this only selects among them, folding
    /// katakana to hiragana so ハシ and はし are one query.
    ///
    /// A reading **in kana** is required, and a query in romaji is a message
    /// rather than an empty page: every reading the artifact holds is kana, so
    /// "hashi" matching nothing would be a typo dressed as a gap in the data —
    /// invariant 13's rule, one command over from `words_of_kanji`'s refusal of a
    /// character the jōyō set does not hold. An empty page is still a legitimate
    /// answer for a kana reading the course does not carry, which the screen
    /// states in words.
    pub fn words_of_reading(
        &self,
        reading: &str,
        offset: usize,
        limit: usize,
    ) -> Result<WordsOfReading, String> {
        if reading.is_empty() || !reading.chars().all(is_kana) {
            return Err(format!(
                "{reading:?} is not a reading; a reading is written in kana"
            ));
        }
        let words = self
            .words
            .of_reading(reading)
            .skip(offset)
            .take(limit.clamp(1, MAX_WORD_PAGE))
            .map(word_view)
            .collect();
        Ok(WordsOfReading {
            reading: reading.to_string(),
            total: self.words.of_reading(reading).count(),
            offset,
            words,
        })
    }

    /// One word, by its text and its reading.
    pub fn word(&self, text: &str, reading: &str) -> Result<WordView, String> {
        self.words
            .find(text, reading)
            .map(word_view)
            .ok_or_else(|| format!("{text} ({reading}) is not in the vocabulary"))
    }

    /// One word, by its text alone.
    ///
    /// What a tapped passage token has: the token carries the word's text, because
    /// that is what the sentence shows, and the token's own reading is the
    /// *surface's* — 行き is read いき while the word is 行く, read いく — so it
    /// cannot identify the entry. Six texts in the vocabulary are read two ways;
    /// this returns the one the course reaches first, which is the earlier band and
    /// the more frequent.
    pub fn word_of_text(&self, text: &str) -> Result<WordView, String> {
        self.words
            .of_text(text)
            .map(word_view)
            .ok_or_else(|| format!("{text} is not in the vocabulary"))
    }

    /// Check a typed reading against a word.
    ///
    /// The word is graded **as a word**: its own reading, taken from the
    /// dictionary, and never assembled from its characters — which is the whole
    /// reason 大人 is おとな and not だいじん. Romaji is accepted, and so is kana
    /// typed directly, because a learner who can already read kana should not have
    /// to transliterate to answer.
    pub fn check_word(&self, text: &str, reading: &str, typed: &str) -> Result<ReadingCheck, String> {
        let word = self
            .words
            .find(text, reading)
            .ok_or_else(|| format!("{text} ({reading}) is not in the vocabulary"))?;

        let typed = typed.trim();
        let produced = if typed.chars().any(is_kana) {
            typed.to_string()
        } else {
            to_kana(typed).unwrap_or_default()
        };
        let correct = !produced.is_empty()
            && normalise_to_hiragana(&produced) == normalise_to_hiragana(&word.reading);
        Ok(ReadingCheck { correct, produced })
    }

    /// The passages, as a list a screen can offer.
    pub fn passages(&self) -> Vec<PassageSummary> {
        self.passages
            .passages()
            .iter()
            .map(|passage| PassageSummary {
                key: passage.key.clone(),
                title: passage.title.clone(),
                gloss: passage.gloss.clone(),
                lines: passage.lines.len(),
                tokens: passage.tokens().count(),
            })
            .collect()
    }

    /// One passage, segmented, with the reading over every kanji.
    pub fn passage(&self, key: &str) -> Result<PassageView, String> {
        self.passages
            .get(key)
            .map(passage_view)
            .ok_or_else(|| format!("{key} is not one of the passages"))
    }

    /// The bands the phrases are levelled into, with the counts a screen shows.
    ///
    /// The same ladder the vocabulary uses, and the same names, because it is the
    /// same derivation: a phrase's band is the band of its hardest word, so a
    /// "kyōiku 3" phrase is one whose hardest word is a kyōiku-3 word. Every band is
    /// present even when the corpus filled none of it, so a screen drawing a chip
    /// per band does not have to know which ones the import happened to reach.
    pub fn phrase_bands(&self) -> Vec<PhraseBandView> {
        self.phrases
            .band_counts()
            .into_iter()
            .map(|(band, phrases)| PhraseBandView {
                band,
                name: band_name(band).to_string(),
                phrases,
            })
            .collect()
    }

    /// One band's phrases, shortest first.
    ///
    /// Not paged, and deliberately: the corpus itself caps each band
    /// (`prepare-phrases --per-band`), so a band is already the size of a list a
    /// screen can draw, and a page of a page would be two bounds to explain.
    pub fn phrases_in_band(&self, band: u8) -> Vec<PhraseView> {
        self.phrases.of_band(band).map(phrase_view).collect()
    }
}

/// The largest page a caller may ask for.
///
/// Bounded here rather than trusted from the interface: `limit` crosses the IPC
/// boundary, and an unbounded one would let a stray number build a payload the
/// window cannot draw.
const MAX_WORD_PAGE: usize = 200;

/// Whether a string is kana, for telling a typed reading from typed romaji.
fn is_kana(ch: char) -> bool {
    matches!(ch as u32, 0x3041..=0x309F | 0x30A1..=0x30FA | 0x30FC)
}

fn word_view(word: &Word) -> WordView {
    WordView {
        text: word.text.clone(),
        reading: word.reading.clone(),
        meaning: word.meaning.clone(),
        band: word.band,
        band_name: band_name(word.band).to_string(),
        nf: word.nf,
        furigana: word.furigana.clone(),
    }
}

fn passage_view(passage: &Passage) -> PassageView {
    PassageView {
        key: passage.key.clone(),
        title: passage.title.clone(),
        gloss: passage.gloss.clone(),
        lines: passage
            .lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|token: &PassageToken| TokenView {
                        surface: token.surface.clone(),
                        rt: token.rt.clone(),
                        word: token.word.clone(),
                    })
                    .collect()
            })
            .collect(),
    }
}

fn phrase_view(phrase: &Phrase) -> PhraseView {
    PhraseView {
        id: phrase.id,
        text: phrase.text.clone(),
        english: phrase.english.clone(),
        author: phrase.author.clone(),
        licence: phrase.licence.clone(),
        tokens: phrase
            .tokens()
            .map(|token| TokenView {
                surface: token.surface.clone(),
                rt: token.rt.clone(),
                word: token.word.clone(),
            })
            .collect(),
    }
}

/// What the course contains.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DatasetStats {
    pub kana: usize,
    pub hiragana: usize,
    pub katakana: usize,
    pub lessons: usize,
    /// How many words the vocabulary holds, so the header can say so without a
    /// second call.
    pub words: usize,
    /// How many reading passages are shipped.
    pub passages: usize,
    /// How many graded phrases are shipped.
    pub phrases: usize,
    pub strokes: usize,
    /// How many jōyō kanji the artifact holds.
    pub kanji: usize,
    /// How many of them are kyōiku — grades 1 to 6.
    pub kyoiku: usize,
    /// How many Kangxi radicals the table holds: always 214.
    pub radicals: usize,
    /// How many lessons the kanji course is sliced into.
    pub kanji_lessons: usize,
}

/// One lesson, as the sidebar lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonView {
    pub key: String,
    pub title: String,
    pub kana: Vec<char>,
    pub voiced: bool,
    pub count: usize,
}

/// One row of the kana chart: the grid's five slots, holes included.
///
/// `cells` is always five long — a, i, u, e, o — and a slot is null where the row
/// has no kana. A shorter list would draw や's three kana left-aligned and put ゆ
/// under い, which is a chart teaching the wrong vowel.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChartRowView {
    /// The row's Hepburn label: `ka`, `ya`, `n`.
    pub sound: String,
    /// True for the dakuten and handakuten rows, which a chart sets apart.
    pub voiced: bool,
    pub cells: Vec<Option<String>>,
}

/// A group of kana the chart draws **beside** the grid: the small kana, the rare
/// ones, and — in katakana only — the v-series and the prolonged sound mark.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChartGroupView {
    /// Stable key, e.g. `hiragana-small`.
    pub key: String,
    /// What the group is called, with its kana in the title as the course writes
    /// them: `Small kana — ゃ ゅ ょ っ`.
    pub title: String,
    pub kana: Vec<String>,
}

/// The whole kana chart for one script.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChartView {
    /// The script this chart is of, echoed back so a screen can tell an answer to
    /// its own question from one about the script that was up a moment ago.
    pub script: String,
    /// The five vowels the columns stand for — `a i u e o` — so a screen can label
    /// them. They are the grid's meaning: it is why ゆ is under う and not under い.
    pub vowels: Vec<String>,
    /// The gojūon grid, plain rows first and then the voiced ones.
    pub rows: Vec<ChartRowView>,
    /// The characters that are not on the grid.
    pub off_grid: Vec<ChartGroupView>,
}

/// One kana, as a practice screen uses it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KanaView {
    pub ch: char,
    pub script: String,
    pub stroke_count: u8,
    pub practisable: bool,
    /// Every spelling a learner might type, Hepburn first.
    pub romaji: Vec<String>,
    /// The one to show as *the* reading.
    pub hepburn: String,
    /// True for っ and ー, which have no sound of their own.
    pub silent: bool,
    /// SVG path data in font space, one per taught stroke, in stroke order.
    pub outlines: Vec<String>,
    /// Centre-lines in display space, for the faint guide and for grading.
    pub medians: Vec<Vec<Point>>,
    pub confusions: Vec<ConfusionView>,
}

/// A kana this one is mistaken for.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfusionView {
    pub ch: char,
    pub tell: String,
}

/// One question for the discrimination drill.
///
/// The pair travels with the question because the answer is recorded against the
/// *pair*, and the asker is the only thing that knows which pair it chose. The
/// component is free to shuffle `options` for display; it must send `pair`,
/// `ch` and the spelling that was picked back unchanged.
///
/// A **spelling** rather than a kana, because a yōon contrast's two answers are
/// two characters each: the question is きゃ against きや, and asking it with one
/// character per answer is not possible.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrillQuestion {
    /// Which exercise asked this — `confusion`, `yoon-hiragana` or
    /// `yoon-katakana`. Echoed back so a screen can tell a question from the
    /// exercise it is showing now from one it left.
    pub kind: String,
    /// The canonical key of the pair under test, e.g. `シ|ツ` or `きゃ|きや`.
    pub pair: String,
    /// The spelling the learner is being asked to recognise.
    pub ch: String,
    /// The reading to prompt with.
    pub hepburn: String,
    /// The spellings to offer as answers, one of which is `ch`. Two, for now: the
    /// pair itself — see `next_drill_question_with_rolls` for why not more.
    pub options: Vec<String>,
    /// What tells the two apart, so a miss teaches as well as records.
    pub tell: String,
}

/// What the learner's record for one pair now is, after an answer.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DrillTally {
    pub pair: String,
    pub asked: u32,
    pub correct: u32,
    pub wrong: u32,
    /// The pair's share of the drill, per the rule in `nihongo_core::drill`.
    pub weight: f64,
}

/// A yōon digraph.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YoonView {
    pub key: String,
    pub display: String,
    pub hepburn: String,
    pub kunrei: String,
    pub kana: Vec<char>,
}

/// One lesson of the kanji course, as the sidebar lists it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KanjiLessonView {
    pub key: String,
    /// Where the lesson sits inside its grade, e.g. `1–10`.
    pub title: String,
    /// KANJIDIC2's grade: 1–6 kyōiku, 8 the jōyō remainder.
    pub grade: u8,
    /// The grade's label, which is the vocabulary ladder's — see
    /// `nihongo_core::grade_name`.
    pub grade_name: String,
    pub count: usize,
    pub kanji: Vec<char>,
}

/// The radical a character is classified under, in both of its shapes.
///
/// `ch` is the head form from the 214-radical table (手) and `form` is the shape
/// written inside the character (扌). They differ for most characters, and the
/// difference is the lesson rather than an inconsistency — see
/// `nihongo_core::Radical`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RadicalRefView {
    pub number: u8,
    pub ch: char,
    pub form: char,
    pub note: Option<String>,
    pub stroke_count: u8,
    /// How many characters in the course are classified under it.
    pub characters: usize,
}

/// One kanji, as the course draws it: the geometry, the readings, the radical and
/// the components.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KanjiView {
    pub ch: char,
    pub grade: u8,
    /// What the screen calls the grade, from `nihongo_core::grade_name`.
    pub grade_name: String,
    pub stroke_count: u8,
    /// KANJIDIC2's frequency rank, or `null` for the characters it does not rank.
    pub frequency: Option<u16>,
    /// On'yomi in katakana, in KANJIDIC2's order.
    pub on: Vec<String>,
    /// Kun'yomi, with KANJIDIC2's okurigana markers intact.
    pub kun: Vec<String>,
    pub meanings: Vec<String>,
    pub nanori: Vec<String>,
    pub radical: RadicalRefView,
    /// The IDS decomposition, parsed: the outermost arrangement in words and the
    /// parts, each marked with whether the board can write it.
    pub decomposition: Decomposition,
    pub practisable: bool,
    pub outlines: Vec<String>,
    pub medians: Vec<Vec<Point>>,
}

/// One radical and the characters that share it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RadicalFamilyView {
    pub number: u8,
    pub ch: char,
    pub stroke_count: u8,
    /// The characters classified under it, most frequent first. Empty for the
    /// sixteen radicals no jōyō character uses.
    pub characters: Vec<char>,
}

/// One radical on its own, with the geometry the board writes it with.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RadicalView {
    pub number: u8,
    pub ch: char,
    pub stroke_count: u8,
    pub characters: Vec<char>,
    pub outlines: Vec<String>,
    pub medians: Vec<Vec<Point>>,
}

/// One band of the vocabulary ladder.
///
/// `name` is the label a screen shows, and it is deliberately not "JLPT n": the
/// bands are derived from the kyōiku grades and EDRDG's frequency ranking, and
/// `nihongo_core::band_name` is where the words a learner sees come from.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BandView {
    pub band: u8,
    pub name: String,
    pub words: usize,
}

/// A page of one band's words.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordPage {
    pub band: u8,
    /// How many words the band holds in all, so a screen can page without
    /// guessing.
    pub total: usize,
    pub offset: usize,
    pub words: Vec<WordView>,
}

/// One page of the words a character is written in, for its card.
///
/// The same page shape as a band's, with the character in place of the band: `ch`
/// is echoed back so a screen can tell an answer about the character it asked for
/// from one about the character that was up a moment ago.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordsOfKanji {
    pub ch: char,
    /// How many words this course teaches that use the character, in all.
    pub total: usize,
    pub offset: usize,
    pub words: Vec<WordView>,
}

/// One page of the words this course teaches that are read `reading`.
///
/// The same page shape as a band's, with the reading in place of the band:
/// `reading` is echoed back so a screen can tell an answer about the reading it
/// asked for from one about the reading that was up a moment ago, and `total` is
/// what lets it say in words that one sound names several words.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordsOfReading {
    pub reading: String,
    /// How many words this course teaches that are read this way, in all.
    pub total: usize,
    pub offset: usize,
    pub words: Vec<WordView>,
}

/// One word, as the vocabulary screen draws it: the word, its own reading, the
/// furigana that puts that reading over the right characters, and its band.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WordView {
    pub text: String,
    /// The whole word's reading, from the dictionary — never composed from the
    /// characters.
    pub reading: String,
    pub meaning: String,
    pub band: u8,
    /// The band's name, so the screen never has to map a number to a claim.
    pub band_name: String,
    /// EDRDG's frequency block, 1–48, or `null` for a word it marks common
    /// without ranking.
    pub nf: Option<u8>,
    /// The furigana, in order. Empty for the words JmdictFurigana does not align.
    pub furigana: Vec<Ruby>,
}

/// One passage, as the reading screen offers it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassageSummary {
    pub key: String,
    pub title: String,
    pub gloss: Option<String>,
    pub lines: usize,
    pub tokens: usize,
}

/// One passage, segmented, with a reading over every kanji.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PassageView {
    pub key: String,
    pub title: String,
    pub gloss: Option<String>,
    pub lines: Vec<Vec<TokenView>>,
}

/// One word of a text a screen draws — a passage line or a phrase.
///
/// One type for both because it is one shape produced by one rule: the passages and
/// the phrases are segmented by the same code in `nihongo_core::segment`, so a
/// "passage token" and a "phrase token" would be two names for the same three
/// fields and a second thing to keep in step.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenView {
    pub surface: String,
    /// The reading to draw over it, in hiragana; `null` for kana, which needs no
    /// ruby.
    pub rt: Option<String>,
    /// The vocabulary word this token is, when it is one, so a tap can open its
    /// card. The dictionary form: 行き links to 行く.
    pub word: Option<String>,
}

/// One band of the phrase ladder, as the Phrases screen offers it.
///
/// The same shape as [`BandView`] with the count of phrases where that one has the
/// count of words — and the same names, because it is the same ladder.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PhraseBandView {
    pub band: u8,
    pub name: String,
    pub phrases: usize,
}

/// One graded phrase, as the Phrases screen draws it.
///
/// `id`, `author` and `licence` are the attribution the corpus's licence requires
/// to travel with the sentence: the id is Tatoeba's, so the original can be found
/// and cited, and the other two are what CC BY 2.0 FR asks to be named. They are
/// sent to the interface rather than kept in the artifact alone so every row can
/// say where it came from.
///
/// The phrase's **band is not here**, and that is deliberate: the screen asks for
/// one band at a time and draws it under the band's own chip, so a band on every
/// row would be the same word repeated down the list. The band is in the artifact,
/// where the artifact test recomputes it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PhraseView {
    /// Tatoeba's sentence id.
    pub id: u32,
    /// The sentence as written, including its final punctuation.
    pub text: String,
    /// The English translation the corpus pairs with it.
    pub english: String,
    /// The contributor the corpus names.
    pub author: String,
    /// The licence the sentence is under.
    pub licence: String,
    pub tokens: Vec<TokenView>,
}

/// The result of checking a typed romaji answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingCheck {
    pub correct: bool,
    /// What the typing actually produced, so a wrong answer can show the learner
    /// what they wrote rather than only that it was wrong.
    pub produced: String,
}

/// The verdict on one handwritten character, and the form it was read as.
///
/// `report` is what the engine has always returned. `joined` is the Japanese
/// half: a kana drawn with adjacent strokes joined — さ in two strokes, き in
/// three — is graded against a reference put into the same grouping, and this
/// names the taught strokes the hand drew as one, 1-based and in taught order
/// (`vec![vec![3, 4]]`). Empty for the taught form, and always empty for a kanji
/// or a radical: they keep the strict taught stroke count and order, by decision.
///
/// The screen needs it for two reasons rather than one. It is the honest answer
/// to "the prompt says four strokes and you drew three", and `report`'s own
/// `refIndex` numbers the *drawn* strokes, so a per-stroke list would otherwise
/// label the third drawn stroke "3" when it is taught strokes 3 and 4.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradedCharacter {
    pub report: GradeReport,
    #[serde(default)]
    pub joined: Vec<Vec<u8>>,
}

/// The verdict on an attempt, and what the review schedule did with it.
///
/// Grading and scheduling are one command because they are one action to the
/// learner: they wrote a character and pressed Grade. The report is unchanged —
/// the same four scores the engine has always returned — and the fields beside
/// it are what the screen needs to say what happened and what happens next.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradedAttempt {
    pub report: GradeReport,
    /// For a kana drawn with strokes joined, the taught strokes the hand drew as
    /// one — see [`GradedCharacter::joined`]. Empty otherwise.
    #[serde(default)]
    pub joined: Vec<Vec<u8>>,
    /// True when this attempt advanced the schedule: the character was new, or its
    /// due date had passed. A later attempt inside the interval is practice — it
    /// is still graded, and the schedule does not move.
    pub scheduled: bool,
    /// When the character comes up next, ISO-8601 UTC. Present whenever a card
    /// exists for the character, whether or not this attempt changed it.
    #[serde(default)]
    pub next_due: Option<String>,
    /// Set when the attempt was counted but could not be written to the learner's
    /// file, or when the schedule could not be opened at all. The screen says so
    /// rather than losing it quietly — see `store.rs`.
    #[serde(default)]
    pub warning: Option<String>,
}

/// The review queue, as one course's Review screen draws it.
///
/// Every count here is the **section's**, not the whole schedule's: a kana screen
/// that said "12 scheduled" while eleven of them were kanji would be describing a
/// queue it cannot show.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewQueueView {
    /// Due characters in the section, most overdue first, capped at the caller's
    /// limit.
    pub items: Vec<DueItem>,
    /// How many characters the section's schedule holds at all, due or not.
    pub cards: usize,
    /// How many are due, which is the length of the *uncapped* queue.
    pub due: usize,
    /// When the next character of this section comes back, when nothing is due now.
    #[serde(default)]
    pub next_due: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
}

fn script_of(name: &str) -> Result<Script, String> {
    Script::from_name(name).ok_or_else(|| format!("unknown script {name:?}; expected hiragana or katakana"))
}

#[tauri::command]
fn app_info() -> AppInfo {
    licences::APP
}

/// Every notice this app owes, so a Licences screen can show them without
/// shipping a second copy of the text in the frontend.
#[tauri::command]
fn licences() -> Vec<LicenceNotice> {
    licence_notices()
}

/// The notice catalogue, for a caller that should not have to open a window.
pub fn licence_notices() -> Vec<LicenceNotice> {
    licences::notices().to_vec()
}

#[tauri::command]
fn dataset_stats(state: State<'_, AppState>) -> DatasetStats {
    state.stats()
}

#[tauri::command]
fn lessons(state: State<'_, AppState>, script: String) -> Result<Vec<LessonView>, String> {
    Ok(state.lessons(script_of(&script)?))
}

#[tauri::command]
fn kana(state: State<'_, AppState>, ch: char) -> Result<KanaView, String> {
    state.kana(ch)
}

/// Grade a handwritten attempt against whatever the app can draw, and schedule
/// when it comes back.
///
/// The report is the same shape it has always been; what changed is that the
/// attempt is also offered to the review schedule, and the response says what
/// that did. `scheduled` is false for a second attempt inside a character's
/// interval, which is the rule that keeps one sitting from stretching an interval
/// by months.
#[tauri::command]
fn grade_attempt(
    state: State<'_, AppState>,
    ch: char,
    strokes: Vec<Vec<Point>>,
    options: Option<GradeOptions>,
) -> Result<GradedAttempt, String> {
    state.grade_and_schedule(ch, &strokes, &options.unwrap_or_default())
}

/// The whole kana chart — the gojūon grid and the characters off it — for one
/// script.
///
/// One answer rather than two commands: the grid and the off-grid groups are what
/// one screen draws together, and asking for them separately would let the two
/// halves describe different scripts.
#[tauri::command]
fn kana_chart(state: State<'_, AppState>, script: String) -> Result<ChartView, String> {
    Ok(state.chart(script_of(&script)?))
}

/// The next question for the discrimination drill, weighted towards the pairs
/// this learner gets wrong. `None` only if no pair can be asked at all.
///
/// `kind` picks the exercise — the classic confusions, or the yōon contrasts of
/// one script — and defaults to the classic pairs, so a caller that predates the
/// yōon drill asks for exactly what it used to get.
#[tauri::command]
fn next_drill_question(
    state: State<'_, AppState>,
    kind: Option<DrillKind>,
) -> Option<DrillQuestion> {
    state.next_drill_question(kind.unwrap_or(DrillKind::Confusion))
}

/// What the board has taught this learner, and what is due, in one section.
///
/// `limit` caps the items returned, not the count: a learner with two hundred due
/// characters is shown the most overdue ones and told how many there are.
/// `section` is which half of the app is asking; an older interface that sends
/// nothing gets the whole schedule rather than a rejection.
#[tauri::command]
fn review_queue(
    state: State<'_, AppState>,
    limit: Option<usize>,
    section: Option<Section>,
) -> ReviewQueueView {
    state.review_queue(limit.unwrap_or(REVIEW_PAGE), section)
}

/// Which half of the app to open on.
#[tauri::command]
fn prefs(state: State<'_, AppState>) -> Prefs {
    state.prefs()
}

/// Remember the half of the app the learner moved to.
///
/// The answer is a warning string rather than a failure, so a preference that
/// cannot be written does not look like a command that did not run.
#[tauri::command]
fn set_section(state: State<'_, AppState>, section: Section) -> Option<String> {
    state.set_section(section)
}

/// How many due characters one page of the review queue carries.
///
/// The queue is a list of single characters, so a page is cheap; a learner with a
/// backlog wants to work through the top of it rather than scroll all of it, and
/// the screen can ask for more.
const REVIEW_PAGE: usize = 40;

/// Record what the learner answered, and return the pair's record now.
///
/// `target` and `picked` are **spellings**, not kana: a yōon question's answers
/// are two characters each.
#[tauri::command]
fn record_drill_answer(
    state: State<'_, AppState>,
    pair: String,
    target: String,
    picked: String,
) -> Result<DrillTally, String> {
    state.record_drill_answer(&pair, &target, &picked)
}

/// Check a typed reading against a kana, accepting either romanisation.
#[tauri::command]
fn check_reading(ch: char, typed: String) -> ReadingCheck {
    let produced = to_kana(&typed).unwrap_or_default();
    ReadingCheck {
        correct: nihongo_core::matches_reading(ch, &typed),
        produced,
    }
}

/// Turn typed romaji into kana — the input half of the course.
#[tauri::command]
fn romaji_to_kana(input: String, script: Option<String>) -> Result<String, String> {
    let script = match script.as_deref() {
        None => Script::Hiragana,
        Some(name) => script_of(name)?,
    };
    to_kana_in(script, &input).map_err(|e| e.to_string())
}

#[tauri::command]
fn yoon(script: String) -> Result<Vec<YoonSummary>, String> {
    Ok(build_yoon(script_of(&script)?)
        .into_iter()
        .map(|y| YoonSummary {
            hepburn: y.hepburn,
            display: y.display,
        })
        .collect())
}

/// The kanji course: every grade in teaching order, sliced into lessons.
#[tauri::command]
fn kanji_lessons(state: State<'_, AppState>) -> Vec<KanjiLessonView> {
    state.kanji_lessons()
}

/// One kanji, with its geometry, readings, radical and components.
#[tauri::command]
fn kanji(state: State<'_, AppState>, ch: char) -> Result<KanjiView, String> {
    state.kanji(ch)
}

/// The 214 Kangxi radicals, each with the characters that share it.
#[tauri::command]
fn radicals(state: State<'_, AppState>) -> Vec<RadicalFamilyView> {
    state.radicals()
}

/// One radical, with the geometry the board writes it with.
#[tauri::command]
fn radical(state: State<'_, AppState>, number: u8) -> Result<RadicalView, String> {
    state.radical(number)
}

/// The vocabulary ladder, one entry per band, with each band's size.
#[tauri::command]
fn word_bands(state: State<'_, AppState>) -> Vec<BandView> {
    state.word_bands()
}

/// One page of a band's words, in course order.
#[tauri::command]
fn words_in_band(state: State<'_, AppState>, band: u8, offset: usize, limit: usize) -> WordPage {
    state.words_in_band(band, offset, limit)
}

/// One page of the words a character is written in, in course order — what its
/// card lists beside the readings and the radical.
///
/// A jōyō character no word uses answers with an empty page; a character outside
/// the jōyō set is a message, because the vocabulary holds no word it could be in.
#[tauri::command]
fn words_of_kanji(
    state: State<'_, AppState>,
    ch: char,
    offset: usize,
    limit: usize,
) -> Result<WordsOfKanji, String> {
    state.words_of_kanji(ch, offset, limit)
}

/// One page of the words this course teaches that are read `reading`, in course
/// order — what the Start screen demonstrates with.
///
/// A reading the course does not carry answers with an empty page and a total of
/// nothing, which the screen states in words; anything that is not kana is a
/// message rather than a silent nothing, so a romanised query does not read as a
/// gap in the vocabulary.
#[tauri::command]
fn words_of_reading(
    state: State<'_, AppState>,
    reading: String,
    offset: usize,
    limit: usize,
) -> Result<WordsOfReading, String> {
    state.words_of_reading(&reading, offset, limit)
}

/// One word, by its text and its reading.
#[tauri::command]
fn word(state: State<'_, AppState>, text: String, reading: String) -> Result<WordView, String> {
    state.word(&text, &reading)
}

/// One word, by its text alone — what a tapped passage token has.
#[tauri::command]
fn word_of_text(state: State<'_, AppState>, text: String) -> Result<WordView, String> {
    state.word_of_text(&text)
}

/// Check a typed reading against a word. The word is graded as a word — its own
/// reading, which is never composed from its characters.
#[tauri::command]
fn check_word(
    state: State<'_, AppState>,
    text: String,
    reading: String,
    typed: String,
) -> Result<ReadingCheck, String> {
    state.check_word(&text, &reading, &typed)
}

/// The passages a learner can read.
#[tauri::command]
fn passages(state: State<'_, AppState>) -> Vec<PassageSummary> {
    state.passages()
}

/// One passage, segmented, with the reading over every kanji.
#[tauri::command]
fn passage(state: State<'_, AppState>, key: String) -> Result<PassageView, String> {
    state.passage(&key)
}

/// The bands the graded phrases are levelled into, with their counts.
#[tauri::command]
fn phrase_bands(state: State<'_, AppState>) -> Vec<PhraseBandView> {
    state.phrase_bands()
}

/// One band's graded phrases, shortest first.
#[tauri::command]
fn phrases_in_band(state: State<'_, AppState>, band: u8) -> Vec<PhraseView> {
    state.phrases_in_band(band)
}

/// Hear `text` in the system's Japanese voice, cutting off anything already being
/// said.
///
/// This is the whole of the app's audio surface: the caller passes what it is
/// showing — a kana, or a word's own stored reading — and never a reading composed
/// here, for invariant 21's reason. Resolves as soon as the synthesiser has
/// started rather than when the sound ends, so the button is never blocked by an
/// utterance.
#[tauri::command]
fn speak(state: State<'_, AppState>, text: String) -> Result<(), String> {
    state.speak(&text)
}

/// Stop the current utterance.
///
/// Separate from [`speak`] because a learner who has heard enough should be able
/// to say so without starting another one, and because cutting off a long word is
/// the one thing "speak" cannot express.
#[tauri::command]
fn stop_speaking(state: State<'_, AppState>) {
    state.stop_speaking();
}

/// The voice pronunciation will use, or `null` when the machine has none.
///
/// The interface asks once at startup so it can disable every "Hear it" button
/// and say why, rather than offering a control that silently does nothing.
#[tauri::command]
fn voice(state: State<'_, AppState>) -> Option<String> {
    state.voice_status()
}

/// Resolve the voice and build the synthesiser before the first tap.
///
/// A thread of its own, and not on the startup path, because enumerating the
/// installed voices takes about a second on macOS and the first screen must not
/// wait for it. Nothing reports a failure: the worst case is that the first
/// utterance pays the cost this exists to move, which is what happened before —
/// `Speaker::prime` is a no-op on the platforms that have nothing to warm, so this
/// is not gated by platform here.
fn warm_voice(speaker: Arc<Speaker>) {
    std::thread::spawn(move || {
        match speaker.status() {
            Some(voice) => eprintln!("[speech] using voice {voice}"),
            None => eprintln!(
                "[speech] no {} voice installed; pronunciation will be unavailable",
                speaker.language().display_name()
            ),
        }
        speaker.prime();
    });
}

/// A yōon digraph in the cut-down form the input helper needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct YoonSummary {
    hepburn: String,
    display: String,
}

/// A roll in `0..1`, from the clock and a counter.
///
/// Not cryptography, and not a game's RNG: it decides which pair to ask, and the
/// only requirements are that two calls in quick succession differ and that the
/// result is spread out rather than stuck at one end. `splitmix64`'s finaliser
/// over `nanoseconds ^ counter` is enough for that, and it is written out here
/// because a dependency for four lines of arithmetic would be the tail wagging
/// the dog — `HANDOVER.md`'s "the app has no network path" is the same instinct.
fn roll() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.subsec_nanos() as u64)
        .unwrap_or(0);
    let mut z = nanos
        ^ COUNTER
            .fetch_add(1, Ordering::Relaxed)
            .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    // 53 bits is the whole of a f64's mantissa, so this is uniform where it can
    // be and never rounds up to exactly 1.0.
    (z >> 11) as f64 / (1u64 << 53) as f64
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // The learner's own file, in the app's own directory. Rust writes it
            // rather than the webview, so the app still needs no filesystem
            // permission in `capabilities/default.json` — the webview has no way
            // to name, read or write a path.
            let state = match app.path().app_data_dir() {
                Ok(dir) => AppState::load_at(dir),
                Err(err) => {
                    eprintln!("no app data directory ({err}); this session will not be remembered");
                    AppState::load()
                }
            };
            // Taken before the state is handed to Tauri, because the warm-up
            // thread outlives this closure and `manage` takes ownership.
            let speaker = state.speaker_handle();
            app.manage(state);
            warm_voice(speaker);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            licences,
            dataset_stats,
            lessons,
            kana,
            kana_chart,
            grade_attempt,
            review_queue,
            prefs,
            set_section,
            next_drill_question,
            record_drill_answer,
            check_reading,
            romaji_to_kana,
            yoon,
            kanji_lessons,
            kanji,
            radicals,
            radical,
            word_bands,
            words_in_band,
            words_of_kanji,
            words_of_reading,
            word,
            word_of_text,
            check_word,
            passages,
            passage,
            phrase_bands,
            phrases_in_band,
            speak,
            stop_speaking,
            voice,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nihongo Tutor");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::load()
    }

    #[test]
    fn the_embedded_dataset_is_the_whole_kana_set() {
        let state = state();
        let stats = state.stats();
        assert_eq!(stats.kana, 177);
        assert_eq!(stats.hiragana, 86);
        assert_eq!(stats.katakana, 91);
        assert_eq!(stats.strokes, 516);
        // Eleven plain rows, five voiced rows, then small, rare, and — in
        // katakana only — the v-series and the prolonged sound mark.
        assert_eq!(state.lessons(Script::Hiragana).len(), 18);
        assert_eq!(state.lessons(Script::Katakana).len(), 20);
        assert_eq!(stats.lessons, 38);
    }

    /// The kanji artifact, read the way the course reads it: the jōyō set, its
    /// grades, and the 214-radical table behind the panels.
    #[test]
    fn the_embedded_kanji_artifact_is_the_joyo_set_with_its_radical_table() {
        let stats = state().stats();
        assert_eq!(stats.kanji, 2_136);
        assert_eq!(stats.kyoiku, 1_026, "grades 1 to 6");
        assert_eq!(stats.radicals, 214);
        assert_eq!(stats.kanji_lessons, 216, "2,136 characters in tens, per grade");
    }

    #[test]
    fn a_kanji_view_carries_the_geometry_the_readings_and_the_structure() {
        let view = state().kanji('学').expect("学 is jōyō");
        assert_eq!(view.ch, '学');
        assert_eq!(view.grade, 1);
        assert_eq!(view.grade_name, "kyōiku 1");
        assert_eq!(view.stroke_count, 8);
        assert!(view.practisable);
        assert_eq!(view.outlines.len(), 8);
        assert_eq!(view.medians.len(), 8);
        assert_eq!(view.on, vec!["ガク".to_string()]);
        assert_eq!(view.kun, vec!["まな.ぶ".to_string()]);
        assert!(!view.meanings.is_empty());

        // The radical in both of its shapes: 学 writes 子, and 子 is radical 39.
        assert_eq!(view.radical.number, 39);
        assert_eq!(view.radical.ch, '子');
        assert_eq!(view.radical.form, '子');
        assert_eq!(view.radical.note, None);
        assert_eq!(view.radical.stroke_count, 3);
        assert_eq!(view.radical.characters, 9, "the characters classified under 子");

        // And its components, from AnimCJK's IDS string.
        assert_eq!(view.decomposition.raw, "⿳𰃮子");
        assert_eq!(view.decomposition.layout, "above, middle and below");
        let parts: Vec<(Option<char>, bool)> = view
            .decomposition
            .parts
            .iter()
            .map(|part| (part.ch, part.drawable))
            .collect();
        assert!(
            parts.contains(&(Some('子'), true)),
            "子 is a course character, so it can be written on its own: {parts:?}"
        );
        assert!(
            parts.iter().any(|(ch, drawable)| *ch != Some('子') && !*drawable),
            "a component the course does not hold is still named, and is not drawable: {parts:?}"
        );
    }

    #[test]
    fn a_radical_is_both_shapes_of_the_same_thing() {
        // 持 writes 扌 and is classified under 64, whose head form is 手 — the
        // difference between the two shapes is what the radicals screen teaches.
        let view = state().kanji('持').expect("持 is jōyō");
        assert_eq!(view.radical.number, 64);
        assert_eq!(view.radical.form, '扌');
        assert_eq!(view.radical.ch, '手');
        assert_eq!(view.radical.note.as_deref(), Some("手"));
        assert_eq!(view.radical.stroke_count, 4);
        assert!(view.radical.characters >= 90, "手 heads a large family");
    }

    #[test]
    fn the_radical_panel_gets_all_two_hundred_and_fourteen() {
        let state = state();
        let radicals = state.radicals();
        assert_eq!(radicals.len(), 214);
        assert_eq!(radicals[0].number, 1);
        assert_eq!(radicals[0].ch, '一');
        assert_eq!(radicals[213].number, 214);
        assert_eq!(radicals[213].ch, '龠');

        let empty: Vec<char> = radicals
            .iter()
            .filter(|r| r.characters.is_empty())
            .map(|r| r.ch)
            .collect();
        assert_eq!(empty.len(), 16, "no jōyō character uses these: {empty:?}");

        let hand = radicals.iter().find(|r| r.number == 64).expect("64");
        assert_eq!(hand.ch, '手');
        assert_eq!(hand.characters.len(), 95);
        assert_eq!(hand.characters[0], '手', "the head form leads its own family");
        assert!(hand.characters.contains(&'持'));

        // Every member of every family can be opened on the board, which is what
        // makes a family a set of things to practise rather than a list.
        let mut members = 0;
        for family in &radicals {
            for ch in &family.characters {
                assert!(state.kanji(*ch).is_ok(), "{ch} cannot be opened");
                members += 1;
            }
        }
        assert_eq!(members, 2_136, "every character is in exactly one family");
    }

    #[test]
    fn one_radical_comes_back_with_the_geometry_the_board_writes_it_with() {
        let state = state();
        let radical = state.radical(64).expect("64 is 手");
        assert_eq!(radical.ch, '手');
        assert_eq!(radical.stroke_count, 4);
        assert_eq!(radical.outlines.len(), 4);
        assert_eq!(radical.medians.len(), 4);
        assert_eq!(radical.characters.len(), 95);

        // The board can grade it with the same engine that grades a kana.
        let graded = state
            .grade('手', &[radical.medians[0].clone()], &GradeOptions::default())
            .expect("grades");
        assert_eq!(graded.report.expected_strokes, 4);
        assert!(
            graded.joined.is_empty(),
            "a radical is graded by the shared engine, not the kana rule"
        );

        // A number that is not a radical is a message rather than a panic.
        for number in [0u8, 215] {
            let err = state.radical(number).unwrap_err();
            assert!(err.contains("numbered 1 to 214"), "{err}");
        }
    }

    #[test]
    fn a_kanji_outside_the_joyo_set_is_an_error_not_a_panic() {
        let err = state().kanji('鳩').unwrap_err();
        assert!(err.contains("not one of the jōyō kanji"), "{err}");
    }

    #[test]
    fn every_lesson_of_the_kanji_course_names_characters_that_can_be_opened() {
        let state = state();
        let lessons = state.kanji_lessons();
        let mut taught = 0;
        for lesson in &lessons {
            assert_eq!(lesson.count, lesson.kanji.len());
            assert!(!lesson.grade_name.is_empty());
            for ch in &lesson.kanji {
                assert!(
                    state.kanji(*ch).is_ok(),
                    "lesson {} lists {ch}, which cannot be opened",
                    lesson.key
                );
                taught += 1;
            }
        }
        assert_eq!(taught, 2_136);
    }

    #[test]
    fn a_kana_view_carries_the_geometry_the_screen_draws_and_grades() {
        let view = state().kana('あ').expect("あ is in the set");
        assert_eq!(view.script, "hiragana");
        assert_eq!(view.stroke_count, 3);
        assert!(view.practisable);
        assert!(!view.silent);
        assert_eq!(view.hepburn, "a");
        assert_eq!(view.outlines.len(), 3);
        assert_eq!(view.medians.len(), 3);
        assert!(view.romaji.contains(&"a".to_string()));
    }

    #[test]
    fn a_kana_view_explains_what_it_is_confused_with() {
        let view = state().kana('シ').expect("シ is in the set");
        assert!(
            view.confusions.iter().any(|c| c.ch == 'ツ'),
            "シ should be flagged against ツ"
        );
        assert!(!view.confusions[0].tell.is_empty());
    }

    #[test]
    fn a_kana_outside_the_set_is_an_error_not_a_panic() {
        let err = state().kana('一').unwrap_err();
        assert!(err.contains("not in the kana set"), "{err}");
    }

    #[test]
    fn a_silent_mark_reports_itself_as_silent() {
        assert!(state().kana('ー').expect("ー is in the set").silent);
        assert!(state().kana('っ').expect("っ is in the set").silent);
        assert!(!state().kana('ん').expect("ん is in the set").silent);
    }

    #[test]
    fn grading_traces_the_stored_centre_line_as_legible() {
        let state = state();
        let view = state.kana('ー').expect("ー is in the set");
        let attempt = vec![view.medians[0].clone()];
        let graded = state
            .grade('ー', &attempt, &GradeOptions::default())
            .expect("grades");
        let report = graded.report;
        assert!(report.legible, "tracing the guide must be legible: {:.0}", report.overall);
        assert_eq!(report.expected_strokes, 1);
        assert!(graded.joined.is_empty(), "nothing was joined");
    }

    #[test]
    fn grading_a_character_that_is_not_taught_is_an_error() {
        // 鳩 is jinmeiyō: not a kana, not jōyō, and not one of the 214 radicals,
        // so there is no reference to grade against and the message says so.
        let err = state().grade('鳩', &[], &GradeOptions::default()).unwrap_err();
        assert!(err.contains("not a kana, a jōyō kanji, or a radical"), "{err}");
    }

    #[test]
    fn grading_uses_the_corrected_stroke_count_not_the_raw_one() {
        // あ arrives from upstream as four drawing segments and is taught with
        // three. The grader must ask for three.
        let state = state();
        let view = state.kana('あ').expect("あ is in the set");
        assert_eq!(view.stroke_count, 3);
        let graded = state
            .grade('あ', &[], &GradeOptions::default())
            .expect("grades");
        assert_eq!(graded.report.expected_strokes, 3);
    }

    #[test]
    fn a_joined_kana_is_graded_as_that_kana_and_names_the_join() {
        // さ is taught in three strokes and commonly written in two, the first
        // two drawn as one. The board must call it legible, and say which taught
        // strokes were joined, because the report's own stroke numbers are the
        // *drawn* strokes and the screen lists them by number.
        let state = state();
        let view = state.kana('さ').expect("さ is in the set");
        let mut attempt = view.medians.clone();
        let second = attempt.remove(1);
        attempt[0].extend(second);

        let graded = state
            .grade('さ', &attempt, &GradeOptions::default())
            .expect("grades");
        assert!(graded.report.legible, "{:.0}/100", graded.report.overall);
        assert_eq!(graded.joined, vec![vec![1, 2]]);
        assert_eq!(
            graded.report.expected_strokes, 2,
            "the report is about the form the hand wrote"
        );
    }

    #[test]
    fn a_kana_with_a_stroke_missing_is_not_read_as_a_join() {
        // The rule accepts a *joined* hand, never a dropped stroke: with the
        // third stroke of さ absent, no regrouping of the reference clears the
        // per-stroke bars, so the taught verdict is what comes back.
        let state = state();
        let view = state.kana('さ').expect("さ is in the set");
        let attempt = vec![view.medians[0].clone(), view.medians[1].clone()];

        let graded = state
            .grade('さ', &attempt, &GradeOptions::default())
            .expect("grades");
        assert!(!graded.report.legible);
        assert!(graded.joined.is_empty());
        assert_eq!(graded.report.expected_strokes, 3, "three strokes are taught");
    }

    #[test]
    fn a_kanji_keeps_the_strict_count_and_order_and_no_combined_strokes() {
        // The joined-stroke rule is a kana rule. A kanji keeps the strict taught
        // stroke count and order — kanji stroke order is what the course teaches,
        // and the Chinese app has always graded its characters this way — so a
        // kanji drawn with two strokes joined is refused with the taught count
        // rather than read as a different form.
        let state = state();
        let kanji = state.kanji('日').expect("日 is jōyō");
        let mut attempt = kanji.medians.clone();
        let second = attempt.remove(1);
        attempt[0].extend(second);

        let graded = state
            .grade('日', &attempt, &GradeOptions::default())
            .expect("grades");
        assert!(!graded.report.legible, "a combined kanji stroke is refused");
        assert!(graded.joined.is_empty());
        assert_eq!(
            graded.report.expected_strokes, 4,
            "four strokes taught, and the strict count is what comes back"
        );
    }

    #[test]
    fn every_classic_pair_can_actually_be_asked() {
        let state = state();
        let kind = DrillKind::Confusion;
        let askable = state.askable_pairs(kind);
        assert_eq!(
            askable.len(),
            kind.pairs().len(),
            "a pair the dataset cannot prompt would silently shrink the drill"
        );

        // And the question each of them produces is answerable: the target is one
        // of the options, both options are the pair's own spellings, and the
        // prompt is not empty.
        for (index, pair) in askable.iter().enumerate() {
            let roll = (index as f64 + 0.5) / askable.len() as f64;
            for side in [0.0, 0.75] {
                let question = state
                    .next_drill_question_with_rolls(kind, roll, side)
                    .expect("an askable pair yields a question");
                assert_eq!(question.kind, kind.name());
                assert_eq!(question.pair, pair.key());
                assert!(
                    question.options.contains(&question.ch),
                    "{} must be one of its own options",
                    question.ch
                );
                assert_eq!(question.options.len(), 2, "the pair, and nothing else");
                for option in &question.options {
                    assert!(pair.holds(option), "{option} is not in {}", question.pair);
                }
                assert!(!question.hepburn.is_empty(), "{} has no prompt", question.ch);
                assert!(!question.tell.is_empty(), "{} has no tell", question.pair);
            }
        }

        // The two sides are two questions, not the same one twice.
        let first = state.next_drill_question_with_rolls(kind, 0.0, 0.0).expect("a question");
        let second = state.next_drill_question_with_rolls(kind, 0.0, 1.0).expect("a question");
        assert_eq!(first.pair, second.pair);
        assert_ne!(first.ch, second.ch, "the pair asked from the other side");
        let mut offered = first.options.clone();
        offered.sort();
        let mut both = vec![first.ch, second.ch];
        both.sort();
        assert_eq!(offered, both, "the pair's two spellings, whichever side is asked");
    }

    #[test]
    fn the_yoon_drill_asks_the_digraph_against_the_long_spelling() {
        let state = state();
        for (kind, digraph, plain) in [
            (DrillKind::YoonHiragana, "きゃ", "きや"),
            (DrillKind::YoonKatakana, "キャ", "キヤ"),
        ] {
            assert_eq!(state.askable_pairs(kind).len(), 33, "{kind:?}");

            // The contrast, found by its key rather than by a roll, is the one
            // the milestone names: one mora against two.
            let pair = state
                .askable_pairs(kind)
                .into_iter()
                .find(|p| p.key() == format!("{digraph}|{plain}"))
                .unwrap_or_else(|| panic!("{kind:?} holds the {digraph} contrast"));
            assert_eq!(pair.kind, kind);

            // Ask for the digraph, then for the long spelling: two questions,
            // two prompts — `kya` and `kiya` — and one recorded pair.
            let long = {
                let index = state
                    .askable_pairs(kind)
                    .iter()
                    .position(|p| p.key() == pair.key())
                    .expect("the pair is in the pool");
                let roll = (index as f64 + 0.5) / 33.0;
                let asked_for_digraph =
                    state.next_drill_question_with_rolls(kind, roll, 0.0).expect("a question");
                assert_eq!(asked_for_digraph.ch, digraph);
                assert_eq!(asked_for_digraph.hepburn, "kya");
                assert_eq!(asked_for_digraph.options.len(), 2);
                assert!(asked_for_digraph.options.contains(&plain.to_string()));

                state
                    .record_drill_answer(&asked_for_digraph.pair, &asked_for_digraph.ch, plain)
                    .expect("records");
                asked_for_digraph.pair
            };

            let tally = state.log().tally(&long);
            assert_eq!(tally.asked, 1);
            assert_eq!(tally.wrong, 1, "writing the long spelling for きゃ is a miss");
            assert_eq!(
                state.log().weight(&long),
                3.0,
                "and a yōon miss is weighted by the same rule"
            );
        }
    }

    #[test]
    fn the_voicing_drill_asks_the_plain_kana_against_its_voiced_form() {
        let state = state();
        for (kind, plain, voiced) in [
            (DrillKind::VoicingHiragana, "か", "が"),
            (DrillKind::VoicingKatakana, "カ", "ガ"),
        ] {
            let pool = state.askable_pairs(kind);
            assert_eq!(pool.len(), 25, "{kind:?}: five voiced rows of five");

            // The contrast the exercise is for, found by its key rather than by a
            // roll: the plain kana and the same kana with the mark.
            let key = format!("{plain}|{voiced}");
            let index = pool
                .iter()
                .position(|p| p.key() == key)
                .unwrap_or_else(|| panic!("{kind:?} holds {key}"));
            let roll = (index as f64 + 0.5) / pool.len() as f64;

            let asked = state
                .next_drill_question_with_rolls(kind, roll, 0.0)
                .expect("a question");
            assert_eq!(asked.kind, kind.name());
            assert_eq!(asked.ch, plain, "the plain kana is the side the low roll asks for");
            assert_eq!(asked.hepburn, "ka");
            assert_eq!(asked.options.len(), 2);
            assert!(asked.options.contains(&voiced.to_string()));
            assert!(asked.tell.contains(plain) && asked.tell.contains(voiced));

            // Writing the voiced kana for the plain one is a miss, recorded in the
            // same file as every other exercise's.
            let tally = state
                .record_drill_answer(&asked.pair, &asked.ch, voiced)
                .expect("records");
            assert_eq!(tally.pair, key);
            assert_eq!(tally.wrong, 1);
            assert_eq!(tally.weight, 3.0, "the weighting rule is the same one");
        }
    }

    #[test]
    fn an_answer_is_recorded_against_the_pair_and_the_arithmetic_says_so() {
        let state = state();
        let pair = "シ|ツ".to_string();

        // A miss: 1 + 2.
        let after_miss = state
            .record_drill_answer(&pair, "シ", "ツ")
            .expect("records");
        assert_eq!(after_miss.pair, pair);
        assert_eq!(after_miss.asked, 1);
        assert_eq!(after_miss.wrong, 1);
        assert_eq!(after_miss.correct, 0);
        assert_eq!(after_miss.weight, 3.0);

        // The same pair named the other way round, answered correctly: one pair,
        // and the weight comes back down.
        let after_hit = state
            .record_drill_answer("ツ|シ", "ツ", "ツ")
            .expect("records");
        assert_eq!(after_hit.pair, pair, "one pair, whichever way it is named");
        assert_eq!(after_hit.asked, 2);
        assert_eq!(after_hit.weight, 2.0);

        // Correctness is decided here, from the two spellings — not sent by the
        // caller.
        let lying = state
            .record_drill_answer(&pair, "シ", "ツ")
            .expect("records");
        assert_eq!(lying.wrong, 2, "シ asked for and ツ picked is a miss");
        assert_eq!(lying.correct, 1);
    }

    #[test]
    fn an_answer_about_something_that_is_not_a_pair_is_refused() {
        let state = state();
        let err = state
            .record_drill_answer("あ|い", "あ", "い")
            .unwrap_err();
        assert!(err.contains("not one of the pairs this drill asks about"), "{err}");

        // A pair, but a spelling that is not in it.
        let err = state.record_drill_answer("シ|ツ", "あ", "ツ").unwrap_err();
        assert!(err.contains("is not in シ|ツ"), "{err}");
        let err = state.record_drill_answer("シ|ツ", "シ", "あ").unwrap_err();
        assert!(err.contains("neither of the two answers"), "{err}");

        // A yōon contrast named with the *other* script's characters is refused:
        // `kya` names きゃ here and キャ there, which is why the two scripts are
        // separate exercises.
        let err = state.record_drill_answer("きゃ|きや", "キャ", "キヤ").unwrap_err();
        assert!(err.contains("is not in きゃ|きや"), "{err}");

        // And nothing was written by any of it.
        assert!(state.log().is_empty());
    }

    #[test]
    fn the_drill_prefers_the_pair_the_learner_keeps_getting_wrong() {
        let state = state();
        let kind = DrillKind::Confusion;
        // ン/ソ (ソ|ン) sits early in the list, so a mid-field roll lands past it
        // while everything weighs the same.
        let roll = 0.3;
        let before = state.next_drill_question_with_rolls(kind, roll, 0.0).expect("a question");
        assert_ne!(before.pair, "ソ|ン");

        state.record_drill_answer("ソ|ン", "ン", "ソ").expect("records");
        state.record_drill_answer("ソ|ン", "ン", "ソ").expect("records");
        let after = state.next_drill_question_with_rolls(kind, roll, 0.0).expect("a question");
        assert_eq!(after.pair, "ソ|ン", "the same roll now lands on the missed pair");
    }

    #[test]
    fn the_question_is_never_one_the_board_cannot_draw() {
        // Every character the drill can ask about is in the dataset, so a question
        // is never outside the set the rest of the app works in — and that holds
        // for a digraph, whose two characters are checked rather than its spelling
        // being one character, which it is not.
        let state = state();
        for kind in DrillKind::ALL {
            let pool = state.askable_pairs(kind);
            // A yōon exercise asks with two-character spellings and the others ask
            // with one kana — read from the pool rather than from a list of which
            // kinds are which.
            let expected = pool
                .first()
                .map_or(1, |pair| pair.sides[0].spelling.chars().count());
            for index in 0..pool.len() {
                let roll = (index as f64 + 0.01) / pool.len() as f64;
                let question = state
                    .next_drill_question_with_rolls(kind, roll, 0.5)
                    .expect("a question");
                for option in &question.options {
                    assert_eq!(
                        option.chars().count(),
                        expected,
                        "{kind:?} asks with {expected}-character spellings"
                    );
                    for ch in option.chars() {
                        assert!(state.kana(ch).is_ok(), "{ch} of {option} cannot be drawn");
                    }
                }
            }
        }
    }

    #[test]
    fn the_chart_is_the_grid_with_its_holes_and_the_characters_off_it() {
        let state = state();
        for script in [Script::Hiragana, Script::Katakana] {
            let chart = state.chart(script);
            assert_eq!(chart.script, script.name());
            assert_eq!(chart.vowels, vec!["a", "i", "u", "e", "o"], "the column labels");
            assert_eq!(chart.rows.len(), 16, "eleven plain rows and five voiced");
            assert!(chart.rows.iter().all(|row| row.cells.len() == 5));

            // The grid holds 71 kana in 80 slots, and the holes are the language's.
            let filled: usize = chart
                .rows
                .iter()
                .map(|row| row.cells.iter().filter(|cell| cell.is_some()).count())
                .sum();
            assert_eq!(filled, 71);
            let holes: usize = chart
                .rows
                .iter()
                .map(|row| row.cells.iter().filter(|cell| cell.is_none()).count())
                .sum();
            assert_eq!(holes, 9);

            // や is the row a left-aligned chart would get wrong: ゆ is in the u
            // column and the i and e columns are empty.
            let ya = chart.rows.iter().find(|row| row.sound == "ya").expect("the や row");
            let expected: Vec<Option<String>> = if script == Script::Hiragana {
                vec![Some("や".into()), None, Some("ゆ".into()), None, Some("よ".into())]
            } else {
                vec![Some("ヤ".into()), None, Some("ユ".into()), None, Some("ヨ".into())]
            };
            assert_eq!(ya.cells, expected);

            // And the voiced rows are the last five, marked as such.
            assert_eq!(chart.rows.iter().filter(|row| row.voiced).count(), 5);
            assert!(chart.rows[..11].iter().all(|row| !row.voiced));
            assert!(chart.rows[11..].iter().all(|row| row.voiced));

            // Nothing on the grid is offered twice, and the off-grid groups hold
            // the rest — katakana's two extra groups included.
            let mut on_grid: Vec<String> =
                chart.rows.iter().flat_map(|row| row.cells.iter().flatten().cloned()).collect();
            let total = on_grid.len();
            on_grid.sort();
            on_grid.dedup();
            assert_eq!(total, on_grid.len(), "{script:?} draws a kana twice");

            let keys: Vec<&str> = chart.off_grid.iter().map(|g| g.key.as_str()).collect();
            assert!(keys.contains(&format!("{}-small", script.name()).as_str()));
            assert!(keys.contains(&format!("{}-rare", script.name()).as_str()));
            assert_eq!(keys.contains(&"katakana-v"), script == Script::Katakana);
            assert_eq!(keys.contains(&"katakana-choonpu"), script == Script::Katakana);
        }
    }

    #[test]
    fn every_kana_the_chart_offers_can_be_opened() {
        // Acceptance criterion 3 of N4: a kana offered anywhere can be opened and
        // graded. The chart is the screen that offers them all, so it is the one
        // that has to be exhaustive rather than indicative.
        let state = state();
        for script in [Script::Hiragana, Script::Katakana] {
            let chart = state.chart(script);
            let offered: Vec<String> = chart
                .rows
                .iter()
                .flat_map(|row| row.cells.iter().flatten().cloned())
                .chain(chart.off_grid.iter().flat_map(|group| group.kana.iter().cloned()))
                .collect();
            assert!(!offered.is_empty());
            for ch in offered {
                let mut chars = ch.chars();
                let (Some(one), None) = (chars.next(), chars.next()) else {
                    panic!("{ch} in the chart is not one character")
                };
                let view = state
                    .kana(one)
                    .unwrap_or_else(|err| panic!("the chart offers {ch}, which cannot be opened: {err}"));
                assert!(view.practisable, "{ch} is offered but cannot be graded");
            }
        }

        // And the count is the whole script, not most of it: the chart is the one
        // screen that has to show ヷ ヸ ヹ ヺ ー as well as the gojūon.
        let katakana = state.chart(Script::Katakana);
        let offered: usize = katakana
            .rows
            .iter()
            .map(|row| row.cells.iter().filter(|cell| cell.is_some()).count())
            .sum::<usize>()
            + katakana.off_grid.iter().map(|group| group.kana.len()).sum::<usize>();
        assert_eq!(offered, state.dataset().of_script(Script::Katakana).count());
    }

    #[test]
    fn reading_checks_accept_both_romanisations() {
        assert!(check_reading('し', "shi".into()).correct);
        assert!(check_reading('し', "si".into()).correct);
        assert!(!check_reading('し', "chi".into()).correct);
        // A wrong answer still reports what it produced.
        let wrong = check_reading('し', "chi".into());
        assert_eq!(wrong.produced, "ち");
    }

    #[test]
    fn romaji_converts_in_both_scripts() {
        assert_eq!(romaji_to_kana("kana".into(), None).expect("converts"), "かな");
        assert_eq!(
            romaji_to_kana("kana".into(), Some("katakana".into())).expect("converts"),
            "カナ"
        );
        assert!(romaji_to_kana("kana".into(), Some("kanji".into())).is_err());
        assert!(romaji_to_kana("!!!".into(), None).is_err());
    }

    #[test]
    fn every_lesson_names_kana_the_screen_can_open() {
        let state = state();
        for script in [Script::Hiragana, Script::Katakana] {
            for lesson in state.lessons(script) {
                assert_eq!(lesson.count, lesson.kana.len());
                for ch in lesson.kana {
                    assert!(
                        state.kana(ch).is_ok(),
                        "lesson {} lists {ch}, which cannot be opened",
                        lesson.key
                    );
                }
            }
        }
    }

    #[test]
    fn the_ladder_comes_back_with_our_names_and_its_sizes() {
        let state = state();
        let bands = state.word_bands();
        assert_eq!(bands.len(), 7, "six kyōiku grades and the remainder");
        assert_eq!(
            bands.iter().map(|b| b.words).sum::<usize>(),
            state.words.len()
        );
        assert_eq!(bands[0].band, 1);
        assert_eq!(bands[0].name, "kyōiku 1");
        assert_eq!(bands[6].name, "jōyō beyond the school grades");
        for band in &bands {
            assert!(
                !band.name.to_ascii_uppercase().contains("JLPT"),
                "the ladder is derived, not the JLPT's: {:?}",
                band.name
            );
        }
        // No band is empty, and band 1 is the smallest — a fact about Japanese
        // rather than a hole in the data.
        assert!(bands.iter().all(|b| b.words > 0));
        assert!(bands[0].words < bands[6].words);
    }

    #[test]
    fn a_page_of_words_carries_what_a_word_card_draws() {
        let state = state();
        let page = state.words_in_band(1, 0, 10);
        assert_eq!(page.band, 1);
        assert_eq!(page.offset, 0);
        assert_eq!(page.words.len(), 10);
        assert!(
            page.total > page.words.len(),
            "band 1 holds more than one page: {} words",
            page.total
        );

        let first = &page.words[0];
        assert!(!first.text.is_empty());
        assert!(!first.reading.is_empty());
        assert!(!first.meaning.is_empty());
        assert_eq!(first.band, 1);
        assert_eq!(first.band_name, "kyōiku 1");
        assert!(first.nf.is_none_or(|rank| (1..=48).contains(&rank)));

        // A page past the end is empty rather than an error, and a limit the
        // interface should not have asked for is clamped.
        assert!(state.words_in_band(1, page.total, 10).words.is_empty());
        assert!(state.words_in_band(1, 0, 10_000).words.len() <= 200);
    }

    /// **The phrase ladder, which is the vocabulary's.** Same seven bands, same
    /// names, same derivation — the third screen to read a phrase's band as its
    /// hardest word, and the reason the two screens cannot disagree about what
    /// "kyōiku 3" means.
    #[test]
    fn the_phrase_ladder_is_the_vocabularys_with_its_own_counts() {
        let state = state();
        let bands = state.phrase_bands();
        assert_eq!(bands.len(), 7);
        assert_eq!(
            bands.iter().map(|band| band.phrases).sum::<usize>(),
            state.phrases.len()
        );
        assert_eq!(bands[0].band, 1);
        assert_eq!(bands[0].name, "kyōiku 1");
        assert_eq!(bands[6].name, "jōyō beyond the school grades");
        for band in &bands {
            assert!(
                !band.name.to_ascii_uppercase().contains("JLPT"),
                "the ladder is derived, not the JLPT's: {:?}",
                band.name
            );
        }
        // Unlike the vocabulary, a band *can* be empty: the corpus is filtered, not
        // authored, so nothing guarantees a band 1 exists. This build happens to
        // fill all seven, which is what the artifact test pins.
        assert!(bands.iter().all(|band| band.phrases > 0));
    }

    /// A phrase, as a row draws it: the sentence, its words, its translation and its
    /// attribution. The band is deliberately absent — the screen asks for one band at
    /// a time and draws it under that band's chip.
    #[test]
    fn a_band_of_phrases_carries_what_a_row_draws() {
        let state = state();
        let phrases = state.phrases_in_band(1);
        assert_eq!(phrases.len(), 200, "every band is capped and full");

        let first = &phrases[0];
        assert!(!first.text.is_empty());
        assert!(!first.english.is_empty());
        assert!(!first.author.is_empty());
        assert_eq!(first.licence, "CC BY 2.0 FR");
        assert!(!first.tokens.is_empty());

        // Every kanji-bearing word is one the course teaches, which is what makes the
        // tap work — the invariant the artifact test proves for all 1,400. A
        // kanji-bearing token is the one that carries a reading: the artifact test
        // asserts the two go together, and the vocabulary has no kana-only entry for
        // a kana token to link to.
        for phrase in &phrases {
            for token in &phrase.tokens {
                if let Some(rt) = token.rt.as_deref() {
                    assert!(!rt.is_empty(), "#{}: an empty reading", phrase.id);
                    let word = token.word.as_deref().unwrap_or_else(|| {
                        panic!("#{}: {:?} has a kanji and no card", phrase.id, token.surface)
                    });
                    assert!(
                        state.words.of_text(word).is_some(),
                        "#{}: {word:?} is not in the vocabulary",
                        phrase.id
                    );
                }
            }
            assert_eq!(
                phrase.tokens.iter().map(|t| t.surface.as_str()).collect::<String>(),
                phrase.text,
                "#{}'s tokens do not spell its text",
                phrase.id
            );
        }

        // Shortest first, and a band the corpus did not fill answers with nothing.
        let lengths: Vec<usize> = phrases.iter().map(|p| p.text.chars().count()).collect();
        assert!(lengths.windows(2).all(|w| w[0] <= w[1]), "{lengths:?}");
        assert!(state.phrases_in_band(9).is_empty());
    }

    /// **The character card's vocabulary list.** A page of what
    /// `WordDataset::of_kanji` holds, with the character echoed back, paged for a
    /// measured reason: 一 is written in 223 words.
    #[test]
    fn a_page_of_a_characters_words_arrives_in_course_order() {
        let state = state();
        let first = state.words_of_kanji('一', 0, 12).expect("一 is jōyō");
        assert_eq!(first.ch, '一');
        assert_eq!(first.total, 223, "一 is the busiest character in the vocabulary");
        assert_eq!(first.offset, 0);
        assert_eq!(first.words.len(), 12);
        for word in &first.words {
            assert!(word.text.contains('一'), "{} does not use 一", word.text);
        }
        let mut band = 0;
        for word in &first.words {
            assert!(
                word.band >= band,
                "{} arrives in band {} after band {band}, which is not the course's order",
                word.text,
                word.band
            );
            band = word.band;
        }

        // The second page continues rather than repeating, a page past the end is
        // empty, and a limit the interface should not have asked for is clamped.
        let second = state.words_of_kanji('一', 12, 12).expect("一 is jōyō");
        assert_eq!(second.offset, 12);
        assert!(
            !second
                .words
                .iter()
                .any(|w| w.text == first.words[0].text && w.reading == first.words[0].reading),
            "the second page repeats the first"
        );
        assert!(state.words_of_kanji('一', first.total, 12).expect("一").words.is_empty());
        assert!(state.words_of_kanji('一', 0, 10_000).expect("一").words.len() <= MAX_WORD_PAGE);
    }

    /// A jōyō character in no word is a fact to state; a character outside the set
    /// is a message, because the vocabulary holds no word it could be in.
    #[test]
    fn a_character_in_no_word_is_empty_and_one_outside_the_set_is_an_error() {
        let state = state();
        let none = state.words_of_kanji('且', 0, 12).expect("且 is jōyō");
        assert_eq!(none.ch, '且');
        assert_eq!(none.total, 0);
        assert!(none.words.is_empty(), "no word of this course uses 且");

        let err = state.words_of_kanji('鳩', 0, 12).unwrap_err();
        assert!(err.contains("not one of the jōyō kanji"), "{err}");
    }

    /// **The Start screen's demonstration.** One sound, several words, ordered the
    /// way the course orders them — and a query that is not a reading refused
    /// rather than answered with an empty page a learner would read as a gap.
    #[test]
    fn the_words_read_alike_arrive_in_course_order() {
        let state = state();
        let alike = state.words_of_reading("はし", 0, 12).expect("はし is a reading");
        assert_eq!(alike.reading, "はし", "the reading asked for, echoed back");
        assert_eq!(alike.total, 3, "橋, 端 and 箸");
        assert_eq!(alike.offset, 0);
        let texts: Vec<&str> = alike.words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, vec!["橋", "端", "箸"], "the course's order, not a second sort");
        for word in &alike.words {
            assert_eq!(word.reading, "はし", "{} is drawn with the dictionary's reading", word.text);
            assert!(!word.meaning.is_empty(), "{} has a gloss to show", word.text);
        }

        // A reading is the same reading in either kana, so the katakana query finds
        // the same three words.
        assert_eq!(state.words_of_reading("ハシ", 0, 12).expect("a reading").total, 3);
        // A kana reading the course does not carry is an empty page — a fact to
        // state in words — and not an error.
        let none = state.words_of_reading("ぬれ", 0, 12).expect("a reading");
        assert_eq!(none.total, 0);
        assert!(none.words.is_empty());
        // Romaji is a message: no reading in the artifact is written that way, so
        // an empty page would be a typo wearing the clothes of a gap in the data.
        let err = state.words_of_reading("hashi", 0, 12).unwrap_err();
        assert!(err.contains("is not a reading"), "{err}");
        assert!(state.words_of_reading("", 0, 12).is_err());
        // A limit the interface should not have asked for is clamped, and the
        // **lower** bound is the one that can be observed here: an upper-bound
        // assertion would be unfalsifiable, because no reading in this vocabulary
        // has more than a page's worth of words (`words_of_kanji`'s 一, with 223,
        // is where the upper bound is really tested).
        assert_eq!(state.words_of_reading("はし", 0, 0).expect("はし").words.len(), 1);
        // And a page past the end is empty rather than an error.
        assert!(state.words_of_reading("はし", 3, 12).expect("はし").words.is_empty());
    }

    #[test]
    fn a_word_is_graded_as_a_word_and_never_from_its_characters() {
        let state = state();
        // 大人 is おとな. A reading composed from the characters would be
        // だいじん, and the point of carrying the word's own reading is that it is
        // not.
        assert!(state.check_word("大人", "おとな", "otona").expect("a word").correct);
        assert!(!state.check_word("大人", "おとな", "daijin").expect("a word").correct);
        // The wrong answer says what it produced, so a screen can show it.
        assert_eq!(
            state.check_word("大人", "おとな", "daijin").expect("a word").produced,
            "だいじん"
        );
        // A word that is not one is an error rather than a silent `false`.
        assert!(state.check_word("大人", "だいじん", "otona").is_err());
    }

    #[test]
    fn a_kana_answer_is_accepted_as_well_as_romaji() {
        let state = state();
        assert!(state.check_word("学生", "がくせい", "gakusei").expect("a word").correct);
        assert!(
            state.check_word("学生", "がくせい", "がくせい").expect("a word").correct,
            "a learner who can read kana should not have to transliterate"
        );
        assert!(
            state.check_word("学生", "がくせい", "ガクセイ").expect("a word").correct,
            "the kana type should not decide the answer"
        );
    }

    #[test]
    fn a_word_can_be_fetched_by_its_text_and_reading() {
        let state = state();
        let word = state.word("食べる", "たべる").expect("食べる is in the vocabulary");
        assert_eq!(word.text, "食べる");
        assert_eq!(word.reading, "たべる");
        assert_eq!(word.meaning, "to eat");
        assert_eq!(word.furigana.len(), 2);
        assert_eq!(word.furigana[0].ruby, "食");
        assert_eq!(word.furigana[0].rt.as_deref(), Some("た"));
        assert!(state.word("食べる", "くう").is_err(), "that is a different word");
    }

    #[test]
    fn a_word_a_passage_links_to_is_found_by_its_text_alone() {
        // The token carries the word's text, not its reading, so a tap has to be
        // able to open an entry from that alone — 行き's own reading is いき and the
        // word it links to is 行く, read いく.
        let state = state();
        let word = state.word_of_text("行く").expect("行く is in the vocabulary");
        assert_eq!(word.text, "行く");
        assert_eq!(word.reading, "いく");

        // 行き is an entry in its own right — a noun — so this finds that one, and
        // the two are different words rather than one answer to two questions.
        let noun = state.word_of_text("行き").expect("行き is a noun entry");
        assert_eq!(noun.text, "行き");
        assert_ne!(noun.reading, word.reading);

        // Anything the course does not teach is a message rather than a panic.
        assert!(state.word_of_text("あいうえお").is_err());
    }

    #[test]
    fn the_passages_come_back_segmented_with_their_readings() {
        let state = state();
        let summaries = state.passages();
        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().all(|p| p.lines > 0 && p.tokens > 0));
        assert!(summaries.iter().all(|p| p.gloss.is_some()));

        let asa = state.passage("asa").expect("asa is a passage");
        assert_eq!(asa.title, "あさ");
        // 学生 is one token, carries its reading, and links to the vocabulary word.
        let student = asa
            .lines
            .iter()
            .flatten()
            .find(|token| token.surface == "学生")
            .expect("学生 is in the passage");
        assert_eq!(student.rt.as_deref(), Some("がくせい"));
        assert_eq!(student.word.as_deref(), Some("学生"));
        // A particle is a token too, and needs no ruby.
        let particle = asa
            .lines
            .iter()
            .flatten()
            .find(|token| token.surface == "は")
            .expect("は is in the passage");
        assert_eq!(particle.rt, None);

        assert!(state.passage("nope").is_err());
    }

    #[test]
    fn every_token_of_every_passage_that_has_a_word_can_be_opened() {
        // What a tap does, as a test: the link a passage carries has to resolve to
        // a word the vocabulary screen can actually show.
        let state = state();
        let mut opened = 0;
        for summary in state.passages() {
            for line in state.passage(&summary.key).expect("a passage").lines {
                for token in line {
                    let Some(text) = &token.word else { continue };
                    // The link is the word's text; the reading is whatever the
                    // vocabulary holds for it, which is what the card will show.
                    let held = state
                        .words
                        .of_text(text)
                        .unwrap_or_else(|| panic!("{text} is linked but not held"));
                    let word = state
                        .word(&held.text, &held.reading)
                        .unwrap_or_else(|e| panic!("{text} does not open: {e}"));
                    assert_eq!(word.text, *text);
                    if token.surface.chars().any(nihongo_core::is_kanji) {
                        assert!(
                            token.rt.is_some(),
                            "{text}: a kanji token with no reading cannot be drawn"
                        );
                    }
                    opened += 1;
                }
            }
        }
        assert!(opened >= 10, "the passages link to {opened} words in all");
    }

    #[test]
    fn the_yoon_helper_covers_every_digraph() {
        assert_eq!(yoon("hiragana".into()).expect("known script").len(), 33);
        let all = yoon("katakana".into()).expect("known script");
        assert!(all.iter().any(|y| y.display == "キュ" && y.hepburn == "kyu"));
        assert!(yoon("kanji".into()).is_err());
    }

    /// A brand-new learner has written nothing, so nothing is due and there is no
    /// next date to name. The screen has to be able to say that rather than show
    /// an empty list with no explanation.
    #[test]
    fn a_learner_who_has_written_nothing_has_an_empty_queue_and_no_next_date() {
        let queue = state().review_queue(40, None);
        assert_eq!(queue.cards, 0);
        assert_eq!(queue.due, 0);
        assert!(queue.items.is_empty());
        assert!(queue.next_due.is_none());
        assert!(queue.warning.is_none(), "an in-memory schedule is not a problem");
    }

    /// When nothing is due, the queue still names when the next character comes
    /// back — the answer to "what now?" after a clean attempt.
    #[test]
    fn a_character_that_is_not_due_yet_says_when_it_comes_back() {
        let state = state();
        let said = state.kana('あ').expect("あ");
        let graded = state
            .grade_and_schedule('あ', &said.medians, &GradeOptions::default())
            .expect("grades");
        assert!(graded.scheduled, "score {:.0}", graded.report.overall);
        let due = graded.next_due.clone().expect("a new card has a due date");

        let queue = state.review_queue(40, None);
        assert_eq!(queue.cards, 1);
        assert_eq!(queue.due, 0, "a clean attempt is not due for a day");
        assert!(queue.items.is_empty());
        assert_eq!(queue.next_due.as_deref(), Some(due.as_str()));
    }

    /// The board writes a head form the character course cannot reach — 92 of the
    /// 214 are not jōyō characters — and writing one is scheduled like anything
    /// else the board can draw.
    #[test]
    fn a_radical_head_form_can_be_written_and_scheduled() {
        let state = state();
        let radical = state.radical(6).expect("亅 is radical 6");
        let graded = state
            .grade_and_schedule(radical.ch, &[], &GradeOptions::default())
            .expect("the board can write a head form the character course cannot");
        assert!(graded.scheduled);
        assert!(graded.next_due.is_some());
        assert_eq!(state.review_queue(40, None).cards, 1);
    }

    // ---- the two sections -------------------------------------------------

    /// A section with nothing scheduled names no next date, rather than the other
    /// section's — which is what a shared count would have shown it.
    #[test]
    fn an_empty_section_names_no_date_from_the_other_sections_cards() {
        let state = state();
        let said = state.kana('あ').expect("あ");
        state
            .grade_and_schedule('あ', &said.medians, &GradeOptions::default())
            .expect("grades");

        let kana = state.review_queue(40, Some(Section::Kana));
        assert_eq!(kana.cards, 1);
        assert!(kana.next_due.is_some(), "the kana card comes back");

        let kanji = state.review_queue(40, Some(Section::Kanji));
        assert_eq!(kanji.cards, 0);
        assert_eq!(kanji.due, 0);
        assert!(
            kanji.next_due.is_none(),
            "a kana's due date is not the character course's: {:?}",
            kanji.next_due
        );
    }

    // ---- pronunciation ----------------------------------------------------

    /// The app speaks Japanese, and that is asserted rather than assumed.
    ///
    /// [`Speaker::default`] is **Chinese**, because the two apps that existed
    /// before this app were, and a Chinese voice reading あ is the one
    /// failure here that no other test in this file could see: the command would
    /// answer `Ok(())`, the button would not report an error, and the learner
    /// would hear Mandarin. `hanzi-voice`'s own suite pins the two languages
    /// apart; this pins which one this app asked for.
    #[test]
    fn the_app_speaks_japanese_and_not_chinese() {
        let state = state();
        assert_eq!(state.speaker_handle().language(), Language::Japanese);

        // The voice the machine will really use, where it has one. A test machine
        // with no Japanese voice installed is not a failure — the interface says
        // so and disables the button — so this half is conditional on there being
        // an answer at all, and the language above is the part that always holds.
        if let Some(voice) = state.voice_status() {
            assert!(
                voice.contains("ja_JP") || voice.contains("ja-JP") || voice.contains("Japanese"),
                "the voice in use must be a Japanese one: {voice}"
            );
        }
    }

    /// Nothing to say is an error, and a huge utterance is refused.
    ///
    /// Both are decided before the synthesiser is touched, which is what makes
    /// this safe to run in a test suite: a test that spoke would make a noise on
    /// whoever is at the machine, and a successful `speak` is checked by tapping
    /// the button, not from here.
    #[test]
    fn an_utterance_with_nothing_in_it_is_refused_before_anything_speaks() {
        let state = state();
        assert!(state.speak("").is_err());
        assert!(state.speak("   ").is_err());
        let absurd = "あ".repeat(65);
        let error = state.speak(&absurd).unwrap_err();
        assert!(error.contains("limit"), "unexpected error: {error}");

        // And stopping when nothing has ever been said is not a panic: the
        // speaker is dropped with the state at the end of every one of these
        // tests, which calls `stop` too.
        state.stop_speaking();
    }
}
