//! Hermetic committed-tip merge conflict prediction.
//!
//! The pinned `gix` facade's `Repository::merge_commits()` cannot enforce this
//! module's boundary because its resource cache consults the live index for
//! attributes. This module therefore uses `gix::merge::plumbing::commit` with
//! an index and attribute stack built only from the captured `ours` tree. The
//! probe enables object-memory storage before merging, so merged blobs, trees,
//! and virtual merge bases never reach the on-disk object database. No direct
//! plumbing dependency is needed because `gix` re-exports the required merge,
//! filter, worktree, index, and object APIs.
//!
//! Applicable external drivers, filters, and `merge.renormalize` are rejected
//! before the merge runs, not after: the built-in text driver is only a valid
//! approximation once nothing external could have changed the outcome, and a
//! clean built-in result is not evidence that it could not. "Applicable" is
//! scoped to the paths participating in the merge, so a merge touching no path
//! at all is exempt and still returns `[]`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gix::bstr::{BStr, BString, ByteSlice};

use crate::{Result, SniffError};

/// Merge `theirs` into `ours` without consulting or mutating live repository state.
pub(crate) fn merge_conflicts_between(
    repo: &gix::Repository,
    ours: gix::ObjectId,
    theirs: gix::ObjectId,
) -> Result<Vec<PathBuf>> {
    let probe = repo.clone().with_object_memory();
    let ours_commit = probe
        .find_commit(ours)
        .map_err(|error| SniffError::git("merge_ours_commit", error))?;
    let ours_tree = ours_commit
        .tree_id()
        .map_err(|error| SniffError::git("merge_ours_tree", error))?
        .detach();
    let theirs_commit = probe
        .find_commit(theirs)
        .map_err(|error| SniffError::git("merge_theirs_commit", error))?;
    let theirs_tree = theirs_commit
        .tree_id()
        .map_err(|error| SniffError::git("merge_theirs_tree", error))?
        .detach();

    let ours_index = probe
        .index_from_tree(&ours_tree)
        .map_err(|error| SniffError::git("merge_ours_index", error))?;

    let mut diff_cache = committed_diff_cache(&probe, &ours_index)?;

    // Rejection precedes the merge: the built-in approximation is only a valid
    // prediction once no external driver or filter could have changed its
    // outcome, and running it first would already have executed them.
    let participating =
        participating_paths(&probe, ours, theirs, ours_tree, theirs_tree, &mut diff_cache)?;
    reject_unsafe_configuration(&probe, &ours_index, &participating)?;

    let attribute_stack = committed_attribute_stack(&probe, &ours_index);
    let filter = gix::filter::plumbing::Pipeline::default();
    let pipeline = gix::merge::blob::Pipeline::new(Default::default(), filter, Default::default());
    let default_driver = probe
        .config_snapshot()
        .string("merge.default")
        .map(|value| value.into_owned());
    let mut blob_merge = gix::merge::blob::Platform::new(
        pipeline,
        gix::merge::blob::pipeline::Mode::ToGit,
        attribute_stack,
        Vec::new(),
        gix::merge::blob::platform::Options { default_driver },
    );
    let commit_graph = probe
        .commit_graph_if_enabled()
        .map_err(|error| SniffError::git("merge_commit_graph", error))?;
    let mut graph = probe.revision_graph(commit_graph.as_ref());
    let mut tree_options: gix::merge::plumbing::tree::Options = probe
        .tree_merge_options()
        .map_err(|error| SniffError::git("merge_options", error))?
        .into();
    tree_options.fail_on_conflict = None;
    let options = gix::merge::plumbing::commit::Options {
        allow_missing_merge_base: false,
        tree_merge: tree_options,
        use_first_merge_base: false,
    };
    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: None,
        current: Some(BStr::new(b"HEAD")),
        other: Some(BStr::new(b"MERGE_HEAD")),
    };
    let outcome = gix::merge::plumbing::commit(
        ours,
        theirs,
        labels,
        &mut graph,
        &mut diff_cache,
        &mut blob_merge,
        &probe,
        &mut |id| id.to_string(),
        options,
    )
    .map_err(|error| SniffError::git("merge", error))?;

    let mut merged_tree = outcome.tree_merge.tree;
    let merged_tree_id = merged_tree
        .write(|tree| probe.write_object(tree).map(|id| id.detach()))
        .map_err(|error| SniffError::git("merge_tree", error))?;
    let mut index = probe
        .index_from_tree(&merged_tree_id)
        .map_err(|error| SniffError::git("merge_index", error))?;
    gix::merge::tree::apply_index_entries(
        &outcome.tree_merge.conflicts,
        gix::merge::tree::TreatAsUnresolved::git(),
        &mut index,
        gix::merge::plumbing::tree::apply_index_entries::RemovalMode::Prune,
    );

    let state: &gix::index::State = &index;
    let mut paths = index
        .entries()
        .iter()
        .filter(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
        .map(|entry| lossy_path(entry.path(state)))
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| portable_path(path));
    paths.dedup_by(|left, right| portable_path(left) == portable_path(right));
    Ok(paths)
}

fn committed_diff_cache(
    repo: &gix::Repository,
    index: &gix::index::State,
) -> Result<gix::diff::blob::Platform> {
    let options = gix::filter::plumbing::pipeline::Options {
        object_hash: repo.object_hash(),
        ..Default::default()
    };
    let filter = gix::filter::plumbing::Pipeline::new(
        repo.command_context()
            .map_err(|error| SniffError::git("merge_command_context", error))?,
        options,
    );
    let pipeline = gix::diff::blob::Pipeline::new(
        Default::default(),
        filter,
        Vec::new(),
        Default::default(),
    );
    let options = gix::diff::blob::platform::Options {
        algorithm: Some(
            repo.diff_algorithm()
                .map_err(|error| SniffError::git("merge_diff_algorithm", error))?,
        ),
        skip_internal_diff_if_external_is_configured: false,
    };
    Ok(gix::diff::blob::Platform::new(
        options,
        pipeline,
        gix::diff::blob::pipeline::Mode::ToGit,
        committed_attribute_stack(repo, index),
    ))
}

fn committed_attribute_stack(
    repo: &gix::Repository,
    index: &gix::index::State,
) -> gix::worktree::Stack {
    let attributes = gix::worktree::stack::state::Attributes::new(
        Default::default(),
        None,
        gix::worktree::stack::state::attributes::Source::IdMapping,
        Default::default(),
    );
    let state = gix::worktree::stack::State::AttributesStack(attributes);
    let ignore_case = repo.config_snapshot().boolean("core.ignoreCase").unwrap_or(false);
    gix::worktree::Stack::from_state_and_ignore_case(
        repo.workdir().unwrap_or(repo.git_dir()),
        ignore_case,
        state,
        index,
        index.path_backing(),
    )
}

/// Committed paths taking part in the merge: every path that differs between a
/// merge base and either side.
///
/// This is deliberately wider than the set of paths the built-in merge reports
/// as conflicted. A path is only safe to predict once nothing external could
/// apply to it, and whether an external driver conflicts is exactly what the
/// built-in approximation cannot answer.
///
/// When no merge base is reachable — unrelated histories, or a base lookup that
/// fails outright — the sides are compared directly so the caller still receives
/// the real merge error rather than one raised from here.
///
/// `diff_cache` must be the committed-tree cache: the diff platform otherwise
/// builds its own from the live index, which this module must never read.
fn participating_paths(
    repo: &gix::Repository,
    ours: gix::ObjectId,
    theirs: gix::ObjectId,
    ours_tree: gix::ObjectId,
    theirs_tree: gix::ObjectId,
    diff_cache: &mut gix::diff::blob::Platform,
) -> Result<BTreeSet<BString>> {
    let bases = repo
        .merge_bases_many(ours, &[theirs])
        .map(|bases| bases.into_iter().map(|base| base.detach()).collect::<Vec<_>>())
        .unwrap_or_default();

    // A single merge base that is one of the sides means one tip already
    // contains the other, so the merge is a fast-forward or a no-op: git
    // resolves it by taking a side wholesale and never runs a three-way content
    // merge. No path participates, however far the tips have diverged in
    // content, so no driver, filter, or renormalization can apply.
    if bases.len() == 1 && (bases[0] == ours || bases[0] == theirs) {
        return Ok(BTreeSet::new());
    }

    let mut paths = BTreeSet::new();
    if bases.is_empty() {
        collect_tree_changes(repo, ours_tree, theirs_tree, diff_cache, &mut paths)?;
        return Ok(paths);
    }
    for base in bases {
        let base_tree = repo
            .find_commit(base)
            .map_err(|error| SniffError::git("merge_base_commit", error))?
            .tree_id()
            .map_err(|error| SniffError::git("merge_base_tree", error))?
            .detach();
        for side in [ours_tree, theirs_tree] {
            collect_tree_changes(repo, base_tree, side, diff_cache, &mut paths)?;
        }
    }
    Ok(paths)
}

fn collect_tree_changes(
    repo: &gix::Repository,
    from: gix::ObjectId,
    to: gix::ObjectId,
    diff_cache: &mut gix::diff::blob::Platform,
    out: &mut BTreeSet<BString>,
) -> Result<()> {
    let from = repo
        .find_tree(from)
        .map_err(|error| SniffError::git("merge_participating_from_tree", error))?;
    let to = repo
        .find_tree(to)
        .map_err(|error| SniffError::git("merge_participating_to_tree", error))?;
    from.changes()
        .map_err(|error| SniffError::git("merge_participating_diff", error))?
        // Rename tracking would read blobs to score similarity while adding
        // nothing: an untracked rename already yields both endpoints.
        .options(|options| {
            options.track_rewrites(None);
        })
        .for_each_to_obtain_tree_with_cache(&to, diff_cache, |change| {
            out.insert(change.location().to_owned());
            Ok::<_, std::convert::Infallible>(gix::object::tree::diff::Action::Continue(()))
        })
        .map_err(|error| SniffError::git("merge_participating_changes", error))?;
    Ok(())
}

/// Reject configuration whose external behavior the built-in merge cannot model.
///
/// No participating path means the merge is trivial — same-branch, ancestor,
/// already-contained, or fast-forward — and nothing is accepted on faith, so
/// even `merge.renormalize` is rejection-exempt: git takes a side wholesale and
/// leaves no content for a driver, filter, or renormalization to act on.
/// Rejecting here instead would fail the no-op merges that must return `[]`.
fn reject_unsafe_configuration(
    repo: &gix::Repository,
    ours_index: &gix::index::State,
    paths: &BTreeSet<BString>,
) -> Result<()> {
    let Some(representative) = paths.first() else {
        return Ok(());
    };

    if repo
        .config_snapshot()
        .boolean("merge.renormalize")
        .unwrap_or(false)
    {
        // Renormalization is repository-global rather than path-scoped, so it is
        // reported against a representative participating path.
        return Err(SniffError::UnsupportedMergeConfiguration {
            setting: "merge.renormalize".to_string(),
            path: lossy_path(representative.as_bstr()),
        });
    }

    let default_merge_driver = repo
        .config_snapshot()
        .string("merge.default")
        .map(|value| value.to_string());
    let mut stack = committed_attribute_stack(repo, ours_index);
    for path in paths {
        let mut selected = gix::filter::plumbing::attributes::search::Outcome::default();
        selected.initialize_with_selection(&Default::default(), ["merge", "filter"]);
        stack
            .at_entry(path.as_bstr(), None, repo)
            .map_err(|error| SniffError::git("merge_attributes", error))?
            .matching_attributes(&mut selected);
        let mut assignments = selected.iter_selected();
        let merge = assignments.next().expect("selected merge attribute");
        let filter = assignments.next().expect("selected filter attribute");
        let merge_name = match merge.assignment.state {
            gix::filter::plumbing::attributes::StateRef::Value(name) => {
                Some(name.as_bstr().to_str_lossy().into_owned())
            }
            gix::filter::plumbing::attributes::StateRef::Unspecified => {
                default_merge_driver.clone()
            }
            _ => None,
        };
        if let Some(name) = merge_name.filter(|name| !is_builtin_merge_driver(name)) {
            let setting = format!("merge.{name}.driver");
            if repo.config_snapshot().string(setting.as_str()).is_some() {
                return Err(SniffError::UnsupportedMergeConfiguration {
                    setting,
                    path: lossy_path(path.as_bstr()),
                });
            }
        }

        if let gix::filter::plumbing::attributes::StateRef::Value(name) = filter.assignment.state {
            let name = name.as_bstr().to_str_lossy();
            for suffix in ["process", "clean", "smudge"] {
                let setting = format!("filter.{name}.{suffix}");
                if repo.config_snapshot().string(setting.as_str()).is_some() {
                    return Err(SniffError::UnsupportedMergeConfiguration {
                        setting,
                        path: lossy_path(path.as_bstr()),
                    });
                }
            }
        }
    }
    Ok(())
}

fn is_builtin_merge_driver(name: &str) -> bool {
    matches!(name, "text" | "binary" | "union")
}

fn lossy_path(bytes: &BStr) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes.as_ref()).as_ref())
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}
