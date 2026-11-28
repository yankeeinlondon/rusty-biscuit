//! The `test-fixtures` record of every local source file `md` opens through
//! the shared reader (`simplified_schema::resolve::buffers::open_source`).
//!
//! In a `test-fixtures` build, when `MD_TEST_SOURCE_OPENS` names a file,
//! [`install`] makes each open append one line, the opened path, so a CLI
//! test can count from outside the process how many times one invocation
//! opened an included file. Without the feature [`install`] does nothing and
//! reads no environment.

/// The file a `test-fixtures` build appends one line to per source open.
#[cfg(feature = "test-fixtures")]
const TEST_SOURCE_OPENS_ENV: &str = "MD_TEST_SOURCE_OPENS";

/// Installs the open record for the rest of the process.
pub fn install() {
    #[cfg(feature = "test-fixtures")]
    simplified_schema::resolve::buffers::observe_opens(|path| {
        use std::io::Write;
        if let Ok(log) = std::env::var(TEST_SOURCE_OPENS_ENV) {
            // A test reads the file after the process exits; a failed
            // append shows up there as a missing line.
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
                .and_then(|mut file| writeln!(file, "{}", path.display()));
        }
    });
}
