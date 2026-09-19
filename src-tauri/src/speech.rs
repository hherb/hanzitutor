//! Pronunciation, through the operating system's own speech synthesiser.
//!
//! Nothing is downloaded and nothing leaves the machine. On macOS this drives
//! `say`, which ships with the system and already knows how to read Chinese. On
//! iOS it speaks through `AVSpeechSynthesizer` **in process**, because there is
//! no `say` binary there and the app sandbox would refuse to spawn one anyway.
//! Both backends pick a voice with the same rule ([`pick_voice`]), so the
//! mainland-Mandarin preference is one decision rather than two.
//!
//! On iOS the audio session is taken for the duration of an utterance and given
//! back when it ends. That is not decoration: under the default session category
//! iOS mutes speech synthesis whenever the Ring/Silent switch is on, so a tap on
//! an explicit pronunciation button produced nothing on a phone while the very
//! same build spoke perfectly on the simulator, which has no such switch.
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
#[cfg(not(target_os = "ios"))]
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

/// A voice as reported by `say -v '?'`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voice {
    pub name: String,
    pub locale: String,
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
/// macOS holds the `say` process so that killing it stops the speech. iOS holds
/// nothing here — its synthesiser is not `Send` and so cannot live in the shared
/// [`Speaker`]; it lives on the main thread instead (see [`with_main`]).
#[cfg(target_os = "macos")]
type Utterance = Child;
#[cfg(not(any(target_os = "macos", target_os = "ios")))]
type Utterance = ();

/// Pronunciation, with at most one utterance in flight.
#[derive(Default)]
pub struct Speaker {
    /// The utterance in flight, kept so a new one can cut off the last instead
    /// of talking over it.
    #[cfg(not(target_os = "ios"))]
    current: Mutex<Option<Utterance>>,
    /// Resolved on first use: enumerating voices takes about a second, which is
    /// too slow to pay at startup. `AppState` warms it on a background thread.
    voice: OnceLock<Option<Voice>>,
}

impl Speaker {
    /// The voice that will be used, resolving and caching it on first call.
    pub fn voice(&self) -> Option<Voice> {
        self.voice.get_or_init(resolve_voice).clone()
    }

    /// A human-readable description of the active voice.
    pub fn status(&self) -> Option<String> {
        self.voice()
            .map(|v| format!("{} ({})", v.name, v.locale))
    }

    /// Start speaking, cutting off any previous utterance.
    ///
    /// Returns as soon as the synthesiser has been started; it does not wait for
    /// the audio to finish, so the caller is never blocked by speech.
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

        self.stop();

        let Some(voice) = self.voice() else {
            return Err(no_voice_message());
        };

        #[cfg(target_os = "macos")]
        {
            let child = Command::new("/usr/bin/say")
                .arg("-v")
                .arg(&voice.name)
                // `--` so that text beginning with a dash is still read as text.
                .arg("--")
                .arg(text)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("could not start the speech synthesiser: {e}"))?;
            self.hold(child);
            Ok(())
        }

        #[cfg(target_os = "ios")]
        {
            speak_on_main(text, &voice.name)
        }

        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            // Windows and Linux are the remaining backends (M6). Returning a
            // clear error beats shelling out to something unverified.
            let _ = voice;
            Err(
                "pronunciation is not implemented on this platform yet; \
                 it uses the system synthesiser on macOS and iOS"
                    .into(),
            )
        }
    }

    /// Stop the current utterance, if any.
    ///
    /// On macOS that means killing the `say` process and reaping it; on iOS,
    /// asking the synthesiser to stop — it keeps its own queue, and dropping it
    /// would leave the queue speaking with nothing able to stop it.
    pub fn stop(&self) {
        #[cfg(target_os = "macos")]
        {
            let mut slot = self.lock();
            if let Some(mut child) = slot.take() {
                // A child that already finished makes `kill` fail harmlessly;
                // the `wait` afterwards is what actually reaps it.
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        #[cfg(target_os = "ios")]
        // Cutting off a queued utterance cannot be a no-op: the synthesiser
        // would finish it in its own time, which is the one thing "stop" may not
        // do. See `stop_on_main`.
        stop_on_main();
    }

    #[cfg(target_os = "macos")]
    fn hold(&self, child: Child) {
        *self.lock() = Some(child);
    }

    /// Take the lock, ignoring poisoning: a panic while holding it cannot leave
    /// the utterance handle in a state that matters.
    #[cfg(not(target_os = "ios"))]
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Utterance>> {
        self.current.lock().unwrap_or_else(|e| e.into_inner())
    }
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
#[cfg(target_os = "ios")]
fn with_main<R: Send + 'static>(work: impl FnOnce() -> R + Send + 'static) -> R {
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

/// Hand the audio session back once an utterance is over.
///
/// The delegate reports both endings — finished and cancelled — but the session
/// is only released when the queue really is quiet. [`Speaker::speak`] stops the
/// previous utterance before starting the next, and AVFoundation may deliver
/// that cancellation after its replacement has already begun; releasing the
/// session then would cut the new word off mid-syllable. That order is also why
/// this never holds a borrow of `SPEECH` while it asks.
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
        release_session();
    }
}

/// Stop whatever is being spoken, on the main thread.
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
        let speaking = unsafe { synthesizer.isSpeaking() };
        // SAFETY: as above.
        unsafe { synthesizer.stopSpeakingAtBoundary(AVSpeechBoundary::Immediate) };
        // Stopping nothing fires no delegate callback, so without this the
        // session would stay active — and other audio ducked — after a `stop`
        // that had nothing left to cut off.
        if !speaking {
            release_session();
        }
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
    // The path through Settings differs enough between the two systems to be
    // worth getting right: telling someone on a phone to open "System Settings"
    // sends them looking for a window that does not exist.
    #[cfg(target_os = "ios")]
    const WHERE: &str = "Settings → Accessibility → Spoken Content → Voices";
    #[cfg(not(target_os = "ios"))]
    const WHERE: &str =
        "System Settings → Accessibility → Spoken Content → System Voice → Manage Voices";

    format!(
        "no Chinese voice is installed, so pronunciation is unavailable. \
         Add one in {WHERE}, or set {VOICE_OVERRIDE} to a voice name."
    )
}

/// Pick the voice to use, honouring [`VOICE_OVERRIDE`] first.
fn resolve_voice() -> Option<Voice> {
    if let Ok(name) = std::env::var(VOICE_OVERRIDE) {
        let name = name.trim();
        if !name.is_empty() {
            return Some(Voice {
                name: name.to_string(),
                locale: "override".to_string(),
            });
        }
    }
    let voices = list_voices().unwrap_or_default();
    pick_voice(&voices)
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
            .map(|voice| Voice {
                // SAFETY: as above.
                name: unsafe { voice.name() }.to_string(),
                locale: unsafe { voice.language() }.to_string(),
            })
            .collect::<Vec<Voice>>()
    }))
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
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
            Some(Voice {
                name: name.to_string(),
                locale: locale.to_string(),
            })
        })
        .collect()
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
            .map(|(name, locale)| Voice {
                name: (*name).to_string(),
                locale: (*locale).to_string(),
            })
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
            .map(|(name, locale)| Voice {
                name: (*name).to_string(),
                locale: (*locale).to_string(),
            })
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
}
