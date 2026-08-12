use std::borrow::Cow;

pub(crate) fn normalize(value: &str) -> Cow<'_, str> {
    if value.contains('\\') {
        Cow::Owned(value.replace('\\', "/"))
    } else {
        Cow::Borrowed(value)
    }
}

pub(crate) fn is_absolute_spelling(path: &str) -> bool {
    path.starts_with('/') || is_windows_absolute_spelling(path)
}

pub(crate) fn is_windows_absolute_spelling(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with("//")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'/')
}

pub(crate) fn is_exact_or_descendant(path: &str, prefix: &str) -> bool {
    path == prefix
        || path.strip_prefix(prefix).is_some_and(|remainder| {
            remainder.starts_with('/') || (prefix.ends_with('/') && !remainder.is_empty())
        })
}

pub(crate) fn segments(path: &str) -> impl DoubleEndedIterator<Item = &str> {
    path.split('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_portable_absolute_spellings() {
        assert!(is_absolute_spelling("/var/tmp"));
        assert!(is_absolute_spelling("C:/proj/file"));
        assert!(is_absolute_spelling("//server/share/file"));
        assert!(!is_absolute_spelling("C:proj/file"));
        assert!(!is_absolute_spelling("proj/file"));
    }

    #[test]
    fn descendant_match_requires_a_segment_boundary() {
        assert!(is_exact_or_descendant("C:/proj", "C:/proj"));
        assert!(is_exact_or_descendant("C:/proj/src/lib.rs", "C:/proj"));
        assert!(!is_exact_or_descendant("C:/proj2/src/lib.rs", "C:/proj"));
    }

    #[test]
    fn path_matching_modules_keep_separator_semantics_centralized() {
        let sources = [
            (
                "permissions/matchers.rs",
                include_str!("permissions/matchers.rs"),
            ),
            ("protect/path.rs", include_str!("protect/path.rs")),
        ];
        let forbidden = [
            "warn_windows_path_matching_is_broken",
            "Some(&b'/')",
            ".split('/')",
            ".rsplit('/')",
            "format!(\"{}/\"",
        ];

        for (path, source) in sources {
            for needle in forbidden {
                assert!(
                    !source.contains(needle),
                    "{path} reintroduced raw separator-boundary construct {needle:?}"
                );
            }
        }
    }
}
