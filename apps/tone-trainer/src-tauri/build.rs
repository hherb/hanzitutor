fn main() {
    // The character dataset is generated rather than committed, and this app
    // embeds it the same way the full app does. Checking here turns a cryptic
    // `include_bytes!` failure into instructions.
    let artifact = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/hanzi-core/data/hanzi.bin.gz");
    println!("cargo:rerun-if-changed={}", artifact.display());
    if !artifact.exists() {
        panic!(
            "\n\nThe character dataset has not been generated yet.\n\
             Expected it at:\n  {}\n\n\
             Build it with:\n  ./scripts/fetch-data.sh\n  pnpm run prepare-data\n\n",
            artifact.display()
        );
    }
    tauri_build::build()
}
