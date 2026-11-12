//! Contracts for `scripts/shipped_schemas.py`, the root `just shipped-schemas`
//! coordinator that regenerates the three shipped schema artifacts
//! (Simplified Schema, Darkmatter, Claudine) in dependency order.
//!
//! Its observable contract: every artifact is generated and accepted before
//! any file is replaced, each generator over the lower candidates emitted
//! before it, so a failed, malformed, or refused generation (any owner's) or a
//! lower candidate that differs from the one a generator built over leaves
//! all three files exactly as they were and no temporary file behind; a
//! successful run replaces them, and a repeated run is a byte-identical
//! no-op. The real generators are the crates' `shipped_schemas` examples,
//! whose own suites test their acceptance; here the script runs through its
//! normal command line with `--generator` naming a stand-in that prints
//! canned artifacts from a scratch tree, so no test builds a crate.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    let manifest_dir = biscuit_test_harness::manifest_dir!();
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .expect("test-toolkit must live under <repo>/tools/test-toolkit")
        .to_path_buf()
}

/// `python3` everywhere this suite runs, `python` where only the Windows
/// spelling exists.
fn python() -> Command {
    for name in ["python3", "python"] {
        if Command::new(name).arg("--version").output().is_ok_and(|output| output.status.success()) {
            return Command::new(name);
        }
    }
    panic!("neither python3 nor python is on PATH; the coordinator needs one");
}

const ARTIFACTS: [(&str, &str); 3] = [
    ("simplified-schema", "simplified-schema/lib/generated/shipped.json"),
    ("darkmatter", "darkmatter/lib/generated/shipped.json"),
    ("claudine", "claudine/lib/generated/shipped.json"),
];

/// The stand-in generator, modeling the examples' emit mode: it takes one
/// `--lower <owner>=<path>` per lower owner and exits 2 on any other
/// argument; it fails when `fake/<package>.fail` exists; it refuses (exit 1,
/// nothing printed) when a lower candidate differs from the lower it "built
/// over" (`fake/<package>.over-<owner>.json`, else `fake/<owner>.json`) or
/// when `fake/<package>.refuse` exists; otherwise it prints
/// `fake/<package>.json`.
const FAKE_GENERATOR: &str = r#"import sys
from pathlib import Path
ORDER = ["simplified-schema", "darkmatter", "claudine"]
package, rest = sys.argv[1], sys.argv[2:]
fake = Path.cwd() / "fake"
lowers = {}
while rest:
    if rest[0] != "--lower" or len(rest) < 2:
        print(f"unexpected arguments {rest}", file=sys.stderr)
        sys.exit(2)
    owner, _, path = rest[1].partition("=")
    lowers[owner] = Path(path).read_bytes()
    rest = rest[2:]
if sorted(lowers) != sorted(ORDER[:ORDER.index(package)]):
    print(f"{package} was given lowers {sorted(lowers)}", file=sys.stderr)
    sys.exit(2)
if (fake / f"{package}.fail").exists():
    print(f"{package} does not prepare", file=sys.stderr)
    sys.exit(1)
for owner, candidate in lowers.items():
    built_over = fake / f"{package}.over-{owner}.json"
    if not built_over.exists():
        built_over = fake / f"{owner}.json"
    if candidate != built_over.read_bytes():
        print(f"the {owner} candidate is not the one {package} is built over", file=sys.stderr)
        sys.exit(1)
if (fake / f"{package}.refuse").exists():
    print(f"freshly generated {package} is refused", file=sys.stderr)
    sys.exit(1)
sys.stdout.buffer.write((fake / f"{package}.json").read_bytes())
"#;

/// A scratch tree holding the three checked-in artifacts and the stand-in.
struct Tree {
    directory: tempfile::TempDir,
}

impl Tree {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("scratch tree");
        let fake = directory.path().join("fake");
        fs::create_dir_all(&fake).unwrap();
        fs::write(fake.join("generator.py"), FAKE_GENERATOR).unwrap();
        for (index, (owner, path)) in ARTIFACTS.iter().enumerate() {
            let target = directory.path().join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, format!("previous {owner}\n")).unwrap();
            let lowers: Vec<&str> = ARTIFACTS[..index].iter().map(|(owner, _)| *owner).collect();
            fs::write(fake.join(format!("{owner}.json")), artifact(owner, &lowers, "fresh")).unwrap();
        }
        Self { directory }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn fake(&self, name: &str) -> PathBuf {
        self.root().join("fake").join(name)
    }

    fn run(&self, script: &Path) -> Output {
        python()
            .arg(script)
            .arg("--root")
            .arg(self.root())
            .arg("--generator")
            .arg(self.fake("generator.py"))
            .current_dir(self.root())
            .output()
            .expect("the coordinator spawns")
    }

    fn artifacts(&self) -> Vec<Vec<u8>> {
        ARTIFACTS.iter().map(|(_, path)| fs::read(self.root().join(path)).unwrap()).collect()
    }

    fn previous(&self) -> Vec<Vec<u8>> {
        ARTIFACTS.iter().map(|(owner, _)| format!("previous {owner}\n").into_bytes()).collect()
    }

    fn leftovers(&self) -> Vec<PathBuf> {
        ARTIFACTS
            .iter()
            .map(|(_, path)| self.root().join(format!("{path}.tmp")))
            .filter(|temporary| temporary.exists())
            .collect()
    }
}

/// A minimal artifact in the generated layout: `owner`'s header naming
/// `lowers` as dependencies, and a payload marked `marker`.
fn artifact(owner: &str, lowers: &[&str], marker: &str) -> String {
    let dependencies: Vec<String> = lowers
        .iter()
        .map(|lower| format!(r#"{{"owner": "{lower}", "preparation_revision": 1, "digest": "0000000000000000"}}"#))
        .collect();
    format!(
        "{{\n\"header\": {{\"wire_version\": 1, \"owner\": \"{owner}\", \"preparation_revision\": 1, \"sources\": [], \"dependencies\": [{}], \"configuration\": []}},\n\"payload\": {{\n\"marker\":\"{marker}\"\n}}\n}}\n",
        dependencies.join(", ")
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_successful_run_replaces_every_artifact_and_a_repeat_is_a_no_op() {
    let script = repo_root().join("scripts/shipped_schemas.py");
    let tree = Tree::new();
    let first = tree.run(&script);
    assert!(first.status.success(), "{}", stderr(&first));
    let expected: Vec<Vec<u8>> =
        ARTIFACTS.iter().map(|(owner, _)| fs::read(tree.fake(&format!("{owner}.json"))).unwrap()).collect();
    assert_eq!(tree.artifacts(), expected);
    assert!(String::from_utf8_lossy(&first.stdout).contains("regenerated claudine/lib/generated/shipped.json"));

    let second = tree.run(&script);
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(tree.artifacts(), expected, "a repeated run writes the same bytes");
    assert!(String::from_utf8_lossy(&second.stdout).contains("every shipped artifact is current"));
    assert!(tree.leftovers().is_empty());
}

#[test]
fn a_failed_generation_leaves_all_three_artifacts_in_place() {
    let script = repo_root().join("scripts/shipped_schemas.py");
    let tree = Tree::new();
    // The last generator fails, after the other two already produced output.
    fs::write(tree.fake("claudine.fail"), "").unwrap();
    let output = tree.run(&script);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("claudine does not prepare"), "{}", stderr(&output));
    assert_eq!(tree.artifacts(), tree.previous());
    assert!(tree.leftovers().is_empty());
}

#[test]
fn output_that_is_not_the_owner_s_artifact_replaces_nothing() {
    let script = repo_root().join("scripts/shipped_schemas.py");
    for (case, darkmatter_output, expected) in [
        ("not JSON", "darkmatter\n".to_string(), "other than JSON"),
        ("another owner", artifact("claudine", &["simplified-schema"], "fresh"), "not 'darkmatter'"),
        ("a missing dependency", artifact("darkmatter", &[], "fresh"), "dependencies"),
        ("two final line feeds", format!("{}\n", artifact("darkmatter", &["simplified-schema"], "fresh")), "line feed"),
        ("nothing", String::new(), "printed nothing"),
    ] {
        let tree = Tree::new();
        fs::write(tree.fake("darkmatter.json"), darkmatter_output).unwrap();
        let output = tree.run(&script);
        assert_eq!(output.status.code(), Some(1), "{case}");
        assert!(stderr(&output).contains(expected), "{case}: {}", stderr(&output));
        assert_eq!(tree.artifacts(), tree.previous(), "{case}");
        assert!(tree.leftovers().is_empty(), "{case}");
    }
}

#[test]
fn any_owner_s_refused_candidate_replaces_nothing() {
    let script = repo_root().join("scripts/shipped_schemas.py");
    for (owner, _) in ARTIFACTS {
        let tree = Tree::new();
        fs::write(tree.fake(&format!("{owner}.refuse")), "").unwrap();
        let output = tree.run(&script);
        assert_eq!(output.status.code(), Some(1), "{owner}");
        assert!(stderr(&output).contains(&format!("freshly generated {owner} is refused")), "{owner}: {}", stderr(&output));
        assert_eq!(tree.artifacts(), tree.previous(), "{owner}");
        assert!(tree.leftovers().is_empty(), "{owner}");
    }
}

#[test]
fn a_lower_candidate_other_than_the_one_a_generator_built_over_replaces_nothing() {
    let script = repo_root().join("scripts/shipped_schemas.py");
    for (owner, lower) in [("darkmatter", "simplified-schema"), ("claudine", "simplified-schema"), ("claudine", "darkmatter")] {
        let tree = Tree::new();
        let lowers: Vec<&str> = ARTIFACTS.iter().map(|(owner, _)| *owner).take_while(|candidate| *candidate != lower).collect();
        fs::write(tree.fake(&format!("{owner}.over-{lower}.json")), artifact(lower, &lowers, "another generation")).unwrap();
        let output = tree.run(&script);
        assert_eq!(output.status.code(), Some(1), "{owner} over {lower}");
        assert!(
            stderr(&output).contains(&format!("the {lower} candidate is not the one {owner} is built over")),
            "{owner} over {lower}: {}",
            stderr(&output)
        );
        assert_eq!(tree.artifacts(), tree.previous(), "{owner} over {lower}");
        assert!(tree.leftovers().is_empty(), "{owner} over {lower}");
    }
}
