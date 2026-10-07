//! Local repository metadata read in-process with `gix`, so reads that `wt
//! list` and its refresh worker repeat start no git process. A git process
//! costs about 5 ms on macOS and about 47 ms on Windows; a refresh attempt
//! used to start a dozen for these reads alone.
//!
//! Every read opens the repository afresh, so a configuration change between
//! two reads is seen. A read returns `None` when `gix` cannot open or read the
//! repository (an untrusted owner, a reftable repository), and the caller
//! then runs the git command it replaces. Each read matches that command's
//! answer; the parity tests below compare them.

use std::path::Path;

use gix::bstr::{BStr, ByteSlice};

/// The repository containing `dir`, discovered as git does: upwards from
/// `dir`, honoring `GIT_DIR`, and refused when its owner is not trusted.
fn open(dir: &Path) -> Option<gix::Repository> {
    let options = gix::open::Options::default().bail_if_untrusted(true);
    let trust = gix::sec::trust::Mapping { full: options.clone(), reduced: options };
    gix::ThreadSafeRepository::discover_with_environment_overrides_opts(dir, Default::default(), trust)
        .ok()
        .map(|repo| repo.to_thread_local())
}

/// What `git remote get-url origin` prints: the first `remote.origin.url`,
/// rewritten by the longest matching `url.<base>.insteadOf`. `Some(None)`
/// when the repository has no origin URL.
pub(crate) fn origin_url(dir: &Path) -> Option<Option<String>> {
    let repo = open(dir)?;
    let config = repo.config_snapshot();
    let file = config.plumbing();
    let Some(url) = file.strings("remote.origin.url").and_then(|urls| urls.into_iter().next()) else {
        return Some(None);
    };
    Some(Some(rewrite_instead_of(file, url.as_ref()).to_str_lossy().into_owned()))
}

/// `url` with the longest `url.<base>.insteadOf` prefix replaced by its
/// `<base>`, as git rewrites a fetch URL.
fn rewrite_instead_of(file: &gix::config::File<'_>, url: &BStr) -> Vec<u8> {
    let mut best: Option<(usize, Vec<u8>)> = None;
    for section in file.sections_by_name("url").into_iter().flatten() {
        let Some(base) = section.header().subsection_name() else {
            continue;
        };
        for prefix in section.values("insteadOf") {
            if url.starts_with(prefix.as_ref()) && best.as_ref().is_none_or(|(length, _)| prefix.len() > *length) {
                best = Some((prefix.len(), base.to_vec()));
            }
        }
    }
    match best {
        Some((length, mut base)) => {
            base.extend_from_slice(&url[length..]);
            base
        }
        None => url.to_vec(),
    }
}

/// The branch `refs/remotes/origin/HEAD` names, else a `main` or `master`
/// that resolves as a revision, else `Some(None)`.
pub(crate) fn default_branch(dir: &Path) -> Option<Option<String>> {
    let repo = open(dir)?;
    if let Ok(Some(reference)) = repo.try_find_reference("refs/remotes/origin/HEAD")
        && let gix::refs::TargetRef::Symbolic(target) = reference.target()
        && let Some(branch) = target.as_bstr().strip_prefix(b"refs/remotes/origin/")
    {
        return Some(Some(branch.to_str_lossy().into_owned()));
    }
    Some(["main", "master"].into_iter().find(|candidate| repo.rev_parse_single(*candidate).is_ok()).map(str::to_string))
}

/// The object ID `refname` points at, following symbolic references but
/// never peeling, as `git rev-parse --verify --quiet <refname>` prints it.
/// `Some(None)` when the reference does not exist.
pub(crate) fn reference_target(dir: &Path, refname: &str) -> Option<Option<String>> {
    let repo = open(dir)?;
    match repo.try_find_reference(refname) {
        Ok(Some(mut reference)) => Some(reference.follow_to_object().ok().map(|id| id.to_string())),
        Ok(None) => Some(None),
        Err(_) => None,
    }
}

/// `core.sshCommand` as `git config --get` prints it (the last value), or
/// `Some(None)` when it is unset.
pub(crate) fn ssh_command(dir: &Path) -> Option<Option<String>> {
    let repo = open(dir)?;
    Some(repo.config_snapshot().string("core.sshCommand").map(|value| value.to_str_lossy().into_owned()))
}

/// Whether `git check-ref-format --branch <branch>` accepts `branch` and
/// prints it unchanged: a valid `refs/heads/<branch>` (which already refuses
/// the `@{...}` forms git expands) other than `HEAD`.
pub(crate) fn is_valid_branch_name(branch: &str) -> bool {
    !branch.starts_with('-')
        && branch != "HEAD"
        && gix::validate::reference::name(format!("refs/heads/{branch}").as_bytes().as_bstr()).is_ok()
}

/// Each read against the git command it replaces, on the same repository.
/// Both sides read the same user and system configuration, so the comparison
/// holds whatever the host has configured.
#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::git::git_from;
    use crate::test_support::configure;

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        git_from(dir.path(), dir.path(), &["init", "-q", "-b", "main"]).unwrap();
        configure(dir.path(), &[("user.email", "test@example.com"), ("user.name", "Test User"), ("commit.gpgsign", "false")]);
        git_from(dir.path(), dir.path(), &["commit", "-q", "--allow-empty", "-m", "first"]).unwrap();
        dir
    }

    fn git(dir: &Path, args: &[&str]) -> Option<String> {
        git_from(dir, dir, args).ok()
    }

    fn assert_origin_url_matches_git(dir: &Path, case: &str) {
        let expected = git(dir, &["remote", "get-url", "origin"]);
        assert_eq!(origin_url(dir), Some(expected), "{case}");
    }

    #[test]
    fn origin_url_matches_git_remote_get_url() {
        let dir = repo();
        let path = dir.path();
        assert_origin_url_matches_git(path, "no origin");

        let bare = tempfile::tempdir().unwrap();
        let local = bare.path().to_string_lossy().into_owned();
        for url in [local.as_str(), "git@github.com:owner/repo.git", "https://github.com/owner/repo.git", "ssh://git@host:2222/o/r"] {
            git(path, &["config", "remote.origin.url", url]).or(Some(String::new()));
            assert_origin_url_matches_git(path, url);
        }

        git(path, &["config", "remote.origin.url", "gh:owner/repo"]);
        git(path, &["config", "url.https://github.com/.insteadOf", "gh:"]);
        assert_origin_url_matches_git(path, "insteadOf");
        git(path, &["config", "url.ssh://git@github.com/owner/.insteadOf", "gh:owner/"]);
        assert_origin_url_matches_git(path, "the longest insteadOf prefix wins");
        git(path, &["config", "--add", "url.https://elsewhere.example/.insteadOf", "gh:"]);
        assert_origin_url_matches_git(path, "several insteadOf values");

        git(path, &["config", "--add", "remote.origin.url", "https://second.example/r.git"]);
        assert_origin_url_matches_git(path, "the first of several URLs");
    }

    #[test]
    fn default_branch_matches_git() {
        let dir = repo();
        let path = dir.path();
        assert_eq!(default_branch(path), Some(Some("main".to_string())), "a local main");

        git(path, &["update-ref", "refs/remotes/origin/trunk", "HEAD"]);
        git(path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
        let expected = git(path, &["symbolic-ref", "refs/remotes/origin/HEAD"]);
        assert_eq!(expected.as_deref(), Some("refs/remotes/origin/trunk"));
        assert_eq!(default_branch(path), Some(Some("trunk".to_string())), "origin/HEAD");

        let other = repo();
        git(other.path(), &["branch", "-m", "main", "master"]);
        assert_eq!(default_branch(other.path()), Some(Some("master".to_string())), "a local master");
        git(other.path(), &["branch", "-m", "master", "develop"]);
        assert_eq!(default_branch(other.path()), Some(None), "neither");
    }

    #[test]
    fn reference_target_matches_git_rev_parse_verify() {
        let dir = repo();
        let path = dir.path();
        git(path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        git(path, &["tag", "-a", "-m", "annotated", "v1"]);
        git(path, &["update-ref", "refs/remotes/origin/tagged", "refs/tags/v1"]);
        git(path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
        for refname in ["refs/remotes/origin/main", "refs/remotes/origin/tagged", "refs/remotes/origin/HEAD", "refs/remotes/origin/absent"] {
            let expected = git(path, &["rev-parse", "--verify", "--quiet", refname]);
            assert_eq!(reference_target(path, refname), Some(expected), "{refname}");
        }
        git(path, &["pack-refs", "--all"]);
        let expected = git(path, &["rev-parse", "--verify", "--quiet", "refs/remotes/origin/main"]);
        assert_eq!(reference_target(path, "refs/remotes/origin/main"), Some(expected), "a packed ref");
    }

    #[test]
    fn ssh_command_matches_git_config_get() {
        let dir = repo();
        let path = dir.path();
        let get = |path: &Path| git(path, &["config", "--get", "core.sshCommand"]);
        assert_eq!(ssh_command(path), Some(get(path)), "unset");
        git(path, &["config", "core.sshCommand", "ssh -i 'key file'"]);
        assert_eq!(ssh_command(path), Some(get(path)), "set");
        git(path, &["config", "--add", "core.sshCommand", "ssh -v"]);
        assert_eq!(ssh_command(path), Some(get(path)), "the last of several values");
    }

    #[test]
    fn branch_validity_matches_git_check_ref_format() {
        let dir = repo();
        let path = dir.path();
        for branch in [
            "main", "feat/x", "ü", "a-b_c.d", "v1.2", "x@y", "ma..in", "-x", "@{-1}", "@", "HEAD", "a b", "main.lock", "",
            "feat/x:refs/heads/y", "a//b", "/a", "a/", "a.", ".a", "a/.b", "a~1", "a^", "a?", "a*", "a[", "a\\b", "a@{b",
        ] {
            let git_accepts =
                !branch.starts_with('-') && git(path, &["check-ref-format", "--branch", branch]).is_some_and(|out| out == branch);
            assert_eq!(is_valid_branch_name(branch), git_accepts, "{branch:?}");
        }
    }

    #[test]
    fn a_linked_worktree_reads_the_main_repositorys_metadata() {
        let dir = repo();
        let path = dir.path();
        git(path, &["config", "remote.origin.url", "https://github.com/owner/repo.git"]);
        let linked = tempfile::tempdir().unwrap();
        let checkout = linked.path().join("linked");
        git(path, &["worktree", "add", "-q", "-b", "topic", checkout.to_str().unwrap()]).unwrap();
        assert_eq!(origin_url(&checkout), Some(git(&checkout, &["remote", "get-url", "origin"])));
        assert_eq!(default_branch(&checkout), Some(Some("main".to_string())));
    }
}
