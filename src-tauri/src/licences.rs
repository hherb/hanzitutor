//! The licence and attribution notices that ship with the app.
//!
//! Hanzi Tutor's own code is AGPL-3.0, but the **data** it embeds and the
//! third-party **code** compiled into it come from other people, and every one of
//! those licences requires its notice to travel with a redistribution. This module
//! is the catalogue of those notices, and it is the single source of truth for
//! three things at once:
//!
//! 1. what the Licences screen shows the reader,
//! 2. which files in `licences/` exist, and
//! 3. which of them the bundle copies into `Contents/Resources/`.
//!
//! The text is **compiled into the binary** with `include_str!`, not read from
//! disk at run time. That is deliberate: the roadmap's warning is that notices
//! are easy to lose in packaging and the loss only shows up in the built app,
//! and a compiled-in notice cannot be lost that way. The same files are *also*
//! copied into the bundle as plain text, so a redistributor can read them out of
//! the `.app` without launching it — see `tauri.conf.json`'s `bundle.resources`.
//!
//! `tests/licences.rs` holds the three-way correspondence together: it fails if
//! a file on disk is not catalogued, if a catalogued file is missing, if the
//! compiled-in text differs from the file, or if the bundle config does not copy
//! exactly this set. Adding a notice therefore cannot be half-done.

use serde::Serialize;

/// What the app is, and under what terms it is offered.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub identifier: &'static str,
    pub licence: &'static str,
    pub copyright: &'static str,
    pub repository: &'static str,
}

/// One notice, ready to display.
///
/// `file` is the repository-relative path the text came from, `bundle_path` is
/// where the bundle puts a plain-text copy of it beside the compiled-in one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenceNotice {
    /// Stable key, used as the DOM key and in tests.
    pub id: &'static str,
    /// The work the notice belongs to, as the screen should head it.
    pub title: &'static str,
    /// Which licence that work is under, in one line.
    pub licence: &'static str,
    /// Where it came from, so a reader can check it.
    pub source: &'static str,
    /// What this app takes from that work.
    pub covers: &'static str,
    /// The repository-relative path of the notice text.
    pub file: &'static str,
    /// Where the plain-text copy lands inside the application bundle.
    pub bundle_path: &'static str,
    /// The notice text itself, compiled into the binary.
    pub text: &'static str,
}

/// The app's own identity. The version is the workspace `Cargo.toml`'s, and a
/// test asserts it matches `tauri.conf.json` and `package.json` — three files
/// that are easy to let drift apart.
pub const APP: AppInfo = AppInfo {
    name: "Hanzi Tutor",
    version: env!("CARGO_PKG_VERSION"),
    identifier: "com.hanzitutor.app",
    licence: "GNU Affero General Public License v3.0 only (AGPL-3.0-only)",
    copyright: "Copyright © Horst Herb and contributors",
    repository: "https://github.com/hherb/hanzitutor",
};

/// The notices, in the order the Licences screen should show them: the app
/// first, then the data in the order it is described in `LICENSES.md`, then the
/// interface font.
pub const NOTICES: &[LicenceNotice] = &[
    LicenceNotice {
        id: "app",
        title: "Hanzi Tutor",
        licence: "AGPL-3.0-only",
        source: "https://github.com/hherb/hanzitutor",
        covers: "The application itself: the grading engine, the data pipeline, \
                 the Tauri shell and the interface.",
        file: "LICENSE",
        bundle_path: "licences/AGPL-3.0.txt",
        text: include_str!("../../LICENSE"),
    },
    LicenceNotice {
        id: "provenance",
        title: "Data provenance and licences — Hanzi Tutor's own notice",
        licence: "AGPL-3.0-only",
        source: "https://github.com/hherb/hanzitutor/blob/main/LICENSES.md",
        covers: "This project's record of where each piece of bundled data came \
                 from, and what its licence obliges a redistributor to do.",
        file: "LICENSES.md",
        bundle_path: "licences/PROVENANCE.md",
        text: include_str!("../../LICENSES.md"),
    },
    LicenceNotice {
        id: "arphic",
        title: "Stroke outlines and centrelines — Arphic Technology",
        licence: "Arphic Public License",
        source: "https://github.com/skishore/makemeahanzi (APL/english/ARPHICPL.TXT)",
        covers: "Every stroke outline and centreline the board draws and grades. \
                 They were extracted by Make Me a Hanzi from the Arphic PL \
                 KaitiM GB and Arphic PL UKai fonts. The outlines are not \
                 modified; they are re-encoded.",
        file: "licences/Arphic-Public-License.txt",
        bundle_path: "licences/Arphic-Public-License.txt",
        text: include_str!("../../licences/Arphic-Public-License.txt"),
    },
    LicenceNotice {
        id: "makemeahanzi",
        title: "Make Me a Hanzi — upstream notice",
        licence: "Arphic Public License and LGPL-3.0-or-later",
        source: "https://github.com/skishore/makemeahanzi (COPYING)",
        covers: "The upstream project's own statement of what it derived from \
                 what, and under which licence. Reproduced verbatim because it \
                 names the two sources above.",
        file: "licences/MakeMeAHanzi-COPYING.txt",
        bundle_path: "licences/MakeMeAHanzi-COPYING.txt",
        text: include_str!("../../licences/MakeMeAHanzi-COPYING.txt"),
    },
    LicenceNotice {
        id: "lgpl",
        title: "Etymology hints — Make Me a Hanzi dictionary.txt",
        licence: "LGPL-3.0-or-later (with the Unicode/Unihan notice)",
        source: "https://github.com/skishore/makemeahanzi (LGPL)",
        covers: "The etymology mnemonics shown with a character. The file also \
                 carries the notice for the Unihan database and CJKlib, from \
                 which the upstream dictionary was derived.",
        file: "licences/LGPL-3.0.txt",
        bundle_path: "licences/LGPL-3.0.txt",
        text: include_str!("../../licences/LGPL-3.0.txt"),
    },
    LicenceNotice {
        id: "hanzidb",
        title: "Frequency rank, pinyin, meaning, radical, HSK level — hanziDB.csv",
        licence: "MIT",
        source: "https://github.com/ruddfawcett/hanziDB.csv",
        covers: "Which characters are in the course and in what order, and each \
                 character's reading, gloss, radical, stroke count and HSK \
                 level. The list derives from Jun Da's Modern Chinese Character \
                 Frequency List.",
        file: "licences/MIT-hanziDB.txt",
        bundle_path: "licences/MIT-hanziDB.txt",
        text: include_str!("../../licences/MIT-hanziDB.txt"),
    },
    LicenceNotice {
        id: "hsk-vocabulary",
        title: "HSK 3.0 word list — complete-hsk-vocabulary",
        licence: "MIT",
        source: "https://github.com/drkameleon/complete-hsk-vocabulary",
        covers: "The 9,443 words, their characters and their HSK 3.0 levels. The \
                 rank the app orders them by is computed locally from the MIT \
                 character frequency list, not from this file.",
        file: "licences/MIT-complete-hsk-vocabulary.txt",
        bundle_path: "licences/MIT-complete-hsk-vocabulary.txt",
        text: include_str!("../../licences/MIT-complete-hsk-vocabulary.txt"),
    },
    LicenceNotice {
        id: "cc-cedict",
        title: "Word readings and definitions — CC-CEDICT",
        licence: "CC BY-SA 4.0",
        source: "https://cc-cedict.org/wiki/ via https://www.mdbg.net/chinese/dictionary?page=cc-cedict",
        covers: "The reading and the English definition of every listed word, \
                 including the polyphonic cases the isolated character cannot \
                 resolve. This is the one bundled dataset under a share-alike \
                 licence, and this notice records what was changed.",
        file: "licences/CC-CEDICT.txt",
        bundle_path: "licences/CC-CEDICT.txt",
        text: include_str!("../../licences/CC-CEDICT.txt"),
    },
    LicenceNotice {
        id: "cc-by-sa",
        title: "Creative Commons Attribution-ShareAlike 4.0 — legal text",
        licence: "CC BY-SA 4.0",
        source: "https://creativecommons.org/licenses/by-sa/4.0/legalcode",
        covers: "The full legal text of the licence above, which the licence \
                 itself requires to be distributed with the work.",
        file: "licences/CC-BY-SA-4.0.txt",
        bundle_path: "licences/CC-BY-SA-4.0.txt",
        text: include_str!("../../licences/CC-BY-SA-4.0.txt"),
    },
    LicenceNotice {
        id: "font",
        title: "Noto Sans SC — interface font",
        licence: "SIL Open Font License 1.1",
        source: "https://github.com/notofonts/noto-cjk, via https://fonts.google.com/noto/specimen/Noto+Sans+SC",
        covers: "The Chinese face used for every character the interface renders \
                 as text — lists, headings, buttons — so the app looks the same \
                 whatever fonts the machine has. The practice board does not use \
                 it: it draws the stored vector outlines.",
        file: "licences/OFL-1.1.txt",
        bundle_path: "licences/OFL-1.1.txt",
        text: include_str!("../../licences/OFL-1.1.txt"),
    },
    LicenceNotice {
        id: "sqlite",
        title: "SQLite — the study database",
        licence: "Public domain",
        source: "https://sqlite.org/ (SQLite 3.45.0, via the libsqlite3-sys crate)",
        covers: "The database the study data lives in: the practice schedule, the \
                 attempt log, the vocabulary list and the course cursor. It is \
                 compiled into the app from the vendored amalgamation, so no \
                 external process is started and nothing has to be installed.",
        file: "licences/SQLite-Public-Domain.txt",
        bundle_path: "licences/SQLite-Public-Domain.txt",
        text: include_str!("../../licences/SQLite-Public-Domain.txt"),
    },
    LicenceNotice {
        id: "rusqlite",
        title: "rusqlite — the SQLite bindings",
        licence: "MIT",
        source: "https://github.com/rusqlite/rusqlite",
        covers: "The Rust bindings the app opens and queries the study database \
                 with. The same licence and copyright line cover libsqlite3-sys, \
                 which vendors SQLite itself.",
        file: "licences/MIT-rusqlite.txt",
        bundle_path: "licences/MIT-rusqlite.txt",
        text: include_str!("../../licences/MIT-rusqlite.txt"),
    },
    LicenceNotice {
        id: "cpal",
        title: "cpal — cross-platform audio capture",
        licence: "Apache-2.0",
        source: "https://github.com/RustAudio/cpal",
        covers: "Opening the microphone for tone practice, and reading the audio \
                 buffers it delivers. This is the app's only input-audio \
                 dependency. On macOS and iOS it drives CoreAudio through \
                 coreaudio-rs (MIT/Apache-2.0); the Linux, Windows and Android \
                 backends in the same crate are not compiled into the macOS \
                 build. Holding the button opens the device and releasing it \
                 closes it again, so no audio is captured between utterances and \
                 none is ever written to disk.",
        file: "licences/Apache-2.0.txt",
        bundle_path: "licences/Apache-2.0.txt",
        text: include_str!("../../licences/Apache-2.0.txt"),
    },
    LicenceNotice {
        id: "sherpa-onnx",
        title: "sherpa-onnx — speech recognition engine",
        licence: "Apache-2.0",
        source: "https://github.com/k2-fsa/sherpa-onnx",
        covers: "Running the speech model, and the feature extraction, decoding \
                 and tokenisation around it. This is the app's largest native \
                 dependency: a prebuilt library archive, pinned by digest in \
                 scripts/fetch-sherpa.sh, is linked into the binary. It is only \
                 ever *used* when a learner has installed a model — which this \
                 app does not redistribute; see LICENSES.md for the model's own \
                 terms. The archive vendors several further libraries (ONNX \
                 Runtime, kaldi-native-fbank, kissfft, ssentencepiece and \
                 others); LICENSES.md records each one. One of them, espeak-ng, \
                 is under a copyleft licence and is redistributed inside the \
                 **mobile** builds whether or not it is ever called — see the \
                 notice below, which is the one that has to travel for it.",
        file: "licences/Apache-2.0.txt",
        bundle_path: "licences/Apache-2.0.txt",
        text: include_str!("../../licences/Apache-2.0.txt"),
    },
    LicenceNotice {
        id: "onnxruntime",
        title: "ONNX Runtime — model inference",
        licence: "MIT",
        source: "https://github.com/microsoft/onnxruntime",
        covers: "Executing the speech model. It is not a dependency of this \
                 project directly: it arrives inside the sherpa-onnx native \
                 archive, statically linked, which is why its notice has to \
                 travel with the app rather than with a crate.",
        file: "licences/MIT-onnxruntime.txt",
        bundle_path: "licences/MIT-onnxruntime.txt",
        text: include_str!("../../licences/MIT-onnxruntime.txt"),
    },
    LicenceNotice {
        id: "espeak-ng",
        title: "espeak-ng — the text-to-phoneme front end inside the speech engine",
        licence: "GPL-3.0-or-later",
        source: "https://github.com/espeak-ng/espeak-ng, vendored by \
                 https://github.com/k2-fsa/sherpa-onnx v1.13.8. Licence text from \
                 https://www.gnu.org/licenses/gpl-3.0.txt",
        covers: "Compiled, and never called — which is not the same as not \
                 shipped. espeak-ng is speech *synthesis*, and this app only ever \
                 recognises: no learner action reaches a synthesiser, and the \
                 model this app offers is a recognition model. That does keep it \
                 out of the macOS build, where the sherpa-onnx archive's \
                 components are linked individually and the linker simply drops \
                 what nothing references. It does **not** keep it out of the \
                 mobile builds: Android and iOS are handed an already-linked \
                 `libsherpa-onnx`, so espeak-ng is inside the library that ships \
                 whether or not a line of it ever runs. GPLv3 §6 conditions \
                 *conveying* the object code rather than using it, so the licence \
                 text and a route to the source travel with the app. \
                 Corresponding Source: espeak-ng is unmodified here, and the \
                 revision vendored by the sherpa-onnx release named above is \
                 published in the espeak-ng repository.",
        file: "licences/GPL-3.0.txt",
        bundle_path: "licences/GPL-3.0.txt",
        text: include_str!("../../licences/GPL-3.0.txt"),
    },
    LicenceNotice {
        id: "android-libraries",
        title: "The Android build's libraries",
        licence: "Apache-2.0",
        source: "AndroidX — https://developer.android.com/jetpack/androidx — and \
                 https://github.com/material-components/material-components-android",
        covers: "Everything the Android app is built against rather than linked \
                 into: the AndroidX libraries — biometric, which shows the \
                 fingerprint prompt; webkit, appcompat and activity, which the \
                 activity and the webview the interface runs in are built on; and \
                 lifecycle-process — together with com.google.android.material \
                 for the theme and the Kotlin standard library and coroutines. \
                 They are redistributed as compiled bytecode inside the APK and \
                 the AAB, so Apache-2.0 §4(a) applies to each: the recipient is \
                 given a copy of the licence, which is this file. None of them \
                 ships a `NOTICE` of its own, so §4(d) is not triggered. The set \
                 is the build's rather than this text's — it changes whenever \
                 build.gradle.kts does, and a library added there belongs in this \
                 sentence too.",
        file: "licences/Apache-2.0.txt",
        bundle_path: "licences/Apache-2.0.txt",
        text: include_str!("../../licences/Apache-2.0.txt"),
    },
    // ---- pronunciation audio ------------------------------------------------
    LicenceNotice {
        id: "melo-tts",
        title: "MeloTTS — speech synthesis for the pronunciation clips",
        licence: "MIT",
        source: "https://github.com/myshell-ai/MeloTTS, and the ONNX conversion \
                 at https://huggingface.co/csukuangfj/vits-melo-tts-zh_en",
        covers: "The model that synthesised every bundled pronunciation clip, and \
                 that the app will use to speak a phrase it has no clip for. It is \
                 not compiled into the app and not downloaded by it: \
                 scripts/fetch-tts.sh fetches it for a build, pinning each file by \
                 SHA-256. The clips it produces are a product of the model, which \
                 is why this notice travels with the app.",
        file: "licences/MIT-MeloTTS.txt",
        bundle_path: "licences/MIT-MeloTTS.txt",
        text: include_str!("../../licences/MIT-MeloTTS.txt"),
    },
    LicenceNotice {
        id: "phrases-no7z",
        title: "Graded sentences with audio — no7z/hsk-sentences-audio",
        licence: "CC BY-SA 4.0",
        source: "https://huggingface.co/datasets/no7z/hsk-sentences-audio",
        covers: "The text of the HSK-graded sentences the phrase screen \
                 practises: the simplified Chinese, its pinyin and its English \
                 translation. This is an adaptation of upstream's CC BY-SA 4.0 \
                 data and stays under that licence. **The audio is not \
                 upstream's**: this app regenerates it with MeloTTS and does not \
                 redistribute the CosyVoice2 clips the dataset ships, so \
                 CosyVoice2 is not an obligation this app carries.",
        file: "licences/NO7Z-hsk-sentences-audio.txt",
        bundle_path: "licences/NO7Z-hsk-sentences-audio.txt",
        text: include_str!("../../licences/NO7Z-hsk-sentences-audio.txt"),
    },
    LicenceNotice {
        id: "phrases-harukicoder",
        title: "HSK 3.0 graded readers — harukicoder/hsk30-graded-readers",
        licence: "CC BY 4.0",
        source: "https://huggingface.co/datasets/harukicoder/hsk30-graded-readers",
        covers: "The reading passages on the phrase screen, word-aligned with \
                 pinyin and gloss. Attribution only — this licence has no \
                 share-alike condition, unlike the material beside it. Upstream \
                 ships no audio; the clips are this app's own.",
        file: "licences/HARUKICODER-hsk30-graded-readers.txt",
        bundle_path: "licences/HARUKICODER-hsk30-graded-readers.txt",
        text: include_str!("../../licences/HARUKICODER-hsk30-graded-readers.txt"),
    },
    LicenceNotice {
        id: "cc-by",
        title: "Creative Commons Attribution 4.0 — legal text",
        licence: "CC BY 4.0",
        source: "https://creativecommons.org/licenses/by/4.0/legalcode",
        covers: "The full legal text of the licence the graded readers are under, \
                 which the licence itself requires to be distributed with the \
                 work. Its counterpart for the share-alike material above is \
                 catalogued separately.",
        file: "licences/CC-BY-4.0.txt",
        bundle_path: "licences/CC-BY-4.0.txt",
        text: include_str!("../../licences/CC-BY-4.0.txt"),
    },
];

/// The notices the Licences screen shows, in catalogue order.
pub fn notices() -> &'static [LicenceNotice] {
    NOTICES
}
