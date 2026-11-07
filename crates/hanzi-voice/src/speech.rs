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
use block2::RcBlock;
#[cfg(target_os = "ios")]
use dispatch2::DispatchQueue;
#[cfg(target_os = "ios")]
use objc2::runtime::Bool;
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
    AVAudioSession, AVAudioSessionActivationOptions, AVAudioSessionCategoryOptions,
    AVAudioSessionCategoryPlayback, AVAudioSessionModeSpokenAudio, AVAudioSessionSetActiveOptions,
};
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_avf_audio::{
    AVSpeechBoundary, AVSpeechSynthesisVoice, AVSpeechSynthesizer, AVSpeechSynthesizerDelegate,
    AVSpeechUtterance,
};
#[cfg(target_os = "ios")]
use objc2_foundation::NSError;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use objc2_foundation::{NSObject, NSObjectProtocol, NSString};

/// A voice as the platform reports it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Voice {
    /// The platform's own unique name for this voice, or empty where it has none.
    ///
    /// On macOS and iOS this is `AVSpeechSynthesisVoice.identifier` — e.g.
    /// `com.apple.eloquence.zh-CN.Eddy` — and it is the only thing that tells
    /// the voices apart: AVFoundation reports *bare* names, and the novelty
    /// voices (Eddy, Flo, Reed, Sandy, Shelley, Grandma, Grandpa, Rocko) exist
    /// under the same name in every language. Matching on the name alone is how
    /// a learner who picked the mainland Eddy heard 妈 read by the English one
    /// (issue #39). Android's voice names are already unique
    /// (`zh-cn-x-ccc-local`), so its listing leaves this empty and [`Self::id`]
    /// falls back to the name.
    #[serde(default)]
    pub identifier: String,
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
    /// How good the voice's data is — `standard`, `enhanced` or `premium` — or
    /// `None` where the platform does not say.
    ///
    /// Only macOS and iOS report it (`AVSpeechSynthesisVoice.quality`), and it
    /// matters there because a downloaded enhanced voice is listed beside the
    /// compact one under the same name and locale: without it the settings
    /// screen shows two identical rows.
    #[serde(default)]
    pub quality: Option<String>,
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
            identifier: String::new(),
            name: name.into(),
            locale: locale.into(),
            network: false,
            quality: None,
        }
    }

    /// The same voice, carrying how good its data is (see [`Self::quality`]).
    pub fn with_quality(mut self, quality: Option<&str>) -> Self {
        self.quality = quality.map(str::to_string);
        self
    }

    /// The same voice, carrying the platform's identifier for it.
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
    }

    /// What a choice of this voice is stored as: the platform's identifier, or
    /// the name on a platform whose names are already unique.
    ///
    /// The settings screen sends this back as the learner's choice, and
    /// [`Speaker::set_voice`] accepts it — as well as a bare name, which is what
    /// every build before identifiers stored.
    pub fn id(&self) -> &str {
        if self.identifier.is_empty() {
            &self.name
        } else {
            &self.identifier
        }
    }
}

/// The locale [`resolve_voice`] gives an override that names no installed voice
/// of the speaker's language — a name handed to the synthesiser as it is.
const OVERRIDE: &str = "override";

/// Which language a [`Speaker`] pronounces.
///
/// The synthesiser belongs to the platform and knows every language the machine
/// has installed; what it cannot be asked is which of them *this app* teaches.
/// That is one decision per app rather than one per utterance — the kana tutor
/// pronounces Japanese and never Chinese, and the two Hanzi Tutor apps do the
/// reverse — so it is a field on [`Speaker`] rather than an argument to
/// [`Speaker::speak`].
///
/// The default is Chinese, and that is not a shrug: `Speaker::default()` was the
/// only constructor there was, and every caller that predates this type is a
/// Chinese app. A speaker that started out undecided would have to fail its first
/// call or guess, and neither is better than the language those apps already mean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Chinese,
    Japanese,
}

impl Language {
    /// The first letters of the locales this language's voices carry.
    ///
    /// A prefix rather than an exact tag, because the tags disagree about their
    /// separator — macOS reports `zh_CN` and `ja_JP`, iOS and Android `zh-CN` and
    /// `ja-JP` — and the region is not what the choice turns on. `zh` also
    /// deliberately keeps the Taiwanese and Cantonese voices in the list rather
    /// than filtering a learner out of them.
    pub fn locale_prefix(self) -> &'static str {
        match self {
            Language::Chinese => "zh",
            Language::Japanese => "ja",
        }
    }

    /// The locale whose voices are preferred, in the normalised form
    /// [`locale_key`] produces.
    fn home_locale(self) -> &'static str {
        match self {
            Language::Chinese => "zh_cn",
            Language::Japanese => "ja_jp",
        }
    }

    /// The locale whose voices come next after the home locale's, if one does.
    ///
    /// Taiwan for Chinese: its voices speak Mandarin, where Hong Kong's speak
    /// Cantonese, so a Mandarin learner with no mainland voice is better served
    /// by them. Japanese has no second choice worth naming.
    fn second_locale(self) -> Option<&'static str> {
        match self {
            Language::Chinese => Some("zh_tw"),
            Language::Japanese => None,
        }
    }

    /// The voice names preferred within that locale, in order.
    ///
    /// Ordered for a learner rather than for novelty. Chinese: Tingting is the
    /// long-standing mainland system voice, and the newer "expressive" voices are
    /// only worth falling back to. Japanese: Kyoko is the voice macOS has shipped
    /// as its Japanese system voice for years and Otoya is the other long-standing
    /// one, while the expressive set — Eddy, Flo, Grandma, Grandpa, Reed, Rocko,
    /// Sandy, Shelley — is the fallback rather than the first choice.
    fn preferred_names(self) -> &'static [&'static str] {
        match self {
            Language::Chinese => &["Tingting", "Ting-Ting", "Meijia"],
            Language::Japanese => &["Kyoko", "Otoya"],
        }
    }

    /// How a person says this language's name, for a message.
    pub fn display_name(self) -> &'static str {
        match self {
            Language::Chinese => "Chinese",
            Language::Japanese => "Japanese",
        }
    }

    /// The environment variable that overrides the automatically chosen voice.
    ///
    /// One per language rather than one shared: `HANZI_TUTOR_VOICE=Meijia` is
    /// about the Chinese app, and a kana tutor obeying a variable named after
    /// another product would be the same tangle as a shared learner store. The
    /// two apps are separate products, and their settings are separate with them.
    fn override_var(self) -> &'static str {
        match self {
            Language::Chinese => "HANZI_TUTOR_VOICE",
            Language::Japanese => "NIHONGO_TUTOR_VOICE",
        }
    }
}

/// The longest string that will be handed to the synthesiser.
const MAX_UTTERANCE: usize = 64;

/// Pronunciation, with at most one utterance in flight.
///
/// A speaker pronounces **one language** for its whole life ([`Language`]), which
/// is what each app means: the kana tutor has no Chinese to say and neither Hanzi
/// Tutor app has Japanese. The language is fixed at construction rather than
/// passed to [`Self::speak`] because the *voice list*, the *override variable* and
/// the *message when nothing is installed* are all per-language too, and an
/// argument repeated on every call would be a decision the caller could get wrong
/// one call at a time.
#[derive(Default)]
pub struct Speaker {
    /// Which language this speaker speaks, and therefore which voices it will
    /// accept, which override variable it obeys, and what it says when the machine
    /// has none of them.
    language: Language,
    /// The voice the learner has asked for — its [`Voice::id`], or the bare
    /// name an older build stored — or `None` for the automatic choice. Set
    /// from the settings screen at startup and on every change; a voice this
    /// machine does not have falls back to the automatic choice rather than
    /// being an error, so a preference carried from another machine cannot
    /// break pronunciation here.
    preferred: Mutex<Option<String>>,
    /// Every voice the system offers, resolved once. Enumerating them takes
    /// about a second, which is why this is warmed in the background (see
    /// `warm_voice`) and why the settings screen, which asks for the list again
    /// every time it is opened, is served from here rather than listing them
    /// afresh. It never changes while the app runs, so a `OnceLock` is the right
    /// shape for it — unlike the *choice*, which is resolved on every call from
    /// this list and the preference together.
    ///
    /// Every voice, not the language's: the list is the platform's and is
    /// resolved once for the process, while which of them count is
    /// [`Self::learner_voices`]'s question and costs a scan.
    voices: OnceLock<Vec<Voice>>,
}

impl Speaker {
    /// A speaker for `language`.
    ///
    /// [`Default`] is Chinese, so the two apps that predate this constructor are
    /// unchanged; a new app says which language it teaches here.
    pub fn new(language: Language) -> Self {
        // Written out rather than `..Self::default()`: `Speaker` implements
        // `Drop`, so the functional-update form would be a partial move out of a
        // type that has a destructor.
        Self {
            language,
            preferred: Mutex::new(None),
            voices: OnceLock::new(),
        }
    }

    /// The language this speaker pronounces.
    pub fn language(&self) -> Language {
        self.language
    }

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
        let overridden = override_voice(self.language);
        resolve_voice(
            self.voices(),
            preferred.as_deref(),
            overridden.as_ref().map(|voice| voice.name.as_str()),
            self.language,
        )
    }

    /// The installed voice the learner's stored choice names, or `None` when
    /// there is no choice or this machine does not have the voice it names.
    ///
    /// Not [`Self::voice`]: that is what will be *spoken with*, after the
    /// override and the automatic fallback, while this is what the *choice*
    /// resolves to. The settings screen needs both — this one to select the
    /// right entry for a bare name an older build stored (`Eddy` is two
    /// entries now, one per locale), and the difference between the two to say
    /// honestly when the choice is not what is heard.
    pub fn chosen_voice(&self) -> Option<Voice> {
        let preferred = self
            .preferred
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        find_voice(self.voices(), &preferred, self.language)
    }

    /// The installed voice of *another* language the stored choice names, when
    /// it names none of this speaker's.
    ///
    /// So the settings screen can say "Eddy (en-US) isn't available as a
    /// Chinese voice" rather than "not installed" about a voice the learner can
    /// see in the system's own list. `None` when the choice resolves (see
    /// [`Self::chosen_voice`]), when nothing is chosen, and when the machine
    /// has no such voice at all.
    pub fn chosen_elsewhere(&self) -> Option<Voice> {
        if self.chosen_voice().is_some() {
            return None;
        }
        let preferred = self
            .preferred
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        let voices = self.voices();
        position_named(voices, &preferred).map(|index| voices[index].clone())
    }

    /// Every voice the system offers.
    fn voices(&self) -> &[Voice] {
        self.voices
            .get_or_init(|| list_voices().unwrap_or_default())
    }

    /// The voices a character of this speaker's language can be spoken with.
    ///
    /// Filtered to the language's locales, sorted by name so that the settings
    /// screen's list does not reorder itself between one launch and the next
    /// (the order the system reports is not a promise), and carrying each
    /// voice's locale, which is the only thing that tells 美佳's `zh_TW` apart
    /// from a mainland voice of a similar name.
    pub fn learner_voices(&self) -> Vec<Voice> {
        voices_for(self.voices(), self.language)
    }

    /// Choose a voice by its [`Voice::id`] — or by name, which is what older
    /// builds stored — or `None` for the automatic choice.
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
            return Err(no_voice_message(self.language));
        };

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            // Stopping first, and after the voice is known, so a failed lookup
            // cannot silence what is already being said.
            self.stop();
            speak_on_main(text, voice)
        }

        // Android's synthesiser belongs to the Kotlin side of the platform
        // bridge, so speaking is a command to it rather than something kept
        // here. The voice is named rather than identified, which is safe here
        // and not on Apple: Android's voice names (`zh-cn-x-ccc-local`) are
        // unique, while AVFoundation's bare `Eddy` is one in every language.
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
        // Not fatal and not worth a dialog: a cold route costs the crackle
        // fix, not the speech. `speak_on_main` takes the session again.
        // `engage_session` already logs its own failures asynchronously.
        #[cfg(target_os = "ios")]
        audio_ready();

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

/// Run `work` on the main thread without waiting for it, on iOS.
///
/// [`hold_session_then_release`] is the one caller: its timer thread has
/// nothing to do with the result and is about to exit either way, so blocking
/// it on [`with_main`]'s round trip bought nothing but a parked thread —
/// which is exactly what Xcode's Thread Performance Checker flagged as a
/// priority inversion, since a timer thread here can run at a QoS the main
/// run loop does not immediately match. Not waiting removes the wait, not
/// just the warning.
#[cfg(target_os = "ios")]
fn spawn_on_main(work: impl FnOnce() + Send + 'static) {
    if MainThreadMarker::new().is_some() {
        return work();
    }
    DispatchQueue::main().exec_async(work);
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

/// Speak `text` in `voice`, on the main thread.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn speak_on_main(text: &str, voice: Voice) -> Result<(), String> {
    let text = text.to_string();
    with_main(move || {
        let utterance = utterance_for(&text, &voice)?;
        // The audio session is an iOS-only concept: macOS has no Ring/Silent
        // switch and nothing to duck, so there is nothing to take here.
        #[cfg(target_os = "ios")]
        {
            // Logs its own failure asynchronously; see `engage_session`. Worth
            // saying, not worth refusing to speak over: this costs volume, not
            // words. Speech still happens, it may just be muted by the
            // Ring/Silent switch the way it was before this call.
            engage_session();
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
///
/// Activation itself is asynchronous (`activateWithOptions:completionHandler:`),
/// not `setActive:error:` — Apple's own documentation calls activation "a
/// relatively time consuming operation", and Xcode's Thread Performance
/// Checker flags the synchronous call as a hang risk exactly where
/// `with_main` runs this, on the real main thread. Fire-and-forget is the
/// right shape for what this already was: nothing here ever waited for the
/// *result* of activation, only logged it, and [`speak_on_main`] proceeds to
/// speak either way — a session that failed to activate costs volume, not
/// words, which is the whole reason logging rather than refusing was already
/// the contract before this was async.
#[cfg(target_os = "ios")]
fn engage_session() {
    // SAFETY: on the main thread (see `with_main`).
    unsafe {
        let session = AVAudioSession::sharedInstance();
        if let Err(error) = session.setCategory_mode_options_error(
            AVAudioSessionCategoryPlayback.expect("declared by AVFAudio"),
            AVAudioSessionModeSpokenAudio.expect("declared by AVFAudio"),
            AVAudioSessionCategoryOptions::DuckOthers,
        ) {
            eprintln!(
                "[speech] could not configure the audio session: {}",
                error.localizedDescription()
            );
            return;
        }
        let handler = RcBlock::new(move |active: Bool, error: *mut NSError| {
            if !active.as_bool() {
                let message = if error.is_null() {
                    "no reason given".to_string()
                } else {
                    // SAFETY: AVFoundation hands the handler a non-null NSError
                    // whenever `active` is false, and the surrounding `unsafe`
                    // block on `engage_session`'s body already covers this
                    // closure lexically.
                    (&*error).localizedDescription().to_string()
                };
                eprintln!("[speech] could not take the audio session: {message}");
            }
        });
        session.activateWithOptions_completionHandler(AVAudioSessionActivationOptions::None, &handler);
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
        spawn_on_main(move || {
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
fn audio_ready() {
    with_main(|| {
        // Building the synthesiser is the part that only has to happen once.
        SPEECH.with(|slot| {
            let mut slot = slot.borrow_mut();
            let _ = slot.get_or_insert_with(Speech::new);
        });
        engage_session();
        take_session();
        hold_session_then_release();
    })
}

/// An utterance for `text`, spoken in `voice`.
///
/// The voice is looked up afresh each time rather than held as an object — the
/// resolved [`Voice`] is platform-neutral and crosses threads, an
/// `AVSpeechSynthesisVoice` does neither. A voice with an identifier is asked
/// for by it (`voiceWithIdentifier:`), which costs no listing; one without, or
/// one the system no longer answers for, is found in a fresh `speechVoices()`
/// list by [`installed_index`]. It used to be the first voice whose *name*
/// matched, which on a system where `Eddy` exists in every language read
/// Chinese with whichever Eddy came first (issue #39).
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn utterance_for(text: &str, voice: &Voice) -> Result<Retained<AVSpeechUtterance>, String> {
    let string = NSString::from_str(text);
    // SAFETY: on the main thread (see `with_main`).
    let utterance = unsafe { AVSpeechUtterance::speechUtteranceWithString(&string) };
    if !voice.identifier.is_empty() {
        let identifier = NSString::from_str(&voice.identifier);
        // SAFETY: as above. `nil` — an unknown identifier, or a voice not on
        // the device — falls through to the listing below.
        if let Some(found) = unsafe { AVSpeechSynthesisVoice::voiceWithIdentifier(&identifier) } {
            // SAFETY: as above.
            unsafe { utterance.setVoice(Some(&found)) };
            return Ok(utterance);
        }
    }
    // SAFETY: as above.
    let installed: Vec<Retained<AVSpeechSynthesisVoice>> =
        unsafe { AVSpeechSynthesisVoice::speechVoices() }
            .iter()
            .collect();
    let listed: Vec<Voice> = installed.iter().map(|v| voice_from(v)).collect();
    let wanted = installed_index(&listed, voice)
        .map(|index| &installed[index])
        .ok_or_else(|| {
            format!(
                "the voice {} ({}) is no longer available on this device",
                voice.name, voice.locale
            )
        })?;
    // SAFETY: as above.
    unsafe { utterance.setVoice(Some(wanted)) };
    Ok(utterance)
}

/// The platform-neutral description of an AVFoundation voice.
///
/// One function for both places a voice is read — the list the speaker
/// resolves from, and the lookup [`utterance_for`] does — so the two can never
/// disagree about what a voice is called or which identifier it carries.
#[cfg(any(target_os = "ios", target_os = "macos"))]
fn voice_from(voice: &AVSpeechSynthesisVoice) -> Voice {
    // SAFETY: only ever called on the thread `with_main` guarantees.
    let (name, language, identifier, quality) = unsafe {
        (
            voice.name().to_string(),
            voice.language().to_string(),
            voice.identifier().to_string(),
            voice.quality().0,
        )
    };
    let quality = quality_name(quality).or_else(|| quality_from_identifier(&identifier));
    Voice::local(name, language)
        .with_identifier(identifier)
        .with_quality(quality)
}

/// `AVSpeechSynthesisVoiceQuality`'s raw value, as the word the settings
/// screen shows: 1 is `standard`, 2 `enhanced`, 3 `premium`.
#[cfg_attr(not(any(target_os = "ios", target_os = "macos")), allow(dead_code))]
fn quality_name(raw: isize) -> Option<&'static str> {
    match raw {
        1 => Some("standard"),
        2 => Some("enhanced"),
        3 => Some("premium"),
        _ => None,
    }
}

/// The quality an Apple identifier spells out —
/// `com.apple.voice.compact.zh-CN.Tingting`, `….enhanced.…`, `….premium.…` —
/// for a voice whose `quality` is a value this file does not know.
#[cfg_attr(not(any(target_os = "ios", target_os = "macos")), allow(dead_code))]
fn quality_from_identifier(identifier: &str) -> Option<&'static str> {
    identifier.split('.').find_map(|segment| match segment {
        "compact" => Some("standard"),
        "enhanced" => Some("enhanced"),
        "premium" => Some("premium"),
        _ => None,
    })
}

impl Drop for Speaker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn no_voice_message(language: Language) -> String {
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
        "no {} voice is installed, so pronunciation is unavailable. \
         Add one in {WHERE}, or set {} to a voice name.",
        language.display_name(),
        language.override_var(),
    )
}

/// The voice named by this language's override variable, if it is set to
/// anything usable.
///
/// The override outranks the stored preference, because that is what an
/// environment variable is for: it is the escape hatch for a run that has to be
/// reproducible, and a settings row silently outranking it would make
/// `HANZI_TUTOR_VOICE=… pnpm run dev` a lie.
fn override_voice(language: Language) -> Option<Voice> {
    let name = std::env::var(language.override_var()).ok()?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some(Voice::local(name, OVERRIDE))
}

/// The voice that will be used: the override, then the learner's choice, then
/// the automatic pick.
///
/// A function of the installed voices, the preference and the override, so every
/// ordering rule here is testable without a synthesiser and without an
/// environment variable in the way — the caller reads the override variable and
/// passes the result in. The three inputs are also the three things that can
/// change, and keeping the decision in one function is what makes "does my
/// choice actually take effect?" answerable by reading this.
fn resolve_voice(
    installed: &[Voice],
    preferred: Option<&str>,
    overridden: Option<&str>,
    language: Language,
) -> Option<Voice> {
    // An override is not "a preference that wins": it does not even have to name
    // an installed voice, because it exists to hand the synthesiser a name the
    // list does not know about. When it *does* name one of this language's
    // voices, it resolves to that voice — identifier and all — so
    // `HANZI_TUTOR_VOICE=Eddy` means the Chinese Eddy, not the first Eddy the
    // system happens to list.
    if let Some(name) = overridden {
        return Some(
            find_voice(installed, name, language).unwrap_or_else(|| Voice::local(name, OVERRIDE)),
        );
    }
    if let Some(wanted) = preferred {
        if let Some(voice) = find_voice(installed, wanted, language) {
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
    pick_voice(installed, language)
}

/// Find one of `language`'s installed voices by identifier, or by name.
///
/// A stored choice is a [`Voice::id`] — the platform identifier — from this
/// version on, and an exact identifier match is what it resolves to. A bare
/// name is what every earlier build stored, and it still has to work, which is
/// where the care is: AVFoundation's novelty voices (Eddy, Flo, Reed, Sandy,
/// Shelley, Grandma, Grandpa, Rocko) carry the *same* bare name in every
/// language, so a name is matched only among `language`'s voices, the home
/// locale's first. A stored `Eddy` therefore means the mainland Eddy in the
/// Chinese apps and the Japanese one in the kana tutor, never whichever Eddy the
/// system lists first (issue #39), and a name no voice of the language carries
/// finds nothing — the automatic choice is a better answer than an English
/// voice guessing at 汉.
///
/// The exact name wins over the base name, so a machine that has both `Meijia`
/// and `Meijia (Chinese (China mainland))` uses the one that was named exactly.
/// Otherwise the locale qualifier is ignored on **both** sides: the `say -v '?'`
/// era (v0.3.0–v0.5.11) stored names like `Eddy (Chinese (China mainland))`,
/// while AVFoundation now reports a bare `Eddy`, and the reverse is just as
/// possible. A stored qualifier is not thrown away, though — it names the
/// locale ([`qualifier_locale`]), so `Eddy (Chinese (Taiwan))` is the Taiwanese
/// Eddy. Where it names none, the order is [`locale_rank`]'s: the home locale,
/// then the language's second one, then the rest as listed.
fn find_voice(voices: &[Voice], wanted: &str, language: Language) -> Option<Voice> {
    let wanted_locale = qualifier_locale(wanted);
    let wanted_base = base_name(wanted);
    let ours: Vec<&Voice> = voices
        .iter()
        .filter(|v| locale_key(v).starts_with(language.locale_prefix()))
        .collect();
    // Among the voices that match, the best-placed locale; the first listed of
    // those, since `min_by_key` keeps the first of equal keys.
    let best = |matches: &dyn Fn(&Voice) -> bool| {
        ours.iter()
            .copied()
            .filter(|v| matches(v))
            .min_by_key(|v| locale_rank(v, language, wanted_locale))
    };

    best(&|v| !v.identifier.is_empty() && v.identifier == wanted)
        .or_else(|| best(&|v| v.name.eq_ignore_ascii_case(wanted)))
        .or_else(|| best(&|v| base_name(&v.name).eq_ignore_ascii_case(wanted_base)))
        .cloned()
}

/// Where `voice` stands in `language`'s order of locales: the locale a stored
/// name's qualifier asked for (`wanted`), then the home locale, then the
/// language's second locale, then everything else. Lower is better.
fn locale_rank(voice: &Voice, language: Language, wanted: Option<&str>) -> u8 {
    let key = locale_key(voice);
    if wanted.is_some_and(|wanted| key.starts_with(wanted)) {
        0
    } else if key.starts_with(language.home_locale()) {
        1
    } else if language
        .second_locale()
        .is_some_and(|second| key.starts_with(second))
    {
        2
    } else {
        3
    }
}

/// The locale a `say -v '?'`-style qualifier names, in [`locale_key`] form.
///
/// `Eddy (Chinese (China mainland))` is `zh_cn`, `(Chinese (Taiwan))` `zh_tw`,
/// `(Chinese (Hong Kong))` `zh_hk` and `(Japanese (Japan))` `ja_jp`. Those are
/// the qualifiers the voices this app offers carried; anything else names no
/// locale and the voice is found by its base name alone.
fn qualifier_locale(name: &str) -> Option<&'static str> {
    // `base_name` is a prefix of `name`, so what follows it is the qualifier.
    let qualifier = name[base_name(name).len()..].to_ascii_lowercase();
    const KNOWN: [(&str, &str); 4] = [
        ("china mainland", "zh_cn"),
        ("taiwan", "zh_tw"),
        ("hong kong", "zh_hk"),
        ("japan", "ja_jp"),
    ];
    KNOWN
        .into_iter()
        .find(|(words, _)| qualifier.contains(words))
        .map(|(_, locale)| locale)
}

/// The first of `voices` that `name` names, in any language: by identifier,
/// then by exact name, then by base name, the qualifier ignored on both sides.
///
/// For the two places a name is *not* confined to a language: an override that
/// named nothing of the speaker's language ([`installed_index`]), and telling a
/// learner that their choice exists here, just not in the language this app
/// speaks ([`Speaker::chosen_elsewhere`]).
fn position_named(voices: &[Voice], name: &str) -> Option<usize> {
    let base = base_name(name);
    voices
        .iter()
        .position(|v| !v.identifier.is_empty() && v.identifier == name)
        .or_else(|| {
            voices
                .iter()
                .position(|v| v.name.eq_ignore_ascii_case(name))
        })
        .or_else(|| {
            voices
                .iter()
                .position(|v| base_name(&v.name).eq_ignore_ascii_case(base))
        })
}

/// Which of the system's voices, as listed now, is `wanted`.
///
/// The last step before speaking on macOS and iOS: [`resolve_voice`] has
/// already decided *which* voice, and this finds that same voice in a fresh
/// `speechVoices()` list so the utterance can be given it. By identifier where
/// the voice carries one; by name **and locale** where it does not, because the
/// name alone is ambiguous — matching on it is how the mainland Eddy came out in
/// English (issue #39). Only an override that named no installed voice of the
/// speaker's language (locale [`OVERRIDE`]) is matched in any language — by
/// identifier or name, see [`position_named`] — since what it was given is all
/// it ever was.
///
/// A function of plain lists, so the rule is tested on every platform even
/// though only Apple's backend calls it.
#[cfg_attr(not(any(target_os = "ios", target_os = "macos")), allow(dead_code))]
fn installed_index(installed: &[Voice], wanted: &Voice) -> Option<usize> {
    let exact = |v: &Voice| v.name.eq_ignore_ascii_case(&wanted.name);
    let base = |v: &Voice| base_name(&v.name).eq_ignore_ascii_case(&wanted.name);
    let same_locale = |v: &Voice| locale_key(v) == locale_key(wanted);

    if !wanted.identifier.is_empty() {
        if let Some(index) = installed
            .iter()
            .position(|v| v.identifier == wanted.identifier)
        {
            return Some(index);
        }
    }
    let by_name_and_locale = installed
        .iter()
        .position(|v| same_locale(v) && exact(v))
        .or_else(|| installed.iter().position(|v| same_locale(v) && base(v)));
    if by_name_and_locale.is_some() || wanted.locale != OVERRIDE {
        return by_name_and_locale;
    }
    position_named(installed, &wanted.name)
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
    // Only `Voice` — plain `String`s — crosses back out of the main thread;
    // the `NSArray` of voices does not leave it.
    Ok(with_main(|| {
        // SAFETY: on the thread `with_main` guarantees.
        let voices = unsafe { AVSpeechSynthesisVoice::speechVoices() };
        voices
            .iter()
            .map(|voice| voice_from(&voice))
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

/// The voices of `language` that a character can be spoken with, sorted by name.
///
/// Filtered to the language's locales, sorted so that the settings screen's list
/// does not reorder itself between one launch and the next — the order the
/// system reports is not a promise — and keeping each voice's locale, which is
/// the only thing that tells 美佳's `zh_TW` apart from a mainland voice of a
/// similar name. A function of the list so it can be tested without a
/// synthesiser, which is what keeps the *shape* of the offered list honest.
///
/// **Deduplicated**, because `say -v '?'` lists every voice twice — a known
/// quirk, see HANDOVER §6 — and a list with a repeated entry is not merely
/// untidy: the settings screen keys its options by [`Voice::id`], so a
/// duplicate is a rendering error that takes the screen down. The first entry
/// wins; the duplicates are the same voice. macOS lists Kyoko twice for the
/// same reason, so this is not a Chinese-only repair.
///
/// A duplicate is the same *voice*, though, not the same name: the same
/// identifier, or — where the platform reports none — the same name in the
/// same locale. AVFoundation's `Eddy` exists in `zh-CN` and in `zh-TW`, and
/// those are two voices a learner can tell apart by ear; merging them under one
/// name is half of issue #39. Sorted by name and then locale, so the two sit
/// together and in the same order on every launch.
fn voices_for(all: &[Voice], language: Language) -> Vec<Voice> {
    let same = |a: &Voice, b: &Voice| {
        if !a.identifier.is_empty() || !b.identifier.is_empty() {
            a.identifier == b.identifier
        } else {
            a.name.eq_ignore_ascii_case(&b.name) && locale_key(a) == locale_key(b)
        }
    };
    let mut voices: Vec<Voice> = Vec::new();
    for voice in all {
        if !locale_key(voice).starts_with(language.locale_prefix()) {
            continue;
        }
        if voices.iter().any(|seen| same(seen, voice)) {
            continue;
        }
        voices.push(voice.clone());
    }
    voices.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| locale_key(a).cmp(&locale_key(b)))
    });
    voices
}

/// A voice's locale, lower-cased with any dash turned into an underscore.
///
/// The two spellings are both real — macOS reports `zh_CN` and `ja_JP`, iOS and
/// Android `zh-CN` and `ja-JP` — so every locale comparison in this file goes
/// through here rather than each one remembering to normalise.
fn locale_key(voice: &Voice) -> String {
    voice.locale.replace('-', "_").to_ascii_lowercase()
}

/// The bare voice name, without the locale qualifier macOS appends.
///
/// macOS reports names as `Tingting (Chinese (China mainland))` and
/// `Kyoko (Japanese (Japan))`, so matching a preference against the full name
/// would silently never fire.
fn base_name(name: &str) -> &str {
    match name.find(" (") {
        Some(index) => &name[..index],
        None => name,
    }
}

/// Choose the best voice for `language` from those installed.
///
/// The language's home locale is preferred — mainland simplified Chinese
/// (`zh_CN`) or Japanese as spoken in Japan (`ja_JP`) — then any locale of that
/// language in [`locale_rank`]'s order, which keeps a Taiwanese or Cantonese
/// voice reachable for Chinese rather than refusing to speak, Taiwan's Mandarin
/// first.
fn pick_voice(voices: &[Voice], language: Language) -> Option<Voice> {
    let home = |v: &&Voice| locale_key(v).starts_with(language.home_locale());
    let preferred = |v: &&Voice| {
        language
            .preferred_names()
            .iter()
            .any(|name| base_name(&v.name).eq_ignore_ascii_case(name))
    };
    let any = |v: &&Voice| locale_key(v).starts_with(language.locale_prefix());

    voices
        .iter()
        .find(|v| home(v) && preferred(v))
        .or_else(|| {
            voices
                .iter()
                .filter(any)
                .min_by_key(|v| locale_rank(v, language, None))
        })
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

    /// The Japanese voice list, in the shape the legacy `say -v '?'` reports it.
    ///
    /// Genuine names rather than tidy ones, for the reason the Chinese fixture
    /// exists: the qualified `Kyoko (Japanese (Japan))` form is what that command
    /// prints, and it is the shape a preference written on one machine used to be
    /// matched against. It lists Kyoko twice, which the settings list's
    /// deduplication has to survive.
    ///
    /// **Measured, not assumed:** `AVSpeechSynthesisVoice.speechVoices()` — what
    /// this file actually reads on macOS — reports the same voice as the bare name
    /// `Kyoko` with the BCP-47 tag `ja-JP`, not the qualified `say` spelling. The
    /// kana tutor's live check printed `[speech] using voice Kyoko (ja-JP)`, which
    /// is both differences at once. Every rule here normalises the separator and
    /// strips the qualifier, so the two shapes resolve to the same voice; a fixture
    /// in only one of them is what would hide a regression in the other.
    const SAMPLE_JA: &str = "\
Albert              en_US    # Hello! My name is Albert.
Eddy (Japanese (Japan)) ja_JP    # こんにちは! 私の名前はEddyです。
Kyoko (Japanese (Japan)) ja_JP    # こんにちは! 私の名前はKyokoです。
Kyoko (Japanese (Japan)) ja_JP    # こんにちは! 私の名前はKyokoです。
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
        let picked = pick_voice(&voices, Language::Chinese).expect("a Chinese voice is present");
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
        let picked = pick_voice(&parse_voices(real), Language::Chinese).expect("a Chinese voice");
        assert_eq!(base_name(&picked.name), "Tingting");

        // Without Tingting, a mainland voice is still preferred over the
        // Taiwanese and Cantonese ones.
        let without = parse_voices(real)
            .into_iter()
            .filter(|v| base_name(&v.name) != "Tingting")
            .collect::<Vec<_>>();
        let fallback = pick_voice(&without, Language::Chinese).expect("a Chinese voice");
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
            base_name(&pick_voice(&no_preferred, Language::Chinese).unwrap().name),
            "Eddy"
        );

        // With no mainland voice, a Taiwanese one is better than nothing.
        let only_taiwan = parse_voices(SAMPLE)
            .into_iter()
            .filter(|v| v.locale != "zh_CN")
            .collect::<Vec<_>>();
        assert_eq!(pick_voice(&only_taiwan, Language::Chinese).unwrap().locale, "zh_TW");

        // With no Chinese voice at all, say so rather than speaking English.
        let no_chinese = parse_voices(SAMPLE)
            .into_iter()
            .filter(|v| !v.locale.starts_with("zh"))
            .collect::<Vec<_>>();
        assert!(pick_voice(&no_chinese, Language::Chinese).is_none());
        assert!(pick_voice(&[], Language::Chinese).is_none());
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
        let picked = pick_voice(&voices, Language::Chinese).expect("iOS ships a Mandarin voice");
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
        assert_eq!(pick_voice(&voices, Language::Chinese), None);
        assert!(no_voice_message(Language::Chinese).contains("no Chinese voice"));
    }

    #[test]
    fn accepts_a_dash_separated_locale() {
        let voices = parse_voices("Tingting zh-CN # 你好\n");
        assert_eq!(pick_voice(&voices, Language::Chinese).unwrap().name, "Tingting");
    }

    #[test]
    fn a_chosen_voice_beats_the_automatic_choice() {
        // The point of the setting: 美佳 is `zh_TW`, so the automatic rule would
        // never pick it — a learner who wants a Taiwanese voice has to be able
        // to say so, and the setting has to be what actually gets used.
        let voices = parse_voices(SAMPLE);
        assert_eq!(base_name(&pick_voice(&voices, Language::Chinese).unwrap().name), "Tingting");

        let chosen = resolve_voice(&voices, Some("Meijia"), None, Language::Chinese).expect("a voice");
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
        let overruled = resolve_voice(&voices, Some("Meijia"), Some("Some Unlisted Voice"), Language::Chinese)
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
            let found = find_voice(&voices, wanted, Language::Chinese)
                .unwrap_or_else(|| panic!("{wanted}"));
            assert_eq!(base_name(&found.name), "Tingting");
        }
        assert!(find_voice(&voices, "Nobody", Language::Chinese).is_none());
    }

    #[test]
    fn an_exact_name_wins_over_a_base_name_match() {
        let voices = vec![
            Voice::local("Meijia (Chinese (China mainland))", "zh_CN"),
            Voice::local("Meijia", "zh_TW"),
        ];
        assert_eq!(
            find_voice(&voices, "Meijia", Language::Chinese)
                .unwrap()
                .locale,
            "zh_TW"
        );
        assert_eq!(
            find_voice(
                &voices,
                "Meijia (Chinese (China mainland))",
                Language::Chinese
            )
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
        let picked = resolve_voice(&voices, Some("Nonexistent Voice"), None, Language::Chinese).expect("a voice");
        assert_eq!(base_name(&picked.name), "Tingting");

        // And with nothing Chinese installed there is still no voice to invent.
        let none: Vec<Voice> = Vec::new();
        assert!(resolve_voice(&none, Some("Meijia"), None, Language::Chinese).is_none());
    }

    #[test]
    fn a_japanese_speaker_offers_only_japanese_voices() {
        // The kana tutor's settings-free list: the machine's Japanese voices,
        // and not one English or Chinese voice among them. Picking one is what
        // would make the app read あ as an English letter.
        let offered = voices_for(&parse_voices(SAMPLE_JA), Language::Japanese);
        let names: Vec<&str> = offered.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Eddy (Japanese (Japan))", "Kyoko (Japanese (Japan))"],
            "{offered:?}"
        );
        assert!(offered.iter().all(|v| v.locale.starts_with("ja")));
    }

    /// The Japanese half of the macOS preference rule.
    ///
    /// Kyoko is the voice macOS has shipped as its Japanese system voice for
    /// years, so she wins over the expressive voices even though `Eddy` sorts
    /// first — the same shape as `Tingting` beating the mainland `Eddy`.
    #[test]
    fn prefers_kyoko_for_japanese() {
        let picked = pick_voice(&parse_voices(SAMPLE_JA), Language::Japanese)
            .expect("a Japanese voice is present");
        assert_eq!(base_name(&picked.name), "Kyoko");

        // Without Kyoko, a Japanese voice is still better than nothing, and the
        // choice falls through to the expressive set rather than to silence.
        let without: Vec<Voice> = parse_voices(SAMPLE_JA)
            .into_iter()
            .filter(|v| base_name(&v.name) != "Kyoko")
            .collect();
        let fallback = pick_voice(&without, Language::Japanese).expect("a Japanese voice");
        assert_eq!(fallback.locale, "ja_JP");
        assert_eq!(base_name(&fallback.name), "Eddy");
    }

    /// The language rule is a filter in both directions, which is the point.
    ///
    /// A Japanese speaker handed nothing but Chinese voices must find none, and
    /// the Chinese speaker must not claim a Japanese one — the two apps ship
    /// from one crate, so a rule that leaked would make the kana tutor speak
    /// Mandarin and nobody would see it until they heard it.
    #[test]
    fn the_two_languages_do_not_claim_each_others_voices() {
        let mut both = parse_voices(SAMPLE);
        both.extend(parse_voices(SAMPLE_JA));
        assert!(pick_voice(&both, Language::Japanese).unwrap().locale.starts_with("ja"));
        assert!(pick_voice(&both, Language::Chinese).unwrap().locale.starts_with("zh"));

        let chinese_only = parse_voices(SAMPLE);
        assert_eq!(pick_voice(&chinese_only, Language::Japanese), None);
        assert!(no_voice_message(Language::Japanese).contains("no Japanese voice"));

        let japanese_only = parse_voices(SAMPLE_JA);
        assert_eq!(pick_voice(&japanese_only, Language::Chinese), None);
        assert!(no_voice_message(Language::Chinese).contains("no Chinese voice"));
    }

    /// Each language has its own override variable, and its own message names it.
    ///
    /// A shared variable would be one app obeying a setting named after the
    /// other, which is the same mistake as a shared learner store — see
    /// `HANDOVER_NIHONGO.md` invariant 15.
    #[test]
    fn each_language_names_its_own_override_variable() {
        assert_eq!(Language::Chinese.override_var(), "HANZI_TUTOR_VOICE");
        assert_eq!(Language::Japanese.override_var(), "NIHONGO_TUTOR_VOICE");
        assert!(no_voice_message(Language::Japanese).contains("NIHONGO_TUTOR_VOICE"));
        assert!(no_voice_message(Language::Chinese).contains("HANZI_TUTOR_VOICE"));
        assert!(no_voice_message(Language::Japanese).contains("no Japanese voice"));
    }

    /// A preference written the way macOS names a voice still finds it.
    #[test]
    fn a_japanese_preference_matches_the_qualified_name() {
        let voices = parse_voices(SAMPLE_JA);
        for wanted in ["Kyoko", "Kyoko (Japanese (Japan))", "kyoko"] {
            let found = find_voice(&voices, wanted, Language::Japanese)
                .unwrap_or_else(|| panic!("{wanted}"));
            assert_eq!(base_name(&found.name), "Kyoko");
        }
        let chosen = resolve_voice(&voices, Some("Kyoko"), None, Language::Japanese).expect("a voice");
        assert_eq!(base_name(&chosen.name), "Kyoko");
    }

    /// The shape `AVSpeechSynthesisVoice` really reports, which is *not* the shape
    /// `say -v '?'` prints.
    ///
    /// Measured rather than assumed: the kana tutor's live check printed
    /// `[speech] using voice Kyoko (ja-JP)` — a bare name and a BCP-47 tag, where
    /// the legacy command prints `Kyoko (Japanese (Japan)) ja_JP`. Both spellings
    /// have to resolve to the same voice, and this fixture is the one the app
    /// actually sees; the qualified one above is what a preference might have been
    /// written from.
    #[test]
    fn the_avfoundation_spelling_of_a_japanese_voice_resolves_too() {
        let voices: Vec<Voice> = vec![
            Voice::local("Daniel", "en-GB"),
            Voice::local("Eddy", "ja-JP"),
            Voice::local("Kyoko", "ja-JP"),
        ];
        let picked = pick_voice(&voices, Language::Japanese).expect("a Japanese voice");
        assert_eq!(picked.name, "Kyoko", "the preferred name still wins");
        assert_eq!(picked.locale, "ja-JP");

        // And the language filter holds for this spelling too: the English voice
        // is not offered, and the two Japanese ones are, once each.
        let offered = voices_for(&voices, Language::Japanese);
        assert_eq!(offered.len(), 2, "{offered:?}");
        assert!(offered.iter().all(|v| v.locale.starts_with("ja")));
    }

    /// A speaker remembers the language it was built for.
    ///
    /// `Default` has to keep meaning Chinese: two apps in this workspace were
    /// written against it and neither says a word of Japanese.
    #[test]
    fn a_speaker_remembers_the_language_it_speaks() {
        assert_eq!(Speaker::default().language(), Language::Chinese);
        assert_eq!(Speaker::new(Language::Japanese).language(), Language::Japanese);
        assert_eq!(Language::Japanese.display_name(), "Japanese");
        assert_eq!(Language::Japanese.locale_prefix(), "ja");
    }

    #[test]
    fn only_chinese_voices_are_offered_for_a_chinese_character() {
        // The settings screen's list is built from `voices_for`, so an English
        // voice must not appear in it — picking one is what makes the app read
        // 汉 as an English word.
        let offered = voices_for(&parse_voices(SAMPLE), Language::Chinese);
        assert_eq!(offered.len(), 4, "{offered:?}");
        assert!(offered.iter().all(|v| v.locale.starts_with("zh")));
        // Sorted, so the list does not shuffle between launches.
        let mut names: Vec<&str> = offered.iter().map(|v| v.name.as_str()).collect();
        let unsorted = names.clone();
        names.sort();
        assert_eq!(names, unsorted);
        // An iOS-style dash separator is filtered the same way.
        assert_eq!(
            voices_for(
                &[
                    Voice::local("Daniel", "en-GB"),
                    Voice::local("Tingting", "zh-CN"),
                ],
                Language::Chinese,
            )
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
        let offered = voices_for(&[network.clone(), on_device.clone()], Language::Chinese);

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
        // screen keys its options by `Voice::id` — the name, for a voice with
        // no identifier — so a duplicate is not untidiness, it is a rendering
        // error that takes the whole screen down. This fixture is the shape of
        // the real command's output: the same lines, twice.
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

        let offered = voices_for(&parsed, Language::Chinese);
        assert_eq!(
            offered.len(),
            2,
            "but the settings screen is offered each voice once: {offered:?}"
        );
        assert_eq!(offered[0].name, "Meijia");
        assert_eq!(offered[1].name, "Tingting (Chinese (China mainland))");

        // With no identifiers the names are the ids the screen keys on, so they
        // have to be distinct.
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

    /// The novelty voices AVFoundation reports under the same bare name in
    /// every language (issue #39), in the order a real `speechVoices()` list
    /// has them: English and German before Chinese.
    fn novelty_voices() -> Vec<Voice> {
        vec![
            Voice::local("Eddy", "en-US"),
            Voice::local("Eddy", "de-DE"),
            Voice::local("Eddy", "ja-JP"),
            Voice::local("Eddy", "zh-TW"),
            Voice::local("Eddy", "zh-CN"),
            Voice::local("Flo", "en-GB"),
            Voice::local("Flo", "zh-CN"),
        ]
    }

    #[test]
    fn a_name_shared_across_languages_resolves_to_the_learners_language() {
        let voices = novelty_voices();
        let chosen =
            resolve_voice(&voices, Some("Eddy"), None, Language::Chinese).expect("a voice");
        assert_eq!(chosen.locale, "zh-CN", "{chosen:?}");
        let chosen = resolve_voice(&voices, Some("Flo"), None, Language::Chinese).expect("a voice");
        assert_eq!(chosen.locale, "zh-CN", "{chosen:?}");
        let chosen =
            resolve_voice(&voices, Some("Eddy"), None, Language::Japanese).expect("a voice");
        assert_eq!(chosen.locale, "ja-JP", "{chosen:?}");
    }

    #[test]
    fn the_same_name_in_two_chinese_locales_is_offered_twice() {
        let offered = voices_for(&novelty_voices(), Language::Chinese);
        let shown: Vec<(&str, &str)> = offered
            .iter()
            .map(|v| (v.name.as_str(), v.locale.as_str()))
            .collect();
        assert_eq!(
            shown,
            vec![("Eddy", "zh-CN"), ("Eddy", "zh-TW"), ("Flo", "zh-CN")],
            "{offered:?}"
        );
    }

    /// The same novelty voices with the identifiers AVFoundation gives them —
    /// the spelling the app sees on macOS and iOS from this version on.
    fn identified_voices() -> Vec<Voice> {
        novelty_voices()
            .into_iter()
            .map(|v| {
                let identifier = format!("com.apple.eloquence.{}.{}", v.locale, v.name);
                v.with_identifier(identifier)
            })
            .collect()
    }

    #[test]
    fn a_stored_identifier_names_exactly_one_voice() {
        let voices = identified_voices();
        // The Taiwanese Eddy, chosen explicitly: not the mainland one the bare
        // name would prefer, and not the English one listed first.
        let chosen = resolve_voice(
            &voices,
            Some("com.apple.eloquence.zh-TW.Eddy"),
            None,
            Language::Chinese,
        )
        .expect("a voice");
        assert_eq!(
            (chosen.name.as_str(), chosen.locale.as_str()),
            ("Eddy", "zh-TW")
        );
        assert_eq!(chosen.id(), "com.apple.eloquence.zh-TW.Eddy");

        // An identifier of another language's voice is not a Chinese voice to
        // read 汉 with, even though the settings row could hold one: the
        // automatic choice is used instead.
        let english = find_voice(&voices, "com.apple.eloquence.en-US.Eddy", Language::Chinese);
        assert_eq!(english, None);
        let fallback = resolve_voice(
            &voices,
            Some("com.apple.eloquence.en-US.Eddy"),
            None,
            Language::Chinese,
        )
        .expect("a voice");
        assert!(fallback.locale.starts_with("zh"), "{fallback:?}");
    }

    #[test]
    fn an_identifier_match_beats_a_name_match() {
        // A voice whose *name* is another voice's identifier is contrived, but
        // it is the cleanest way to show which rule runs first.
        let voices = vec![
            Voice::local("com.apple.voice.compact.zh-CN.Tingting", "zh-CN"),
            Voice::local("Tingting", "zh-TW")
                .with_identifier("com.apple.voice.compact.zh-CN.Tingting"),
        ];
        let found = find_voice(
            &voices,
            "com.apple.voice.compact.zh-CN.Tingting",
            Language::Chinese,
        )
        .expect("a voice");
        assert_eq!(found.name, "Tingting");
    }

    #[test]
    fn a_bare_name_from_an_older_build_still_resolves_to_the_learners_language() {
        // What every build before identifiers stored: a bare `Eddy`. It keeps
        // working, it means the home locale's Eddy, and it resolves to the voice
        // *with* its identifier, so the utterance is given that voice and no other.
        let voices = identified_voices();
        let chinese =
            resolve_voice(&voices, Some("Eddy"), None, Language::Chinese).expect("a voice");
        assert_eq!(chinese.id(), "com.apple.eloquence.zh-CN.Eddy");
        let japanese =
            resolve_voice(&voices, Some("eddy"), None, Language::Japanese).expect("a voice");
        assert_eq!(japanese.id(), "com.apple.eloquence.ja-JP.Eddy");

        // With no mainland Eddy, the Taiwanese one is still a Chinese Eddy.
        let no_mainland: Vec<Voice> = voices.into_iter().filter(|v| v.locale != "zh-CN").collect();
        let taiwan =
            resolve_voice(&no_mainland, Some("Eddy"), None, Language::Chinese).expect("a voice");
        assert_eq!(taiwan.locale, "zh-TW");

        // A name only another language's voice carries is not taken: Flo exists
        // here in English only, so the Japanese app falls back to its own pick.
        let only_english_flo = vec![
            Voice::local("Flo", "en-GB").with_identifier("com.apple.eloquence.en-GB.Flo"),
            Voice::local("Kyoko", "ja-JP").with_identifier("com.apple.voice.compact.ja-JP.Kyoko"),
        ];
        let picked = resolve_voice(&only_english_flo, Some("Flo"), None, Language::Japanese)
            .expect("a voice");
        assert_eq!(picked.name, "Kyoko");
    }

    #[test]
    fn an_override_naming_a_shared_name_means_the_learners_language() {
        let voices = identified_voices();
        let chinese =
            resolve_voice(&voices, None, Some("Eddy"), Language::Chinese).expect("a voice");
        assert_eq!(chinese.id(), "com.apple.eloquence.zh-CN.Eddy");
        // And one naming nothing of the language is still passed through as given.
        let unlisted =
            resolve_voice(&voices, None, Some("Albert"), Language::Chinese).expect("a voice");
        assert_eq!(
            (unlisted.name.as_str(), unlisted.locale.as_str()),
            ("Albert", OVERRIDE)
        );
    }

    #[test]
    fn the_automatic_pick_carries_the_identifier_of_the_voice_it_chose() {
        // The automatic choice hits #39 too when Tingting and Meijia are absent:
        // it settles on the mainland Eddy, and the utterance must get *that* one.
        let voices = identified_voices();
        let picked = pick_voice(&voices, Language::Chinese).expect("a voice");
        assert_eq!(picked.id(), "com.apple.eloquence.zh-CN.Eddy");
        let picked = pick_voice(&voices, Language::Japanese).expect("a voice");
        assert_eq!(picked.id(), "com.apple.eloquence.ja-JP.Eddy");
    }

    #[test]
    fn the_voice_spoken_with_is_found_by_identifier_not_by_its_bare_name() {
        // `utterance_for`'s lookup, on the list `speechVoices()` reports:
        // English and German Eddy first, as on a real machine.
        let listed = identified_voices();
        for wanted in [
            pick_voice(&listed, Language::Chinese).unwrap(),
            pick_voice(&listed, Language::Japanese).unwrap(),
            resolve_voice(
                &listed,
                Some("com.apple.eloquence.zh-TW.Eddy"),
                None,
                Language::Chinese,
            )
            .unwrap(),
        ] {
            let index = installed_index(&listed, &wanted).expect("the voice is listed");
            assert_eq!(listed[index], wanted);
        }
    }

    #[test]
    fn without_an_identifier_the_voice_is_found_by_name_and_locale() {
        // A voice from a listing with no identifiers — or an identifier this
        // list no longer has — is still never matched on its name alone.
        let listed = novelty_voices();
        let wanted = Voice::local("Eddy", "zh_CN"); // either separator
        assert_eq!(
            listed[installed_index(&listed, &wanted).unwrap()].locale,
            "zh-CN"
        );
        let stale = Voice::local("Eddy", "ja-JP").with_identifier("com.apple.gone.ja-JP.Eddy");
        assert_eq!(
            listed[installed_index(&listed, &stale).unwrap()].locale,
            "ja-JP"
        );

        // A voice this list does not have is reported missing, not swapped for
        // another language's voice of the same name.
        let gone = Voice::local("Eddy", "fr-FR");
        assert_eq!(installed_index(&listed, &gone), None);

        // Only an override that matched no voice of the language is a bare name,
        // and it is used as one: the first voice of that name, whatever it speaks.
        let unlisted = Voice::local("Flo", OVERRIDE);
        assert_eq!(
            listed[installed_index(&listed, &unlisted).unwrap()].locale,
            "en-GB"
        );
    }

    #[test]
    fn the_same_name_in_two_locales_is_two_entries_with_distinct_ids() {
        let offered = voices_for(&identified_voices(), Language::Chinese);
        let ids: Vec<&str> = offered.iter().map(Voice::id).collect();
        assert_eq!(
            ids,
            vec![
                "com.apple.eloquence.zh-CN.Eddy",
                "com.apple.eloquence.zh-TW.Eddy",
                "com.apple.eloquence.zh-CN.Flo",
            ]
        );
        // The same voice listed twice is still one entry.
        let mut twice = identified_voices();
        twice.extend(identified_voices());
        assert_eq!(voices_for(&twice, Language::Chinese), offered);

        let japanese = voices_for(&identified_voices(), Language::Japanese);
        assert_eq!(
            japanese.iter().map(Voice::id).collect::<Vec<_>>(),
            vec!["com.apple.eloquence.ja-JP.Eddy"]
        );
    }

    #[test]
    fn a_voice_with_no_identifier_is_stored_by_its_name() {
        // Android's names are unique and its listing carries no identifier, so
        // a choice there is stored exactly as it was before identifiers existed.
        let android = Voice::local("zh-cn-x-ccc-local", "zh-CN");
        assert_eq!(android.id(), "zh-cn-x-ccc-local");
        let listing: Voice = serde_json::from_str(
            r#"{"name":"zh-cn-x-ccc-local","locale":"zh-CN","network":false}"#,
        )
        .unwrap();
        assert_eq!(listing, android);
    }

    /// A bare-named AVFoundation list in several Chinese locales and two other
    /// languages, every voice identified — what a learner's Mac reports now.
    fn bare_voices() -> Vec<Voice> {
        [
            ("Eddy", "en-US"),
            ("Eddy", "zh-HK"),
            ("Eddy", "zh-TW"),
            ("Eddy", "zh-CN"),
            ("Eddy", "ja-JP"),
            ("Sinji", "zh-HK"),
            ("Meijia", "zh-TW"),
            ("Tingting", "zh-CN"),
            ("Kyoko", "ja-JP"),
        ]
        .into_iter()
        .map(|(name, locale)| {
            Voice::local(name, locale)
                .with_identifier(format!("com.apple.eloquence.{locale}.{name}"))
        })
        .collect()
    }

    #[test]
    fn a_suffixed_name_from_the_say_era_finds_the_bare_voice_in_its_locale() {
        // v0.3.0–v0.5.11 listed voices through `say -v '?'`, so their stored
        // names carry the qualifier: `Eddy (Chinese (China mainland))`. Today's
        // list says `Eddy`, and the qualifier is what says *which* Eddy.
        let voices = bare_voices();
        for (stored, locale) in [
            ("Eddy (Chinese (China mainland))", "zh-CN"),
            ("Eddy (Chinese (Taiwan))", "zh-TW"),
            ("Eddy (Chinese (Hong Kong))", "zh-HK"),
            ("Tingting (Chinese (China mainland))", "zh-CN"),
        ] {
            let found = find_voice(&voices, stored, Language::Chinese)
                .unwrap_or_else(|| panic!("{stored}"));
            assert_eq!(found.locale, locale, "{stored}");
        }
        let kyoko = find_voice(&voices, "Kyoko (Japanese (Japan))", Language::Japanese).unwrap();
        assert_eq!(kyoko.id(), "com.apple.eloquence.ja-JP.Kyoko");
        let eddy = find_voice(&voices, "Eddy (Japanese (Japan))", Language::Japanese).unwrap();
        assert_eq!(eddy.locale, "ja-JP");

        // A qualifier this file does not know is still the voice's name: the
        // home locale first, then the language's other locales.
        let odd = find_voice(&voices, "Eddy (Chinese (Somewhere))", Language::Chinese).unwrap();
        assert_eq!(odd.locale, "zh-CN");
        let without_mainland: Vec<Voice> = voices
            .iter()
            .filter(|v| v.locale != "zh-CN")
            .cloned()
            .collect();
        let taiwan = find_voice(
            &without_mainland,
            "Eddy (Chinese (China mainland))",
            Language::Chinese,
        )
        .unwrap();
        assert_eq!(
            taiwan.locale, "zh-TW",
            "Taiwan before Hong Kong for Mandarin"
        );

        // A suffixed name never reaches across languages.
        assert_eq!(
            find_voice(&voices, "Eddy (English (US))", Language::Japanese)
                .unwrap()
                .locale,
            "ja-JP"
        );
        assert!(find_voice(&voices, "Kyoko (Japanese (Japan))", Language::Chinese).is_none());
    }

    #[test]
    fn mandarin_falls_back_to_taiwan_before_hong_kong() {
        // Hong Kong's voices speak Cantonese; for a Mandarin learner with no
        // mainland voice, Taiwan's Mandarin is the nearer answer.
        let no_mainland: Vec<Voice> = bare_voices()
            .into_iter()
            .filter(|v| v.locale != "zh-CN")
            .collect();
        assert_eq!(
            pick_voice(&no_mainland, Language::Chinese).unwrap().locale,
            "zh-TW"
        );
    }

    #[test]
    fn an_override_may_name_another_languages_voice_by_identifier() {
        let listed = bare_voices();
        let english = resolve_voice(
            &listed,
            None,
            Some("com.apple.eloquence.en-US.Eddy"),
            Language::Chinese,
        )
        .expect("the override is used as given");
        let index = installed_index(&listed, &english).expect("and found to speak with");
        assert_eq!(listed[index].locale, "en-US");
    }

    #[test]
    fn a_speaker_resolves_its_stored_choice_against_its_own_voice_list() {
        let speaker = Speaker::new(Language::Chinese);
        speaker
            .voices
            .set(bare_voices())
            .expect("nothing has listed the voices yet");
        assert_eq!(speaker.chosen_voice(), None, "nothing chosen");
        for (stored, id) in [
            ("Eddy", "com.apple.eloquence.zh-CN.Eddy"),
            ("Eddy (Chinese (Taiwan))", "com.apple.eloquence.zh-TW.Eddy"),
            (
                "com.apple.eloquence.zh-HK.Eddy",
                "com.apple.eloquence.zh-HK.Eddy",
            ),
        ] {
            speaker.set_voice(Some(stored));
            assert_eq!(
                speaker
                    .chosen_voice()
                    .map(|v| v.id().to_string())
                    .as_deref(),
                Some(id),
                "{stored}"
            );
            if std::env::var_os(Language::Chinese.override_var()).is_none() {
                assert_eq!(
                    speaker.voice().map(|v| v.id().to_string()).as_deref(),
                    Some(id),
                    "{stored}"
                );
            }
        }
        // Another language's voice is not a choice this speaker can honour.
        speaker.set_voice(Some("com.apple.eloquence.en-US.Eddy"));
        assert_eq!(speaker.chosen_voice(), None);
    }

    #[test]
    fn a_choice_found_only_in_another_language_is_reported_as_such() {
        let speaker = Speaker::new(Language::Japanese);
        speaker.voices.set(bare_voices()).expect("fresh speaker");
        assert_eq!(speaker.chosen_elsewhere(), None, "nothing chosen");
        speaker.set_voice(Some("Kyoko"));
        assert_eq!(speaker.chosen_elsewhere(), None, "the choice resolves");
        speaker.set_voice(Some("Meijia"));
        assert_eq!(
            speaker.chosen_elsewhere().map(|v| v.locale).as_deref(),
            Some("zh-TW")
        );
        speaker.set_voice(Some("com.apple.eloquence.en-US.Eddy"));
        assert_eq!(
            speaker.chosen_elsewhere().map(|v| v.locale).as_deref(),
            Some("en-US")
        );
        speaker.set_voice(Some("Nobody At All"));
        assert_eq!(speaker.chosen_elsewhere(), None, "nowhere on this machine");
    }

    #[test]
    fn a_voices_quality_is_read_from_the_platform_or_its_identifier() {
        assert_eq!(quality_name(1), Some("standard"));
        assert_eq!(quality_name(2), Some("enhanced"));
        assert_eq!(quality_name(3), Some("premium"));
        assert_eq!(quality_name(0), None);
        assert_eq!(
            quality_from_identifier("com.apple.voice.compact.zh-CN.Tingting"),
            Some("standard")
        );
        assert_eq!(
            quality_from_identifier("com.apple.voice.enhanced.zh-CN.Tingting"),
            Some("enhanced")
        );
        assert_eq!(
            quality_from_identifier("com.apple.voice.premium.ja-JP.Kyoko"),
            Some("premium")
        );
        assert_eq!(
            quality_from_identifier("com.apple.eloquence.zh-CN.Eddy"),
            None
        );
        assert_eq!(quality_from_identifier(""), None);
    }

    #[test]
    fn a_compact_and_an_enhanced_voice_of_one_name_are_two_entries() {
        let compact = Voice::local("Tingting", "zh-CN")
            .with_identifier("com.apple.voice.compact.zh-CN.Tingting")
            .with_quality(Some("standard"));
        let enhanced = Voice::local("Tingting", "zh-CN")
            .with_identifier("com.apple.voice.enhanced.zh-CN.Tingting")
            .with_quality(Some("enhanced"));
        let offered = voices_for(&[compact.clone(), enhanced.clone()], Language::Chinese);
        assert_eq!(offered, vec![compact, enhanced]);
    }

    /// Every voice the settings screen would offer on this Mac is found again
    /// by the lookup that speaks with it, and an utterance can be given it.
    ///
    /// The real `speechVoices()` list rather than a fixture, so a difference
    /// between what `voice_from` reads and what `utterance_for` looks up fails
    /// here and not in a learner's ear. It asserts nothing about which voices
    /// exist: a runner with no Chinese or Japanese voice has nothing to check,
    /// and passes. macOS only — iOS tests have no run loop to answer `with_main`.
    #[cfg(target_os = "macos")]
    #[test]
    fn every_offered_voice_is_found_again_on_this_mac() {
        let listed = list_voices().expect("listing the voices");
        for language in [Language::Chinese, Language::Japanese] {
            for voice in voices_for(&listed, language) {
                let index = installed_index(&listed, &voice)
                    .unwrap_or_else(|| panic!("{voice:?} was not found again"));
                assert_eq!(listed[index].id(), voice.id());
                let made = with_main(move || utterance_for("汉", &voice).map(|_| ()));
                assert_eq!(made, Ok(()));
            }
        }
    }

    // There is deliberately no test here that asserts *which* voices a machine
    // has installed (there was one — `the_system_actually_offers_a_chinese_voice`
    // — before this file moved off `say`). The *list* is real: `list_voices` goes
    // through `with_main`, whose macOS side is a worker thread this module owns,
    // precisely so that a `cargo test` binary with no run loop can still read
    // `AVSpeechSynthesisVoice.speechVoices()` — which is why
    // `choosing_a_voice_does_not_change_what_the_speaker_resolves_to_elsewhere`
    // can call `voice()` at all. What no test may assert is that a particular
    // voice is present, because a CI machine and a learner's machine do not have
    // the same set.
    //
    // There is deliberately no `cargo test` equivalent of "speak a character and
    // listen to it" either: `speak` and `stop` on macOS reach real AVFoundation
    // calls with no test-only stand-in, and a test that spoke would make a noise
    // on whoever is at the machine. iOS was never testable this way, for the
    // run-loop reason. Pronunciation is verified by running the actual built app —
    // see `HANDOVER_NIHONGO.md` §5 for the probe that does it, which is how the
    // kana tutor's Japanese voice was confirmed as `Kyoko (ja-JP)`.
}
