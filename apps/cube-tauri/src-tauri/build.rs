fn main() {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tables/manifest.bin");
    if !manifest.is_file() {
        println!(
            "cargo:warning=two-phase tables are not at {} — run `cargo run --release --bin cube-cli -- gen-tables` from the repo root before `cargo tauri build`",
            manifest.display()
        );
    }
    println!("cargo:rerun-if-changed=../../../tables/manifest.bin");
    tauri_build::build()
}
