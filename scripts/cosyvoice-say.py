#!/usr/bin/env python3
"""Synthesise one phrase with CosyVoice 3, including pinyin pronunciation control.

This is a **build-time** tool. It is the only Python in the audio pipeline, and it
is here because the model's own runtime is Python — CosyVoice 3 is distributed as
a Python package with a submodule dependency, not as an ONNX graph the Rust
sherpa-onnx runtime can execute the way MeloTTS is. Nothing at runtime depends on
this file; the on-device synthesiser stays in Rust (`src-tauri/src/say.rs`).

## Why CosyVoice 3 is used for the bundled clips

MeloTTS mispronounces the first syllable of roughly one HSK-1 phrase in five.
Measured over 208 phrases with the same recogniser and judge, MeloTTS failed 57
and CosyVoice2 failed 15; isolating the recogniser's own errors leaves 46 phrases
that only MeloTTS gets wrong. See
`docs/research/MELOTTS_PRONUNCIATION_ACCURACY.md`.

CosyVoice 3 can be **told** the pronunciation. It tokenises Chinese pinyin as
control tokens — every initial (`[q]`, `[zh]`), final (`[ing]`, `[üe]`) and
tone-marked vowel (`[ǐ]`, `[ǚ]`) — so a syllable can be forced by writing it into
the text:

    报道[j][ǐ]予好评        pins 给's reading to jǐ

That turns a defect from something to avoid into something to fix, without
touching the sentence the learner reads.

## Invocation

Two modes, because the caller has a per-phrase override and shelling out per
phrase with a fresh model load would be absurd:

    cosyvoice-say.py --list <requests.jsonl> --out-dir <dir>

Each request line:  {"id": "...", "text": "...", "speed": 1.0, "syllables": null}

`syllables`, when present, is a list of pinyin strings to splice into the text for
the characters that were mispronounced — the inpainting above. The caller decides
which; this script only applies them.

Writes `<out-dir>/<id>.wav` per request and a `<out-dir>/result.json` summary.
"""

import argparse
import json
import os
import sys


def build_text(text: str, syllables) -> str:
    """Splice pinyin control tokens into `text`.

    `syllables` is a list parallel to the characters of `text`, with `None` where
    the model's own reading is kept. Kept deliberately dumb: the caller decides
    what to override and this only inserts the brackets the tokenizer expects.
    Working out *which* character was mispronounced is a judgement the acoustic
    evidence makes, not something to infer from a string here.

    A length mismatch is refused rather than tolerated. `zip` would silently stop
    at the shorter of the two, which drops the rest of the sentence — audio that
    quietly ends early, which is the exact class of defect this whole exercise is
    about.
    """
    if not syllables:
        return text
    if len(syllables) != len(text):
        raise ValueError(
            f"syllables has {len(syllables)} entries for {len(text)} characters "
            f"({text!r}); it must be parallel to the text"
        )
    out = []
    for ch, override in zip(text, syllables):
        if override:
            # `[j][ǐ]` — the tokenizer wants the initial and the tone-marked
            # final as separate brackets, so the caller supplies e.g. "j ǐ".
            parts = str(override).split()
            out.append("".join(f"[{p}]" for p in parts))
        else:
            out.append(ch)
    return "".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--model-dir", required=True,
                    help="CosyVoice model directory, e.g. pretrained_models/Fun-CosyVoice3-0.5B")
    ap.add_argument("--repo", required=True, help="the cloned CosyVoice checkout")
    ap.add_argument("--list", dest="requests", required=True,
                    help="JSONL of {id, text, speed, syllables}")
    ap.add_argument("--prompt-wav", required=True,
                    help="reference clip for zero-shot cloning")
    ap.add_argument("--prompt-text", default="You are a helpful assistant.<|endofprompt|>",
                    help="the transcript of the prompt clip, including the chat prefix")
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()

    # The model's own package tree, and its Matcha-TTS submodule, are imported
    # from the checkout rather than installed.
    sys.path.insert(0, os.path.join(args.repo, "third_party", "Matcha-TTS"))
    sys.path.insert(0, args.repo)

    try:
        import torchaudio
        from cosyvoice.cli.cosyvoice import AutoModel
    except Exception as e:  # noqa: BLE001 - the message is the point
        print(
            f"error: could not import CosyVoice: {e}\n"
            "       This needs its own environment. It pins torch==2.3.1,\n"
            "       numpy==1.26.4 and transformers==4.51.3, which will not\n"
            "       coexist with a newer torch — see docs/research/\n"
            "       MELOTTS_PRONUNCIATION_ACCURACY.md, 'Running this'.",
            file=sys.stderr,
        )
        return 1

    model = AutoModel(model_dir=args.model_dir)
    os.makedirs(args.out_dir, exist_ok=True)

    requests = []
    with open(args.requests, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                requests.append(json.loads(line))
    if args.limit:
        requests = requests[: args.limit]

    # Resumable: each clip is written under its final name as soon as it exists,
    # and the progress file is rewritten after every one. A run stopped after
    # 400 of 1,600 items therefore costs the item in flight, not the whole pass —
    # which is what an earlier version got wrong, writing one manifest at the very
    # end and so throwing away hours on a Ctrl-C.
    progress_path = os.path.join(args.out_dir, "progress.json")
    done = {}
    if os.path.exists(progress_path):
        try:
            with open(progress_path, encoding="utf-8") as f:
                done = {row["id"]: row for row in json.load(f).get("done", [])}
        except Exception:
            done = {}

    def save_progress():
        tmp = progress_path + ".partial"
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump({"model": args.model_dir, "done": list(done.values())}, f,
                      ensure_ascii=False)
        # Renamed into place so an interruption cannot leave a half-written
        # progress file that a re-run would then trust.
        os.replace(tmp, progress_path)

    results = []
    skipped = 0
    for n, request in enumerate(requests, 1):
        text = build_text(request["text"], request.get("syllables"))
        speed = float(request.get("speed", 1.0))

        # Skip what a previous run finished *and whose file is still there*. The
        # artifact is checked as well as the bookkeeping: a truncated WAV is not
        # a finished one, and trusting the record alone would ship silence.
        previous = done.get(request["id"])
        if previous and os.path.exists(previous["wav"]) and os.path.getsize(previous["wav"]) > 1024:
            results.append(previous)
            skipped += 1
            continue
        pieces = []
        # CosyVoice returns a generator of chunks; concatenate them.
        for chunk in model.inference_zero_shot(
            text, args.prompt_text, args.prompt_wav, stream=False, speed=speed
        ):
            pieces.append(chunk["tts_speech"])
        wav = pieces[0] if len(pieces) == 1 else torchaudio.functional.cat(pieces)  # noqa: F821
        dest = os.path.join(args.out_dir, f"{request['id']}.wav")
        torchaudio.save(dest, wav, model.sample_rate)
        row = {"id": request["id"], "text": text, "wav": dest}
        results.append(row)
        done[request["id"]] = row
        save_progress()
        print(f"  [{n}/{len(requests)}] {request['id']}  {text}", flush=True)

    with open(os.path.join(args.out_dir, "result.json"), "w", encoding="utf-8") as f:
        json.dump({"model": args.model_dir, "phrases": results}, f, ensure_ascii=False, indent=1)
    if skipped:
        print(f"\nresumed: {skipped} already present, {len(results) - skipped} new")
    print(f"{len(results)} clips into {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
