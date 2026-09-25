//! Pronunciation, through the operating system's own speech synthesiser.
//!
//! Nothing is downloaded and nothing leaves the machine. On macOS this drives
//! `say`, which ships with the system and already knows how to read Chinese. On
//! iOS it speaks through `AVSpeechSynthesizer` **in process**, because there is
//! no `say` binary there and the app sandbox would refuse to spawn one anyway.
//! Both backends pick a voice with the same rule ([`pick_voice`]), so the
//! mainland-Mandarin preference is one decision rather than two.
//!
//! On iOS the audio session is taken for an utterance and given back a few
//! seconds after it ends, rather than the instant it ends. That is not
//! decoration: under the default session category iOS mutes speech synthesis
//! whenever the Ring/Silent switch is on, so a tap on an explicit pronunciation
//! button produced nothing on a phone while the very same build spoke perfectly
//! on the simulator, which has no such switch. The *hold* is the other half —
//! see [`audio_ready`] for why handing the session straight back made the next
//! word crackle.
//!
//! The **character** is spoken rather than its pinyin: `say` has a Chinese
//! lexicon, so handing it 汉 produces the Mandarin reading, whereas handing an
//! English-trained voice the string `hàn` would have it guess at the
//! diacritics. Speaking the character also does not give away how to write it,
//! so it is safe to offer in recall mode — hearing the sound and producing the
//! glyph is exactly the skill being trained.

// The macOS backend drives a process, so that is where the process types live.
#[cfg(target_os = "macos")]
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::OnceLock;

// The iOS backend speaks in process. AVFoundation's objects are not `Send`, so
// they are created and used on the main thread and never stored in `Speaker`
// (which is shared): see `with_main`.
#[cfg(target_os = "ios")]
use std::cell::RefCell;
#[cfg(target_os = "ios")]
use dispatch2::DispatchQueue;
#[cfg(target_os = "ios")]
use objc2::rc::Retained;
#[cfg(target_os = "ios")]
use objc2::runtime::ProtocolObject;
#[cfg(target_os = "ios")]
use objc2::{define_class, msg_send, AnyThread, MainThreadMarker};
#[cfg(target_os = "ios")]
use objc2_avf_audio::{
    AVAudioSession, AVAudioSessionCategoryOptions, AVAudioSessionCategoryPlayback,
    AVAudioSessionModeSpokenAudio, AVAudioSessionSetActiveOptions, AVSpeechBoundary,
    AVSpeechSynthesisVoice, AVSpeechSynthesizer, AVSpeechSynthesizerDelegate, AVSpeechUtterance,
};
#[cfg(target_os = "ios")]
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

/// A voice as the platform reports it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Voice {
    pub name: String,
    pub locale: String,
    /// Whether speaking with this voice needs a network connection.
    ///
    /// Android's engine offers both a network voice and an on-device one for the
    /// same locale, and this app's whole premise is that it needs no network, so
    /// the settings screen says which is which. macOS and iOS do not report such
    /// a thing and every voice they list is local, which is what the default is
    /// for — the field is only ever *set* from a platform that can tell.
    #[serde(default)]
    pub network: bool,
}

impl Voice {
    /// A voice from a platform that cannot say whether it needs a network.
    ///
    /// Local as the default, and that is not a shrug: every voice macOS and iOS
    /// list is on the device, and an `HANZI_TUTOR_VOICE` override is a name
    /// handed to the synthesiser either way. Only Android distinguishes the two,
    /// and it says so in the listing.
    pub fn local(name: impl Into<String>, locale: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            locale: locale.into(),
            network: false,
        }
    }
}

/// Voices preferred within mainland Mandarin, in order.
///
/// Ordered for a learner rather than for novelty: Tingting is the long-standing
/// zh_CN system voice, and the newer "expressive" voices are only worth falling
/// back to.
const PREFERRED_NAMES: [&str; 3] = ["Tingting", "Ting-Ting", "Meijia"];

/// Environment variable that overrides the automatically chosen voice.
const VOICE_OVERRIDE: &str = "HANZI_TUTOR_VOICE";

/// The longest string that will be handed to the synthesiser.
const MAX_UTTERANCE: usize = 64;

/// What "the utterance in flight" is on this platform.
///
/// Only the macOS backend holds anything here, and it holds the **player**, not
/// the synthesiser — see [`render`] for why speech is rendered to a file before
/// it is heard. iOS holds nothing: its synthesiser is not `Send` and so cannot
/// live in the shared [`Speaker`]; it lives on the main thread instead (see
/// [`with_main`]). Android holds nothing either, because its synthesiser lives on
/// the Kotlin side of the bridge and stopping is a command to that side (see
/// [`crate::platform`]).
#[cfg(target_os = "macos")]
type Utterance = Child;

/// A rendered utterance shorter than this is not speech.
///
/// **`say` does not fail when a voice is not installed** — it exits 0 and writes
/// about 11 ms of near-silence. So a name this crate resolved but the synthesiser
/// cannot use produces a silent "success" that the learner experiences as the
/// sound being cut off, which is exactly the complaint this floor exists to turn
/// into a message. 50 ms is far below the shortest Mandarin syllable (a tone 4 is
/// comfortably over 150 ms) and far above the placeholder.
#[cfg(target_os = "macos")]
const MIN_RENDER_MS: u64 = 50;

/// Pronunciation, with at most one utterance in flight.
#[derive(Default)]
pub struct Speaker {
    /// What is being played, kept so the next utterance can cut it off instead
    /// of talking over it. macOS only, for the reason [`Utterance`] gives.
    #[cfg(target_os = "macos")]
    current: Mutex<Option<Utterance>>,
    /// The voice the learner has asked for, by name, or `None` for the
    /// automatic choice. Set from the settings screen at startup and on every
    /// change; a name this machine does not have falls back to the automatic
    /// choice rather than being an error, so a preference carried from another
    /// machine cannot break pronunciation here.
    preferred: Mutex<Option<String>>,
    /// Every voice the system offers, resolved once. Enumerating them takes
    /// about a second, which is why this is warmed in the background (see
    /// `warm_voice`) and why the settings screen, which asks for the list again
    /// every time it is opened, is served from here rather than listing them
    /// afresh. It never changes while the app runs, so a `OnceLock` is the right
    /// shape for it — unlike the *choice*, which is resolved on every call from
    /// this list and the preference together.
    voices: OnceLock<Vec<Voice>>,
}

impl Speaker {
    /// The voice that will be used.
    ///
    /// Resolved on every call from the installed voices, the override and the
    /// preference together, which is what makes a change take effect
    /// immediately: the *list* is the part that costs a second and it is cached,
    /// so this is now a scan of a short vector.
    pub fn voice(&self) -> Option<Voice> {
        let preferred = self
            .preferred
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let overridden = override_voice();
        resolve_voice(
            self.voices(),
            preferred.as_deref(),
            overridden.as_ref().map(|voice| voice.name.as_str()),
        )
    }

    /// Every voice the system offers.
    fn voices(&self) -> &[Voice] {
        self.voices
            .get_or_init(|| list_voices().unwrap_or_default())
    }

    /// The voices a Chinese character can be spoken with.
    ///
    /// Filtered to the Chinese locales, sorted by name so that the settings
    /// screen's list does not reorder itself between one launch and the next
    /// (the order the system reports is not a promise), and carrying each
    /// voice's locale, which is the only thing that tells 美佳's `zh_TW` apart
    /// from a mainland voice of a similar name.
    pub fn chinese_voices(&self) -> Vec<Voice> {
        chinese_voices(self.voices())
    }

    /// Choose a voice by name, or `None` for the automatic choice.
    ///
    /// Returns what will actually be spoken with, so the settings screen can say
    /// plainly when a name this machine does not have is not the one in use.
    pub fn set_voice(&self, name: Option<&str>) -> Option<Voice> {
        let name = name.map(str::trim).filter(|name| !name.is_empty());
        *self.preferred.lock().unwrap_or_else(|e| e.into_inner()) = name.map(str::to_string);
        self.voice()
    }

    /// A human-readable description of the active voice.
    pub fn status(&self) -> Option<String> {
        self.voice()
            .map(|v| format!("{} ({})", v.name, v.locale))
    }

    /// Start speaking, cutting off any previous utterance.
    ///
    /// Returns once the audio has started — or, on macOS, as soon as the
    /// utterance has been *rendered* and its player started. It does not wait for
    /// the sound to finish, so the caller is never blocked by speech.
    pub fn speak(&self, text: &str) -> Result<(), String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("there is nothing to pronounce".into());
        }
        if text.chars().count() > MAX_UTTERANCE {
            return Err(format!(
                "refusing to pronounce {} characters (limit {MAX_UTTERANCE})",
                text.chars().count()
            ));
        }

        let Some(voice) = self.voice() else {
            return Err(no_voice_message());
        };

        #[cfg(target_os = "macos")]
        {
            // Rendered in full before anything is heard, then played. Killing a
            // `say` process mid-utterance is what produced clipped, crackling
            // pronunciation: see [`render`]. The player is stopped rather than
            // the synthesiser, which is a plain file read and can be cut
            // anywhere without a glitch.
            let file = render(text, &voice.name)?;
            self.play(&file)
        }

        #[cfg(target_os = "ios")]
        {
            // Stopping first, and after the voice is known, so a failed lookup
            // cannot silence what is already being said.
            self.stop();
            speak_on_main(text, &voice.name)
        }

        // Android's synthesiser belongs to the Kotlin side of the platform
        // bridge, so speaking is a command to it rather than something kept
        // here. The voice is named, not identified, which is the same contract
        // the other two backends have.
        #[cfg(target_os = "android")]
        {
            self.stop();
            crate::platform::call::<serde_json::Value>(
                "speak",
                serde_json::json!({ "text": text, "voice": voice.name }),
            )
            .map(|_| ())
        }

        #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
        {
            // Windows and Linux are the remaining backends (M6). Returning a
            // clear error beats shelling out to something unverified.
            let _ = voice;
            Err(
                "pronunciation is not implemented on this platform yet; \
                 it uses the system synthesiser on macOS, iOS and Android"
                    .into(),
            )
        }
    }

    /// Get the platform's output path ready before the learner asks for it.
    ///
    /// A no-op everywhere but iOS, where there is something expensive and
    /// glitch-prone to do in advance — see [`audio_ready`]. Called from the voice
    /// warm-up thread rather than from startup, so none of it is on the path that
    /// shows the first screen.
    pub fn prime(&self) {
        #[cfg(target_os = "ios")]
        if let Err(problem) = audio_ready() {
            // Not fatal and not worth a dialog: a cold route costs the crackle
            // fix, not the speech. `speak_on_main` takes the session again.
            eprintln!("[speech] could not warm the audio route: {problem}");
        }

        // macOS has its own cold start, and it sounds like crackle rather than
        // like silence: the first `afplay` or two after the machine has been quiet
        // underrun the output device while CoreAudio sets it up, which is the
        // "first few tones are rough, then it is clear" report. Warming the device
        // once here — a player started on a silent file and stopped immediately —
        // leaves the first real utterance playing into a device that is already
        // awake. The voice list is resolved on the same pass, so the first
        // pronunciation does not pay for that either.
        #[cfg(target_os = "macos")]
        {
            let _ = self.voices();
            match silence() {
                Ok(file) => {
                    if let Err(problem) = self.play(&file) {
                        // Not fatal: this costs the warm-up, not the speech.
                        eprintln!("[speech] could not warm the audio device: {problem}");
                    }
                    self.stop();
                }
                Err(problem) => {
                    eprintln!("[speech] could not prepare the audio warm-up: {problem}");
                }
            }
        }
    }

    /// Stop the current utterance, if any.
    ///
    /// On macOS this stops the *player*, which is a file read that can be
    /// interrupted anywhere without a glitch — it never touches the synthesiser.
    /// On iOS it asks the synthesiser to stop, because it keeps its own queue and
    /// dropping it would leave the queue speaking with nothing able to stop it.
    pub fn stop(&self) {
        #[cfg(target_os = "macos")]
        {
            // Taken out of the lock before it is reaped: `wait` blocks, and
            // holding the speaker's lock across it would stall every other call
            // for as long as the player took to die.
            let running = self.lock().take();
            if let Some(mut child) = running {
                // A child that already finished makes `kill` fail harmlessly;
                // the `wait` afterwards is what actually reaps it. SIGTERM, not
                // the default SIGKILL: the player tears its audio unit down on
                // the way out, and it exits in about a millisecond.
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        #[cfg(target_os = "ios")]
        // Cutting off a queued utterance cannot be a no-op: the synthesiser
        // would finish it in its own time, which is the one thing "stop" may not
        // do. See `stop_on_main`.
        stop_on_main();
        // Android's utterance lives on the Kotlin side, so this is a command
        // too. A failure to reach it is deliberately ignored: it means nothing
        // is speaking through that side, which is what `stop` wanted anyway.
        #[cfg(target_os = "android")]
        {
            let _ = crate::platform::call::<serde_json::Value>("stop", ());
        }
    }

    /// Cut off the current player and start `file`.
    ///
    /// macOS only. The stop happens here rather than in [`Self::speak`] so that
    /// the rendering — the slow part — happens *before* anything is silenced: a
    /// learner drilling one syllable after another keeps hearing the previous one
    /// until the next is ready, instead of getting a gap.
    #[cfg(target_os = "macos")]
    fn play(&self, file: &std::path::Path) -> Result<(), String> {
        self.stop();
        let child = Command::new("/usr/bin/afplay")
            .arg(file)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("could not start the audio player: {e}"))?;
        self.hold(child);
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn hold(&self, child: Child) {
        *self.lock() = Some(child);
    }

    /// Take the lock, ignoring poisoning: a panic while holding it cannot leave
    /// the utterance handle in a state that matters. macOS only, like the field.
    #[cfg(target_os = "macos")]
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Utterance>> {
        self.current.lock().unwrap_or_else(|e| e.into_inner())
    }
}

// ---------------------------------------------------------------------------
// macOS: render once, then play the file
// ---------------------------------------------------------------------------
//
// ## Why speech is not streamed straight out of `say`
//
// It used to be: `say -v <voice> -- <text>` was spawned and its process kept, so
// that the next utterance could kill it. That produced pronunciation that was
// **clipped and crackling on macOS and nowhere else**, and the reason is a race
// this code created rather than a fault in the synthesiser.
//
// `say` needs roughly a third of a second before it starts making sound — the
// speech daemon has to answer and a CoreAudio unit has to be built. Killing it
// before that, which is what happens whenever a learner taps a second character
// while the first is still being set up, tears the audio unit down mid-stream.
// Measured on this machine with the app's own pattern — spawn, wait 0.5 s, SIGKILL
// — each attempt was audible for about 170 ms of a 4.4 s utterance. The click at
// the cut is the crackle. Android and iOS never had this because neither of them
// is a process being killed: Android's synthesiser runs in Kotlin and is asked to
// stop, and iOS's is asked to stop in process.
//
// **A second, quieter fault made the first one worse.** `say` exits 0 and writes
// about 11 ms of near-silence when it cannot use the voice it was named — no
// error, no message, an empty output on both streams. So any name this crate
// resolved but the synthesiser could not use sounded exactly like being cut off,
// with nothing to report. [`render`] now measures what came out and refuses to
// call it speech.
//
// ## What replaced it
//
// Render the whole utterance to a file with `say -o`, check that the file is
// really speech, then play it with `afplay`. Three things fall out of that:
//
// - **Interruption is safe.** The process being killed is playing a file, not
//   synthesising; there is no audio unit to leave half-built.
// - **It is also faster, not slower.** `say -o` renders any length in about
//   0.9 s because it does not wait for playback, so a render plus a play of a
//   short word is ~1.05 s against ~1.8 s for `say` streaming it live. A repeat is
//   a cache hit and costs the play alone.
// - **A failure can be reported.** An empty render is a sentence for the learner
//   instead of silence.
//
// The cache is per-process and lives in the system temp directory, which is the
// one place this app is guaranteed to be able to write. That also means it cannot
// grow without bound across runs: a fresh directory is rendered on each launch,
// and the previous one is removed by the OS.

/// Whether this process has cleared its cache directory yet.
#[cfg(target_os = "macos")]
static CACHE_PREPARED: std::sync::Once = std::sync::Once::new();

/// Render `text` in `voice` and return the file holding it.
///
/// Cached by text and voice, because the whole point of this app is saying the
/// same few characters over and over. `voice` is the synthesiser's own name for
/// it, not the display name, so two names that resolve to the same voice share an
/// entry.
#[cfg(target_os = "macos")]
fn render(text: &str, voice: &str) -> Result<std::path::PathBuf, String> {
    let dir = cache_dir()?;
    CACHE_PREPARED.call_once(|| {
        // A directory left by a previous run is cleared rather than reused: the
        // files are keyed by a hash, and the text behind a hash is not knowable
        // from the file itself. Failing to clear is not fatal — the next render
        // simply writes over the entry it needs.
        let _ = std::fs::remove_dir_all(&dir);
    });
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create the speech cache at {}: {e}", dir.display()))?;

    let file = dir.join(format!("{}.aiff", cache_key(text, voice)));
    if is_speech(&file) {
        return Ok(file);
    }

    let output = Command::new("/usr/bin/say")
        .arg("-v")
        .arg(voice)
        // `--` so that text beginning with a dash is still read as text.
        .arg("-o")
        .arg(&file)
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("could not start the speech synthesiser: {e}"))?;

    if !output.status.success() {
        // Both streams, because which one carries the complaint has varied
        // between macOS releases, and a silent failure here is what this whole
        // path exists to stop being silent.
        let complaint = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let said = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = match (complaint.is_empty(), said.is_empty()) {
            (false, _) => complaint,
            (true, false) => said,
            (true, true) => format!("no message, {}", output.status),
        };
        return Err(format!(
            "the speech synthesiser could not speak {text:?} in voice {voice:?}: {detail}"
        ));
    }

    if !is_speech(&file) {
        // The voice named is one the synthesiser will not use — see the module
        // note above. Say so, because the alternative is a silence the learner
        // cannot tell from a bug.
        return Err(format!(
            "the system voice {voice:?} produced no audio for {text:?}; \
             choose another voice in Settings, or install the Chinese voice for this system"
        ));
    }

    Ok(file)
}

/// Where rendered utterances live for the life of this process.
///
/// The system temp directory, because it is the one place every platform this
/// app runs on is guaranteed to allow a write — including a sandboxed mobile
/// bundle and a **sandboxed build harness**, where a child process may be denied
/// directories the parent can use. `HANZI_TUTOR_SPEECH_CACHE` overrides it, which
/// is what a restricted environment points at a writable directory.
#[cfg(target_os = "macos")]
fn cache_dir() -> Result<std::path::PathBuf, String> {
    if let Some(dir) = std::env::var_os("HANZI_TUTOR_SPEECH_CACHE") {
        let dir = std::path::PathBuf::from(dir);
        if !dir.as_os_str().is_empty() {
            return Ok(dir);
        }
    }
    Ok(std::env::temp_dir().join("hanzi-speech-cache"))
}

/// True when `file` holds audio long enough to be a spoken syllable.
#[cfg(target_os = "macos")]
fn is_speech(file: &std::path::Path) -> bool {
    match rendered_ms(file) {
        Some(ms) => ms >= MIN_RENDER_MS,
        None => false,
    }
}

/// The length of a rendered file in milliseconds, or `None` when it cannot be
/// read.
///
/// The two header fields are the whole of what is needed: every byte of a
/// 16-bit PCM AIFF is audio, so `(data size / channels / 2) / rate` is exactly
/// the duration. Deliberately not a call to `afinfo`: this runs on every play,
/// and a process spawn to answer a question two integers already answer would be
/// the slowest thing in the path.
#[cfg(target_os = "macos")]
fn rendered_ms(file: &std::path::Path) -> Option<u64> {
    render_header(file).map(|(rate, channels, frames, _bytes_per_sample)| {
        if rate == 0 || channels == 0 {
            return 0;
        }
        frames as u64 * 1000 / (rate as u64 * channels as u64)
    })
}

/// A short, silent PCM file, for waking the output device.
///
/// Written by hand rather than rendered: `say -o` would need a voice and would
/// make this a synthesis problem, and the point is only to give the player
/// something valid to open. **22,050 Hz mono 16-bit PCM**, which is the format
/// `say` itself writes (see [`render_header`]), so a warm-up can never fail for a
/// reason a real utterance would not.
///
/// Overwritten on every call rather than cached: it is 1,024 bytes, it is written
/// once per process, and a file that is always the same cannot be stale.
#[cfg(target_os = "macos")]
fn silence() -> Result<std::path::PathBuf, String> {
    const RATE: u32 = 22_050;
    /// Long enough for the device to come up, short enough to be inaudible even
    /// if the stop below were missed — which cannot happen, since it is silence.
    const FRAMES: u32 = 512;

    let dir = cache_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("could not create the speech cache at {}: {e}", dir.display()))?;
    let file = dir.join("silence.aiff");
    std::fs::write(&file, silent_aiff(RATE, FRAMES))
        .map_err(|e| format!("could not write the warm-up file: {e}"))?;
    Ok(file)
}

/// The bytes of a silent mono 16-bit PCM AIFF holding `frames` samples.
///
/// Laid out as `FORM` → `AIFC` → `COMM` → `SSND`, all big-endian, which is what
/// the [`render_header`] reader expects — so the warm-up file parses like any
/// other render and could be measured with the same code.
#[cfg(target_os = "macos")]
fn silent_aiff(rate: u32, frames: u32) -> Vec<u8> {
    let payload = frames as usize * 2;
    let mut out = Vec::with_capacity(payload + 64);
    out.extend_from_slice(b"FORM");
    // The FORM length covers everything after this field's 8 bytes: the four
    // format bytes and both chunks with their headers. The reader ignores it, but
    // a player may not, so it is written correctly.
    out.extend_from_slice(&(4u32 + (8 + 18) + (8 + 8 + payload as u32)).to_be_bytes());
    out.extend_from_slice(b"AIFC");

    out.extend_from_slice(b"COMM");
    out.extend_from_slice(&18u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes()); // one channel
    out.extend_from_slice(&frames.to_be_bytes());
    out.extend_from_slice(&16u16.to_be_bytes()); // bits per sample
    out.extend_from_slice(&extended_f64(rate as f64));

    out.extend_from_slice(b"SSND");
    out.extend_from_slice(&(payload as u32 + 8).to_be_bytes());
    out.extend_from_slice(&[0u8; 8]); // offset and block size
    out.extend(std::iter::repeat_n(0u8, payload));
    out
}

/// A `f64` as the 80-bit extended float AIFF stores sample rates in.
///
/// The inverse of what [`extended_u32`] reads. Used only by the warm-up file,
/// which is why it takes a `f64` rather than an integer: the encoding is about
/// the format, not about the value being whole.
#[cfg(target_os = "macos")]
fn extended_f64(value: f64) -> [u8; 10] {
    let mut out = [0u8; 10];
    if value <= 0.0 || !value.is_finite() {
        return out;
    }
    // Normalise into [1, 2) and record the binary exponent, biased by 16,383.
    let mut exponent = 0i32;
    let mut mantissa = value;
    while mantissa >= 2.0 {
        mantissa /= 2.0;
        exponent += 1;
    }
    while mantissa < 1.0 {
        mantissa *= 2.0;
        exponent -= 1;
    }
    let biased = (exponent + 16_383) as u16;
    out[0..2].copy_from_slice(&biased.to_be_bytes());
    // The explicit integer bit, then 63 fraction bits.
    let scaled = (mantissa * (1u64 << 63) as f64) as u64;
    out[2..10].copy_from_slice(&scaled.to_be_bytes());
    out
}

/// A hash of `(text, voice)`, as the file name of that utterance's rendering.
///
/// FNV-1a, written out rather than taken from a crate: the name only has to be
/// stable within one process and not collide among the few hundred utterances a
/// drill produces, and a `u64` of hex is a fine file name. The voice is part of
/// the key because the same character in two voices is two different recordings.
#[cfg(target_os = "macos")]
fn cache_key(text: &str, voice: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in voice.bytes().chain(std::iter::once(0)).chain(text.bytes()) {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// `(sample rate, channels, sample frames, bytes per sample)` from an AIFF file.
///
/// Written out rather than shelling out to `afinfo`, which is a process spawn per
/// play, and rather than adding an audio-file crate for two integers. `say`
/// writes PCM (AIFF-C with `twos` or `sowt`, or plain AIFF), so the frame count in
/// the `COMM` chunk is the whole truth about the duration — no decoding, no
/// estimate. Verified against `afinfo`: 10,332 frames at 22,050 Hz reports
/// 0.468571 s, which is 10332 / 22050 exactly.
#[cfg(target_os = "macos")]
fn render_header(file: &std::path::Path) -> Option<(u32, u16, u32, u16)> {
    let bytes = std::fs::read(file).ok()?;
    let tag = |at: usize| bytes.get(at..at + 4);

    // `FORM` then a big-endian length then `AIFF` (or `AIFC`). The length is not
    // used: a truncated file should be read for whatever chunks it does hold, and
    // the caller's duration check is what rejects it.
    if tag(0)? != b"FORM" {
        return None;
    }
    if tag(8)? != b"AIFF" && tag(8)? != b"AIFC" {
        return None;
    }

    let mut at = 12usize;
    while at + 8 <= bytes.len() {
        let id = tag(at)?;
        let size = be_u32(&bytes, at + 4)? as usize;
        let body = at + 8;
        if id == b"COMM" && body + 18 <= bytes.len() {
            let channels = be_u16(&bytes, body)?;
            let frames = be_u32(&bytes, body + 2)?;
            let bits = be_u16(&bytes, body + 6)?;
            let rate = extended_u32(&bytes, body + 8)?;
            if rate == 0 || channels == 0 {
                return None;
            }
            return Some((rate, channels, frames, bits / 8));
        }
        // Chunks are padded to an even length, and the pad byte is not counted
        // in the size.
        at = body + size + (size & 1);
    }
    None
}

/// One big-endian `u16`, or `None` when the slice runs out.
#[cfg(target_os = "macos")]
fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let raw: [u8; 2] = bytes.get(at..at + 2)?.try_into().ok()?;
    Some(u16::from_be_bytes(raw))
}

/// One big-endian `u32`, or `None` when the slice runs out.
#[cfg(target_os = "macos")]
fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_be_bytes(raw))
}

/// An 80-bit IEEE 754 extended float, as the sample rate in an AIFF `COMM`
/// chunk, reduced to the integer it holds.
///
/// AIFF's sample rate is an extended-precision float and every real one is a
/// whole number of hertz, so the mantissa's integer part is taken and the
/// fraction dropped. `None` for the one encoding that cannot be a rate: an
/// explicit zero, or a value with the integer bit clear.
#[cfg(target_os = "macos")]
fn extended_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let raw = bytes.get(at..at + 10)?;
    let exponent = u16::from_be_bytes(raw[0..2].try_into().ok()?);
    let sign = exponent & 0x8000 != 0;
    if sign {
        return None;
    }
    let exponent = (exponent & 0x7fff) as i32 - 16_383;
    let mantissa = u64::from_be_bytes(raw[2..10].try_into().ok()?);
    if mantissa == 0 || exponent < 0 {
        // Zero, or a rate below 1 Hz — both meaningless for speech. A rate this
        // small is also a file that is not what it claims, so refusing beats
        // returning a number that would make the duration nonsense.
        return None;
    }
    // The margin stops a file whose exponent is absurd from being read as a
    // plausible rate: 63 bits of shift is already more than a u32 can hold.
    if exponent > 31 {
        return None;
    }
    let value = mantissa >> (63 - exponent);
    u32::try_from(value).ok()
}

#[cfg(target_os = "ios")]
thread_local! {
    /// The synthesiser and its delegate, on the only thread allowed to touch
    /// them.
    ///
    /// A `thread_local` rather than a field or a global because that is what
    /// makes "main thread only" structural instead of a promise: the value is
    /// created by the thread that uses it, and the newtype machinery that keeps
    /// AVFoundation objects out of `Send` structs never has to be worked around.
    /// Everything that reaches it goes through [`with_main`].
    static SPEECH: RefCell<Option<Speech>> = const { RefCell::new(None) };
}

/// The iOS synthesiser, paired with the delegate it only holds weakly.
#[cfg(target_os = "ios")]
struct Speech {
    synthesizer: Retained<AVSpeechSynthesizer>,
    /// `AVSpeechSynthesizer.delegate` is a weak property, so this reference is
    /// what keeps the delegate alive; it is also what gives the audio session
    /// back once an utterance is over.
    _delegate: Retained<SpeechSession>,
}

#[cfg(target_os = "ios")]
impl Speech {
    /// Create the synthesiser and its delegate, on the main thread.
    fn new() -> Self {
        // SAFETY: on the main thread, which is where AVSpeechSynthesizer has to
        // be created and used.
        let synthesizer = unsafe { AVSpeechSynthesizer::new() };
        let delegate = SpeechSession::new();
        // SAFETY: the synthesizer holds its delegate weakly, and this same
        // reference is kept alive by `Speech` for as long as it exists.
        unsafe { synthesizer.setDelegate(Some(ProtocolObject::from_ref(&*delegate))) };
        Self {
            synthesizer,
            _delegate: delegate,
        }
    }
}

#[cfg(target_os = "ios")]
define_class!(
    /// Ends the audio session's involvement when an utterance is over.
    ///
    /// The synthesizer calls this on the main thread — the only thread it runs
    /// on — which is also where AVFoundation's session calls belong.
    ///
    /// SAFETY:
    /// - `NSObject` has no subclassing requirements.
    /// - `SpeechSession` is stateless and does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "HanziTutorSpeechSession"]
    #[ivars = ()]
    struct SpeechSession;

    unsafe impl NSObjectProtocol for SpeechSession {}

    unsafe impl AVSpeechSynthesizerDelegate for SpeechSession {
        #[unsafe(method(speechSynthesizer:didFinishSpeechUtterance:))]
        fn finished(&self, _synthesizer: &AVSpeechSynthesizer, _utterance: &AVSpeechUtterance) {
            utterance_ended();
        }

        #[unsafe(method(speechSynthesizer:didCancelSpeechUtterance:))]
        fn cancelled(&self, _synthesizer: &AVSpeechSynthesizer, _utterance: &AVSpeechUtterance) {
            utterance_ended();
        }
    }
);

/// Allocate and initialise a delegate.
///
/// Outside the `define_class!` block because everything inside one is read as an
/// Objective-C method, and this is an ordinary Rust constructor.
#[cfg(target_os = "ios")]
impl SpeechSession {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        // SAFETY: `init` is `NSObject`'s designated initialiser, and this
        // subclass adds no state of its own to initialise.
        unsafe { msg_send![super(this), init] }
    }
}

/// Run `work` on the main thread and wait for its result.
///
/// Only the main thread may use AVFoundation's objects, and Tauri commands do
/// not promise which thread they arrive on, so every call goes through here. On
/// the main thread already it runs inline — dispatching synchronously to the
/// queue you are standing on is a deadlock, and the round trip buys nothing.
///
/// Capture borrows this for the audio session, which is AVFAudio too and shared
/// with the synthesiser: see `capture::engage_input_session`.
#[cfg(target_os = "ios")]
pub(crate) fn with_main<R: Send + 'static>(work: impl FnOnce() -> R + Send + 'static) -> R {
    if MainThreadMarker::new().is_some() {
        return work();
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    DispatchQueue::main().exec_async(move || {
        let _ = sender.send(work());
    });
    receiver
        .recv()
        .expect("the main thread dropped the speech work without running it")
}

/// Speak `text` in the voice called `name`, on the main thread.
#[cfg(target_os = "ios")]
fn speak_on_main(text: &str, name: &str) -> Result<(), String> {
    let text = text.to_string();
    let name = name.to_string();
    with_main(move || {
        let utterance = utterance_for(&text, &name)?;
        if let Err(problem) = engage_session() {
            // Worth saying, not worth refusing to speak over: this costs
            // volume, not words. Speech still happens, it may just be muted by
            // the Ring/Silent switch the way it was before this call.
            eprintln!("[speech] could not take the audio session: {problem}");
        }
        // This utterance owns the session now, so any hold timer still counting
        // down from the last one must not hand it back mid-word.
        take_session();
        // The synthesiser is cloned out of the `thread_local` rather than the
        // borrow being held across the call: speaking may run a delegate
        // callback on this very thread, and that callback borrows `SPEECH`
        // again. A `RefCell` does not allow that while a `RefMut` is live.
        let synthesizer = SPEECH.with(|slot| {
            let mut slot = slot.borrow_mut();
            let speech = slot.get_or_insert_with(Speech::new);
            speech.synthesizer.clone()
        });
        // SAFETY: on the main thread, and the delegate that the synthesizer
        // refers to is kept alive inside `SPEECH`.
        unsafe { synthesizer.speakUtterance(&utterance) };
        Ok(())
    })
}

/// Start the countdown that gives the audio session back after an utterance.
///
/// The delegate reports both endings — finished and cancelled — but the hold is
/// only armed when the queue really is quiet. [`Speaker::speak`] stops the
/// previous utterance before starting the next, and AVFoundation may deliver
/// that cancellation after its replacement has already begun; arming a release
/// then would race the word now being spoken. That order is also why this never
/// holds a borrow of `SPEECH` while it asks.
///
/// "Quiet" is `Some(false)`, not "not true": a `None` means the callback did not
/// arrive on the thread that owns the synthesizer — an empty `SPEECH` was just
/// created for this thread — and then the state is unknown, so the session is
/// left alone rather than yanked out from under whatever is speaking.
#[cfg(target_os = "ios")]
fn utterance_ended() {
    let still_speaking = SPEECH.with(|slot| {
        slot.borrow().as_ref().map(|speech| {
            // SAFETY: on the main thread, like every call a delegate makes.
            unsafe { speech.synthesizer.isSpeaking() }
        })
    });
    if still_speaking == Some(false) {
        hold_session_then_release();
    }
}

/// Stop whatever is being spoken, on the main thread.
///
/// Nothing here hands the session back. Stopping something fires the
/// cancellation callback, which arms the hold; stopping nothing has no callback
/// to fire and nothing to give back, because whatever is holding the session —
/// the last utterance, or [`audio_ready`] — is already counting down. Releasing
/// here would be the cold start this file now exists to avoid, since
/// [`Speaker::speak`] stops before it speaks.
#[cfg(target_os = "ios")]
fn stop_on_main() {
    with_main(|| {
        // Cloned out of the borrow for the same reason as in `speak_on_main`:
        // stopping can run the cancellation callback inline.
        let synthesizer = SPEECH.with(|slot| {
            slot.borrow()
                .as_ref()
                .map(|speech| speech.synthesizer.clone())
        });
        let Some(synthesizer) = synthesizer else {
            return;
        };
        // SAFETY: on the main thread, and the clone keeps the object alive.
        unsafe { synthesizer.stopSpeakingAtBoundary(AVSpeechBoundary::Immediate) };
    });
}

/// Take the audio session for the duration of one utterance.
///
/// The category is `playback` rather than the default `soloAmbient`, and that is
/// the entire point: iOS silences `soloAmbient` whenever the Ring/Silent switch
/// is on, so on a phone an explicit tap on the pronunciation button produced
/// nothing at all, while the simulator — which has no such switch — played it
/// perfectly. Someone who taps a speaker button has asked to hear something, so
/// the switch does not apply to that request. `duckOthers` lowers whatever else
/// is playing instead of stopping it, and [`release_session`] restores it.
///
/// The mode is the one Apple documents for text-to-speech prompts, so routing
/// behaves on CarPlay and similar outputs as a spoken prompt rather than as
/// music.
#[cfg(target_os = "ios")]
fn engage_session() -> Result<(), String> {
    // SAFETY: on the main thread (see `with_main`).
    unsafe {
        let session = AVAudioSession::sharedInstance();
        session
            .setCategory_mode_options_error(
                AVAudioSessionCategoryPlayback.expect("declared by AVFAudio"),
                AVAudioSessionModeSpokenAudio.expect("declared by AVFAudio"),
                AVAudioSessionCategoryOptions::DuckOthers,
            )
            .map_err(|error| error.localizedDescription().to_string())?;
        session
            .setActive_error(true)
            .map_err(|error| error.localizedDescription().to_string())
    }
}

/// Give the audio session back, so anything that was ducked returns to volume.
#[cfg(target_os = "ios")]
fn release_session() {
    // SAFETY: on the main thread (see `with_main`).
    unsafe {
        let session = AVAudioSession::sharedInstance();
        let _ = session.setActive_withOptions_error(
            false,
            AVAudioSessionSetActiveOptions::NotifyOthersOnDeactivation,
        );
    }
}

/// How long the audio session outlives the last utterance, in milliseconds.
///
/// Zero is the old behaviour — hand the session back the instant the word ends —
/// and it is what left the route cold at the start of the next one. Holding it
/// has a cost, because anything the app ducked stays ducked for this long, so it
/// is a few seconds rather than "until the app goes away": long enough to cover a
/// learner drilling the same character, short enough that their music is back to
/// volume before they have moved on.
#[cfg(target_os = "ios")]
const SESSION_HOLD_MS: u64 = 4_000;

/// Which utterance the session currently belongs to.
///
/// Every take and every arm bumps it. A timer that wakes to find the counter
/// moved on knows a newer utterance has already claimed the session and leaves it
/// alone. Without that, the timer armed by the previous word could release the
/// session out from under the word being spoken now — the same failure the
/// `isSpeaking` check guards against, one step further out in time.
#[cfg(target_os = "ios")]
static SESSION_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Claim the session for the utterance about to be spoken, cancelling any
/// pending release.
///
/// Called on the main thread, like everything else that touches the session.
#[cfg(target_os = "ios")]
fn take_session() {
    SESSION_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
}

/// Arm the delayed release. See [`SESSION_HOLD_MS`].
///
/// The wait happens on a thread of its own and the work is done back on the main
/// thread, where the check and the release cannot be interleaved with a
/// [`take_session`] — a tap that lands while this is asleep must win.
#[cfg(target_os = "ios")]
fn hold_session_then_release() {
    let generation = SESSION_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(SESSION_HOLD_MS));
        with_main(move || {
            if SESSION_GENERATION.load(std::sync::atomic::Ordering::SeqCst) == generation {
                release_session();
            }
        });
    });
}

/// Build the synthesiser and start the audio route before anyone needs them.
///
/// **Why this exists.** On the phone, the first tap on "Hear it" after a period
/// of silence crackled, and a second tap on the same character was clean.
/// Nothing about the utterance differed; what differed is that the first tap paid
/// for three things at once, back to back, with the word already being rendered
/// into an audio unit that had not finished starting: constructing the
/// `AVSpeechSynthesizer`, activating the `AVAudioSession` for `playback`, and
/// bringing the output hardware out of its idle power state. AVFoundation begins
/// feeding buffers as soon as `speakUtterance` returns, and a buffer rendered
/// before the hardware is running is heard as a crackle. Doing those three before
/// there is any speech to lose leaves the first tap nothing to pay for.
///
/// The session is handed back by the same idle timer an utterance uses, so this
/// does not duck the learner's music for the life of the app — only for
/// [`SESSION_HOLD_MS`] after launch, when nothing is playing to duck anyway. A
/// tap arriving after that window still meets a cold route; holding the session
/// indefinitely would fix that too, at the price of keeping the learner's music
/// down the whole time they are practising.
#[cfg(target_os = "ios")]
fn audio_ready() -> Result<(), String> {
    with_main(|| {
        // Building the synthesiser is the part that only has to happen once.
        SPEECH.with(|slot| {
            let mut slot = slot.borrow_mut();
            let _ = slot.get_or_insert_with(Speech::new);
        });
        engage_session()?;
        take_session();
        hold_session_then_release();
        Ok(())
    })
}

/// An utterance for `text`, spoken in the voice called `name`.
///
/// The voice is looked up by name each time rather than held as an object: the
/// resolved [`Voice`] is platform-neutral — a name and a locale, so the status
/// line reads the same on both systems — and the alternative, an identifier
/// smuggled through `locale`, would make that field a lie.
#[cfg(target_os = "ios")]
fn utterance_for(text: &str, name: &str) -> Result<Retained<AVSpeechUtterance>, String> {
    let string = NSString::from_str(text);
    // SAFETY: on the main thread (see `with_main`).
    let utterance = unsafe { AVSpeechUtterance::speechUtteranceWithString(&string) };
    let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
    let wanted = voices
        .iter()
        .find(|voice| base_name(&unsafe { voice.name() }.to_string()).eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("the voice {name:?} is no longer available on this device"))?;
    unsafe { utterance.setVoice(Some(&wanted)) };
    Ok(utterance)
}

impl Drop for Speaker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn no_voice_message() -> String {
    // The path through Settings differs enough between the systems to be worth
    // getting right: telling someone on a phone to open "System Settings"
    // sends them looking for a window that does not exist, and Android's own
    // settings are somewhere else again.
    #[cfg(target_os = "ios")]
    const WHERE: &str = "Settings → Accessibility → Spoken Content → Voices";
    #[cfg(target_os = "android")]
    const WHERE: &str = "Settings → System → Languages & input → Text-to-speech output";
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    const WHERE: &str =
        "System Settings → Accessibility → Spoken Content → System Voice → Manage Voices";

    format!(
        "no Chinese voice is installed, so pronunciation is unavailable. \
         Add one in {WHERE}, or set {VOICE_OVERRIDE} to a voice name."
    )
}

/// The voice named by [`VOICE_OVERRIDE`], if it is set to anything usable.
///
/// The override outranks the stored preference, because that is what an
/// environment variable is for: it is the escape hatch for a run that has to be
/// reproducible, and a settings row silently outranking it would make
/// `HANZI_TUTOR_VOICE=… pnpm run dev` a lie.
fn override_voice() -> Option<Voice> {
    let name = std::env::var(VOICE_OVERRIDE).ok()?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(Voice::local(name, "override"))
}

/// The voice that will be used: the override, then the learner's choice, then
/// the automatic pick.
///
/// A function of the installed voices, the preference and the override, so every
/// ordering rule here is testable without a synthesiser and without an
/// environment variable in the way — the caller reads [`VOICE_OVERRIDE`] and
/// passes the result in. The three inputs are also the three things that can
/// change, and keeping the decision in one function is what makes "does my
/// choice actually take effect?" answerable by reading this.
fn resolve_voice(
    installed: &[Voice],
    preferred: Option<&str>,
    overridden: Option<&str>,
) -> Option<Voice> {
    // An override is not "a preference that wins": it does not even have to name
    // an installed voice, because it exists to hand the synthesiser a name the
    // list does not know about.
    if let Some(name) = overridden {
        return Some(Voice::local(name, "override"));
    }
    if let Some(wanted) = preferred {
        if let Some(voice) = find_voice(installed, wanted) {
            return Some(voice);
        }
        // The preference names a voice this machine does not have. Say so where
        // somebody will see it — the settings screen shows which voice is really
        // in use — and carry on with the automatic choice, because refusing to
        // pronounce anything would be a far worse answer than a different voice.
        eprintln!(
            "[speech] the chosen voice {wanted:?} is not installed here; \
             falling back to the automatic choice"
        );
    }
    pick_voice(installed)
}

/// Find an installed voice by name, ignoring the locale qualifier macOS appends.
///
/// The exact name wins over the base name, so a machine that has both
/// `Meijia` and `Meijia (Chinese (China mainland))` uses the one that was named
/// exactly, and a preference written on one of those machines still finds a
/// voice on the other.
fn find_voice(voices: &[Voice], wanted: &str) -> Option<Voice> {
    voices
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(wanted))
        .or_else(|| {
            voices
                .iter()
                .find(|v| base_name(&v.name).eq_ignore_ascii_case(wanted))
        })
        .cloned()
}

/// Enumerate installed voices.
#[cfg(target_os = "macos")]
fn list_voices() -> Result<Vec<Voice>, String> {
    let output = Command::new("/usr/bin/say")
        .arg("-v")
        .arg("?")
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not list the installed voices: {e}"))?;

    // Which stream the list goes to has varied between macOS releases, so read
    // both rather than depending on one.
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(parse_voices(&text))
}

/// Enumerate the installed voices through `AVSpeechSynthesizer`.
///
/// iOS reports a BCP-47 tag (`zh-CN`) where macOS reports `zh_CN`; `pick_voice`
/// normalises the separator, so the same mainland preference holds on both.
#[cfg(target_os = "ios")]
fn list_voices() -> Result<Vec<Voice>, String> {
    // Only `Voice` — two `String`s — crosses back out of the main thread; the
    // `NSArray` of voices does not leave it.
    Ok(with_main(|| {
        // SAFETY: on the main thread (see `with_main`).
        let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
        voices
            .iter()
            .map(|voice| {
                // SAFETY: as above.
                Voice::local(unsafe { voice.name() }.to_string(), unsafe { voice.language() }.to_string())
            })
            .collect::<Vec<Voice>>()
    }))
}

/// Enumerate the installed voices through Android's own `TextToSpeech`.
///
/// The Kotlin side reports a BCP-47 tag (`zh-CN`) and Android's own voice names
/// (`zh-cn-x-ccc-local`). `pick_voice` normalises the separator and matches on
/// the locale, so the mainland preference holds; none of Android's names are on
/// the macOS preferred list, which only means the automatic choice falls through
/// to "any mainland voice" — and the Kotlin side has already ordered those
/// on-device-first, because a network voice would quietly break the promise that
/// this app needs no network.
#[cfg(target_os = "android")]
fn list_voices() -> Result<Vec<Voice>, String> {
    /// The shape `PlatformPlugin.voices` answers with.
    #[derive(serde::Deserialize)]
    struct Listing {
        voices: Vec<Voice>,
    }

    let listing: Listing = crate::platform::call("voices", ())?;
    Ok(listing.voices)
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
fn list_voices() -> Result<Vec<Voice>, String> {
    Ok(Vec::new())
}

/// Parse `say -v '?'` output: `Name   locale   # sample sentence`.
///
/// Only the macOS backend feeds this, but the tests are what keep it honest, so
/// it is compiled wherever either exists — and not on iOS, where neither does
/// and an unused function would fail the target's `-D warnings` clippy run.
///
/// Names may contain spaces and parentheses — `Eddy (Chinese (China
/// mainland))` is a real one — so the locale is taken as the last
/// whitespace-separated field before the sample comment, and everything before
/// it is the name.
#[cfg(any(target_os = "macos", test))]
fn parse_voices(output: &str) -> Vec<Voice> {
    output
        .lines()
        .filter_map(|line| {
            let before_sample = line.split('#').next()?.trim_end();
            if before_sample.is_empty() {
                return None;
            }
            let mut fields = before_sample.rsplitn(2, char::is_whitespace);
            let locale = fields.next()?.trim();
            let name = fields.next().unwrap_or("").trim();
            if name.is_empty() || locale.is_empty() {
                return None;
            }
            Some(Voice::local(name, locale))
        })
        .collect()
}

/// The voices a Chinese character can be spoken with, sorted by name.
///
/// Filtered to the Chinese locales, sorted so that the settings screen's list
/// does not reorder itself between one launch and the next — the order the
/// system reports is not a promise — and keeping each voice's locale, which is
/// the only thing that tells 美佳's `zh_TW` apart from a mainland voice of a
/// similar name. A function of the list so it can be tested without a
/// synthesiser, which is what keeps the *shape* of the offered list honest.
///
/// **Deduplicated**, because `say -v '?'` lists every voice twice — a known
/// quirk, see HANDOVER §6 — and a list with a repeated name is not merely
/// untidy: the settings screen keys its options by name, so a duplicate is a
/// rendering error that takes the screen down. The first entry for a name wins;
/// the duplicates are the same voice.
fn chinese_voices(all: &[Voice]) -> Vec<Voice> {
    let mut voices: Vec<Voice> = Vec::new();
    for voice in all {
        if !voice
            .locale
            .replace('-', "_")
            .to_ascii_lowercase()
            .starts_with("zh")
        {
            continue;
        }
        if voices
            .iter()
            .any(|seen: &Voice| seen.name.eq_ignore_ascii_case(&voice.name))
        {
            continue;
        }
        voices.push(voice.clone());
    }
    voices.sort_by(|a, b| a.name.cmp(&b.name));
    voices
}

/// The bare voice name, without the locale qualifier macOS appends.
///
/// macOS reports names as `Tingting (Chinese (China mainland))` and
/// `Eddy (Chinese (China mainland))`, so matching a preference against the full
/// name would silently never fire.
fn base_name(name: &str) -> &str {
    match name.find(" (") {
        Some(index) => &name[..index],
        None => name,
    }
}

/// Choose the best voice for Mandarin from those installed.
///
/// Mainland simplified Chinese (`zh_CN`) is preferred, then any Chinese locale.
/// Some macOS releases tag the mainland locale `zh-CN`, so the separator is
/// normalised before matching.
fn pick_voice(voices: &[Voice]) -> Option<Voice> {
    let key = |v: &Voice| v.locale.replace('-', "_").to_ascii_lowercase();
    let mandarin = |v: &&Voice| key(v).starts_with("zh_cn");
    let preferred = |v: &&Voice| {
        PREFERRED_NAMES
            .iter()
            .any(|name| base_name(&v.name).eq_ignore_ascii_case(name))
    };
    let chinese = |v: &&Voice| key(v).starts_with("zh");

    voices
        .iter()
        .find(|v| mandarin(v) && preferred(v))
        .or_else(|| voices.iter().find(mandarin))
        .or_else(|| voices.iter().find(chinese))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Albert              en_US    # Hello! My name is Albert.
Alice               it_IT    # Ciao! Mi chiamo Alice.
Eddy (Chinese (China mainland)) zh_CN    # 你好！我叫Eddy。
Meijia              zh_TW    # 你好，我叫美佳。
Sinji               zh_HK    # 你好！我叫善怡。
Tingting (Chinese (China mainland)) zh_CN    # 你好！我叫婷婷。
";

    #[test]
    fn parses_names_containing_spaces_and_parentheses() {
        let voices = parse_voices(SAMPLE);
        assert_eq!(voices.len(), 6);
        assert_eq!(voices[0].name, "Albert");
        assert_eq!(voices[0].locale, "en_US");
        assert_eq!(voices[2].name, "Eddy (Chinese (China mainland))");
        assert_eq!(voices[2].locale, "zh_CN");
        assert_eq!(voices[5].name, "Tingting (Chinese (China mainland))");
        assert_eq!(voices[5].locale, "zh_CN");
    }

    #[test]
    fn strips_the_locale_qualifier_from_a_name() {
        assert_eq!(base_name("Tingting"), "Tingting");
        assert_eq!(
            base_name("Tingting (Chinese (China mainland))"),
            "Tingting"
        );
        assert_eq!(base_name("Eddy (Chinese (Taiwan))"), "Eddy");
    }

    #[test]
    fn ignores_blank_and_malformed_lines() {
        let voices = parse_voices("\n   \nTingting zh_CN # 你好\nonlyname\n");
        assert_eq!(voices.len(), 1);
        assert_eq!(voices[0].name, "Tingting");
    }

    #[test]
    fn prefers_mainland_mandarin_and_the_named_voice() {
        let voices = parse_voices(SAMPLE);
        let picked = pick_voice(&voices).expect("a Chinese voice is present");
        // Tingting is zh_CN and on the preferred list, so it wins over the
        // other zh_CN voice and over the zh_TW / zh_HK entries.
        assert_eq!(base_name(&picked.name), "Tingting");
    }

    /// The genuine shape of the voice list, where every voice carries a locale
    /// qualifier in its name. Matching preferences against the *full* name
    /// silently never fired here — this fixture is what caught that.
    #[test]
    fn prefers_tingting_over_the_expressive_voices() {
        let real = "\
Eddy (Chinese (China mainland)) zh_CN    # 你好！我叫Eddy。
Flo (Chinese (China mainland)) zh_CN    # 你好！我叫Flo。
Sandy (Chinese (China mainland)) zh_CN    # 你好！我叫Sandy。
Tingting (Chinese (China mainland)) zh_CN    # 你好！我叫婷婷。
Eddy (Chinese (Taiwan)) zh_TW    # 你好，我叫Eddy。
Meijia              zh_TW    # 你好，我叫美佳。
Sinji               zh_HK    # 你好！我叫善怡。
";
        let picked = pick_voice(&parse_voices(real)).expect("a Chinese voice");
        assert_eq!(base_name(&picked.name), "Tingting");

        // Without Tingting, a mainland voice is still preferred over the
        // Taiwanese and Cantonese ones.
        let without = parse_voices(real)
            .into_iter()
            .filter(|v| base_name(&v.name) != "Tingting")
            .collect::<Vec<_>>();
        let fallback = pick_voice(&without).expect("a Chinese voice");
        assert_eq!(base_name(&fallback.name), "Eddy");
        assert_eq!(fallback.locale, "zh_CN");
    }

    #[test]
    fn falls_back_through_the_locales() {
        // With no preferred name, any mainland voice will do.
        let no_preferred = parse_voices(SAMPLE)
            .into_iter()
            .filter(|v| base_name(&v.name) != "Tingting")
            .collect::<Vec<_>>();
        assert_eq!(
            base_name(&pick_voice(&no_preferred).unwrap().name),
            "Eddy"
        );

        // With no mainland voice, a Taiwanese one is better than nothing.
        let only_taiwan = parse_voices(SAMPLE)
            .into_iter()
            .filter(|v| v.locale != "zh_CN")
            .collect::<Vec<_>>();
        assert_eq!(pick_voice(&only_taiwan).unwrap().locale, "zh_TW");

        // With no Chinese voice at all, say so rather than speaking English.
        let no_chinese = parse_voices(SAMPLE)
            .into_iter()
            .filter(|v| !v.locale.starts_with("zh"))
            .collect::<Vec<_>>();
        assert!(pick_voice(&no_chinese).is_none());
        assert!(pick_voice(&[]).is_none());
    }

    /// The voices iOS actually reports, captured from the simulator's log: 65
    /// voices installed, of which these are the Chinese ones.
    ///
    /// Genuine names rather than tidy ones because that is exactly what went
    /// wrong on macOS: a fixture of neat names passed while the real list
    /// (`Tingting (Chinese (China mainland))`) never matched the preference, and
    /// the app quietly used another voice.
    const IOS_VOICES: &[(&str, &str)] = &[
        ("Daniel", "en-GB"),
        ("Tingting", "zh-CN"),
        ("Sinji", "zh-HK"),
        ("Meijia", "zh-TW"),
    ];

    #[test]
    fn picks_the_mainland_voice_from_the_ios_voice_list() {
        let voices: Vec<Voice> = IOS_VOICES
            .iter()
            .map(|(name, locale)| Voice::local(*name, *locale))
            .collect();
        let picked = pick_voice(&voices).expect("iOS ships a Mandarin voice");
        assert_eq!(picked.name, "Tingting");
        assert_eq!(picked.locale, "zh-CN");
    }

    #[test]
    fn a_device_with_no_chinese_voice_is_reported_rather_than_guessed() {
        // Nothing Chinese installed: the interface disables pronunciation and
        // explains, rather than reading Chinese in an English voice.
        let voices: Vec<Voice> = IOS_VOICES[..1]
            .iter()
            .map(|(name, locale)| Voice::local(*name, *locale))
            .collect();
        assert_eq!(pick_voice(&voices), None);
        assert!(no_voice_message().contains("no Chinese voice"));
    }

    #[test]
    fn accepts_a_dash_separated_locale() {
        let voices = parse_voices("Tingting zh-CN # 你好\n");
        assert_eq!(pick_voice(&voices).unwrap().name, "Tingting");
    }

    #[test]
    fn a_chosen_voice_beats_the_automatic_choice() {
        // The point of the setting: 美佳 is `zh_TW`, so the automatic rule would
        // never pick it — a learner who wants a Taiwanese voice has to be able
        // to say so, and the setting has to be what actually gets used.
        let voices = parse_voices(SAMPLE);
        assert_eq!(base_name(&pick_voice(&voices).unwrap().name), "Tingting");

        let chosen = resolve_voice(&voices, Some("Meijia"), None).expect("a voice");
        assert_eq!(chosen.name, "Meijia");
        assert_eq!(chosen.locale, "zh_TW");
    }

    #[test]
    fn the_environment_override_outranks_a_stored_choice() {
        // An environment variable is the escape hatch for a reproducible run. A
        // settings row that outranked it would make `HANZI_TUTOR_VOICE=…` a lie,
        // and the override does not have to name an installed voice at all —
        // that is the whole point of it.
        let voices = parse_voices(SAMPLE);
        let overruled = resolve_voice(&voices, Some("Meijia"), Some("Some Unlisted Voice"))
            .expect("the override is used as given");
        assert_eq!(overruled.name, "Some Unlisted Voice");
        assert_eq!(overruled.locale, "override");
    }

    #[test]
    fn a_preference_matches_a_name_with_or_without_its_locale_qualifier() {
        // macOS names the mainland voices `Tingting (Chinese (China mainland))`,
        // so both the bare name the settings screen shows and the full name the
        // system uses have to find the same voice. A fixture of exact names is
        // what previously hid this mismatch.
        let voices = parse_voices(SAMPLE);
        for wanted in ["Tingting", "Tingting (Chinese (China mainland))", "tingting"] {
            let found = find_voice(&voices, wanted).unwrap_or_else(|| panic!("{wanted}"));
            assert_eq!(base_name(&found.name), "Tingting");
        }
        assert!(find_voice(&voices, "Nobody").is_none());
    }

    #[test]
    fn an_exact_name_wins_over_a_base_name_match() {
        let voices = vec![
            Voice::local("Meijia (Chinese (China mainland))", "zh_CN"),
            Voice::local("Meijia", "zh_TW"),
        ];
        assert_eq!(find_voice(&voices, "Meijia").unwrap().locale, "zh_TW");
        assert_eq!(
            find_voice(&voices, "Meijia (Chinese (China mainland))")
                .unwrap()
                .locale,
            "zh_CN"
        );
    }

    #[test]
    fn a_voice_this_machine_does_not_have_falls_back_rather_than_failing() {
        // A preference carried from another machine must not break
        // pronunciation: the automatic choice is used instead, and the settings
        // screen — which asks `voice()` what is really in use — can say so.
        let voices = parse_voices(SAMPLE);
        let picked = resolve_voice(&voices, Some("Nonexistent Voice"), None).expect("a voice");
        assert_eq!(base_name(&picked.name), "Tingting");

        // And with nothing Chinese installed there is still no voice to invent.
        let none: Vec<Voice> = Vec::new();
        assert!(resolve_voice(&none, Some("Meijia"), None).is_none());
    }

    #[test]
    fn only_chinese_voices_are_offered_for_a_chinese_character() {
        // The settings screen's list is built from `chinese_voices`, so an
        // English voice must not appear in it — picking one is what makes the
        // app read 汉 as an English word.
        let offered = chinese_voices(&parse_voices(SAMPLE));
        assert_eq!(offered.len(), 4, "{offered:?}");
        assert!(offered.iter().all(|v| v.locale.starts_with("zh")));
        // Sorted, so the list does not shuffle between launches.
        let mut names: Vec<&str> = offered.iter().map(|v| v.name.as_str()).collect();
        let unsorted = names.clone();
        names.sort();
        assert_eq!(names, unsorted);
        // An iOS-style dash separator is filtered the same way.
        assert_eq!(
            chinese_voices(&[
                Voice::local("Daniel", "en-GB"),
                Voice::local("Tingting", "zh-CN"),
            ])
            .len(),
            1
        );
    }

    /// Whether a voice needs a network is carried to the settings screen.
    ///
    /// Android offers a network voice and an on-device one for the same locale —
    /// `zh-cn-x-ccc-network` and `zh-cn-x-ccc-local` are the real names — and
    /// this flag is the only thing that tells them apart. It matters here more
    /// than in most apps: the whole premise is that this one works with no
    /// network, so a learner who picked the wrong one of the pair would find
    /// pronunciation quietly dependent on being online.
    #[test]
    fn a_voice_carries_whether_it_needs_a_network() {
        let network = Voice {
            network: true,
            ..Voice::local("zh-cn-x-ccc-network", "zh-CN")
        };
        let on_device = Voice::local("zh-cn-x-ccc-local", "zh-CN");
        let offered = chinese_voices(&[network.clone(), on_device.clone()]);

        assert_eq!(offered.len(), 2, "{offered:?}");
        // Sorted by name for the screen, so `-local` comes before `-network`.
        assert_eq!(offered[0].name, "zh-cn-x-ccc-local");
        assert!(!offered[0].network, "the on-device voice needs no network");
        assert!(offered[1].network, "the network voice says so");
        assert!(network.network && !on_device.network);
    }

    #[test]
    fn a_voice_listed_twice_is_offered_once() {
        // `say -v '?'` really does list every voice twice, and the settings
        // screen keys its options by name — so a duplicate is not untidiness, it
        // is a rendering error that takes the whole screen down. This fixture is
        // the shape of the real command's output: the same lines, twice.
        let real = "\
Tingting (Chinese (China mainland)) zh_CN    # 你好！我叫婷婷。
Meijia              zh_TW    # 你好，我叫美佳。
Daniel              en_GB    # Hello, my name is Daniel.
Tingting (Chinese (China mainland)) zh_CN    # 你好！我叫婷婷。
Meijia              zh_TW    # 你好，我叫美佳。
Daniel              en_GB    # Hello, my name is Daniel.
";
        let parsed = parse_voices(real);
        assert_eq!(parsed.len(), 6, "the parser keeps both copies");

        let offered = chinese_voices(&parsed);
        assert_eq!(
            offered.len(),
            2,
            "but the settings screen is offered each voice once: {offered:?}"
        );
        assert_eq!(offered[0].name, "Meijia");
        assert_eq!(offered[1].name, "Tingting (Chinese (China mainland))");

        // The names are what the screen keys on, so they have to be distinct.
        let mut names: Vec<&str> = offered.iter().map(|v| v.name.as_str()).collect();
        let offered_count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), offered_count, "every offered name is unique");
    }

    #[test]
    fn choosing_a_voice_does_not_change_what_the_speaker_resolves_to_elsewhere() {
        // The preference is the speaker's, not the process's: `set_voice` is the
        // only way in, and going back to `None` is the automatic choice again.
        // (The machine this runs on may have no `say` at all, so the assertions
        // are about `set_voice` returning the same thing `voice()` reports
        // rather than about any particular voice being installed.)
        let speaker = Speaker::default();
        let automatic = speaker.voice().map(|v| v.name);
        let chose = speaker.set_voice(Some("Meijia"));
        assert_eq!(chose.map(|v| v.name), speaker.voice().map(|v| v.name));
        let back = speaker.set_voice(None);
        assert_eq!(back.map(|v| v.name), automatic.clone());
        // A blank name is the automatic choice, not a voice called "".
        speaker.set_voice(Some("   "));
        assert_eq!(speaker.voice().map(|v| v.name), automatic);
    }

    #[test]
    fn refuses_empty_and_absurd_utterances() {
        let speaker = Speaker::default();
        assert!(speaker.speak("").is_err());
        assert!(speaker.speak("   ").is_err());
        let long = "一".repeat(MAX_UTTERANCE + 1);
        let error = speaker.speak(&long).unwrap_err();
        assert!(error.contains("limit"), "unexpected error: {error}");
    }

    /// Enumerating the real voices takes about a second, so it is exercised
    /// once here rather than being part of the app's startup path.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_system_actually_offers_a_chinese_voice() {
        let voices = match list_voices() {
            Ok(voices) if voices.is_empty() => return, // no `say`; nothing to check
            Ok(voices) => voices,
            Err(_) => return,
        };
        assert!(
            voices.len() > 50,
            "expected the full voice list, got {}",
            voices.len()
        );
        assert!(
            pick_voice(&voices).is_some(),
            "no Chinese voice installed on this machine"
        );
    }

    // ---- macOS: the rendered-file backend ---------------------------------
    //
    // Everything below is macOS-only because everything it tests is. The
    // regression it exists for — clipped, crackling pronunciation — could not
    // happen on the other two platforms, which is what pointed at this backend in
    // the first place.

    /// Build an AIFF header of the shape `say` writes, around `payload` bytes of
    /// (fake) samples.
    #[cfg(target_os = "macos")]
    fn aiff(rate: u32, channels: u16, bits: u16, frames: u32, payload: usize) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"FORM");
        // The FORM length is deliberately *wrong* here. Nothing in the reader
        // consults it — a truncated file must still be readable for whatever it
        // holds — and writing it as a lie is what keeps that true.
        out.extend_from_slice(&0u32.to_be_bytes());
        out.extend_from_slice(b"AIFC");

        // A small odd-length chunk before `COMM`, to pin the pad-byte arithmetic:
        // the size is 1, the body is 1 byte, and the pad makes 2 in the file.
        out.extend_from_slice(b"FVER");
        out.extend_from_slice(&1u32.to_be_bytes());
        out.push(0x00);
        out.push(0x00); // the pad byte: not counted in the chunk's size

        out.extend_from_slice(b"COMM");
        out.extend_from_slice(&18u32.to_be_bytes());
        out.extend_from_slice(&channels.to_be_bytes());
        out.extend_from_slice(&frames.to_be_bytes());
        out.extend_from_slice(&bits.to_be_bytes());
        out.extend_from_slice(&extended(rate));

        out.extend_from_slice(b"SSND");
        out.extend_from_slice(&(payload as u32 + 8).to_be_bytes());
        out.extend_from_slice(&[0u8; 8]); // offset and block size
        out.extend(std::iter::repeat_n(0u8, payload));
        out
    }

    /// An 80-bit extended float holding a whole number of hertz.
    #[cfg(target_os = "macos")]
    fn extended(value: u32) -> [u8; 10] {
        let exponent = 16_383 + 31;
        let mantissa = (value as u64) << 32;
        let mut out = [0u8; 10];
        out[0..2].copy_from_slice(&(exponent as u16).to_be_bytes());
        out[2..10].copy_from_slice(&mantissa.to_be_bytes());
        out
    }

    /// Write `bytes` to a scratch file and read it back as a header.
    #[cfg(target_os = "macos")]
    fn header_of(bytes: &[u8]) -> Option<(u32, u16, u32, u16)> {
        let file = std::env::temp_dir().join(format!(
            "hanzi-speech-header-{:p}.aiff",
            bytes.as_ptr()
        ));
        std::fs::write(&file, bytes).expect("a scratch file in the temp directory");
        let read = render_header(&file);
        let _ = std::fs::remove_file(&file);
        read
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_the_sample_rate_out_of_an_aiff_header() {
        // 22,050 Hz — what `say` actually writes — and the other two rates a
        // render can plausibly carry, so a mistake in the extended-float exponent
        // shows up as a wrong rate rather than as a wrong duration only.
        for rate in [16_000u32, 22_050, 44_100] {
            assert_eq!(header_of(&aiff(rate, 1, 16, 100, 200)), Some((rate, 1, 100, 2)));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_the_frame_count_from_the_common_chunk_not_the_payload() {
        // The count is what the COMM chunk says, so a payload padded to the even
        // byte does not change the duration, and neither does a short one.
        assert_eq!(header_of(&aiff(22_050, 1, 16, 10_332, 7)), Some((22_050, 1, 10_332, 2)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn refuses_a_file_that_is_not_aiff() {
        assert_eq!(header_of(b"RIFF\x00\x00\x00\x00WAVE"), None);
        assert_eq!(header_of(b"FORM\x00\x00\x00\x00WAVE"), None);
        assert_eq!(header_of(b""), None);
        // Truncated before the COMM body: nothing to read, and no panic.
        assert_eq!(header_of(b"FORM\x00\x00\x00\x04AIFCCOMM\x00\x00\x00\x12"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn an_eleven_millisecond_render_is_not_speech() {
        // The floor's whole purpose. `say` exits 0 with about 11 ms of
        // near-silence when it cannot use the voice it was named, and before this
        // existed the learner heard that as pronunciation being cut off. Build
        // exactly that file and check it is refused.
        let silent = aiff(22_050, 1, 16, 256, 512);
        let file = std::env::temp_dir().join("hanzi-speech-silent-probe.aiff");
        std::fs::write(&file, &silent).expect("a scratch file in the temp directory");
        assert_eq!(rendered_ms(&file), Some(11));
        assert!(!is_speech(&file), "an 11 ms render must not count as speech");
        let _ = std::fs::remove_file(&file);

        // And that a real utterance does. 300 ms at 22,050 Hz.
        let spoken = aiff(22_050, 1, 16, 6_615, 13_230);
        std::fs::write(&file, &spoken).expect("a scratch file in the temp directory");
        assert_eq!(rendered_ms(&file), Some(300));
        assert!(is_speech(&file), "a 300 ms render is speech");
        let _ = std::fs::remove_file(&file);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_cache_key_separates_text_and_voice() {
        // The same character in two voices is two recordings, and two characters
        // in one voice are two more. A key that ignored the voice would serve one
        // voice's audio for another.
        let a = cache_key("马", "Tingting");
        assert_eq!(a, cache_key("马", "Tingting"), "the key must be stable");
        assert_ne!(a, cache_key("马", "Meijia"));
        assert_ne!(a, cache_key("骂", "Tingting"));
        // A separator, so ("ab", "c") and ("a", "bc") cannot collide.
        assert_ne!(cache_key("ab", "c"), cache_key("a", "bc"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_warm_up_file_is_a_valid_silent_render() {
        // It has to parse like any other render, because the same reader guards
        // it and a format `afplay` refused would make the warm-up itself the bug.
        let file = silence().expect("a warm-up file in the cache directory");
        let (rate, channels, frames, bytes_per_sample) =
            render_header(&file).expect("the warm-up file should carry an AIFF header");
        assert_eq!(rate, 22_050, "the rate `say` itself writes");
        assert_eq!(channels, 1);
        assert_eq!(bytes_per_sample, 2);
        assert_eq!(frames, 512);
        assert_eq!(rendered_ms(&file), Some(23));
        // Silence is not speech, and that is the point: nothing about the warm-up
        // should be mistaken for an utterance.
        assert!(!is_speech(&file));
        let _ = std::fs::remove_file(&file);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_extended_float_round_trips_through_the_reader() {
        // The warm-up writes the sample rate with `extended_f64` and every render
        // is read back with `extended_u32`. A disagreement between the two would
        // be silent, so each rate a render can carry is checked both ways.
        for rate in [8_000u32, 16_000, 22_050, 44_100, 48_000] {
            let encoded = extended_f64(rate as f64);
            assert_eq!(
                extended_u32(&encoded, 0),
                Some(rate),
                "rate {rate} did not survive the round trip"
            );
        }
        // And the degenerate encodings the reader must refuse.
        assert_eq!(extended_u32(&[0u8; 10], 0), None, "zero");
        let mut negative = extended_f64(22_050.0);
        negative[0] |= 0x80; // set the sign bit
        assert_eq!(extended_u32(&negative, 0), None, "a negative rate");
    }

    /// The regression test: a real render really is speech.
    ///
    /// This is the assertion that would have caught the clipping. It renders
    /// through the same function the app speaks with, reads the duration back
    /// with the same reader that guards the play, and pins that a single syllable
    /// is a plausible length rather than a sliver.
    ///
    /// The child process has to be *allowed* to write the cache, which is not the
    /// same question as whether this process can: a sandboxed test harness can
    /// write a directory its own children cannot. When that is the case there is
    /// nothing about the audio to check, so the test says so and stops rather than
    /// reporting a fault in the app — the same shape as the "no Chinese voice
    /// installed" arms.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_rendered_character_is_audible_speech() {
        let speaker = Speaker::default();
        let Some(voice) = speaker.voice() else {
            return;
        };

        let file = match render("马", &voice.name) {
            Ok(file) => file,
            Err(problem) if problem.contains("Opening output file failed") => {
                eprintln!(
                    "skipping: the speech synthesiser cannot write the cache in this \
                     environment ({problem}); set HANZI_TUTOR_SPEECH_CACHE to a writable \
                     directory to run this"
                );
                return;
            }
            Err(problem) => panic!("rendering 马 with {}: {problem}", voice.name),
        };

        let ms = rendered_ms(&file).expect("the rendered file should carry an AIFF header");
        assert!(
            ms >= MIN_RENDER_MS,
            "马 rendered to {ms} ms, below the {MIN_RENDER_MS} ms speech floor — \
             this is the clipped-audio fault the file backend exists to prevent"
        );
        // A single syllable is not a fraction of a second at any speaking rate.
        assert!(ms < 3_000, "马 rendered to an implausible {ms} ms");

        let (rate, channels, frames, _) = render_header(&file).expect("a header");
        assert!(rate >= 8_000, "a speech rate, not {rate} Hz");
        assert!(channels >= 1);
        assert_eq!(frames as u64 * 1000 / rate as u64 / channels as u64, ms);
    }

    /// A second play of the same character does not re-render.
    ///
    /// The cache is what makes a drill feel instant, so it is worth pinning: the
    /// file's modification time must not move when the same text is rendered
    /// again in the same voice.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_repeat_is_served_from_the_cache() {
        let speaker = Speaker::default();
        let Some(voice) = speaker.voice() else {
            return;
        };
        let first = match render("马", &voice.name) {
            Ok(file) => file,
            Err(_) => return, // an environment that cannot write the cache; see above
        };
        let stamp = std::fs::metadata(&first).and_then(|m| m.modified()).ok();
        let second = render("马", &voice.name).expect("a second render");
        assert_eq!(first, second, "the same text and voice must name one file");
        let again = std::fs::metadata(&second).and_then(|m| m.modified()).ok();
        assert_eq!(stamp, again, "the second render rewrote the cached file");
    }

    /// The whole macOS path, actually spoken: render, then play.
    ///
    /// Ignored because it makes an audible sound, which a test run should not do
    /// unasked. Run it with `--ignored --nocapture` when the pronunciation path is
    /// changed, and listen: this is the one check that the sound a learner hears
    /// is the sound intended, and it is what the clipping fault needed.
    ///
    /// ```text
    /// cargo test -p hanzi-voice -- --ignored --nocapture speaks_a_character_out_loud
    /// ```
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "plays audio out loud"]
    fn speaks_a_character_out_loud() {
        let speaker = Speaker::default();
        let Some(voice) = speaker.voice() else {
            eprintln!("skipping: no Chinese voice installed");
            return;
        };
        println!("speaking with {}", voice.name);

        for text in ["妈", "麻", "马", "骂"] {
            speaker.speak(text).unwrap_or_else(|problem| {
                panic!("speaking {text}: {problem}");
            });
            // Long enough to hear one syllable and to prove the next did not cut
            // it off; the drill's own gap is 900 ms.
            std::thread::sleep(std::time::Duration::from_millis(1_200));
        }
        println!("done — every tone should have been complete, with no clicks");
    }
}
