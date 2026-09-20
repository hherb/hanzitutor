#!/usr/bin/env python3
"""Line-count statistics for this repository, by language and by file.

Counts every text file git knows about — tracked, plus anything untracked that
`.gitignore` does not exclude — and splits each one into code, documentation,
comment and blank lines.

The counting is done by a small per-language scanner rather than by matching
line prefixes, because prefix matching gets this repo wrong: a `#` inside a
Rust string, a `//` inside the `https://` of an href, and a `/*` inside a
nested Rust block comment all look like comments to a prefix matcher. The
scanner tracks strings, escapes and comment nesting, so they are not.

Usage:

    scripts/code_statistics.py                  # summary plus the largest files
    scripts/code_statistics.py --all            # every file, not just the top 25
    scripts/code_statistics.py --by-dir         # add a per-directory breakdown
    scripts/code_statistics.py --sort total     # rank files by total lines
    scripts/code_statistics.py --json           # the same numbers, for a machine
    scripts/code_statistics.py --include-generated

Lockfiles, the vendored licence texts and the generated mobile projects under
`src-tauri/gen/` are left out by default, so that the totals describe code that
is actually written here rather than code that a tool emitted. They are listed
at the foot of the report, and `--include-generated` folds them back in.

Standard library only: it runs wherever python3 does, with no install step.
"""

from __future__ import annotations

import argparse
import dataclasses
import fnmatch
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Paths whose lines nobody wrote by hand. Matched against the repo-relative
# path with fnmatch, so a trailing /* covers a whole tree.
GENERATED = (
    "src-tauri/gen/*",  # Tauri regenerates the Android and Xcode projects.
    "Cargo.lock",
    "pnpm-lock.yaml",
    "package-lock.json",
    "*.lock",
    "licences/*",  # Upstream licence texts, reproduced verbatim.
    "LICENSE",
    "crates/*/data/*",  # The compact dataset artifact.
)

# Directories never worth walking when git is not available to filter for us.
SKIP_DIRS = {
    ".git",
    "node_modules",
    "target",
    "dist",
    ".cargo-home",
    ".cargo-target",
    ".cargo-tools",
    ".pnpm-store",
    "__pycache__",
    ".venv",
}

CODE, MARKUP, DATA, PROSE = "code", "markup", "data", "prose"


@dataclasses.dataclass(frozen=True)
class Syntax:
    """How one language marks comments, documentation and strings.

    `documentation` is deliberately narrower than `comment`: it is the doc
    comment a tool would publish (`///` and `//!` in Rust, `/** */` in
    TypeScript, a docstring in Python), not every explanatory aside. Both are
    reported, so either reading of "documentation lines" is available.
    """

    kind: str = CODE
    line_comment: tuple[str, ...] = ()
    doc_line_comment: tuple[str, ...] = ()
    block_comment: tuple[tuple[str, str], ...] = ()
    doc_block_comment: tuple[tuple[str, str], ...] = ()
    string: tuple[str, ...] = ()
    multiline_string: tuple[str, ...] = ()
    #: Triple-quoted strings that count as documentation when they open a line.
    docstring: tuple[str, ...] = ()
    escape: str = "\\"
    #: Rust and Kotlin allow `/* /* */ */`; C and JavaScript do not.
    nested_block: bool = False
    #: Prose files: every non-blank line is documentation, except inside a
    #: fenced code block, which is code.
    prose: bool = False


C_STRINGS = ('"', "'")
C_LIKE = dict(
    line_comment=("//",),
    block_comment=(("/*", "*/"),),
    doc_block_comment=(("/**", "*/"),),
    string=C_STRINGS,
)
HASH_LIKE = dict(line_comment=("#",), string=C_STRINGS)
XML_LIKE = dict(kind=MARKUP, block_comment=(("<!--", "-->"),), string=('"',))

LANGUAGES: dict[str, tuple[str, Syntax]] = {}


def language(name: str, syntax: Syntax, *extensions: str) -> None:
    for extension in extensions:
        LANGUAGES[extension] = (name, syntax)


language(
    "Rust",
    Syntax(
        line_comment=("//",),
        doc_line_comment=("///", "//!"),
        block_comment=(("/*", "*/"),),
        doc_block_comment=(("/**", "*/"), ("/*!", "*/")),
        string=('"',),
        nested_block=True,
    ),
    ".rs",
)
language(
    "Python",
    Syntax(
        line_comment=("#",),
        string=C_STRINGS,
        docstring=('"""', "'''"),
    ),
    ".py",
)
language(
    "TypeScript",
    Syntax(**C_LIKE, multiline_string=("`",)),
    ".ts",
    ".tsx",
    ".mts",
)
language(
    "JavaScript",
    Syntax(**C_LIKE, multiline_string=("`",)),
    ".js",
    ".mjs",
    ".cjs",
    ".jsx",
)
language(
    "Svelte",
    # Markup, script and style in one file: the union of the three comment
    # styles. Only `"` counts as a string delimiter, because an apostrophe in
    # prose between two tags is far more common than a single-quoted attribute.
    Syntax(
        line_comment=("//",),
        block_comment=(("/*", "*/"), ("<!--", "-->")),
        doc_block_comment=(("/**", "*/"),),
        string=('"',),
        multiline_string=("`",),
    ),
    ".svelte",
)
language(
    "Kotlin",
    Syntax(
        line_comment=("//",),
        block_comment=(("/*", "*/"),),
        doc_block_comment=(("/**", "*/"),),
        string=('"',),
        multiline_string=('"""',),
        nested_block=True,
    ),
    ".kt",
    ".kts",
)
language("Swift", Syntax(**C_LIKE, nested_block=True), ".swift")
language("Objective-C", Syntax(**C_LIKE), ".m", ".mm", ".h")
language("Java", Syntax(**C_LIKE), ".java")
language("Groovy", Syntax(**C_LIKE), ".gradle", ".groovy")
language("Ruby", Syntax(line_comment=("#",), string=C_STRINGS), ".rb")
language("Shell", Syntax(**HASH_LIKE), ".sh", ".bash", ".zsh")
language("CSS", Syntax(kind=CODE, block_comment=(("/*", "*/"),), string=C_STRINGS), ".css")
language("HTML", Syntax(**XML_LIKE), ".html", ".htm")
language(
    "XML",
    Syntax(**XML_LIKE),
    ".xml",
    ".plist",
    ".storyboard",
    ".xcscheme",
    ".xcworkspacedata",
    ".entitlements",
    ".svg",
    ".xib",
)
language("TOML", Syntax(kind=DATA, line_comment=("#",), string=C_STRINGS), ".toml", ".lock")
language("YAML", Syntax(kind=DATA, line_comment=("#",), string=C_STRINGS), ".yml", ".yaml")
language("JSON", Syntax(kind=DATA, string=('"',)), ".json", ".xcsettings")
language(
    "Config",
    Syntax(kind=DATA, line_comment=("#", "!")),
    ".properties",
    ".editorconfig",
    ".gitignore",
    ".gitattributes",
    ".pro",
)
language("Batch", Syntax(line_comment=("rem ", "REM ", "::"), string=('"',)), ".bat", ".cmd")
language("Xcode project", Syntax(kind=DATA, block_comment=(("/*", "*/"),), string=('"',)), ".pbxproj")
language("Markdown", Syntax(kind=PROSE, prose=True), ".md", ".markdown")
language("Plain text", Syntax(kind=PROSE, prose=True), ".txt")

# Files that carry their language in their name rather than an extension.
BY_NAME: dict[str, tuple[str, Syntax]] = {
    "Podfile": ("Ruby", LANGUAGES[".rb"][1]),
    "Rakefile": ("Ruby", LANGUAGES[".rb"][1]),
    "Gemfile": ("Ruby", LANGUAGES[".rb"][1]),
    "gradlew": ("Shell", LANGUAGES[".sh"][1]),
    "Makefile": ("Make", Syntax(line_comment=("#",), string=C_STRINGS)),
    "Dockerfile": ("Dockerfile", Syntax(kind=DATA, line_comment=("#",), string=C_STRINGS)),
    "LICENSE": ("Plain text", LANGUAGES[".txt"][1]),
    "COPYING": ("Plain text", LANGUAGES[".txt"][1]),
}

# Interpreters seen in a shebang, for files with neither extension nor a known
# name — `src-tauri/gen/android/gradlew` is one of these.
BY_SHEBANG = {
    "sh": ("Shell", LANGUAGES[".sh"][1]),
    "bash": ("Shell", LANGUAGES[".sh"][1]),
    "zsh": ("Shell", LANGUAGES[".sh"][1]),
    "python": ("Python", LANGUAGES[".py"][1]),
    "python3": ("Python", LANGUAGES[".py"][1]),
    "ruby": ("Ruby", LANGUAGES[".rb"][1]),
    "node": ("JavaScript", LANGUAGES[".js"][1]),
}


@dataclasses.dataclass
class Counts:
    """Lines of each kind. Every line of a file falls into exactly one."""

    files: int = 0
    code: int = 0
    documentation: int = 0
    comment: int = 0
    blank: int = 0

    @property
    def total(self) -> int:
        return self.code + self.documentation + self.comment + self.blank

    @property
    def commented(self) -> int:
        """Documentation and plain comments together."""
        return self.documentation + self.comment

    def add(self, other: "Counts") -> None:
        self.files += other.files
        self.code += other.code
        self.documentation += other.documentation
        self.comment += other.comment
        self.blank += other.blank

    def as_dict(self) -> dict[str, int]:
        out = dataclasses.asdict(self)
        out["total"] = self.total
        return out


def _match(line: str, i: int, options: tuple[str, ...]) -> str | None:
    """The longest of `options` that starts at `line[i]`, if any."""
    best = None
    for option in options:
        if line.startswith(option, i) and (best is None or len(option) > len(best)):
            best = option
    return best


def _match_pair(line: str, i: int, pairs: tuple[tuple[str, str], ...]) -> tuple[str, str] | None:
    best = None
    for opener, closer in pairs:
        if line.startswith(opener, i) and (best is None or len(opener) > len(best[0])):
            best = (opener, closer)
    return best


def _skip_string(line: str, i: int, delimiter: str, escape: str) -> int:
    """The index just past the string that opened at `i`, or the line's end.

    A single-line string that is not closed on its line is treated as ending
    there, which is what an unbalanced quote in a comment-like position needs.
    """
    while i < len(line):
        if escape and line[i] == escape:
            i += 2
            continue
        if line.startswith(delimiter, i):
            return i + len(delimiter)
        i += 1
    return len(line)


def scan(text: str, syntax: Syntax) -> Counts:
    """Classify each line of `text` as code, documentation, comment or blank.

    A line holding both code and a trailing comment counts as code, which is
    the convention every other line counter uses. A whitespace-only line counts
    as blank wherever it appears, including inside a block comment or a
    docstring, so that blank lines mean the same thing in every column.
    """
    counts = Counts(files=1)
    block: tuple[str, bool] | None = None  # (closing delimiter, is documentation)
    depth = 0
    string: tuple[str, bool] | None = None  # (delimiter, is documentation)

    for line in text.splitlines():
        if not line.strip():
            counts.blank += 1
            continue

        has_code = has_doc = has_comment = False
        i, n = 0, len(line)
        while i < n:
            if block is not None:
                closer, is_doc = block
                if is_doc:
                    has_doc = True
                else:
                    has_comment = True
                opener = _match_pair(line, i, syntax.block_comment + syntax.doc_block_comment)
                if syntax.nested_block and opener and opener[1] == closer:
                    depth += 1
                    i += len(opener[0])
                    continue
                if line.startswith(closer, i):
                    i += len(closer)
                    depth -= 1
                    if depth <= 0:
                        block, depth = None, 0
                    continue
                i += 1
                continue

            if string is not None:
                delimiter, is_doc = string
                if is_doc:
                    has_doc = True
                else:
                    has_code = True
                if syntax.escape and line[i] == syntax.escape:
                    i += 2
                    continue
                if line.startswith(delimiter, i):
                    string = None
                    i += len(delimiter)
                    continue
                i += 1
                continue

            if line[i].isspace():
                i += 1
                continue

            hit = _match(line, i, syntax.doc_line_comment)
            # `////` is a divider, not a Rust doc comment: a prefix only counts
            # when the next character does not simply repeat it.
            if hit and not line.startswith(hit + hit[-1], i):
                has_doc = True
                break

            hit = _match(line, i, syntax.line_comment)
            # `://` is a URL, in an href or in a comment, and never the start of
            # a comment itself.
            if hit and not (hit == "//" and i and line[i - 1] == ":"):
                has_comment = True
                break

            opener = _match_pair(line, i, syntax.doc_block_comment)
            if opener:
                block, depth, has_doc = (opener[1], True), 1, True
                i += len(opener[0])
                continue

            opener = _match_pair(line, i, syntax.block_comment)
            if opener:
                block, depth, has_comment = (opener[1], False), 1, True
                i += len(opener[0])
                continue

            delimiter = _match(line, i, syntax.docstring)
            if delimiter:
                # A triple-quoted string is documentation when it opens the
                # line, and a value when something precedes it on the line —
                # which distinguishes a module or function docstring from
                # `SQL = """..."""` without needing to parse Python.
                is_doc = not has_code
                string = (delimiter, is_doc)
                has_doc = has_doc or is_doc
                has_code = has_code or not is_doc
                i += len(delimiter)
                continue

            delimiter = _match(line, i, syntax.multiline_string)
            if delimiter:
                has_code = True
                string = (delimiter, False)
                i += len(delimiter)
                continue

            delimiter = _match(line, i, syntax.string)
            if delimiter:
                has_code = True
                i = _skip_string(line, i + len(delimiter), delimiter, syntax.escape)
                continue

            has_code = True
            i += 1

        if has_code:
            counts.code += 1
        elif has_doc:
            counts.documentation += 1
        elif has_comment:
            counts.comment += 1
        else:
            counts.code += 1

    return counts


def scan_prose(text: str) -> Counts:
    """Count a prose file: documentation, except inside a fenced code block.

    Worth the few lines, because this repo's Markdown carries a lot of shell
    and Rust in fences, and calling all of it documentation would overstate the
    prose and hide the examples.
    """
    counts = Counts(files=1)
    fence: str | None = None
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            counts.blank += 1
            continue
        if fence is None and (stripped.startswith("```") or stripped.startswith("~~~")):
            fence = stripped[:3]
            counts.documentation += 1
            continue
        if fence is not None:
            if stripped.startswith(fence):
                fence = None
                counts.documentation += 1
            else:
                counts.code += 1
            continue
        counts.documentation += 1
    return counts


def identify(path: Path) -> tuple[str, Syntax] | None:
    """The language of `path`, or None if it is not one we count."""
    if path.name in BY_NAME:
        return BY_NAME[path.name]
    # `.gitignore` and friends: pathlib calls the whole name a stem, not a
    # suffix, so a dotfile has to be looked up by name.
    if path.name.startswith(".") and path.name in LANGUAGES:
        return LANGUAGES[path.name]
    if path.suffix in LANGUAGES:
        return LANGUAGES[path.suffix]
    if path.suffix:
        return None
    try:
        with path.open("rb") as handle:
            first = handle.readline(256).decode("utf-8", "replace")
    except OSError:
        return None
    if first.startswith("#!"):
        interpreter = first.split()[-1] if "env " in first else first[2:].strip().split()[0]
        return BY_SHEBANG.get(Path(interpreter).name)
    return None


def read_text(path: Path) -> str | None:
    """`path` decoded as UTF-8, or None when it is binary or unreadable."""
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return None


def git_files(root: Path) -> list[str] | None:
    """Everything git would show as part of the working tree, or None.

    Tracked files plus untracked ones that `.gitignore` does not exclude: the
    same set a contributor sees, without reimplementing ignore rules here.
    """
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            capture_output=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return [name for name in result.stdout.decode("utf-8", "replace").split("\0") if name]


def walk_files(root: Path) -> list[str]:
    """Fallback discovery for a tree that is not a git checkout."""
    found = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.is_symlink():
            continue
        if any(part in SKIP_DIRS for part in path.relative_to(root).parts):
            continue
        found.append(path.relative_to(root).as_posix())
    return found


def is_generated(relative: str) -> bool:
    return any(fnmatch.fnmatch(relative, pattern) for pattern in GENERATED)


@dataclasses.dataclass
class FileStat:
    path: str
    language: str
    kind: str
    counts: Counts
    generated: bool


def collect(root: Path, excludes: tuple[str, ...]) -> tuple[list[FileStat], list[str]]:
    """Per-file counts for the whole tree, plus the files that were skipped."""
    names = git_files(root)
    if names is None:
        names = walk_files(root)

    stats: list[FileStat] = []
    skipped: list[str] = []
    for relative in sorted(names):
        if any(fnmatch.fnmatch(relative, pattern) for pattern in excludes):
            continue
        path = root / relative
        if not path.is_file():  # A deleted-but-tracked file, or a submodule.
            continue
        found = identify(path)
        if found is None:
            skipped.append(relative)
            continue
        name, syntax = found
        text = read_text(path)
        if text is None:
            skipped.append(relative)
            continue
        counts = scan_prose(text) if syntax.prose else scan(text, syntax)
        stats.append(FileStat(relative, name, syntax.kind, counts, is_generated(relative)))
    return stats, skipped


def totals_by(stats: list[FileStat], key) -> dict[str, Counts]:
    grouped: dict[str, Counts] = {}
    for stat in stats:
        grouped.setdefault(key(stat), Counts()).add(stat.counts)
    return grouped


def number(value: int) -> str:
    return f"{value:,}"


def share(part: int, whole: int) -> str:
    return f"{100 * part / whole:5.1f}%" if whole else "    - "


def print_table(title: str, rows: list[tuple[str, Counts]], label: str, width: int) -> None:
    print(f"\n{title}")
    header = f"{label:<{width}} {'files':>6} {'code':>9} {'doc':>8} {'comment':>8} {'blank':>8} {'total':>9}  doc+cmt"
    print(header)
    print("-" * len(header))
    for name, counts in rows:
        print(
            f"{name[:width]:<{width}} {number(counts.files):>6} {number(counts.code):>9} "
            f"{number(counts.documentation):>8} {number(counts.comment):>8} "
            f"{number(counts.blank):>8} {number(counts.total):>9} "
            f"{share(counts.commented, counts.total)}"
        )


def print_files(stats: list[FileStat], sort: str, limit: int | None) -> None:
    order = {
        "code": lambda s: (s.counts.code, s.counts.total),
        "total": lambda s: (s.counts.total, s.counts.code),
        "doc": lambda s: (s.counts.documentation, s.counts.total),
        "comment": lambda s: (s.counts.commented, s.counts.total),
        "blank": lambda s: (s.counts.blank, s.counts.total),
    }[sort]
    ranked = sorted(stats, key=order, reverse=True)
    shown = ranked if limit is None else ranked[:limit]
    title = f"Files by {sort} lines, descending"
    if limit is not None and len(ranked) > limit:
        title += f" (top {limit} of {len(ranked)}; --all for every file)"
    width = max([len(s.path) for s in shown] + [len("file")])
    width = min(width, 60)

    print(f"\n{title}")
    header = (
        f"{'file':<{width}} {'language':<12} {'code':>8} {'doc':>7} "
        f"{'comment':>8} {'blank':>7} {'total':>8}"
    )
    print(header)
    print("-" * len(header))
    for stat in shown:
        path = stat.path if len(stat.path) <= width else "..." + stat.path[-(width - 3):]
        counts = stat.counts
        print(
            f"{path:<{width}} {stat.language[:12]:<12} {number(counts.code):>8} "
            f"{number(counts.documentation):>7} {number(counts.comment):>8} "
            f"{number(counts.blank):>7} {number(counts.total):>8}"
        )


def report(stats: list[FileStat], skipped: list[str], args: argparse.Namespace) -> None:
    kept = [s for s in stats if args.include_generated or not s.generated]
    generated = [s for s in stats if s.generated]
    if not kept:
        print("No countable files found.")
        return

    grand = Counts()
    for stat in kept:
        grand.add(stat.counts)

    print(f"Code statistics for {ROOT.name} ({ROOT})")
    print(f"{number(grand.files)} files, {number(grand.total)} lines")

    by_language = totals_by(kept, lambda s: s.language)
    kinds = {s.language: s.kind for s in kept}
    width = max([len(name) for name in by_language] + [len("language"), len("(subtotal)")])

    for kind, title in (
        (CODE, "Source code"),
        (MARKUP, "Markup"),
        (DATA, "Configuration and data"),
        (PROSE, "Documentation and prose"),
    ):
        rows = [(n, c) for n, c in by_language.items() if kinds[n] == kind]
        if not rows:
            continue
        rows.sort(key=lambda row: row[1].code + row[1].documentation, reverse=True)
        subtotal = Counts()
        for _, counts in rows:
            subtotal.add(counts)
        if len(rows) > 1:
            rows.append(("(subtotal)", subtotal))
        print_table(title, rows, "language", width)

    print_table("Everything", [("all files", grand)], "", width)

    if args.by_dir:
        by_dir = totals_by(kept, lambda s: s.path.split("/")[0] if "/" in s.path else "(root)")
        rows = sorted(by_dir.items(), key=lambda row: row[1].total, reverse=True)
        dir_width = max(max(len(name) for name, _ in rows), len("directory"))
        print_table("Top-level directories", rows, "directory", dir_width)

    print_files(kept, args.sort, None if args.all else args.top)

    if generated and not args.include_generated:
        counts = Counts()
        for stat in generated:
            counts.add(stat.counts)
        print(
            f"\nExcluded as generated or vendored: {number(counts.files)} files, "
            f"{number(counts.total)} lines (--include-generated to count them)."
        )
    if skipped and args.verbose:
        print(f"\nSkipped {number(len(skipped))} binary or unrecognised files:")
        for name in skipped:
            print(f"  {name}")
    elif skipped:
        print(f"Skipped {number(len(skipped))} binary or unrecognised files (--verbose to list).")


def as_json(stats: list[FileStat], skipped: list[str], args: argparse.Namespace) -> str:
    kept = [s for s in stats if args.include_generated or not s.generated]
    grand = Counts()
    for stat in kept:
        grand.add(stat.counts)
    payload = {
        "root": str(ROOT),
        "total": grand.as_dict(),
        "by_language": {
            name: counts.as_dict()
            for name, counts in sorted(
                totals_by(kept, lambda s: s.language).items(),
                key=lambda row: row[1].total,
                reverse=True,
            )
        },
        "by_kind": {
            kind: counts.as_dict() for kind, counts in totals_by(kept, lambda s: s.kind).items()
        },
        "files": [
            {"path": s.path, "language": s.language, "kind": s.kind, **s.counts.as_dict()}
            for s in sorted(kept, key=lambda s: s.counts.code, reverse=True)
        ],
        "skipped": skipped,
    }
    return json.dumps(payload, indent=2)


def main() -> int:
    global ROOT

    parser = argparse.ArgumentParser(
        description="Line-count statistics by language and by file.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "Lines are classified as code, documentation (doc comments, "
            "docstrings, prose), comment (everything else) or blank. A line "
            "with code and a trailing comment counts as code."
        ),
    )
    parser.add_argument(
        "path",
        nargs="?",
        type=Path,
        default=None,
        help="directory to measure (default: the repository root)",
    )
    parser.add_argument("--all", action="store_true", help="list every file, not just the largest")
    parser.add_argument("--top", type=int, default=25, help="how many files to list (default: 25)")
    parser.add_argument(
        "--sort",
        choices=("code", "total", "doc", "comment", "blank"),
        default="code",
        help="what to rank the per-file list by (default: code)",
    )
    parser.add_argument("--by-dir", action="store_true", help="add a top-level directory breakdown")
    parser.add_argument(
        "--include-generated",
        action="store_true",
        help="count lockfiles, vendored licences and src-tauri/gen/",
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        metavar="GLOB",
        help="skip paths matching this glob (repeatable)",
    )
    parser.add_argument("--json", action="store_true", help="print JSON instead of a report")
    parser.add_argument("--verbose", action="store_true", help="list the files that were skipped")
    args = parser.parse_args()

    ROOT = (args.path or ROOT).resolve()
    if not ROOT.is_dir():
        print(f"not a directory: {ROOT}", file=sys.stderr)
        return 2

    stats, skipped = collect(ROOT, tuple(args.exclude))
    if args.json:
        print(as_json(stats, skipped, args))
    else:
        report(stats, skipped, args)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except BrokenPipeError:
        # Piping into `head` closes the pipe early. Redirect what is left of
        # stdout so the interpreter's own flush at exit does not report it.
        os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        sys.exit(0)
    except KeyboardInterrupt:
        sys.exit(130)
