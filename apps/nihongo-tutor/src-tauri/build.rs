fn main() {
    // The kana artifact is committed, unlike the Chinese one, but a clone that
    // deleted it or a `prepare-kana` that failed would otherwise surface as a
    // cryptic `include_bytes!` error. Checking here turns that into instructions.
    let artifact = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/nihongo-core/data/kana.bin.gz");
    println!("cargo:rerun-if-changed={}", artifact.display());
    if !artifact.exists() {
        panic!(
            "\n\nThe kana dataset is missing.\n\
             Expected it at:\n  {}\n\n\
             It is committed, so this usually means it was deleted. Regenerate it with:\n\
             \x20 ./scripts/fetch-data.sh\n  pnpm run prepare-kana\n\n",
            artifact.display()
        );
    }
    tauri_build::build()
}
