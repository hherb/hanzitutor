fn main() {
    // The kana and the kanji artifacts are committed, unlike the Chinese one, but
    // a clone that deleted one — or a `prepare-*` that failed — would otherwise
    // surface as a cryptic `include_bytes!` error. Checking here turns that into
    // instructions.
    //
    // Both are checked because both are read: the kana course has always drawn
    // from one, and the kanji screens now draw from the other. The kanji artifact
    // sat in the repository unread for a whole milestone, which is exactly the
    // state in which nobody notices it going missing.
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
