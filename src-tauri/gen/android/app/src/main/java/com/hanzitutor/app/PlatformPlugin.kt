// The Android side of the platform seam.
//
// Everything here exists because it is only reachable from Java: the system
// speech synthesiser, and the window insets that the page cannot measure for
// itself. Rust calls in through Tauri's mobile plugin machinery — see
// `src-tauri/src/platform.rs` — and the commands below are the whole surface.
//
// Three facts about how these run, all learned the hard way:
//
//  * **They run on the main thread.** Tauri dispatches mobile plugin commands
//    through `run_on_android_context`, which is the UI thread. So nothing here
//    may block, which is why starting the synthesiser is asynchronous and
//    commands that arrive first are *held* rather than waited for. The
//    fingerprint prompt is asynchronous for the same reason: `BiometricPrompt`
//    answers from a callback, so a learner may take as long as they like over
//    it without the interface waiting on them.
//  * **`env(safe-area-inset-*)` is not the status bar.** Android's WebView
//    reports the *display cutout* through those CSS variables, so on a phone
//    with a punch-hole camera the layout is inset correctly and on one without
//    any cutout — an emulator, or a phone with a bezel — they are all zero and
//    the header ends up underneath the clock. The `insets` command is how the
//    frontend gets the real system bar heights instead of guessing.
//  * **`speak` returning SUCCESS does not mean anything was heard.** The engine
//    answers `SUCCESS` as soon as it has *accepted* an utterance, and then says
//    nothing at all if the voice's data was never downloaded, if a network
//    voice has no network, or if the output cannot be opened. That is
//    indistinguishable from working unless the utterance's own progress
//    callbacks are listened to, which is what `UtteranceProgressListener` is
//    here for: `speak` now resolves when the engine reports that it has
//    **started**, and rejects with the engine's own reason when it does not.

package com.hanzitutor.app

import android.app.Activity
import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Handler
import android.os.Looper
import android.provider.Settings
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import android.util.Base64
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.fragment.app.FragmentActivity
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.io.FileOutputStream
import java.security.KeyStore
import java.security.UnrecoverableEntryException
import java.util.Locale
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** The arguments of `speak`. */
@InvokeArg
class SpeakArgs {
    lateinit var text: String
    var voice: String? = null
}

/** The arguments of `saveSecret`. */
@InvokeArg
class SaveSecretArgs {
    lateinit var secret: String

    /**
     * Whether the learner wants to be asked for a fingerprint before the sign-in
     * is read back. Writing it does not ask for anything either way — see
     * `saveSecret` — so this chooses the mode of the key, not of this call.
     */
    var requireAuth: Boolean = false
}

@TauriPlugin
class PlatformPlugin(private val activity: Activity) : Plugin(activity) {
    /** The synthesiser, once it has started. Null until then. */
    private var engine: TextToSpeech? = null

    /** Why the synthesiser never started, if it did not. */
    private var startupProblem: String? = null

    /** Work held back until the synthesiser is ready, in arrival order. */
    private val waiting = mutableListOf<() -> Unit>()

    /** The caller waiting to hear that an utterance began, if any. */
    private var speaking: Invoke? = null

    /** Which utterance that caller is waiting on, so stale callbacks are ignored. */
    private var speakingUtterance: String? = null

    /** The guard that answers a caller whose utterance never reported anything. */
    private var speakTimeout: Runnable? = null

    private val main = Handler(Looper.getMainLooper())

    /** The audio policy, for asking to be heard rather than assumed. */
    private val audio: AudioManager? by lazy {
        activity.getSystemService(AudioManager::class.java)
    }

    /** The focus this app is currently holding, if any. */
    private var focus: AudioFocusRequest? = null

    /** The recorder while it is running. */
    private var recordDevice: AudioRecord? = null

    /** The thread filling the file. Nothing joins it — it answers for itself. */
    private var recordThread: Thread? = null

    /** Which audio source opened, for the status line. */
    private var recordSource: String = ""

    /**
     * Cleared to end the read loop. Written on the main thread, read on the
     * audio one, so it has to be volatile; the loop notices within one buffer.
     */
    @Volatile
    private var recording = false

    /** Whoever is waiting to be told that the recording file is complete. */
    private var pendingStop: Invoke? = null

    /** A recording that finished before anything asked for it, rate and problem. */
    private var finished: Pair<Int, String?>? = null

    /** What the engine said the last time it was asked to speak. */
    @Volatile
    private var lastProblem: String? = null

    // ---- system bar insets -------------------------------------------------

    /**
     * The window's system bar insets, in CSS pixels.
     *
     * `systemBars()` covers the status and navigation bars; `displayCutout()`
     * covers a notch or punch-hole that reaches into them. Asking for the union
     * is what makes one number work on both a notched phone and a plain one.
     * The values are divided by the display density because the page thinks in
     * CSS pixels and Android reports device ones.
     */
    @Command
    fun insets(invoke: Invoke) {
        val bars = ViewCompat.getRootWindowInsets(activity.window.decorView)
            ?.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
            )
        val density = activity.resources.displayMetrics.density.toDouble()
        val result = JSObject()
        result.put("top", (bars?.top ?: 0) / density)
        result.put("bottom", (bars?.bottom ?: 0) / density)
        result.put("left", (bars?.left ?: 0) / density)
        result.put("right", (bars?.right ?: 0) / density)
        invoke.resolve(result)
    }

    // ---- speech ------------------------------------------------------------

    /**
     * Every voice the system offers, on-device ones first.
     *
     * The order is load-bearing rather than tidy. Rust picks the first Mandarin
     * voice it finds to speak with, and Android lists a network-backed voice for
     * a locale right beside the on-device one — and this app's whole premise is
     * that it works with no network at all. Sorting the offline voices to the
     * front means the automatic choice is the one that keeps that promise. The
     * settings screen sorts what it shows by name itself, so this ordering is
     * not visible as a list that shuffles.
     *
     * Names are Android's own (`zh-cn-x-ccc-local`), and the locale is reported
     * as a BCP-47 tag (`zh-CN`), which is what the Rust side already expects
     * from iOS.
     */
    @Command
    fun voices(invoke: Invoke) {
        withEngine(invoke) { tts ->
            val list = JSArray()
            for (voice in tts.voices.orEmpty().sortedWith(
                // Voices that can actually be spoken with come first, then
                // on-device ones over network ones, then alphabetically. Rust
                // takes the first Mandarin voice it sees, so this ordering is
                // what decides which voice the app speaks with by default —
                // and a voice whose data was never downloaded is not a choice.
                compareBy({ !isInstalled(it) }, { it.isNetworkConnectionRequired }, { it.name })
            )) {
                val entry = JSObject()
                entry.put("name", voice.name)
                entry.put("locale", voice.locale.toLanguageTag())
                // Carried through so the settings screen can say which voices
                // need a network connection. This app is meant to work on a
                // train, and a learner choosing a voice deserves to know before
                // they choose it rather than after.
                entry.put("network", voice.isNetworkConnectionRequired)
                list.put(entry)
            }
            val result = JSObject()
            result.put("voices", list)
            invoke.resolve(result)
        }
    }

    /**
     * Speak [SpeakArgs.text] and answer once the engine has actually begun.
     *
     * `QUEUE_FLUSH` is the whole of "cut off the previous utterance": the Rust
     * side stops the last word before starting the next, and this is how the
     * synthesiser is told to mean it rather than to queue up behind it.
     *
     * The caller is answered by the progress listener rather than here — see the
     * note at the top of this file about `SUCCESS` meaning nothing. A named voice
     * that is no longer installed, or whose data is missing, falls back to
     * Mandarin and, failing that, is reported: a different voice is a far better
     * answer than silence, but silence is never an answer.
     */
    @Command
    fun speak(invoke: Invoke) {
        val args = invoke.parseArgs(SpeakArgs::class.java)
        withEngine(invoke) { tts ->
            val unusable = applyVoice(tts, args.voice)
            if (unusable != null) {
                invoke.reject(unusable)
                return@withEngine
            }

            lastProblem = null
            speaking = invoke
            makeAudible(tts)
            val utterance = UUID.randomUUID().toString()
            speakingUtterance = utterance
            if (tts.speak(args.text, TextToSpeech.QUEUE_FLUSH, null, utterance) == TextToSpeech.ERROR) {
                speaking = null
                speakingUtterance = null
                invoke.reject("the speech engine refused to start on that text")
                return@withEngine
            }

            // An engine that reports neither start nor failure would leave the
            // caller waiting for ever, and a learner staring at a button that
            // never answers. Three seconds is far longer than starting takes.
            val held = invoke
            val guard = Runnable {
                if (speaking === held) {
                    speaking = null
                    speakingUtterance = null
                    lastProblem = "the speech engine never reported starting"
                    giveBackAudio()
                    held.resolve()
                }
            }
            speakTimeout = guard
            main.postDelayed(guard, SPEAK_TIMEOUT_MS)
        }
    }

    /** Stop the current utterance. */
    @Command
    fun stop(invoke: Invoke) {
        // Not routed through `withEngine`: if the engine never started there is
        // nothing to stop, and starting one in order to stop it would be absurd.
        engine?.stop()
        giveBackAudio()
        invoke.resolve()
    }

    /**
     * Say that this sound is deliberate, and route it where the learner expects.
     *
     * The same lesson iOS taught, met again on Android through a different
     * mechanism. A synthesiser is not automatically audible: the default
     * attributes put speech on the accessibility usage, and a vendor ROM may
     * mute outright what it decides is background playback — the RedMagic build
     * logs `AudioHardening background playback would be muted for
     * com.google.android.tts`, which is precisely "the button does nothing".
     * Taking audio focus, and asking for the media usage with speech content, is
     * what distinguishes a learner who tapped a button from a process talking to
     * itself. Ducking rather than stopping is borrowed from the iOS session's
     * `duckOthers`: a learner's music comes back when the word ends.
     */
    private fun makeAudible(tts: TextToSpeech) {
        val speech = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
            .build()
        tts.setAudioAttributes(speech)
        val manager = audio ?: return
        val request = AudioFocusRequest
            .Builder(AudioManager.AUDIOFOCUS_GAIN_TRANSIENT_MAY_DUCK)
            .setAudioAttributes(speech)
            .build()
        focus = request
        manager.requestAudioFocus(request)
    }

    /** Give the audio back, so anything that was ducked returns to volume. */
    private fun giveBackAudio() {
        val manager = audio ?: return
        val request = focus ?: return
        focus = null
        manager.abandonAudioFocusRequest(request)
    }

    // ---- microphone --------------------------------------------------------

    /**
     * Whether the microphone can be captured from, and at what rate.
     *
     * `AudioRecord` is the thing that actually records on Android — `cpal`'s
     * AAudio input starts a stream, reports success, and then never calls back
     * at all (see `HANDOVER.md` §9) — so this asks `AudioRecord` rather than
     * reporting a microphone that cannot be used.
     */
    @Command
    fun recordStatus(invoke: Invoke) {
        val result = JSObject()
        val running = recordDevice
        if (running != null) {
            result.put("available", true)
            result.put("sampleRate", RECORD_RATE)
            result.put("source", recordSource)
            result.put("recording", true)
            result.put("problem", "")
            invoke.resolve(result)
            return
        }
        val rate = RECORD_RATE
        val minimum = AudioRecord.getMinBufferSize(
            rate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        if (minimum <= 0) {
            result.put("available", false)
            result.put("sampleRate", 0)
            result.put("source", "")
            result.put("recording", false)
            result.put("problem", "AudioRecord reports no usable buffer size ($minimum).")
            invoke.resolve(result)
            return
        }
        val opened = openRecorder(rate, minimum)
        if (opened == null) {
            result.put("available", false)
            result.put("sampleRate", 0)
            result.put("source", "")
            result.put("recording", false)
            result.put("problem", "No audio source on this device would open for recording.")
            invoke.resolve(result)
            return
        }
        val (device, source) = opened
        device.release()
        result.put("available", true)
        result.put("sampleRate", rate)
        result.put("source", source)
        result.put("recording", false)
        result.put("problem", "")
        invoke.resolve(result)
    }

    /**
     * Begin recording, writing PCM16 little-endian mono to a scratch file.
     *
     * Answers as soon as the device is running, like the desktop backend, and
     * lets the reading happen on a thread of its own — a blocking `read` on the
     * main thread would freeze the interface for as long as the learner holds the
     * button.
     */
    @Command
    fun recordStart(invoke: Invoke) {
        if (recordDevice != null) {
            invoke.reject("already recording")
            return
        }
        val rate = RECORD_RATE
        val minimum = AudioRecord.getMinBufferSize(
            rate,
            AudioFormat.CHANNEL_IN_MONO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        if (minimum <= 0) {
            invoke.reject("AudioRecord reports no usable buffer size ($minimum).")
            return
        }
        val opened = openRecorder(rate, minimum)
        if (opened == null) {
            invoke.reject("No audio source on this device would open for recording.")
            return
        }
        val (device, source) = opened
        val file = File(activity.cacheDir, RECORDING_FILE)
        recording = true
        finished = null
        val thread = Thread {
            val problem = captureTo(device, file, rate)
            main.post { finishRecording(rate, problem) }
        }
        thread.start()
        recordDevice = device
        recordThread = thread
        recordSource = source
        val result = JSObject()
        result.put("sampleRate", rate)
        result.put("source", source)
        invoke.resolve(result)
    }

    /**
     * Stop, and answer once the file is complete.
     *
     * The audio thread answers, not this one: the file is only whole after that
     * thread has closed it, and Rust reads it the moment this resolves. Nothing
     * here blocks — the thread notices the flag within one buffer, which is a
     * tenth of a second.
     */
    @Command
    fun recordStop(invoke: Invoke) {
        if (recordDevice == null || recordThread == null) {
            invoke.reject("not recording")
            return
        }
        recording = false
        // A thread that has already posted its result: answer from that rather
        // than waiting for a callback that has been and gone.
        val done = finished
        if (done != null) {
            finished = null
            resolveRecording(invoke, done.first, done.second)
            return
        }
        pendingStop = invoke
    }

    /**
     * The first audio source that will open, and its name.
     *
     * `VOICE_RECOGNITION` first, because it is the source Android offers for
     * speech and on the phone this was measured on it gave ten times the level of
     * `UNPROCESSED` — which is the purer choice for pitch in theory, and too
     * quiet to be one there. `MIC` is the fallback, for devices that will not
     * admit to having the first.
     */
    private fun openRecorder(rate: Int, minimum: Int): Pair<AudioRecord, String>? {
        val candidates = listOf(
            "VOICE_RECOGNITION" to MediaRecorder.AudioSource.VOICE_RECOGNITION,
            "MIC" to MediaRecorder.AudioSource.MIC
        )
        for ((name, source) in candidates) {
            val device = try {
                AudioRecord(
                    source,
                    rate,
                    AudioFormat.CHANNEL_IN_MONO,
                    AudioFormat.ENCODING_PCM_16BIT,
                    maxOf(minimum, rate * 2)
                )
            } catch (problem: Exception) {
                continue
            }
            if (device.state == AudioRecord.STATE_INITIALIZED) {
                return device to name
            }
            device.release()
        }
        return null
    }

    /**
     * Read until told to stop. Returns why it stopped early, or null.
     *
     * The cap here is a memory guard, not the app's limit: ten seconds is the
     * app's policy and it belongs in Rust, which applies it when it reads the
     * file back. This one only stops a button held down for an hour from filling
     * the cache directory.
     */
    private fun captureTo(device: AudioRecord, file: File, rate: Int): String? {
        val samples = ShortArray(rate / 10)
        val bytes = ByteArray(samples.size * 2)
        var written = 0L
        val cap = rate.toLong() * MAX_RECORD_SECONDS * 2
        var problem: String? = null
        try {
            FileOutputStream(file).use { out ->
                device.startRecording()
                while (recording && written < cap) {
                    val read = device.read(samples, 0, samples.size)
                    if (read < 0) {
                        problem = "the microphone stopped (AudioRecord error $read)"
                        break
                    }
                    if (read == 0) continue
                    var at = 0
                    for (index in 0 until read) {
                        val value = samples[index].toInt()
                        bytes[at++] = (value and 0xFF).toByte()
                        bytes[at++] = ((value shr 8) and 0xFF).toByte()
                    }
                    out.write(bytes, 0, at)
                    written += at
                }
                device.stop()
            }
        } catch (failure: Exception) {
            problem = "the microphone stopped: ${failure.message}"
        } finally {
            device.release()
        }
        return problem
    }

    /** Hand the finished recording to whoever asked for it, on the main thread. */
    private fun finishRecording(rate: Int, problem: String?) {
        recordDevice = null
        recordThread = null
        recording = false
        val held = pendingStop
        pendingStop = null
        if (held == null) {
            // Nobody has asked yet. Keep it for the stop that is about to come.
            finished = rate to problem
            return
        }
        resolveRecording(held, rate, problem)
    }

    private fun resolveRecording(invoke: Invoke, rate: Int, problem: String?) {
        if (problem != null) {
            invoke.reject(problem)
            return
        }
        val result = JSObject()
        result.put("path", File(activity.cacheDir, RECORDING_FILE).absolutePath)
        result.put("sampleRate", rate)
        invoke.resolve(result)
    }

    // ---- the Dropbox sign-in -----------------------------------------------

    /**
     * Keep the Dropbox refresh token where copying a file does not give it away.
     *
     * The token is long-lived and is the whole of this app's access to a
     * learner's Dropbox, so it does not belong in the study database — an
     * ordinary file in an ordinary directory that backup tools copy around.
     * Android has no keychain of the Apple kind; what it has is the **keystore**,
     * which holds *keys*, not secrets. So the shape here is: an AES-256-GCM key
     * generated inside the keystore, never exportable, used to encrypt the token,
     * with only the ciphertext written to preferences. The key material lives in
     * the device's secure hardware where there is any, and the file on disk is
     * useless without this device.
     *
     * **Asking for the learner.** [SaveSecretArgs.requireAuth] has the key made
     * with `setUserAuthenticationRequired`, which is what puts the sign-in behind
     * a fingerprint, a face or the device PIN. That is a property of the key,
     * fixed when the key is made, so the mode the blob was written in is recorded
     * beside it and a change of mode rebuilds the key — see [store]. Writing asks
     * for nothing even then: keystore permits encryption with such a key and
     * refuses only decryption, which is why the prompt lives in [loadSecret] and
     * can never appear while the learner is connecting.
     *
     * A request for authentication that the device cannot honour — a phone with
     * no screen lock at all, where keystore will not make such a key — degrades
     * to a key that asks nothing rather than refusing to connect, and the answer
     * says which of the two the token actually got. That is the same honesty as
     * the Rust side's `Protection`, and the reason this command answers at all
     * rather than resolving an empty object.
     *
     * **Why the ciphertext is what is written.** `apply()` rather than `commit()`:
     * the write is to this app's own preferences and a lost race with process
     * death costs one reconnect, while blocking the main thread — which is where
     * mobile plugin commands run — costs the frame the learner is looking at.
     */
    @Command
    fun saveSecret(invoke: Invoke) {
        val args = invoke.parseArgs(SaveSecretArgs::class.java)
        var asksForAuth = false
        try {
            store(args.secret, args.requireAuth)
            asksForAuth = args.requireAuth
        } catch (problem: Exception) {
            if (!args.requireAuth) {
                invoke.reject(notStored(problem))
                return
            }
            // A device with no screen lock cannot make a key that asks for
            // authentication at all. Refusing to connect there would trade a
            // working sign-in for a principle, so the request falls back to the
            // mode every earlier build used — and the answer admits that it did.
            forgetSecret()
            try {
                store(args.secret, false)
            } catch (fallback: Exception) {
                invoke.reject(notStored(fallback))
                return
            }
        }
        val result = JSObject()
        result.put("protected", asksForAuth)
        invoke.resolve(result)
    }

    /** What a keystore that would not keep the sign-in is told, without a stack. */
    private fun notStored(problem: Exception): String =
        "the Android keystore would not store the Dropbox sign-in: ${problem.message}"

    /**
     * Encrypt [secret] under a keystore key in the mode [requireAuth] names.
     *
     * The mode cannot be changed on a key that already exists, so a request that
     * differs from the mode the stored blob is in cannot be met by re-encrypting:
     * the key has to go and be made again. The blob goes with it rather than
     * being left encrypted under a key nothing will use again, which is also what
     * keeps the recorded mode and the key from ever disagreeing.
     */
    private fun store(secret: String, requireAuth: Boolean) {
        val prefs = preferences()
        if (prefs.getBoolean(SECRET_AUTH_ENTRY, false) != requireAuth) {
            forgetSecret()
        }
        val cipher = Cipher.getInstance(SECRET_TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, secretKey(requireAuth))
        val encrypted = cipher.doFinal(secret.toByteArray(Charsets.UTF_8))
        // The IV is generated per encryption and is not secret; it has to be
        // kept, because GCM cannot decrypt without it.
        val blob = encode(cipher.iv) + ":" + encode(encrypted)
        prefs.edit()
            .putString(SECRET_ENTRY, blob)
            .putBoolean(SECRET_AUTH_ENTRY, requireAuth)
            .apply()
    }

    /**
     * Read the sign-in back, or answer without one.
     *
     * An absent `secret` in the answer means "nothing is stored", which is the
     * state a device nobody has connected is in. That is spelled as an *omitted*
     * field rather than a null one on purpose: `JSObject` is a `JSONObject`, and
     * `put(key, null)` removes the mapping instead of writing a null, so an
     * explicit null would arrive as the same absent field by a route nobody could
     * read from the code.
     *
     * `protected` says whether the blob is behind the learner's fingerprint, a
     * face or the device PIN, which the Rust side reports as `userPresence` or
     * `keychainOnly`. It is read from beside the blob rather than inferred from
     * the key, because it is what decides whether this call can answer at once or
     * has to ask somebody first.
     *
     * A blob that cannot be decrypted is **forgotten rather than reported**, and
     * that is a decision rather than laziness. It is what a restored backup looks
     * like: the preferences came back and the keystore key did not, because the
     * key is bound to the device. It is also what a fingerprint enrolled after
     * the key was made looks like, because keystore invalidates the key instead
     * of letting it decrypt. An undecryptable token is worth nothing to anybody,
     * and failing here would leave a learner looking at an error they cannot act
     * on, when the useful thing is to be told they are not connected and offered
     * Connect.
     */
    @Command
    fun loadSecret(invoke: Invoke) {
        val blob = preferences().getString(SECRET_ENTRY, null)
        if (blob == null) {
            invoke.resolve(JSObject())
            return
        }
        val parts = blob.split(":")
        if (parts.size != 2) {
            // Not shaped like anything this wrote, so no key will decrypt it.
            forgetSecret()
            invoke.resolve(JSObject())
            return
        }
        val iv = Base64.decode(parts[0], Base64.NO_WRAP)
        val encrypted = Base64.decode(parts[1], Base64.NO_WRAP)
        if (!preferences().getBoolean(SECRET_AUTH_ENTRY, false)) {
            // A key that asks for nothing, so reading is silent — which is what
            // a sync that starts by itself requires. See the note at the top.
            resolveSecret(invoke, decrypt(iv, encrypted), false)
            return
        }
        askForFingerprint(invoke, iv, encrypted)
    }

    /**
     * Whether the learner can be asked to identify themselves at all.
     *
     * Rust asks this before it offers the switch, so a device that cannot ask is
     * told so rather than offered a setting that would do nothing. The device
     * credential is part of the question because a phone whose only lock is a PIN
     * can still satisfy a keystore key made with `setUserAuthenticationRequired`,
     * so "no fingerprint reader" is not the same as "cannot ask".
     */
    @Command
    fun canLock(invoke: Invoke) {
        val status = BiometricManager.from(activity).canAuthenticate(ASKABLE)
        val result = JSObject()
        result.put("available", status == BiometricManager.BIOMETRIC_SUCCESS)
        invoke.resolve(result)
    }

    /** Forget the sign-in, the mode it was kept in, and the key that encrypted it. */
    @Command
    fun clearSecret(invoke: Invoke) {
        forgetSecret()
        invoke.resolve(JSObject())
    }

    /** This app's own preferences, where the encrypted blob lives. */
    private fun preferences() =
        activity.getSharedPreferences(SECRET_PREFERENCES, Context.MODE_PRIVATE)

    /**
     * Forget the blob, the mode it was in, and the key that encrypted it.
     *
     * All three together: a blob whose key is gone is readable by nobody, and a
     * mode left behind after its blob would have the next write keep a key from
     * the other mode — the one thing [store] must not do.
     */
    private fun forgetSecret() {
        preferences().edit().remove(SECRET_ENTRY).remove(SECRET_AUTH_ENTRY).apply()
        try {
            KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }.deleteEntry(SECRET_KEY_ALIAS)
        } catch (problem: Exception) {
            // A keystore with no key in it is the ordinary case for a device that
            // never stored one, and not a failure worth reporting.
        }
    }

    /**
     * Ask the learner to identify themselves, then decrypt with the cipher the
     * prompt authorises.
     *
     * The cipher is built and initialised first because `CryptoObject` is how the
     * system ties one successful authentication to one keystore operation:
     * keystore gives the operation a challenge at init and refuses to finish it
     * until that challenge has been answered. Nothing here blocks — `authenticate`
     * returns at once and the learner's answer arrives in the callback, which is
     * where the invoke is resolved. That is the note at the top of this file,
     * applied to a prompt instead of to the synthesiser.
     */
    private fun askForFingerprint(invoke: Invoke, iv: ByteArray, encrypted: ByteArray) {
        val host = activity as? FragmentActivity
        if (host == null) {
            invoke.reject(
                "this app cannot show a fingerprint prompt on this device, so the stored " +
                    "Dropbox sign-in cannot be read"
            )
            return
        }
        val cipher = try {
            val prepared = Cipher.getInstance(SECRET_TRANSFORMATION)
            prepared.init(
                Cipher.DECRYPT_MODE,
                secretKey(true),
                GCMParameterSpec(SECRET_TAG_BITS, iv)
            )
            prepared
        } catch (problem: UserNotAuthenticatedException) {
            // Keystore wants the learner authenticated before it will even prepare
            // the operation. The prompt is what authenticates, and it cannot be
            // handed a cipher that failed to initialise, so this is reported
            // rather than treated as an undecryptable blob — the sign-in may
            // still be perfectly good.
            invoke.reject(
                "this device will not prepare the Dropbox sign-in to be read, so it was not read"
            )
            return
        } catch (problem: Exception) {
            // Keystore refused the operation outright rather than asking anybody:
            // a key invalidated by a newly enrolled fingerprint is the ordinary
            // case, and the blob is undecryptable by any route then, so it goes
            // the way of a restored backup instead of becoming an error.
            forgetSecret()
            invoke.resolve(JSObject())
            return
        }
        var answered = false
        val prompt = BiometricPrompt(
            host,
            ContextCompat.getMainExecutor(activity),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(
                    result: BiometricPrompt.AuthenticationResult
                ) {
                    if (answered) return
                    answered = true
                    val unlocked = result.cryptoObject?.cipher
                    if (unlocked == null) {
                        // The prompt hands back the cipher it was given. Without it
                        // there is nothing that can decrypt, and a blob that may
                        // still be good is not worth throwing away over that.
                        invoke.reject(
                            "the fingerprint prompt did not return the key it was given, so " +
                                "the Dropbox sign-in could not be read"
                        )
                        return
                    }
                    val secret = try {
                        String(unlocked.doFinal(encrypted), Charsets.UTF_8)
                    } catch (problem: Exception) {
                        forgetSecret()
                        null
                    }
                    resolveSecret(invoke, secret, true)
                }

                override fun onAuthenticationError(errorCode: Int, message: CharSequence) {
                    if (answered) return
                    answered = true
                    invoke.reject(reasonForPrompt(errorCode, message))
                }

                override fun onAuthenticationFailed() {
                    // **The one callback that is not an answer.** A finger the sensor
                    // did not recognise leaves the prompt up and asks the learner to
                    // try again, which is why it is `onAuthenticationFailed` and not
                    // `onAuthenticationError` — the two are separate precisely so this
                    // one can be told apart from a refusal. Resolving here would turn
                    // a slightly misplaced finger into a failed sync and a prompt that
                    // vanished before it could be used, so nothing happens: the answer
                    // arrives from the success callback or from the error one, and the
                    // learner gets the retry the system is offering them.
                }
            }
        )
        val info = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Unlock your Dropbox sign-in")
            .setSubtitle("Confirm it is you to read the sign-in saved on this device.")
            // No negative button: with the device credential allowed the system
            // draws its own cancel, and adding a second one is refused outright
            // rather than being a belt and braces.
            .setAllowedAuthenticators(ASKABLE)
            .build()
        prompt.authenticate(info, BiometricPrompt.CryptoObject(cipher))
    }

    /**
     * Answer with the sign-in and how well it is protected, or with neither.
     *
     * The protection is what Rust turns into `userPresence` or `keychainOnly`,
     * and it is why a read that asked for nothing can still be honest about an
     * item that is worth asking for: a phone with no screen lock answers no to
     * the switch and keychainOnly here, while one that does ask answers
     * userPresence.
     */
    private fun resolveSecret(invoke: Invoke, secret: String?, guarded: Boolean) {
        val result = JSObject()
        if (secret != null) {
            result.put("secret", secret)
            result.put("protected", guarded)
        }
        invoke.resolve(result)
    }

    /**
     * The prompt's own reason in words, so the learner is told which of the
     * possibilities happened instead of being handed a code.
     *
     * The system's `message` is a last resort: it is localised and can be as
     * uninformative as "Authenticate error", while these codes each ask something
     * different of the learner — to try again, to enrol a fingerprint, or to stop
     * trying and use the PIN.
     */
    private fun reasonForPrompt(errorCode: Int, message: CharSequence): String = when (errorCode) {
        BiometricPrompt.ERROR_USER_CANCELED,
        BiometricPrompt.ERROR_NEGATIVE_BUTTON,
        BiometricPrompt.ERROR_CANCELED ->
            "the request to read the Dropbox sign-in was cancelled"
        BiometricPrompt.ERROR_NO_BIOMETRICS ->
            "no fingerprint or face is set up on this device, so the Dropbox sign-in cannot be read"
        BiometricPrompt.ERROR_HW_NOT_PRESENT,
        BiometricPrompt.ERROR_HW_UNAVAILABLE ->
            "this device has no fingerprint reader available, so the Dropbox sign-in cannot be read"
        BiometricPrompt.ERROR_NO_DEVICE_CREDENTIAL ->
            "this device has no screen lock set up, so the Dropbox sign-in cannot be read"
        BiometricPrompt.ERROR_LOCKOUT,
        BiometricPrompt.ERROR_LOCKOUT_PERMANENT ->
            "the fingerprint reader has had too many unrecognised attempts, so the Dropbox " +
                "sign-in was not read"
        BiometricPrompt.ERROR_TIMEOUT ->
            "the fingerprint prompt timed out, so the Dropbox sign-in was not read"
        BiometricPrompt.ERROR_SECURITY_UPDATE_REQUIRED ->
            "this device needs a security update before it can read the Dropbox sign-in"
        BiometricPrompt.ERROR_UNABLE_TO_PROCESS ->
            "the fingerprint reader could not read that, so the Dropbox sign-in was not read"
        else -> "the Dropbox sign-in could not be read: $message"
    }

    /**
     * The AES key used for the sign-in, created inside the keystore on first use.
     *
     * `AndroidKeyStore` is a *provider*, not a file: `KeyStore.getInstance` and
     * `load(null)` reach the hardware-backed store, and the private material never
     * leaves it. The key is generated once and kept — regenerating it would make
     * every stored token undecryptable — so [requireAuth] decides something only
     * when there is no key to keep, and a caller that wants the other mode has to
     * delete this one first. [store] does exactly that.
     *
     * A key a newly enrolled fingerprint invalidated does not read back as a key
     * at all. It can decrypt nothing, and the sign-in being written here is a
     * fresh one, so it is replaced rather than reported.
     */
    private fun secretKey(requireAuth: Boolean): SecretKey {
        val store = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        val existing = try {
            store.getEntry(SECRET_KEY_ALIAS, null) as? KeyStore.SecretKeyEntry
        } catch (problem: UnrecoverableEntryException) {
            null
        }
        if (existing != null) {
            return existing.secretKey
        }
        val spec = KeyGenParameterSpec.Builder(
            SECRET_KEY_ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
        if (requireAuth) {
            spec.setUserAuthenticationRequired(true)
            // A fingerprint enrolled after this key was made must not be able to
            // read what the old one encrypted. Keystore invalidates the key
            // instead, and the token then goes the way of a restored backup:
            // forgotten, rather than reported as an error nobody can act on.
            spec.setInvalidatedByBiometricEnrollment(true)
        }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        generator.init(spec.build())
        return generator.generateKey()
    }

    /**
     * The sign-in from a blob written by a key that asks for nothing.
     *
     * A failure is the restored-backup or invalidated-key case, and is forgotten
     * here for the reason [loadSecret] gives. The mode is not a parameter because
     * this is only ever reached on that silent path; a blob behind a prompt is
     * decrypted with the cipher the prompt authorises.
     */
    private fun decrypt(iv: ByteArray, encrypted: ByteArray): String? = try {
        val cipher = Cipher.getInstance(SECRET_TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, secretKey(false), GCMParameterSpec(SECRET_TAG_BITS, iv))
        String(cipher.doFinal(encrypted), Charsets.UTF_8)
    } catch (problem: Exception) {
        forgetSecret()
        null
    }

    /** Base64 without line wrapping, which would corrupt a blob split on `:`. */
    private fun encode(bytes: ByteArray): String = Base64.encodeToString(bytes, Base64.NO_WRAP)

    /**
     * Run [work] with the synthesiser, starting it on first use.
     *
     * `TextToSpeech` reports readiness through a callback and nothing may block
     * on this thread, so a command that arrives before the engine is up is put
     * in [waiting] and resolved when the engine reports back — which is the
     * normal case for the first call, because the pronunciation warm-up asks
     * for the voice list while the app is still starting.
     */
    private fun withEngine(invoke: Invoke, work: (TextToSpeech) -> Unit) {
        engine?.let {
            work(it)
            return
        }
        startupProblem?.let {
            invoke.reject(it)
            return
        }
        waiting.add {
            val started = engine
            if (started != null) {
                work(started)
            } else {
                invoke.reject(startupProblem ?: "the system speech engine did not start")
            }
        }
        if (waiting.size == 1) {
            start()
        }
    }

    /** Begin starting the synthesiser. Its answer arrives on this same thread. */
    private fun start() {
        engine = TextToSpeech(activity) { status ->
            if (status != TextToSpeech.SUCCESS) {
                startupProblem = "no usable speech engine is installed on this device"
            } else {
                listen(engine)
            }
            val held = waiting.toList()
            waiting.clear()
            for (work in held) {
                work()
            }
        }
    }

    /**
     * Watch one utterance from acceptance to its first sound, or to its failure.
     *
     * The callbacks arrive on a binder thread, not this one, so they only record
     * what happened and hand the answering back to the main thread — which is
     * where `Invoke` belongs.
     */
    private fun listen(tts: TextToSpeech?) {
        tts?.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
            override fun onStart(utteranceId: String?) {
                finishSpeaking(utteranceId, null)
            }

            override fun onDone(utteranceId: String?) {
                // The word is over, so hand the audio back and let anything that
                // was ducked return to volume.
                giveBackAudio()
            }

            override fun onError(utteranceId: String?) {
                finishSpeaking(utteranceId, "the speech engine reported that it could not speak")
                giveBackAudio()
            }

            override fun onError(utteranceId: String?, errorCode: Int) {
                finishSpeaking(utteranceId, reasonFor(errorCode))
                giveBackAudio()
            }
        })
    }

    /**
     * Answer whoever asked to speak, once, on the main thread.
     *
     * The utterance is matched by id because a callback for a *previous*
     * utterance can arrive after the next one has started — `speak` cuts off
     * what came before, and the engine reports that cancellation in its own
     * time. Without the match, an old failure would be reported against a new
     * request, which is a lie about which call failed. An id of null is the
     * engine telling us it does not know, and is taken at face value.
     */
    private fun finishSpeaking(utteranceId: String?, problem: String?) {
        if (utteranceId != null && utteranceId != speakingUtterance) {
            return
        }
        lastProblem = problem
        main.post {
            val held = speaking ?: return@post
            speaking = null
            speakingUtterance = null
            speakTimeout?.let(main::removeCallbacks)
            speakTimeout = null
            if (problem == null) {
                held.resolve()
            } else {
                held.reject(problem)
            }
        }
    }

    /**
     * Point the engine at the voice called [name], or at Mandarin.
     *
     * Returns a reason the voice cannot be used, or null when it can. The status
     * codes are the entire point: `setVoice` and `setLanguage` are the only way
     * to find out that a voice is *listed* but unusable — data never
     * downloaded, locale unsupported — which otherwise looks exactly like
     * success from `speak`.
     */
    private fun applyVoice(tts: TextToSpeech, name: String?): String? {
        val all = tts.voices.orEmpty()
        val chinese = all.filter { isChinese(it) }
        val installed = chinese.filter { isInstalled(it) }

        // Prefer what was asked for, but only if the engine can really speak
        // with it. Google's engine lists every voice it knows about — including
        // ones whose data has never been downloaded — and asking for one of
        // those fails with a service error that names nothing useful. This is
        // the whole reason the voice list is asked about `isInstalled` at all.
        val chosen = name?.let { wanted -> all.firstOrNull { it.name == wanted } }
            ?.takeIf { isInstalled(it) }
            ?: installed.firstOrNull()
            ?: all.firstOrNull { it.name == name }

        if (chosen == null) {
            // Nothing Chinese can be spoken with. Which of the two situations
            // this is matters, because they ask different things of the learner.
            return if (chinese.isEmpty()) {
                "this device's speech engine has no Chinese voice at all"
            } else {
                "the Chinese voice data has not been downloaded on this device, so " +
                    "there is nothing to speak with — see Settings → System → " +
                    "Languages & input → Text-to-speech output"
            }
        }

        if (tts.setVoice(chosen) == TextToSpeech.SUCCESS) {
            return null
        }
        // A preference carried from another device, or a voice the engine has
        // since dropped, should cost a different voice rather than silence.
        return when (tts.setLanguage(Locale.SIMPLIFIED_CHINESE)) {
            TextToSpeech.LANG_MISSING_DATA ->
                "the Chinese voice data is not installed on this device, so there is nothing to speak with"
            TextToSpeech.LANG_NOT_SUPPORTED ->
                "this device's speech engine has no Chinese voice"
            else -> null
        }
    }

    /** Whether the engine is offering a Chinese voice. */
    private fun isChinese(voice: android.speech.tts.Voice): Boolean =
        voice.locale.language == "zh" || voice.locale.language == "cmn"

    /**
     * Whether a listed voice can actually be spoken with.
     *
     * Android's engine advertises voices whose data has not been downloaded and
     * marks them with [`KEY_FEATURE_NOT_INSTALLED`]; they are listed, they can be
     * selected, and they produce either a service error or nothing at all.
     */
    private fun isInstalled(voice: android.speech.tts.Voice): Boolean =
        !voice.features.orEmpty().contains(TextToSpeech.Engine.KEY_FEATURE_NOT_INSTALLED)

    /** The engine's own words for why it could not speak. */
    private fun reasonFor(errorCode: Int): String = when (errorCode) {
        TextToSpeech.ERROR_NETWORK ->
            "the chosen voice needs a network connection and there is none"
        TextToSpeech.ERROR_NETWORK_TIMEOUT ->
            "the chosen voice needs a network connection and it timed out"
        TextToSpeech.ERROR_NOT_INSTALLED_YET ->
            "the voice data for the chosen voice has not finished downloading"
        TextToSpeech.ERROR_OUTPUT ->
            "the speech engine could not open an audio output"
        TextToSpeech.ERROR_SERVICE ->
            "the system speech service stopped"
        TextToSpeech.ERROR_SYNTHESIS ->
            "the speech engine could not synthesise that text"
        TextToSpeech.ERROR_INVALID_REQUEST ->
            "the speech engine rejected the request"
        else -> "the speech engine failed (error $errorCode)"
    }

    /** `isLanguageAvailable`'s answer in words. */
    private fun languageStatus(status: Int): String = when (status) {
        TextToSpeech.LANG_AVAILABLE -> "available"
        TextToSpeech.LANG_COUNTRY_AVAILABLE -> "available (country)"
        TextToSpeech.LANG_COUNTRY_VAR_AVAILABLE -> "available (variant)"
        TextToSpeech.LANG_MISSING_DATA -> "missing data"
        TextToSpeech.LANG_NOT_SUPPORTED -> "not supported"
        else -> "unknown ($status)"
    }

    private companion object {
        /**
         * How long to wait for an utterance to report starting.
         *
         * Long enough that a slow engine is never wrongly called silent, short
         * enough that a learner is not left waiting on a button.
         */
        const val SPEAK_TIMEOUT_MS = 3000L

        /**
         * The rate capture runs at.
         *
         * `hanzi_core::tone::TARGET_SAMPLE_RATE` is the same number, so Android's
         * samples go straight to the analyser while the `cpal` path has to
         * resample to reach it.
         */
        const val RECORD_RATE = 16_000

        /**
         * Where the samples are put for Rust to read.
         *
         * In the app's own cache directory, and overwritten by every recording:
         * ten seconds of a learner's voice is not something to leave lying about
         * in an app whose claim is that it keeps nothing it does not have to.
         */
        const val RECORDING_FILE = "tone-recording.pcm"

        /**
         * A ceiling on one recording, in seconds.
         *
         * A guard against a stuck button, not the app's own limit: the app stops
         * at `MAX_RECORD_SECS` and enforces that in Rust when it reads the file.
         * This one is generous because being cut off here would be a mystery,
         * while being cut off there is reported as truncation.
         */
        const val MAX_RECORD_SECONDS = 30

        /** Where the sign-in is kept, and under what name. */
        const val SECRET_PREFERENCES = "hanzi-sync"
        const val SECRET_ENTRY = "dropbox"

        /**
         * Which mode the blob was written in, recorded beside it.
         *
         * The mode belongs to the keystore key and cannot be read back from it,
         * so this is what says whether reading the blob is silent or has to ask
         * somebody — and what [store] compares a new request against before it
         * decides whether the key has to be replaced.
         */
        const val SECRET_AUTH_ENTRY = "dropbox-auth"

        /**
         * Who may answer the sign-in's prompt, and who counts as askable.
         *
         * A weak biometric rather than a strong one, together with the device
         * credential, because a phone whose only lock is a PIN is still a phone
         * that can be asked — and refusing it would make the switch mean
         * "fingerprint or nothing" on a device that cannot have a fingerprint.
         * [canLock] asks about this same set, so the switch is never offered for
         * something the prompt would turn down.
         */
        val ASKABLE = BiometricManager.Authenticators.BIOMETRIC_WEAK or
            BiometricManager.Authenticators.DEVICE_CREDENTIAL

        /**
         * The keystore the AES key is generated in, and the alias it is filed
         * under. `AndroidKeyStore` is a provider name, spelled the same on every
         * device.
         */
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val SECRET_KEY_ALIAS = "hanzi-tutor-sync"

        /** AES-GCM, which authenticates as well as encrypts. */
        const val SECRET_TRANSFORMATION = "AES/GCM/NoPadding"

        /** GCM's authentication tag, in bits. 128 is the only size to use. */
        const val SECRET_TAG_BITS = 128
    }
}
