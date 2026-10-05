//! The licence and attribution notices that ship with Nihongo Tutor.
//!
//! Nihongo Tutor's own code is AGPL-3.0, but the **data** it embeds comes from
//! AnimCJK, and that licence requires its notice to travel with a
//! redistribution. This module is the catalogue of those notices, and — exactly
//! as in the main app's catalogue — it is the single source of truth for three
//! things at once:
//!
//! 1. what a Licences screen shows the reader,
//! 2. which notice files exist, and
//! 3. which of them the bundle copies into `Contents/Resources/`.
//!
//! The text is **compiled into the binary** with `include_str!`, not read from
//! disk at run time, and the same files are *also* copied into the bundle as
//! plain text so a redistributor can read them out of the `.app` without
//! launching it. `tests/licences.rs` holds the three-way correspondence
//! together.
//!
//! # Why this app keeps its own directory rather than using `licences/`
//!
//! The repository's `licences/` directory is shared, and its own test
//! (`src-tauri/tests/licences.rs` in the main app) requires **every** file in it
//! to be catalogued by the Chinese app — so adding a kana notice there would make
//! the Chinese app ship and display a notice for data it does not contain. The
//! AnimCJK statement therefore lives beside the app that needs it, while the two
//! licence *texts* it points at are referenced out of the shared `licences/`
//! directory, because those genuinely are shared.

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

pub const APP: AppInfo = AppInfo {
    name: "Nihongo Tutor",
    version: env!("CARGO_PKG_VERSION"),
    identifier: "com.hanzitutor.kana",
    licence: "AGPL-3.0-only",
    copyright: "Copyright © Horst Herb and contributors",
    repository: "https://github.com/hherb/hanzitutor",
};

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

/// Every notice this app owes, in the order a reader should meet them.
pub const NOTICES: &[LicenceNotice] = &[
    LicenceNotice {
        id: "agpl",
        title: "Nihongo Tutor",
        licence: "GNU Affero General Public License, version 3",
        source: "https://github.com/hherb/hanzitutor",
        covers: "The app's own source code.",
        file: "LICENSE",
        bundle_path: "licences/AGPL-3.0.txt",
        text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../LICENSE")),
    },
    LicenceNotice {
        id: "provenance",
        title: "Data provenance and licences",
        licence: "AGPL-3.0-only (this document)",
        source: "https://github.com/hherb/hanzitutor/blob/main/LICENSES.md",
        covers: "The repository's record of where every bundled dataset came from \
                 and what had to travel with it. The Japanese sections cover this \
                 app.",
        file: "LICENSES.md",
        bundle_path: "licences/PROVENANCE.md",
        text: include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../LICENSES.md")),
    },
    LicenceNotice {
        id: "animcjk",
        title: "AnimCJK",
        licence: "Arphic Public License, or LGPL-3.0-or-later for the kana",
        source: "https://github.com/parsimonhi/animCJK",
        covers: "The kana stroke outlines and centre-lines, from \
                 `graphicsJaKana.txt`; the kanji outlines and centre-lines, from \
                 `graphicsJa.txt`; and the 214 radical head forms and the IDS \
                 decompositions, from `dictionaryJa.txt`. Compacted into \
                 `kana.bin.gz` and `kanji.bin.gz`, and modified: the drawing \
                 segments of a kana stroke that crosses itself are folded back \
                 into the single stroke that is taught, and the radical table's \
                 numbering is read from the file's order rather than from its \
                 glosses and then checked against KANJIDIC2's classical radical \
                 number.",
        file: "apps/nihongo-tutor/src-tauri/licences/AnimCJK-COPYING.txt",
        bundle_path: "licences/AnimCJK-COPYING.txt",
        text: include_str!("../licences/AnimCJK-COPYING.txt"),
    },
    LicenceNotice {
        id: "edrdg",
        title: "EDRDG — JMdict and KANJIDIC2",
        licence: "CC BY-SA 4.0",
        source: "https://www.edrdg.org/edrdg/licence.html",
        covers: "The readings and their okurigana, the English glosses, the current \
                 kyōiku grades, the frequency ranks and the stroke counts, from \
                 KANJIDIC2 and JMdict, compacted into `kanji.bin.gz` and \
                 `words.bin.gz`. EDRDG's terms require a software package that uses \
                 the files to acknowledge them on a screen reached from a menu, to \
                 ship the licence text, and to keep the data updated — this notice, \
                 the `ccbysa` entry beside it and the refresh procedure in \
                 LICENSES.md are those three things. The data is modified, and this \
                 notice records how.",
        file: "apps/nihongo-tutor/src-tauri/licences/EDRDG-JMdict-KANJIDIC2.txt",
        bundle_path: "licences/EDRDG-JMdict-KANJIDIC2.txt",
        text: include_str!("../licences/EDRDG-JMdict-KANJIDIC2.txt"),
    },
    LicenceNotice {
        id: "ccbysa",
        title: "Creative Commons Attribution-ShareAlike 4.0",
        licence: "CC BY-SA 4.0",
        source: "https://creativecommons.org/licenses/by-sa/4.0/legalcode",
        covers: "The full legal text of the licence EDRDG's JMdict and KANJIDIC2 \
                 are under, and of the derived artifacts that carry their readings, \
                 glosses, grades and ranks. EDRDG asks for copies of the licence \
                 files to travel with a software package, so the legal code does.",
        file: "licences/CC-BY-SA-4.0.txt",
        bundle_path: "licences/CC-BY-SA-4.0.txt",
        text: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../licences/CC-BY-SA-4.0.txt"
        )),
    },
    LicenceNotice {
        id: "jmdict-furigana",
        title: "JmdictFurigana",
        licence: "MIT",
        source: "https://github.com/Doublevil/JmdictFurigana",
        covers: "The alignment of a word's reading to the characters it belongs \
                 over — the furigana the vocabulary screen and every word card \
                 draw — compacted into `words.bin.gz`. Where the project publishes \
                 no alignment the word keeps its reading and is drawn without ruby, \
                 so no alignment is invented here.",
        file: "apps/nihongo-tutor/src-tauri/licences/PROVENANCE-JmdictFurigana.txt",
        bundle_path: "licences/PROVENANCE-JmdictFurigana.txt",
        text: include_str!("../licences/PROVENANCE-JmdictFurigana.txt"),
    },
    LicenceNotice {
        id: "jmdict-furigana-mit",
        title: "MIT License — JmdictFurigana",
        licence: "MIT",
        source: "https://github.com/Doublevil/JmdictFurigana/blob/master/LICENSE",
        covers: "The licence text the notice above is required to accompany, \
                 Copyright (c) 2025 Doublevil, reproduced verbatim.",
        file: "apps/nihongo-tutor/src-tauri/licences/MIT-JmdictFurigana.txt",
        bundle_path: "licences/MIT-JmdictFurigana.txt",
        text: include_str!("../licences/MIT-JmdictFurigana.txt"),
    },
    LicenceNotice {
        id: "unidic",
        title: "UniDic — the dictionary the passages were segmented with",
        licence: "BSD-3-Clause",
        source: "https://clrd.ninjal.ac.jp/unidic/ (via https://crates.io/crates/lindera-unidic)",
        covers: "The word boundaries and readings the reading screen draws over its \
                 passages, computed at build time through lindera. Only the \
                 segmentation is redistributed — no dictionary entry is — and the \
                 dictionary is not bundled with this app or fetched by it.",
        file: "apps/nihongo-tutor/src-tauri/licences/ANALYSER-UniDic-lindera.txt",
        bundle_path: "licences/ANALYSER-UniDic-lindera.txt",
        text: include_str!("../licences/ANALYSER-UniDic-lindera.txt"),
    },
    LicenceNotice {
        id: "tatoeba",
        title: "Tatoeba — the graded phrases",
        licence: "CC BY 2.0 FR",
        source: "https://tatoeba.org (terms: https://tatoeba.org/en/terms_of_use)",
        covers: "The text of every sentence on the Phrases screen and the English \
                 translation shown under it, imported from Tatoeba's per-language \
                 exports, filtered and compacted into `phrases.bin.gz`. The text of \
                 each sentence is unchanged; this app adds a word segmentation, a \
                 reading over every kanji, a level on its own ladder, and modifies \
                 nothing else — and it leaves out every sentence whose contributor \
                 the export does not name, because CC BY 2.0 FR requires the author \
                 to be named. The sentences are contributed under CC BY 2.0 FR, \
                 except the two the corpus lists as CC0 1.0; there is no \
                 NoDerivatives or NonCommercial variant of a Tatoeba sentence. The \
                 notice text this points at lists every contributor whose sentences \
                 ship, with the sentence ids taken from them.",
        file: "apps/nihongo-tutor/src-tauri/licences/TATOEBA-phrases.txt",
        bundle_path: "licences/TATOEBA-phrases.txt",
        text: include_str!("../licences/TATOEBA-phrases.txt"),
    },
    LicenceNotice {
        id: "lgpl",
        title: "GNU Lesser General Public License",
        licence: "LGPL-3.0-or-later",
        source: "https://www.gnu.org/licenses/lgpl-3.0.html",
        covers: "The licence the kana SVGs and the kana and kanji `graphics` files \
                 are under, per AnimCJK's own statement. See the AnimCJK notice \
                 above for the ambiguity between this and the Arphic Public \
                 License.",
        file: "licences/LGPL-3.0.txt",
        bundle_path: "licences/LGPL-3.0.txt",
        text: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../licences/LGPL-3.0.txt"
        )),
    },
    LicenceNotice {
        id: "arphic",
        title: "Arphic Public License",
        licence: "Arphic Public License",
        source: "https://ftp.gnu.org/non-gnu/chinese-fonts-truetype/LICENSE",
        covers: "The other licence AnimCJK's `graphics*` files could be under — its \
                 statement assigns `graphics*` files to this one and kana SVGs to \
                 the LGPL. Both texts travel so that neither reading is left without \
                 its notice.",
        file: "licences/Arphic-Public-License.txt",
        bundle_path: "licences/Arphic-Public-License.txt",
        text: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../licences/Arphic-Public-License.txt"
        )),
    },
];

/// The catalogue, for the app to show and for the tests to check.
pub fn notices() -> &'static [LicenceNotice] {
    NOTICES
}
