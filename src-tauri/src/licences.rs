//! The licence and attribution notices that ship with the app.
//!
//! Hanzi Tutor's own code is AGPL-3.0, but the **data** it embeds comes from
//! four upstream projects under four different licences, and every one of them
//! requires its notice to travel with a redistribution. This module is the
//! catalogue of those notices, and it is the single source of truth for three
//! things at once:
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
];

/// The notices the Licences screen shows, in catalogue order.
pub fn notices() -> &'static [LicenceNotice] {
    NOTICES
}
