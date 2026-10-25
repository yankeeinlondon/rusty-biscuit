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
pub(crate) fn open(dir: &Path) -> Option<gix::Repository> {
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

/// The commit `refname` resolves to, as `git rev-parse --verify --quiet
/// <refname>^{commit}` prints it: symbolic references followed and tags
/// peeled. `Some(None)` when the reference does not exist; `None` also when
/// it does not peel to a commit, so the caller's git reports that.
pub(crate) fn commit_of(dir: &Path, refname: &str) -> Option<Option<String>> {
    let repo = open(dir)?;
    match repo.try_find_reference(refname) {
        Ok(Some(mut reference)) => reference.peel_to_commit().ok().map(|commit| Some(commit.id.to_string())),
        Ok(None) => Some(None),
        Err(_) => None,
    }
}

/// The repository's common directory as `git rev-parse --path-format=absolute
/// --git-common-dir` names it. On Unix the spelling matches (symlinks
/// resolved, as Git's realpath does); on Windows the separators are native
/// where Git prints `/`, which every path comparison here tolerates.
pub(crate) fn common_dir(dir: &Path) -> Option<std::path::PathBuf> {
    let repo = open(dir)?;
    let common = repo.common_dir();
    #[cfg(unix)]
    return std::fs::canonicalize(common).ok();
    #[cfg(not(unix))]
    return std::path::absolute(common).ok();
}

/// `branch.<branch>.<key>` as `git config --get` prints it (the last value),
/// or `Some(None)` when it is unset.
pub(crate) fn branch_config(dir: &Path, branch: &str, key: &str) -> Option<Option<String>> {
    let repo = open(dir)?;
    let config = repo.config_snapshot();
    let value = config.plumbing().string_by("branch", Some(branch.as_bytes().as_bstr()), key);
    Some(value.map(|value| value.to_str_lossy().into_owned()))
}

/// Whether `git remote get-url <name>` would fail with "No such remote":
/// no `remote.<name>` key in any configuration file (Git ignores an empty
/// section) and no legacy
/// `remotes/<name>` or `branches/<name>` file. `None` when unknown.
pub(crate) fn remote_absent(dir: &Path, name: &str) -> Option<bool> {
    let repo = open(dir)?;
    let config = repo.config_snapshot();
    let configured = config
        .plumbing()
        .sections_by_name("remote")
        .into_iter()
        .flatten()
        .any(|section| section.header().subsection_name() == Some(name.as_bytes().as_bstr()) && section.num_values() > 0);
    let common = repo.common_dir();
    let legacy = ["remotes", "branches"].iter().any(|kind| std::fs::symlink_metadata(common.join(kind).join(name)).is_ok());
    Some(!configured && !legacy)
}

/// Whether `git check-ref-format --branch <branch>` accepts `branch` and
/// prints it unchanged: a valid `refs/heads/<branch>` (which already refuses
/// the `@{...}` forms git expands) other than `HEAD`.
pub(crate) fn is_valid_branch_name(branch: &str) -> bool {
    !branch.starts_with('-')
        && branch != "HEAD"
        && gix::validate::reference::name(format!("refs/heads/{branch}").as_bytes().as_bstr()).is_ok()
}

/// What `git worktree list --porcelain` prints for the repository containing
/// `dir`, byte for byte, for a repository whose every worktree is plain:
/// the main checkout first, then the linked ones sorted by path as git
/// sorts them, each as `worktree`, `HEAD`, and `branch` (or `detached`) lines and
/// a blank line.
///
/// `None`, so the caller runs git, whenever git might print anything else or
/// spell a path differently: `GIT_DIR` or `GIT_COMMON_DIR` set, a bare main
/// repository or a common directory not named `.git`, a linked worktree that
/// is locked, would be marked `prunable` (its recorded `.git` gone, its
/// `gitdir` file empty or unreadable), or records a relative path, a `HEAD`
/// that is neither a branch under `refs/heads/` nor a full object ID, or a
/// path with characters git might quote.
///
/// The main checkout's path is the common directory's real path less
/// `/.git`, as git computes it; on Windows that real path is what
/// `GetFinalPathNameByHandleW` gives (long names, the stored case), with `/`
/// separators, which is how Git for Windows spells it. A linked worktree's
/// path is its `gitdir` file's content less `/.git`, which Git wrote.
pub(crate) fn worktree_list_porcelain(dir: &Path) -> Option<String> {
    if ["GIT_DIR", "GIT_COMMON_DIR"].iter().any(|name| std::env::var_os(name).is_some()) {
        return None;
    }
    let repo = open(dir)?;
    if repo.object_hash() != gix::hash::Kind::Sha1 || repo.config_snapshot().boolean("core.bare") == Some(true) {
        return None;
    }
    // From a linked worktree gix names it through `commondir` (`../..`).
    let common = std::fs::canonicalize(repo.common_dir()).ok()?;
    if common.file_name() != Some(std::ffi::OsStr::new(".git")) {
        return None;
    }
    let main = plain_path(&real_path_spelling(&common)?)?.strip_suffix("/.git")?.to_string();

    let mut listing = porcelain_entry(&repo, &main, &common)?;
    let admins = match std::fs::read_dir(common.join("worktrees")) {
        Ok(admins) => admins,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(listing),
        Err(_) => return None,
    };
    let mut linked = Vec::new();
    for admin in admins {
        let admin = admin.ok()?;
        if !admin.file_type().ok()?.is_dir() {
            return None;
        }
        let admin = admin.path();
        if std::fs::symlink_metadata(admin.join("locked")).is_ok() {
            return None;
        }
        let gitdir = std::fs::read_to_string(admin.join("gitdir")).ok()?;
        let gitdir = gitdir.trim_end();
        if !Path::new(gitdir).is_absolute() || !Path::new(gitdir).exists() {
            return None;
        }
        let path = plain_path(gitdir.strip_suffix("/.git")?)?;
        linked.push((path.to_string(), porcelain_entry(&repo, path, &admin)?));
    }
    // `git worktree list` sorts the linked worktrees by path with
    // `fspathcmp`: ASCII case folded when `core.ignoreCase` is set.
    let fold = repo.config_snapshot().boolean("core.ignoreCase") == Some(true);
    let key = |path: &str| if fold { path.to_ascii_lowercase() } else { path.to_string() };
    linked.sort_by_key(|(path, _)| key(path));
    if linked.windows(2).any(|pair| key(&pair[0].0) == key(&pair[1].0)) {
        // Git's sort is not stable, so their order is its to choose.
        return None;
    }
    for (_, block) in linked {
        listing.push_str(&block);
    }
    Some(listing)
}

/// One worktree's block of [`worktree_list_porcelain`], its `HEAD` read from
/// `admin`, a branch resolved in `repo`'s shared refs.
fn porcelain_entry(repo: &gix::Repository, path: &str, admin: &Path) -> Option<String> {
    let head = std::fs::read_to_string(admin.join("HEAD")).ok()?;
    let head = head.trim_end_matches(['\n', '\r']);
    let (oid, branch) = match head.strip_prefix("ref: ") {
        Some(target) if target.starts_with("refs/heads/") => match repo.try_find_reference(target) {
            Ok(Some(mut reference)) => (reference.follow_to_object().ok()?.to_string(), Some(plain_path(target)?)),
            Ok(None) => ("0".repeat(40), Some(plain_path(target)?)),
            Err(_) => return None,
        },
        None if head.len() == 40 && head.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')) => (head.to_string(), None),
        _ => return None,
    };
    let tail = match branch {
        Some(branch) => format!("branch {branch}\n"),
        None => "detached\n".to_string(),
    };
    Some(format!("worktree {path}\nHEAD {oid}\n{tail}\n"))
}

/// Whether `git cat-file -e <object>^{commit}` would succeed, when that is
/// certain here: `true` only for an object this repository holds that
/// peels to a commit. Anything else is `false`, and the caller asks git,
/// which may know more (a promisor remote's lazy fetch, for one).
pub(crate) fn surely_a_commit(dir: &Path, object: &str) -> bool {
    open(dir).is_some_and(|repo| {
        repo.rev_parse_single(format!("{object}^{{commit}}").as_str())
            .ok()
            .and_then(|id| id.object().ok())
            .is_some_and(|object| object.kind == gix::object::Kind::Commit)
    })
}

/// The checkout `dir` is in, as `git rev-parse --path-format=absolute
/// --show-toplevel --git-dir --git-common-dir` names it, each path
/// canonicalized: its top level, its git directory, and the common
/// directory. `Some(None)` where git would fail or a path does not resolve
/// (a bare repository has no top level); `None` when `gix` cannot open it.
pub(crate) fn checkout_layout(dir: &Path) -> Option<Option<[std::path::PathBuf; 3]>> {
    let repo = open(dir)?;
    let Some(top) = repo.workdir() else {
        return Some(None);
    };
    let paths = [top, repo.git_dir(), repo.common_dir()].map(std::fs::canonicalize);
    Some(match paths {
        [Ok(top), Ok(git_dir), Ok(common)] => Some([top, git_dir, common]),
        _ => None,
    })
}

/// An index file's signature, version, and entry count.
const INDEX_HEADER_BYTES: u64 = 12;

/// What `git ls-files --stage -z` prints for the index file `index`, byte
/// for byte: each entry as `<mode> <object> <stage>\t<path>` and a NUL, in
/// index order. `None`, so the caller runs git, for a missing or unreadable
/// index, one that is not SHA-1, a split index (its entries live in a
/// second file, so any `sharedindex.*` beside it counts), or a sparse one (git expands its directory entries first).
pub(crate) fn staged_entries(index: &Path) -> Option<Vec<u8>> {
    // A split index keeps its shared part beside it; gix merges the two
    // itself, but only git's own reading of that pair is authoritative.
    let beside = std::fs::read_dir(index.parent()?).ok()?;
    for entry in beside {
        if entry.ok()?.file_name().to_string_lossy().starts_with("sharedindex.") {
            return None;
        }
    }
    // gix-index 0.52 panics on a file shorter than its trailing checksum
    // rather than failing; git reports such an index as corrupt.
    if std::fs::metadata(index).ok()?.len() < INDEX_HEADER_BYTES + gix::hash::Kind::Sha1.len_in_bytes() as u64 {
        return None;
    }
    let options = gix::index::decode::Options { thread_limit: Some(1), ..Default::default() };
    let file = gix::index::File::at(index, gix::hash::Kind::Sha1, false, options).ok()?;
    if file.link().is_some() || file.is_sparse() {
        return None;
    }
    let mut listing = Vec::new();
    for entry in file.entries() {
        listing.extend_from_slice(format!("{:06o} {} {}\t", entry.mode.bits(), entry.id, entry.stage_raw()).as_bytes());
        listing.extend_from_slice(entry.path(&file));
        listing.push(0);
    }
    Some(listing)
}

/// The index file git reads for the checkout `dir` is in: `None` when
/// `GIT_INDEX_FILE` names another or `gix` cannot open the repository.
pub(crate) fn index_path(dir: &Path) -> Option<std::path::PathBuf> {
    if std::env::var_os("GIT_INDEX_FILE").is_some() {
        return None;
    }
    open(dir).map(|repo| repo.index_path())
}

/// `path` unchanged when it has no character git's porcelain output might
/// quote or escape (control characters, `"`, `\`, anything outside ASCII).
fn plain_path(path: &str) -> Option<&str> {
    path.bytes().all(|byte| (0x20..0x7f).contains(&byte) && byte != b'"' && byte != b'\\').then_some(path)
}

/// `path`'s real path spelled as git prints one: as is on Unix; on Windows
/// without the `\\?\` prefix and with `/` separators.
fn real_path_spelling(path: &Path) -> Option<String> {
    let real = std::fs::canonicalize(path).ok()?;
    let real = real.to_str()?;
    if cfg!(windows) {
        if real.starts_with(r"\\?\UNC\") {
            return None;
        }
        Some(real.strip_prefix(r"\\?\").unwrap_or(real).replace('\\', "/"))
    } else {
        Some(real.to_string())
    }
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
    fn commit_of_matches_git_rev_parse_verify_commit() {
        let dir = repo();
        let path = dir.path();
        git(path, &["tag", "-a", "-m", "annotated", "v1"]);
        git(path, &["update-ref", "refs/heads/tagged", "refs/tags/v1"]);
        git(path, &["symbolic-ref", "refs/heads/alias", "refs/heads/main"]);
        for refname in ["refs/heads/main", "refs/tags/v1", "refs/heads/tagged", "refs/heads/alias", "refs/heads/absent"] {
            let expected = git(path, &["rev-parse", "--verify", "--quiet", &format!("{refname}^{{commit}}")]);
            assert_eq!(commit_of(path, refname), Some(expected), "{refname}");
        }
        git(path, &["pack-refs", "--all"]);
        let expected = git(path, &["rev-parse", "--verify", "--quiet", "refs/heads/main^{commit}"]);
        assert_eq!(commit_of(path, "refs/heads/main"), Some(expected), "a packed ref");
    }

    #[test]
    fn common_dir_matches_git_rev_parse_git_common_dir() {
        let dir = repo();
        let path = dir.path();
        let linked = tempfile::tempdir().unwrap();
        let checkout = linked.path().join("linked");
        git(path, &["worktree", "add", "-q", "-b", "topic", checkout.to_str().unwrap()]).unwrap();
        for from in [path, checkout.as_path()] {
            let expected = git(from, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).map(std::path::PathBuf::from);
            let read = common_dir(from);
            #[cfg(unix)]
            assert_eq!(read, expected, "{from:?}");
            // Git prints `/` separators on Windows; the directory is the same.
            #[cfg(not(unix))]
            assert!(
                read.as_deref().zip(expected.as_deref()).is_some_and(|(read, expected)| crate::remove::admin_entry::same_location(read, expected)),
                "{from:?}: {read:?} vs {expected:?}"
            );
        }
    }

    #[test]
    fn branch_config_matches_git_config_get() {
        let dir = repo();
        let path = dir.path();
        for branch in ["main", "feat/x", "v1.2"] {
            let get = |key: &str| git(path, &["config", "--get", &format!("branch.{branch}.{key}")]);
            assert_eq!(branch_config(path, branch, "remote"), Some(get("remote")), "{branch} unset");
            git(path, &["config", &format!("branch.{branch}.remote"), "origin"]);
            git(path, &["config", "--add", &format!("branch.{branch}.merge"), "refs/heads/a"]);
            git(path, &["config", "--add", &format!("branch.{branch}.merge"), "refs/heads/b"]);
            assert_eq!(branch_config(path, branch, "remote"), Some(get("remote")), "{branch} set");
            assert_eq!(branch_config(path, branch, "merge"), Some(get("merge")), "{branch}: the last of several values");
        }
    }

    #[test]
    fn remote_absent_matches_git_remote_get_url_failing() {
        let dir = repo();
        let path = dir.path();
        let fails = |path: &Path| git(path, &["remote", "get-url", "origin"]).is_none();
        assert_eq!(remote_absent(path, "origin"), Some(fails(path)), "no remote");
        git(path, &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"]);
        assert_eq!(remote_absent(path, "origin"), Some(fails(path)), "a remote without a URL");
        git(path, &["config", "--unset", "remote.origin.fetch"]);
        assert_eq!(remote_absent(path, "origin"), Some(fails(path)), "an empty remote section");
        let common = common_dir(path).unwrap();
        std::fs::create_dir_all(common.join("remotes")).unwrap();
        std::fs::write(common.join("remotes").join("origin"), "URL: https://example.invalid/r.git\n").unwrap();
        assert_eq!(remote_absent(path, "origin"), Some(fails(path)), "a legacy remotes file");
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

    /// `git worktree list --porcelain` exactly as git prints it, from `dir`.
    fn git_listing(dir: &Path) -> String {
        crate::git::git_from_raw(dir, dir, &["worktree", "list", "--porcelain"]).unwrap()
    }

    fn assert_listing_matches_git(dir: &Path, case: &str) {
        assert_eq!(worktree_list_porcelain(dir), Some(git_listing(dir)), "{case}: {dir:?}");
        assert_eq!(crate::git::worktree_list_from(dir, dir).unwrap(), git_listing(dir).trim(), "{case}: the wrapper");
    }

    /// Git answers, and the wrapper passes its answer on unchanged.
    fn assert_listing_left_to_git(dir: &Path, case: &str) {
        assert_eq!(worktree_list_porcelain(dir), None, "{case}");
        assert_eq!(crate::git::worktree_list_from(dir, dir).unwrap(), git_listing(dir).trim(), "{case}: the wrapper");
    }

    #[test]
    fn the_worktree_listing_matches_git_worktree_list() {
        let dir = repo();
        let path = dir.path();
        std::fs::create_dir(path.join("sub")).unwrap();
        assert_listing_matches_git(path, "the main checkout alone");
        assert_listing_matches_git(&path.join("sub"), "from a subdirectory");

        let linked = tempfile::tempdir().unwrap();
        let add = |name: &str, extra: &[&str]| {
            let checkout = linked.path().join(name);
            let mut args = vec!["worktree", "add", "-q"];
            args.extend_from_slice(extra);
            args.push(checkout.to_str().unwrap());
            git(path, &args).unwrap();
            checkout
        };
        // Added out of name order: git lists them in its admin directory's
        // order, whatever that is on this filesystem.
        let zeta = add("zeta", &["-b", "zeta"]);
        add("alpha", &["-b", "feat/alpha"]);
        // Sorted with ASCII case folded where `core.ignoreCase` is set.
        add("Beta", &["-b", "beta"]);
        let detached = add("with space", &["--detach"]);
        std::fs::create_dir(zeta.join("deep")).unwrap();
        for from in [path, zeta.as_path(), &zeta.join("deep"), detached.as_path()] {
            assert_listing_matches_git(from, "linked worktrees");
        }
        git(path, &["pack-refs", "--all"]);
        assert_listing_matches_git(&zeta, "packed branches");
        git(&zeta, &["commit", "-q", "--allow-empty", "-m", "zeta moves"]);
        assert_listing_matches_git(path, "a linked worktree's branch moved");

        #[cfg(unix)]
        {
            let link = tempfile::tempdir().unwrap();
            let through = link.path().join("through");
            std::os::unix::fs::symlink(path, &through).unwrap();
            assert_listing_matches_git(&through, "reached through a symlink");
        }
    }

    #[test]
    fn an_unborn_main_branch_lists_the_null_object_id_as_git_does() {
        let dir = tempfile::tempdir().unwrap();
        git_from(dir.path(), dir.path(), &["init", "-q", "-b", "trunk"]).unwrap();
        assert_listing_matches_git(dir.path(), "no commit yet");
        assert!(git_listing(dir.path()).contains(&format!("HEAD {}", "0".repeat(40))));
    }

    #[test]
    fn a_locked_prunable_or_bare_listing_is_left_to_git() {
        let dir = repo();
        let path = dir.path();
        let linked = tempfile::tempdir().unwrap();
        let checkout = linked.path().join("locked");
        git(path, &["worktree", "add", "-q", "-b", "locked", checkout.to_str().unwrap()]).unwrap();
        git(path, &["worktree", "lock", "--reason", "on a stick", checkout.to_str().unwrap()]).unwrap();
        assert_listing_left_to_git(path, "locked");
        git(path, &["worktree", "unlock", checkout.to_str().unwrap()]).unwrap();
        assert_listing_matches_git(path, "unlocked again");

        std::fs::remove_dir_all(&checkout).unwrap();
        assert!(git_listing(path).contains("prunable"));
        assert_listing_left_to_git(path, "prunable");

        let bare = tempfile::tempdir().unwrap();
        git_from(path, path, &["clone", "-q", "--bare", ".", bare.path().to_str().unwrap()]).unwrap();
        assert_listing_left_to_git(bare.path(), "bare");
    }

    #[test]
    fn the_checkout_layout_matches_git_rev_parse() {
        let dir = repo();
        let path = dir.path();
        std::fs::create_dir(path.join("sub")).unwrap();
        let linked = tempfile::tempdir().unwrap();
        let checkout = linked.path().join("linked");
        git(path, &["worktree", "add", "-q", "-b", "topic", checkout.to_str().unwrap()]).unwrap();
        let bare = tempfile::tempdir().unwrap();
        git(path, &["clone", "-q", "--bare", ".", bare.path().to_str().unwrap()]);
        for from in [path, &path.join("sub"), checkout.as_path(), bare.path()] {
            let expected = git(from, &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir", "--git-common-dir"])
                .and_then(|out| {
                    let paths: Vec<_> = out.lines().map(|line| std::fs::canonicalize(line.trim()).ok()).collect();
                    match paths.as_slice() {
                        [Some(top), Some(git_dir), Some(common)] => Some([top.clone(), git_dir.clone(), common.clone()]),
                        _ => None,
                    }
                });
            assert_eq!(checkout_layout(from), Some(expected), "{from:?}");
        }
    }

    /// gix-index 0.52 panics on an index shorter than its checksum; such a
    /// file goes to git, whose own error is the answer.
    #[test]
    fn a_truncated_index_is_left_to_git_rather_than_panicking() {
        let dir = repo();
        let path = dir.path();
        let index = index_path(path).unwrap();
        for length in [0, 4, 31] {
            std::fs::write(&index, vec![b'D'; length]).unwrap();

            assert_eq!(staged_entries(&index), None, "{length} bytes");
            let listed = crate::git::staged_entries_of(path, &index);
            assert!(matches!(&listed, Err(error) if error.to_string().contains("index")), "{length} bytes: {listed:?}");
        }
    }

    #[test]
    fn the_staged_entries_match_git_ls_files_stage() {
        let dir = repo();
        let path = dir.path();
        let staged = |path: &Path| -> Vec<u8> {
            crate::git::git_from_bytes(path, path, &["ls-files", "--stage", "-z"], None).unwrap()
        };
        let assert_matches = |path: &Path, case: &str| {
            let index = index_path(path).unwrap();
            assert_eq!(staged_entries(&index).map(String::from_utf8), Some(Ok(String::from_utf8(staged(path)).unwrap())), "{case}");
            assert_eq!(crate::git::staged_entries_from(path, path).unwrap(), staged(path), "{case}: the wrapper");
        };
        for (file, contents) in [("a.txt", "a"), ("dir/b.txt", "b"), ("ü nicode.txt", "u"), ("run.sh", "#!/bin/sh")] {
            std::fs::create_dir_all(path.join(file).parent().unwrap()).unwrap();
            std::fs::write(path.join(file), contents).unwrap();
        }
        git(path, &["add", "."]);
        git(path, &["update-index", "--chmod=+x", "run.sh"]);
        git(path, &["commit", "-q", "-m", "files"]);
        let blob = git(path, &["rev-parse", "HEAD:a.txt"]).unwrap();
        let commit = git(path, &["rev-parse", "HEAD"]).unwrap();
        git(path, &["update-index", "--add", "--cacheinfo", &format!("120000,{blob},link")]);
        git(path, &["update-index", "--add", "--cacheinfo", &format!("160000,{commit},sub")]);
        std::fs::write(path.join("new.txt"), "n").unwrap();
        git(path, &["add", "-N", "new.txt"]);
        git(path, &["update-index", "--skip-worktree", "dir/b.txt"]);
        assert_matches(path, "modes, a gitlink, intent to add, skip-worktree, a non-ASCII path");

        git(path, &["update-index", "--index-version", "4"]);
        assert_matches(path, "index version 4");

        let linked = tempfile::tempdir().unwrap();
        let checkout = linked.path().join("linked");
        git(path, &["worktree", "add", "-q", "-b", "topic", checkout.to_str().unwrap()]).unwrap();
        assert_matches(&checkout, "a linked worktree's own index");

        // A conflict: stages 1 to 3 for one path.
        git(path, &["update-index", "--index-version", "2"]);
        git(path, &["reset", "-q", "--hard"]);
        git(path, &["checkout", "-q", "-b", "side"]);
        std::fs::write(path.join("a.txt"), "side").unwrap();
        git(path, &["commit", "-q", "-am", "side"]);
        git(path, &["checkout", "-q", "main"]);
        std::fs::write(path.join("a.txt"), "main").unwrap();
        git(path, &["commit", "-q", "-am", "main"]);
        assert!(git(path, &["merge", "-q", "side"]).is_none(), "the merge conflicts");
        assert_matches(path, "a conflict");
    }

    #[test]
    fn a_split_index_is_left_to_git() {
        let dir = repo();
        let path = dir.path();
        std::fs::write(path.join("a.txt"), "a").unwrap();
        git(path, &["add", "a.txt"]);
        git(path, &["update-index", "--split-index"]);
        assert_eq!(staged_entries(&index_path(path).unwrap()), None);
        let expected = crate::git::git_from_bytes(path, path, &["ls-files", "--stage", "-z"], None).unwrap();
        assert_eq!(crate::git::staged_entries_from(path, path).unwrap(), expected);
    }

    #[test]
    fn a_surely_committed_object_is_one_git_cat_file_finds() {
        let dir = repo();
        let path = dir.path();
        git(path, &["-c", "tag.gpgSign=false", "tag", "-a", "-m", "annotated", "v1"]).unwrap();
        let head = git(path, &["rev-parse", "HEAD"]).unwrap();
        let tag = git(path, &["rev-parse", "v1"]).unwrap();
        let tree = git(path, &["rev-parse", "HEAD^{tree}"]).unwrap();
        let absent = "0123456789abcdef0123456789abcdef01234567".to_string();
        for object in [head, tag, tree, absent] {
            let git_finds = git(path, &["cat-file", "-e", &format!("{object}^{{commit}}")]).is_some();
            if surely_a_commit(path, &object) {
                assert!(git_finds, "{object}: in-process yes, git no");
            }
            assert_eq!(surely_a_commit(path, &object), git_finds, "{object}: no promisor here, so both agree");
        }
    }
}
