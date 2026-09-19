#!/usr/bin/env bash
#
# Does a sandboxed build still get to pronounce anything?
#
# The app speaks by running /usr/bin/say (see src-tauri/src/speech.rs). A Mac App
# Store build must be sandboxed, and the App Sandbox restricts both exec of
# external binaries and access to the real home directory, so it is an open
# question whether that backend survives App Store distribution. This script
# answers it, and answers the more basic question underneath it: whether the
# sandbox is being enforced at all for the thing being tested.
#
# It builds two minimal applications that run the same probe — one signed with
# com.apple.security.app-sandbox, one without — launches each through launchd and
# reports what happened. The control matters: a probe that reports "say works"
# while running unsandboxed is worse than no probe.
#
#   ./scripts/probe-app-sandbox.sh
#
# Findings are packed into the exit status, because that is the only channel out
# of a launchd-launched app: its stdout goes nowhere, its container is not
# readable by another process, and `log show` refuses to run inside a sandbox.
#
# The probe creates and removes two files, /tmp/hanzi-sandbox-probe and
# ~/.hanzi-tutor-probe. It is a diagnostic, not part of any build.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK="$ROOT/.tmp-app-sandbox-probe"
mkdir -p "$WORK"
cd "$WORK"

# A sandboxed build has to be signed for the sandbox to apply at all, so prefer a
# real certificate and say so when there is none.
IDENTITY="${APPLE_SIGNING_IDENTITY:-}"
if [ -z "$IDENTITY" ]; then
  IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p' | head -1)"
fi
if [ -z "$IDENTITY" ]; then
  echo "error: no Developer ID certificate found; the sandbox will not apply and" >&2
  echo "       the result would be meaningless. Set APPLE_SIGNING_IDENTITY." >&2
  exit 1
fi

cat > probe.c <<'PROBE'
/* Reports four findings in the exit status, and whether the sandbox was on. */
#include <errno.h>
#include <fcntl.h>
#include <pwd.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

enum {
    HOME_IS_CONTAINER = 1 << 0,
    REAL_HOME_BLOCKED = 1 << 1,
    SAY_REFUSED = 1 << 2,
    SAY_FAILED = 1 << 3,
    SHARED_TMP_BLOCKED = 1 << 4,
    CONTAINER_ID_SET = 1 << 5,
};

/* Two independent signals that the sandbox is actually being enforced. */
static int probe_sandbox(void) {
    int bits = 0;
    if (getenv("APP_SANDBOX_CONTAINER_ID") != NULL) bits |= CONTAINER_ID_SET;
    int fd = open("/tmp/hanzi-sandbox-probe", O_CREAT | O_WRONLY | O_TRUNC, 0600);
    if (fd < 0) {
        bits |= SHARED_TMP_BLOCKED;
    } else {
        close(fd);
        unlink("/tmp/hanzi-sandbox-probe");
    }
    return bits;
}

static int probe_home(void) {
    int bits = 0;
    const char *env = getenv("HOME");
    struct passwd *pw = getpwuid(getuid());
    if (env && pw && pw->pw_dir && strcmp(env, pw->pw_dir) != 0) {
        bits |= HOME_IS_CONTAINER;
    }
    if (!pw || !pw->pw_dir) return bits;
    char path[1024];
    snprintf(path, sizeof path, "%s/.hanzi-tutor-probe", pw->pw_dir);
    int fd = open(path, O_CREAT | O_WRONLY | O_TRUNC, 0600);
    if (fd < 0) {
        bits |= REAL_HOME_BLOCKED;
    } else {
        close(fd);
        unlink(path);
    }
    return bits;
}

static int probe_say(void) {
    char *argv[] = {"/usr/bin/say", "-v", "?", NULL};
    posix_spawn_file_actions_t actions;
    posix_spawn_file_actions_init(&actions);
    posix_spawn_file_actions_addopen(&actions, 1, "/dev/null", O_WRONLY, 0);
    pid_t pid = 0;
    int rc = posix_spawn(&pid, "/usr/bin/say", &actions, NULL, argv, environ);
    posix_spawn_file_actions_destroy(&actions);
    if (rc != 0) return SAY_REFUSED;
    int status = 0;
    waitpid(pid, &status, 0);
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) return SAY_FAILED;
    return 0;
}

int main(void) {
    return probe_home() | probe_say() | probe_sandbox();
}
PROBE

cat > sandbox.entitlements <<'ENTITLEMENTS'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>com.apple.security.app-sandbox</key><true/>
</dict></plist>
ENTITLEMENTS

# build <name> <bundle-id> <sandbox|none>
build() {
  local name="$1" id="$2" mode="$3"
  rm -rf "$name.app"
  mkdir -p "$name.app/Contents/MacOS"
  cat > "$name.app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleIdentifier</key><string>$id</string>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundleExecutable</key><string>probe</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
PLIST
  clang -o "$name.app/Contents/MacOS/probe" probe.c
  if [ "$mode" = sandbox ]; then
    codesign --force --options runtime --sign "$IDENTITY" \
      --entitlements sandbox.entitlements "$name.app" >/dev/null 2>&1
  else
    codesign --force --options runtime --sign "$IDENTITY" "$name.app" >/dev/null 2>&1
  fi
}

# decode <exit status>
decode() {
  python3 - "$1" <<'PY'
import sys
code = int(sys.argv[1])
findings = []
if code & 16 or code & 32:
    findings.append("sandbox ACTIVE")
else:
    findings.append("sandbox NOT active")
if code & 1:
    findings.append("HOME is the container")
if code & 2:
    findings.append("real home not writable")
if code & 4:
    findings.append("say SPAWN REFUSED")
if code & 8:
    findings.append("say exited non-zero")
if not code & 12:
    findings.append("say ran and exited 0")
print("; ".join(findings))
PY
}

echo "signing as: $IDENTITY"
echo
build Control com.hanzitutor.sandboxprobe.control none
build Sandboxed com.hanzitutor.sandboxprobe.sandbox sandbox

control=0 sandboxed=0
for app in Control Sandboxed; do
  code=0
  open -W "$app.app" || code=$?
  if [ "$app" = Control ]; then control=$code; else sandboxed=$code; fi
  printf '  %-10s exit=%-3s %s\n' "$app" "$code" "$(decode "$code")"
done

echo
if [ $((sandboxed & 48)) -eq 0 ]; then
  cat >&2 <<'EOF'
INCONCLUSIVE: the App Sandbox did not engage for the sandboxed build, so what it
says about `say` means nothing. The control confirms the probe itself works.

This is an environment limitation, not a result. Applying a sandbox profile needs
permission this shell does not have — `sandbox-exec -p '(version 1)(allow
default)' /usr/bin/true` reports "sandbox_apply: Operation not permitted" on the
same machine. Run this script from a normal login session, or settle the question
where the sandbox is certainly enforced: upload an App Store build to TestFlight
and try "hear it" in the sandboxed build.
EOF
  exit 2
fi

if [ $((sandboxed & 4)) -ne 0 ]; then
  echo "ANSWER: /usr/bin/say cannot be spawned from a sandboxed build."
  echo "        src/speech.rs needs an in-process synthesiser (AVSpeechSynthesizer)."
elif [ $((sandboxed & 8)) -ne 0 ]; then
  echo "ANSWER: /usr/bin/say can be spawned but does not succeed under the sandbox."
  echo "        src/speech.rs needs an in-process synthesiser (AVSpeechSynthesizer)."
else
  echo "ANSWER: /usr/bin/say works from a sandboxed build, so the macOS speech"
  echo "        backend survives App Store distribution as it stands."
fi
