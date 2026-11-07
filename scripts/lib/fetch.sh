# shellcheck shell=bash
#
# A download checked against a recorded SHA-256. Sourced by `fetch-data.sh`, and
# by `scripts/tests/test-scripts.sh`, which drives it with a stubbed `curl`.
#
# Written for bash 3.2, which is what `/usr/bin/env bash` finds on a stock Mac.

# hash_of <file>
#
# macOS ships `shasum`; coreutils gives `sha256sum`. Same digest either way. The
# file is read from stdin so that neither tool prints its name: GNU `sha256sum`
# prefixes the line with a backslash when a name needs escaping, which `cut`
# would then hand back as part of the digest.
if command -v sha256sum >/dev/null 2>&1; then
  hash_of() { sha256sum <"$1" | cut -d' ' -f1; }
else
  hash_of() { shasum -a 256 <"$1" | cut -d' ' -f1; }
fi

# fetch <url> <destination> <sha256> [advice]
#
# Skips a destination that is already there, but checks it either way: a copy
# left by an older, unpinned run is exactly what the digest is for. A fresh
# download that does not match is deleted; one that was already on disk is left
# alone (it may be a committed file) and named, so the person running this
# decides. An empty digest checks nothing — `fetch-data.sh` passes one only for
# `JMdict_e.gz`, which upstream does not version.
#
# `advice`, when given, replaces the closing lines of either mismatch message: the
# generic "re-record the digest" is wrong for a source whose digest the caller
# supplied for itself.
#
# How it retries: `raw.githubusercontent.com` resets the connection (`curl: (56)`)
# rather than answering when it is asked for hundreds of files at once, and a
# retry is the whole difference between "the file is not there" and "the fetch
# was unlucky". Deliberately *not* `--retry-all-errors`: a genuine 404 should stay
# a 404.
fetch() {
  local url="$1" dest="$2" want="$3" advice="${4:-}" got
  if [ -s "$dest" ]; then
    echo "  have  $(basename "$dest")  ($(du -h "$dest" | cut -f1))"
  else
    echo "  get   $(basename "$dest")"
    if ! curl --fail --location --silent --show-error --retry 4 --retry-delay 1 --retry-connrefused "$url" --output "$dest.partial"; then
      rm -f "$dest.partial"
      echo "        failed: $url" >&2
      return 1
    fi
    got="$(hash_of "$dest.partial")"
    if [ -n "$want" ] && [ "$got" != "$want" ]; then
      rm -f "$dest.partial"
      echo "error: $(basename "$dest") did not match its recorded digest; deleted." >&2
      echo "  from     $url" >&2
      echo "  expected $want" >&2
      echo "  got      $got" >&2
      echo "${advice:-Re-record the digest in this script only after reading what changed.}" >&2
      return 1
    fi
    mv "$dest.partial" "$dest"
    echo "        -> $(du -h "$dest" | cut -f1)"
  fi

  if [ -z "$want" ]; then
    return 0
  fi
  got="$(hash_of "$dest")"
  if [ "$got" != "$want" ]; then
    echo "error: $dest does not match its recorded digest." >&2
    echo "  expected $want" >&2
    echo "  got      $got" >&2
    if [ -n "$advice" ]; then
      echo "$advice" >&2
    else
      echo "It was already on disk, so it was left alone. If it came from an earlier," >&2
      echo "unpinned run of this script, delete it and re-run; if it is committed," >&2
      echo "run git diff on it: the committed copy is what the digest was recorded from." >&2
    fi
    return 1
  fi
}
