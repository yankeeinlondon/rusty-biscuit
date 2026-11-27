//! The `test-fixtures` record of every local source file opened through
//! the shared reader (`simplified_schema::resolve::buffers::open_source`),
//! and of every effective schema prepared rather than reused
//! (`darkmatter::markdown::schemas::observe_preparations`).
//!
//! In a `test-fixtures` build, when `CLAUDINE_TEST_SOURCE_OPENS` names a
//! file, [`install`] makes each open append one line, the opened path, so a
//! CLI test can prove from outside the process how many times a dry run
//! opened a prompt, an inclusion, or a schema file; when
//! `CLAUDINE_TEST_SCHEMA_PREPARATIONS` names one, each preparation appends
//! the prepared document's path (empty for a document with none). Without
//! the feature [`install`] does nothing and reads no environment.

/// The file a `test-fixtures` build appends one line to per source open.
#[cfg(feature = "test-fixtures")]
const TEST_SOURCE_OPENS_ENV: &str = "CLAUDINE_TEST_SOURCE_OPENS";

/// The file a `test-fixtures` build appends one line to per effective
/// schema preparation.
#[cfg(feature = "test-fixtures")]
const TEST_SCHEMA_PREPARATIONS_ENV: &str = "CLAUDINE_TEST_SCHEMA_PREPARATIONS";

/// Installs both records for the rest of the process.
pub fn install() {
    #[cfg(feature = "test-fixtures")]
    {
        simplified_schema::resolve::buffers::observe_opens(|path| append(TEST_SOURCE_OPENS_ENV, path));
        darkmatter::markdown::schemas::observe_preparations(|document| {
            append(TEST_SCHEMA_PREPARATIONS_ENV, document.unwrap_or(std::path::Path::new("")));
        });
    }
}

/// Appends `path` as one line to the file `variable` names, when it names
/// one.
#[cfg(feature = "test-fixtures")]
fn append(variable: &str, path: &std::path::Path) {
    use std::io::Write;
    if let Ok(log) = std::env::var(variable) {
        // A test reads the file after the process exits; a failed append
        // shows up there as a missing line.
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .and_then(|mut file| writeln!(file, "{}", path.display()));
    }
}
