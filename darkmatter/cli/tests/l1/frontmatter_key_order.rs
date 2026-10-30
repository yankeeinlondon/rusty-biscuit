//! Frontmatter key order through the `md` binary: `get`/`set`/`rm` keep the
//! author's order at every depth, the non-strict hash ignores it, and the
//! strict hash follows it.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

const DOCUMENT: &str = "---\n\
zeta: 1\n\
meta:\n  zulu: z\n  alpha: a\n  nested:\n    yankee: 1\n    bravo: 2\n\
alpha: 2\n\
---\n# H\n\nBody.\n";

/// The document's frontmatter block, between the fences.
fn frontmatter_block(text: &str) -> &str {
    let rest = text.strip_prefix("---\n").expect("opening fence");
    &rest[..rest.find("\n---\n").expect("closing fence") + 1]
}

/// Read, edit, save, reread, and repeat: every pass leaves top-level and
/// nested order as authored, a replaced key in place, and a new key last.
#[test]
fn get_set_rm_round_trip_keeps_author_order_at_every_depth() {
    let fixture = CliProcessFixture::named("frontmatter-key-order-round-trip");
    let file = fixture.write_file("doc.md", DOCUMENT);
    let get = |property: &str| {
        let output = fixture.command().arg("get").arg(&file).arg(property).output().unwrap();
        assert!(output.status.success(), "md get {property} failed: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    };

    let expected_meta = "{\n  \"zulu\": \"z\",\n  \"alpha\": \"a\",\n  \"nested\": {\n    \"yankee\": 1,\n    \"bravo\": 2\n  }\n}\n";
    for pass in 0..2 {
        assert_eq!(get("meta"), expected_meta, "pass {pass}: nested order read back");

        // Replacing an existing key keeps its position.
        fixture.command().arg("set").arg(&file).args(["alpha", "3", "--save"]).assert().success();
        // A new key appends, keeping its own object's order.
        fixture
            .command()
            .arg("set")
            .arg(&file)
            .args(["beta", r#"{"yankee":1,"bravo":2}"#, "--save"])
            .assert()
            .success();
        let edited = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            frontmatter_block(&edited),
            "zeta: 1\nmeta:\n  zulu: z\n  alpha: a\n  nested:\n    yankee: 1\n    bravo: 2\n\
             alpha: 3\nbeta:\n  yankee: 1\n  bravo: 2\n",
            "pass {pass}: after set"
        );
        assert!(edited.ends_with("---\n# H\n\nBody.\n"), "pass {pass}: body kept");
        assert_eq!(get("beta"), "{\n  \"yankee\": 1,\n  \"bravo\": 2\n}\n");

        // Replacing a nested object writes the new object in its own order,
        // still at the key's position.
        fixture
            .command()
            .arg("set")
            .arg(&file)
            .args(["meta", r#"{"zulu":"z2","alpha":"a","nested":{"yankee":1,"bravo":2},"new":1}"#, "--save"])
            .assert()
            .success();
        assert_eq!(
            frontmatter_block(&std::fs::read_to_string(&file).unwrap()),
            "zeta: 1\nmeta:\n  zulu: z2\n  alpha: a\n  nested:\n    yankee: 1\n    bravo: 2\n  new: 1\n\
             alpha: 3\nbeta:\n  yankee: 1\n  bravo: 2\n",
            "pass {pass}: after nested replace"
        );

        // Removing a key closes the gap without moving its neighbors.
        fixture.command().arg("rm").arg(&file).arg("beta").assert().success();
        fixture
            .command()
            .arg("set")
            .arg(&file)
            .args(["meta", r#"{"zulu":"z","alpha":"a","nested":{"yankee":1,"bravo":2}}"#, "--save"])
            .assert()
            .success();
        fixture.command().arg("set").arg(&file).args(["alpha", "2", "--save"]).assert().success();
        // Every pass ends where it began, so the next pass starts from the
        // original document read back from disk.
        assert_eq!(std::fs::read_to_string(&file).unwrap(), DOCUMENT, "pass {pass}: back to the original");
    }
}

/// A saved non-strict baseline survives a reorder of nested keys only: no
/// semantic change, no rewrite, and no `last_updated` bump.
#[test]
fn hash_baseline_ignores_a_nested_only_reorder() {
    let fixture = CliProcessFixture::named("frontmatter-key-order-hash-baseline");
    let file = fixture.write_file(
        "doc.md",
        "---\ntitle: T\nlast_updated: 2026-01-01\nmeta:\n  zulu: z\n  alpha: a\n  nested:\n    yankee: 1\n    bravo: 2\n---\n# H\n\nBody.\n",
    );
    fixture
        .command()
        .args(["hash", "--save"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("baseline"));
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("\nhash: "), "baseline written:\n{saved}");

    let reordered = saved
        .replace("  zulu: z\n  alpha: a\n", "  alpha: a\n  zulu: z\n")
        .replace("    yankee: 1\n    bravo: 2\n", "    bravo: 2\n    yankee: 1\n");
    assert_ne!(reordered, saved, "the reorder edit applies");
    std::fs::write(&file, &reordered).unwrap();

    fixture
        .command()
        .args(["hash", "--diff"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("No semantic changes detected"));
    fixture.command().args(["hash", "--save"]).arg(&file).assert().success();
    let after = std::fs::read_to_string(&file).unwrap();
    assert_eq!(after, reordered, "the baseline is not rewritten");
    assert!(after.contains("last_updated: 2026-01-01\n"), "no timestamp bump");
}

/// Strict hashes follow author key order at every depth. A strict baseline
/// is not migrated: a nested-only reorder reports a frontmatter change until
/// the author refreshes it through the normal save path.
#[test]
fn strict_hash_follows_nested_author_order_and_is_refreshed_only_by_save() {
    let fixture = CliProcessFixture::named("frontmatter-key-order-strict");
    let hash = |args: &[&str], stdin: &str| {
        let output = fixture.command().args(args).write_stdin(stdin.to_string()).output().unwrap();
        assert!(output.status.success(), "md {args:?} failed: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    let written = "---\nmeta:\n  zulu: z\n  alpha: a\n---\nBody.\n";
    let sorted = "---\nmeta:\n  alpha: a\n  zulu: z\n---\nBody.\n";
    assert_ne!(hash(&["hash", "--strict", "-"], written), hash(&["hash", "--strict", "-"], sorted));
    assert_eq!(hash(&["hash", "-"], written), hash(&["hash", "-"], sorted));

    let file = fixture.write_file("doc.md", written);
    fixture.command().args(["hash", "--strict", "--save"]).arg(&file).assert().success();
    let saved = std::fs::read_to_string(&file).unwrap();
    let reordered = saved.replace("  zulu: z\n  alpha: a\n", "  alpha: a\n  zulu: z\n");
    std::fs::write(&file, &reordered).unwrap();

    fixture
        .command()
        .args(["hash", "--strict", "--diff"])
        .arg(&file)
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Frontmatter has changed, body remains unchanged"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), reordered, "--diff never migrates");

    fixture.command().args(["hash", "--strict", "--save"]).arg(&file).assert().success();
    assert_ne!(std::fs::read_to_string(&file).unwrap(), reordered, "save refreshes the baseline");
    fixture.command().args(["hash", "--strict", "--diff"]).arg(&file).assert().success();
}
