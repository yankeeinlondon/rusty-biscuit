use super::windows::{self, PrefixKind};
use super::*;

fn host(text: &str) -> PathIdentity {
    PathIdentity::new(Path::new(text))
}

fn win(text: &str) -> PathIdentity {
    PathIdentity::from_windows_text(text)
}

fn names(identity: &PathIdentity) -> Vec<String> {
    identity
        .components()
        .iter()
        .map(|name| name.to_string_lossy().into_owned())
        .collect()
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn route(target: &PathIdentity, dir: &PathIdentity) -> Option<(usize, Vec<String>)> {
    target.relative_from(dir).map(|route| {
        (
            route.parent_hops(),
            route
                .forward()
                .iter()
                .map(|name| name.to_string_lossy().into_owned())
                .collect(),
        )
    })
}

// ---- prefix comparison -------------------------------------------------

#[test]
fn prefix_matches_whole_components_only() {
    let root = host("/opt/config");
    assert!(host("/opt/config/app.toml").starts_with(&root));
    assert!(host("/opt/config").starts_with(&root));
    assert!(!host("/opt/config-old").starts_with(&root));
    assert!(!host("/opt/config-old/app.toml").starts_with(&root));
    assert_eq!(host("/opt/config-old/app.toml").strip_prefix(&root), None);

    let win_root = win(r"C:\opt\config");
    assert!(win(r"C:\opt\config\app.toml").starts_with(&win_root));
    assert!(!win(r"C:\opt\config-old\app.toml").starts_with(&win_root));
}

#[test]
fn strip_prefix_returns_the_names_below_the_base() {
    let root = host("/repo");
    let target = host("/repo/docs/guide.md");
    let rest: Vec<String> = target
        .strip_prefix(&root)
        .unwrap()
        .iter()
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    assert_eq!(rest, ["docs", "guide.md"]);
    assert_eq!(root.strip_prefix(&root), Some(&[][..]));
}

#[test]
fn rooted_and_relative_paths_never_share_a_prefix() {
    assert!(!host("/a/b").starts_with(&host("a")));
    assert!(!host("a/b").starts_with(&host("/a")));
    assert_eq!(route(&host("/a/b"), &host("a")), None);
}

// ---- `.` and `..` on ordinary paths -------------------------------------

#[test]
fn dot_segments_collapse_on_ordinary_paths() {
    assert_eq!(host("/a/b/../c/./d"), host("/a/c/d"));
    assert_eq!(host("./a"), host("a"));
    assert_eq!(host("/a/b/.."), host("/a"));
    assert_eq!(win(r"C:\a\b\..\c\.\d"), win(r"C:\a\c\d"));
}

#[test]
fn parent_segments_never_walk_above_a_root() {
    assert_eq!(host("/../a"), host("/a"));
    assert_eq!(host("/a/../../b"), host("/b"));
    assert_eq!(win(r"C:\..\a"), win(r"C:\a"));
    assert_eq!(win(r"\\server\share\..\a"), win(r"\\server\share\a"));
}

#[test]
fn relative_paths_keep_leading_parent_hops() {
    let identity = host("a/../../b");
    assert_eq!(identity, host("../b"));
    assert_ne!(identity, host("b"));
    assert_eq!(names(&identity), ["b"]);
    assert!(!host("../b").starts_with(&host("b")));
    assert_eq!(win(r"C:..\x"), win(r"C:a\..\..\x"));
    assert_ne!(win(r"C:..\x"), win(r"C:x"));
}

// ---- relative routes ----------------------------------------------------

#[test]
fn routes_between_directories_and_targets() {
    let target = host("/repo/assets/images/logo.png");
    assert_eq!(
        route(&target, &host("/repo/docs/deep")),
        Some((2, vec!["assets".into(), "images".into(), "logo.png".into()]))
    );
    assert_eq!(
        route(&target, &host("/repo/assets/images")),
        Some((0, vec!["logo.png".into()]))
    );
    assert_eq!(
        route(&host("/repo/file.md"), &host("/repo/a/b/c")),
        Some((3, vec!["file.md".into()]))
    );
}

#[test]
fn equal_paths_give_an_empty_route_rendered_as_dot() {
    let route = host("/a/b").relative_from(&host("/a/b")).unwrap();
    assert_eq!(route.parent_hops(), 0);
    assert!(route.forward().is_empty());
    assert_eq!(route.to_path_buf(), PathBuf::from("."));
}

/// The directory is given, never guessed: a directory whose last name looks
/// like a file (`v1.2`) and a file-like name without an extension (`README`)
/// are both read as directories.
#[test]
fn the_from_operand_is_always_a_directory() {
    assert_eq!(
        route(&host("/repo/x.md"), &host("/repo/v1.2")),
        Some((1, vec!["x.md".into()]))
    );
    assert_eq!(
        route(&host("/repo/x.md"), &host("/repo/README")),
        Some((1, vec!["x.md".into()]))
    );
}

#[test]
fn routes_between_relative_paths_respect_leading_hops() {
    assert_eq!(route(&host("../../b"), &host("../a")), Some((2, vec!["b".into()])));
    assert_eq!(route(&host("../b"), &host("../a")), Some((1, vec!["b".into()])));
    // From `../../a` back down to `../b` would need the name of the directory
    // `..` refers to, which the path does not spell.
    assert_eq!(route(&host("../b"), &host("../../a")), None);
}

#[test]
fn route_to_path_buf_joins_hops_and_names() {
    let route = host("/r/assets/a.png").relative_from(&host("/r/docs")).unwrap();
    assert_eq!(route.to_path_buf(), Path::new("..").join("assets").join("a.png"));
}

// ---- Windows roots (portable fixtures) ----------------------------------

#[test]
fn drive_letters_are_case_insensitive_and_names_are_not() {
    assert_eq!(win(r"c:\x"), win(r"C:\x"));
    assert_ne!(win(r"C:\Repo"), win(r"C:\repo"));
}

#[test]
fn verbatim_drive_equals_its_legacy_spelling() {
    assert_eq!(win(r"\\?\C:\x\y"), win(r"C:\x\y"));
    assert_eq!(win(r"\\?\c:\x"), win(r"C:/x"));
    assert!(win(r"\\?\C:\repo\docs").starts_with(&win(r"C:\repo")));
}

#[test]
fn drive_absolute_and_drive_relative_differ() {
    assert_ne!(win(r"C:\a"), win(r"C:a"));
    assert!(!win(r"C:a\b").starts_with(&win(r"C:\a")));
}

#[test]
fn different_drives_and_shares_are_separate_roots() {
    assert!(!win(r"D:\repo\x").starts_with(&win(r"C:\repo")));
    assert_eq!(route(&win(r"D:\repo\x"), &win(r"C:\repo")), None);
    assert!(!win(r"\\server\other\x").starts_with(&win(r"\\server\share")));
    assert_eq!(route(&win(r"\\server\other\x"), &win(r"\\server\share")), None);
    assert_eq!(route(&win(r"\\server\share\x"), &win(r"C:\x")), None);
}

#[test]
fn unc_spellings_of_one_share_are_equal() {
    assert_eq!(win(r"\\server\share\x"), win(r"\\?\UNC\server\share\x"));
    assert_eq!(win(r"\\server\share\x"), win("//server/share/x"));
    // A share root is rooted with or without its trailing separator.
    assert_eq!(win(r"\\server\share"), win(r"\\server\share\"));

    let legacy_root = win(r"\\server\share\repo");
    let verbatim_child = win(r"\\?\UNC\server\share\repo\docs\f.md");
    assert!(verbatim_child.starts_with(&legacy_root));
    assert_eq!(
        route(&verbatim_child, &win(r"\\server\share\repo\assets")),
        Some((1, vec!["docs".into(), "f.md".into()]))
    );
}

#[test]
fn device_and_other_verbatim_prefixes_keep_their_own_text() {
    let device = windows::parse(&units(r"\\.\COM1\x"));
    assert_eq!(String::from_utf16_lossy(&device.root), r"\\.\COM1");
    assert_ne!(win(r"\\.\C:\x"), win(r"C:\x"));

    let volume = windows::parse(&units(r"\\?\Volume{1234}\a"));
    assert_eq!(String::from_utf16_lossy(&volume.root), r"\\?\Volume{1234}");
    assert!(volume.rooted);
    assert_ne!(win(r"\\?\Volume{1234}\a"), win(r"\\?\Volume{5678}\a"));
}

/// A long verbatim descendant of a short root that `dunce` could reduce must
/// still be inside that root; the identity equates roots regardless of whether
/// the whole path has a legacy spelling.
#[test]
fn long_verbatim_descendant_stays_inside_a_short_legacy_root() {
    let long_a = "a".repeat(150);
    let long_b = "b".repeat(150);
    let descendant = win(&format!(r"\\?\C:\r\assets\{long_a}\{long_b}\image.png"));
    assert!(descendant.starts_with(&win(r"C:\r")));
    assert_eq!(
        route(&descendant, &win(r"C:\r\docs")),
        Some((1, vec!["assets".into(), long_a, long_b, "image.png".into()]))
    );
}

/// `Path::join("docs/file.md")` keeps the literal `/`, so a candidate built on
/// a Windows root carries mixed separators; under ordinary Windows grammar
/// both separators split names.
#[test]
fn mixed_separators_on_an_ordinary_path_share_one_identity() {
    assert_eq!(win(r"C:\repo\docs/file.md"), win(r"C:\repo\docs\file.md"));
    assert!(win(r"C:\repo\docs/file.md").starts_with(&win(r"C:\repo\docs")));
}

#[test]
fn other_drives_and_drive_relative_spellings_are_distinct_identities() {
    assert_ne!(win(r"C:\repo"), win(r"D:\repo"));
    assert_ne!(win(r"C:repo"), win(r"C:\repo"));
    assert_ne!(win(r"\\?\C:\repo"), win(r"\\?\D:\repo"));
}

#[test]
fn a_verbatim_share_is_distinct_from_another_server_or_share() {
    let verbatim = win(r"\\?\UNC\server\share\x");
    assert_ne!(verbatim, win(r"\\other\share\x"));
    assert_ne!(verbatim, win(r"\\server\other\x"));
    assert!(!verbatim.starts_with(&win(r"\\server\other")));
}

/// Containment is by whole names in every spelling of the boundary.
#[test]
fn a_sibling_with_the_boundary_as_text_prefix_is_outside() {
    let boundary = win(r"C:\repo");
    for document in [r"C:\repo-old\x.md", r"\\?\C:\repo-old\x.md", r"c:/repo-old/x.md"] {
        assert!(!win(document).starts_with(&boundary), "{document}");
        assert_eq!(win(document).strip_prefix(&boundary), None, "{document}");
    }
    assert!(win(r"\\?\C:\repo\x.md").starts_with(&boundary));
}

/// Text alone cannot show that `RUNNER~1` names `runneradmin`; only the
/// filesystem can, so the identity keeps them apart in every spelling.
#[test]
fn short_and_long_names_are_not_lexically_unified() {
    let short = win(r"C:\Users\RUNNER~1\AppData\Local\Temp");
    let long = win(r"C:\Users\runneradmin\AppData\Local\Temp");
    assert_ne!(short, long);
    assert!(!short.starts_with(&win(r"C:\Users\runneradmin")));
    assert_ne!(win(r"\\?\C:\Users\RUNNER~1"), win(r"C:\Users\runneradmin"));
    assert_eq!(route(&short, &win(r"C:\Users\runneradmin")).map(|(hops, _)| hops), Some(1));
}

// ---- first-seen ordering (production dedupe) ----------------------------

/// Run `first_seen_by_identity` over tagged Windows spellings and report
/// each survivor's tag and its spelling, unchanged.
fn first_seen<'a>(entries: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    first_seen_by_identity(entries.iter().copied(), |(_, text)| win(text))
}

fn reversed<'a>(entries: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    entries.iter().rev().copied().collect()
}

/// The PR #66 shape: a repository root reached through `canonicalize`
/// (verbatim) and a source directory from `gix` (legacy) name one directory.
/// Whichever comes first survives, with its own tag and spelling.
#[test]
fn verbatim_and_legacy_duplicates_keep_the_first_occurrence_in_either_order() {
    let entries = [("source", r"C:\repo\x.md"), ("repository", r"\\?\C:\repo\x.md")];
    assert_eq!(first_seen(&entries), [("source", r"C:\repo\x.md")]);
    assert_eq!(
        first_seen(&reversed(&entries)),
        [("repository", r"\\?\C:\repo\x.md")]
    );
}

#[test]
fn unc_verbatim_and_legacy_duplicates_keep_the_first_occurrence() {
    let entries = [
        ("legacy", r"\\server\share\repo\x.md"),
        ("verbatim", r"\\?\UNC\server\share\repo\x.md"),
        ("slashes", "//server/share/repo/x.md"),
        ("other-share", r"\\server\other\repo\x.md"),
    ];
    assert_eq!(
        first_seen(&entries),
        [("legacy", r"\\server\share\repo\x.md"), ("other-share", r"\\server\other\repo\x.md")]
    );
    assert_eq!(
        first_seen(&reversed(&entries)),
        [("other-share", r"\\server\other\repo\x.md"), ("slashes", "//server/share/repo/x.md")]
    );
}

/// Drive case and separators fold; name case, other drives, and
/// drive-relative spellings do not. The count is the same in both orders;
/// only which tag represents each identity changes.
#[test]
fn ordering_folds_only_the_windows_equalities() {
    let entries = [
        ("upper-name", r"C:\Repo\x.md"),
        ("lower-name", r"C:\repo\x.md"),
        ("lower-drive", r"c:\repo\x.md"),
        ("mixed", r"C:\repo/x.md"),
        ("verbatim-upper-name", r"\\?\C:\Repo\x.md"),
        ("other-drive", r"D:\repo\x.md"),
        ("drive-relative", r"C:repo\x.md"),
    ];
    let tags = |survivors: Vec<(&str, &str)>| -> Vec<String> {
        survivors.into_iter().map(|(tag, _)| tag.to_string()).collect()
    };
    assert_eq!(
        tags(first_seen(&entries)),
        ["upper-name", "lower-name", "other-drive", "drive-relative"]
    );
    assert_eq!(
        tags(first_seen(&reversed(&entries))),
        ["drive-relative", "other-drive", "verbatim-upper-name", "mixed"]
    );
}

/// A verbatim path too long for a legacy spelling still names the same file
/// as its legacy form, so the two collapse even though the verbatim spelling
/// cannot be reduced.
#[test]
fn a_long_verbatim_duplicate_collapses_onto_its_legacy_spelling() {
    let long = "a".repeat(300);
    let legacy = format!(r"C:\repo\{long}\x.md");
    let verbatim = format!(r"\\?\C:\repo\{long}\x.md");
    let entries = [("verbatim", verbatim.as_str()), ("legacy", legacy.as_str())];
    assert_eq!(first_seen(&entries), [("verbatim", verbatim.as_str())]);
    assert_eq!(first_seen(&reversed(&entries)), [("legacy", legacy.as_str())]);
}

/// Under `\\?\`, `..` is a directory name, so a verbatim path with a literal
/// `..` is not a duplicate of the path ordinary normalization would give.
#[test]
fn verbatim_literal_dot_segments_are_not_duplicates_of_collapsed_paths() {
    let entries = [("verbatim", r"\\?\C:\a\..\b\x.md"), ("legacy", r"C:\b\x.md")];
    assert_eq!(first_seen(&entries), entries);
    assert_eq!(first_seen(&reversed(&entries)), reversed(&entries));
}

#[test]
fn short_and_long_spellings_are_both_kept() {
    let entries = [("short", r"C:\Users\RUNNER~1\x.md"), ("long", r"C:\Users\runneradmin\x.md")];
    assert_eq!(first_seen(&entries), entries);
}

#[cfg(unix)]
#[test]
fn ordering_keeps_distinct_non_unicode_names_on_unix() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let entries = [
        ("ff", Path::new(OsStr::from_bytes(b"/repo/bad\xFF.md"))),
        ("fe", Path::new(OsStr::from_bytes(b"/repo/bad\xFE.md"))),
        ("ff-again", Path::new(OsStr::from_bytes(b"/repo/./bad\xFF.md"))),
    ];
    let survivors = first_seen_by_identity(entries, |(_, path)| PathIdentity::new(path));
    let tags: Vec<&str> = survivors.iter().map(|(tag, _)| *tag).collect();
    assert_eq!(tags, ["ff", "fe"]);
}

// ---- verbatim literal dot segments --------------------------------------

#[test]
fn verbatim_dot_segments_are_literal_names() {
    let identity = win(r"\\?\C:\a\..\b");
    assert_eq!(names(&identity), ["a", "..", "b"]);
    assert_ne!(identity, win(r"C:\b"));
    assert_ne!(identity, win(r"C:\a\..\b"));

    assert_eq!(names(&win(r"\\?\C:\a\.\b")), ["a", ".", "b"]);
    assert_eq!(names(&win(r"\\?\UNC\s\h\..")), [".."]);
}

#[test]
fn verbatim_paths_split_only_on_backslash() {
    assert_eq!(names(&win(r"\\?\C:\a/b")), ["a/b"]);
    assert_eq!(names(&win(r"C:\a/b")), ["a", "b"]);
}

#[test]
fn a_literal_verbatim_parent_name_is_kept_in_a_route() {
    let route = win(r"\\?\C:\r\..\x.md").relative_from(&win(r"C:\r\docs")).unwrap();
    assert_eq!(route.parent_hops(), 1);
    let forward: Vec<String> = route
        .forward()
        .iter()
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    assert_eq!(forward, ["..", "x.md"]);
}

// ---- encoding -----------------------------------------------------------

/// Two paths that differ only in an unpaired surrogate must not share an
/// identity; a lossy conversion would map both onto one U+FFFD name.
#[test]
fn unpaired_surrogates_stay_distinct() {
    let with_trailing = |unit: u16| {
        let mut path = units(r"C:\repo\");
        path.push(unit);
        windows::parse(&path)
    };
    let first = with_trailing(0xD800);
    let second = with_trailing(0xD801);
    assert_eq!(
        String::from_utf16_lossy(&first.components[1]),
        String::from_utf16_lossy(&second.components[1]),
        "fixture must be one a lossy key would collapse"
    );
    assert_ne!(first, second);
    assert_eq!(first.root, units("C:"));
}

#[cfg(unix)]
#[test]
fn non_unicode_names_stay_distinct_on_unix() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let first = PathIdentity::new(Path::new(OsStr::from_bytes(b"/repo/bad\xFF.md")));
    let second = PathIdentity::new(Path::new(OsStr::from_bytes(b"/repo/bad\xFE.md")));
    assert_ne!(first, second);
    assert!(first.starts_with(&host("/repo")));
    assert_eq!(
        first.relative_from(&host("/repo")).unwrap().forward()[0],
        OsStr::from_bytes(b"bad\xFF.md")
    );
}

/// On Unix `\` is an ordinary filename character, so it never splits a name.
#[cfg(unix)]
#[test]
fn backslash_is_part_of_a_unix_name() {
    let identity = host(r"/repo/my\report.md");
    assert_eq!(names(&identity), ["repo", r"my\report.md"]);
    assert_ne!(identity, host("/repo/my/report.md"));
}

// ---- prefix grammar -----------------------------------------------------

const PREFIX_FIXTURES: &[(&str, Option<PrefixKind>)] = &[
    (r"C:\a", Some(PrefixKind::Disk)),
    ("c:a", Some(PrefixKind::Disk)),
    (r"\\?\C:\a", Some(PrefixKind::VerbatimDisk)),
    (r"\\?\C:", Some(PrefixKind::VerbatimDisk)),
    (r"\\?\C:x", Some(PrefixKind::Verbatim)),
    (r"\\?\UNC\server\share\a", Some(PrefixKind::VerbatimUnc)),
    (r"\\?\Volume{1}\a", Some(PrefixKind::Verbatim)),
    (r"\\.\COM1", Some(PrefixKind::DeviceNs)),
    (r"//./COM1", Some(PrefixKind::DeviceNs)),
    (r"\\server\share\a", Some(PrefixKind::Unc)),
    ("//server/share/a", Some(PrefixKind::Unc)),
    (r"\\server", None),
    (r"\a", None),
    ("a", None),
];

#[test]
fn prefix_grammar_classifies_each_windows_form() {
    for (text, expected) in PREFIX_FIXTURES {
        assert_eq!(windows::prefix_kind(&units(text)), *expected, "{text}");
    }
}

/// The portable parser and the standard library must classify every fixture
/// the same way, or the portable tests above prove nothing about Windows.
#[cfg(windows)]
#[test]
fn prefix_grammar_agrees_with_the_standard_library() {
    use std::path::{Component, Prefix};

    for (text, expected) in PREFIX_FIXTURES {
        let std_kind = match Path::new(text).components().next() {
            Some(Component::Prefix(prefix)) => Some(match prefix.kind() {
                Prefix::Disk(_) => PrefixKind::Disk,
                Prefix::VerbatimDisk(_) => PrefixKind::VerbatimDisk,
                Prefix::UNC(..) => PrefixKind::Unc,
                Prefix::VerbatimUNC(..) => PrefixKind::VerbatimUnc,
                Prefix::DeviceNS(_) => PrefixKind::DeviceNs,
                Prefix::Verbatim(_) => PrefixKind::Verbatim,
            }),
            _ => None,
        };
        assert_eq!(std_kind, *expected, "{text}");
    }
}

/// On a Windows host the real constructor must agree with the portable
/// fixtures, including keeping verbatim dot segments that
/// `Path::components` would reinterpret.
#[cfg(windows)]
#[test]
fn host_identity_matches_the_portable_parser_on_windows() {
    for text in [
        r"\\?\C:\a\..\b",
        r"\\?\UNC\server\share\x",
        r"C:\a\b\..\c",
        r"c:a\..\..\x",
    ] {
        assert_eq!(host(text), win(text), "{text}");
    }
}

/// The real Windows constructor must stay lossless: two paths that differ
/// only in an unpaired surrogate must not share an identity.
#[cfg(windows)]
#[test]
fn host_identity_keeps_unpaired_surrogates_distinct_on_windows() {
    use std::os::windows::ffi::OsStringExt;

    let with_trailing_unit = |unit: u16| {
        let units: Vec<u16> = r"C:\repo\".encode_utf16().chain([unit]).collect();
        PathBuf::from(OsString::from_wide(&units))
    };
    let first = PathIdentity::new(&with_trailing_unit(0xD800));
    let second = PathIdentity::new(&with_trailing_unit(0xD801));
    assert_ne!(first, second);
    assert!(!first.starts_with(&second));
    assert!(first.starts_with(&host(r"C:\repo")));
}

/// Text cannot show that a short name and its long name are one directory, so
/// the identities differ while the filesystem resolves both to one target.
/// Needs a Windows volume that generates 8.3 names; many disable generation,
/// and then there is no alias to test, so the test reports that and returns.
#[cfg(windows)]
#[test]
fn a_real_short_name_alias_is_one_directory_but_two_identities() {
    use std::os::windows::process::CommandExt;

    let temp = tempfile::TempDir::new().unwrap();
    let long = temp.path().join("Long Directory Name");
    std::fs::create_dir(&long).unwrap();
    std::fs::write(long.join("x.md"), b"x").unwrap();
    let output = std::process::Command::new("cmd")
        .raw_arg(format!(r#"/C for %I in ("{}") do @echo %~sI"#, long.display()))
        .output()
        .unwrap();
    assert!(output.status.success(), "cmd failed: {output:?}");
    let short = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if short == long {
        eprintln!("8.3 name generation is disabled for {}; no alias to test", long.display());
        return;
    }

    assert_ne!(PathIdentity::new(&short), PathIdentity::new(&long));
    assert!(!PathIdentity::new(&short.join("x.md")).starts_with(&PathIdentity::new(&long)));
    assert_eq!(std::fs::read(short.join("x.md")).unwrap(), b"x");
    assert_eq!(
        std::fs::canonicalize(&short).unwrap(),
        std::fs::canonicalize(&long).unwrap()
    );
}
