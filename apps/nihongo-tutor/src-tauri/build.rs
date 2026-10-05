fn main() {
    // The kana and the kanji artifacts are committed, unlike the Chinese one, but
    // a clone that deleted one — or a `prepare-*` that failed — would otherwise
    // surface as a cryptic `include_bytes!` error. Checking here turns that into
    // instructions.
    //
    // All six are checked because all six are read: the kana course has always
    // drawn from one, and the other five carry the character course, the
    // vocabulary, the passages, the graded phrases and the grammar. The kanji
    // artifact sat in the repository unread for a whole milestone, which is exactly
    // the state in which nobody notices a dataset going missing — and the two
    // artifacts N18 and N19 added are the ones a clone is likeliest to be missing,
    // because they are the ones with a `prepare-*` of their own.
    let data =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../crates/nihongo-core/data");
    let artifacts = [
        (
            "kana.bin.gz",
            "./scripts/fetch-data.sh\n  pnpm run prepare-kana",
        ),
        (
            "kanji.bin.gz",
            "./scripts/fetch-data.sh\n  pnpm run prepare-kanji",
        ),
        (
            "words.bin.gz",
            "./scripts/fetch-data.sh\n  pnpm run prepare-words",
        ),
        (
            "passages.bin.gz",
            "./scripts/fetch-data.sh\n  pnpm run prepare-passages",
        ),
        (
            "phrases.bin.gz",
            "./scripts/fetch-tatoeba.sh\n  pnpm run prepare-phrases",
        ),
        (
            "grammar.bin.gz",
            "./scripts/fetch-openjlpt.sh\n  pnpm run prepare-grammar",
        ),
    ];

    for (name, recipe) in artifacts {
        let artifact = data.join(name);
        println!("cargo:rerun-if-changed={}", artifact.display());
        if !artifact.exists() {
            panic!(
                "\n\nThe {name} dataset is missing.\n\
                 Expected it at:\n  {}\n\n\
                 It is committed, so this usually means it was deleted. Regenerate it with:\n\
                 \x20 {recipe}\n\n",
                artifact.display()
            );
        }
    }

    tauri_build::build()
}
