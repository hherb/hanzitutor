# Licence texts that ship with the app

Every file in this directory is **redistributed inside the application bundle**,
because the data the app embeds carries notice obligations. The catalogue in
`src-tauri/src/licences.rs` names each one, and a test fails if a file here is
not named there — an unlisted notice is a notice that does not ship.

`../LICENSE` (the app's own AGPL-3.0 text) and `../LICENSES.md` (the provenance
record) are bundled from the repository root rather than duplicated here, so
there is exactly one copy of each to keep current.

| File | What it covers | Licence |
| --- | --- | --- |
| `Arphic-Public-License.txt` | Stroke outlines and centrelines from Make Me a Hanzi, derived from the Arphic PL KaitiM GB and UKai fonts | Arphic Public License |
| `MakeMeAHanzi-COPYING.txt` | The upstream project's own notice, naming both of its sources | Arphic PL / LGPL-3.0-or-later |
| `LGPL-3.0.txt` | Etymology hints from Make Me a Hanzi `dictionary.txt`; also carries the Unicode/Unihan notice | LGPL-3.0-or-later |
| `MIT-hanziDB.txt` | Frequency rank, pinyin, meaning, radical and HSK level from `hanziDB.csv` | MIT |
| `MIT-complete-hsk-vocabulary.txt` | The HSK 3.0 word list, its levels, and the locally derived rank | MIT |
| `CC-CEDICT.txt` | The word readings and definitions, with the attribution and the changes this app made | CC BY-SA 4.0 |
| `CC-BY-SA-4.0.txt` | The full legal text of the share-alike licence above | CC BY-SA 4.0 |
| `OFL-1.1.txt` | Noto Sans SC, the CJK face bundled for the interface | SIL Open Font License 1.1 |
| `SQLite-Public-Domain.txt` | SQLite, the engine behind the study database — the first piece of third-party *code* in the binary | Public domain |
| `MIT-rusqlite.txt` | The `rusqlite` bindings, and `libsqlite3-sys` which vendors SQLite | MIT |
| `Apache-2.0.txt` | `cpal`, which opens the microphone for tone practice; `sherpa-onnx`, which runs the speech model; and the Android build's libraries — AndroidX, Material and Kotlin | Apache-2.0 |
| `MIT-onnxruntime.txt` | ONNX Runtime, which executes the speech model. It arrives statically linked inside the `sherpa-onnx` native archive rather than as a crate, so nothing in `Cargo.toml` points at it | MIT |
| `GPL-3.0.txt` | `espeak-ng`, which the **Android and iOS** builds redistribute inside the prebuilt `sherpa-onnx` library — compiled into it and never called, because this app recognises speech and does not synthesise it | GPL-3.0-or-later |
| `MIT-MeloTTS.txt` | MeloTTS, the model that synthesises the bundled pronunciation clips. Fetched for a build by `scripts/fetch-tts.sh` and pinned there by SHA-256; not compiled into the app and not downloaded by it | MIT |
| `NO7Z-hsk-sentences-audio.txt` | The HSK 1–2 sentences, their pinyin and their English translations, with the upstream attribution chain reproduced | CC BY-SA 4.0 |
| `HARUKICODER-hsk30-graded-readers.txt` | The reading passages with word-aligned pinyin and gloss | CC BY 4.0 |
| `CC-BY-4.0.txt` | The full legal text of the attribution licence the graded readers are under | CC BY 4.0 |

All of these are fetched verbatim by `scripts/fetch-data.sh` (`CC-CEDICT.txt`,
`SQLite-Public-Domain.txt`, `MIT-rusqlite.txt` and `MIT-onnxruntime.txt` are
written by hand, since they are attribution or provenance notices rather than
upstream files — the SQLite one quotes the blessing from the source that is
compiled in, and the other two are copied from those projects' `LICENSE` files).
They are committed rather than generated, so a clone has them without a network
round-trip. The speech **models** are not here, because neither is redistributed:
the recognition model is downloaded by the learner from its publisher, and the
synthesis model is fetched only by a developer building the phrase audio. See
[`../LICENSES.md`](../LICENSES.md) for both discussions — including why
`GPL-3.0.txt` says this app does not synthesise while `MIT-MeloTTS.txt` says it
does. The two are about different things: `espeak-ng` is a text-to-phoneme
front end for the *recognition* stack, and the sentence above is about what the
app does with it.
