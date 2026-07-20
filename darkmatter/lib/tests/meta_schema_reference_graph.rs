//! `$schema` file-reference delegation: multi-hop dependency edges and the
//! cycle/depth guard.
//!
//! A schema file whose whole payload is a reference (`$schema: ./other.yaml`)
//! delegates to that file, and root-union file arms do the same. Both paths
//! re-enter reference resolution, so this module pins two contracts:
//!
//! - **Every hop is a dependency edge.** `EffectiveSchema::dependencies`
//!   promises that editing any listed file invalidates a cached schema. A
//!   chain that reported only its first hop would let an edit to the terminal
//!   file go unnoticed.
//! - **A loop is a structured error, not a stack overflow.** Without a
//!   canonical-path frame stack these tests would recurse until the process
//!   died — an unrecoverable crash rather than a diagnosable `SchemaError`.
//! - **Depth exhaustion is not a loop.** A chain of distinct files that
//!   outruns the cap reports `ReferenceDepthExceeded`, whose advice is to
//!   flatten the chain. Reporting it as a cycle would send the author hunting
//!   for a self-reference that is not there.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::schemas::{
    SchemaError, SchemaOriginKind, resolve::resolve_yaml_schema,
};

/// Mirrors the resolver's private `MAX_REFERENCE_DEPTH`. The depth tests
/// assert the error's reported `limit` against this, so changing the
/// production cap fails here loudly instead of silently moving the boundary
/// out from under the fixtures.
const MAX_REFERENCE_DEPTH: usize = 32;

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("write schema fixture");
    path.canonicalize().expect("canonicalize schema fixture")
}

fn reference(target: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(target.to_string())
}

/// Writes `len` schema files where each is a pure redirect to the next and
/// only the last declares a schema, returning the entry reference. Every file
/// is distinct, so the only guard such a chain can trip is the depth cap.
fn write_delegation_chain(dir: &Path, len: usize) -> String {
    assert!(len >= 2, "a delegation chain needs at least one redirect");
    for index in 0..len - 1 {
        write(
            dir,
            &format!("link{index:02}.yaml"),
            &format!("$schema: ./link{:02}.yaml\n", index + 1),
        );
    }
    write(
        dir,
        &format!("link{:02}.yaml", len - 1),
        "$schema:\n  title: 'string(required)'\n",
    );
    "./link00.yaml".to_string()
}

/// Renders the terminal status block with whitespace collapsed, so assertions
/// on advice wording survive the width-dependent wrapping a 32-path chain
/// forces.
fn render_advice(err: &SchemaError) -> String {
    let term = biscuit_terminal::terminal::Terminal::default();
    let rendered = strip_escape_codes(err.status_block(&term).render_optimistic(Some(100)));
    rendered.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn multi_hop_chain_resolves_and_records_every_hop() {
    // document -> a.yaml -> b.yaml -> real schema. `a.yaml` and `b.yaml` are
    // pure redirects; only the terminal file declares properties.
    let dir = tempfile::tempdir().unwrap();
    let a = write(dir.path(), "a.yaml", "$schema: ./b.yaml\n");
    let b = write(dir.path(), "b.yaml", "$schema: ./terminal.yaml\n");
    let terminal = write(
        dir.path(),
        "terminal.yaml",
        "$schema:\n  title: 'string(required)'\n",
    );

    let resolved = resolve_yaml_schema(&reference("./a.yaml"), dir.path())
        .expect("a terminating delegation chain resolves");

    // The schema is the terminal file's, not an empty redirect shell.
    assert_eq!(
        resolved.json_schema["required"].as_array().unwrap(),
        &vec![serde_json::Value::String("title".into())],
    );
    assert!(resolved.simplified.is_some());

    // Every hop is a dependency edge: editing `terminal.yaml` must be able to
    // invalidate a schema cached for this document.
    let mut expected = vec![a, b, terminal.clone()];
    expected.sort();
    assert_eq!(
        resolved.referenced_files, expected,
        "the dependency list must contain every hop, sorted and deduplicated",
    );

    // The origin is the file that authors the schema, not the redirect that
    // has no declaration for `relatedInformation` to point at.
    assert_eq!(resolved.origin.kind, SchemaOriginKind::ReferencedFile);
    assert_eq!(
        resolved.origin.uri.as_deref().and_then(Path::file_name),
        terminal.file_name(),
        "the redirect must not claim an origin the declaration owns",
    );
}

#[test]
fn self_referencing_file_is_a_structured_cycle_error() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.yaml", "$schema: ./a.yaml\n");

    let err = resolve_yaml_schema(&reference("./a.yaml"), dir.path())
        .expect_err("a file that references itself cannot resolve");

    let SchemaError::ReferenceCycle { chain } = err else {
        panic!("expected a reference cycle, got {err:?}");
    };
    assert!(chain.contains("a.yaml"), "chain must name the file: {chain}");
}

#[test]
fn two_file_cycle_is_a_structured_cycle_error() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.yaml", "$schema: ./b.yaml\n");
    write(dir.path(), "b.yaml", "$schema: ./a.yaml\n");

    let err = resolve_yaml_schema(&reference("./a.yaml"), dir.path())
        .expect_err("mutually referencing files cannot resolve");

    let SchemaError::ReferenceCycle { chain } = err else {
        panic!("expected a reference cycle, got {err:?}");
    };
    assert!(chain.contains("a.yaml"), "chain must name a.yaml: {chain}");
    assert!(chain.contains("b.yaml"), "chain must name b.yaml: {chain}");
}

#[test]
fn root_union_arm_cycling_back_into_the_chain_is_rejected() {
    // A root-union file arm re-enters reference resolution just like a scalar
    // delegation, so a union arm pointing back at an open file is the same
    // unbounded recursion by a different door.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.yaml", "$schema: ./union.yaml\n");
    write(
        dir.path(),
        "union.yaml",
        "$schema:\n  - title: 'string(required)'\n  - ./a.yaml\n",
    );

    let err = resolve_yaml_schema(&reference("./a.yaml"), dir.path())
        .expect_err("a union arm may not re-enter an open file");

    let SchemaError::ReferenceCycle { chain } = err else {
        panic!("expected a reference cycle, got {err:?}");
    };
    assert!(chain.contains("a.yaml"), "chain must name a.yaml: {chain}");
    assert!(
        chain.contains("union.yaml"),
        "chain must name union.yaml: {chain}",
    );
}

#[test]
fn document_root_union_arm_cycling_through_a_chain_is_rejected() {
    // The cycle enters from an inline document-level root union rather than a
    // schema file, exercising the same guard from the other entry point.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.yaml", "$schema: ./b.yaml\n");
    write(dir.path(), "b.yaml", "$schema: ./a.yaml\n");

    let union = serde_yaml_ng::Value::Sequence(vec![reference("./a.yaml")]);
    let err = resolve_yaml_schema(&union, dir.path())
        .expect_err("a document union arm entering a cycle cannot resolve");

    assert!(
        matches!(err, SchemaError::ReferenceCycle { .. }),
        "expected a reference cycle, got {err:?}",
    );
}

#[test]
fn a_file_referenced_twice_without_a_cycle_still_resolves() {
    // Diamond, not a loop: `union.yaml` names `shared.yaml` in two arms. The
    // guard tracks *open* frames, so a repeat visit on a closed frame is fine.
    let dir = tempfile::tempdir().unwrap();
    let shared = write(
        dir.path(),
        "shared.yaml",
        "$schema:\n  title: 'string(required)'\n",
    );
    let union = write(
        dir.path(),
        "union.yaml",
        "$schema:\n  - ./shared.yaml\n  - ./shared.yaml\n",
    );

    let resolved = resolve_yaml_schema(&reference("./union.yaml"), dir.path())
        .expect("a repeated non-cyclic reference resolves");

    let mut expected = vec![shared, union];
    expected.sort();
    assert_eq!(
        resolved.referenced_files, expected,
        "a file named twice contributes one deduplicated edge",
    );
}

#[test]
fn the_deepest_permitted_acyclic_chain_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let entry = write_delegation_chain(dir.path(), MAX_REFERENCE_DEPTH);

    let resolved = resolve_yaml_schema(&reference(&entry), dir.path())
        .expect("a chain of exactly MAX_REFERENCE_DEPTH files is within the cap");

    assert_eq!(
        resolved.json_schema["required"].as_array().unwrap(),
        &vec![serde_json::Value::String("title".into())],
        "the terminal file's schema must survive a full-depth chain",
    );
    assert_eq!(
        resolved.referenced_files.len(),
        MAX_REFERENCE_DEPTH,
        "every hop of a full-depth chain is still a dependency edge",
    );
}

#[test]
fn one_hop_past_the_cap_reports_depth_exhaustion_not_a_cycle() {
    let dir = tempfile::tempdir().unwrap();
    let entry = write_delegation_chain(dir.path(), MAX_REFERENCE_DEPTH + 1);

    let err = resolve_yaml_schema(&reference(&entry), dir.path())
        .expect_err("a chain one file past the cap cannot resolve");

    let SchemaError::ReferenceDepthExceeded { limit, chain } = &err else {
        panic!("expected depth exhaustion, got {err:?}");
    };
    assert_eq!(*limit, MAX_REFERENCE_DEPTH);
    assert!(
        chain.contains(&format!("link{MAX_REFERENCE_DEPTH:02}.yaml")),
        "chain must end at the hop that would have exceeded the cap: {chain}",
    );

    // Every file in this chain is distinct, so the diagnosis is the contract,
    // not just the variant: a regression back to `ReferenceCycle` would send
    // the author looking for a loop that is not there.
    let display = err.to_string();
    assert!(
        display.contains("depth limit"),
        "the message must describe depth exhaustion; got: {display}",
    );
    assert!(
        !display.contains("cycle"),
        "an acyclic chain must never be called a cycle; got: {display}",
    );

    let advice = render_advice(&err);
    assert!(
        advice.contains("Flatten the delegation"),
        "recovery advice must be about shortening the chain; got: {advice}",
    );
    assert!(
        !advice.contains("Break the loop"),
        "recovery advice must not tell the author to break a loop; got: {advice}",
    );
}

#[test]
fn a_root_union_arm_is_bounded_by_the_same_depth_cap() {
    // The union arm is the resolver's other recursive entry point. If only the
    // scalar route were capped, wrapping the chain in a union would bypass the
    // guard entirely.
    let dir = tempfile::tempdir().unwrap();
    let entry = write_delegation_chain(dir.path(), MAX_REFERENCE_DEPTH + 1);

    let union = serde_yaml_ng::Value::Sequence(vec![reference(&entry)]);
    let err = resolve_yaml_schema(&union, dir.path())
        .expect_err("a union arm may not outrun the depth cap");

    let SchemaError::ReferenceDepthExceeded { limit, .. } = &err else {
        panic!("expected depth exhaustion, got {err:?}");
    };
    assert_eq!(*limit, MAX_REFERENCE_DEPTH);
}
