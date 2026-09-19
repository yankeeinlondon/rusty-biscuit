//! Emits the build-script outputs the archive has to carry.
//!
//! Two distinct classes, because they fail differently under relocation:
//!
//! * `generated.rs` is `include!`d by the library, so it is baked into every
//!   compiled binary and travels for free. It is here as the control.
//! * `build-script-asset.txt` is read at *run* time from the build script's
//!   `OUT_DIR`. Nothing hands a relocated test that path, so the fixture
//!   re-exports the directory through `cargo::rustc-link-search`, which nextest
//!   archives as a linked path and republishes to the consumer's dynamic-library
//!   search path.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));

    fs::write(
        out_dir.join("generated.rs"),
        "pub const GENERATED_MARKER: &str = \"build-script-generated\";\n",
    )
    .expect("writing the generated source");

    fs::write(out_dir.join("build-script-asset.txt"), "build-script-asset\n")
        .expect("writing the build-script runtime asset");

    println!("cargo::rustc-link-search=native={}", out_dir.display());
    println!("cargo::rerun-if-changed=build.rs");
}
