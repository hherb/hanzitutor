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

All of these are fetched verbatim by `scripts/fetch-data.sh` (`CC-CEDICT.txt` is
written by hand, since it is an attribution notice rather than an upstream
file). They are committed rather than generated, so a clone has them without a
network round-trip. See [`../LICENSES.md`](../LICENSES.md) for the full
provenance discussion.
