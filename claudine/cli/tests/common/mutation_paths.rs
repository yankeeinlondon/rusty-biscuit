//! Path expectations for lifecycle mutation tests.
//!
//! A lifecycle write has two independent inputs: the **boundary** (the
//! caller's root: the working tree containing the launch directory) and the
//! **authoring base** (the directory of the document whose action wrote).
//! These helpers state expectations in those terms, "written beside this
//! document", "nothing under that directory", so a test never re-derives the
//! file-reference resolution it is checking.
//!
//! A marker name used with [`assert_only_beside`] must not exist anywhere in
//! the tree before the run: an existing file would be found by lookup rather
//! than created beside the document.

use std::fs;
use std::path::{Path, PathBuf};

use super::CliProcessFixture;

/// A caller repository: `cwd` is a git working tree's top level, so it is the
/// mutation root of every run launched from it.
///
/// `outside` is an empty directory beside the caller's tree (under the
/// fixture home), the place a refused absolute target names. `library` is a
/// separate git repository under the fixture home, the shape of a shared
/// prompt library.
pub struct CallerTree {
    pub fixture: CliProcessFixture,
    pub outside: PathBuf,
    pub library: PathBuf,
}

/// The `.git/config` of a healthy non-bare repository.
pub const HEALTHY_GIT_CONFIG: &str = "[core]\n\trepositoryformatversion = 0\n\tbare = false\n";

/// A minimal non-bare repository written directly at `root`, with `config` as
/// its `.git/config`: discovery needs no more, and no `git` is spawned (two
/// `git init` runs were most of a fixture's own cost).
pub fn write_git_dir(root: &Path, config: &str) {
    let git = root.join(".git");
    fs::create_dir_all(git.join("objects")).unwrap();
    fs::create_dir_all(git.join("refs/heads")).unwrap();
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(git.join("config"), config).unwrap();
}

impl CallerTree {
    pub fn stage(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        write_git_dir(fixture.cwd(), HEALTHY_GIT_CONFIG);
        let outside = fixture.home().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let library = fixture.home().join("library");
        write_git_dir(&library, HEALTHY_GIT_CONFIG);
        assert!(!outside.starts_with(fixture.cwd()) && !library.starts_with(fixture.cwd()));
        Self { fixture, outside, library }
    }

    /// The mutation root every run of this fixture must use.
    pub fn root(&self) -> &Path {
        self.fixture.cwd()
    }

    /// `relative` under the caller's root.
    pub fn in_root(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    /// Write `body` at `path` (creating its parents) and return `path`.
    pub fn write(&self, path: PathBuf, body: &str) -> PathBuf {
        super::write(&path, body);
        path
    }

    /// An absolute target in [`Self::outside`], as a single-quoted YAML
    /// scalar so Windows separators are not read as escapes.
    pub fn outside_target(&self, name: &str) -> String {
        yaml_path(&self.outside.join(name))
    }

    /// Assert no lifecycle action wrote into [`Self::outside`].
    pub fn assert_outside_untouched(&self) {
        let written = files_under(&self.outside);
        assert!(written.is_empty(), "the boundary was widened; written outside the caller root: {written:?}");
    }
}

/// `path` as a single-quoted YAML scalar.
pub fn yaml_path(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

/// The file named `name` beside `document`.
pub fn beside(document: &Path, name: &str) -> PathBuf {
    document.parent().expect("a document has a directory").join(name)
}

/// Assert `name` beside `document` holds exactly `lines`.
pub fn assert_written_beside(document: &Path, name: &str, lines: &[&str]) {
    let path = beside(document, name);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("expected {} beside {}: {error}", name, document.display()));
    assert_eq!(text.lines().collect::<Vec<_>>(), lines, "contents of {}", path.display());
}

/// Assert the only files named `name` under `root` are the ones beside
/// `documents`: a relative write landed beside its own authoring document and
/// nowhere else (not at the root, not beside another document).
pub fn assert_only_beside(root: &Path, documents: &[&Path], name: &str) {
    let mut found: Vec<PathBuf> = files_under(root)
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|file| file == name))
        .collect();
    let mut expected: Vec<PathBuf> = documents.iter().map(|document| beside(document, name)).collect();
    found.sort();
    expected.sort();
    expected.dedup();
    assert_eq!(found, expected, "every `{name}` under {}", root.display());
}

/// Assert no file named `name` exists anywhere under `dir`.
pub fn assert_nothing_under(dir: &Path, name: &str) {
    let found: Vec<PathBuf> = files_under(dir)
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|file| file == name))
        .collect();
    assert!(found.is_empty(), "no `{name}` may exist under {}: {found:?}", dir.display());
}

/// Assert `output` reports a write refused at the mutation root. Status
/// blocks wrap and carry a gutter, so the check is on the text with both
/// removed.
pub fn assert_refused_outside_root(output: &str) {
    assert!(collapsed(output).contains("outsidethemutationroot"), "expected a mutation-root refusal; output was:\n{output}");
}

/// Assert `output` reports no mutation-root refusal.
pub fn assert_no_refusal(output: &str) {
    assert!(!collapsed(output).contains("outsidethemutationroot"), "no write may be refused; output was:\n{output}");
}

fn collapsed(output: &str) -> String {
    output.replace(['┃', '│'], " ").split_whitespace().collect()
}

/// Every regular file under `dir`, skipping `.git`.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = fs::read_dir(&next) else { continue };
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                if entry.file_name() != ".git" {
                    pending.push(path);
                }
            } else {
                files.push(path);
            }
        }
    }
    files
}
