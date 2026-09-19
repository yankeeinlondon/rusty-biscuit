//! One collection pipeline for [`RecentCommits`]: select history, filter it
//! inline, then enrich only the commits that survive.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use gix::bstr::ByteSlice;

use super::options::{RecentCommitsOptions, Selection};
use super::payload::{
    RecentCommit, RecentCommitAuthor, RecentCommitFile, RecentCommitFileTypes,
    RecentCommitPackages, RecentCommits,
};
use crate::filesystem::git::commit_links::{
    COMMIT_VISIT_BUDGET, link_commits, link_commits_from_snapshot,
};
use crate::filesystem::git::discovery::{
    committed_file_changes_with_cache, get_commit_files_with_cache_fallible, head_id_opt,
    resolve_single_opt,
};
use crate::filesystem::git::remote_refresh::RefSnapshot;
use crate::filesystem::git::remote_resolver::preferred_remote_order;
use crate::filesystem::git::{ConventionalCommit, GitRepo};
use crate::filesystem::path_kind::{ChangeCategory, classify_path};
use crate::filesystem::repo::ownership::PackageOwnershipIndex;
use crate::filesystem::repo::{RepoInfo, detect_repo_structure};
use crate::performance::{self, counters};
use crate::{Result, SniffError};

impl RecentCommits {
    /// Collect the commits `options` selects, newest first.
    ///
    /// History is walked from `HEAD`, or from the tip of `options`' branch,
    /// and every filter is applied during the walk: a count selection keeps
    /// walking until that many commits match or history ends. Only matching
    /// commits are then diffed, attributed to monorepo packages, and linked to
    /// remotes. Linking reads locally recorded remote-tracking refs only; no
    /// fetch or network request is made.
    ///
    /// A valid selection that matches nothing, including an unborn `HEAD`, is
    /// an empty collection rather than an error.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// use sniff::filesystem::git::{GitRepo, RecentCommits, RecentCommitsOptions};
    /// use std::path::Path;
    ///
    /// let repo = GitRepo::discover(Path::new("."))?.expect("inside a repository");
    /// let options = RecentCommitsOptions::new().count(5).operation("fix");
    /// let commits = RecentCommits::collect(&repo, &options)?;
    /// println!("{}", commits.to_json());
    /// # Ok::<(), sniff::SniffError>(())
    /// ```
    ///
    /// ## Errors
    ///
    /// - [`SniffError::InvalidPeriod`] for a zero count.
    /// - [`SniffError::UnknownBranch`] when the branch is neither a local nor a
    ///   remote-tracking branch.
    /// - [`SniffError::HashNotReachable`] when the hash names no commit or one
    ///   that is not an ancestor of the starting tip.
    /// - [`SniffError::NotAMonorepo`], [`SniffError::UnknownPackage`],
    ///   [`SniffError::AmbiguousPackage`], or [`SniffError::UnknownPackageArea`]
    ///   for a package or package-area filter the catalog cannot satisfy.
    /// - [`SniffError::Git`] for unreadable refs, objects, or history.
    pub fn collect(repo: &GitRepo, options: &RecentCommitsOptions) -> Result<Self> {
        options.validate()?;
        let structure = if repo.is_bare() {
            None
        } else {
            detect_repo_structure(repo.repo_root())?
        };
        let catalog = PackageCatalog::new(repo.repo_root(), structure, options)?;
        let now = Utc::now();
        let commits =
            repo.with_cached_gix(|gix| collect_from(gix, options, &catalog, now, None))?;
        Ok(commits.with_repo_root(repo.repo_root().to_path_buf()))
    }

    /// [`collect`](Self::collect) over a package catalog and ref snapshot the
    /// caller already observed for the same request, so neither repository
    /// structure nor refs are read a second time.
    ///
    /// `structure` must be the repository's detected catalog (any tier whose
    /// package list matches the structure tier's); `refs` must include
    /// remote branches.
    pub(crate) fn collect_observed(
        repo: &GitRepo,
        options: &RecentCommitsOptions,
        structure: Option<&RepoInfo>,
        refs: &RefSnapshot,
    ) -> Result<Self> {
        options.validate()?;
        let catalog = PackageCatalog::new(repo.repo_root(), structure.cloned(), options)?;
        let now = Utc::now();
        let commits =
            repo.with_cached_gix(|gix| collect_from(gix, options, &catalog, now, Some(refs)))?;
        Ok(commits.with_repo_root(repo.repo_root().to_path_buf()))
    }
}

fn collect_from(
    gix: &gix::Repository,
    options: &RecentCommitsOptions,
    catalog: &PackageCatalog,
    now: DateTime<Utc>,
    refs: Option<&RefSnapshot>,
) -> Result<RecentCommits> {
    let Some(tip) = resolve_tip(gix, options.branch.as_deref())? else {
        return Ok(RecentCommits::default());
    };
    let hidden = match &options.selection {
        Selection::Hash(hash) => hash_boundary(gix, tip, hash)?,
        _ => Vec::new(),
    };
    let count_limit = match options.selection {
        Selection::Count(count) => Some(count),
        _ => None,
    };
    let window = options.selection.time_window(now, options.timezone);
    let filters = MessageFilters::new(options);
    let path_filtered = catalog.filters_paths() || !options.file_types.is_empty();

    let walk = gix
        .rev_walk(Some(tip))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .with_hidden(hidden)
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;
    let mut diff_cache = gix
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    let mut candidates: Vec<Candidate> = Vec::new();
    for info_result in walk {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);

        // Git commit times are whole seconds. `ByCommitTime` is a lazy
        // frontier walk, so an out-of-window commit may still precede in-window
        // ancestors behind a skewed timestamp: skip it rather than stop.
        let datetime = DateTime::from_timestamp(info.commit_time(), 0).unwrap_or_default();
        let in_window = window.is_none_or(|window| window.contains(datetime));

        if in_window {
            let commit = info.object().map_err(|e| SniffError::git("object", e))?;
            let message = commit
                .message_raw()
                .map_err(|e| SniffError::git("message", e))?;
            let author = commit.author().map_err(|e| SniffError::git("author", e))?;
            let candidate = Candidate {
                id: info.id,
                datetime,
                message: String::from_utf8_lossy(message.trim()).into_owned(),
                author: RecentCommitAuthor {
                    name: author.name.to_str_lossy().into_owned(),
                    email: author.email.to_str_lossy().into_owned(),
                },
            };

            let matches = filters.matches(&candidate)
                && (!path_filtered || {
                    let paths: Vec<PathBuf> =
                        get_commit_files_with_cache_fallible(gix, info.id, &mut diff_cache)?
                            .into_iter()
                            .map(|(path, _)| path)
                            .collect();
                    catalog.matches(&paths) && has_every_category(&paths, &options.file_types)
                });
            if matches {
                candidates.push(candidate);
            }
        }

        if count_limit.is_some_and(|limit| candidates.len() >= limit) {
            break;
        }
    }

    // The candidate set is final: diff, attribute, and link each survivor once.
    let ids: Vec<gix::ObjectId> = candidates.iter().map(|candidate| candidate.id).collect();
    let links = match refs {
        Some(refs) => link_commits_from_snapshot(gix, refs, &ids, COMMIT_VISIT_BUDGET),
        None => link_commits(gix, &ids)?,
    };
    let mut commits = Vec::with_capacity(candidates.len());
    for (candidate, link) in candidates.into_iter().zip(links) {
        let files: Vec<RecentCommitFile> =
            committed_file_changes_with_cache(gix, candidate.id, &mut diff_cache)?
                .into_iter()
                .map(RecentCommitFile::from)
                .collect();
        let message = ParsedMessage::parse(&candidate.message);
        commits.push(RecentCommit {
            hash: candidate.id.to_string(),
            datetime: candidate.datetime,
            author: candidate.author,
            operation: message.operation,
            scope: message.scope,
            heading: message.heading,
            description: message.description,
            bullet_points: message.bullet_points,
            file_types: RecentCommitFileTypes::from_files(&files),
            attribution: catalog.attribute(&files),
            files,
            remote: link.remote,
            commit_url: link.commit_url,
        });
    }
    Ok(RecentCommits::from_commits(commits))
}

/// A commit that passed selection, before diff, attribution, and linking.
struct Candidate {
    id: gix::ObjectId,
    datetime: DateTime<Utc>,
    message: String,
    author: RecentCommitAuthor,
}

/// The history walk's starting commit, or `None` for an unborn `HEAD`.
///
/// A branch resolves as a local branch, then as a remote-tracking name such as
/// `origin/feature`, then as `feature` under each remote in preferred-remote
/// order. Nothing is fetched.
fn resolve_tip(gix: &gix::Repository, branch: Option<&str>) -> Result<Option<gix::ObjectId>> {
    let Some(branch) = branch else {
        return head_id_opt(gix);
    };
    if let Some(tip) = reference_tip(gix, &format!("refs/heads/{branch}"))? {
        return Ok(Some(tip));
    }
    if let Some(tip) = reference_tip(gix, &format!("refs/remotes/{branch}"))? {
        return Ok(Some(tip));
    }
    let remote_names: Vec<String> = gix
        .remote_names()
        .into_iter()
        .map(|name| name.to_string())
        .collect();
    for remote in preferred_remote_order(remote_names.iter().map(String::as_str)) {
        if let Some(tip) = reference_tip(gix, &format!("refs/remotes/{remote}/{branch}"))? {
            return Ok(Some(tip));
        }
    }
    Err(SniffError::UnknownBranch {
        name: branch.to_string(),
    })
}

/// The peeled tip of `full_name`, or `None` when no such reference exists or
/// the name is not a valid reference name.
fn reference_tip(gix: &gix::Repository, full_name: &str) -> Result<Option<gix::ObjectId>> {
    if gix::refs::FullName::try_from(full_name).is_err() {
        return Ok(None);
    }
    match gix.try_find_reference(full_name) {
        Ok(Some(reference)) => Ok(Some(
            reference
                .into_fully_peeled_id()
                .map_err(|e| SniffError::git("peel", e))?
                .detach(),
        )),
        Ok(None) => Ok(None),
        Err(e) => Err(SniffError::git("find_reference", e)),
    }
}

/// The commits to hide so a hash selection yields the commit `hash` names plus
/// everything reachable from `tip` that is not already behind it — the
/// `<hash>^@..<tip>` range, inclusive of the boundary.
///
/// Hiding the boundary's parents makes the range a question of graph
/// membership. Stopping the walk once the boundary is yielded would not:
/// commit times are not monotonic across a merge, so a future-dated boundary
/// can surface through one parent while an older descendant is still queued
/// behind the other.
fn hash_boundary(
    gix: &gix::Repository,
    tip: gix::ObjectId,
    hash: &str,
) -> Result<Vec<gix::ObjectId>> {
    let target = resolve_ancestor(gix, tip, hash)?;
    let commit = gix
        .find_commit(target)
        .map_err(|e| SniffError::git("commit", e))?;
    Ok(commit.parent_ids().map(|id| id.detach()).collect())
}

/// Resolve `hash` to a commit that is `tip` or one of its ancestors.
fn resolve_ancestor(gix: &gix::Repository, tip: gix::ObjectId, hash: &str) -> Result<gix::ObjectId> {
    let not_reachable = || SniffError::HashNotReachable {
        hash: hash.to_string(),
    };
    let target = resolve_single_opt(gix, hash)?.ok_or_else(not_reachable)?;
    if target == tip {
        return Ok(target);
    }
    // No merge base (unrelated history, or a non-commit object) means the hash
    // cannot be on this history.
    match gix.merge_base(tip, target) {
        Ok(base) if base.detach() == target => Ok(target),
        _ => Err(not_reachable()),
    }
}

/// Filters decided from the commit object alone, checked before any diff.
struct MessageFilters {
    operations: Vec<String>,
    scope: Option<String>,
    author: Option<String>,
}

impl MessageFilters {
    fn new(options: &RecentCommitsOptions) -> Self {
        Self {
            operations: options.operations.clone(),
            scope: options.scope.clone(),
            author: options.author.as_ref().map(|author| author.to_lowercase()),
        }
    }

    fn matches(&self, candidate: &Candidate) -> bool {
        if !self.operations.is_empty() || self.scope.is_some() {
            let conventional = ConventionalCommit::parse(&candidate.message);
            if !self.operations.is_empty() {
                let Some(operation) = conventional.operation.as_deref() else {
                    return false;
                };
                if !self
                    .operations
                    .iter()
                    .any(|wanted| wanted.eq_ignore_ascii_case(operation))
                {
                    return false;
                }
            }
            if let Some(wanted) = &self.scope
                && !conventional
                    .scope
                    .as_deref()
                    .is_some_and(|scope| scope.eq_ignore_ascii_case(wanted))
            {
                return false;
            }
        }
        if let Some(wanted) = &self.author {
            let author = &candidate.author;
            if !author.name.to_lowercase().contains(wanted)
                && !author.email.to_lowercase().contains(wanted)
            {
                return false;
            }
        }
        true
    }
}

fn has_every_category(paths: &[PathBuf], wanted: &BTreeSet<ChangeCategory>) -> bool {
    if wanted.is_empty() {
        return true;
    }
    let touched: BTreeSet<ChangeCategory> = paths.iter().map(|path| classify_path(path)).collect();
    wanted.is_subset(&touched)
}

/// The structure-tier package catalog for one collection, shared by
/// attribution and the package filters so paths are resolved through one
/// ownership index.
#[derive(Default)]
struct PackageCatalog {
    /// Present only for a monorepo with a package list.
    monorepo: Option<(RepoInfo, PackageOwnershipIndex)>,
    /// Index into the package list of the `package` filter's package.
    package: Option<usize>,
    /// Indices of the packages inside the `package_area` filter's area.
    area_packages: Option<Vec<usize>>,
}

impl PackageCatalog {
    /// Index a detected structure catalog and validate the package filters
    /// against it.
    fn new(
        repo_root: &Path,
        info: Option<RepoInfo>,
        options: &RecentCommitsOptions,
    ) -> Result<Self> {
        let monorepo = info
            .filter(|info| info.is_monorepo && info.packages.is_some())
            .map(|info| {
                let index = PackageOwnershipIndex::from_packages(
                    &info.root,
                    info.packages.as_deref().unwrap_or_default(),
                );
                (info, index)
            });

        if options.package.is_none() && options.package_area.is_none() {
            return Ok(Self {
                monorepo,
                ..Self::default()
            });
        }
        let Some(packages) = monorepo
            .as_ref()
            .and_then(|(info, _)| info.packages.as_deref())
        else {
            return Err(SniffError::NotAMonorepo(repo_root.to_path_buf()));
        };

        let mut package = None;
        let mut area_packages = None;
        if let Some(name) = &options.package {
            let matches: Vec<usize> = packages
                .iter()
                .enumerate()
                .filter(|(_, package)| package.name.eq_ignore_ascii_case(name))
                .map(|(index, _)| index)
                .collect();
            package = Some(match matches.as_slice() {
                [index] => *index,
                [] => {
                    return Err(SniffError::UnknownPackage {
                        name: name.clone(),
                        valid: packages
                            .iter()
                            .map(|package| package.name.as_str())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect::<Vec<_>>()
                            .join(", "),
                    });
                }
                _ => {
                    return Err(SniffError::AmbiguousPackage {
                        name: name.clone(),
                        count: matches.len(),
                        matches: matches
                            .iter()
                            .map(|index| packages[*index].relative.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    });
                }
            });
        }

        if let Some(area) = &options.package_area {
            let area = area.to_ascii_lowercase();
            let prefix = format!("{area}/");
            let members: Vec<usize> = packages
                .iter()
                .enumerate()
                .filter(|(_, package)| {
                    let package_area = package.package_area.to_ascii_lowercase();
                    package_area == area || package_area.starts_with(&prefix)
                })
                .map(|(index, _)| index)
                .collect();
            if members.is_empty() {
                return Err(SniffError::UnknownPackageArea {
                    area: options.package_area.clone().unwrap_or_default(),
                    valid: packages
                        .iter()
                        .map(|package| package.package_area.as_str())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
            area_packages = Some(members);
        }
        Ok(Self {
            monorepo,
            package,
            area_packages,
        })
    }

    fn packages(&self) -> Option<&[crate::filesystem::repo::Package]> {
        self.monorepo
            .as_ref()
            .and_then(|(info, _)| info.packages.as_deref())
    }

    fn owner(&self, path: &Path) -> Option<usize> {
        self.monorepo
            .as_ref()
            .and_then(|(_, index)| index.lookup_relative(path))
    }

    fn filters_paths(&self) -> bool {
        self.package.is_some() || self.area_packages.is_some()
    }

    /// Whether the changed `paths` satisfy both package filters.
    fn matches(&self, paths: &[PathBuf]) -> bool {
        let owners: Vec<usize> = paths.iter().filter_map(|path| self.owner(path)).collect();
        self.package.is_none_or(|package| owners.contains(&package))
            && self
                .area_packages
                .as_ref()
                .is_none_or(|members| owners.iter().any(|owner| members.contains(owner)))
    }

    /// Package attribution for a monorepo commit; `None` outside a monorepo.
    ///
    /// A file is attributed only to the deepest package owning it. A file
    /// directly under a package area but outside every package is left
    /// unattributed. A moved file's source path counts too.
    fn attribute(&self, files: &[RecentCommitFile]) -> Option<RecentCommitPackages> {
        let packages = self.packages()?;
        let mut names = BTreeSet::new();
        let mut areas = BTreeSet::new();
        let paths = files.iter().flat_map(|file| {
            std::iter::once(file.path.as_str()).chain(file.original_path.as_deref())
        });
        for path in paths {
            if let Some(owner) = self.owner(Path::new(path)) {
                names.insert(packages[owner].name.clone());
                areas.insert(packages[owner].package_area.clone());
            }
        }
        Some(RecentCommitPackages {
            packages: names.into_iter().collect(),
            package_areas: areas.into_iter().collect(),
        })
    }
}

/// The payload's message fields, parsed per spec Decision 15.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedMessage {
    pub(crate) operation: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) heading: String,
    pub(crate) description: String,
    pub(crate) bullet_points: Vec<String>,
}

impl ParsedMessage {
    /// Parse a raw commit message.
    ///
    /// - The operation and scope come from a conventional-commit subject line.
    /// - The heading is the subject (after any conventional prefix) up to its
    ///   first `.`, so an abbreviation such as `e.g.` ends it early.
    /// - The description is the remaining prose, with lines and paragraphs
    ///   joined by single spaces, up to the first `- ` or `* ` bullet line. It
    ///   is empty when there is no such prose.
    /// - Each bullet keeps its order. A non-blank line directly below a
    ///   bullet continues it, and anything after a blank line that follows the
    ///   bullets (such as trailers) is dropped.
    pub(crate) fn parse(message: &str) -> Self {
        let message = message.trim();
        let (subject, body) = message.split_once('\n').unwrap_or((message, ""));
        let conventional = ConventionalCommit::parse(subject);
        let title = conventional.description.trim();
        let (heading, rest) = title.split_once('.').unwrap_or((title, ""));

        let mut prose: Vec<&str> = Vec::new();
        let rest = rest.trim();
        if !rest.is_empty() {
            prose.push(rest);
        }
        let mut bullet_points: Vec<String> = Vec::new();
        let mut continues_bullet = false;
        for line in body.lines() {
            let line = line.trim();
            if let Some(bullet) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                bullet_points.push(bullet.trim().to_string());
                continues_bullet = true;
            } else if line.is_empty() {
                continues_bullet = false;
            } else if bullet_points.is_empty() {
                prose.push(line);
            } else if continues_bullet && let Some(last) = bullet_points.last_mut() {
                last.push(' ');
                last.push_str(line);
            }
        }

        Self {
            operation: conventional.operation,
            scope: conventional.scope,
            heading: heading.trim().to_string(),
            description: prose.join(" "),
            bullet_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod message_parsing {
        use super::*;

        fn parse(message: &str) -> ParsedMessage {
            ParsedMessage::parse(message)
        }

        #[test]
        fn conventional_subject_splits_operation_scope_and_heading() {
            let parsed = parse("feat(sniff): add recent commits. Second sentence");
            assert_eq!(parsed.operation.as_deref(), Some("feat"));
            assert_eq!(parsed.scope.as_deref(), Some("sniff"));
            assert_eq!(parsed.heading, "add recent commits");
            assert_eq!(parsed.description, "Second sentence");
            assert!(parsed.bullet_points.is_empty());
        }

        #[test]
        fn abbreviation_truncates_the_heading_at_its_first_period() {
            let parsed = parse("fix: use e.g. foo");
            assert_eq!(parsed.operation.as_deref(), Some("fix"));
            assert_eq!(parsed.heading, "use e");
            assert_eq!(parsed.description, "g. foo");
        }

        #[test]
        fn punctuation_free_subject_is_the_whole_heading_with_empty_description() {
            let parsed = parse("chore: bump dependencies");
            assert_eq!(parsed.heading, "bump dependencies");
            assert_eq!(parsed.description, "");
            assert!(parsed.bullet_points.is_empty());
        }

        #[test]
        fn heading_ends_at_the_newline_and_multi_paragraph_prose_joins() {
            let parsed = parse(
                "refactor(lib): split module\n\nFirst paragraph\nwraps here.\n\nSecond paragraph.\n\n- one\n- two",
            );
            assert_eq!(parsed.heading, "split module");
            assert_eq!(
                parsed.description,
                "First paragraph wraps here. Second paragraph."
            );
            assert_eq!(parsed.bullet_points, ["one", "two"]);
        }

        #[test]
        fn bullet_only_body_keeps_order_and_an_empty_description() {
            let parsed = parse("feat: add\n\n* first\n- second\n* third");
            assert_eq!(parsed.heading, "add");
            assert_eq!(parsed.description, "");
            assert_eq!(parsed.bullet_points, ["first", "second", "third"]);
        }

        #[test]
        fn wrapped_bullet_lines_continue_and_later_paragraphs_are_dropped() {
            let parsed = parse(
                "fix: x\n\n- a bullet that\n  wraps onto two lines\n- b\n\nCo-authored-by: Someone <s@example.com>",
            );
            assert_eq!(
                parsed.bullet_points,
                ["a bullet that wraps onto two lines", "b"]
            );
            assert_eq!(parsed.description, "");
        }

        #[test]
        fn non_conventional_subject_has_no_operation_or_scope() {
            let parsed = parse("Merge branch 'main' into feature");
            assert_eq!(parsed.operation, None);
            assert_eq!(parsed.scope, None);
            assert_eq!(parsed.heading, "Merge branch 'main' into feature");
        }

        #[test]
        fn operation_and_scope_come_from_the_subject_line_only() {
            let parsed = parse("Update readme\n\nfeat(body): not a subject");
            assert_eq!(parsed.operation, None);
            assert_eq!(parsed.scope, None);
            assert_eq!(parsed.description, "feat(body): not a subject");
        }

        #[test]
        fn empty_message_parses_to_empty_fields() {
            let parsed = parse("  \n ");
            assert_eq!(parsed.heading, "");
            assert_eq!(parsed.description, "");
            assert!(parsed.bullet_points.is_empty());
        }
    }
}
