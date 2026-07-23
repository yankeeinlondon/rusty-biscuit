//! Cross-platform, dialect-aware expansion of workspace membership globs.
//!
//! Workspace manifests (`Cargo.toml`, `pnpm-workspace.yaml`, `package.json`)
//! declare their members with glob patterns whose semantics differ per
//! standard. [`expand_membership_globs`] interprets those patterns using the
//! correct [`GlobDialect`] and resolves them to [`Package`] values.
//!
//! Workspace files always use `/` separators, even on Windows. Patterns and
//! candidate paths are normalized to slash-separated logical paths before
//! matching, then converted back to native [`PathBuf`]s at the final
//! [`PackageSeed`] step.
//!
//! This replaces the former `prefix*`-only expander, which silently missed
//! deep (`**`), brace (`{a,b}`), and negated (`!`) patterns.

use std::collections::BTreeSet;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use tracing::debug;

use crate::filesystem::file_types::should_skip_directory_name;
use crate::performance;
use crate::performance::counters;

use super::detection::{RepoEvidence, probe_exists, probe_is_dir};
use super::seed::PackageSeed;
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

/// Manifest file names that mark a directory as a package boundary.
const MANIFEST_FILES: [&str; 4] = ["Cargo.toml", "package.json", "pyproject.toml", "go.mod"];

/// Expand workspace membership `patterns` into [`PackageSeed`] values.
///
/// Explicit (glob-free) patterns are resolved by directory existence alone, so
/// a declared member directory without a manifest is still reported — this
/// preserves the historical contract for literal workspace members. Glob
/// patterns are matched against manifest-bearing directories discovered by a
/// single bounded walk, so a `**` pattern resolves package boundaries instead
/// of every intermediate directory.
///
/// ## Notes
///
/// `dialect` selects the pattern grammar: [`GlobDialect::Cargo`] accepts only
/// Cargo's documented subset (prefix `*`, full-component `**`) and rejects
/// brace expansion and negation; [`GlobDialect::Minimatch`] supports `**`,
/// `{a,b}`, and a leading `!` exclusion.
pub(crate) fn expand_membership_globs(
    root: &Path,
    patterns: &[String],
    dialect: GlobDialect,
    standard: MonorepoStandard,
    provenance: Option<PackageProvenance>,
    evidence: RepoEvidence<'_>,
) -> Vec<PackageSeed> {
    let provenance = provenance.unwrap_or_else(|| standard.membership_provenance());
    // BTreeSet dedupes directories matched by overlapping patterns and yields a
    // deterministic order before the (relatively expensive) package build.
    let mut matched: BTreeSet<PathBuf> = BTreeSet::new();

    let mut include_globs: Vec<String> = Vec::new();
    let mut exclude_globs: Vec<String> = Vec::new();

    for pattern in patterns {
        let normalized = to_logical(pattern.trim());
        if normalized.is_empty() {
            continue;
        }

        if dialect == GlobDialect::Minimatch
            && let Some(negated) = normalized.strip_prefix('!')
        {
            exclude_globs.push(negated.to_string());
            continue;
        }

        if !is_glob(&normalized) {
            let native = PathBuf::from(normalized.replace('/', MAIN_SEPARATOR_STR));
            let path = root.join(native);
            if probe_exists(&path) {
                matched.insert(path);
            }
            continue;
        }

        if dialect == GlobDialect::Cargo && !cargo_pattern_valid(&normalized) {
            debug!(
                pattern = %pattern,
                "rejecting unsupported Cargo workspace member glob"
            );
            continue;
        }

        include_globs.push(normalized);
    }

    if !include_globs.is_empty() {
        let include_set = build_globset(&include_globs);
        let exclude_set = build_globset(&exclude_globs);
        for dir in manifest_dirs(root, &include_globs, evidence) {
            let Ok(relative) = dir.strip_prefix(root) else {
                continue;
            };
            let logical = to_logical(&relative.to_string_lossy());
            if include_set.is_match(&logical) && !exclude_set.is_match(&logical) {
                matched.insert(dir);
            }
        }
    }

    matched
        .into_iter()
        .map(|path| PackageSeed::new(&path, root, standard, provenance))
        .collect()
}

/// Normalize a path or pattern to a slash-separated logical form.
///
/// Workspace manifests use `/` on every platform; candidate paths may use the
/// native separator. Normalizing both before matching keeps the matcher
/// platform-independent.
fn to_logical(value: &str) -> String {
    value.replace('\\', "/")
}

/// Whether a pattern contains any glob metacharacter.
fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', '{'])
}

/// Whether a Cargo workspace member glob is within Cargo's documented subset.
///
/// Cargo accepts a trailing `*` on a path component (prefix match) and `**`
/// only as a complete, trailing component (Cargo 1.74+). Brace expansion and
/// negation are not Cargo syntax, a `**` embedded inside a component (`a**b`)
/// is rejected, and a mid-path `**` (`a/**/b`) is rejected.
fn cargo_pattern_valid(pattern: &str) -> bool {
    if pattern.starts_with('!') || pattern.contains(['{', '}']) {
        return false;
    }
    let components: Vec<&str> = pattern.split('/').collect();
    let last = components.len() - 1;
    components.iter().enumerate().all(|(index, component)| {
        !component.contains("**") || (*component == "**" && index == last)
    })
}

/// Compile a slash-form pattern list into a [`GlobSet`].
///
/// `literal_separator(true)` makes `*` and `?` stop at `/` (single-component
/// matches) while `**` spans separators — the minimatch semantics Cargo's
/// subset also obeys. An unparseable pattern is logged and skipped rather than
/// aborting the whole set.
fn build_globset(patterns: &[String]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match GlobBuilder::new(pattern).literal_separator(true).build() {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(error) => debug!(pattern = %pattern, %error, "skipping invalid membership glob"),
        }
    }
    builder.build().unwrap_or_else(|_| GlobSet::empty())
}

/// Every manifest-bearing directory a glob could match, from observed evidence
/// when available and from a bounded walk otherwise.
///
/// ## Notes
///
/// The two sources agree. `walk_manifest_dirs` only enumerates each pattern's
/// literal-prefix subtree, while the observed set spans the whole observation
/// root — but a glob always begins with its own literal prefix, so a manifest
/// directory outside that prefix cannot match it. Filtering by pattern
/// afterwards is therefore equivalent to bounding the walk beforehand.
///
/// The evidence is deliberately the unfiltered `manifest_dirs` rather than the
/// `ManifestIndex`: the index drops generated and fixture manifests because
/// they are not discovery boundaries, but membership globs resolve a boundary
/// by marker presence alone and always have.
fn manifest_dirs(
    root: &Path,
    include_globs: &[String],
    evidence: RepoEvidence<'_>,
) -> Vec<PathBuf> {
    match evidence.manifest_dirs {
        Some(dirs) => dirs.to_vec(),
        None => walk_manifest_dirs(root, include_globs),
    }
}

/// Walk the bounded subtrees implied by `include_globs` and yield every
/// manifest-bearing directory.
///
/// Each pattern's literal prefix (the components before its first glob
/// metacharacter) bounds the walk, so `packages/*` walks only `packages/`
/// rather than the entire repository. Roots subsumed by an ancestor root are
/// dropped so overlapping patterns walk each subtree once.
fn walk_manifest_dirs(root: &Path, include_globs: &[String]) -> Vec<PathBuf> {
    let walk_roots = resolve_walk_roots(root, include_globs);
    let mut dirs = Vec::new();

    for walk_root in walk_roots {
        if !probe_is_dir(&walk_root) {
            continue;
        }
        performance::increment_counter(counters::FS_READ_DIRS, 1);
        performance::increment_counter(counters::REPO_MEMBERSHIP_GLOB_WALKS, 1);
        let walker = WalkBuilder::new(&walk_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|entry| {
                if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    return true;
                }
                !entry
                    .file_name()
                    .to_str()
                    .is_some_and(should_skip_directory_name)
            })
            .build();

        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }
            if dir_has_manifest(entry.path()) {
                dirs.push(entry.path().to_path_buf());
            }
        }
    }

    dirs
}

/// Resolve the minimal set of subtree roots to walk for `include_globs`.
fn resolve_walk_roots(root: &Path, include_globs: &[String]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = include_globs
        .iter()
        .map(|pattern| root.join(literal_prefix(pattern)))
        .collect();
    roots.sort();
    roots.dedup();

    // Drop any root that lives under another root; the ancestor's walk already
    // covers it. `root` itself subsumes everything.
    roots
        .iter()
        .filter(|candidate| {
            !roots
                .iter()
                .any(|other| *candidate != other && candidate.starts_with(other))
        })
        .cloned()
        .collect()
}

/// The native-path literal prefix of a slash-form glob: the components before
/// the first one containing a glob metacharacter.
fn literal_prefix(pattern: &str) -> PathBuf {
    let mut prefix = PathBuf::new();
    for component in pattern.split('/') {
        if is_glob(component) {
            break;
        }
        prefix.push(component);
    }
    prefix
}

/// Whether `dir` contains a recognized package manifest.
fn dir_has_manifest(dir: &Path) -> bool {
    MANIFEST_FILES
        .iter()
        .any(|name| probe_exists(&dir.join(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(patterns: &[&str]) -> GlobSet {
        let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        build_globset(&owned)
    }

    #[test]
    fn cargo_prefix_glob_matches_single_component_only() {
        let globs = set(&["members/*"]);
        assert!(globs.is_match("members/a"));
        assert!(!globs.is_match("members/a/b"));
        assert!(!globs.is_match("members"));
    }

    #[test]
    fn cargo_double_star_is_full_component_and_rejects_mid_path() {
        assert!(cargo_pattern_valid("members/**"));
        assert!(cargo_pattern_valid("crates/*"));
        assert!(!cargo_pattern_valid("members/**/pkg"));
        assert!(!cargo_pattern_valid("a**b"));
        assert!(!cargo_pattern_valid("packages/{app,lib}"));
        assert!(!cargo_pattern_valid("!excluded"));
    }

    #[test]
    fn minimatch_double_star_matches_arbitrary_depth() {
        let globs = set(&["members/**"]);
        assert!(globs.is_match("members/a"));
        assert!(globs.is_match("members/a/b/c"));
    }

    #[test]
    fn minimatch_brace_expansion_matches_alternatives() {
        let globs = set(&["packages/{app,lib}/*"]);
        assert!(globs.is_match("packages/app/one"));
        assert!(globs.is_match("packages/lib/two"));
        assert!(!globs.is_match("packages/other/three"));
    }

    #[test]
    fn minimatch_negation_partitions_into_exclude_set() {
        let include = set(&["packages/*"]);
        let exclude = set(&["packages/excluded"]);
        assert!(include.is_match("packages/kept"));
        assert!(include.is_match("packages/excluded"));
        // The excluded path matches the include glob but is removed by the
        // exclude set — mirroring the runtime `include && !exclude` rule.
        assert!(exclude.is_match("packages/excluded"));
        assert!(!exclude.is_match("packages/kept"));
    }

    #[test]
    fn windows_style_candidate_matches_posix_pattern_after_normalization() {
        let globs = set(&["packages/*"]);
        let candidate = to_logical("packages\\app");
        assert_eq!(candidate, "packages/app");
        assert!(globs.is_match(&candidate));
    }

    #[test]
    fn literal_prefix_stops_at_first_glob_component() {
        assert_eq!(literal_prefix("packages/*"), PathBuf::from("packages"));
        assert_eq!(literal_prefix("a/b/{c,d}/*"), PathBuf::from("a").join("b"));
        assert_eq!(literal_prefix("**"), PathBuf::new());
        assert_eq!(literal_prefix("crates/cli"), PathBuf::from("crates/cli"));
    }

    #[test]
    fn resolve_walk_roots_drops_subsumed_descendants() {
        let root = Path::new("/repo");
        let roots = resolve_walk_roots(
            root,
            &["packages/*".to_string(), "packages/app/*".to_string()],
        );
        assert_eq!(roots, vec![PathBuf::from("/repo/packages")]);
    }

    #[test]
    fn is_glob_detects_metacharacters() {
        assert!(is_glob("a/*"));
        assert!(is_glob("a/**"));
        assert!(is_glob("a/{b,c}"));
        assert!(is_glob("a/[bc]"));
        assert!(!is_glob("a/b/c"));
    }
}
