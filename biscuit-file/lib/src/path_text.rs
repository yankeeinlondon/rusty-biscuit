//! The path→text boundary: rendering an OS path as portable text.

use std::path::Path;

/// Render a filesystem path as portable text, or `None` when no faithful
/// portable spelling exists.
///
/// Returns `None` exactly when [`to_portable_string`] would fall back to the
/// native spelling: a Windows UNC, device-namespace, or verbatim path that
/// [`dunce::simplified`] declined to reduce. Callers that must not emit a
/// native spelling — a Markdown link destination, for instance — branch on
/// this rather than inspecting the rendered string.
pub fn try_portable_string(path: &Path) -> Option<String> {
    let reduced = dunce::simplified(path);

    #[cfg(windows)]
    {
        // A prefix surviving `simplified` is one `dunce` declined to reduce,
        // or a namespace with no faithful slash-separated form. Replacing
        // separators here would produce `//?/C:/CON` — neither a path nor a
        // URL — from a path the caller had every reason to think was fine.
        if has_non_disk_prefix(reduced) {
            return None;
        }
    }

    Some(reduced.to_string_lossy().replace('\\', "/"))
}

/// Render a filesystem path according to biscuit-file's portable-text policy.
///
/// On Windows, a safely reducible verbatim-disk prefix is removed through
/// [`dunce::simplified`]. Disk, rooted, and relative paths are then rendered
/// with `/` separators. UNC, device-namespace, and verbatim paths that cannot
/// be reduced faithfully retain their native spelling; use
/// [`try_portable_string`] when that fallback is not acceptable.
///
/// The result uses [`Path::to_string_lossy`]: non-Unicode data is replaced
/// with U+FFFD. On Unix, `\` is a legal filename character and is rendered as
/// `/`; see the feature spec's accepted limitations.
pub fn to_portable_string(path: &Path) -> String {
    // `try_portable_string` returns `None` only for a prefix `simplified`
    // left untouched, so the fallback can render `path` itself.
    try_portable_string(path).unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Canonicalize `path` and reduce a Windows verbatim-disk (`\\?\C:\...`)
/// result to its legacy spelling when the reduction is faithful.
///
/// `std::fs::canonicalize` yields a verbatim path on Windows, which never
/// compares equal — lexically or by component — to spellings from other
/// producers (gix discovery, environment variables, CLI arguments). Any
/// canonicalization whose result crosses a comparison or containment
/// boundary should go through this function. On non-Windows hosts it is
/// exactly `std::fs::canonicalize`.
///
/// ## Errors
///
/// Propagates the underlying `std::fs::canonicalize` error (for example,
/// when `path` does not exist).
pub fn canonicalize_simplified(path: &Path) -> std::io::Result<std::path::PathBuf> {
    dunce::canonicalize(path)
}

#[cfg(windows)]
fn has_non_disk_prefix(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    matches!(
        path.components().next(),
        Some(Component::Prefix(prefix)) if !matches!(prefix.kind(), Prefix::Disk(_))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_backslash_path_becomes_forward_slashes() {
        assert_eq!(
            to_portable_string(Path::new(r"docs\file.md")),
            "docs/file.md"
        );
    }

    #[test]
    fn relative_forward_slash_path_is_unchanged() {
        assert_eq!(
            to_portable_string(Path::new("docs/file.md")),
            "docs/file.md"
        );
    }

    #[test]
    fn rooted_path_is_unchanged() {
        assert_eq!(
            to_portable_string(Path::new("/repo/file.md")),
            "/repo/file.md"
        );
    }

    /// Pins the accepted limitation: on Unix `\` is an ordinary filename
    /// character, and the renderer converts it anyway.
    #[test]
    #[cfg(not(windows))]
    fn unix_literal_backslash_in_a_filename_is_converted() {
        assert_eq!(
            to_portable_string(Path::new(r"my\report.md")),
            "my/report.md"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_non_unicode_input_matches_lossy_conversion() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(OsStr::from_bytes(b"bad\xFF.md"));
        let expected = path.to_string_lossy().replace('\\', "/");
        assert_eq!(to_portable_string(path), expected);
    }

    #[test]
    #[cfg(windows)]
    fn windows_non_unicode_input_matches_lossy_conversion() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        // 0xD800 is an unpaired high surrogate: valid UTF-16 storage, not
        // representable in UTF-8.
        let raw: Vec<u16> = "bad"
            .encode_utf16()
            .chain([0xD800])
            .chain(".md".encode_utf16())
            .collect();
        let os_string = OsString::from_wide(&raw);
        let path = Path::new(&os_string);
        let expected = path.to_string_lossy().replace('\\', "/");
        assert_eq!(to_portable_string(path), expected);
    }

    #[test]
    #[cfg(windows)]
    fn windows_disk_path_becomes_forward_slashes() {
        assert_eq!(
            to_portable_string(Path::new(r"C:\repo\file.md")),
            "C:/repo/file.md"
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_safe_verbatim_disk_path_is_reduced() {
        assert_eq!(
            to_portable_string(Path::new(r"\\?\C:\repo\docs\file.md")),
            "C:/repo/docs/file.md"
        );
    }

    /// The load-bearing case. Under `\\?\`, `.` is a literal name rather than
    /// a self-reference, so `dunce` declines; a renderer that collapsed it
    /// lexically — or that replaced separators unconditionally after
    /// `simplified` — would silently rename the path.
    #[test]
    #[cfg(windows)]
    fn windows_verbatim_dot_component_stays_native() {
        let input = r"\\?\C:\repo\.\docs";
        assert_eq!(to_portable_string(Path::new(input)), input);
    }

    #[test]
    #[cfg(windows)]
    fn windows_unreducible_verbatim_disk_paths_stay_native() {
        let long_tail = "a".repeat(300);
        let over_max_path = format!(r"\\?\C:\repo\{long_tail}");
        let inputs = [
            r"\\?\C:\CON",
            r"\\?\C:\repo\trailing.",
            r"\\?\C:\repo\trailing ",
            over_max_path.as_str(),
        ];

        for input in inputs {
            let rendered = to_portable_string(Path::new(input));
            assert_eq!(rendered, input, "{input} should keep its native spelling");
            assert!(
                rendered.starts_with(r"\\?\"),
                "{input} should keep `\\\\?\\`"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn windows_unc_and_device_paths_stay_native() {
        let inputs = [
            r"\\server\share\file.md",
            r"\\?\UNC\server\share\file.md",
            r"\\.\COM1",
        ];

        for input in inputs {
            assert_eq!(to_portable_string(Path::new(input)), input);
        }
    }

    /// The predicate and the renderer must never disagree: every consumer
    /// branch built on the decline assumes the fallback is exactly the lossy
    /// native spelling.
    #[test]
    fn try_and_lossy_renderers_agree() {
        #[cfg(windows)]
        let long_tail = "a".repeat(300);
        #[cfg(windows)]
        let over_max_path = format!(r"\\?\C:\repo\{long_tail}");

        let cases: Vec<(&str, Option<&str>)> = vec![
            (r"docs\file.md", Some("docs/file.md")),
            ("docs/file.md", Some("docs/file.md")),
            ("/repo/file.md", Some("/repo/file.md")),
            #[cfg(not(windows))]
            (r"my\report.md", Some("my/report.md")),
            #[cfg(windows)]
            (r"C:\repo\file.md", Some("C:/repo/file.md")),
            #[cfg(windows)]
            (r"\\?\C:\repo\docs\file.md", Some("C:/repo/docs/file.md")),
            #[cfg(windows)]
            (r"\\?\C:\repo\.\docs", None),
            #[cfg(windows)]
            (r"\\?\C:\CON", None),
            #[cfg(windows)]
            (r"\\?\C:\repo\trailing.", None),
            #[cfg(windows)]
            (r"\\?\C:\repo\trailing ", None),
            #[cfg(windows)]
            (over_max_path.as_str(), None),
            #[cfg(windows)]
            (r"\\server\share\file.md", None),
            #[cfg(windows)]
            (r"\\?\UNC\server\share\file.md", None),
            #[cfg(windows)]
            (r"\\.\COM1", None),
        ];

        for (input, expected) in cases {
            let path = Path::new(input);
            assert_eq!(
                try_portable_string(path).as_deref(),
                expected,
                "unexpected decline decision for {input}"
            );
            match expected {
                Some(portable) => assert_eq!(to_portable_string(path), portable),
                None => assert_eq!(to_portable_string(path), path.to_string_lossy()),
            }
        }
    }
}
