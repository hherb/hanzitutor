# Android licence and attribution obligations — research

**Status:** research only. No code in this repository was changed.
**Date:** 2026-09-21.
**Question asked:** the Android build depends on `androidx.biometric:biometric:1.1.0` (and six other declared Gradle dependencies), none of which appear in `LICENSES.md` or in the catalogue in `src-tauri/src/licences.rs`. What does a *published* Android artifact actually owe under those licences, and what should the project do about it?

The short answer is that the Android/Gradle set owes less than review feared, and that the interesting problem is somewhere else in the same artifact. Both are set out below.

Every claim about what a library contains is taken from the artifact that is actually built — the release APK, the release AAB, the pinned sherpa-onnx archives under `.sherpa-onnx/`, and the read-only Gradle cache under `~/.gradle/caches/modules-2/files-2.1/` — not from what the libraries say about themselves. Those inspections are reproducible with `unzip -l`, `strings` and `llvm-readelf`; no build, Gradle task, `pnpm` or `cargo` command was run.

---

## 1. Verdict

1. **The Gradle dependencies create no unmet licence obligation today.** Every one of them is Apache-2.0, or Apache-2.0-or-MIT (Tauri). Apache-2.0's only condition that bites when you redistribute an unmodified binary is §4(a): *"You must give any other recipients of the Work or Derivative Works a copy of this License."* The published AAB already contains `assets/licences/Apache-2.0.txt`, because Tauri copies `bundle.resources` into the Android asset set exactly as it copies them into `Contents/Resources/` on macOS. §4(d) — reproducing a `NOTICE` — is not triggered, because **none of these artifacts ships a NOTICE file**. §4(b) and §4(c) concern modified files and the *Source* form respectively, and neither applies.
2. **The documentation gap is real but it is the project's own standard, not a licence breach** — and it is not Android-specific. `LICENSES.md` catalogues five pieces of third-party *code* (SQLite, `rusqlite`, `cpal`, `sherpa-onnx`, ONNX Runtime) out of a Rust graph of hundreds of crates; Tauri itself, which is Apache-2.0 OR MIT and is most of the application, is not named. No npm package is named. Android is the same kind of omission, not a new kind.
3. **There is one genuine compliance problem in the Android (and iOS) artifact, and it is not a Gradle dependency.** The prebuilt `sherpa-onnx` shared libraries that ship inside the APK/AAB — and the iOS framework — contain **espeak-ng** (GPL-3.0-or-later). `LICENSES.md` states that espeak-ng "is verified not to be in the linked binary". That verification is true for the macOS build and false for both mobile builds. GPL-3.0-or-later is compatible with AGPL-3.0-only (GPLv3 §13 and AGPLv3 §13 expressly permit the combination), so this is not a licensing dead end — but it obliges the GPL-3.0 text and a Corresponding Source offer, and the project currently ships neither.

---

## 2. What the artifact actually contains

### 2.1 The Gradle set, as resolved

`src-tauri/gen/android/app/build.gradle.kts` declares seven compilation dependencies. The built release AAB and APK carry 47 `META-INF/<group>_<artifact>.version` markers, which is the resolved runtime set, versions included. Those 47 are:

```
androidx.activity:activity 1.10.1            androidx.lifecycle:lifecycle-livedata 2.10.0
androidx.activity:activity-ktx 1.10.1        androidx.lifecycle:lifecycle-livedata-core 2.10.0
androidx.annotation:annotation-experimental 1.4.1   ...-core-ktx 2.10.0
androidx.appcompat:appcompat 1.7.1           androidx.lifecycle:lifecycle-process 2.10.0
androidx.appcompat:appcompat-resources 1.7.1 androidx.lifecycle:lifecycle-runtime 2.10.0
androidx.arch.core:core-runtime (marker unset)      ...-runtime-ktx 2.10.0
androidx.browser:browser 1.8.0               androidx.lifecycle:lifecycle-viewmodel 2.10.0
androidx.cardview:cardview 1.0.0             ...-viewmodel-ktx 2.10.0
androidx.coordinatorlayout 1.1.0             ...-viewmodel-savedstate 2.10.0
androidx.core:core 1.13.1                    androidx.loader:loader 1.0.0
androidx.core:core-ktx 1.13.1                androidx.profileinstaller 1.4.0
androidx.core:core-viewtree 1.0.0            androidx.recyclerview 1.2.1
androidx.cursoradapter 1.0.0                 androidx.savedstate:savedstate 1.4.0
androidx.customview 1.1.0                    androidx.savedstate:savedstate-ktx 1.4.0
androidx.drawerlayout 1.1.1                  androidx.startup:startup-runtime 1.1.1
androidx.dynamicanimation 1.1.0              androidx.tracing:tracing 1.0.0
androidx.emoji2:emoji2 1.3.0                 androidx.transition:transition 1.5.0
androidx.emoji2:emoji2-views-helper 1.3.0    androidx.vectordrawable 1.1.0 / -animated 1.1.0
androidx.fragment:fragment 1.5.4             androidx.versionedparcelable 1.1.1
androidx.graphics:graphics-shapes 1.0.1      androidx.viewpager:viewpager 1.0.0
androidx.interpolator 1.0.0                  androidx.viewpager2:viewpager2 1.0.0
                                             androidx.webkit:webkit 1.14.0
com.google.android.material:material 1.13.0  kotlinx.coroutines:core / -android 1.9.0
```

Two observations worth keeping:

- **`androidx.biometric` is not in this list.** The marker set comes from a build predating commit `06e3e1b` ("Put the Android sign-in behind a fingerprint too", 2026-09-21), which is the commit that added the dependency. The premise of the review is correct — the dependency is new and nothing has been rebuilt-and-catalogued since — but the artifact I could inspect does not yet contain it.
- **The declared version is not the resolved version.** `build.gradle.kts` declares `material:1.12.0`; the built AAB resolves `material:1.13.0` (marker content, `base/root/META-INF/com.google.android.material_material.version`). Anything hand-copied from `build.gradle.kts` will therefore be slightly wrong, which is an argument for generating the list from a build rather than transcribing it — see §7.2.

### 2.2 What is redistributed as native code

The release APK carries five `.so` files per ABI: `libhanzi_tutor_lib.so` (the Rust application), `libonnxruntime.so`, `libsherpa-onnx-c-api.so`, `libsherpa-onnx-cxx-api.so` and `libsherpa-onnx-jni.so`. `libhanzi_tutor_lib.so` declares `DT_NEEDED: libsherpa-onnx-c-api.so`, so the sherpa shared object is loaded as part of the app.

### 2.3 What the artifact carries as licence material

`unzip -l` on the release APK and AAB lists, under `assets/`:

```
assets/licences/AGPL-3.0.txt              assets/licences/MIT-hanziDB.txt
assets/licences/Apache-2.0.txt            assets/licences/MIT-onnxruntime.txt
assets/licences/Arphic-Public-License.txt assets/licences/MIT-rusqlite.txt
assets/licences/CC-BY-SA-4.0.txt          assets/licences/MakeMeAHanzi-COPYING.txt
assets/licences/CC-CEDICT.txt             assets/licences/OFL-1.1.txt
assets/licences/LGPL-3.0.txt              assets/licences/PROVENANCE.md
assets/licences/MIT-complete-hsk-vocabulary.txt  assets/licences/SQLite-Public-Domain.txt
```

That is all fourteen catalogued files, byte-identical in size to their repository counterparts. **The desktop `bundle.resources` map is the mechanism that puts them in the Android artifact** (Tauri copies resources into the mobile asset set), so a notice added to the catalogue and to `tauri.conf.json` reaches every platform through one edit. That fact drives the recommendation in §7.

Under `META-INF/`, the APK carries:

```
META-INF/FastDoubleParser-LICENSE     (Apache-2.0, 11358 bytes)
META-INF/FastDoubleParser-NOTICE      (1704 bytes)
META-INF/bigint-LICENSE               (BSD-2-Clause, 1274 bytes)
META-INF/androidx/annotation/annotation/LICENSE.txt        (Apache-2.0, 10175 bytes)
META-INF/androidx/lifecycle/lifecycle-common/LICENSE.txt   (Apache-2.0, 10175 bytes)
```

and **no other `NOTICE` of any kind**. The 10175-byte file is the same Apache-2.0 text in every AndroidX artifact that has one (identical SHA-256, `809fa1ed…`).

Which artifacts' embedded copies survive into the APK is informative and inconsistent: the AndroidX AARs for `webkit:1.14.0`, `activity:1.10.1` and `lifecycle-process:2.10.0` each *do* contain `META-INF/androidx/<group>/<artifact>/LICENSE.txt`, but that file is **absent from both the debug and release APK**; only the two plain-JAR artifacts (`androidx.annotation:annotation`, `androidx.lifecycle:lifecycle-common`) keep theirs. The most likely mechanism is that AGP unpacks an AAR and uses only its recognised parts, so an AAR's `META-INF/` is dropped while a JAR's is merged — but I did not find AGP documentation stating the rule, so treat the mechanism as inference and the observation as the fact (verified in both the debug APK, where R8 is off, and the release APK).

---

## 3. The obligation, per dependency or coherent group

Apache-2.0 §4, as fetched from `https://www.apache.org/licenses/LICENSE-2.0.txt`, applies the following conditions. Quoting the clause that decides most of this:

> **(d)** If the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file…

> **(a)** You must give any other recipients of the Work or Derivative Works a copy of this License

So: a library that ships **no** NOTICE triggers (a) only. A library that ships one triggers (a) **and** (d).

| Group | Licence | What it requires on this artifact | Ships a NOTICE? |
| --- | --- | --- | --- |
| **AndroidX** — `biometric:1.1.0`, `webkit:1.14.0`, `appcompat:1.7.1`, `activity:activity-ktx:1.10.1`, `lifecycle:lifecycle-process:2.10.0`, and the ~40 transitives (`core`, `fragment`, `annotation`, `emoji2`, `savedstate`, `tracing`, …) | Apache-2.0 | §4(a): ship the licence text. Already done (`assets/licences/Apache-2.0.txt`). §4(d) not triggered. §4(b)/(c) do not apply — nothing was modified, and only Object form is distributed. | **No.** No `NOTICE` in any AAR in the Gradle cache; and the AndroidX repository root has `LICENSE.txt` but its `NOTICE` is a 404. |
| **`com.google.android.material:material`** | Apache-2.0 | §4(a) only. | **No.** Repo `LICENSE` is Apache-2.0; repo `NOTICE` is a 404. Its POM declares Apache-2.0. |
| **Kotlin runtime** — `kotlin-stdlib`, `kotlinx-coroutines-core`/`-android`, plus `org.jetbrains:annotations` | Apache-2.0 | §4(a). `kotlin-stdlib`'s jar contains no `LICENSE`/`NOTICE`; its POM declares Apache-2.0. | **No** for the stdlib and coroutines. JetBrains' `license/NOTICE.txt` is explicitly *"for the Kotlin Compiler distribution"*, which this app does not redistribute. |
| **Kotlin Gradle plugin** (`org.jetbrains.kotlin.android`), AGP, R8 | Apache-2.0 | **Nothing.** Build tooling, not conveyed in the artifact. | The plugin jar's `META-INF/NOTICE.txt` is for Apache Commons Compress, which it embeds — irrelevant to a redistributor of the app. |
| **Tauri Android library** (`:tauri-android`, from the `tauri` 2.11.5 crate's `mobile/android`) and `tauri-plugin-dialog` / `tauri-plugin-opener` | Apache-2.0 OR MIT | §4(a), or the MIT notice; the crate carries `LICENSE_APACHE-2.0` and `LICENSE_MIT`. | **No** (repo `NOTICE` is a 404). |
| **Jackson** — `jackson-databind` + `jackson-core`, pulled by Tauri's Android library | Apache-2.0 | §4(a). | **Yes — and this is the only NOTICE that reaches the APK.** `jackson-core` bundles FastDoubleParser, and ships `FastDoubleParser-NOTICE` + `FastDoubleParser-LICENSE` (Apache-2.0) + `bigint-LICENSE` (BSD-2-Clause). All three are already inside the published APK/AAB, so §4(a) and §4(d) for them are satisfied **by the artifact itself**. |
| **Test-only** — `junit:junit:4.13.2`, `androidx.test.ext:junit`, `espresso-core` | JUnit 4 is **EPL-1.0**; the AndroidX test artifacts are Apache-2.0 | **Nothing.** `testImplementation`/`androidTestImplementation` are not in the shipped APK — verified: zero `junit`/`espresso` entries in the release APK. | n/a |

**The transitive set is knowable, and exhausting it is not what people actually do.** Mechanically it is trivial: the 47 `META-INF/*.version` markers in the built APK *are* the resolved set at exact versions, and the AndroidX POMs do declare `<licenses>` (spot-checked `core`, `appcompat`, `activity-ktx`, `fragment`, `annotation-jvm`, `emoji2`, plus `material` and `kotlin-stdlib`: all Apache-2.0). That is precisely what Google's own tooling consumes: [`oss-licenses-plugin` with `play-services-oss-licenses`](https://developers.google.com/android/guides/opensource) generates a per-library list from the POMs and renders it in an activity, and the community equivalent is [AboutLibraries](https://github.com/mikepenz/AboutLibraries), which has a *strict mode* that fails the build on a licence outside an allow-list. So a per-library list **is** standard output — but it is generated, and it is a list of *names* pointing at a handful of shared licence texts, not forty-seven licence reproductions. Because every one of the 47 is Apache-2.0 and none ships a NOTICE, the legal content of that list is: *one* copy of Apache-2.0 plus a statement of which libraries it covers. The per-library enumeration is documentation quality, not a licence condition.

Two honest caveats about generated lists: AboutLibraries' own README records that it "could lead to dependencies which are only used during compilation … to be listed", that it "might also fail to identify licenses if the dependencies do not define it properly in their pom.xml", and that "native sub dependencies can *not* be resolved automatically" — which is exactly the class of thing §6 is about.

---

## 4. What the project already does, and whether Android is genuinely different

`LICENSES.md` covers **data and a hand-picked set of embedded code**, not the dependency graph. Concretely:

| Platform | How notices travel | What is catalogued |
| --- | --- | --- |
| macOS / Windows / Linux | compiled in with `include_str!` **and** copied to `Contents/Resources/licences/` by `bundle.resources` | 15 `LicenceNotice` entries over 14 distinct files: the app, the provenance record, six datasets, the font, SQLite + `rusqlite`, `cpal`, `sherpa-onnx`, ONNX Runtime |
| Android | the same 14 files land in `assets/licences/` (verified in APK and AAB); the Licences screen works from the compiled-in text | identical — nothing Android-specific |
| iOS | the same files land in the app bundle | identical |

So the **data** notices are not an Android gap; Android receives them correctly, unexamined or not. The gap review noticed is about the **code** set, and there the project's existing standard is selective on every platform:

- Not catalogued on any platform: Tauri itself (Apache-2.0 OR MIT — the single largest compiled-in work after the app), `tauri-plugin-dialog`, `tauri-plugin-opener`, `serde`, `serde_json`, `ureq`, `tar`, `bzip2`, `sha2`, `objc2`, `objc2-foundation`, `objc2-avf-audio`, `dispatch2`, `security-framework`, and every npm dependency (`@tauri-apps/api`, `svelte` — the latter built into the shipped JS).
- Not catalogued on Android in particular: `ndk` 0.9, `jni` 0.21/0.22, `ndk-sys`, `ndk-context`, `jni-sys`, `cesu8`, `combine`, `simd_cesu8` and the rest of the mobile-only Rust subtree in `Cargo.lock`. All permissive (Apache-2.0/MIT), none shipping a NOTICE.

The honest reading is therefore: **the Android/Gradle case is merely unexamined, not different in kind.** The project's actual standard is "catalogue the third-party works whose notices are judged material", and the Android libraries are as material as Tauri — which was also left out. One consequence is a documentation claim that is currently false: `README.md` says the About screen "shows the full text of every third-party licence its data, font and compiled-in code are under", and `LICENSES.md` says "Some of the entries below are **code rather than data** … Each is recorded here rather than left implicit." Neither is true of Tauri or the Android set. That is worth fixing in the prose regardless of what is added to the catalogue.

`docs/privacy-policy.md` and `store/listing.md` were read as instructed. Neither makes a licensing claim: the policy's "Third parties: There are none" is about data recipients, and the listing copy never mentions attribution. Nothing there conflicts.

---

## 5. Compatibility with the app's own AGPL-3.0-only licence

- **Apache-2.0 → GPLv3 is compatible, and therefore → AGPLv3.** The FSF's licence list states Apache-2.0 "is a free software license, compatible with version 3 of the GNU GPL" and, separately, that it is incompatible with GPLv2 only. `LICENSES.md` already relies on this for `cpal` and `sherpa-onnx` ("Apache-2.0 combines into AGPL-3.0"). The whole AndroidX/Material/Kotlin/Tauri set is the same case. **No conflict.**
- **MIT, BSD-2-Clause, Apache-2.0-or-MIT** (ONNX Runtime, `bigint`, Tauri): permissive, no conditions that reach the AGPL.
- **EPL-1.0 (JUnit 4)** is the one licence in the Gradle file that is not permissive in the same way, and it is the one that does not ship. It is confined to `testImplementation`; the release APK contains no JUnit. It would become a real problem only if a test-only artifact were ever packaged.
- **Nothing in the shipped Android set is copyleft or carries a "commons clause".** There is no restriction more aggressive than Apache-2.0/MIT among the Gradle dependencies — which is worth saying plainly, because that was the thing most likely to be a genuine problem and it is not.
- **The one copyleft component is espeak-ng** (GPL-3.0-or-later), and it is already shipped on Android and iOS — see §6. GPLv3 §13 (quoted from espeak-ng's own `COPYING`) says: *"you have permission to link or combine any covered work with a work licensed under version 3 of the GNU Affero General Public License into a single combined work, and to convey the resulting work."* AGPLv3 §13 contains the mirror provision. So the combination is lawful and the app's licence need not change — but the GPL's own conditions (§4: give recipients the licence; §6: convey the Corresponding Source) attach to the espeak-ng part.

---

## 6. The finding that has nothing to do with Gradle: espeak-ng ships on Android and iOS

`LICENSES.md` says of the `sherpa-onnx` archive:

> `espeak-ng` is the one component under a copyleft licence (**GPL-3.0-or-later**, compatible with AGPL-3.0 either way) and it is *speech synthesis only*: **it is verified not to be in the linked binary** — `nm` finds none of its symbols and `strings` finds none of its data.

That verification holds for macOS and fails for both mobile targets.

**Method.** The pinned archives under `.sherpa-onnx/` are the inputs the build actually uses. I identified five strings that are *unique to `libespeak-ng.a`* among the macOS archives — i.e. present in `libespeak-ng.a`, absent from `libsherpa-onnx-core.a` and every other member:

```
Wrong version of espeak-ng-data
The requested functionality has not been built into espeak-ng
The espeak-ng library has not been initialized
/usr/share/espeak-ng-data
/tmp/espeakXXXXXX
```

All five are present in the Android `libsherpa-onnx-c-api.so` for **all four ABIs** (`arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64`) in `sherpa-onnx-v1.13.8-android.tar.bz2`, and in `libsherpa-onnx-jni.so`. The APK's copies are the same files. The device slice of `sherpa-onnx-v1.13.8-ios-shared-onnxruntime-static.xcframework` also contains them. The macOS **linked** binary contains none of them — it contains only the four `espeak-ng` mentions that are the compiled-in `LICENSES.md` text itself. That is consistent with the architecture: on macOS the distribution ships `libespeak-ng.a` separately and the linker drops the unreferenced objects; on Android and iOS sherpa-onnx distributes an **already-linked** `.so`/framework that includes it.

**Why the "we never call it" argument does not dispose of it.** The TTS path in this app is never invoked, and the `espeak-ng-data` asset is not shipped, so the code is probably non-functional here. But GPLv3 §6 conditions *conveying the object code*, not calling it. The APK/AAB is conveyed, and the code is in it.

**What it therefore obliges**, and what is currently missing:

1. A copy of the **GPL-3.0** text. `licences/` contains `LGPL-3.0.txt` and the app's own AGPL-3.0 `LICENSE`, but no GPL-3.0. (The AGPL text is GPLv3 plus §13; it is not a substitute.)
2. **Corresponding Source** for espeak-ng. The usual discharge for an unmodified third-party library is §6(d): convey the object code from a designated place and "offer equivalent access to the Corresponding Source in the same way through the same place at no further charge" — in practice, a clear pointer, next to the notice, to the exact upstream revision. The project has no such statement. The exact espeak-ng revision vendored by sherpa-onnx 1.13.8 could not be established from the artifacts (§8), so the pointer should name the sherpa-onnx release and its vendored espeak-ng source.
3. The consequential point: the app can never be relicensed to anything more permissive while espeak-ng is in the mobile binaries, and AGPLv3 §13's network clause is live for the combination (it already is, for the app's own code).

**A methodological warning worth recording.** The first pass at this looked like a false alarm because `strings … | grep -i espeak` matches `…CreateSp`**`eSpeak`**`er…` in `SherpaOnnxCreateSpeakerEmbeddingExtractor`. Any future audit must use `espeak-`/`espeak_`/`espeak-ng` and compare against the archive members, not grep loosely for "espeak". Without the uniqueness test against `libespeak-ng.a`, the macOS result would also have looked dirty.

---

## 7. Recommendation

Ordered by value, not effort.

### 7.1 Fix the espeak-ng gap (the one that is a real obligation)

1. Add `licences/GPL-3.0.txt`, the GPLv3 text as shipped by espeak-ng (`https://github.com/espeak-ng/espeak-ng/blob/master/COPYING`).
2. Add a `LicenceNotice` for it. Concretely, in `src-tauri/src/licences.rs`'s `NOTICES`:

```rust
LicenceNotice {
    id: "espeak-ng",
    title: "espeak-ng — text-to-phoneme front end inside the speech engine",
    licence: "GPL-3.0-or-later",
    source: "https://github.com/espeak-ng/espeak-ng, vendored by https://github.com/k2-fsa/sherpa-onnx v1.13.8",
    covers: "Compiled, not called: the prebuilt sherpa-onnx archives for Android and iOS \
             contain espeak-ng, so it is redistributed inside the mobile APK/AAB and the iOS \
             framework even though this app only recognises speech and never synthesises it. \
             The macOS build links the same archive statically and the linker drops it. \
             Corresponding Source: the unmodified espeak-ng revision vendored by the \
             sherpa-onnx v1.13.8 release named above.",
    file: "licences/GPL-3.0.txt",
    bundle_path: "licences/GPL-3.0.txt",
    text: include_str!("../../licences/GPL-3.0.txt"),
},
```

3. Add the mapping to `src-tauri/tauri.conf.json`'s `bundle.resources` (this is also what carries it to Android and iOS):

```json
"../licences/GPL-3.0.txt": "licences/GPL-3.0.txt",
```

4. Correct the two places that currently assert the opposite: the `sherpa-onnx` entry's `covers` text in `licences.rs`, and the bullet in `LICENSES.md` under "sherpa-onnx and ONNX Runtime". Say that the check was macOS-specific and that mobile is different.
5. Add the row to `licences/README.md`'s table, and — if the project ever assembles one — the `Before you distribute` checklist.

### 7.2 Document the Android/Gradle set (cheap, and it is what review actually asked for)

This needs **no new licence text**, because it reuses `licences/Apache-2.0.txt`. The existing `cpal` and `sherpa-onnx` entries already establish the pattern of several notices pointing at one file, and the tests tolerate it (§7.3). A single entry covering the group is the honest granularity; a per-library list is optional detail:

```rust
LicenceNotice {
    id: "android-libraries",
    title: "Android runtime libraries — AndroidX, Material Components and Kotlin",
    licence: "Apache-2.0",
    source: "https://maven.google.com/web/index.html (androidx.*, com.google.android.material) and https://kotlinlang.org/",
    covers: "The Android half of the application: the fingerprint prompt \
             (androidx.biometric), the WebView bridge (androidx.webkit), the app shell \
             (appcompat, activity, core, fragment, lifecycle), the Material widgets, the \
             Kotlin standard library and coroutines, and the ~40 transitive AndroidX \
             artifacts they resolve to. All of them are Apache-2.0, none of them ships a \
             NOTICE file, and they are redistributed here unmodified and in binary form, \
             so Apache-2.0 section 4(a) is the clause that applies.",
    file: "licences/Apache-2.0.txt",
    bundle_path: "licences/Apache-2.0.txt",
    text: include_str!("../../licences/Apache-2.0.txt"),
},
```

Optionally add `licences/Android-libraries.txt` listing the resolved coordinates and versions — this is the artifact a reader actually wants, and because it is catalogued it will be copied into the bundle alongside the rest. If it is added it must be catalogued and mapped like any other file. Note the staleness hazard demonstrated in §2.1: `material` is declared `1.12.0` and resolves `1.13.0`, so generate the list from a build (`META-INF/*.version` in the APK, or `AboutLibraries`/`oss-licenses-plugin`) rather than transcribing `build.gradle.kts`.

Then add prose: a subsection in `LICENSES.md` next to "Some of the entries below are code rather than data", a row in `licences/README.md`, and a correction to the `README.md` claim that the screen shows "every third-party licence … compiled-in code" — either make that true (including Tauri) or soften it to "the third-party works catalogued here".

### 7.3 What the `licences.rs` tests would then require

The four tests in `src-tauri/tests/licences.rs` pin a notice in three places and fail if any two disagree. Adding these entries has these exact consequences:

1. **`every_notice_is_on_disk_and_is_the_text_that_was_compiled_in`** — `licences/GPL-3.0.txt` must exist, be non-empty, and match the `include_str!` text byte-for-byte after trailing-whitespace trim. The Apache-2.0 reuse adds nothing here (the file already exists and already matches).
2. **`every_licence_text_in_the_directory_is_catalogued`** — every non-`README.md` file in `licences/` must appear as some notice's `file`. So a new `GPL-3.0.txt` **cannot be added without** a catalogue entry; the check builds a `BTreeSet` of catalogued `licences/`-prefixed paths, so a second entry pointing at `Apache-2.0.txt` is harmless (this is how `cpal` and `sherpa-onnx` already coexist).
3. **`the_bundle_copies_exactly_the_catalogued_notices`** — `tauri.conf.json`'s `bundle.resources` must equal exactly `{"../" + notice.file: notice.bundle_path}` over the catalogue. Because the expected side is a map keyed by source path, the Apache-2.0 reuse adds no mapping; `GPL-3.0.txt` **must** get one, and adding a file to `licences/` without a mapping fails the test. This same map is what puts the file into the Android AAB's `assets/licences/`, so one edit covers every platform.
4. **`the_required_licences_are_present_and_are_the_real_texts`** — add the new markers so a stub cannot pass, e.g. `("espeak-ng", "GNU GENERAL PUBLIC LICENSE")` to the `required` array, plus the `text.len() > 500` assertion that already applies to full legal codes; and add `"espeak-ng"` to the `for id in ["arphic", "lgpl", "cc-by-sa", "font"]` list of notices whose absence would be a breach. A marker for the Android entry is optional — its text is the shared Apache-2.0 file already checked under `cpal`/`sherpa-onnx`.

Also note the module doc comment still says the notices come from "four upstream projects under four different licences"; that has been stale for some time and could be corrected in the same pass.

### 7.4 Cost

Roughly: one new licence text file (GPLv3, ~35 KB), one optional list file, two `LicenceNotice` entries (~30 lines), one or two `tauri.conf.json` lines, four prose edits (`LICENSES.md`, `licences/README.md`, `README.md`, the `licences.rs` doc comment), and three or four lines of test. There is no build-system change, no new dependency, and no per-platform work — the resource map already fans the notices out to macOS, Windows, Linux, iOS and Android. The work is an hour or two, most of it prose. Re-running `cargo test` is what proves it, and I deliberately did not run it because another agent is building the Android app from this checkout.

---

## 8. What I could not establish

- **The exact espeak-ng revision vendored by sherpa-onnx 1.13.8.** The shipped `.so` files are stripped and contain no espeak-ng version string; the archive carries no source. A Corresponding Source pointer therefore has to name the sherpa-onnx release rather than a pinned espeak-ng commit.
- **How espeak-ng got into the mobile objects.** `llvm-readelf -d` shows no `libespeak-ng.so` in `DT_NEEDED` and `llvm-nm` finds no espeak symbols because the file is stripped, so "espeak-ng's code is inside the shipped object" is established by the string-uniqueness test, but whether sherpa-onnx links a prebuilt `libespeak-ng.a` into it or compiles the sources in is not. Either way the object is conveyed, so the obligation is the same.
- **Which dependency forces `material` from the declared 1.12.0 to the resolved 1.13.0.** The APK was being rewritten by another agent's build while I read it (its mtime moved from 2026-09-20 23:33 to 2026-09-21 09:58), so I did not try to capture a full resolution graph, and I did not run Gradle to get one.
- **Whether `androidx.biometric:biometric:1.1.0` has any dependency that is *not* Apache-2.0.** Its own POM declares Apache-2.0 and it is a 2021 AndroidX release, so this is very unlikely, but no built artifact containing it existed to enumerate transitives from. It should be re-checked after the next Android build.
- **The AGP rule that drops AAR `META-INF` while keeping JAR `META-INF`.** The behaviour is observed in both the debug and release APK; I found no primary documentation stating it.
- **The licence of each of the 47 resolved artifacts individually.** I verified the POM `<licenses>` for `androidx.biometric`, `webkit`, `core`, `appcompat`, `activity-ktx`, `fragment`, `annotation-jvm` and `emoji2`, plus `material` and `kotlin-stdlib` — all Apache-2.0 — and scanned every cached AAR for `LICENSE`/`NOTICE`/`COPYING`. AndroidX is Apache-2.0 by project policy and the sample is representative, but the remaining artifacts were not checked one by one.
- **Whether Google Play imposes any attribution requirement of its own.** Nothing found suggests it does: Google's guidance places the responsibility for displaying OSS notices on the developer and supplies tooling, and Play's own Data safety disclosure is about data, not licences. I did not exhaustively read Play policy.
- **Whether an `espeak-ng-data` asset is shipped.** It is not in the APK, so the TTS code path is almost certainly non-functional in this app. That does not change the GPL analysis — conveying the binary is what matters — but it does mean the "speech synthesis only" framing describes a capability that is present but unusable here.

---

## 9. Sources

**Primary — the licence texts themselves**

- Apache License 2.0, full text (esp. §4(a) and §4(d)): <https://www.apache.org/licenses/LICENSE-2.0.txt>
- FSF, *Various Licenses and Comments about Them* — Apache-2.0 is "compatible with version 3 of the GNU GPL", incompatible with GPLv2; AGPLv3 combines with GPLv3: <https://www.gnu.org/licenses/license-list.html>
- espeak-ng `COPYING` (GPLv3, including §13 on combining with the AGPL): <https://github.com/espeak-ng/espeak-ng/blob/master/COPYING>
- AndroidX repository `LICENSE.txt` (Apache-2.0; `NOTICE` at the same root returns 404): <https://github.com/androidx/androidx/blob/androidx-main/LICENSE.txt>
- Material Components for Android `LICENSE` (Apache-2.0; no `NOTICE`): <https://github.com/material-components/material-components-android/blob/master/LICENSE>
- Kotlin `license/NOTICE.txt`, which is explicitly for the *compiler* distribution: <https://github.com/JetBrains/kotlin/blob/master/license/NOTICE.txt>
- `kotlin-stdlib` 2.2.10 POM, declaring Apache-2.0: <https://repo1.maven.org/maven2/org/jetbrains/kotlin/kotlin-stdlib/2.2.10/kotlin-stdlib-2.2.10.pom>
- `androidx.biometric:biometric:1.1.0` POM, declaring Apache-2.0: <https://dl.google.com/dl/android/maven2/androidx/biometric/biometric/1.1.0/biometric-1.1.0.pom>
- `androidx.webkit:webkit:1.14.0` POM, declaring Apache-2.0: <https://dl.google.com/dl/android/maven2/androidx/webkit/webkit/1.14.0/webkit-1.14.0.pom>

**Primary — Google's guidance and the projects' own repositories**

- Google, *Include open source notices* — "you as a developer are responsible for appropriately displaying the notices for the open source libraries that your app uses"; documents `oss-licenses-plugin` and `play-services-oss-licenses`: <https://developers.google.com/android/guides/opensource>
- `google/play-services-plugins` (the OSS-licences plugin source): <https://github.com/google/play-services-plugins>
- `k2-fsa/sherpa-onnx` (the pinned 1.13.8 release whose archives are inspected in §6): <https://github.com/k2-fsa/sherpa-onnx>

**Secondary — how real projects handle it**

- AboutLibraries — the de-facto community tool that collects every Gradle dependency and licence at build time, with a strict mode; its README also documents the failure modes of doing this automatically: <https://github.com/mikepenz/AboutLibraries>
- espeak-ng discussion *"Is 'Espeak-ng' License contaminating?"*: <https://github.com/orgs/espeak-ng/discussions/1868>
- espeak-ng issue *"Licensing question regarding linking"*: <https://github.com/espeak-ng/espeak-ng/issues/908>

**Artifacts examined in this checkout (not URLs)**

- `src-tauri/gen/android/app/build/outputs/apk/universal/{release,debug}/app-universal-*.apk`
- `src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab`
- `.sherpa-onnx/sherpa-onnx-v1.13.8-android/jniLibs/*/libsherpa-onnx-*.so`
- `.sherpa-onnx/sherpa-onnx-v1.13.8-ios-shared-onnxruntime-static.xcframework/SherpaOnnxC.xcframework/*/SherpaOnnxC.framework/SherpaOnnxC`
- `.sherpa-onnx/sherpa-onnx-v1.13.8-osx-arm64-static-lib/lib/*.a` and `.cargo-target/release/bundle/macos/Hanzi Tutor.app/Contents/MacOS/hanzi-tutor`
- `~/.gradle/caches/modules-2/files-2.1/` (all cached AARs and JARs scanned for `LICENSE`/`NOTICE`/`COPYING`)
