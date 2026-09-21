#!/usr/bin/env bash
#
# Fetch the MeloTTS text-to-speech model that pronunciation audio is synthesised
# with, and check every file against a pinned digest before anything uses it.
#
#   .melo-tts/<model>/   the model files, ~58 MB
#   .melo-tts/current    a symlink to the model directory above, so the digests
#                        and the path callers use are two separate decisions
#
# Both are gitignored. Like `.sherpa-onnx/`, this is a **build input** — a few
# tens of megabytes of weights — not source that belongs in a commit.
#
# ## Why this script exists
#
# `scripts/fetch-sherpa.sh` states the principle for the native libraries: the
# repository's posture is that data arrives through a reviewed script, that
# notices are pinned in three places at once, and that nothing enters a build
# which a reviewer did not agree to. ROADMAP.md M12 says it in one line:
# *"vendor and pin it, or the build is not reproducible."*
#
# A downloaded model weight is in exactly that position, and it is worse than a
# library in one respect: nobody publishes a checksum for it. So the digest table
# below **is** the pin, and it was recorded by hand — download once, hash what
# arrived, record it here. A model whose digest is not in the table is refused
# rather than quietly trusted, because a file that was never hashed is not pinned.
#
# ## Provenance, and why the digest is not enough on its own
#
# The weights come from `csukuangfj/vits-melo-tts-zh_en` on Hugging Face: the
# sherpa-onnx project's own conversion of MeloTTS to ONNX, so the runtime this
# repository already links can execute it. MeloTTS is MIT (myshell-ai), and the
# model repository carries a `LICENSE` naming MyShell.ai as the copyright holder
# under the same MIT terms — so the weights are MIT, not merely "unknown".
#
# Worth stating plainly, because the Hugging Face API reports no licence for this
# repository (`cardData: null`) and an earlier draft of this script concluded
# that there was therefore no licence metadata at all. There is: it is a file in
# the repository that the card does not surface. The distinction matters, because
# "the API did not say" and "the publisher granted nothing" lead to opposite
# decisions. See LICENSES.md, and the notice at licences/MIT-MeloTTS.txt.
#
# Usage:
#   scripts/fetch-tts.sh          fetch the model, verifying every digest
#   scripts/fetch-tts.sh --check  verify an existing copy and report, fetching nothing
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/.melo-tts"

# The conversion this table pins. The repository is the sherpa-onnx project's
# own model host; the revision is `main`, which is why the digests below carry
# the whole guarantee — a re-upload would be caught rather than picked up.
MODEL="vits-melo-tts-zh_en"
BASE_URL="https://huggingface.co/csukuangfj/${MODEL}/resolve/main"

# `<name>:<sha256>:<bytes>`. The size is recorded as well as the hash because it
# makes a truncated download say so in its own words rather than as a checksum
# mismatch, which is the failure that actually happens on a flaky connection.
#
# First recorded 2026-09-21 by downloading each file and hashing it, during the
# pronunciation-audio spike. `model.int8.onnx` is the quantised network; the four
# `.fst` files are the text-normalisation rewrites sherpa-onnx applies before
# synthesis (dates, numbers, phone numbers, heteronyms) and are what makes a
# sentence read as a sentence rather than as a bag of characters.
FILES=(
  "model.int8.onnx:f085f5079e05f039b800aeb542f5253c26a303211b0c6465d0d9387977855a63:53517430"
  "tokens.txt:d18664a7e12bd7ea1022ddaf951e534e136815016c5a809d6b64156bffb4369d:655"
  "lexicon.txt:7236884b02435ac5d10cf69b4be40a61b45aa676b5300f0e412f185748fee528:6837671"
  "date.fst:eb8aa079ae3cb81d8f4404992f39d61a0cb990947512b5b8d1e54d1f6980e718:59154"
  "number.fst:743f402181fcfebf76cc2f0546b71fa26476e626fbe4e460fb7b4c3a7a8bd5bd:64482"
  "phone.fst:1ac2b6fa56b1442320c4de7db08353bab8963a2b57f365eebcdd3a2d3562f8d7:88630"
  "new_heteronym.fst:ca14b2127e27baa571664e4bb791e143e7425f56a6bc29db08d74f97e6aa4e29:21974"
)

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

MODEL_DIR="$DEST/$MODEL"

# macOS ships `shasum`; coreutils gives `sha256sum`. Same digest either way.
if command -v sha256sum >/dev/null 2>&1; then
  hash_of() { sha256sum "$1" | cut -d' ' -f1; }
else
  hash_of() { shasum -a 256 "$1" | cut -d' ' -f1; }
fi

size_of() { wc -c <"$1" | tr -d '[:space:]'; }

verify() { # <path> <expected-sha> <expected-bytes> -> 0 when it matches
  local path="$1" want_sha="$2" want_size="$3" got_sha got_size
  [ -f "$path" ] || return 1
  got_size="$(size_of "$path")"
  if [ "$got_size" != "$want_size" ]; then
    echo "  bad   $(basename "$path"): $got_size bytes, expected $want_size" >&2
    return 1
  fi
  got_sha="$(hash_of "$path")"
  if [ "$got_sha" != "$want_sha" ]; then
    echo "  bad   $(basename "$path"): sha256 $got_sha" >&2
    echo "        expected $want_sha" >&2
    return 1
  fi
  return 0
}

if [ "$CHECK_ONLY" = 1 ]; then
  echo "checking $MODEL_DIR"
  status=0
  for row in "${FILES[@]}"; do
    IFS=: read -r name sha size <<<"$row"
    if verify "$MODEL_DIR/$name" "$sha" "$size"; then
      echo "  ok    $name"
    else
      status=1
    fi
  done
  exit "$status"
fi

mkdir -p "$MODEL_DIR"

echo "fetching MeloTTS weights into $MODEL_DIR"
for row in "${FILES[@]}"; do
  IFS=: read -r name sha size <<<"$row"
  target="$MODEL_DIR/$name"

  # Already present and correct: the common case on a re-run, and the reason
  # this script is safe to run again on a complete checkout.
  if verify "$target" "$sha" "$size"; then
    echo "  have  $name  ($(du -h "$target" | cut -f1))"
    continue
  fi

  echo "  get   $name"
  if ! curl --fail --location --silent --show-error "$BASE_URL/$name" --output "$target.partial"; then
    rm -f "$target.partial"
    echo "        failed: $BASE_URL/$name" >&2
    exit 1
  fi
  mv "$target.partial" "$target"

  # A download that does not match its recorded digest is deleted rather than
  # kept, so a failed run cannot leave something that a later one mistakes for
  # a pinned copy. This is the same rule fetch-sherpa.sh applies to its archives.
  if ! verify "$target" "$sha" "$size"; then
    rm -f "$target"
    echo "        digest mismatch — deleted; refusing to use it" >&2
    exit 1
  fi
  echo "        -> $(du -h "$target" | cut -f1)"
done

ln -sfn "$MODEL" "$DEST/current"
echo "done — $DEST/current -> $MODEL"
