//! The `test-fixtures` record of this module's file-system access.
//!
//! Every file-system call the publisher and the legacy cleanup make is
//! preceded by [`note`]. In a `test-fixtures` build, when
//! `CLAUDINE_TEST_REGISTRATION_ACCESS` names a file, each call appends one line,
//! `<operation>\t<path>`, so a CLI test can prove from outside the process
//! that an ordinary command never reads (or writes) the published bundle,
//! and that the publishing routes do. Without the feature [`note`] does
//! nothing and reads no environment.

use std::path::Path;

/// The file a `test-fixtures` build appends one line to per file-system
/// call of this module.
#[cfg(feature = "test-fixtures")]
const TEST_REGISTRATION_ACCESS_ENV: &str = "CLAUDINE_TEST_REGISTRATION_ACCESS";

/// Records that `operation` (`inspect`, `read`, `list`, `lock`, `create`,
/// `write`, `remove`) is about to touch `path`.
pub(super) fn note(operation: &str, path: &Path) {
    #[cfg(feature = "test-fixtures")]
    if let Ok(log) = std::env::var(TEST_REGISTRATION_ACCESS_ENV) {
        use std::io::Write;
        // A test reads the file after the process exits; a failed append
        // shows up there as a missing line.
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .and_then(|mut file| writeln!(file, "{operation}\t{}", path.display()));
    }
    #[cfg(not(feature = "test-fixtures"))]
    let _ = (operation, path);
}
