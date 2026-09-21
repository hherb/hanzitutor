#!/usr/bin/env bash
#
# Check the bundled HSK 1-2 clips, and compare them against the reference audio
# the dataset publishes for the same sentences.
#
# The differential is the point. A raw failure count cannot tell "the model said
# it wrong" from "the recogniser heard it wrong" — `verify-audio` has one
# transcription and no ground truth. Running the *reference* audio for the same
# sentence through the same recogniser and the same judge gives that ground truth
# by construction: a phrase the reference passes and ours fails is ours alone.
#
# Measured 2026-09-21 on a 208-phrase sample, normal takes, MeloTTS:
#
#   ours 57 failed, reference 15, both 11, ours alone 46
#
# The `both` set is the recogniser's limit and should be reviewed by ear rather
# than trusted either way.
#
# Usage:
#   scripts/check-phrases.sh              check and compare
#   scripts/check-phrases.sh --no-ref     skip the reference download
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

ASR="${ASR_DIR:-$ROOT/.tmp-asr/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-int8-2024-07-17}"
CORPUS="$ROOT/data/phrases/no7z-sentences.jsonl"
READINGS="${READINGS:-/tmp/readings.tsv}"
WORK="${WORK:-/tmp/phrase-check}"

if [ ! -f "$ASR/model.int8.onnx" ]; then
  echo "no recogniser at $ASR — set ASR_DIR to one" >&2
  exit 1
fi
if [ ! -f "$READINGS" ]; then
  echo "no readings at $READINGS — generate character<TAB>pinyin lines first" >&2
  exit 1
fi

verify() { # <base> <manifest> <label>
  ./scripts/with-cargo-env.sh cargo run --release -q -p hanzi-say --bin verify-audio -- \
    --base "$1" --manifest "$2" --corpus "$CORPUS" --readings "$READINGS" --asr "$ASR"
}

echo "=== ours ==="
verify public public/audio/no7z/manifest.json || true

if [ "${1:-}" = "--no-ref" ]; then
  exit 0
fi

# The reference audio, fetched once and cached: this is ~26 MB for the HSK 1-2
# set and only needs downloading the first time.
mkdir -p "$WORK/ref" "$WORK/root/audio"
python3 - "$ROOT" "$WORK" <<'PY'
import json, os, sys, urllib.request
root, work = sys.argv[1], sys.argv[2]
manifest = json.load(open(os.path.join(root, "public/audio/no7z/manifest.json")))
ref = {"source": "no7z-reference", "model": "cosyvoice2", "speeds": [1.0, 0.7], "phrases": []}
missing = 0
for phrase in manifest["phrases"]:
    cid = phrase["id"]
    dest = os.path.join(work, "ref", f"{cid}.mp3")
    if not (os.path.exists(dest) and os.path.getsize(dest) > 1000):
        try:
            urllib.request.urlretrieve(
                "https://huggingface.co/datasets/no7z/hsk-sentences-audio/resolve/main/audio/"
                f"{cid}.mp3", dest)
        except Exception:
            missing += 1
            continue
    ref["phrases"].append({**phrase, "audio": {"normal": f"audio/{cid}.mp3",
                                               "slow": f"audio/{cid}.mp3"}})
json.dump(ref, open(os.path.join(work, "ref-manifest.json"), "w"), ensure_ascii=False)
print(f"reference manifest: {len(ref['phrases'])} phrases ({missing} could not be fetched)")
PY

echo
echo "=== reference (the dataset's own CosyVoice2 audio) ==="
verify "$WORK/root" "$WORK/ref-manifest.json" || true

# Split the two failure sets. `ours alone` is the number that matters.
: > "$WORK/mine.txt"; : > "$WORK/ref.txt"
verify public public/audio/no7z/manifest.json 2>/dev/null \
  | grep '^  FAIL' | awk '{print $2}' | sort > "$WORK/mine.txt" || true
verify "$WORK/root" "$WORK/ref-manifest.json" 2>/dev/null \
  | grep '^  FAIL' | awk '{print $2}' | sort > "$WORK/ref.txt" || true

echo
echo "=== differential ==="
echo "  ours fails        : $(wc -l < "$WORK/mine.txt" | tr -d ' ')"
echo "  reference fails   : $(wc -l < "$WORK/ref.txt" | tr -d ' ')"
echo "  both (recogniser) : $(comm -12 "$WORK/mine.txt" "$WORK/ref.txt" | wc -l | tr -d ' ')"
echo "  OURS ALONE        : $(comm -23 "$WORK/mine.txt" "$WORK/ref.txt" | wc -l | tr -d ' ')"
echo
echo "ours alone — the clips to fix or inpaint:"
comm -23 "$WORK/mine.txt" "$WORK/ref.txt" | head -40
echo
echo "both fail — unrecognisable to the recogniser; needs an ear, not a verdict:"
comm -12 "$WORK/mine.txt" "$WORK/ref.txt" | tr '\n' ' '
echo
