//! Pronunciation, through the operating system's own speech synthesiser.
//!
//! Nothing is downloaded and nothing leaves the machine. macOS and iOS both
//! speak through `AVSpeechSynthesizer` **in process**: macOS used to shell out
//! to `/usr/bin/say` and `/usr/bin/afplay`, but a sandboxed Mac App Store build
//! cannot spawn either, the same reason iOS never had a subprocess backend to
//! begin with. Both platforms pick a voice with the same rule ([`pick_voice`]),
//! so the mainland-Mandarin preference is one decision rather than two.
//!
//! On iOS the audio session is taken for an utterance and given back a few
//! seconds after it ends, rather than the instant it ends. That is not
//! decoration: under the default session category iOS mutes speech synthesis
//! whenever the Ring/Silent switch is on, so a tap on an explicit pronunciation
//! button produced nothing on a phone while the very same build spoke perfectly
//! on the simulator, which has no such switch. The *hold* is the other half —
//! see [`audio_ready`] for why handing the session straight back made the next
//! word crackle. macOS has no audio session and no Ring/Silent switch, so none
//! of this — [`engage_session`], [`release_session`] and [`audio_ready`] — is
//! compiled there; macOS just speaks.
//!
//! Both platforms fence every AVFoundation call through [`with_main`], but not
//! the same way: iOS dispatches to `DispatchQueue::main()`, serviced by the
//! real app's UIKit run loop, while macOS uses a dedicated worker thread this
//! module owns outright. That difference is load-bearing, not stylistic — see
//! `with_main`'s own doc comment and HANDOVER.md §6 for the two ways the GCD
//! route hung before this was sorted out.
//!
//! The **character** is spoken rather than its pinyin: the synthesiser's
//! Chinese voices have their own lexicon, so handing them 汉 produces the
//! Mandarin reading, whereas handing an English-trained voice the string
//! `hàn` would have it guess at the diacritics. Speaking the character also
//! does not give away how to write it, so it is safe to offer in recall mode —
//! hearing the sound and producing the glyph is exactly the skill being
//! trained.

use std::sync::Mutex;
use std::sync::OnceLock;

// Both backends speak in process. AVFoundation's objects are not `Send`, so
// they are created and used on the main thread and never stored in `Speaker`
// (which is shared): see `with_main`.
#[cfg(any(target_os = "ios", target_os = "macos"))]
use std::cell::RefCell;
// iOS dispatches to the main thread through GCD's main queue, which only a
// real run loop services — see the iOS `with_main`. macOS uses a dedicated
// worker thread instead and needs neither of these.
#[cfg(target_os = "ios")]
use dispatch2::DispatchQueue;
#[cfg(target_os = "ios")]
use objc2::MainThreadMarker;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::rc::Retained;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::runtime::ProtocolObject;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2::{define_class, msg_send, AnyThread};
// The audio session is AVFAudio too, but it is an iOS-only concept — macOS has
// no Ring/Silent switch and nothing to duck — so only iOS imports it.
#[cfg(target_os = "ios")]
use objc2_avf_audio::{
    AVAudioSession, AVAudioSessionCategoryOptions, AVAudioSessionCategoryPlayback,
    AVAudioSessionModeSpokenAudio, AVAudioSessionSetActiveOptions,
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_avf_audio::{
    AVSpeechBoundary, AVSpeechSynthesisVoice, AVSpeechSynthesizer, AVSpeechSynthesizerDelegate,
    AVSpeechUtterance,
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
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

/// Pronunciation, with at most one utterance in flight.
#[derive(Default)]
pub struct Speaker {
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

        #[cfg(any(target_os = "macos", target_os = "ios"))]
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
    /// A no-op everywhere but macOS and iOS, where there is something expensive
    /// and glitch-prone to do in advance — see [`audio_ready`] for the iOS half.
    /// Called from the voice warm-up thread rather than from startup, so none of
    /// it is on the path that shows the first screen.
    pub fn prime(&self) {
        #[cfg(target_os = "ios")]
        if let Err(problem) = audio_ready() {
            // Not fatal and not worth a dialog: a cold route costs the crackle
            // fix, not the speech. `speak_on_main` takes the session again.
            eprintln!("[speech] could not warm the audio route: {problem}");
        }

        // macOS has no audio session to take, but it shares iOS's other cold
        // start: constructing `AVSpeechSynthesizer` for the first time costs
        // something, and paying that before the first tap is what makes the
        // first tap and every later one feel the same. The voice list is
        // resolved on the same pass, so the first pronunciation does not pay
        // for that either.
        #[cfg(target_os = "macos")]
        {
            let _ = self.voices();
            with_main(|| {
                SPEECH.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    let _ = slot.get_or_insert_with(Speech::new);
                });
            });
        }
    }

    /// Stop the current utterance, if any.
    ///
    /// Both macOS and iOS ask the synthesiser to stop, because it keeps its own
    /// queue and dropping it would leave the queue speaking with nothing able to
    /// stop it. See [`stop_on_main`].
    pub fn stop(&self) {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
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
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
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

/// Whether a synthesiser has been constructed yet, on any thread.
///
/// Checked before [`stop_on_main`] dispatches to the main thread at all: if
/// nothing has ever spoken, there is nothing there to stop, and a hop to the
/// main thread to confirm that is a real cost in the shipped app and an
/// unconditional hang under `cargo test`, which has no run loop to answer the
/// hop with. Every test `Speaker` is dropped without ever having spoken, and
/// `Drop for Speaker` calls `stop()` unconditionally — this is what keeps that
/// free rather than a deadlock, on both platforms.
#[cfg(any(target_os = "ios", target_os = "macos"))]
static SPEECH_EVER_CREATED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// The synthesiser, paired with the delegate it only holds weakly.
#[cfg(any(target_os = "ios", target_os = "macos"))]
struct Speech {
    synthesizer: Retained<AVSpeechSynthesizer>,
    /// `AVSpeechSynthesizer.delegate` is a weak property, so this reference is
    /// what keeps the delegate alive; on iOS it is also what gives the audio
    /// session back once an utterance is over.
    _delegate: Retained<SpeechSession>,
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
impl Speech {
    /// Create the synthesiser and its delegate, on the main thread.
    fn new() -> Self {
        SPEECH_EVER_CREATED.store(true, std::sync::atomic::Ordering::Relaxed);
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

#[cfg(any(target_os = "ios", target_os = "macos"))]
define_class!(
    /// Ends the audio session's involvement when an utterance is over, on iOS.
    ///
    /// The synthesizer calls this on the main thread — the only thread it runs
    /// on — which is also where AVFoundation's session calls belong. macOS has
    /// no session to end, so `utterance_ended` is a no-op there beyond the
    /// `isSpeaking` check.
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
#[cfg(any(target_os = "ios", target_os = "macos"))]
impl SpeechSession {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        // SAFETY: `init` is `NSObject`'s designated initialiser, and this
        // subclass adds no state of its own to initialise.
        unsafe { msg_send![super(this), init] }
    }
}

/// Run `work` on the main thread and wait for its result, on iOS.
///
/// Only the main thread may use AVFoundation's objects, and Tauri commands do
/// not promise which thread they arrive on, so every call goes through here. On
/// the main thread already it runs inline — dispatching synchronously to the
/// queue you are standing on is a deadlock, and the round trip buys nothing.
///
/// Capture borrows this for the audio session, which is AVFAudio too and
/// shared with the synthesiser: see `capture::engage_input_session`. This
/// works because the real app's main thread runs a UIKit run loop, which is
/// what actually services `DispatchQueue::main()`; a bare `cargo test` binary
/// has none, but iOS's tests never build into one in the first place — this
/// whole module is `target_os = "ios"`, which a host running `cargo test`
/// never is. macOS needs a different mechanism for exactly that reason: see
/// its own `with_main` below.
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

/// Run `work` on a single dedicated thread and wait for its result, on macOS.
///
/// `AVSpeechSynthesizer` and its delegate are not `Send`, and `SPEECH` is a
/// `thread_local!` for that reason — so *some* one thread has to own it, or
/// two Tauri commands arriving on two different Tokio worker threads would
/// each lazily create their own synthesiser, and `Speaker::stop` on a third
/// thread would find neither. Which thread that is does not matter to
/// AVFoundation the way it does on iOS — nothing here touches `AVAudioSession`
/// or anything else that Apple documents as UI-main-thread-bound — so rather
/// than iOS's `DispatchQueue::main()`, this dispatches to one thread this
/// module spawns and keeps for the life of the process.
///
/// That difference is not cosmetic: `DispatchQueue::main()` is only serviced
/// by a run loop actually running on the process's real main thread, which
/// `tao` provides in the shipped app — but not in `cargo test`, whether the
/// test lives in this crate or, transitively, in `hanzi-tutor`'s (`cfg(test)`
/// does not cross a crate boundary, so a `#[cfg(test)]`-only fix here would
/// not have covered callers in a different crate). A plain worker thread and
/// a channel need no run loop and no `NSApplicationMain`, so this is correct
/// everywhere this code is linked, not merely convenient for tests.
#[cfg(target_os = "macos")]
pub(crate) fn with_main<R: Send + 'static>(work: impl FnOnce() -> R + Send + 'static) -> R {
    type Job = Box<dyn FnOnce() + Send>;
    static WORKER: OnceLock<std::sync::mpsc::SyncSender<Job>> = OnceLock::new();

    let sender = WORKER.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<Job>(0);
        std::thread::Builder::new()
            .name("hanzi-tutor-speech".to_string())
            .spawn(move || {
                for job in receiver {
                    job();
                }
            })
            .expect("spawn the speech worker thread");
        sender
    });

    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    sender
        .send(Box::new(move || {
            let _ = result_sender.send(work());
        }))
        .expect("the speech worker thread outlives every call in, since nothing ever closes it");
    result_receiver
        .recv()
        .expect("the speech worker thread dropped the job without running it")
}

/// Speak `text` in the voice called `name`, on the main thread.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn speak_on_main(text: &str, name: &str) -> Result<(), String> {
    let text = text.to_string();
    let name = name.to_string();
    with_main(move || {
        let utterance = utterance_for(&text, &name)?;
        // The audio session is an iOS-only concept: macOS has no Ring/Silent
        // switch and nothing to duck, so there is nothing to take here.
        #[cfg(target_os = "ios")]
        {
            if let Err(problem) = engage_session() {
                // Worth saying, not worth refusing to speak over: this costs
                // volume, not words. Speech still happens, it may just be muted
                // by the Ring/Silent switch the way it was before this call.
                eprintln!("[speech] could not take the audio session: {problem}");
            }
            // This utterance owns the session now, so any hold timer still
            // counting down from the last one must not hand it back mid-word.
            take_session();
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
/// left alone rather than yanked out from under whatever is speaking. macOS has
/// no session to hold, so it only cares that the delegate fired at all.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn utterance_ended() {
    let still_speaking = SPEECH.with(|slot| {
        slot.borrow().as_ref().map(|speech| {
            // SAFETY: on the main thread, like every call a delegate makes.
            unsafe { speech.synthesizer.isSpeaking() }
        })
    });
    #[cfg(target_os = "ios")]
    if still_speaking == Some(false) {
        hold_session_then_release();
    }
    #[cfg(target_os = "macos")]
    let _ = still_speaking;
}

/// Stop whatever is being spoken, on the main thread.
///
/// Nothing here hands the session back. Stopping something fires the
/// cancellation callback, which arms the hold; stopping nothing has no callback
/// to fire and nothing to give back, because whatever is holding the session —
/// the last utterance, or [`audio_ready`] — is already counting down. Releasing
/// here would be the cold start this file now exists to avoid, since
/// [`Speaker::speak`] stops before it speaks. macOS has no session to give
/// back, so this is the whole of what "stop" means there.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn stop_on_main() {
    // See `SPEECH_EVER_CREATED`: nothing has ever spoken, so there is nothing
    // on the main thread worth asking about.
    if !SPEECH_EVER_CREATED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
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
#[cfg(any(target_os = "ios", target_os = "macos"))]
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

/// Enumerate the installed voices through `AVSpeechSynthesizer`.
///
/// iOS and macOS both report a BCP-47 tag (`zh-CN`); `pick_voice` also accepts
/// the underscore form an older macOS backend used to report, so a preference
/// saved under either still resolves.
///
/// Routed through [`with_main`], like every other AVFoundation call in this
/// file. On iOS that is a dispatch to `DispatchQueue::main()`, serviced by the
/// real app's UIKit run loop; on macOS it is a dedicated worker thread this
/// module owns outright, needing no run loop at all — see macOS's own
/// `with_main` for why that distinction matters here specifically: an earlier
/// version called `speechVoices()` directly, reasoning that a synchronous,
/// delegate-free class query needs no consistent thread, and it hung
/// indefinitely under `cargo test`.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn list_voices() -> Result<Vec<Voice>, String> {
    // Only `Voice` — two `String`s — crosses back out of the main thread; the
    // `NSArray` of voices does not leave it.
    Ok(with_main(|| {
        // SAFETY: on the thread `with_main` guarantees.
        let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
        voices
            .iter()
            .map(|voice| {
                // SAFETY: as above.
                Voice::local(
                    unsafe { voice.name() }.to_string(),
                    unsafe { voice.language() }.to_string(),
                )
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

/// Parse `say -v '?'`-shaped text: `Name   locale   # sample sentence`.
///
/// No backend feeds this any more — macOS moved to `AVSpeechSynthesizer`,
/// which needs no parsing — but it stays as a test-only fixture builder: it is
/// a much more compact way to write a voice list by hand than repeating
/// `Voice::local(...)` for every entry, and several tests below still lean on
/// that.
///
/// Names may contain spaces and parentheses — `Eddy (Chinese (China
/// mainland))` is a real one — so the locale is taken as the last
/// whitespace-separated field before the sample comment, and everything before
/// it is the name.
#[cfg(test)]
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
        // (The machine this runs on may have no Chinese voice installed at
        // all, so the assertions are about `set_voice` returning the same
        // thing `voice()` reports rather than about any particular voice being
        // installed.)
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

    // There is deliberately no test here that checks the *real* macOS voice
    // list (there was one — `the_system_actually_offers_a_chinese_voice` —
    // before this file moved off `say`). `list_voices` is faked under
    // `cfg(test)` on macOS (see its own doc comment), for a reason a
    // real-system check can't route around: `AVSpeechSynthesisVoice` needs a
    // run loop `cargo test` never provides. Checking against the real system
    // voice list now happens by running the actual built app.
    //
    // There is deliberately no `cargo test` equivalent of "speak a character
    // and listen to it" here any more, for the same run-loop reason: `speak`
    // and `stop` on macOS go through `speak_on_main`/`stop_on_main`, both real
    // AVFoundation calls with no test-only stand-in the way `list_voices` has
    // one. iOS was never testable this way either, for the same reason.
    // Verify pronunciation by running the actual built app — see
    // HANDOVER.md's macOS App Store section.
}
