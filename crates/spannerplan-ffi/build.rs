fn main() {
    let crate_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let header_path = out_dir.join("spannerplan.h");

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("SPANNERPLAN_H")
        .generate()
        .expect("generate C header")
        .write_to_file(header_path);
}
