//! The type-narrowing conformance matrix (`2026-10-16-type-narrowing`,
//! acceptance 4, 9, and 13): every site that reads the shared type scope,
//! crossed with every guard case, has one explicit state.
//!
//! - **Run here.** A Darkmatter-owned site. The cell runs below through the
//!   public preflight, compose, and directive-target APIs and asserts the
//!   public outcome: a guard-proven missing value is a guarded,
//!   template-required hole, an unproven one is `MissingValue` at its line, a
//!   transclusion target is narrowed or not, and a false block skips its site.
//! - **Claudine.** A host site. Its cells are tested where the host action
//!   model lives, named in the cell.
//! - **Inapplicable.** No guard of that kind can reach the site, with why.
//! - **Planned.** A projection the scope does not make yet. Nothing claims it.
//!
//! The guard rows are shared with DMLS (`tests/common/type_scope_cases.rs`).
//! Site cells prove wiring and public outcomes; the algebra itself is covered
//! row by row in `expression_type_scope.rs`.

#[path = "../common/type_scope_cases.rs"]
mod cases;

use std::path::{Path, PathBuf};

use cases::{BlockCase, GuardCase, frontmatter, rows, wrap};
use darkmatter::markdown::compose::collection_blocks::CollectionKind;
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use darkmatter::markdown::compose::directive_targets::{DirectiveTargetAnalysis, analyze_directive_targets};
use darkmatter::markdown::compose::preflight::ComposePreflightReport;
use darkmatter::markdown::compose::shell_expansion::template::{HoleTiming, ModeRequirement, ShellPreApprovals};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellCommandOrigin, ShellExpansionError,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

/// Declares [`ScopeSite`] and [`ScopeSite::ALL`] from one list, so the list
/// cannot omit a variant.
macro_rules! scope_sites {
    ($($(#[$meta:meta])* $variant:ident),+ $(,)?) => {
        /// A place whose static analysis reads the shared type scope.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        enum ScopeSite {
            $($(#[$meta])* $variant),+
        }

        impl ScopeSite {
            const ALL: &'static [ScopeSite] = &[$(ScopeSite::$variant),+];
        }
    };
}

scope_sites! {
    /// A body `::shell` command.
    BodyShell,
    /// A command inside a body `::shell-block`.
    BodyShellBlock,
    /// A `::file` / `::code` / `::url` target expression.
    TransclusionTarget,
    /// A standalone frontmatter `$( … )` value.
    FrontmatterShell,
    /// A lifecycle `shell:` action under `when`/`then`/`else`.
    LifecycleShell,
    /// A lifecycle action with its own inline `when:`.
    LifecycleInlineGuard,
    /// A whole-value lifecycle `set:` shell value.
    LifecycleSetShell,
    /// A sequence task's `shell:` under its `when:`.
    SequenceTaskShell,
    /// A sequence task's `with:` values under its `when:`.
    SequenceTaskWith,
    /// A `::shell` command in the body of a `::map`, itself under `::block`
    /// guards.
    MapBodyShell,
    /// A `::shell` command in the body of a `::filter`.
    FilterBodyShell,
    /// A `::file` target expression in the body of a `::map`.
    MapBodyTarget,
    /// A `::file` target expression in the body of a `::filter`.
    FilterBodyTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Run,
    /// The Claudine test module that owns the cell.
    Claudine(&'static str),
    Inapplicable(&'static str),
    Planned(&'static str),
}

const LIFECYCLE_OWNER: &str = "claudine lib `composition/lifecycle/executor/tests/` (`guarded_shell_sites`)";
const SEQUENCE_OWNER: &str = "claudine lib `composition/sequence/preflight/tests.rs` (`task_guards`)";

const NO_BODY_WRITES: &str =
    "a document body performs no write between a `::block` guard and the sites inside it; frontmatter is settled first";
const FRONTMATTER_UNGUARDED: &str = "a standalone frontmatter `$( … )` value never acquires a body or lifecycle guard, \
     and a hole's own expression is evaluated, not narrowed";
const ONE_TASK_GUARD: &str =
    "a task has one `when:`; a `task:` reference's guard takes precedence over the referenced file's instead of nesting";
const NO_TASK_WRITES: &str = "nothing writes between a task's `when:` and its `shell:` / `with:` values";

const NESTED_UNION: &str = "nested-union and discriminant projection: a guard on `x.kind` narrows only `x.kind`";
const FUNCTION_RESULT: &str = "function-result narrowing: a guard on `f(x)` proves nothing about `x`";
const HOST_TYPES: &str = "typed projections of the host globals `seq`, `loop`, `group`";

/// The planned state shared by every site a guard can reach.
fn planned(case: GuardCase) -> Option<State> {
    match case {
        GuardCase::NestedUnionProjection => Some(State::Planned(NESTED_UNION)),
        GuardCase::FunctionResultNarrowing => Some(State::Planned(FUNCTION_RESULT)),
        GuardCase::HostGlobalTypes => Some(State::Planned(HOST_TYPES)),
        _ => None,
    }
}

/// The matrix. Both matches are exhaustive, so a new site or case does not
/// compile until it is given a state here.
fn state(site: ScopeSite, case: GuardCase) -> State {
    use GuardCase as C;
    match site {
        ScopeSite::BodyShell
        | ScopeSite::BodyShellBlock
        | ScopeSite::TransclusionTarget
        | ScopeSite::MapBodyShell
        | ScopeSite::FilterBodyShell
        | ScopeSite::MapBodyTarget
        | ScopeSite::FilterBodyTarget => match case {
            C::TrueGuard
            | C::NegatedGuard
            | C::NestedGuards
            | C::ConjunctionOperand
            | C::DisjunctionJoin
            | C::FmAlias
            | C::PredicateAlias
            | C::NotNull
            | C::Equality
            | C::OptionalDeclaration
            | C::UnknownValue
            | C::ContradictoryGuard
            | C::SiblingOrParentPath
            | C::OtherValueNotDeferred
            | C::RuntimeSelection => State::Run,
            C::WriteInvalidation => State::Inapplicable(NO_BODY_WRITES),
            C::NestedUnionProjection | C::FunctionResultNarrowing | C::HostGlobalTypes => {
                planned(case).expect("planned case")
            }
        },
        ScopeSite::FrontmatterShell => match case {
            C::OptionalDeclaration | C::UnknownValue => State::Run,
            C::TrueGuard
            | C::NegatedGuard
            | C::NestedGuards
            | C::ConjunctionOperand
            | C::DisjunctionJoin
            | C::FmAlias
            | C::PredicateAlias
            | C::NotNull
            | C::Equality
            | C::ContradictoryGuard
            | C::SiblingOrParentPath
            | C::OtherValueNotDeferred
            | C::WriteInvalidation
            | C::RuntimeSelection
            | C::NestedUnionProjection
            | C::FunctionResultNarrowing
            | C::HostGlobalTypes => State::Inapplicable(FRONTMATTER_UNGUARDED),
        },
        ScopeSite::LifecycleShell | ScopeSite::LifecycleInlineGuard | ScopeSite::LifecycleSetShell => match case {
            C::TrueGuard
            | C::NegatedGuard
            | C::NestedGuards
            | C::ConjunctionOperand
            | C::DisjunctionJoin
            | C::FmAlias
            | C::PredicateAlias
            | C::NotNull
            | C::Equality
            | C::OptionalDeclaration
            | C::UnknownValue
            | C::ContradictoryGuard
            | C::SiblingOrParentPath
            | C::OtherValueNotDeferred
            | C::WriteInvalidation
            | C::RuntimeSelection => State::Claudine(LIFECYCLE_OWNER),
            C::NestedUnionProjection | C::FunctionResultNarrowing | C::HostGlobalTypes => {
                planned(case).expect("planned case")
            }
        },
        ScopeSite::SequenceTaskShell | ScopeSite::SequenceTaskWith => match case {
            C::TrueGuard
            | C::NegatedGuard
            | C::ConjunctionOperand
            | C::DisjunctionJoin
            | C::FmAlias
            | C::PredicateAlias
            | C::NotNull
            | C::Equality
            | C::OptionalDeclaration
            | C::UnknownValue
            | C::ContradictoryGuard
            | C::SiblingOrParentPath
            | C::OtherValueNotDeferred
            | C::RuntimeSelection => State::Claudine(SEQUENCE_OWNER),
            C::NestedGuards => State::Inapplicable(ONE_TASK_GUARD),
            C::WriteInvalidation => State::Inapplicable(NO_TASK_WRITES),
            C::NestedUnionProjection | C::FunctionResultNarrowing | C::HostGlobalTypes => {
                planned(case).expect("planned case")
            }
        },
    }
}

/// Generates one `#[test]` per run-here cell and lists the generated cells in
/// `RUN_HERE`, which the completeness test compares with the matrix.
macro_rules! run_here {
    ($($module:ident: $site:ident { $($test:ident: $case:ident),+ $(,)? })+) => {
        const RUN_HERE: &[(ScopeSite, GuardCase)] = &[$($((ScopeSite::$site, GuardCase::$case)),+),+];
        $(
            mod $module {
                use super::*;
                $(
                    #[test]
                    fn $test() {
                        run(ScopeSite::$site, GuardCase::$case);
                    }
                )+
            }
        )+
    };
}

run_here! {
    body_shell: BodyShell {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    body_shell_block: BodyShellBlock {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    transclusion_target: TransclusionTarget {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    map_body_shell: MapBodyShell {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    filter_body_shell: FilterBodyShell {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    map_body_target: MapBodyTarget {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    filter_body_target: FilterBodyTarget {
        true_guard: TrueGuard,
        negated_guard: NegatedGuard,
        nested_guards: NestedGuards,
        conjunction_operand: ConjunctionOperand,
        disjunction_join: DisjunctionJoin,
        fm_alias: FmAlias,
        predicate_alias: PredicateAlias,
        not_null: NotNull,
        equality: Equality,
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
        contradictory_guard: ContradictoryGuard,
        sibling_or_parent_path: SiblingOrParentPath,
        other_value_not_deferred: OtherValueNotDeferred,
        runtime_selection: RuntimeSelection,
    }
    frontmatter_shell: FrontmatterShell {
        optional_declaration: OptionalDeclaration,
        unknown_value: UnknownValue,
    }
}

// ── the matrix itself ───────────────────────────────────────────────────

/// Every site and every case is in the matrix, every cell's state is
/// explicit and reasoned, and each state sits where it can: run-here cells
/// only at Darkmatter sites, Claudine cells only at host sites.
#[test]
fn every_site_and_case_has_one_explicit_state() {
    assert_eq!(ScopeSite::ALL.len(), 13, "the matrix names all thirteen sites");
    let darkmatter_sites = [
        ScopeSite::BodyShell,
        ScopeSite::BodyShellBlock,
        ScopeSite::TransclusionTarget,
        ScopeSite::FrontmatterShell,
        ScopeSite::MapBodyShell,
        ScopeSite::FilterBodyShell,
        ScopeSite::MapBodyTarget,
        ScopeSite::FilterBodyTarget,
    ];
    for &site in ScopeSite::ALL {
        let mut live = 0;
        for &case in GuardCase::ALL {
            match state(site, case) {
                State::Run => {
                    assert!(darkmatter_sites.contains(&site), "{site:?} × {case:?}: a host site cannot run here");
                    live += 1;
                }
                State::Claudine(owner) => {
                    assert!(!darkmatter_sites.contains(&site), "{site:?} × {case:?}: a Darkmatter site owns its cells");
                    assert!(owner.starts_with("claudine lib `composition/"), "{site:?} × {case:?}: {owner}");
                    live += 1;
                }
                State::Inapplicable(reason) | State::Planned(reason) => {
                    assert!(!reason.trim().is_empty(), "{site:?} × {case:?} needs a reason");
                }
            }
        }
        assert!(live > 0, "{site:?} has no applicable cell, so nothing proves its wiring");
    }
    // A frontmatter value never acquires a body or lifecycle guard.
    for &case in GuardCase::ALL {
        if !matches!(case, GuardCase::OptionalDeclaration | GuardCase::UnknownValue) {
            assert_eq!(state(ScopeSite::FrontmatterShell, case), State::Inapplicable(FRONTMATTER_UNGUARDED), "{case:?}");
        }
    }
    // Future projections are planned at every site a guard reaches, never run.
    for case in [
        GuardCase::NestedUnionProjection,
        GuardCase::FunctionResultNarrowing,
        GuardCase::HostGlobalTypes,
    ] {
        for &site in ScopeSite::ALL {
            if site != ScopeSite::FrontmatterShell {
                assert!(matches!(state(site, case), State::Planned(_)), "{site:?} × {case:?}");
            }
        }
    }
}

/// The generated tests are exactly the run-here cells, so a cell cannot be
/// marked run here without a test, nor a test outlive its cell.
#[test]
fn every_run_here_cell_has_exactly_one_generated_test() {
    let mut expected: Vec<(ScopeSite, GuardCase)> = ScopeSite::ALL
        .iter()
        .flat_map(|&site| GuardCase::ALL.iter().map(move |&case| (site, case)))
        .filter(|&(site, case)| state(site, case) == State::Run)
        .collect();
    let mut generated = RUN_HERE.to_vec();
    expected.sort();
    generated.sort();
    let unique = generated.len();
    generated.dedup();
    assert_eq!(generated.len(), unique, "a cell is generated twice");
    assert_eq!(generated, expected);
    // A body-guard cell at a site that reads body guards has rows to run.
    for &(site, case) in RUN_HERE {
        if site != ScopeSite::FrontmatterShell && case != GuardCase::RuntimeSelection {
            assert!(rows(case).next().is_some(), "{site:?} × {case:?} has no guard rows");
        }
    }
}

// ── running a cell ──────────────────────────────────────────────────────

fn run(site: ScopeSite, case: GuardCase) {
    assert_eq!(state(site, case), State::Run, "{site:?} × {case:?} is not a run-here cell");
    match (site, case) {
        (ScopeSite::FrontmatterShell, case) => frontmatter_value_stays_unguarded(case),
        (site, GuardCase::RuntimeSelection) => runtime_selection(site),
        (ScopeSite::TransclusionTarget, case) => rows(case).for_each(transclusion_row),
        (site @ (ScopeSite::MapBodyTarget | ScopeSite::FilterBodyTarget), case) => {
            rows(case).for_each(|row| collection_target_row(site, row))
        }
        (site, case) => rows(case).for_each(|row| shell_row(site, row)),
    }
}

fn options_for(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
        .with_shell_timeout(std::time::Duration::from_secs(60))
}

/// A document written into its own temp dir; every request is anchored there.
struct Doc {
    dir: TempDir,
    path: PathBuf,
}

impl Doc {
    fn new(content: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, content).unwrap();
        Self { dir, path }
    }

    fn markdown(&self) -> Markdown {
        Markdown::try_from(self.path.as_path()).unwrap()
    }

    fn options(&self) -> ComposeOptions {
        options_for(self.dir.path(), &self.path)
    }

    fn preflight(&self) -> Result<ComposePreflightReport, MarkdownError> {
        self.markdown().compose_preflight(&crate::request_support::request(self.options()))
    }

    /// Approves exactly the shapes discovery reported and composes under them.
    fn compose_approving_discovered_shapes(&self) -> Result<String, MarkdownError> {
        let markdown = self.markdown();
        let report = markdown
            .compose_preflight(&crate::request_support::request(self.options()))
            .expect("discovery passes");
        let mut approvals = ShellPreApprovals::new();
        for candidate in &report.candidates {
            approvals.insert_template(candidate.shape().expect("every site is a template"));
        }
        let options = self.options().with_pre_approved_commands(approvals);
        markdown
            .compose_with(&crate::request_support::request(options))
            .map(|(composed, _)| composed.content().to_string())
    }
}

/// The 1-indexed line of the first line of `text` that starts with `prefix`.
fn line_of(text: &str, prefix: &str) -> usize {
    text.lines()
        .position(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line starting `{prefix}` in\n{text}"))
        + 1
}

fn missing_value(error: &MarkdownError) -> Option<(&str, &ShellCommandOrigin)> {
    match error {
        MarkdownError::ShellExpansion(inner) => match &**inner {
            ShellExpansionError::MissingValue { expression, origin, .. } => Some((expression.as_str(), origin)),
            _ => None,
        },
        _ => None,
    }
}

/// The command line a body shell origin points at.
fn origin_line(origin: &ShellCommandOrigin) -> usize {
    match origin {
        ShellCommandOrigin::Body { line } => *line,
        ShellCommandOrigin::ShellBlock { command_line, .. } => *command_line,
        ShellCommandOrigin::Frontmatter { .. } => panic!("a body site has a body origin: {origin:?}"),
    }
}

/// The directive kind of a collection site.
fn collection_kind(site: ScopeSite) -> Option<CollectionKind> {
    match site {
        ScopeSite::MapBodyShell | ScopeSite::MapBodyTarget => Some(CollectionKind::Map),
        ScopeSite::FilterBodyShell | ScopeSite::FilterBodyTarget => Some(CollectionKind::Filter),
        _ => None,
    }
}

/// `body` repeated once by a collection region over the declared `data`.
fn collection_region(kind: CollectionKind, body: &str) -> String {
    format!("::{} data\n{body}::{}\n", kind.opener(), kind.closer())
}

/// The cell's frontmatter: the shared declarations, plus the one-item `data`
/// array a collection site iterates.
fn site_frontmatter(site: ScopeSite) -> String {
    match collection_kind(site) {
        Some(_) => frontmatter("  data: any[](required)\ndata: [1]\n"),
        None => frontmatter(""),
    }
}

/// One guard row at a body `::shell` or `::shell-block` site: a proven read
/// is one guarded, template-required hole labelled by an enclosing block; an
/// unproven read is `MissingValue` for that read at the command's line.
fn shell_row(site: ScopeSite, row: &BlockCase) {
    let command = format!("git --version {{{{ {} }}}}", row.read);
    let inner = match site {
        ScopeSite::BodyShell => format!("::shell {command}\n"),
        ScopeSite::BodyShellBlock => format!("::shell-block\n{command}\n::end-block\n"),
        ScopeSite::MapBodyShell => collection_region(CollectionKind::Map, &format!("::shell {command}\n")),
        ScopeSite::FilterBodyShell => collection_region(CollectionKind::Filter, &format!("::shell {command}\n")),
        other => unreachable!("{other:?} is not a body shell site"),
    };
    let text = format!("{}# Doc\n{}", site_frontmatter(site), wrap(row, &inner));
    let command_line = text
        .lines()
        .position(|line| line.contains(&command))
        .expect("the command is in the document")
        + 1;
    let label = format!("{site:?} {row:?}");

    match Doc::new(&text).preflight() {
        Ok(report) => {
            assert!(row.proven, "{label}: an unproven read passed preflight");
            let [candidate] = report.candidates.as_slice() else {
                panic!("{label}: one site: {:?}", report.candidates);
            };
            assert_eq!(candidate.requirement, ModeRequirement::TemplateRequired, "{label}");
            assert!(candidate.resolved.is_none(), "{label}: a guarded hole has no resolved bytes");
            assert!(report.entries.is_empty(), "{label}");
            assert_eq!(origin_line(&candidate.site.origin), command_line, "{label}");
            if site == ScopeSite::BodyShellBlock {
                assert!(matches!(candidate.site.origin, ShellCommandOrigin::ShellBlock { .. }), "{label}");
            }
            let template = candidate.template.as_ref().expect("valid template");
            let [placeholder] = template.placeholders().collect::<Vec<_>>()[..] else {
                panic!("{label}: one hole");
            };
            let HoleTiming::Guarded { guard, .. } = &placeholder.timing else {
                panic!("{label}: the hole is guarded: {:?}", placeholder.timing);
            };
            // The label names the marker's file line, as the origin does.
            let enclosing = row.guards.iter().map(|guard| {
                let marker = format!("::block when=\"{guard}\"");
                let line = text.lines().position(|line| line == marker).expect("the marker is in the document") + 1;
                format!("{marker} (line {line})")
            });
            assert!(
                enclosing.clone().any(|expected| guard.label() == expected),
                "{label}: {} is not one of the enclosing guards {:?}",
                guard.label(),
                enclosing.collect::<Vec<_>>()
            );
        }
        Err(error) => {
            assert!(!row.proven, "{label}: a proven read failed: {error}");
            let (expression, origin) = missing_value(&error).unwrap_or_else(|| panic!("{label}: {error:?}"));
            assert_eq!(expression, row.read, "{label}");
            assert_eq!(origin_line(origin), command_line, "{label}");
        }
    }
}

/// The directive-target analysis of a document's body, under its schema.
fn analyze(text: &str) -> Vec<DirectiveTargetAnalysis> {
    let dir = TempDir::new().unwrap();
    let markdown = Markdown::from(text);
    let effective = DarkmatterSchemas::new(crate::request_support::context_at(dir.path()))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("effective schema");
    let frontmatter = serde_json::Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .filter(|(key, _)| key.as_str() != "$schema")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    analyze_directive_targets(markdown.content(), Some(&effective), &frontmatter).expect("blocks pair")
}

/// One guard row at a `::file` target: the target is narrowed exactly when
/// the row is proven, at the authored expression's span.
fn transclusion_row(row: &BlockCase) {
    let target = format!("{{{{{}}}}}", row.read);
    let text = format!("{}{}", frontmatter(""), wrap(row, &format!("::file {target}\n")));
    let [analysis] = analyze(&text).try_into().unwrap_or_else(|all: Vec<_>| panic!("{row:?}: one target: {all:?}"));
    let body = Markdown::from(text.as_str()).content().to_string();
    assert_eq!(&body[analysis.expression_span.clone()], target, "{row:?}");
    assert!(analysis.root.is_some(), "{row:?}: a path target has a root");
    assert_eq!(analysis.is_narrowed(), row.proven, "{row:?}: narrowed paths {:?}", analysis.narrowed_paths);
}

/// [`transclusion_row`] for a target inside the body of a collection region
/// that sits under the row's guards: the region adds no scope that hides the
/// guarded read, so the target is narrowed exactly when the row is proven.
fn collection_target_row(site: ScopeSite, row: &BlockCase) {
    let kind = collection_kind(site).expect("a collection site");
    let target = format!("{{{{{}}}}}", row.read);
    let region = collection_region(kind, &format!("::file {target}\n"));
    let text = format!("{}{}", site_frontmatter(site), wrap(row, &region));
    let [analysis] = analyze(&text).try_into().unwrap_or_else(|all: Vec<_>| panic!("{site:?} {row:?}: one target: {all:?}"));
    let body = Markdown::from(text.as_str()).content().to_string();
    assert_eq!(&body[analysis.expression_span.clone()], target, "{site:?} {row:?}");
    assert_eq!(analysis.is_narrowed(), row.proven, "{site:?} {row:?}: narrowed paths {:?}", analysis.narrowed_paths);
}

/// A false block removes its site before it runs; a selected one runs with
/// the live value, approved by nothing but the shape discovery reported.
fn runtime_selection(site: ScopeSite) {
    if let Some(kind) = collection_kind(site) {
        collection_runtime_selection(site, kind);
        return;
    }
    if site == ScopeSite::TransclusionTarget {
        let doc = Doc::new("---\npart: part.md\n---\n::block when=\"part\"\n::file {{part}}\n::end-block\n::block when=\"!part\"\n::file {{part}}\n::end-block\n::block when=\"x\"\n::file {{x}}\n::end-block\nend\n");
        std::fs::write(doc.dir.path().join("part.md"), "PART-BODY\n").unwrap();
        let composed = doc.compose_approving_discovered_shapes().unwrap();
        assert_eq!(composed.matches("PART-BODY").count(), 1, "{composed}");
        assert!(composed.contains("end"), "{composed}");
        return;
    }
    let wrap_command = |command: &str| match site {
        ScopeSite::BodyShell => format!("::shell {command}\n"),
        ScopeSite::BodyShellBlock => format!("::shell-block\n{command}\n::end-block\n"),
        other => unreachable!("{other:?}"),
    };
    let skipped = Doc::new(&format!("# Doc\n::block when=\"x\"\n{}::end-block\nend\n", wrap_command("git --version {{ x }}")));
    let composed = skipped.compose_approving_discovered_shapes().unwrap();
    assert!(!composed.contains("git version") && composed.contains("end"), "{site:?}: {composed}");

    let selected = Doc::new(&format!("---\nflag: --version\n---\n::block when=\"flag\"\n{}::end-block\n", wrap_command("git {{ flag }}")));
    let composed = selected.compose_approving_discovered_shapes().unwrap();
    assert!(composed.contains("git version"), "{site:?}: {composed}");
}

/// Records each shell approval request and denies it, so no command runs.
struct DenyingRecorder(std::sync::Mutex<Vec<String>>);

impl ShellApprovalHandler for DenyingRecorder {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.lock().unwrap().push(request.raw_command);
        Ok(ShellApprovalDecision::Deny)
    }
}

/// The collection counterpart of [`runtime_selection`]: a false block skips
/// the whole region, so nothing asks for approval; a selected one reaches the
/// body command (shell) or the target (file) once per item.
fn collection_runtime_selection(site: ScopeSite, kind: CollectionKind) {
    let shell = matches!(site, ScopeSite::MapBodyShell | ScopeSite::FilterBodyShell);
    let body = |read: &str| {
        if shell {
            format!("::shell git {{{{ {read} }}}}\n")
        } else {
            format!("::file {{{{ {read} }}}}\n")
        }
    };
    let run = |text: &str| {
        let recorder = std::sync::Arc::new(DenyingRecorder(std::sync::Mutex::default()));
        let doc = Doc::new(text);
        std::fs::write(doc.dir.path().join("part.md"), "PART-BODY\n").unwrap();
        let options = doc.options().with_shell_approval_handler(recorder.clone());
        let composed = doc.markdown().compose_with(&crate::request_support::request(options)).map(|(m, _)| m.content().to_string());
        let asked = recorder.0.lock().unwrap().clone();
        (composed, asked)
    };
    let skipped = format!(
        "{}::block when=\"x\"\n{}::end-block\nend\n",
        site_frontmatter(site),
        collection_region(kind, &body("x"))
    );
    let (composed, asked) = run(&skipped);
    assert!(composed.unwrap().contains("end") && asked.is_empty(), "{site:?}: a false block runs nothing: {asked:?}");

    let selected = format!(
        "{}::block when=\"flag\"\n{}::end-block\n",
        site_frontmatter(site).replace("---\n$schema", "---\nflag: part.md\n$schema"),
        collection_region(kind, &body("flag"))
    );
    let (composed, asked) = run(&selected);
    if shell {
        assert!(composed.is_err(), "{site:?}: the denied command fails the document");
        assert_eq!(asked.len(), 1, "{site:?}: {asked:?}");
    } else {
        assert!(composed.unwrap().contains("PART-BODY"), "{site:?}");
    }
}

/// A frontmatter `$( … )` value reading a missing value fails with
/// `MissingValue` at its key, even under an optional declaration and even
/// when a body block guarding that value reads the key.
fn frontmatter_value_stays_unguarded(case: GuardCase) {
    let read = match case {
        GuardCase::OptionalDeclaration => "x",
        GuardCase::UnknownValue => "u",
        other => unreachable!("{other:?}"),
    };
    let text = format!(
        "{}::block when=\"{read}\"\n{{{{ listing }}}}\n::end-block\n",
        frontmatter(&format!("listing: $(git --version {{{{ {read} }}}})\n"))
    );
    let error = Doc::new(&text).preflight().expect_err("the frontmatter value is unguarded");
    let (expression, origin) = missing_value(&error).unwrap_or_else(|| panic!("{case:?}: {error:?}"));
    assert_eq!(expression, read, "{case:?}");
    let ShellCommandOrigin::Frontmatter { key, line } = origin else {
        panic!("{case:?}: a frontmatter origin: {origin:?}");
    };
    assert_eq!(key, "listing");
    assert_eq!(*line, Some(line_of(&text, "listing:")), "{case:?}");
}

// ── passive analysis and planned projections ────────────────────────────

/// Acceptance 13 at the body shell and transclusion sites: a `file_exists`
/// guard is analyzed, never run. Evaluating this guard would fail on its
/// unreachable URL, so analysis passing proves no handler ran; no engine is
/// built, no network is attempted, and the directory is unchanged.
#[test]
fn file_exists_guards_are_analyzed_without_running_a_handler() {
    let guard = "file_exists(remote) && file_exists(n) && file_exists(x)";
    let tripwire = TempDir::new().unwrap();
    assert!(
        evaluate_condition_against(
            "file_exists(remote)",
            &serde_json::json!({ "remote": "https://127.0.0.1:9/never-fetched.md" }),
            &biscuit_file::FileResolutionContext::new(tripwire.path())
        )
        .is_err(),
        "the tripwire is armed: running the guard fails"
    );

    let text = format!(
        "{}::block when=\"{guard}\"\n::shell git --version {{{{ x }}}}\n::file {{{{x}}}}\n::end-block\n",
        frontmatter("n: 5\nremote: https://127.0.0.1:9/never-fetched.md\n")
    );
    let doc = Doc::new(&text);
    let listing = || {
        let mut names: Vec<_> = std::fs::read_dir(doc.dir.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
        names.sort();
        names
    };
    let files_before = listing();
    #[cfg(feature = "effects-instrumentation")]
    let effects_before = (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count());

    let report = doc.preflight().expect("the guard is analyzed, not run");
    let [analysis] = analyze(&text).try_into().unwrap_or_else(|all: Vec<_>| panic!("one target: {all:?}"));

    assert!(matches!(
        report.candidates[0].template.as_ref().unwrap().placeholders().next().unwrap().timing,
        HoleTiming::Guarded { .. }
    ));
    assert!(analysis.is_narrowed());
    assert_eq!(listing(), files_before, "analysis created or removed files");
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(
        (darkmatter::effects::engine_build_count(), darkmatter::effects::network_attempt_count()),
        effects_before,
        "analysis built an effect engine or attempted network access"
    );
}

/// The planned cells are not claimed: today a function result proves nothing
/// about its argument, and a discriminant guard does not prove its parent.
/// When one of these projections lands, this fails until its matrix cells are
/// updated.
#[test]
fn planned_projections_stay_unclaimed_at_body_shell() {
    for (case, guard, read) in [
        (GuardCase::FunctionResultNarrowing, "is_string(trim(x))", "x"),
        (GuardCase::NestedUnionProjection, "obj.kind == 'a'", "obj"),
    ] {
        assert!(matches!(state(ScopeSite::BodyShell, case), State::Planned(_)));
        let text = format!("# Doc\n::block when=\"{guard}\"\n::shell git --version {{{{ {read} }}}}\n::end-block\n");
        let error = Doc::new(&text).preflight().expect_err(guard);
        assert_eq!(missing_value(&error).map(|(expression, _)| expression), Some(read), "{case:?}: {error:?}");
    }
}

/// `file_exists` inspects its argument without converting it, so only text
/// can make it true: its true side narrows a mixed declaration to `file`.
#[test]
fn file_exists_true_side_narrows_a_mixed_declaration_to_file() {
    let text = format!(
        "{}::block when=\"file_exists(target)\"\n::shell git --version {{{{ target }}}}\n::end-block\n",
        frontmatter("")
    );
    let report = Doc::new(&text).preflight().unwrap();
    assert_eq!(report.candidates[0].shape().unwrap().canonical_text(), "git --version {{ file }}");
}
