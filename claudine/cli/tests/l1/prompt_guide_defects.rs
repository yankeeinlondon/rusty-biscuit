//! Level-1 coverage for the defect section of the shipped prompt guide
//! (`prompts/_prompt.md`): it finds the fix `2026-09-20-lifecycle-handoff-gaps`
//! by directory name, reads its `status` and `fixed:` list, and renders one
//! warning per finding that is still open.
//!
//! Each case composes a prompt that transcludes the shipped guide, in a
//! disposable repository holding temporary copies of a snapshot of the real
//! specification's frontmatter, with one edit per case. Every assertion reads
//! the composed guide (`claudine compose --dry-run`), never the frontmatter the
//! guide computes.

use crate::common;

use common::{CliProcessFixture, write};
use std::collections::BTreeSet;
use std::path::Path;

const GUIDE: &str = include_str!("../../../../prompts/_prompt.md");
/// The real specification's frontmatter as the guide first read it.
const SPEC: &str = include_str!("../fixtures/prompt_guide/spec.md");

const ACTIVE: &str = "claudine/fixes/2026-09-20-lifecycle-handoff-gaps/spec.md";
const COMPLETED: &str = "claudine/fixes/_completed/2026-09-20-lifecycle-handoff-gaps/spec.md";

/// Each finding id and a phrase only its warning contains.
const WARNINGS: [(&str, &str); 10] = [
    ("F1", "git state is reused across composition runs"),
    ("F2", "fired from inside a looping document is recorded but not performed"),
    ("F3", "is not pre-approved when it interpolates a value"),
    ("F4", "stack fires `failure` and `finalize`, but the process exits"),
    ("F5", "mentioned only in a transcluded file renders empty"),
    ("F6", "Some loop references describe a condition checked before each iteration"),
    ("F7", "task is refused"),
    ("F8", "and referencing one there fails the run"),
    ("D1", "Interpolation literals are converted only in a file"),
    ("D2", "run header can show a `description` wrongly"),
];

const SECTION: &str = "### Defects to design around";
const NOTICE: &str = "The defect list could not be read";

/// The spec snapshot with `status: finalized-spec` and `fixed: []`: the
/// control every case edits once.
fn control_spec() -> String {
    let fixed_start = SPEC.find("\nfixed:\n").expect("the snapshot has a `fixed:` list") + 1;
    let fixed_end = fixed_start + SPEC[fixed_start..].find("\narea:").expect("`area:` follows `fixed:`") + 1;
    let spec = format!("{}fixed: []\n{}", &SPEC[..fixed_start], &SPEC[fixed_end..]);
    replace_once(&spec, "status: draft-spec\n", "status: finalized-spec\n")
}

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` must occur exactly once");
    text.replacen(from, to, 1)
}

struct Composed {
    body: String,
}

impl Composed {
    fn section(&self) -> bool {
        self.body.contains(SECTION)
    }

    fn notice(&self) -> bool {
        self.body.contains(NOTICE)
    }

    fn warnings(&self) -> BTreeSet<&'static str> {
        WARNINGS
            .iter()
            .filter(|(_, phrase)| self.body.contains(phrase))
            .map(|(id, _)| *id)
            .collect()
    }
}

fn all_ids() -> BTreeSet<&'static str> {
    WARNINGS.iter().map(|(id, _)| *id).collect()
}

/// Composes a prompt transcluding the guide, with `specs` written at their
/// repository-relative paths.
fn compose(name: &str, specs: &[(&str, &str)]) -> Composed {
    let fixture = CliProcessFixture::named(name);
    fixture.initialize_repository();
    common::write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let repo = fixture.cwd();
    write(&repo.join("prompts/_prompt.md"), GUIDE);
    write(&repo.join("prompts/probe.md"), "# Probe\n\n::file ./_prompt.md\n");
    for (path, content) in specs {
        write(&repo.join(path), content);
    }
    let output = fixture
        .command()
        .args(["compose", "--dry-run", "--claude", "prompts/probe.md"])
        .output()
        .expect("claudine runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{name}: the guide must compose:\n{stdout}\n{stderr}");
    assert!(!stdout.contains("Could not transclude"), "{name}:\n{stdout}\n{stderr}");
    assert!(stdout.contains("## How Claudine Prompts Work"), "{name}: the guide is present:\n{stdout}");
    assert!(!fixture.audio_spool().exists(), "{name}: no audio escaped the dry run");
    Composed { body: stdout }
}

fn with_spec(name: &str, path: &str, spec: &str) -> Composed {
    compose(name, &[(path, spec)])
}

#[track_caller]
fn assert_notice_and_every_warning(case: &str, composed: &Composed, detail: &str) {
    assert!(composed.section(), "{case}: section\n{}", composed.body);
    assert!(composed.notice(), "{case}: notice\n{}", composed.body);
    assert!(composed.body.contains(detail), "{case}: expected `{detail}`\n{}", composed.body);
    assert_eq!(composed.warnings(), all_ids(), "{case}: every warning\n{}", composed.body);
}

#[test]
fn control_renders_every_warning_and_no_notice() {
    let composed = with_spec("guide-control", ACTIVE, &control_spec());
    assert!(composed.section());
    assert!(!composed.notice(), "{}", composed.body);
    assert_eq!(composed.warnings(), all_ids(), "{}", composed.body);
}

/// One test per finding id, so nextest runs the composes in parallel.
fn assert_listed_id_hides_only_its_warning(id: &'static str) {
    let spec = replace_once(&control_spec(), "fixed: []\n", &format!("fixed:\n    - {id}\n"));
    let composed = with_spec(&format!("guide-fixed-{id}"), ACTIVE, &spec);
    let mut expected = all_ids();
    expected.remove(id);
    assert!(!composed.notice(), "{id}:\n{}", composed.body);
    assert_eq!(composed.warnings(), expected, "{id}:\n{}", composed.body);
}

macro_rules! listed_id_tests {
    ($($test:ident => $id:literal),* $(,)?) => {
        $(#[test]
        fn $test() {
            assert_listed_id_hides_only_its_warning($id);
        })*
    };
}

listed_id_tests! {
    listed_f1_hides_exactly_its_own_warning => "F1",
    listed_f2_hides_exactly_its_own_warning => "F2",
    listed_f3_hides_exactly_its_own_warning => "F3",
    listed_f4_hides_exactly_its_own_warning => "F4",
    listed_f5_hides_exactly_its_own_warning => "F5",
    listed_f6_hides_exactly_its_own_warning => "F6",
    listed_f7_hides_exactly_its_own_warning => "F7",
    listed_f8_hides_exactly_its_own_warning => "F8",
    listed_d1_hides_exactly_its_own_warning => "D1",
    listed_d2_hides_exactly_its_own_warning => "D2",
}

/// The `listed_*` tests above name every finding id; a new one must be added.
#[test]
fn every_warning_id_has_a_listed_test() {
    let ids: Vec<&str> = WARNINGS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "D1", "D2"]);
}

#[test]
fn every_id_listed_renders_no_warning() {
    let list: String = WARNINGS.iter().map(|(id, _)| format!("    - {id}\n")).collect();
    let spec = replace_once(&control_spec(), "fixed: []\n", &format!("fixed:\n{list}"));
    let composed = with_spec("guide-all-fixed", ACTIVE, &spec);
    assert!(!composed.notice(), "{}", composed.body);
    assert!(composed.warnings().is_empty(), "{}", composed.body);
}

#[test]
fn a_completed_spec_hides_the_section() {
    let spec = replace_once(&control_spec(), "status: finalized-spec\n", "status: completed\n");
    let composed = with_spec("guide-completed", ACTIVE, &spec);
    assert!(!composed.section(), "{}", composed.body);
    assert!(composed.warnings().is_empty(), "{}", composed.body);

    // `status` alone decides: a malformed `fixed:` does not bring the section back.
    let spec = replace_once(&spec, "fixed: []\n", "fixed: F1\n");
    let composed = with_spec("guide-completed-bad-fixed", ACTIVE, &spec);
    assert!(!composed.section(), "{}", composed.body);
}

#[test]
fn a_relocated_spec_is_still_found() {
    let spec = replace_once(&control_spec(), "fixed: []\n", "fixed:\n    - F1\n");
    let composed = with_spec("guide-relocated", COMPLETED, &spec);
    assert!(!composed.notice(), "{}", composed.body);
    let mut expected = all_ids();
    expected.remove("F1");
    assert_eq!(composed.warnings(), expected, "{}", composed.body);
}

#[test]
fn a_missing_or_ambiguous_spec_is_reported() {
    let missing = compose("guide-missing", &[]);
    assert_notice_and_every_warning("missing", &missing, "no `spec.md` was found");

    let listed = replace_once(&control_spec(), "fixed: []\n", "fixed:\n    - F1\n");
    let ambiguous = compose("guide-ambiguous", &[(ACTIVE, &listed), (COMPLETED, &listed)]);
    assert_notice_and_every_warning("ambiguous", &ambiguous, "more than one specification matched");
    for path in [ACTIVE, COMPLETED] {
        let lifecycle = Path::new(path).parent().unwrap().to_string_lossy().replace('\\', "/");
        assert!(ambiguous.body.contains(&lifecycle), "names {lifecycle}:\n{}", ambiguous.body);
    }
}

/// The R10 input robustness matrix: every shape of `fixed:` and `status:`,
/// one edit from the control, each rendering a notice and every warning.
/// One test per cell, so nextest runs the composes in parallel.
fn assert_malformed_cell(case: &str, edit: impl FnOnce(&str) -> String, detail: &str) {
    let spec = edit(&control_spec());
    let composed = with_spec(&format!("guide-{}", case.replace(' ', "-")), ACTIVE, &spec);
    assert_notice_and_every_warning(case, &composed, detail);
}

macro_rules! malformed_cell_tests {
    ($($test:ident: $case:literal, |$c:ident| $edit:expr, $detail:literal;)*) => {
        $(#[test]
        fn $test() {
            assert_malformed_cell($case, |$c| $edit, $detail);
        })*
    };
}

malformed_cell_tests! {
    malformed_fixed_absent: "fixed absent", |c| replace_once(c, "fixed: []\n", ""), "`fixed` is missing";
    malformed_fixed_null: "fixed null", |c| replace_once(c, "fixed: []\n", "fixed:\n"), "`fixed` is null";
    malformed_fixed_whole_field_type: "fixed whole-field type", |c| replace_once(c, "fixed: []\n", "fixed: F1\n"), "`fixed` is not a list";
    malformed_fixed_one_element: "fixed one element", |c| replace_once(c, "fixed: []\n", "fixed: [F1, 3]\n"), "holds an entry that is not one of the finding ids";
    malformed_fixed_every_element: "fixed every element", |c| replace_once(c, "fixed: []\n", "fixed: [3]\n"), "holds an entry that is not one of the finding ids";
    malformed_fixed_repeated_id: "fixed repeated id", |c| replace_once(c, "fixed: []\n", "fixed: [F1, F1]\n"), "or holds one twice";
    malformed_fixed_duplicate_key: "fixed duplicate key", |c| replace_once(c, "fixed: []\n", "fixed: []\nfixed: [F1]\n"), "its frontmatter could not be read";
    malformed_fixed_invalid_yaml: "fixed invalid YAML", |c| replace_once(c, "fixed: []\n", "fixed: [F1\n"), "its frontmatter could not be read";
    malformed_status_absent: "status absent", |c| replace_once(c, "status: finalized-spec\n", ""), "`status` is missing";
    malformed_status_null: "status null", |c| replace_once(c, "status: finalized-spec\n", "status:\n"), "`status` is null";
    malformed_status_whole_field_type: "status whole-field type", |c| replace_once(c, "status: finalized-spec\n", "status: 3\n"), "`status` is not a non-empty string";
    malformed_status_empty: "status empty", |c| replace_once(c, "status: finalized-spec\n", "status: \"\"\n"), "`status` is not a non-empty string";
    malformed_status_duplicate_key: "status duplicate key", |c| replace_once(c, "status: finalized-spec\n", "status: finalized-spec\nstatus: completed\n"), "its frontmatter could not be read";
    // An opening fence that never closes leaves no frontmatter at all.
    malformed_unterminated_frontmatter: "unterminated frontmatter", |c| c.replacen("\n---\n", "\n", 1), "`status` is missing";
}

/// R12: the guide's `{{{ … }}}` literals reach the agent as `{{ … }}` without
/// the `as_of` span it once carried only to force an interpolation pass.
#[test]
fn the_guide_needs_no_artificial_span_for_its_literals_to_convert() {
    assert!(!GUIDE.contains("as_of"), "the `as_of` workaround is gone from the guide source");
    assert!(GUIDE.contains("{{{ expr }}}"), "the guide still authors a literal, so the check below means something");

    let composed = with_spec("guide-literals", ACTIVE, &control_spec());
    assert!(!composed.body.contains("{{{"), "no literal reaches the agent unconverted:\n{}", composed.body);
    assert!(!composed.body.contains("}}}"), "{}", composed.body);
    assert!(composed.body.contains("**Interpolation.** {{ expr }} is evaluated"), "{}", composed.body);
}
