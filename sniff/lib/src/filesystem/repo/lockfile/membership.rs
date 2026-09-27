//! Member-path normalization and set comparison.
//!
//! Both sides become `/`-separated paths relative to the layer root, with the
//! root itself excluded. Normalization is lexical and by component: no member
//! path from a lockfile is ever opened or canonicalized, so a stale member
//! that no longer exists still compares.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use super::super::detection::normalize_path;
use super::super::seed::PackageSeed;
use super::{LockfileObservation, LockfileReason, LockfileStatus};

/// Normalize one member path recorded in a lockfile.
///
/// `raw` is resolved against `base`, the components of the directory the
/// format's paths are relative to, itself relative to the layer root. `.` and
/// empty components drop, `..` pops a base or earlier component, and a `..`
/// that climbs above the layer root is kept so a legitimate external member
/// stays distinct. Names keep their leading dots and case. Both `/` and `\`
/// separate components, so a lockfile written on Windows compares the same.
///
/// ## Returns
///
/// The normalized path, `""` for the layer root.
///
/// ## Errors
///
/// [`LockfileReason::InvalidMemberPath`] for an absolute path (a leading
/// separator or a drive prefix) or a path containing NUL, which no supported
/// filesystem can represent.
pub(super) fn normalize_member(raw: &str, base: &[&str]) -> Result<String, LockfileReason> {
    let bytes = raw.as_bytes();
    let has_drive_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if raw.starts_with(['/', '\\']) || has_drive_prefix || raw.contains('\0') {
        return Err(LockfileReason::InvalidMemberPath);
    }
    let mut parts: Vec<&str> = base.to_vec();
    for component in raw.split(['/', '\\']) {
        match component {
            "" | "." => {}
            ".." if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    Ok(parts.join("/"))
}

/// The normalized, root-excluded set of lockfile-recorded member paths.
///
/// Duplicate spellings of one member collapse; the set is never intersected
/// with the manifest members, so stale extra members survive to the
/// comparison.
pub(super) fn recorded_member_set(
    recorded: &[String],
    base: &[&str],
) -> Result<BTreeSet<String>, LockfileReason> {
    let mut members = BTreeSet::new();
    for raw in recorded {
        let member = normalize_member(raw, base)?;
        if !member.is_empty() {
            members.insert(member);
        }
    }
    Ok(members)
}

/// `member`'s layer-relative path in the lockfile spelling.
///
/// A member outside the layer root keeps its `..` components. A member on a
/// different Windows drive, or with a non-UTF-8 name, has no representable
/// relative path.
pub(super) fn manifest_member(layer_root: &Path, member: &Path) -> Result<String, LockfileReason> {
    let relative = match member.strip_prefix(layer_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => lexical_relative(&normalize_path(member), &normalize_path(layer_root))
            .ok_or(LockfileReason::InvalidMemberPath)?,
    };
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => {
                parts.push(name.to_str().ok_or(LockfileReason::InvalidMemberPath)?);
            }
            Component::CurDir => {}
            Component::ParentDir => parts.push(".."),
            Component::Prefix(_) | Component::RootDir => {
                return Err(LockfileReason::InvalidMemberPath);
            }
        }
    }
    normalize_member(&parts.join("/"), &[])
}

/// The root-excluded set of manifest-derived member paths for a layer.
pub(super) fn manifest_member_set(
    layer_root: &Path,
    owned: &[PackageSeed],
) -> Result<BTreeSet<String>, LockfileReason> {
    let mut members = BTreeSet::new();
    for seed in owned {
        let member = manifest_member(layer_root, &seed.path)?;
        if !member.is_empty() {
            members.insert(member);
        }
    }
    Ok(members)
}

/// Exact set comparison: `match` on equality, otherwise `mismatch` with the
/// members only the lockfile records (`extra`) and only the manifest declares
/// (`missing`).
pub(super) fn compare(
    manifest: &BTreeSet<String>,
    locked: &BTreeSet<String>,
    paths: Vec<String>,
) -> LockfileObservation {
    if manifest == locked {
        return LockfileObservation::new(LockfileStatus::Match, paths, None);
    }
    LockfileObservation {
        extra: locked.difference(manifest).cloned().collect(),
        missing: manifest.difference(locked).cloned().collect(),
        ..LockfileObservation::new(LockfileStatus::Mismatch, paths, None)
    }
}

/// `path` relative to `base` by components, with `..` for each unshared
/// component of `base`. `None` when the two do not share a prefix (different
/// Windows drives).
fn lexical_relative(path: &Path, base: &Path) -> Option<PathBuf> {
    let path: Vec<Component<'_>> = path.components().collect();
    let base: Vec<Component<'_>> = base.components().collect();
    let shared = path
        .iter()
        .zip(&base)
        .take_while(|(left, right)| left == right)
        .count();
    let has_shared_anchor = shared > 0
        && path
            .first()
            .is_some_and(|first| matches!(first, Component::Prefix(_) | Component::RootDir));
    if !has_shared_anchor {
        return None;
    }
    let mut relative = PathBuf::new();
    for _ in shared..base.len() {
        relative.push("..");
    }
    for component in &path[shared..] {
        relative.push(component.as_os_str());
    }
    Some(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_member_drops_dot_components_and_trailing_separators() {
        for (raw, expected) in [
            ("packages/alpha", "packages/alpha"),
            ("./packages/alpha/", "packages/alpha"),
            ("packages//alpha/.", "packages/alpha"),
            (".", ""),
            ("", ""),
            ("./", ""),
        ] {
            assert_eq!(normalize_member(raw, &[]), Ok(expected.to_owned()), "{raw:?}");
        }
    }

    #[test]
    fn normalize_member_keeps_leading_dots_in_names_and_case() {
        assert_eq!(
            normalize_member(".tools/hidden", &[]),
            Ok(".tools/hidden".to_owned())
        );
        assert_eq!(
            normalize_member("Packages/Alpha", &[]),
            Ok("Packages/Alpha".to_owned())
        );
        assert_eq!(normalize_member("..hidden", &[]), Ok("..hidden".to_owned()));
    }

    #[test]
    fn normalize_member_resolves_parent_components_against_the_format_base() {
        let rush_base = ["common", "temp"];
        assert_eq!(
            normalize_member("../../packages/alpha", &rush_base),
            Ok("packages/alpha".to_owned())
        );
        assert_eq!(normalize_member(".", &rush_base), Ok("common/temp".to_owned()));
        assert_eq!(
            normalize_member("packages/alpha/../beta", &[]),
            Ok("packages/beta".to_owned())
        );
    }

    #[test]
    fn normalize_member_keeps_parent_components_above_the_layer_root() {
        assert_eq!(normalize_member("../sibling", &[]), Ok("../sibling".to_owned()));
        assert_eq!(
            normalize_member("../../../outside", &["common", "temp"]),
            Ok("../outside".to_owned())
        );
        assert_eq!(normalize_member("../../x", &[]), Ok("../../x".to_owned()));
    }

    #[test]
    fn normalize_member_converts_windows_separators() {
        assert_eq!(
            normalize_member(r"packages\alpha", &[]),
            Ok("packages/alpha".to_owned())
        );
        assert_eq!(
            normalize_member(r"..\..\packages\alpha", &["common", "temp"]),
            Ok("packages/alpha".to_owned())
        );
    }

    #[test]
    fn normalize_member_rejects_absolute_and_unrepresentable_paths() {
        for raw in [
            "/abs/packages/alpha",
            r"\abs\alpha",
            r"\\server\share\alpha",
            "C:/repo/alpha",
            r"c:\repo\alpha",
            "c:relative",
            "packages/al\0pha",
        ] {
            assert_eq!(
                normalize_member(raw, &[]),
                Err(LockfileReason::InvalidMemberPath),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn recorded_member_set_excludes_the_root_and_deduplicates_spellings() {
        let recorded: Vec<String> = [".", "packages/alpha", "./packages/alpha/", "packages/beta"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let set = recorded_member_set(&recorded, &[]).expect("valid paths");
        assert_eq!(
            set.into_iter().collect::<Vec<_>>(),
            ["packages/alpha", "packages/beta"]
        );
    }

    #[test]
    fn recorded_member_set_rejects_the_whole_set_for_one_invalid_path() {
        let recorded = vec!["packages/alpha".to_owned(), "/etc/passwd".to_owned()];
        assert_eq!(
            recorded_member_set(&recorded, &[]),
            Err(LockfileReason::InvalidMemberPath)
        );
    }

    #[test]
    fn manifest_member_uses_forward_slashes_for_native_paths() {
        let root = std::env::temp_dir().join("layer-root");
        let member = root.join("packages").join(".hidden");
        assert_eq!(
            manifest_member(&root, &member),
            Ok("packages/.hidden".to_owned())
        );
        assert_eq!(manifest_member(&root, &root), Ok(String::new()));
    }

    #[test]
    fn manifest_member_keeps_parent_components_for_an_external_member() {
        let parent = std::env::temp_dir().join("outer");
        let root = parent.join("layer");
        let member = parent.join("sibling");
        assert_eq!(manifest_member(&root, &member), Ok("../sibling".to_owned()));
    }

    #[cfg(unix)]
    #[test]
    fn manifest_member_rejects_a_non_utf8_name() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let root = Path::new("/repo");
        let member = root.join(OsStr::from_bytes(b"packages/\xff"));
        assert_eq!(
            manifest_member(root, &member),
            Err(LockfileReason::InvalidMemberPath)
        );
    }

    #[cfg(windows)]
    #[test]
    fn manifest_member_rejects_a_member_on_another_drive() {
        let root = Path::new(r"C:\repo");
        let member = Path::new(r"D:\elsewhere\alpha");
        assert_eq!(
            manifest_member(root, member),
            Err(LockfileReason::InvalidMemberPath)
        );
    }

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn compare_reports_match_only_on_set_equality() {
        let paths = vec!["pnpm-lock.yaml".to_owned()];
        let observation = compare(&set(&["a", "b"]), &set(&["a", "b"]), paths.clone());
        assert_eq!(observation.status, LockfileStatus::Match);
        assert_eq!(observation.reason, None);
        assert_eq!(observation.paths, paths);
        assert!(observation.extra.is_empty() && observation.missing.is_empty());

        let empty = compare(&set(&[]), &set(&[]), paths.clone());
        assert_eq!(empty.status, LockfileStatus::Match);
    }

    #[test]
    fn compare_reports_sorted_extra_and_missing_members() {
        let observation = compare(
            &set(&["packages/b", "packages/a", "packages/c"]),
            &set(&["packages/z", "packages/a", "packages/y"]),
            vec!["pnpm-lock.yaml".to_owned()],
        );
        assert_eq!(observation.status, LockfileStatus::Mismatch);
        assert_eq!(observation.reason, None);
        assert_eq!(observation.extra, ["packages/y", "packages/z"]);
        assert_eq!(observation.missing, ["packages/b", "packages/c"]);
    }
}
