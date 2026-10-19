//! The executable a template-mode command runs, resolved so that its filled
//! arguments reach a native process unchanged.
//!
//! Template mode passes each filled argument as exactly one argument, so the
//! program must not be one an interpreter re-parses the arguments for. A
//! target that would need an interpreter is refused before anything spawns;
//! nothing falls back to a shell.
//!
//! - **Windows:** only a resolved `.exe` or `.com`. A `.cmd`, a `.bat`, or any
//!   other `PATHEXT` hit is refused, because the standard library runs a batch
//!   file through `cmd.exe`, which re-parses its arguments.
//! - **Unix:** the resolved file must begin with a native executable's magic
//!   number (ELF, Mach-O, or a universal Mach-O) or with `#!`. A file with
//!   neither is refused, because `execvp`-style launching falls back to
//!   `/bin/sh` for it. A `#!` script is accepted: the kernel hands it its
//!   argument vector unchanged.
//!
//! A shell builtin or alias is not on `PATH`, so it is refused as not found.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Why a template-mode program cannot be run directly.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateTargetError {
    /// No file of that name on the search path (a shell builtin or alias
    /// included).
    #[error("'{program}' is not an executable on PATH; template commands never run through a shell")]
    NotFound { program: String },
    /// A Windows target other than `.exe`/`.com`, which would run through an
    /// interpreter.
    #[error("'{}' needs an interpreter (a .{extension} file); template commands run only native executables", path.display())]
    Interpreter { path: PathBuf, extension: String },
    /// A Unix file that is neither a native executable nor a `#!` script.
    #[error("'{}' is neither a native executable nor a #! script; template commands never fall back to /bin/sh", path.display())]
    NotNative { path: PathBuf },
    /// The resolved file could not be read to check its format.
    #[error("'{}' could not be read to check its format: {message}", path.display())]
    Unreadable { path: PathBuf, message: String },
}

/// Resolves `program` for direct dispatch, as the command executor would find
/// it, and checks that it can run without an interpreter.
///
/// `path` is the search path the child would use (`None` for this process's
/// `PATH`), and `cwd` the directory a relative program is resolved from.
///
/// ## Errors
///
/// A [`TemplateTargetError`] for a program not found, or one that would run
/// through an interpreter.
pub fn resolve_direct_target(program: &str, path: Option<&OsStr>, cwd: &Path) -> Result<PathBuf, TemplateTargetError> {
    let found = match path {
        Some(path) => which::which_in(program, Some(path), cwd),
        None => which::which(program),
    }
    .map_err(|_| TemplateTargetError::NotFound {
        program: program.to_string(),
    })?;
    check_native(&found)?;
    Ok(found)
}

#[cfg(windows)]
fn check_native(path: &Path) -> Result<(), TemplateTargetError> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if extension == "exe" || extension == "com" {
        return Ok(());
    }
    Err(TemplateTargetError::Interpreter {
        path: path.to_path_buf(),
        extension,
    })
}

#[cfg(not(windows))]
fn check_native(path: &Path) -> Result<(), TemplateTargetError> {
    use std::io::Read as _;

    let mut magic = [0u8; 4];
    let read = std::fs::File::open(path)
        .and_then(|mut file| {
            let mut filled = 0;
            while filled < magic.len() {
                match file.read(&mut magic[filled..])? {
                    0 => break,
                    count => filled += count,
                }
            }
            Ok(filled)
        })
        .map_err(|error| TemplateTargetError::Unreadable {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    if is_native_magic(&magic[..read]) {
        Ok(())
    } else {
        Err(TemplateTargetError::NotNative {
            path: path.to_path_buf(),
        })
    }
}

/// Whether `head`, a file's first bytes, starts a format the kernel executes
/// without a shell.
#[cfg_attr(windows, allow(dead_code))]
fn is_native_magic(head: &[u8]) -> bool {
    const MAGICS: &[[u8; 4]] = &[
        // ELF
        [0x7f, b'E', b'L', b'F'],
        // Mach-O, 32- and 64-bit, both byte orders
        [0xfe, 0xed, 0xfa, 0xce],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xcf, 0xfa, 0xed, 0xfe],
        // Universal (fat) Mach-O, 32- and 64-bit headers, both byte orders
        [0xca, 0xfe, 0xba, 0xbe],
        [0xbe, 0xba, 0xfe, 0xca],
        [0xca, 0xfe, 0xba, 0xbf],
        [0xbf, 0xba, 0xfe, 0xca],
    ];
    head.starts_with(b"#!") || MAGICS.iter().any(|magic| head.starts_with(magic))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_formats_and_shebang_scripts_are_recognized() {
        assert!(is_native_magic(b"\x7fELF\x02"));
        assert!(is_native_magic(&[0xcf, 0xfa, 0xed, 0xfe]));
        assert!(is_native_magic(&[0xca, 0xfe, 0xba, 0xbe]));
        assert!(is_native_magic(b"#!/bin/sh\n"));
        assert!(!is_native_magic(b"echo hi\n"));
        assert!(!is_native_magic(b""));
        assert!(!is_native_magic(b"#"));
    }

    #[test]
    fn a_program_not_on_the_search_path_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let error = resolve_direct_target("definitely-not-a-program", Some(dir.path().as_os_str()), dir.path())
            .unwrap_err();
        assert_eq!(
            error,
            TemplateTargetError::NotFound {
                program: "definitely-not-a-program".to_string()
            }
        );
    }
}
