//! Where the quarantine goes: a hidden sibling of the checkout, named for the
//! operation that moves it there.
//!
//! The sibling shares the checkout's parent directory, so the move never
//! crosses a filesystem and Git's relative links keep their depth. Nothing is
//! created here: the destination comes into existence only through
//! [`super::rename::rename_no_replace`], which fails rather than reuse a path
//! that already exists.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::remove::recovery::OperationId;
use crate::remove::recovery::record::QUARANTINE_PREFIX;

/// The longest slug taken from the checkout's name, in bytes. It keeps the
/// quarantine's path near the original's length on Windows, where paths over
/// 260 characters need long-path support.
const MAX_SLUG: usize = 40;

/// How many hex characters of the operation ID end the name (64 bits).
const TOKEN_LEN: usize = 16;

/// The quarantine path for moving `original` under `operation`:
/// `<parent>/.wt-remove-<slug>-<token>`, where the slug is the checkout's
/// name reduced to ASCII letters, digits, `-`, and `_`, and the token is the
/// start of the operation ID. `None` when `original` has no parent or name.
pub fn quarantine_path(original: &Path, operation: &OperationId) -> Option<PathBuf> {
    let parent = original.parent()?;
    let name = original.file_name()?;
    let mut quarantine = OsString::from(QUARANTINE_PREFIX);
    quarantine.push(slug(&name.to_string_lossy()));
    quarantine.push("-");
    quarantine.push(&operation.as_str()[..TOKEN_LEN]);
    Some(parent.join(quarantine))
}

/// ASCII only, so the name means the same thing to every filesystem's
/// normalization and case rules; the token, not the slug, makes it unique.
fn slug(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '-' })
        .collect();
    let mut slug = mapped.split('-').filter(|part| !part.is_empty()).collect::<Vec<_>>().join("-");
    slug.truncate(MAX_SLUG);
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() { "worktree".into() } else { slug.into() }
}

/// Sets `FILE_ATTRIBUTE_HIDDEN` on `path`, keeping its other attributes. The
/// leading dot hides the quarantine elsewhere.
#[cfg(windows)]
pub fn hide(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_HIDDEN, GetFileAttributesW, INVALID_FILE_ATTRIBUTES, SetFileAttributesW,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let attributes = unsafe { GetFileAttributesW(wide.as_ptr()) };
    if attributes == INVALID_FILE_ATTRIBUTES {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe { SetFileAttributesW(wide.as_ptr(), attributes | FILE_ATTRIBUTE_HIDDEN) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation() -> OperationId {
        OperationId::parse("0123456789abcdef0123456789abcdef").unwrap()
    }

    #[test]
    fn the_quarantine_is_a_hidden_sibling_named_for_the_operation() {
        let original = Path::new("/work/fix-path-spelling");
        let quarantine = quarantine_path(original, &operation()).unwrap();
        assert_eq!(quarantine, Path::new("/work/.wt-remove-fix-path-spelling-0123456789abcdef"));
        assert_eq!(quarantine.parent(), original.parent());
        assert!(quarantine.file_name().unwrap().to_string_lossy().starts_with(QUARANTINE_PREFIX));
    }

    #[test]
    fn different_operations_get_different_quarantines() {
        let original = Path::new("/work/feat-x");
        let first = quarantine_path(original, &OperationId::generate().unwrap()).unwrap();
        let second = quarantine_path(original, &OperationId::generate().unwrap()).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn the_slug_is_ascii_and_bounded() {
        for (name, expected) in [
            ("feat-x", "feat-x"),
            ("Feat X", "Feat-X"),
            ("feat.x", "feat-x"),
            ("--a__b--", "a__b"),
            ("café", "caf"),
            ("日本", "worktree"),
            ("...", "worktree"),
            (&"a".repeat(100), &"a".repeat(MAX_SLUG)),
            // Truncation never leaves a trailing dash before the token.
            (&format!("{}-b", "a".repeat(MAX_SLUG - 1)), &"a".repeat(MAX_SLUG - 1)),
        ] {
            assert_eq!(slug(name), expected, "{name:?}");
        }
    }

    #[test]
    fn a_path_without_a_parent_or_name_has_no_quarantine() {
        assert_eq!(quarantine_path(Path::new("/"), &operation()), None);
        assert_eq!(quarantine_path(Path::new(""), &operation()), None);
    }

    /// The record refuses any quarantine that is not a prefixed sibling; the
    /// name built here must always pass that rule.
    #[cfg(unix)]
    #[test]
    fn a_name_that_is_not_utf8_still_yields_a_valid_sibling() {
        use std::os::unix::ffi::OsStrExt;
        let original = Path::new("/work").join(std::ffi::OsStr::from_bytes(b"feat-\xff"));
        let quarantine = quarantine_path(&original, &operation()).unwrap();
        assert_eq!(quarantine, Path::new("/work/.wt-remove-feat-0123456789abcdef"));
    }
}
