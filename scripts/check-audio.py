#!/usr/bin/env python3
"""Check the synthesised pronunciation clips before they are committed.

The clips are tens of megabytes of binary that a reviewer cannot read, and every
defect found in them so far was found by measuring rather than by looking: a
phrase quietly missing a clause, a clip that was digital silence, a whole set
22 dB too quiet. This is the gate that catches those without an ear.

Per clip it checks:

1. **The file exists.** A manifest entry with no file is a button that plays
   nothing.
2. **It decodes.** A truncated write is a file that exists.
3. **It is audible.** A peak below the threshold is the model having returned
   silence — a real failure mode, not a hypothetical one.
4. **It is clean.** No clipping.
5. **It is the right length for its text.** Real speech runs roughly 0.15–0.5 s
   per character; outside a generous band, a clause was dropped or repeated.

Usage:
    scripts/check-audio.py              every corpus under public/audio
    scripts/check-audio.py no7z         one corpus
"""

import array
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIO = ROOT / "public" / "audio"

# The peak below which a clip is silence. Must agree with `SILENCE_PEAK` in
# crates/hanzi-say/src/lib.rs — synthesis refuses to write one in the first
# place, and this is the check that it did.
SILENCE_PEAK = 1e-3
# Every clip is normalised to a peak of 0.7, so anything well below this means
# normalisation did not happen.
MIN_PEAK = 0.30
# Seconds of speech per character, generous on both sides. Chinese speech runs
# about 0.2-0.3 s/char; the band allows for a slow take at 0.7 speed and for
# pausing at punctuation.
MIN_PER_CHAR, MAX_PER_CHAR = 0.12, 0.75


def decode(path: Path):
    """Mono float samples at 24 kHz, and the duration in seconds."""
    probe = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "csv=p=0", str(path)],
        capture_output=True, text=True,
    )
    if probe.returncode != 0 or not probe.stdout.strip():
        return None, None
    duration = float(probe.stdout.strip())

    pcm = subprocess.run(
        ["ffmpeg", "-v", "error", "-i", str(path), "-f", "f32le",
         "-ac", "1", "-ar", "24000", "-"],
        capture_output=True,
    ).stdout
    if not pcm:
        return None, duration
    samples = array.array("f")
    samples.frombytes(pcm)
    return samples, duration


def check(source: str) -> tuple[int, list[str]]:
    """Check one corpus. Returns (clips checked, problem lines)."""
    manifest_path = AUDIO / source / "manifest.json"
    if not manifest_path.is_file():
        return 0, [f"{source}: no manifest.json"]

    manifest = json.loads(manifest_path.read_text())
    phrases = manifest["phrases"]
    problems: list[str] = []
    durations: list[float] = []
    checked = 0

    for phrase in phrases:
        text = phrase.get("text", "")
        for kind, rel in phrase["audio"].items():
            # `rel` is root-relative and already carries `audio/`, because that
            # is what the webview fetches.
            path = ROOT / "public" / rel
            if not path.is_file():
                problems.append(f"missing file: {rel}")
                continue
            samples, duration = decode(path)
            if samples is None:
                problems.append(f"does not decode: {rel}")
                continue
            checked += 1

            peak = max((abs(s) for s in samples), default=0.0)
            seconds_per_char = duration / max(len(text), 1)
            durations.append(duration)

            if peak < SILENCE_PEAK:
                problems.append(f"silent ({peak:.5f}): {rel}")
            elif peak < MIN_PEAK:
                problems.append(f"too quiet ({peak:.3f}): {rel}")
            if peak >= 1.0:
                problems.append(f"clips ({peak:.3f}): {rel}")
            if not (MIN_PER_CHAR <= seconds_per_char <= MAX_PER_CHAR):
                problems.append(
                    f"wrong length ({duration:.2f}s for {len(text)} chars): {rel}"
                )

    summary = (
        f"  {len(phrases)} phrases, {checked} clips"
        + (
            f", {sum(durations) / len(durations):.2f}s mean "
            f"({min(durations):.2f}-{max(durations):.2f}s)"
            if durations
            else ""
        )
    )
    print(summary)
    return checked, problems


def check_dist() -> list[str]:
    """Check the built frontend actually carries the clips, and where.

    Three places hard-code this layout and none imports the others: the
    synthesiser writes to `public/audio/<source>/`, this script reads it there,
    and `src/lib/audio.ts` fetches `audio/<source>/manifest.json` relative to the
    web root. Vite copies `public/` verbatim, so the path the app asks for is the
    path on disk minus the `public/` prefix — but nothing enforces that, and if it
    drifts the phrase screen shows an error instead of phrases.

    Run after `vite build`. A missing `dist/` is reported rather than ignored:
    the caller asked about the build, so saying nothing would be the wrong answer.
    """
    dist = ROOT / "dist"
    if not dist.is_dir():
        return ["dist/ does not exist — run: npx vite build"]

    problems = []
    for source in sorted(d.name for d in AUDIO.iterdir() if d.is_dir()):
        # Exactly what `audio.ts` fetches, resolved from the web root.
        built = dist / "audio" / source / "manifest.json"
        if not built.is_file():
            problems.append(f"dist is missing audio/{source}/manifest.json")
            continue
        manifest = json.loads(built.read_text())
        for phrase in manifest["phrases"]:
            for rel in phrase["audio"].values():
                if not (dist / rel).is_file():
                    problems.append(f"dist is missing {rel}")
    return problems


def main() -> int:
    dist_only = "--dist" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("--")]

    if not AUDIO.is_dir():
        print("no public/audio — run fetch-phrases and synthesize-audio first", file=sys.stderr)
        return 1

    if dist_only:
        problems = check_dist()
        if problems:
            print(f"{len(problems)} problem(s) in the built frontend:")
            for line in problems[:20]:
                print(f"  {line}")
            return 1
        print("the built frontend carries every clip the manifests name")
        return 0

    sources = args or sorted(
        d.name for d in AUDIO.iterdir() if d.is_dir()
    )
    if not sources:
        print("no corpora under public/audio", file=sys.stderr)
        return 1

    total_problems: list[str] = []
    total_clips = 0
    for source in sources:
        checked, problems = check(source)
        total_clips += checked
        total_problems.extend(problems)

    if total_problems:
        print(f"\n{len(total_problems)} problem(s) across {total_clips} clips:")
        for line in total_problems[:40]:
            print(f"  {line}")
        if len(total_problems) > 40:
            print(f"  … and {len(total_problems) - 40} more")
        return 1

    print(f"\nall {total_clips} clips present, audible, clean and the right length")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
