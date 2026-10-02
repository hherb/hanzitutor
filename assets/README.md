# Artwork and screenshots

The image sets the store listings and the website are built from.

**Nothing in this repository reads them.** There is no build step that consumes
`assets/` — the sets are maintained by hand and uploaded by hand — so nothing
here will break if they change, and equally nothing here will notice if they go
stale. That is why this file exists: it is the only record of what is in them.

## What is where

```
assets/
  showcase/            captures taken from the devices, grouped by device
    Android/phone/     19
    Android/tablet/    16
    iPad/              23
    iPhone/             3
    (and 4 at the top level: the welcome page, the stroke-and-tone board,
     HSK words, radicals)
  website/             12 — the set hherb.com shows
```

The device folders hold the raw captures, named as the device named them
(`Screenshot_20260927_111512.jpg`). The top level of `showcase/` and all of
`website/` hold the curated shots, named for the feature they show.

## Where they go

- **`assets/showcase/`** — the store listings. It is grouped by device, which is
  what the listing requirements want: the App Store and Play ask for different
  sizes, and iPhone, iPad, Android phone and Android tablet are separate sets in
  both.
- **`assets/website/`** — the screenshots on **hherb.com**, where Hanzi Tutor is
  showcased.

**Taking these on trust:** the repository itself references neither directory.
`README.md` points its own images at `docs/screenshots/`, the Play listing's
artwork lives in `store/`, and outside this file the commit that added the two
directories is the only place either name appears. The mapping above is the
maintainer's, recorded here because it is nowhere else.

## Where the other images live

Three directories hold screenshots and it is worth knowing which is which:

| Directory | What it is |
| --- | --- |
| `assets/showcase/`, `assets/website/` | The source artwork, above. Not read by anything. |
| `docs/screenshots/` | The three captures `README.md` embeds. |
| `store/` | The Play listing: the feature graphic, `phone-screenshots/`, and the listing copy. |

## From a capture to a listing image

`store/phone-screenshots/` holds the three phone shots the Play listing uses.
They are produced from a raw capture by `scripts/make-store-assets.py`, which
draws the frame and the caption:

```bash
python3 scripts/make-store-assets.py screenshot raw.png store/phone-screenshots/01-name.png
python3 scripts/make-store-assets.py feature-graphic store/feature-graphic-1024x500.png
```

The script only formats: it takes its source as an argument and does **not** read
`assets/`. So which capture becomes which listing image is a decision made by
hand at the time, and is not recorded anywhere. If that matters later, naming the
source beside the output would be the cheap fix.

## Notes

- **`assets/website/` is a copy, not a distinct set.** All twelve of its files
  also exist inside `assets/showcase/`, and byte for byte — checked by hash, not
  by name. That is 10.0 MB carried twice in the repository. It may well be
  deliberate: a flat, curated twelve is easier to upload from than fishing them
  out of four device folders. But it is worth knowing that the website set is a
  selection of the showcase set rather than material of its own, because that is
  the kind of thing that stops being true silently.
- **Some file names have typos**, and they are the names the listings were
  uploaded under: `character_dictonary.png`, `setings_dropbox.png`, and
  `01_character _drawn_wrong.png`, which has a space before the underscore.
  Anything scripting over these has to match them as they are.
- **`.DS_Store` is not committed.** `.gitignore` covers it, and the four under
  `assets/` — here, in `showcase/`, `showcase/Android/` and `showcase/iPad/` —
  are untracked. The counts above are images only.
- **About 51 MB in 77 image files**, the largest a little over 1 MB, so this does
  not need Git LFS.
