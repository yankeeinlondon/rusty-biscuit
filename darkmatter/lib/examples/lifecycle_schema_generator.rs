//! Generator for the generated lifecycle verb types in
//! `darkmatter/schemas/partials/lifecycle.yaml`.
//!
//! By default this prints each generated region between its marker lines.
//! With `--write` it replaces the lines between every
//! `# BEGIN GENERATED <region>` and `# END GENERATED <region>` pair in the
//! partial and leaves every other line as it is. Builds embed the checked-in
//! file; nothing regenerates it but this.
//!
//! Run with:
//!
//! ```text
//! cargo run -p darkmatter --example lifecycle_schema_generator
//! cargo run -p darkmatter --example lifecycle_schema_generator -- --write
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use darkmatter::lifecycle::schema_projection::{GeneratedRegion, LifecycleSchemaProjection};

fn main() -> ExitCode {
    let projection = match LifecycleSchemaProjection::shipped() {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("cannot project the lifecycle schema: {error}");
            return ExitCode::FAILURE;
        }
    };

    if !std::env::args().skip(1).any(|arg| arg == "--write") {
        for region in GeneratedRegion::ALL {
            println!("{}", region.begin_marker());
            print!("{}", projection.render_region(region));
            println!("{}", region.end_marker());
        }
        return ExitCode::SUCCESS;
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../schemas/partials/lifecycle.yaml");
    let current = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let rewritten = match projection.rewrite(&current) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot rewrite {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    if rewritten != current
        && let Err(error) = std::fs::write(&path, rewritten)
    {
        eprintln!("cannot write {}: {error}", path.display());
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
