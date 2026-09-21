#!/usr/bin/env python3
"""Fetch the dataset's own recordings for the graded phrases.

The `no7z/hsk-sentences-audio` corpus publishes an MP3 for every sentence it
grades — normal and slow — generated with CosyVoice2-0.5B. Its attribution file
grants redistribution of that synthesised output under **Apache-2.0**, asking
only that it be disclosed as synthetic speech (音频由 CosyVoice2-0.5B 本地合成 …
本项目依 Apache-2.0 再分发合成输出).

## Why this rather than generating them

Measured on 208 HSK-1 phrases, through one recogniser and one judge, counting only
the phrases where the reference passed and the generated audio failed:

| source | failures |
| --- | --- |
| this dataset's own audio | **15 / 208** |
| MeloTTS, generated here | 57 / 208 |

Generating was ~4x worse than the audio that already ships with the text. The
synthesis path stays for what it is genuinely needed for — on-device speech for a
phrase with no recording — but the bundled course should use the best available
audio, and this is it.

## What it writes

    public/audio/<source>/<id>.mp3          committed, because the app ships them
    public/audio/<source>/<id>_slow.mp3

The same naming and layout `synthesize-audio` produces, so the manifest, the
frontend and `check-audio.py` cannot tell the difference.

## Resumable

Every clip is written under its final name as it arrives, and a file already
present with a plausible size is not fetched again. A run stopped part-way costs
the download in flight, not the pass — the rule for anything over ten minutes.

Usage:
    scripts/fetch-clip-audio.py                      every HSK 1-2 phrase
    scripts/fetch-clip-audio.py --limit 20           a quick check
    scripts/fetch-clip-audio.py --levels 1 2 3       other levels
"""

import argparse
import concurrent.futures as futures
import json
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "data" / "phrases" / "no7z-sentences.jsonl"
OUT = ROOT / "public" / "audio"
BASE = "https://huggingface.co/datasets/no7z/hsk-sentences-audio/resolve/main/audio"

# Below this a file is a truncated download or an error page, not a clip. The
# smallest real clips are about 5 KB.
MIN_BYTES = 1024


def fetch_one(url: str, dest: Path) -> tuple[bool, str]:
    """Download one clip unless it is already there. Returns (ok, note)."""
    if dest.exists() and dest.stat().st_size >= MIN_BYTES:
        return True, "have"
    tmp = dest.with_suffix(dest.suffix + ".partial")
    try:
        with urllib.request.urlopen(url, timeout=60) as response:
            body = response.read()
    except Exception as e:  # noqa: BLE001 - the note is the report
        return False, f"{type(e).__name__}: {e}"
    if len(body) < MIN_BYTES:
        return False, f"only {len(body)} bytes"
    tmp.write_bytes(body)
    # Renamed into place, so an interruption cannot leave a short file that a
    # later run would count as present.
    os.replace(tmp, dest)
    return True, "get"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--source", default="no7z",
                    help="corpus name; the output subdirectory and id namespace")
    ap.add_argument("--corpus", default=str(CORPUS), help="the sentence file to read ids from")
    ap.add_argument("--levels", nargs="*", type=int, default=[1, 2],
                    help="HSK levels to fetch (default 1 2)")
    ap.add_argument("--limit", type=int, default=0, help="stop after this many phrases")
    ap.add_argument("--jobs", type=int, default=8, help="parallel downloads")
    args = ap.parse_args()

    if not Path(args.corpus).is_file():
        print(f"no corpus at {args.corpus} — run scripts/fetch-phrases.sh first", file=sys.stderr)
        return 1

    phrases = []
    with open(args.corpus, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if args.levels and row.get("hsk_level") not in args.levels:
                continue
            phrases.append(row)
    if args.limit:
        phrases = phrases[: args.limit]
    if not phrases:
        print("nothing to fetch for those levels", file=sys.stderr)
        return 1

    out_dir = OUT / args.source
    out_dir.mkdir(parents=True, exist_ok=True)

    jobs = []
    for row in phrases:
        cid = row["id"]
        for suffix in ("", "_slow"):
            jobs.append((f"{BASE}/{cid}{suffix}.mp3", out_dir / f"{cid}{suffix}.mp3"))

    print(f"fetching {len(jobs)} clips for {len(phrases)} phrases into {out_dir}")
    fetched = have = failed = 0
    failures = []
    with futures.ThreadPoolExecutor(args.jobs) as pool:
        results = pool.map(lambda j: (j[1].name, *fetch_one(*j)), jobs)
        for n, (name, ok, note) in enumerate(results, 1):
            if ok:
                if note == "have":
                    have += 1
                else:
                    fetched += 1
                if n % 100 == 0 or n == len(jobs):
                    print(f"  [{n}/{len(jobs)}] {fetched} fetched, {have} present", flush=True)
            else:
                failed += 1
                failures.append((name, note))
                print(f"  FAILED {name}: {note}", file=sys.stderr)

    print(f"\n{fetched} fetched, {have} already present, {failed} failed")
    if failures:
        print("missing clips (the manifest will skip these):")
        for name, note in failures[:20]:
            print(f"  {name}: {note}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
