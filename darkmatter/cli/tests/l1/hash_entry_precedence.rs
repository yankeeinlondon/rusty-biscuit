//! `md hash` chooses single-document or directory mode from the first
//! planned candidate that exists, with file resolution's precedence: an
//! earlier file outranks a later directory, and an earlier candidate that
//! cannot be probed fails with the `io` class instead of being skipped.

use std::path::{Path, PathBuf};
use std::process::Output;

use biscuit_terminal::utils::escape_codes::strip_escape_codes;

use crate::common::CliProcessFixture;

/// A repository `source/` holding a recognized package `area/pkg` and, for
/// each form, an earlier `collision.md` file and a later `collision.md/`
/// directory:
///
/// - bare, from `docs/`: `docs/collision.md` before the root's directory;
/// - `@`, from `docs/`: `docs/first/collision.md` (first `--magic-root`)
///   before `docs/later/collision.md/` (second root);
/// - `^`, from `area/pkg/`: the package's file before the root's directory.
struct Collision {
    cli: CliProcessFixture,
}

impl Collision {
    fn new(name: &str) -> Self {
        let cli = CliProcessFixture::named(name);
        let fixture = Self { cli };
        let source = fixture.source();
        for dir in [".git/objects", ".git/refs/heads", "area/pkg/src"] {
            std::fs::create_dir_all(source.join(dir)).unwrap();
        }
        for (path, content) in [
            (".git/HEAD", "ref: refs/heads/main\n"),
            (".git/config", "[core]\n\trepositoryformatversion = 0\n\tbare = false\n"),
            ("Cargo.toml", "[workspace]\nmembers = [\"area/pkg\"]\n"),
            ("area/pkg/Cargo.toml", "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
            ("area/pkg/src/lib.rs", ""),
            ("docs/collision.md", "---\ntitle: Near\n---\n# Near document\n"),
            ("collision.md/far.md", "---\ntitle: Far\n---\n# Far directory\n"),
            ("docs/first/collision.md", "---\ntitle: First root\n---\n# First root\n"),
            ("docs/later/collision.md/far.md", "---\ntitle: Later root\n---\n# Later root\n"),
            ("area/pkg/collision.md", "---\ntitle: Package\n---\n# Package document\n"),
            ("loop.md/far.md", "---\ntitle: Behind a failed probe\n---\n# Far\n"),
        ] {
            let path = source.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        fixture
    }

    fn source(&self) -> PathBuf {
        self.cli.workspace_path().join("source")
    }

    /// `md <args>` launched from `source/<launch>`.
    fn md(&self, launch: &str, args: &[&str]) -> Output {
        self.cli
            .command_builder()
            .ambient_context(&self.source().join(launch))
            .build()
            .args(args)
            .output()
            .expect("run md")
    }

    /// The single-line output of a successful `md hash <args>`.
    fn hash(&self, launch: &str, args: &[&str]) -> String {
        let mut full = vec!["hash"];
        full.extend_from_slice(args);
        let output = self.md(launch, &full);
        assert!(output.status.success(), "md {full:?} from {launch}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    /// `md hash <absolute path>`: the hash of exactly that file or directory.
    fn hash_of(&self, relative: &str) -> String {
        let absolute = self.source().join(relative);
        self.hash("docs", &[&absolute.to_string_lossy()])
    }
}

fn is_directory_hash(hash: &str) -> bool {
    hash.split_once('-').is_some()
}

#[test]
fn an_earlier_file_outranks_a_later_directory_for_every_root_form() {
    let fixture = Collision::new("hash_entry_precedence_forms");
    let near = fixture.hash_of("docs/collision.md");
    let far_directory = fixture.hash_of("collision.md");
    assert_ne!(near, far_directory, "the fixture must tell the two targets apart");

    // Controls: explicit-relative and absolute spellings name the earlier file.
    assert_eq!(fixture.hash("docs", &["./collision.md"]), near);
    // Bare: the document directory's file, not the repository root's directory.
    assert_eq!(fixture.hash("docs", &["collision.md"]), near);

    // `@`: the first configured root's file, not the second root's directory.
    let first_root = fixture.hash_of("docs/first/collision.md");
    assert_ne!(first_root, fixture.hash_of("docs/later/collision.md"));
    let magic = fixture.md("docs", &["--magic-root", "first", "--magic-root", "later", "hash", "@collision.md"]);
    assert!(magic.status.success(), "{magic:?}");
    assert_eq!(String::from_utf8(magic.stdout).unwrap().trim(), first_root);

    // `^`: the package's file, not the repository root's directory.
    assert_eq!(fixture.hash("area/pkg", &["^collision.md"]), fixture.hash_of("area/pkg/collision.md"));
}

#[test]
fn an_explicit_directory_still_hashes_as_a_directory() {
    let fixture = Collision::new("hash_entry_precedence_directory");
    let far_directory = fixture.hash_of("collision.md");
    assert!(is_directory_hash(&far_directory), "aggregate form: {far_directory}");
    assert_eq!(fixture.hash("docs", &["../collision.md"]), far_directory);
    assert_eq!(fixture.hash("docs", &["&collision.md"]), far_directory);
}

#[test]
fn diff_and_save_act_on_the_earlier_file() {
    let fixture = Collision::new("hash_entry_precedence_diff_save");
    let near_path = fixture.source().join("docs/collision.md");
    let far_path = fixture.source().join("collision.md/far.md");
    let far_before = std::fs::read_to_string(&far_path).unwrap();

    let diff = fixture.md("docs", &["hash", "--diff", "collision.md"]);
    assert_eq!(diff.status.code(), Some(2), "{diff:?}");
    assert!(
        String::from_utf8_lossy(&diff.stdout).contains("No stored hash to compare against"),
        "{diff:?}"
    );

    let save = fixture.md("docs", &["hash", "--save", "collision.md"]);
    assert!(save.status.success(), "{save:?}");
    assert!(String::from_utf8_lossy(&save.stdout).contains("wrote initial"), "{save:?}");
    let near_after = std::fs::read_to_string(&near_path).unwrap();
    assert!(near_after.contains("hash:"), "the earlier file gained its baseline: {near_after}");
    assert_eq!(std::fs::read_to_string(&far_path).unwrap(), far_before);

    // The saved baseline now matches the earlier file.
    let rediff = fixture.md("docs", &["hash", "--diff", "collision.md"]);
    assert_eq!(rediff.status.code(), Some(0), "{rediff:?}");
}

#[test]
fn an_earlier_candidate_that_cannot_be_probed_fails_with_io() {
    let fixture = Collision::new("hash_entry_precedence_io");
    self_referencing_symlink(&fixture.source().join("docs/loop.md"));
    // Control: the later directory itself hashes.
    assert!(is_directory_hash(&fixture.hash_of("loop.md")));

    let output = fixture.md("docs", &["hash", "loop.md"]);
    let stderr = strip_escape_codes(String::from_utf8_lossy(&output.stderr).as_ref());
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(failure_class(&stderr), Some("io"), "stdout {:?} stderr {stderr}", output.stdout);
}

/// A symlink named `path` whose target is itself, so reading its metadata
/// fails with a loop error rather than `NotFound`.
fn self_referencing_symlink(path: &Path) {
    let name = path.file_name().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(name, path).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(name, path)
        .expect("creating a file symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");
}

/// The class named by the first `failure: <name>` row.
fn failure_class(text: &str) -> Option<&str> {
    let (_, rest) = text.split_once("failure:")?;
    rest.split_whitespace().next()
}
