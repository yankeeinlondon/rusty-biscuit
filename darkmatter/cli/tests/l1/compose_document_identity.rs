//! Document identity through the normal `md` invocation path: `ctx.hash`
//! parity with `md hash` (AC3) and the identity a `::file` fragment inherits
//! from its root (AC5, and the transclusion half of AC30).
//!
//! The library suite proves these values in process against
//! `RootDocument`'s own `compute_hash` call, which cannot distinguish a correct
//! implementation from a hash function compared with itself. `md hash` is an
//! independent second reader of the same file, so agreement between the two
//! binaries' answers is real parity evidence.

use crate::common;

use std::collections::HashMap;

use common::CliProcessFixture;

/// Every identity key a fragment must inherit, rendered as `name=[value]` so
/// one composed line carries the whole set.
const IDENTITY_PROBES: &str = "self=[{{ ctx.self }}] hash=[{{ ctx.hash }}] id=[{{ ctx.id }}] sid=[{{ ctx.sid }}]";

/// Parse `prefix name=[value] ...` pairs out of the composed line whose first
/// word is `prefix`.
fn probes(stdout: &str, prefix: &str) -> HashMap<String, String> {
    let line = stdout
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no `{prefix}` line in:\n{stdout}"));
    line.split_whitespace()
        .filter_map(|token| {
            let (name, value) = token.split_once("=[")?;
            Some((name.to_string(), value.strip_suffix(']')?.to_string()))
        })
        .collect()
}

/// AC3: `ctx.hash` for a document is the value `md hash` prints for the same
/// file.
///
/// The document's own frontmatter carries the uninterpolated `{{ ctx.hash }}`,
/// which both readers hash: `md hash` reads the file from disk, and `ctx.hash`
/// is the root's identity as loaded, before any interpolation. So the expected
/// value is *not* derivable from the composed output, and a `ctx.hash` that
/// leaked the post-compose text would disagree.
#[test]
fn ctx_hash_equals_what_md_hash_prints_for_the_same_file() {
    let fixture = CliProcessFixture::named("ctx_hash_md_hash_parity");
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\ntitle: parity\nh: \"{{ ctx.hash }}\"\n---\n\nSome body text.\n",
    );

    let hashed = fixture
        .command()
        .arg("hash")
        .arg(&document)
        .assert()
        .success();
    let expected = String::from_utf8(hashed.get_output().stdout.clone())
        .expect("md emits UTF-8")
        .trim()
        .to_string();
    assert!(
        expected.contains('-') && expected.len() > 16,
        "`md hash` must print a `{{fm}}-{{body}}` simple hash; it printed `{expected}`"
    );

    let composed = fixture
        .command()
        .arg("compose")
        .arg("--frontmatter")
        .arg(&document)
        .assert()
        .success();
    let stdout = String::from_utf8(composed.get_output().stdout.clone()).expect("md emits UTF-8");

    assert!(
        stdout.lines().any(|line| line == format!("h: {expected}")),
        "`ctx.hash` must equal the `md hash` output `{expected}`; composed \
         frontmatter was:\n{stdout}"
    );
}

/// AC5 and AC30: a `::file` fragment composes under the root document's
/// identity — the same `ctx.self`, `ctx.hash`, `ctx.id`, and `ctx.sid` — and
/// `ctx.self` is the root's absolute, canonical, native-separated path.
///
/// The child is a real file with its own content and its own hash, so an
/// implementation that recaptured identity per fragment would answer with the
/// child's values and fail every assertion here.
#[test]
fn a_transcluded_fragment_composes_under_the_root_documents_identity() {
    let fixture = CliProcessFixture::named("transclusion_identity");
    fixture.write_file(
        "cwd/sub/child.md",
        &format!("CHILD {IDENTITY_PROBES}\n"),
    );
    let root = fixture.write_file(
        "cwd/root.md",
        &format!("ROOT {IDENTITY_PROBES}\n\n::file ./sub/child.md\n"),
    );

    let composed = fixture
        .command()
        .arg("compose")
        .arg(&root)
        .assert()
        .success();
    let stdout = String::from_utf8(composed.get_output().stdout.clone()).expect("md emits UTF-8");

    let root_probes = probes(&stdout, "ROOT ");
    let child_probes = probes(&stdout, "CHILD ");
    for key in ["self", "hash", "id", "sid"] {
        let value = &root_probes[key];
        assert!(
            !value.is_empty(),
            "the root must resolve `ctx.{key}`; composed output was:\n{stdout}"
        );
        assert_eq!(
            child_probes[key], *value,
            "a `::file` fragment must inherit the root's `ctx.{key}`; composed \
             output was:\n{stdout}"
        );
    }

    // `ctx.self` is the canonical spelling, which on macOS resolves the
    // `/var` -> `/private/var` symlink the fixture root is handed as. Putting
    // the fixture's own path through the same function is what makes the
    // comparison portable: it also means the Windows verbatim prefix cannot
    // make the two sides differ for a reason that is not the subject.
    let self_path = std::path::PathBuf::from(&root_probes["self"]);
    assert_eq!(
        self_path,
        biscuit_file::canonicalize_simplified(&root).expect("the fixture root exists"),
        "`ctx.self` must be the root document's canonical path"
    );
    assert!(self_path.is_absolute(), "`ctx.self` must be absolute");
    assert_eq!(
        self_path.file_name().and_then(|name| name.to_str()),
        Some("root.md"),
        "`ctx.self` names the root document, never the transcluded child"
    );
    // Native separators. On Windows a `/`-spelled path still *works*, so only
    // an explicit check on the emitted characters can catch it.
    let foreign = if cfg!(windows) { '/' } else { '\\' };
    assert!(
        !root_probes["self"].contains(foreign),
        "`ctx.self` must use native separators; it emitted `{}`",
        root_probes["self"]
    );
}
