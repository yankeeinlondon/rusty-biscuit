//! The type-scope conformance matrix: every site a guard can narrow, crossed
//! with every guard case, each cell run here, proven by a named live test,
//! owned by Darkmatter, inapplicable for a stated reason, or planned.
//!
//! [`ScopeSite`] and [`GuardCase`] are exhaustive, so a new site or case does
//! not compile until [`cell`] states what holds there. The lifecycle cells
//! run below through pre-flight and event execution; the sequence cells run
//! in `sequence/preflight/tests.rs`'s `scope_site_conformance` module. Both
//! are named `{site}_{case}` so the table needs no second list of names.

use super::guarded_shell_sites::{args_of, guarded_holes, missing, resolve, run, shape_of, shell_sites};
use super::*;
use crate::composition::preflight::settle_lifecycle_shell_modes;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;

/// An enum whose `ALL` lists every variant by construction, each with the
/// snake-case name its matrix tests start with.
macro_rules! listed_enum {
    ($(#[$meta:meta])* enum $name:ident { $($variant:ident => $snake:literal,)* }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum $name { $($variant,)* }

        impl $name {
            const ALL: &[Self] = &[$(Self::$variant,)*];

            const fn snake(self) -> &'static str {
                match self { $(Self::$variant => $snake,)* }
            }
        }
    };
}

listed_enum! {
    /// Every site a type scope types today.
    enum ScopeSite {
        BodyShell => "body_shell",
        BodyShellBlock => "body_shell_block",
        TransclusionTarget => "transclusion_target",
        FrontmatterShell => "frontmatter_shell",
        LifecycleShell => "lifecycle_shell",
        LifecycleInlineGuard => "lifecycle_inline_guard",
        LifecycleSetShell => "lifecycle_set_shell",
        SequenceTaskShell => "sequence_task_shell",
        SequenceTaskWith => "sequence_task_with",
    }
}

listed_enum! {
    /// Every guard behavior the matrix checks at each site.
    enum GuardCase {
        TrueGuard => "true_guard",
        NegatedGuard => "negated_guard",
        ElseFalseSide => "else_false_side",
        NestedFiveLevels => "nested_five_levels",
        AndOperands => "and_operands",
        OrJoin => "or_join",
        BareAndFmAlias => "bare_and_fm_alias",
        IsStringAlias => "is_string_alias",
        FileExistsAlias => "file_exists_alias",
        NotNull => "not_null",
        Equality => "equality",
        UnknownValue => "unknown_value",
        ContradictoryGuard => "contradictory_guard",
        SetBeforeShell => "set_before_shell",
        NestedWriteAfterJoin => "nested_write_after_join",
        GuardDoesNotDeferOther => "guard_does_not_defer_other",
        SiblingPath => "sibling_path",
        FactsStayInTheirOwner => "facts_stay_in_their_owner",
        HostGlobalTypes => "host_global_types",
        NestedUnionProjection => "nested_union_projection",
        FunctionResultNarrowing => "function_result_narrowing",
    }
}

/// A test file whose `#[test]` functions the table names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestFile {
    ThisMatrix,
    SequenceMatrix,
    GuardedShellSites,
    SequenceTaskGuards,
}

impl TestFile {
    fn source(self) -> &'static str {
        match self {
            Self::ThisMatrix => include_str!("scope_site_conformance.rs"),
            Self::GuardedShellSites => include_str!("guarded_shell_sites.rs"),
            Self::SequenceMatrix | Self::SequenceTaskGuards => include_str!("../../../sequence/preflight/tests.rs"),
        }
    }

    /// The module inside [`Self::source`] that holds the test.
    fn module(self) -> Option<&'static str> {
        match self {
            Self::ThisMatrix | Self::GuardedShellSites => None,
            Self::SequenceMatrix => Some("scope_site_conformance"),
            Self::SequenceTaskGuards => Some("task_guards"),
        }
    }
}

/// What holds for one (site, case) pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    /// Run by this matrix as the test `{site}_{case}` in the site's matrix file.
    Run,
    /// Already proven by a live Phase 4 test, which the matrix names instead of
    /// repeating.
    Proven(TestFile, &'static str),
    /// A document site, whose matrix Darkmatter owns.
    Darkmatter,
    Inapplicable(&'static str),
    /// Needs a projection or consumer that has not landed; never passed vacuously.
    Planned(&'static str),
}

const DARKMATTER_MATRIX: &str = "darkmatter/lib/tests/l1/scope_site_conformance.rs";

const NO_LIFECYCLE_ELSE_INLINE: &str = "the inline form `{ when, <action> }` has no `else`; its false side runs nothing";
const NO_TASK_ELSE: &str = "a task's `when:` has no `else`: a false guard skips the task";
const ONE_TASK_GUARD: &str =
    "a task carries one `when:`, and a `task:` reference's guard replaces the referenced file's rather than nesting";
const NO_TASK_WRITE: &str =
    "no action runs between a task's `when:` and its `shell:`/`with:`; both read the task-start values";
const HOST_TYPES: &str = "host types for `seq`/`loop`/`group`: a guard proves a host value present but projects no type";
const NESTED_UNION: &str = "nested-union and discriminant projection: a guard on `x.kind` narrows only `x.kind`";
const FUNCTION_RESULT: &str = "function-result narrowing: a guard on `len(x)` or `upper(x)` proves nothing about `x`";

/// The matrix. Lifecycle and sequence sites are Claudine's; document sites
/// are Darkmatter's [`DARKMATTER_MATRIX`]. The match on `site` is exhaustive,
/// so a new site needs its own rows; a new case at a Claudine site defaults
/// to `Run` and fails [`every_claudine_cell_names_a_live_test_and_every_other_cell_says_why`]
/// until its test exists or its row says otherwise.
fn cell(site: ScopeSite, case: GuardCase) -> Cell {
    use Cell::{Inapplicable, Planned, Proven, Run};
    use GuardCase as C;
    use TestFile::{GuardedShellSites as Lifecycle, SequenceTaskGuards as Tasks};
    let planned = match case {
        C::HostGlobalTypes => Some(HOST_TYPES),
        C::NestedUnionProjection => Some(NESTED_UNION),
        C::FunctionResultNarrowing => Some(FUNCTION_RESULT),
        _ => None,
    };
    let claudine = |rows: fn(GuardCase) -> Cell| planned.map_or_else(|| rows(case), Planned);
    match site {
        ScopeSite::BodyShell | ScopeSite::BodyShellBlock | ScopeSite::TransclusionTarget | ScopeSite::FrontmatterShell => {
            Cell::Darkmatter
        }
        ScopeSite::LifecycleShell => claudine(|case| match case {
            C::TrueGuard => Proven(Lifecycle, "a_guard_proven_missing_value_waits_for_its_template_and_runs_only_when_selected"),
            C::ElseFalseSide => Proven(Lifecycle, "an_else_body_learns_the_false_side_only"),
            C::NestedFiveLevels => Proven(Lifecycle, "nested_guards_compose_to_five_levels"),
            C::SetBeforeShell => Proven(Lifecycle, "a_set_before_the_shell_ends_the_guard_proof"),
            C::NestedWriteAfterJoin => Proven(Lifecycle, "a_write_inside_an_inner_branch_invalidates_after_the_join"),
            C::GuardDoesNotDeferOther => Proven(Lifecycle, "a_guard_on_one_value_does_not_defer_another"),
            C::FactsStayInTheirOwner => Proven(Lifecycle, "a_guard_in_one_event_proves_nothing_in_another"),
            _ => Run,
        }),
        ScopeSite::LifecycleInlineGuard => claudine(|case| match case {
            C::TrueGuard => Proven(Lifecycle, "an_inline_guard_proves_its_sibling_action_like_an_explicit_then"),
            C::ElseFalseSide => Inapplicable(NO_LIFECYCLE_ELSE_INLINE),
            _ => Run,
        }),
        ScopeSite::LifecycleSetShell => claudine(|case| match case {
            C::TrueGuard => Proven(Lifecycle, "a_set_value_is_checked_before_its_own_write"),
            C::UnknownValue => Proven(Lifecycle, "a_guarded_set_value_checks_its_live_value_when_reached"),
            _ => Run,
        }),
        ScopeSite::SequenceTaskShell => claudine(|case| match case {
            C::TrueGuard => Proven(Tasks, "a_guard_proven_missing_value_waits_for_the_tasks_template"),
            C::GuardDoesNotDeferOther => Proven(Tasks, "a_guard_on_one_value_does_not_defer_another"),
            C::ElseFalseSide => Inapplicable(NO_TASK_ELSE),
            C::NestedFiveLevels => Inapplicable(ONE_TASK_GUARD),
            C::SetBeforeShell | C::NestedWriteAfterJoin => Inapplicable(NO_TASK_WRITE),
            _ => Run,
        }),
        ScopeSite::SequenceTaskWith => claudine(|case| match case {
            C::TrueGuard | C::GuardDoesNotDeferOther => {
                Proven(Tasks, "a_guarded_with_value_that_is_missing_now_is_late_for_the_document")
            }
            C::ElseFalseSide => Inapplicable(NO_TASK_ELSE),
            C::NestedFiveLevels => Inapplicable(ONE_TASK_GUARD),
            C::SetBeforeShell | C::NestedWriteAfterJoin => Inapplicable(NO_TASK_WRITE),
            _ => Run,
        }),
    }
}

/// The file and module whose `{site}_{case}` test runs a [`Cell::Run`] cell.
fn matrix_file(site: ScopeSite) -> TestFile {
    match site {
        ScopeSite::SequenceTaskShell | ScopeSite::SequenceTaskWith => TestFile::SequenceMatrix,
        _ => TestFile::ThisMatrix,
    }
}

/// The body of the inline module `module` in `source`, or all of `source`.
fn module_body(source: &'static str, module: Option<&str>) -> &'static str {
    let Some(module) = module else { return source };
    let start = source.find(&format!("mod {module} {{")).unwrap_or_else(|| panic!("no `mod {module}`"));
    let rest = &source[start..];
    let end = rest.find("\n}\n").map_or(rest.len(), |end| end + 3);
    &rest[..end]
}

/// Whether `file` declares `#[test] fn name()`.
fn declares_test(file: TestFile, name: &str) -> bool {
    let body = module_body(file.source(), file.module());
    let signature = format!("fn {name}()");
    body.match_indices(&signature).any(|(at, _)| body[..at].trim_end().ends_with("#[test]"))
}

#[test]
fn the_matrix_has_every_site_and_case() {
    assert_eq!(ScopeSite::ALL.len(), 9);
    assert_eq!(GuardCase::ALL.len(), 21);
    let cells = ScopeSite::ALL.iter().flat_map(|site| GuardCase::ALL.iter().map(move |case| cell(*site, *case))).count();
    assert_eq!(cells, 9 * 21);
}

#[test]
fn every_claudine_cell_names_a_live_test_and_every_other_cell_says_why() {
    let mut failures = Vec::new();
    for &site in ScopeSite::ALL {
        let document = matches!(
            site,
            ScopeSite::BodyShell | ScopeSite::BodyShellBlock | ScopeSite::TransclusionTarget | ScopeSite::FrontmatterShell
        );
        for &case in GuardCase::ALL {
            match cell(site, case) {
                Cell::Run => {
                    let name = format!("{}_{}", site.snake(), case.snake());
                    if !declares_test(matrix_file(site), &name) {
                        failures.push(format!("{site:?} × {case:?}: no `#[test] fn {name}()` in {:?}", matrix_file(site)));
                    }
                }
                Cell::Proven(file, name) if !declares_test(file, name) => {
                    failures.push(format!("{site:?} × {case:?}: {file:?} has no `#[test] fn {name}()`"));
                }
                Cell::Proven(..) => {}
                Cell::Darkmatter if !document => failures.push(format!("{site:?} × {case:?}: Claudine owns this site")),
                Cell::Darkmatter => {}
                Cell::Inapplicable(reason) | Cell::Planned(reason) if reason.trim().is_empty() => {
                    failures.push(format!("{site:?} × {case:?}: no reason"));
                }
                Cell::Inapplicable(_) | Cell::Planned(_) => {}
            }
            if document && cell(site, case) != Cell::Darkmatter {
                failures.push(format!("{site:?} × {case:?}: {DARKMATTER_MATRIX} owns document sites"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// A `{site}_{case}` test the table does not reach would pass without proving
/// any cell.
#[test]
fn every_matrix_test_is_a_cell_of_the_table() {
    let cells: Vec<String> = ScopeSite::ALL
        .iter()
        .flat_map(|site| GuardCase::ALL.iter().map(move |case| (*site, *case)))
        .filter(|(site, case)| cell(*site, *case) == Cell::Run)
        .map(|(site, case)| format!("{}_{}", site.snake(), case.snake()))
        .collect();
    for file in [TestFile::ThisMatrix, TestFile::SequenceMatrix] {
        let body = module_body(file.source(), file.module());
        for (at, _) in body.match_indices("#[test]\nfn ").chain(body.match_indices("#[test]\n    fn ")) {
            let rest = body[at..].split_once("fn ").expect("a test function").1;
            let name = &rest[..rest.find('(').expect("a parameter list")];
            let is_cell = ScopeSite::ALL.iter().any(|site| name.starts_with(site.snake()));
            assert!(!is_cell || cells.iter().any(|cell| cell == name), "{file:?} `{name}` is no `Run` cell of the table");
        }
    }
}

// -- lifecycle cells -------------------------------------------------------

/// `command` as the action a site holds, guarded by `guard` when there is one:
/// a `then` body, the inline form, or a whole-value `set` `$( … )`.
fn site_action(site: ScopeSite, guard: Option<&str>, command: &str) -> Value {
    let action = match site {
        ScopeSite::LifecycleSetShell => json!({ "set": { "out": format!("$({command})") } }),
        _ => json!({ "shell": command }),
    };
    match (site, guard) {
        (_, None) => action,
        (ScopeSite::LifecycleInlineGuard, Some(guard)) => {
            let mut inline = action;
            inline["when"] = json!(guard);
            inline
        }
        (_, Some(guard)) => json!({ "when": guard, "then": [action] }),
    }
}

/// The action a site holds inside an enclosing body: the inline form carries
/// its own guard on an unrelated value, so only the enclosing guard can prove
/// what the command reads.
fn inner_action(site: ScopeSite, command: &str) -> Value {
    let guard = (site == ScopeSite::LifecycleInlineGuard).then_some("ready");
    site_action(site, guard, command)
}

fn start(actions: Value) -> Value {
    json!({ "start": actions })
}

/// Pre-flight accepts the site and marks exactly `expressions` as guarded.
fn assert_guarded(frontmatter: &Value, expressions: &[&str]) {
    let config = resolve(frontmatter).unwrap_or_else(|error| panic!("pre-flight refused {frontmatter}: {error}"));
    let [site] = shell_sites(&config, LifecycleSignal::Start).try_into().expect("one site");
    let mut holes: Vec<String> = guarded_holes(&site).into_iter().map(|(expression, _)| expression).collect();
    holes.sort();
    assert_eq!(holes, expressions, "{frontmatter}");
    assert_eq!(site.resolved(), None, "no bytes stand in for a missing value");
}

/// Pre-flight refuses the site because `expression` is missing.
fn assert_missing(frontmatter: &Value, expression: &str) {
    let message = missing(resolve(frontmatter));
    assert!(message.contains(&format!("`{expression}`")), "{frontmatter}: {message}");
}

/// Approved by the shape pre-flight discovered for it, which its guard has
/// narrowed, `selecting` runs `command` once with `args`; `skipping`
/// launches nothing.
fn assert_runs_only_when_selected(frontmatter: &Value, selecting: Value, args: &[&str], skipping: Value) {
    let config = resolve(frontmatter).expect("pre-flight accepts the guarded site");
    let [site] = shell_sites(&config, LifecycleSignal::Start).try_into().expect("one site");
    let mut discovered = ShellPreApprovals::new();
    discovered.insert_template(TemplateShape::from_template(site.template()));
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &discovered, None);

    let selected = MockShell::new(0);
    let outcome = run(LifecycleSignal::Start, &config, selecting, &selected);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(args_of(&selected), vec![args.to_vec()]);

    let skipped = MockShell::new(0);
    let outcome = run(LifecycleSignal::Start, &config, skipping, &skipped);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(skipped.chains().is_empty() && skipped.commands().is_empty(), "a false guard launches nothing");
}

fn negated_guard(site: ScopeSite) {
    assert_missing(&start(json!([site_action(site, Some("!msg"), "tool {{ msg }}")])), "msg");
}

fn else_false_side(site: ScopeSite) {
    let conditional = |guard: &str| json!([{ "when": guard, "then": [{ "say": "x" }], "else": [inner_action(site, "tool {{ msg }}")] }]);
    assert_guarded(&start(conditional("!msg")), &["msg"]);
    assert_missing(&start(conditional("msg")), "msg");
}

fn nested_five_levels(site: ScopeSite) {
    // The innermost guard is the site's own when the site is the inline form.
    let (innermost, enclosing): (Value, &[&str]) = match site {
        ScopeSite::LifecycleInlineGuard => (json!([site_action(site, Some("e"), "tool {{ a }} {{ e }}")]), &["d", "c", "b", "a"]),
        _ => (json!([site_action(site, None, "tool {{ a }} {{ e }}")]), &["e", "d", "c", "b", "a"]),
    };
    let nested = enclosing.iter().fold(innermost, |body, guard| json!([{ "when": guard, "then": body }]));
    assert_guarded(&start(nested), &["a", "e"]);
}

fn and_operands(site: ScopeSite) {
    assert_guarded(&start(json!([site_action(site, Some("a && b"), "tool {{ a }} {{ b }}")])), &["a", "b"]);
}

fn or_join(site: ScopeSite) {
    assert_missing(&start(json!([site_action(site, Some("a || b"), "tool {{ a }}")])), "a");
}

fn bare_and_fm_alias(site: ScopeSite) {
    assert_guarded(&start(json!([site_action(site, Some("fm.msg"), "tool {{ msg }}")])), &["msg"]);
    assert_guarded(&start(json!([site_action(site, Some("msg"), "tool {{ fm.msg }}")])), &["fm.msg"]);
}

fn is_string_alias(site: ScopeSite) {
    for guard in ["is_string(msg)", "isString(msg)"] {
        assert_guarded(&start(json!([site_action(site, Some(guard), "tool {{ msg }}")])), &["msg"]);
    }
}

/// The guard is never evaluated, so no file named by the missing value is
/// looked up.
fn file_exists_alias(site: ScopeSite) {
    for guard in ["file_exists(target)", "fileExists(target)"] {
        assert_guarded(&start(json!([site_action(site, Some(guard), "tool {{ target }}")])), &["target"]);
    }
}

fn not_null(site: ScopeSite) {
    assert_guarded(&start(json!([site_action(site, Some("msg != null"), "tool {{ msg }}")])), &["msg"]);
}

fn equality(site: ScopeSite) {
    let frontmatter = start(json!([site_action(site, Some("mode == 'release'"), "tool {{ mode }}")]));
    assert_guarded(&frontmatter, &["mode"]);
    if site != ScopeSite::LifecycleSetShell {
        assert_runs_only_when_selected(
            &frontmatter,
            json!({ "mode": "release" }),
            &["tool", "release"],
            json!({ "mode": "debug" }),
        );
    }
}

/// Truthiness removes only `null` from an undeclared value: the live value is
/// still checked when the action is reached, and a container is refused.
fn unknown_value(site: ScopeSite) {
    let frontmatter = start(json!([site_action(site, Some("msg"), "tool {{ msg }}")]));
    let config = resolve(&frontmatter).expect("a guard proves the value present");
    settle_lifecycle_shell_modes(&config, &LifecycleSignal::ALL, &shape_of("tool {{ msg }}"), None);
    let shell = MockShell::new(0);

    let outcome = run(LifecycleSignal::Start, &config, json!({ "msg": ["SECRET-ITEM"] }), &shell);

    let error = outcome.evaluation_error.expect("a container is no scalar argument");
    assert_eq!(error.variant, "LifecycleShellTemplateRefused", "{error:?}");
    assert!(shell.chains().is_empty() && shell.commands().is_empty(), "nothing launches on refusal");
}

/// A guard no value can satisfy keeps the site checked: it is typed as a
/// template, and it never waives a value the guard does not name.
fn contradictory_guard(site: ScopeSite) {
    let guard = "mode == 'a' && mode == 'b'";
    assert_guarded(&start(json!([site_action(site, Some(guard), "tool {{ mode }}")])), &["mode"]);
    assert_missing(&start(json!([site_action(site, Some(guard), "tool {{ mode }} {{ y }}")])), "y");
}

fn set_before_shell(site: ScopeSite) {
    for write in ["msg", "fm.msg"] {
        let body = json!([{ "set": { write: "replaced" } }, inner_action(site, "tool {{ msg }}")]);
        assert_missing(&start(json!([{ "when": "msg", "then": body }])), "msg");
    }
    let body = json!([{ "set": { "other": "x" } }, inner_action(site, "tool {{ msg }}")]);
    assert_guarded(&start(json!([{ "when": "msg", "then": body }])), &["msg"]);
}

fn nested_write_after_join(site: ScopeSite) {
    let body = json!([
        { "when": "other", "then": [{ "set": { "msg": "replaced" } }] },
        inner_action(site, "tool {{ msg }}"),
    ]);
    assert_missing(&start(json!([{ "when": "msg", "then": body }])), "msg");
}

fn guard_does_not_defer_other(site: ScopeSite) {
    assert_missing(&start(json!([site_action(site, Some("x"), "tool {{ x }} {{ y }}")])), "y");
}

fn sibling_path(site: ScopeSite) {
    assert_guarded(&start(json!([site_action(site, Some("x.a"), "tool {{ x.a }}")])), &["x.a"]);
    assert_missing(&start(json!([site_action(site, Some("x.a"), "tool {{ x.b }}")])), "x.b");
}

fn facts_stay_in_their_owner(site: ScopeSite) {
    assert_missing(
        &json!({
            "start": [{ "when": "msg", "then": [{ "say": "{{ msg }}" }] }],
            "success": [inner_action(site, "tool {{ msg }}")],
        }),
        "msg",
    );
}

#[test]
fn lifecycle_shell_negated_guard() {
    negated_guard(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_and_operands() {
    and_operands(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_or_join() {
    or_join(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_bare_and_fm_alias() {
    bare_and_fm_alias(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_is_string_alias() {
    is_string_alias(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_file_exists_alias() {
    file_exists_alias(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_not_null() {
    not_null(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_equality() {
    equality(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_unknown_value() {
    unknown_value(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_contradictory_guard() {
    contradictory_guard(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_shell_sibling_path() {
    sibling_path(ScopeSite::LifecycleShell);
}

#[test]
fn lifecycle_inline_guard_negated_guard() {
    negated_guard(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_nested_five_levels() {
    nested_five_levels(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_and_operands() {
    and_operands(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_or_join() {
    or_join(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_bare_and_fm_alias() {
    bare_and_fm_alias(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_is_string_alias() {
    is_string_alias(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_file_exists_alias() {
    file_exists_alias(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_not_null() {
    not_null(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_equality() {
    equality(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_unknown_value() {
    unknown_value(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_contradictory_guard() {
    contradictory_guard(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_set_before_shell() {
    set_before_shell(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_nested_write_after_join() {
    nested_write_after_join(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_guard_does_not_defer_other() {
    guard_does_not_defer_other(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_sibling_path() {
    sibling_path(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_inline_guard_facts_stay_in_their_owner() {
    facts_stay_in_their_owner(ScopeSite::LifecycleInlineGuard);
}

#[test]
fn lifecycle_set_shell_negated_guard() {
    negated_guard(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_else_false_side() {
    else_false_side(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_nested_five_levels() {
    nested_five_levels(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_and_operands() {
    and_operands(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_or_join() {
    or_join(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_bare_and_fm_alias() {
    bare_and_fm_alias(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_is_string_alias() {
    is_string_alias(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_file_exists_alias() {
    file_exists_alias(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_not_null() {
    not_null(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_equality() {
    equality(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_contradictory_guard() {
    contradictory_guard(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_set_before_shell() {
    set_before_shell(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_nested_write_after_join() {
    nested_write_after_join(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_guard_does_not_defer_other() {
    guard_does_not_defer_other(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_sibling_path() {
    sibling_path(ScopeSite::LifecycleSetShell);
}

#[test]
fn lifecycle_set_shell_facts_stay_in_their_owner() {
    facts_stay_in_their_owner(ScopeSite::LifecycleSetShell);
}
