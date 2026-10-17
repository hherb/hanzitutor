#!/usr/bin/env python3
"""Build the mnemonic JSONL from the authored source plus the raw datasets.

The authored layer is `data/mnemonics/zh-<level>.source.tsv`: one character per
line, tab separated:

    character <TAB> sound hook <TAB> meaning <TAB> mnemonic

Only the four fields a human wrote live there. Everything mechanical — pinyin,
frequency rank, HSK level, the IDS decomposition, the component characters and
their glosses — is joined in from `data/raw/`, so the source file can be edited
and diffed without carrying a copy of the upstream data.

Run from the repository root:

    python3 scripts/build-mnemonics.py

For every level listed in `LEVELS` it writes `data/mnemonics/zh-<level>.jsonl`
(the machine-readable set) and `data/mnemonics/zh-<level>.preview.md` (the same
records as a table, for review), then `data/mnemonics/mnemonics.json` — the
combined lookup the app bundles. It exits non-zero when a source file is
malformed or names a character that is not in that level's set.
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW = ROOT / "data" / "raw"
MNE = ROOT / "data" / "mnemonics"
COMBINED = MNE / "mnemonics.json"

# (level label, hanziDB `hsk_level` value). Adding a level means a line here and
# a `zh-<level>.source.tsv` beside the others; the coverage check then does the
# rest.
LEVELS = (("hsk1", "1"), ("hsk2", "2"))

LICENSE = "CC-BY-SA-4.0"
SOURCE_KIND = "original"
GENERATOR = "deepseek-flash"
GENERATED = "2026-10-17"

# Shown above every review sheet and, in shorter form, in the app. The English
# approximation in the `sound` column is a memory hook and nothing else: it must
# never be presented as how the character is pronounced.
SOUND_NOTE = (
    "The `sound` column is an English memory hook, not a pronunciation guide. "
    "It only has to be close enough to trigger the memory — read the pinyin for "
    "the actual sound."
)

# IDS layout operators. Anything else in a decomposition string is a component.
IDS_OPERATORS = set("⿰⿱⿲⿳⿴⿵⿶⿷⿸⿹⿺⿻⿼⿽⿾⿿")

# Components whose Make Me a Hanzi definition is a philological aside rather
# than a usable image. Everything not named here falls back to the upstream
# definition, so this table should stay short.
CLEAN_GLOSS = {
    "丷": "two dots",
    "乛": "hook",
    "乚": "hook",
    "亅": "hook",
    "丨": "line",
    "丿": "slash",
    "丶": "dot",
    "⺊": "crack",
    "⺍": "three sparks",
    "疋": "foot",
    "未": "wheat",
    "冉": "fish",
    "冋": "far",
    "爫": "hand",
    "厶": "private",
    "匕": "spoon",
    "又": "hand",
    "阝": "city",
    "卩": "kneeling person",
    "彳": "step",
    "冫": "ice",
    "灬": "fire",
    "艹": "grass",
    "讠": "speech",
    "亻": "person",
    "氵": "water",
    "扌": "hand",
    "钅": "metal",
    "饣": "food",
    "犭": "animal",
    "糸": "thread",
    "礻": "altar",
    "宀": "roof",
    "冖": "cover",
    "广": "building",
    "辶": "walk",
    "囗": "border",
    "凵": "container",
    "屮": "sprout",
    "禾": "grain",
    "玉": "jade",
    "王": "jade",
    "目": "eye",
    "舌": "tongue",
    "手": "hand",
    "心": "heart",
    "止": "foot",
    "走": "walk",
    "见": "see",
    "儿": "legs",
    "大": "big person",
    "女": "woman",
    "子": "child",
    "人": "person",
    "八": "divide",
    "刀": "knife",
    "口": "mouth",
    "日": "sun",
    "月": "moon",
    "木": "tree",
    "土": "earth",
    "雨": "rain",
    "门": "door",
    "马": "horse",
    "隹": "bird",
    "羊": "sheep",
    "车": "cart",
    "戈": "halberd",
    "矢": "arrow",
    "斤": "axe",
    "十": "ten",
    "一": "one",
    "二": "two",
    "三": "three",
    "亠": "lid",
    "⺼": "flesh",
    "夬": "divide",
    "曷": "question",
}


def source_path(level: str) -> Path:
    return MNE / f"zh-{level}.source.tsv"


def out_path(level: str) -> Path:
    return MNE / f"zh-{level}.jsonl"


def preview_path(level: str) -> Path:
    return MNE / f"zh-{level}.preview.md"


def load_mmah() -> dict[str, dict]:
    """Make Me a Hanzi dictionary.txt, keyed by character."""
    out: dict[str, dict] = {}
    with (RAW / "dictionary.txt").open(encoding="utf-8") as fh:
        for line in fh:
            obj = json.loads(line)
            out[obj["character"]] = obj
    return out


def load_hanzi_db() -> tuple[dict[str, dict], dict[str, list[str]]]:
    """hanziDB.csv rows keyed by character, plus each level's de-duplicated list.

    The CSV repeats rows — 57 of the HSK 1 characters appear twice — so the level
    lists are built with a seen-set rather than by counting rows.
    """
    rows = list(csv.DictReader((RAW / "hanziDB.csv").open(encoding="utf-8")))
    by_char = {r["charcter"]: r for r in rows}
    levels: dict[str, list[str]] = {}
    seen: dict[str, set[str]] = {}
    for r in rows:
        label = (r.get("hsk_level") or "").strip()
        ch = r["charcter"]
        if not label or not ch:
            continue
        level = f"hsk{label}"
        levels.setdefault(level, [])
        seen.setdefault(level, set())
        if ch not in seen[level]:
            seen[level].add(ch)
            levels[level].append(ch)
    return by_char, levels


def component_gloss(ch: str, mmah: dict[str, dict]) -> str:
    if ch in CLEAN_GLOSS:
        return CLEAN_GLOSS[ch]
    definition = (mmah.get(ch, {}).get("definition") or "").strip()
    if not definition:
        return ""
    # First sense only: "mouth; entrance, gate, opening" -> "mouth".
    head = definition.split(";")[0].split(",")[0].strip()
    return head[:32]


def components_of(decomposition: str, mmah: dict[str, dict]) -> list[dict]:
    parts: list[str] = []
    for ch in decomposition:
        if ch in IDS_OPERATORS or ch == "？":
            continue
        if ch not in parts:
            parts.append(ch)
    return [
        {
            "char": p,
            "gloss": component_gloss(p, mmah),
            "definition": (mmah.get(p, {}).get("definition") or "").split(";")[0].strip()[:64],
        }
        for p in parts
    ]


def read_source(path: Path) -> list[tuple[int, str, str, str, str]]:
    records: list[tuple[int, str, str, str, str]] = []
    seen: set[str] = set()
    errors: list[str] = []
    with path.open(encoding="utf-8") as fh:
        for lineno, raw in enumerate(fh, start=1):
            line = raw.rstrip("\n")
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            fields = line.split("\t")
            if len(fields) != 4:
                errors.append(f"{path.name}:{lineno}: expected 4 tab-separated fields, got {len(fields)}")
                continue
            ch, sound, meaning, mnemonic = (f.strip() for f in fields)
            if len(ch) != 1:
                errors.append(f"{path.name}:{lineno}: expected one character, got {ch!r}")
                continue
            if ch in seen:
                errors.append(f"{path.name}:{lineno}: duplicate character {ch}")
                continue
            if not meaning:
                errors.append(f"{path.name}:{lineno}: empty meaning for {ch}")
                continue
            if not 10 <= len(mnemonic) <= 400:
                errors.append(f"{path.name}:{lineno}: mnemonic for {ch} is {len(mnemonic)} characters")
                continue
            seen.add(ch)
            records.append((lineno, ch, sound, meaning, mnemonic))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        raise SystemExit(1)
    return records


def build_level(
    level: str,
    expected: list[str],
    mmah: dict[str, dict],
    by_char: dict[str, dict],
) -> list[dict]:
    path = source_path(level)
    if not path.exists():
        print(f"skip {level}: {path.name} is missing")
        return []
    records = read_source(path)

    expected_set = set(expected)
    stray = [ch for _, ch, _, _, _ in records if ch not in expected_set]
    if stray:
        print(f"{level}: characters not in the {level} set: {''.join(stray)}", file=sys.stderr)
        raise SystemExit(1)

    order = {ch: i for i, ch in enumerate(expected)}
    records.sort(key=lambda r: order[r[1]])

    rows: list[dict] = []
    for _, ch, sound, meaning, mnemonic in records:
        meta = by_char[ch]
        decomposition = (mmah.get(ch, {}).get("decomposition") or "").strip()
        rows.append(
            {
                "char": ch,
                "pinyin": meta["pinyin"],
                "meaning": meaning,
                "sound": sound,
                "mnemonic": mnemonic,
                "decomposition": decomposition,
                "components": components_of(decomposition, mmah),
                "level": level,
                "rank": int(meta["frequency_rank"]),
                "license": LICENSE,
                "source": SOURCE_KIND,
                "generator": GENERATOR,
                "generated": GENERATED,
                "reviewed": False,
            }
        )

    with out_path(level).open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")

    with preview_path(level).open("w", encoding="utf-8") as fh:
        label = level.upper().replace("HSK", "HSK ")
        fh.write(f"# {label} mnemonics — review sheet\n\n")
        fh.write(f"> **Not a pronunciation guide.** {SOUND_NOTE}\n\n")
        fh.write(
            f"{len(rows)} of {len(expected)} {label} characters. "
            f"Generated draft; edit `{path.name}`, not this file.\n\n"
        )
        fh.write("| # | char | pinyin | meaning | sound | mnemonic |\n")
        fh.write("|---|------|--------|---------|-------|----------|\n")
        for i, row in enumerate(rows, start=1):
            mnemonic = row["mnemonic"].replace("|", "\\|")
            fh.write(
                f"| {i} | {row['char']} | {row['pinyin']} | {row['meaning']} | "
                f"{row['sound']} | {mnemonic} |\n"
            )

    missing = [ch for ch in expected if ch not in {r["char"] for r in rows}]
    print(f"wrote {out_path(level).relative_to(ROOT)}: {len(rows)} records")
    print(f"wrote {preview_path(level).relative_to(ROOT)}")
    print(f"{level.upper()} coverage: {len(rows)}/{len(expected)}")
    if missing:
        print(f"still missing ({len(missing)}): {''.join(missing)}")
    return rows


def main() -> None:
    mmah = load_mmah()
    by_char, levels = load_hanzi_db()

    combined: dict[str, dict] = {}
    for level, _db_value in LEVELS:
        rows = build_level(level, levels.get(level, []), mmah, by_char)
        for row in rows:
            combined[row["char"]] = {
                "level": row["level"],
                "sound": row["sound"],
                "mnemonic": row["mnemonic"],
            }

    with COMBINED.open("w", encoding="utf-8") as fh:
        json.dump(combined, fh, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
        fh.write("\n")
    print(f"wrote {COMBINED.relative_to(ROOT)}: {len(combined)} characters")

    if not combined:
        print("no mnemonic records were built", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
