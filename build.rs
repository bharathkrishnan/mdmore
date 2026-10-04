use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Select the existing theme at build time rather than decompressing and
    // deserializing every bundled theme when the first code block is reached.
    let themes = syntect::highlighting::ThemeSet::load_defaults();
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"))
        .join("base16-ocean.dark.bin");
    syntect::dumps::dump_to_uncompressed_file(&themes.themes["base16-ocean.dark"], path)
        .expect("serialize bundled syntax theme");
}
