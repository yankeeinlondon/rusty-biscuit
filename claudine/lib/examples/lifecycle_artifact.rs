//! Generator for Claudine's host lifecycle artifact,
//! `claudine/schemas/globals.lifecycle.json`, which DMLS reads to type
//! Claudine lifecycle stacks with Claudine's event registry and parse options.
//!
//! By default this prints the artifact. With `--write` it replaces the
//! committed file. A test in `claudine/lib/tests/l1/host_catalog.rs` fails
//! when the two differ.
//!
//! Run with:
//!
//! ```text
//! cargo run -p claudine --example lifecycle_artifact
//! cargo run -p claudine --example lifecycle_artifact -- --write
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let artifact = claudine::composition::lifecycle::lifecycle_artifact();
    if !std::env::args().skip(1).any(|arg| arg == "--write") {
        print!("{artifact}");
        return ExitCode::SUCCESS;
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../schemas/globals.lifecycle.json");
    if std::fs::read_to_string(&path).is_ok_and(|current| current == artifact) {
        return ExitCode::SUCCESS;
    }
    if let Err(error) = std::fs::write(&path, artifact) {
        eprintln!("cannot write {}: {error}", path.display());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
