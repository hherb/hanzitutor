//! The licence and attribution notices that ship with Kana Tutor.
//!
//! Kana Tutor's own code is AGPL-3.0, but the **data** it embeds comes from
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
    name: "Kana Tutor",
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
        title: "Kana Tutor",
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
                 and what had to travel with it. The Japanese kana section covers \
                 this app.",
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
                 `graphicsJaKana.txt`. Compacted into `kana.bin.gz`, and modified: \
                 the drawing segments of a stroke that crosses itself are folded \
                 back into the single stroke that is taught.",
        file: "apps/nihongo-tutor/src-tauri/licences/AnimCJK-COPYING.txt",
        bundle_path: "licences/AnimCJK-COPYING.txt",
        text: include_str!("../licences/AnimCJK-COPYING.txt"),
    },
    LicenceNotice {
        id: "lgpl",
        title: "GNU Lesser General Public License",
        licence: "LGPL-3.0-or-later",
        source: "https://www.gnu.org/licenses/lgpl-3.0.html",
        covers: "The licence the kana SVG files and the kana `graphics` file are \
                 under, per AnimCJK's own statement. See the AnimCJK notice above \
                 for the ambiguity between this and the Arphic Public License.",
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
        covers: "The other licence AnimCJK's kana `graphics` file could be under — \
                 its statement assigns `graphics*` files to this one and kana SVGs \
                 to the LGPL. Both texts travel so that neither reading is left \
                 without its notice.",
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
