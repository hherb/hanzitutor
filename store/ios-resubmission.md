# The iOS resubmission after the 2.1(a) rejection

## What App Review saw

Submission `2207a401-9355-406d-a5e0-50c0c47db557`, reviewed 30 September 2026 on an
iPad Air 11-inch (M3) running iPadOS 27.0, version reviewed **1.0 (0.5.11)**: the app
crashed on launch. Four crash reports were attached, all the same crash —

```
EXC_BAD_ACCESS (SIGSEGV, or SIGBUS after a KERN_PROTECTION_FAILURE) in objc_retain
  ← -[UIApplication _connectUISceneFromFBSScene:transitionContext:] +1268
```

— with no frame of this app's own code on the stack. UIKit was retaining an object it
had been handed earlier by `application:configurationForConnectingSceneSession:options:`,
which `tao` 0.35.3 implemented while returning a pointer it had **already released**. It
is a use-after-free, and it was fatal only under optimisation: at `-Onone` the object
survives in the autorelease pool, optimised it is freed before UIKit looks at it. The
0.5.11 build was the first *release* build of this code ever installed on a device, so
nothing here had seen it.

It was not the reviewer's device. The same crash is in this repository's own device
logs of 2026-09-25, and it reproduces on an iPadOS **27.0** simulator from a release
build — which is the loop to use, and the reason it had been missed: the simulator
installed at the time ran iOS 18.

Fixed with upstream's own one-line fix
([tauri-apps/tao#1245](https://github.com/tauri-apps/tao/pull/1245)), carried at the
time as a one-line patch under `vendor/` (retired since — see "Afterwards" below).
Nothing in this app's own code changed.

## The artifact to upload

| | |
| --- | --- |
| File | `Hanzi Tutor 0.6.0 iOS app-store.ipa` |
| Built from | working tree of 30 September 2026, patch in `vendor/tao` applied |
| `CFBundleShortVersionString` | `0.6.0` |
| `CFBundleVersion` | `0.6.0` (the reviewed build was `0.5.11`) |
| Signing | Apple Distribution: Horst Herb (X5DWXB4283), `beta-reports-active`, `get-task-allow = false` |
| Size | 125,431,213 bytes (119.6 MiB); the executable inside is 120,445,632 bytes |
| SHA-256 | `14d898f410f34c660fbd56e70891669d6f26352ff05a2c52ad2c2c02cdde9ebf` |
| Command | `./scripts/with-build-caches.sh ./scripts/with-cargo-env.sh ./scripts/tauri-cli.sh ios build --target aarch64 --ci` |

Upload with Transporter, or let Xcode's Organizer do it. The build number only has to
be greater than `0.5.11`, which `0.6.0` is; if a `0.6.0` iOS build has been uploaded
since this was written, add `--build-number <n>` to the command rather than reusing it.

The IPA is not committed — it is 120 MB and it is reproducible. The command writes it
to `src-tauri/gen/apple/build/arm64/`, and the copy the checksum above belongs to is
`.tmp-release-060/appstore-ios/`; both are gitignored, so rebuild if they have been
cleaned up. The **archive's** app, in the same build directory, is the one to install
for a device test: the exported IPA is distribution-signed and `devicectl` refuses it
by design (HANDOVER §6).

## Reply to App Store Connect

> Thank you for the review, and for the crash logs — they identified the problem
> precisely.
>
> The crash was in the windowing library the app uses rather than in code we wrote: a
> use-after-free in the object UIKit is handed when it connects the app's scene. It
> was fatal only in optimised builds, which is why our own device testing had not
> caught it — we had been testing debug builds.
>
> We have applied the library maintainers' own fix for it (tauri-apps/tao #1245) and
> verified the result on a physical iPhone running iOS 27, and on iPad Air 11-inch and
> 13-inch simulators running iPadOS 27.0 and iOS 26.2. To be sure the fix addresses
> what you saw, we also rebuilt the app with the fix removed: on those same iPadOS
> 27.0 simulators it terminates at launch with `SIGSEGV`, exactly as your reports show,
> and with the fix it runs.
>
> Build 0.6.0 (0.6.0) is submitted for review. No other behaviour of the app changed.

## What was verified, and where the evidence is

| Check | Result |
| --- | --- |
| Shipped binary passes the configuration to `_objc_autorelease`, not `_objc_release` | disassembled from the exported IPA; HANDOVER §6 has the command |
| Release build runs on a physical iPhone (iOS 27.0) | installed and left running; screenshot in HANDOVER §6 |
| Release build runs on iPad Air 11-inch and 13-inch (iPadOS 27.0) | simulators, screenshots |
| Release build runs on iPad Air 11-inch and 13-inch (iOS 26.2) | simulators |
| The same build **without** the fix dies on those simulators | `SIGSEGV(11)` at launch, from the simulator's system log |
| `pnpm test`, `pnpm run test:web`, `pnpm run check:rust` | pass |

## Known limits of that verification

- The reviewer's exact combination — iPad Air **11-inch (M3)** on iPadOS **27.0** — has
  no simulator. It is covered as M4 on 27.0 (the OS) and M3 on 26.2 (the hardware).
- Touch and rotation could not be injected: `simctl` has no input command and
  AppleScript automation is refused in this environment, so the simulator runs confirm
  launch and rendering at 834 pt and 1024 pt portrait, not interaction.
- The simulator produced no `.ips` file; the crash is evidenced by the exit status
  `domain:signal(2) code:SIGSEGV(11)` in the unified log.

## Afterwards: the patch was retired

The artifact above was built with `vendor/tao` — the published `tao` 0.35.3 carrying
upstream's one-line fix — because `tauri-runtime-wry` 2.11.4 pinned `tao ^0.35` and no
0.35.x release ever received it. The build is what it is and the checksum above still
describes it; nothing about the review depends on how the fix got in.

The patch no longer exists in the tree. `tauri` 2.12.0 (2026-09-26) pulls
`tauri-runtime-wry` 2.12.0, which requires `tao ^0.37.0`, so the fix now arrives as a
dependency and the vendored copy, its `[patch.crates-io]` entry and the workspace
`exclude` were all removed. The upgraded release build was re-verified the same way:
the exported IPA's own binary passes the scene configuration to `_objc_autorelease`,
and release builds run and render on the four iPad simulators (11 and 13 inch, iPadOS
27.0 and iOS 26.2). HANDOVER invariant 38 records the version floor and what to check
if the scene code is ever touched again.

**The version has moved to 0.7.0 for that, so the 0.6.0 build described above is the
one that was submitted** and any further upload is 0.7.0. Nothing a learner would
notice changed in it, which is why 0.7.0 has no "what's new" page — see HANDOVER §8
and the note under **What's New in This Version** in
[`app-store-listing.md`](app-store-listing.md).
