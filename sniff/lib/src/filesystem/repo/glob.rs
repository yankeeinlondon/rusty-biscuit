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

/// The file names sniff treats as package manifests: a directory holding one
/// of them is a package boundary. This is the single list sniff uses for
/// package detection; a consumer that watches for package changes (for
/// example, a language server) should use it rather than copying the names.
pub const PACKAGE_MANIFEST_FILE_NAMES: &[&str] =
    &["Cargo.toml", "package.json", "pyproject.toml", "go.mod"];

/// The seeds a membership expansion resolved, and whether they are every
/// member the patterns declare.
///
/// ## Notes
///
/// Completeness is reported, never inferred (ruling R10 of
/// `2026-09-26-lockfile-corroboration`). The two signals stay separate because
/// standards disagree about a glob-free pattern naming a missing path: Cargo
/// and Rush reject it, while the Node tools and uv treat it as a glob that
/// matched nothing.
pub(crate) struct MembershipExpansion {
    pub(crate) seeds: Vec<PackageSeed>,
    /// `false` when a pattern was dropped as unsupported or unparseable, or
    /// when a directory a glob could match was not enumerated because the walk
    /// failed on it.
    pub(crate) patterns_resolved: bool,
    /// A glob-free pattern named a path that does not exist.
    pub(crate) missing_literal: bool,
}

/// The member patterns a workspace manifest declares.
///
/// ## Notes
///
/// An entry that is not a string is dropped from `patterns` but recorded in
/// `has_invalid`: the declared set is then not fully understood, so the
/// detector reports its outcome incomplete (ruling R10) and the layer is never
/// compared with its lockfile. A declaration whose every entry is invalid,
/// or a present member field that is not a list at all
/// ([`DeclaredPatterns::invalid`]), still yields a layer rather than
/// vanishing.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DeclaredPatterns {
    pub(crate) patterns: Vec<String>,
    pub(crate) has_invalid: bool,
}

impl<'a> FromIterator<Option<&'a str>> for DeclaredPatterns {
    fn from_iter<I: IntoIterator<Item = Option<&'a str>>>(entries: I) -> Self {
        let mut declared = Self::default();
        for entry in entries {
            match entry {
                Some(pattern) => declared.patterns.push(pattern.to_owned()),
                None => declared.has_invalid = true,
            }
        }
        declared
    }
}

impl DeclaredPatterns {
    /// A present member field whose value is not a list.
    pub(crate) fn invalid() -> Self {
        Self {
            patterns: Vec::new(),
            has_invalid: true,
        }
    }

    /// Whether the manifest declares no member entry at all, valid or not.
    pub(crate) fn is_empty(&self) -> bool {
        self.patterns.is_empty() && !self.has_invalid
    }
}

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
) -> MembershipExpansion {
    let provenance = provenance.unwrap_or_else(|| standard.membership_provenance());
    // BTreeSet dedupes directories matched by overlapping patterns and yields a
    // deterministic order before the (relatively expensive) package build.
    let mut matched: BTreeSet<PathBuf> = BTreeSet::new();
    let mut patterns_resolved = true;
    let mut missing_literal = false;

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
            } else {
                missing_literal = true;
            }
            continue;
        }

        if dialect == GlobDialect::Cargo && !cargo_pattern_valid(&normalized) {
            debug!(
                pattern = %pattern,
                "rejecting unsupported Cargo workspace member glob"
            );
            patterns_resolved = false;
            continue;
        }

        include_globs.push(normalized);
    }

    if !include_globs.is_empty() {
        let (include_set, include_valid) = build_globset(&include_globs);
        let (exclude_set, exclude_valid) = build_globset(&exclude_globs);
        let (dirs, walk_complete) = manifest_dirs(root, &include_globs, evidence);
        patterns_resolved &= include_valid && exclude_valid && walk_complete;
        for dir in dirs {
            let Ok(relative) = dir.strip_prefix(root) else {
                continue;
            };
            let logical = to_logical(&relative.to_string_lossy());
            if include_set.is_match(&logical) && !exclude_set.is_match(&logical) {
                matched.insert(dir);
            }
        }
    }

    let seeds = matched
        .into_iter()
        .map(|path| PackageSeed::new(&path, root, standard, provenance))
        .collect();
    MembershipExpansion {
        seeds,
        patterns_resolved,
        missing_literal,
    }
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
/// aborting the whole set; the returned flag is `false` when any was skipped.
fn build_globset(patterns: &[String]) -> (GlobSet, bool) {
    let mut builder = GlobSetBuilder::new();
    let mut valid = true;
    for pattern in patterns {
        match GlobBuilder::new(pattern).literal_separator(true).build() {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(error) => {
                debug!(pattern = %pattern, %error, "skipping invalid membership glob");
                valid = false;
            }
        }
    }
    match builder.build() {
        Ok(set) => (set, valid),
        Err(_) => (GlobSet::empty(), false),
    }
}

/// Every manifest-bearing directory a glob could match, from observed evidence
/// when available and from a bounded walk otherwise, and whether every
/// directory under the globs' walk roots was enumerated.
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
) -> (Vec<PathBuf>, bool) {
    match evidence.manifest_dirs {
        Some(dirs) => {
            let walk_roots = resolve_walk_roots(root, include_globs);
            let complete = !evidence
                .manifest_walk_errors
                .unwrap_or_default()
                .iter()
                .any(|failed| failure_affects(failed, &walk_roots));
            (dirs.to_vec(), complete)
        }
        None => walk_manifest_dirs(root, include_globs),
    }
}

/// Whether a walk failure at `failed` can hide a directory under one of
/// `walk_roots`: the failure is inside a walked subtree, or at one of its
/// ancestors.
fn failure_affects(failed: &Path, walk_roots: &[PathBuf]) -> bool {
    walk_roots
        .iter()
        .any(|walk_root| failed.starts_with(walk_root) || walk_root.starts_with(failed))
}

/// The path of a walk error that may have hidden entries, or `None` for an
/// error that hides nothing (an unparseable ignore rule, for example).
///
/// An I/O failure without a recorded path is attributed to `walk_root`, so it
/// affects every subtree that walk covers.
pub(crate) fn walk_failure_path(error: &ignore::Error, walk_root: &Path) -> Option<PathBuf> {
    fn recorded_path(error: &ignore::Error) -> Option<&Path> {
        match error {
            ignore::Error::WithPath { path, .. } => Some(path),
            ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
                recorded_path(err)
            }
            _ => None,
        }
    }
    error.io_error()?;
    Some(recorded_path(error).map_or_else(|| walk_root.to_path_buf(), Path::to_path_buf))
}

/// Walk the bounded subtrees implied by `include_globs` and yield every
/// manifest-bearing directory.
///
/// Each pattern's literal prefix (the components before its first glob
/// metacharacter) bounds the walk, so `packages/*` walks only `packages/`
/// rather than the entire repository. Roots subsumed by an ancestor root are
/// dropped so overlapping patterns walk each subtree once. The flag is `false`
/// when an entry could not be read.
fn walk_manifest_dirs(root: &Path, include_globs: &[String]) -> (Vec<PathBuf>, bool) {
    let walk_roots = resolve_walk_roots(root, include_globs);
    let mut dirs = Vec::new();
    let mut complete = true;

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

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    if let Some(failed) = walk_failure_path(&error, &walk_root) {
                        debug!(path = %failed.display(), %error, "membership glob walk failed");
                        complete = false;
                    }
                    continue;
                }
            };
            if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }
            if dir_has_manifest(entry.path()) {
                dirs.push(entry.path().to_path_buf());
            }
        }
    }

    (dirs, complete)
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
    PACKAGE_MANIFEST_FILE_NAMES
        .iter()
        .any(|name| probe_exists(&dir.join(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manifest_file_names_are_pinned() {
        assert_eq!(
            PACKAGE_MANIFEST_FILE_NAMES,
            &["Cargo.toml", "package.json", "pyproject.toml", "go.mod"]
        );
    }

    fn set(patterns: &[&str]) -> GlobSet {
        let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        build_globset(&owned).0
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

    fn expand(root: &Path, patterns: &[&str], dialect: GlobDialect) -> MembershipExpansion {
        expand_with(root, patterns, dialect, RepoEvidence::default())
    }

    fn expand_with(
        root: &Path,
        patterns: &[&str],
        dialect: GlobDialect,
        evidence: RepoEvidence<'_>,
    ) -> MembershipExpansion {
        let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        expand_membership_globs(
            root,
            &owned,
            dialect,
            MonorepoStandard::Unknown,
            Some(PackageProvenance::Globbed),
            evidence,
        )
    }

    fn members_fixture() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("tempdir");
        for name in ["a", "b"] {
            let member = dir.path().join("packages").join(name);
            std::fs::create_dir_all(&member).expect("create member");
            std::fs::write(member.join("package.json"), "{}").expect("write manifest");
        }
        dir
    }

    fn relatives(expansion: &MembershipExpansion) -> Vec<&str> {
        expansion
            .seeds
            .iter()
            .map(|seed| seed.relative.as_str())
            .collect()
    }

    #[test]
    fn a_fully_understood_expansion_is_complete() {
        let dir = members_fixture();
        let expansion = expand(dir.path(), &["packages/*", "tools/*"], GlobDialect::Minimatch);
        assert_eq!(relatives(&expansion), ["packages/a", "packages/b"]);
        assert!(expansion.patterns_resolved);
        assert!(!expansion.missing_literal, "a glob matching nothing is not a miss");
    }

    #[test]
    fn a_dropped_pattern_is_reported_unresolved() {
        let dir = members_fixture();
        for (patterns, dialect) in [
            (["packages/*", "tools/{a,b}"], GlobDialect::Cargo),
            (["packages/*", "tools/["], GlobDialect::Minimatch),
            (["packages/*", "!packages/["], GlobDialect::Minimatch),
        ] {
            let expansion = expand(dir.path(), &patterns, dialect);
            assert!(!expansion.patterns_resolved, "{patterns:?}");
            assert!(!expansion.missing_literal, "{patterns:?}");
        }
    }

    #[test]
    fn a_missing_literal_member_is_reported_separately() {
        let dir = members_fixture();
        let expansion = expand(
            dir.path(),
            &["packages/a", "packages/missing"],
            GlobDialect::Cargo,
        );
        assert_eq!(relatives(&expansion), ["packages/a"]);
        assert!(expansion.missing_literal);
        assert!(expansion.patterns_resolved);
    }

    #[test]
    fn an_observed_walk_failure_is_unresolved_only_where_a_glob_walks() {
        let dir = members_fixture();
        let root = dir.path();
        let dirs = [root.join("packages/a"), root.join("packages/b")];
        for (failed, resolved) in [
            (root.join("packages/b/node"), false),
            (root.to_path_buf(), false),
            (root.join("docs"), true),
        ] {
            let failures = [failed.clone()];
            let evidence = RepoEvidence {
                manifest_dirs: Some(&dirs),
                manifest_walk_errors: Some(&failures),
                ..RepoEvidence::default()
            };
            let expansion = expand_with(root, &["packages/*"], GlobDialect::Minimatch, evidence);
            assert_eq!(relatives(&expansion), ["packages/a", "packages/b"]);
            assert_eq!(expansion.patterns_resolved, resolved, "{}", failed.display());
        }
    }

    #[test]
    fn only_io_walk_errors_have_a_failure_path() {
        let walk_root = Path::new("/repo/packages");
        let io = || Box::new(ignore::Error::Io(std::io::Error::other("denied")));
        let with_path = ignore::Error::WithDepth {
            depth: 1,
            err: Box::new(ignore::Error::WithPath {
                path: PathBuf::from("/repo/packages/a"),
                err: io(),
            }),
        };
        assert_eq!(
            walk_failure_path(&with_path, walk_root),
            Some(PathBuf::from("/repo/packages/a"))
        );
        assert_eq!(
            walk_failure_path(&ignore::Error::Io(std::io::Error::other("denied")), walk_root),
            Some(walk_root.to_path_buf())
        );
        let ignore_rule = ignore::Error::Glob {
            glob: Some("[".to_owned()),
            err: "unclosed class".to_owned(),
        };
        assert_eq!(walk_failure_path(&ignore_rule, walk_root), None);
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
