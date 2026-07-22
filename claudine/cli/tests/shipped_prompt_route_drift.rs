//! Drift guard binding the repository's shipped `implement` prompt route to the
//! Level 2 regression fixture that exercises it
//! (`level2_lifecycle_shipped_implement_route_matches_direct_run`,
//! `features/2026-07-13-proxy-with/spec.md:1068-1072`).
//!
//! ## Why a fixture copy exists at all
//!
//! The spec's motivating route is `prompts/implement.md` ->
//! `prompts/_implement/implement-plan.md`. The router is executed **verbatim**
//! by the Level 2 row, so its routing conditions need no copy. The target
//! cannot be: `implement-plan.md` carries three side effects that are
//! disqualifying in an automated suite, none of which the spec's requirement is
//! about.
//!
//! - `say:` is a direct `biscuit_speaks` library call with no configuration
//!   gate, so a test running the shipped file speaks aloud on the host.
//! - `effect: sad-trombone` plays audio through `playa`, likewise ungated.
//! - `shell: "git add ."` / `shell: "just commit"` are denied without an
//!   interactive approval handler, which diverts the run to `blocked` +
//!   `finalize` before the loop's second iteration — so the shipped file could
//!   not demonstrate multi-phase execution even if the audio were tolerable.
//!
//! The fixture is therefore the shipped target minus exactly those three
//! side-effect properties. Everything the requirement *is* about — the `$schema`
//! block, the computed `plan`/`phase`/`total_phases` properties, and the
//! multi-phase `loop:` — is byte-identical, and this file is what keeps it that
//! way.
//!
//! ## What is guarded
//!
//! 1. [`shipped_implement_prompts_have_not_drifted_from_their_fixture`] pins the
//!    Darkmatter `Simple` hash (`md hash` semantics) of both shipped documents.
//!    Any edit to either file fails this test, forcing a human to re-derive the
//!    fixture rather than letting the two drift apart silently.
//! 2. [`fixture_preserves_the_shipped_schema_and_loop_semantics`] compares the
//!    load-bearing frontmatter keys structurally, so a shipped change to the
//!    loop or schema cannot be waved through by refreshing the hash alone.
//! 3. [`shipped_router_carries_no_side_effect_actions`] protects the premise
//!    that the router is safe to execute verbatim. If someone adds a `say:`,
//!    `effect:`, or `shell:` to the router, this fails and the Level 2 row needs
//!    a fixture copy of the router too.
//!
//! Refresh the pinned hashes after **reviewing** the shipped change and
//! re-deriving the fixture:
//!
//! ```text
//! CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift
//! ```

use darkmatter::markdown::Markdown;
use darkmatter::markdown::hash::{MdHashKind, MdHashOptions};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Repository-relative paths of the shipped documents this route is built from.
const SHIPPED_ROUTER: &str = "prompts/implement.md";
const SHIPPED_PLAN: &str = "prompts/_implement/implement-plan.md";

/// Frontmatter keys the fixture must reproduce byte-for-byte. These are the
/// routing/loop semantics the Level 2 row asserts; the deliberate fixture delta
/// is confined to keys outside this set.
const LOAD_BEARING_PLAN_KEYS: &[&str] = &[
    "$schema",
    "plan",
    "phase",
    "total_phases",
    "spec",
    "area",
    "pass_icon",
    "loop",
];

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<repo>/claudine/cli`.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repository root is two levels above claudine/cli")
        .to_path_buf()
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/shipped_implement_route")
}

fn hashes_file() -> PathBuf {
    fixture_dir().join("shipped-hashes.json")
}

fn read_doc(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("shipped prompt {} is unreadable: {e}", path.display()))
}

/// The Darkmatter `Simple` hash — the `md hash` default — rendered as
/// `"{frontmatter}-{body}"`.
fn simple_hash(source: &str) -> String {
    let markdown: Markdown = source.into();
    markdown
        .compute_hash(MdHashKind::Simple, &MdHashOptions::default())
        .flat_string()
        .expect("Simple hash always flattens")
}

fn current_hashes() -> BTreeMap<String, String> {
    let root = repo_root();
    [SHIPPED_ROUTER, SHIPPED_PLAN]
        .iter()
        .map(|rel| {
            let hash = simple_hash(&read_doc(&root.join(rel)));
            ((*rel).to_string(), hash)
        })
        .collect()
}

/// A shipped prompt changed without its Level 2 fixture being re-derived.
///
/// This is intentionally a *review* gate, not a correctness assertion: the hash
/// carries no meaning beyond "the bytes we last looked at". When it fires, diff
/// the shipped file, decide whether the fixture needs the same edit, apply it,
/// then refresh the pin.
#[test]
fn shipped_implement_prompts_have_not_drifted_from_their_fixture() {
    let current = current_hashes();

    if std::env::var_os("CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES").is_some() {
        let rendered =
            serde_json::to_string_pretty(&current).expect("hash map serializes") + "\n";
        fs::write(hashes_file(), rendered).expect("write refreshed hashes");
        return;
    }

    let recorded: BTreeMap<String, String> = serde_json::from_str(&read_doc(&hashes_file()))
        .expect("committed shipped-hashes.json is valid JSON");

    assert_eq!(
        current,
        recorded,
        "a shipped prompt in the `implement` route changed.\n\
         The Level 2 row `level2_lifecycle_shipped_implement_route_matches_direct_run` \
         runs `{SHIPPED_ROUTER}` verbatim and a side-effect-free copy of `{SHIPPED_PLAN}` \
         at `claudine/cli/tests/fixtures/shipped_implement_route/`.\n\
         Re-derive that fixture from the new shipped bytes (keeping only the \
         documented `say:`/`effect:`/`shell:` removals), then refresh this pin with:\n\
         \x20 CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli \
         --test shipped_prompt_route_drift"
    );
}

/// The fixture's deliberate delta must never reach the schema or the loop.
///
/// The hash guard above catches *any* shipped edit, but it can be refreshed
/// without touching the fixture. This one cannot: it fails whenever the shipped
/// and fixture copies disagree on the frontmatter that decides how many phases
/// run and how the plan/spec parameters resolve.
#[test]
fn fixture_preserves_the_shipped_schema_and_loop_semantics() {
    let shipped: Markdown = read_doc(&repo_root().join(SHIPPED_PLAN)).into();
    let fixture: Markdown = read_doc(&fixture_dir().join("_implement/implement-plan.md")).into();

    let shipped_fm = shipped.frontmatter();
    let shipped_fm = shipped_fm.as_map();
    let fixture_fm = fixture.frontmatter();
    let fixture_fm = fixture_fm.as_map();

    for key in LOAD_BEARING_PLAN_KEYS {
        assert_eq!(
            fixture_fm.get(*key),
            shipped_fm.get(*key),
            "the Level 2 fixture must reproduce the shipped `{key}` exactly; the \
             fixture's only sanctioned delta is removing the `say:`, `effect:`, and \
             `shell:` side effects. Copy the shipped `{key}` into \
             claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md"
        );
    }
}

/// The Level 2 row executes the shipped router verbatim, which is only safe
/// while the router stays side-effect free.
#[test]
fn shipped_router_carries_no_side_effect_actions() {
    let source = read_doc(&repo_root().join(SHIPPED_ROUTER));
    let markdown: Markdown = source.into();
    let frontmatter = serde_json::to_string(markdown.frontmatter().as_map())
        .expect("frontmatter serializes to JSON");

    for property in ["\"say\"", "\"speak\"", "\"effect\"", "\"shell\""] {
        assert!(
            !frontmatter.contains(property),
            "`{SHIPPED_ROUTER}` gained a {property} lifecycle property. The Level 2 row \
             `level2_lifecycle_shipped_implement_route_matches_direct_run` runs the router \
             verbatim precisely because it had none: TTS, audio, and shell approval all \
             make an automated run non-deterministic. Either revert the router change or \
             give the router a side-effect-free fixture copy as well."
        );
    }
}
