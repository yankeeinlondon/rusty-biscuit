---
kind: plan
created: 2026-11-08
phase: 6
total_phases: 7
agent: claude/sonnet
yolo: true
spec: 2026-11-05-member-property-check
area: darkmatter
packages:
- darkmatter
- darkmatter-cli
- darkmatter-lsp
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
- darkmatter/lib/src/markdown/compose/expression/member_check.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
- darkmatter/lib/src/markdown/compose/policy.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/tests/l1/dynamic_schema_slots.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/member_check.rs
- darkmatter/cli/src/args/policy.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- darkmatter/dmls/src/diagnostics/call_check.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/overlay/schema.rs
- darkmatter/dmls/src/providers/collections.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/undeclared_member.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/call_precheck.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/member_check.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
- darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs
- darkmatter/lib/src/markdown/compose/inline/interpolation.rs
- darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/member_sites.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/tests/l1/compose_member_warnings.rs
- darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- darkmatter/lib/tests/l1/compose_member_warnings.rs
- darkmatter/cli/tests/l1/compose_member_warnings.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/dmls/tests/l1/undeclared_member.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
---

# Plan: Member Property Check

Source: the `2026-11-05-member-property-check` spec, finalized 2026-11-08,
including its 2026-11-08 additions (sequencing after
2026-11-05-dynamic-schema-types and 2026-11-05-expression-recovery, and
"Unresolved schema templates"). Every implementation choice is settled in
**Phase 1 → Necessary Rules** (`M1`…`M24`). Later phases cite rules by ID and
never reopen them. The only sanctioned way to change a rule is the Phase 1
rebase check (`R0`): an assumption about an earlier feature that the landed
code contradicts revises the rules it names, and the revision is recorded in
the implementation log before any later phase starts.

This plan replaces the 2026-11-08 draft. Kept from it: the declaration verdict,
the span derivation, the static-candidate/runtime-reach join for composition,
and the DMLS choke point. Changed: the template source of declarations
(`M5`), `try` typing (`M8`), the `$()` frontmatter surface and the meaning of
"expression-typed frontmatter" (`M19`), the registry location (`M3`), a
text-equality guard on the join (`M15`), and corrected evidence (root
candidates *are* merged from child reports, `report.rs:565`).

Execution conventions for every phase:

- Run package-area recipes only, inside `darkmatter/`: `just test` and
  `just lint` (they cover `darkmatter`, `darkmatter-cli`, and `darkmatter-lsp`).
  Never run workspace-wide gates. Tests run under nextest through the recipes.
- Never run `cargo fmt`. `just lint` includes a format check, so write code in
  rustfmt style by hand and fix what the check reports by editing.
- Never commit, never run `just complete`, never move the spec. The terminal
  state is **"implementation complete, ready for review"**.
- Stay in this worktree and branch.
- Each task that changes a symbol's behavior fixes that symbol's `///`, `//!`
  and inline comments in the same change (repo drift rule; code is correct,
  comments are wrong).
- No test opens a terminal or browser window; everything here is L1.
- Test file reads use `manifest_dir!().join(..)` or `include_str!` (rust-testing
  rule 7); `md` spawns go through `CliProcessFixture` (rust-testing rule 8).
- The implementation log is `implementation-log.md` beside this plan; create
  it in Phase 1.

## Summary and Definition of Done

### What is being built

A new advisory, `dm.expression.undeclared_member` (Warning), reports a constant
member selector whose base has a known closed object type that does not
declare the member:

```markdown
---
$schema:
  server: "{ name: string, port: number }"
server: { name: web, port: 80 }
---
{{ server.nmae }}                      <- warns at `nmae`
{{ map(load_data("hosts.yaml", "{ name: string, port: number }[]"), h => h.nmae) }}
{{ map(try(load_data("hosts.yaml", "{ name: string }[]"), []), h => h.nmae) }}
```

One library classifier decides the verdict for an expression at a typed site.
Two consumers adapt it:

- **DMLS** checks every branch and callback body of each authored expression
  through the existing call-diagnostic choke point.
- **Composition** warns only for selectors the evaluator actually reached. The
  passive precheck computes per-site *candidates* once per document under the
  prepared schema; the evaluator records which access nodes it reached; a
  warning exists only where both hold.

When a document's schema holds unresolved dynamic slots, declarations come from
the schema **template** (no resolution, no I/O), so both consumers still check
before and during resolution. Member lookup still returns `null`. The code is
registered in the compose diagnostic registry so `md compose --deny
dm.expression.undeclared_member` and `--deny all` block through the existing
checkpoints.

### Definition of Done

Each item has a named test in the phase that delivers it.

- [ ] Every row of the spec's type table and absence table is pinned through
  `expression_member_findings` (Phase 2).
- [x] AC1: the `map(load_data(...), h => h.nmae)` document composes under the
  default policy with exactly one warning at `nmae` for three rows, none for
  `[]`; DMLS reports it in both cases; a computed contract reports neither
  (Phases 3, 5).
- [x] AC2: dot, parenthesized, `fm.`, and constant-bracket forms; optional unset
  member silent; typed function return and literal object bases (Phases 2, 5).
- [x] AC3: a schema with a pending enum slot reports `server.nmae` in DMLS before
  its snapshot is published, and composition reports it with and without
  resolution (Phases 3, 5).
- [x] AC4–AC6: union/scalar/dictionary/open/pattern/bounded-ref/unresolved-import
  rows; nested paths; dotted bracket key; computed key; typed array element;
  no cascade; shadowing; per-site identity; include dedup (Phases 2, 5).
- [x] AC7: every covered composition surface, library/editor agreement on code,
  message, severity, span; the documented reachability difference; body edits,
  multiline frontmatter, Unicode before the member (Phase 5).
- [x] AC8: default success; `--deny` code and `--deny all` stop; structured CLI
  output carries the shared diagnostic with clean stderr; a later failure keeps
  earlier member warnings; existing root, suppression, namespace, and
  unobserved-evaluator tests pass unchanged (Phase 5).
- [x] `try(...)` typing feeds member checks: `try(load_data(..., "{ a: string }[]"), [])`
  items are checked; `try` is not an absence guard (Phases 2, 5).
- [ ] AC9: `docs/topics/darkmatter-expressions.md`, `dmls/docs/diagnostics.md`,
  `docs/cli/policy.md`, and the Darkmatter skill describe current behavior with
  examples and never name a feature or spec (Phase 6).
- [ ] `just test` and `just lint` pass in `darkmatter/` (Phase 7).

**Not in this plan.** No performance spike (spec non-goal; the join costs a
hash lookup per reached access node in a candidate expression). No new test
tier. No `simplified-schema` change (`M2`). No new input-format parser, so no
Input Robustness Matrix applies; the only external input is a `load_data` YAML
fixture read by existing code. `docs/dependencies.md` is unchanged (no crate is
added).

## Phase 1 — Rebase check, decisions, baseline

Goal: confirm the earlier features landed as this plan assumes, record every
decision, and prove the starting state is green.

### Necessary Rules

**Rebase check**

- **R0. Assumptions about earlier features.** This plan is written before
  2026-11-05-dynamic-schema-types and 2026-11-05-expression-recovery land; it
  must not rely on their uncommitted code. Each assumption below names the rule
  of that feature it rests on (`N…` = the dynamic-schema-types plan; `ER/…` = the
  expression-recovery spec section, since its plan is being re-planned) and the
  rules of this plan that depend on it. Phase 1, Wave 1 verifies each against the
  landed code. A failed assumption: record the landed fact in the log, revise the
  dependent rules in this file in the same task, and continue.

  | ID | Assumption | Rests on | Used by |
  | --- | --- | --- | --- |
  | D1 | A slot lives only in a property's constraints (`Constraint::Dynamic`, `Constraint::DynamicValue`); a template's `SchemaShape.properties`, `pattern_keys`, `TypeExpr::InlineObject` bodies, and `DefinitionTable` are complete and unchanged by resolution | N3, N4 | M4, M5 |
  | D2 | `SchemaError::UnresolvedSlots(Box<UnresolvedSlots { slots, pending: Option<PendingSchema> }>)` exists and `PendingSchema` exposes `template: Option<(SchemaRoot, Arc<DefinitionTable>)>` without resolving | N10 | M5 |
  | D3 | Without a slot session, `DarkmatterSchemas::effective_for` returns `Err(UnresolvedSlots)` for a template document layer or a matched pending trigger payload, and `PreparedSchemas::effective_for` (`compose/schema_validation.rs:241`) maps that to `None`; with a session it returns a concrete `EffectiveSchema` holding no `Constraint::Dynamic` | N12a, N31, N34 | M5 |
  | D4 | With a session, slot resolution happens inside `schema_validation::prepare_schemas` / `effective_for`, before `run_call_precheck` (`pipeline/mod.rs:445`), so the precheck sees the resolved schema | N31 (Wave 14) | M5, M13 |
  | D5 | What a library composition with no slot session does once the schema is pending: it fails at schema validation with `dm.schema.slot_pending`, or it composes with validation deferred. Either is acceptable; the log records which, and the AC3 "without resolution" test (Phase 5) asserts the landed behavior. **Revised in Phase 1 (differs):** a library composition always has a session: `run_root_pipeline_collecting` attaches `SlotSession::for_request` in `SessionMode::Full` when none is set (`pipeline/mod.rs:79-82`), so a pure-literal pending schema resolves. A template stays unresolved in composition only under a caller-attached `SessionMode::Passive` session (validation fails, `dm.schema.slot_pending` via `report_diagnostics.rs:341-352`), a `ReferenceCheck` session, or a pins-only discovery pass (both defer, `schema_validation.rs:795-802`). The AC3 "without resolution" test attaches a `Passive` session | N31, N37 | Phase 5 AC3 test |
  | D6 | `Shape::from_definition` / `Shape::of_type` on an atom carrying `Constraint::Dynamic` yields its primitive family (`enum` → string family) with no finite member list (`shape.rs:577` `enum_members` finds no `Members`), and neither the call check nor narrowing panics on it | N3 | M5 |
  | D7 | DMLS serves a fresh or last-good (stale) snapshot as a `SchemaBundle` whose `effective` is concrete; before the first publish the overlay's `SchemaOutcome` (`overlay/mod.rs:201`) holds the unresolved result, and no request performs I/O or evaluation. `DocumentOverlay::stale` (`overlay/mod.rs:198`) still means "last-good frontmatter tree", not snapshot staleness. **Phase 1 finding (holds, shape differs from M12's guess):** `SchemaOutcome` is `Ready(Option<Arc<SchemaBundle>>) \| Failed(Arc<SchemaError>)` (`overlay/mod.rs:74`); the pending state is `Failed(UnresolvedSlots)` plus `DocumentOverlay.slots: Option<SlotStatus>`, whose `SlotStatus { refreshing, unresolved, template_shape }` (`overlay/mod.rs:241`) carries only the template's root shape (no definitions). Assembly calls `schemas.effective_for(&md)?` at `overlay/schema.rs:1048` under `SlotUse::{None, Pins, Resolve}` (`:962`); `declared_types` (`diagnostics/call_check.rs:59`) types only `Ready(Some)` | N39 | M5, M12 |
  | D8 | A pure literal slot (`enum({{ ["web", "db"] }}; required)`) resolves with no approval prompt or grant, so a library test can resolve it with a plain `SlotSession` | N14, N20, N31 | Phase 5 AC3 test |
  | E1 | `try(expr, fallback)` is an ordinary `FunctionCall { name: "try", args }` (no new AST node, no new token); every static walk (`walk_static`, `branch_scope`, `check_expression_calls`) sees both arguments | ER/"Syntax and evaluation" | M7, M8 |
  | E2 | `expression_type` (`call_check.rs:204`) types `try(a, b)` as `Shape::union` of both branches, keeping successful `null`, with an authored `[]` fallback adding no alternative when the other branch admits the empty array | ER/"Static types and editor behavior" | M8 |
  | E3 | `try` handles no absence: `AbsenceScope::call_argument("try", i, 2)` does not set `handles_absence`, so `try(server.nmae, "x")` is an ordinary read, and `try` declares no narrowing | ER/"Reporting…" ("`try` is not an absence guard") | M7 |
  | E4 | **(Holds; the path is `evaluate_recovery`, `expression/mod.rs:1026-1046`, not `evaluate_lazy_function`, `:984`, which is `unreachable!` for `try`.)** The lazy evaluation path passes the observer through each selected argument, and a recovered failure does not discard observations made before it | ER/"Syntax and evaluation" (no undo) | M14 |
  | E5 | Callback item typing for `map(try(...), h => ...)` takes the source argument's type from `expression_type`, so `h` gets the union's item type | ER/"Static types…" (callback item typing) | M8 |

**Library classifier**

- **M1. Code, severity, home.** `pub const UNDECLARED_MEMBER_CODE: &str =
  "dm.expression.undeclared_member";` in
  `lib/src/markdown/compose/expression/diagnostics.rs` next to
  `INCOMPATIBLE_CALL_CODE` (`diagnostics.rs:39`) and `UNCERTAIN_CALL_CODE`
  (`:44`), re-exported from `expression/mod.rs` in the `pub use diagnostics::{…}`
  list (`mod.rs:130-134`). Severity is always `Severity::Warning`; no API takes a
  strict flag. Evidence: spec "Report one warning per authored access site".
- **M2. No `simplified-schema` change; spans are derived.** A postfix
  `MemberAccess` node's span ends at its name token
  (`simplified-schema/lib/src/expression/parser.rs:550-551`:
  `expr.span.start..name_tok.span.end`), and a dotted name and a dotted
  `Variable` are one contiguous lexer token (`lexer.rs:578-582` pushes `.` and
  identifier characters without whitespace). So:
  - postfix: segment *k* of `name` (split on `.`) ends at `node.span.end -
    Σ(len of later segments + 1)` and starts `len(segment)` earlier;
  - dotted variable: segment *k* starts at `span.start + Σ(len of earlier
    segments + 1)`;
  - constant bracket key: the span of the `StringLiteral` index node, quotes
    included ("the key's string token").
  All byte offsets are UTF-8, relative to the parsed text. Evidence: spec
  "Range the offending member name"; `ast.rs:119`/`:132` (`Index`, `MemberAccess`; spanned kinds `:308`/`:315`).
- **M3. Registry.** Add `(super::expression::UNDECLARED_MEMBER_CODE, "a member
  read the known object type does not declare")` to `COMPOSE_DIAGNOSTIC_CODES`
  (`lib/src/markdown/compose/policy.rs:78`), directly after the
  `UNDECLARED_PROPERTY_CODE` row (`:83`). That array is the compose policy
  registry: `cli/src/args/policy.rs:59` (`PolicyCommand::deny_codes`) reads it
  for `--deny` parsing, help, and completion, so no CLI-side table changes. Add
  one unit test to the `cli/src/args/policy.rs` test module (beside
  `link_title_codes_are_deniable_on_compose_and_render_only`, `:630`):
  `parse_selector(Compose, PolicyVerb::Deny, UNDECLARED_MEMBER_CODE)` is
  `PolicySelector::Code`, `long_help(Compose, Deny)` contains it, and `Render`
  rejects it. No new recovery and no new flag. Evidence: darkmatter skill "A new
  coded warning adds its code to `COMPOSE_DIAGNOSTIC_CODES`"; spec "Do not
  introduce new policy flags".
- **M4. Declaration verdict.** In
  `lib/src/markdown/compose/expression/scope/projection.rs`, beside
  `member_shape`/`arm_member` (`:622-673`):

  ```rust
  pub(crate) enum MemberDeclaration { Declared, Undeclared, Unknown }
  pub(crate) fn declares_member(shape: &Shape, name: &str, definitions: &DefinitionTable) -> MemberDeclaration;
  ```

  - `Shape::Unknown` → `Unknown`; `Shape::Arms` with no non-null arm → `Unknown`
    (null-only; `nullable` is ignored).
  - Per non-null arm: `atom.is_array` → `Unknown`; `TypeExpr::InlineObject(s)`
    with non-empty `s.pattern_keys` → `Unknown`; otherwise closed, declaring
    `name` iff `s.properties.contains_key(name)` (optional members are in
    `properties`); `TypeExpr::Ref(key)` with alias budget left (start at
    `definitions.len()`, decrement per hop, as `arm_member` does at `:628`/`:659`) and a
    definition present → classify the body's arms recursively; missing
    definition, exhausted budget, `TypeExpr::Imported` → `Unknown`; every other
    `TypeExpr` (`Primitive` incl. bare `object` and `enum`, `Tuple`, `Function`)
    → `Unknown`.
  - Verdict: any `Unknown` arm → `Unknown`; else any arm declares → `Declared`;
    else `Undeclared`.
  Make `member_shape` `pub(crate)` (it is private at `:622`) and re-export both
  through `scope/mod.rs` as `pub(crate)`. Evidence: spec type table; "An open
  object, pattern-key object, unresolved reference, or any unknown alternative
  prevents a warning"; `unresolved_as_unknown` (`call_check.rs:344`) already
  makes a literal `load_data` contract's references unknown.
- **M5. Declarations without a resolved schema come from the template.**
  - Add to `lib/src/markdown/schemas/mod.rs`:

    ```rust
    pub struct DocumentTemplate { pub root: SchemaRoot, pub definitions: Arc<DefinitionTable> }
    pub enum SiteSchema {
        Effective(Option<EffectiveSchema>),
        Template { template: Option<DocumentTemplate>, unresolved: Box<UnresolvedSlots> },
    }
    impl DarkmatterSchemas {
        pub fn site_schema_for(&self, source: &Markdown) -> Result<SiteSchema, SchemaError>;
    }
    ```

    `site_schema_for` shares the body of `effective_for_with_override`
    (`mod.rs:838`): extract the document-layer resolution and trigger-layer
    steps into a private helper so the work happens once. When the result would
    be `Err(SchemaError::UnresolvedSlots(u))` it returns `Template { template,
    unresolved: u }` instead (the error is kept so a caller that reports slot
    state, M12, rewraps it as `SchemaError::UnresolvedSlots(unresolved)`; revised
    in Phase 1), `template` holding the **document layer's** root: the pending template's
    `(SchemaRoot, definitions)` when the document `$schema` itself is pending
    (D2), or the concrete document layer's `simplified` root and definitions
    when only a trigger payload is pending; `template: None` when the document
    layer has no SimplifiedSchema root. The pending template is read through
    `PendingSchema::template()` (`simplified-schema` `slots/pending.rs:217`, returns
    borrowed `(&SchemaRoot, &Arc<DefinitionTable>)`; clone both). Every other error propagates as today.
    It never resolves a slot and reads nothing `effective_for` would not read
    (D3). `effective_for` keeps its signature and behavior.
  - The site's declared layer is built by one helper,
    `DeclaredTypes::for_site(schema: &SiteSchema) -> DeclaredTypes<'_>` in
    `scope/projection.rs`: `Effective(e)` → `for_schema(e.as_ref())` (`:214`);
    `Template { template: Some(t), .. }` → `DeclaredTypes::new(Some(&t.root)).with_definitions(&t.definitions)`
    (existing `:197`, `:230`); `Template { template: None, .. }` → `for_schema(None)`. The
    template path has no trigger or baseline layer: a property only those
    layers declare is `Declaration::Undeclared` (`projection.rs:415`), its type
    is unknown, and members on it stay silent. That is conservative and correct
    because the document layer wins a shared property anyway (`declaration`,
    `:395-417`).
  - **Decision (approved by the author 2026-11-08, spec Ruling B):** the template-backed layer is the site's
    one declared layer, so the call check at that site sees the template types
    too. Building a second member-only `TypeScope` at every composition and DMLS
    site would duplicate guard, region, and callback scope construction
    (`TypeScope` facts are stored against their declared layer,
    `scope/mod.rs:238-256`, so a layer cannot be swapped after the fact). The
    call check on a template reaches the verdicts it reaches after resolution,
    minus findings that need slot-supplied members (an enum's finite values,
    D6), which is conservative: a constraint still waiting on a slot is
    treated as unknown, never as the literals written beside it; add a test
    that a value outside the literal members of a pending `enum(...)` is not
    rejected by the call check. Before dynamic-schema-types there was no pending
    state, so no shipped behavior changes.
  - A resolved schema (fresh, or DMLS's last-good snapshot, D7) always wins over
    the template; slots never change declarations, so both give the same member
    verdicts.
  Evidence: spec "Unresolved schema templates"; AC3.
- **M6. Eligible bases.** A selector is checked only when its base is a value
  the projection can type: a document property (bare, `fm.x`, or
  `fm["x"]` via `literal_index_key`, `fm_namespace.rs:61`, called at `projection.rs:307`), a lexical local (bare
  or `local.x`, `TypeScope::resolves_to_local`, `scope/mod.rs:221`), or any
  postfix/bracket base expression (call result, literal object or array,
  parenthesized expression). Never checked: a selector directly on `fm` or
  `local` (namespace wrappers), any root `DeclaredTypes::classify_variable`
  (`projection.rs:365`) classifies as `Namespace` (`ctx`, `env`, `current`) or
  `Host`, and `null`. An undeclared root has unknown type, so it yields no member
  finding and keeps its root warning alone. Evidence: spec "Preserve existing
  namespace and host-global diagnostics"; `ctx` already has
  `dm.expression.unknown_context_variable`.
- **M7. Absence handling extends the existing walker.** In
  `expression/absence.rs`, `walk_static` (`:503`) and `callback_reads` (`:230`)
  write into a private sink `struct StaticReads<'e> { variables: Vec<StaticVariableRead<'e>>, members: Vec<StaticMemberRead<'e>> }`.
  The public `static_variable_reads*` functions keep their signatures and
  results (they return `sink.variables`, still dropping lexical variable reads,
  `:238-245`). Add:

  ```rust
  pub(crate) struct StaticMemberRead<'e> {
      pub node: &'e SpannedExpr,      // the Variable, MemberAccess, or Index node
      pub step: usize,                // 0-based selector step within `node`
      pub handles_absence: bool,
  }
  pub(crate) fn static_member_reads(expr: &SpannedExpr) -> Vec<StaticMemberRead<'_>>;
  ```

  `static_member_reads` keeps local-rooted reads (locals are what is checked).
  One record per selector step: each dotted segment after the root (and after
  `fm.`/`local.` plus the root) of a `Variable`; each segment of a
  `MemberAccess` name; a `StringLiteral` `Index`. `handles_absence` is the
  scope's direct-position flag (new accessor `AbsenceScope::handles_absence()`)
  or the facts guarding the step's subject or an ancestor: for a variable, the
  existing `AbsenceScope::handles(prefix_through_step)` (`:82`); for a
  postfix/bracket node, a new `AbsenceScope::handles_expr(&Expr)` that asks
  `facts.subject(expr)` then `guards_path_or_ancestor`, mirroring `handles`.
  Because `walk_static` already passes `scope` unchanged into `Index`/
  `MemberAccess` bases (`:537-541`), direct-position, nullable-parameter,
  overload, consumed-by-call, and branch-fact rules apply to members with no new
  rule. Gates (`when=`, `$()` ternary conditions) are walked with
  `AbsenceScope::default()`, so `when="server.nmae"` is an ordinary read.
  `try` arguments follow E3. Evidence: spec "Preserve intentional absence
  handling"; module docs `absence.rs:1-52`.
- **M8. Typing the base.** For a record, `site_at = branch_scope(expr,
  &node.span, site.clone())` (`absence.rs:427`). Base type of step 0: for a
  dotted `Variable`, `site_at.type_of_expr(&Expr::Variable(prefix))` with the
  prefix up to the step's base; for `MemberAccess`/`Index`,
  `expression_type(base, &site_at)` (`call_check.rs:204`). Step *k*+1's base is
  `member_shape(base_k, Member::Field(name_k), site_at.definitions())`. Stop at
  the first step whose verdict is `Undeclared` (one finding) or `Unknown` (none):
  no cascade. Because the base goes through `expression_type`, `try(a, b)` bases
  and `map(try(...), h => h.x)` item types follow E2/E5 with no member-specific
  code. Evidence: spec "Once a missing step projects to unknown … do not emit
  cascading warnings"; `member_shape` already yields unknown for an undeclared
  member (`projection.rs:526`).
- **M9. Findings API and message.** New file
  `lib/src/markdown/compose/expression/member_check.rs`, declared and
  re-exported from `expression/mod.rs`:

  ```rust
  pub struct MemberFinding {
      pub member: String,
      pub subject: String,
      pub span: SourceSpan,       // member-name span in the parsed text
      pub node_ordinal: usize,    // M10
  }
  impl MemberFinding { pub fn diagnostic(&self) -> ExpressionDiagnostic; pub fn message(&self) -> String; }
  pub fn expression_member_findings(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<MemberFinding>;
  pub fn expression_member_diagnostics(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<ExpressionDiagnostic>;
  ```

  `text` is the exact text `expr` was parsed from. Findings: every
  `static_member_reads` record with `handles_absence == false`, eligible (M6),
  verdict `Undeclared` (M4/M8); sorted by `span.start`. Subject: the authored
  text up to the step's base (`server`, `fm.server`, `local.h`, `server.a`) for a
  variable; `&text[base.span]` for a postfix/bracket base, extended by earlier
  segments. Message (library-owned): `Member '{member}' is not declared by the
  known object type of '{subject}'.` The diagnostic: `code:
  UNDECLARED_MEMBER_CODE`, `Severity::Warning`, `expected_type: None`,
  `actual_type: None`, `span` = the member span. Precedent:
  `expression_call_diagnostics` (`diagnostics.rs:340`), `ExpressionDiagnostic`
  (`:80`).
- **M10. Node ordinals.** The evaluator runs the span-erased `Expr`;
  `SpannedExpr::erase` mirrors node for node (`ast.rs:379-437`). Define in
  `member_check.rs` two pre-order enumerators with one child order:
  `spanned_nodes(&SpannedExpr)` and `expr_nodes(&Expr)`. Order: `ArrayLiteral`
  items; `ObjectLiteral` values; `Paren`/`UnaryNot`/`UnaryMinus` inner;
  `Binary`/`Comparison` left, right; `Index` base, index; `MemberAccess` base;
  `LogicalOr` left, right; `Ternary` condition, then, else; `FunctionCall` args;
  `CallbackCall` args in order, a callback contributing its body. A finding's
  `node_ordinal` is its access node's index in `spanned_nodes`. A unit test pins
  equal length and variant sequence for a corpus covering every variant.

**DMLS**

- **M11. One choke point.** `dmls/src/diagnostics/call_check.rs::call_diagnostics`
  (`:66`) gains `text: &str` and appends `expression_member_diagnostics(expr,
  text, site)` after the call diagnostics, independent of `strict`. Its three
  callers pass the text their `SpannedExpr` was parsed from:
  `providers/dsl.rs:1094` (body `{{ }}` and directive `when=`),
  `providers/collections.rs:72` (`::map`/`::filter` gates and callbacks,
  `authored.site.text`), `diagnostics/frontmatter.rs:1199`
  (`FrontmatterCalls::check`: frontmatter interpolations and `expression`-typed
  values outside lifecycle stacks). Lifecycle stacks
  (`check_diagnostics`, `:80`, fed by `lifecycle_call_checks`) are not extended
  (spec: hosts are not required to change). The LSP record is built by the
  existing `lsp_diagnostic` (`:41`); its `data` carries null expected/actual
  types. Update the module doc (`:1-18`): the entry point also carries member
  diagnostics.
- **M12. DMLS declared layer.** `call_check::declared_types(ctx)` (`:59`) returns
  `DeclaredTypes::for_site(..)` (M5): the bundle's `effective` when a bundle
  exists (fresh or last-good, D7), else the overlay's document template, else
  untyped. **Revised in Phase 1** to the landed overlay (D7): the assembly
  (`overlay/schema.rs:1048`, `schemas.effective_for(&md)?`) calls
  `site_schema_for` instead; `Effective(e)` becomes the bundle as today, and
  `Template { template, unresolved }` becomes the existing
  `SchemaOutcome::Failed(Arc::new(SchemaError::UnresolvedSlots(unresolved)))`
  (so slot diagnostics, `slot_jobs::template_shape`, and job demand at
  `overlay/mod.rs:1309-1350` are unchanged) with `template` stored in a new
  `SlotStatus.template: Option<Arc<DocumentTemplate>>` field beside
  `template_shape` (`overlay/mod.rs:241-252`). `SchemaOutcome` gains no
  variant. Hover and completion (`providers/dsl.rs:~542`) are not changed.
- **M13. DMLS code constant.** `dmls/src/diagnostics/codes.rs`:
  `pub const EXPRESSION_UNDECLARED_MEMBER: &str =
  darkmatter::markdown::compose::expression::UNDECLARED_MEMBER_CODE;` beside
  `EXPRESSION_UNKNOWN_FUNCTION` (`:166`, same delegation style). Add the code to
  any per-code severity table in `codes.rs` as Warning.

**Composition**

- **M14. Runtime reach record.** In `expression/absence.rs`, the observer trait
  (`MissingRootObserver`, `:595`) gains provided items:

  ```rust
  const OBSERVES_MEMBERS: bool = false;
  fn member_read(&mut self, _node: usize) {}
  fn finish_expression(&mut self, _root: &Expr) {}
  ```

  `()` and `Vec<MissingRoot>` keep the defaults, so unobserved evaluators and
  unmigrated surfaces behave as today. `evaluate_expr` (`expression/mod.rs:602`)
  calls `observer.member_read(expr as *const Expr as usize)` when
  `O::OBSERVES_MEMBERS` and the node is a dotted `Expr::Variable`, an
  `Expr::MemberAccess`, or an `Expr::Index` whose index is `StringLiteral`
  (record at node entry, before evaluating the base, so a read reached before
  a failure is kept). `evaluate_observed` (`:559`) calls `observer.finish_expression(expr)`
  after evaluation on success and on error. A new
  `pub(crate) struct Observation { pub(crate) roots: Vec<MissingRoot>, reached: Vec<usize>, addresses: Vec<usize> }`
  implements the trait with `OBSERVES_MEMBERS = true`; `finish_expression`
  turns recorded addresses into ordinals with one `expr_nodes(root)` walk (M10),
  deduplicated per expression evaluation, and exposes
  `take_reached() -> Vec<usize>`. Per-iteration callback visits cost one push
  per node visit; the walk runs once per expression. Lazy calls (and `try`, E4)
  already thread the observer (`mod.rs:913-925`; `try` through `evaluate_recovery`, `:1026-1046`).
- **M15. Candidates and site keys.** New file
  `lib/src/markdown/compose/member_sites.rs`:

  ```rust
  pub(crate) struct MemberSiteKey { pub(crate) scope: Option<String>, pub(crate) start: usize }
  pub(crate) struct MemberCandidate { pub(crate) expression: String, pub(crate) warning: ComposeWarning }
  pub(crate) struct MemberCandidates { /* HashMap<(MemberSiteKey, usize /* ordinal */), MemberCandidate> */ }
  impl MemberCandidates { pub(crate) fn warnings(&self, key: &MemberSiteKey, expression: &str, reached: &[usize]) -> Vec<ComposeWarning>; }
  ```

  `scope` is the frontmatter key-path label (`call_precheck.rs:431`
  `path_label`), `None` for body, condition, and collection sites. `start` is the
  authored byte offset of the construct (the `{{` of an interpolation, the first
  byte of a `when=` or `$()` condition text, the first byte of a collection
  gate or callback expression) in the document's loaded source (body) or in the
  authored leaf (frontmatter). Precheck and runtime compute it through each
  surface's existing projector: `BodyOrigin::project` (`body_origin.rs:346`)
  for body text, `FrontmatterExpressionLocation::project`
  (`frontmatter_interpolation.rs:586`) for frontmatter leaves, the region
  copy's origin for collection bodies. A projection that fails gives no key and
  no warning (spec: unknown positions stay absent). `warnings` also requires the
  evaluated expression text (trimmed) to equal the candidate's `expression`; a
  mismatch (text a stage rewrote) yields nothing, so the ordinal join can never
  attach a finding to a different expression. A repeated `::map` copy and a
  repeated include of the same child project to one authored site, which is how
  iterations and includes deduplicate.
- **M16. Candidates come from the precheck, once per document.**
  `call_precheck::check_document` (`call_precheck.rs:217`) takes
  `&SiteSchema` instead of `Option<&EffectiveSchema>` and builds its layer with
  `DeclaredTypes::for_site` (M5). Wherever it runs `check_expression_calls`
  through `checked` (`:617`) or `rejected_condition` (`:604`) on a frontmatter
  leaf, body interpolation, `::block when`, `::file`/`::code when`, collection
  gate or callback, or region-body surface, it also runs
  `expression_member_findings` on the same `SpannedExpr` under the same scope and
  records a `MemberCandidate` under that surface's key. It also parses each
  frontmatter `$()` leaf with `parse_frontmatter_shell_value_spanned`
  (`frontmatter_shell_expansion.rs:146`) and records candidates for its ternary
  condition and `Value` branches (member findings only; the call check's
  coverage is unchanged). `check_document` returns the candidates beside its
  warnings; `run_call_precheck` (`pipeline/mod.rs:1049`) uses
  `prepared_schemas.site_schema_for(markdown)` (new, wrapping M5 as
  `effective_for` wraps today, `schema_validation.rs:241`) and stores the
  candidates on the report as `member_candidates: Arc<MemberCandidates>`. The
  existing call warnings and rejection order are unchanged. Because candidates
  are fixed under the schema the precheck used (resolved per D4, else the
  template), no frontmatter pass decides a mismatch under a provisional schema.
  A trigger re-match after frontmatter shell expansion does not recompute
  candidates; record this in the docs as the rule (the document layer, which
  wins shared properties, cannot change there).
- **M17. Warning construction.** Each candidate's `ComposeWarning` is built in
  the precheck by the machinery call advisories use (`advisory_warnings`,
  `call_precheck.rs:259`; extract its authored-position projection into a shared
  helper rather than copying it): `source = ComposeWarning::EXPRESSION_SOURCE`,
  `code = UNDECLARED_MEMBER_CODE`, `diagnostic = Some(finding.diagnostic().at(file,
  position))` with the authored member-name position (`one_based_position`,
  `diagnostics.rs:264`, character columns), `identity = WarningSubject::Expression
  { document: None, scope, origin: ExpressionOrigin::Authored(authored
  member-name range) }` (`context/report.rs:718-745`; the document is stamped at
  merge), and the surface's stage name. Message: the library message with its
  trailing `.` removed, escaped with `Prose::escape_text`, then the same location
  suffix call advisories append, then `.`. Identity keys `(source, code,
  subject)` (`report.rs:455-468`), so separate sites with identically spelled
  locals keep separate warnings and repeated reaches collapse.
- **M18. Emission and policy.** Member warnings are produced right after the
  expression that reached them, never in a pass with
  `options.defer_expression_failures` set (the predicate
  `unknown_identifiers::undeclared_warnings` uses, `unknown_identifiers.rs:76`),
  so discovery and reference-check passes report none. Routing, per surface, to
  the checkpoint its other warnings reach:
  - interpolation (body, frontmatter, `$()` branch text): the `Evaluator`
    (`interpolation/evaluator.rs:207`) gains
    `with_member_sites(MemberSiteView { candidates: Arc<MemberCandidates>, scope:
    Option<String>, project: &dyn Fn(usize) -> Option<usize> })` and
    `member_warnings(expression: &str, scanned_start: usize) -> Vec<ComposeWarning>`.
    `interpolate_text_selected` (`interpolation/rewrite.rs:273`) calls it right
    after `evaluator.eval(&expr)` (`:370`) and appends the result to `warnings`
    exactly as it handles the context warnings at `:365-367`, so the next
    `before_eval` (`blocking_guard`, `pipeline/mod.rs:907`) and the stage's final
    check judge them. The bare-variable fast path in `Evaluator::eval`
    (`evaluator.rs:294-320`) records ordinal 0 for a dotted name. The directive
    target pass keeps `drop(evaluator.take_missing_roots())`
    (`inline/interpolation.rs:114`) and installs no member view, so a nullable
    target gets no second warning.
  - conditions (`::block when`, `::file`/`::code when`, collection gates and
    callbacks): `ConditionSite::evaluate` (`conditions.rs:100`) takes an
    `&mut Observation`; the engine turns its reached ordinals into warnings with
    the candidate table and adds them where it adds the condition's other
    warnings, before the existing `ReportWatch` / `stop` check
    (`pipeline/mod.rs:959`, stop at `:1001`).
  - `$()` ternary conditions and value branches
    (`frontmatter_shell_expansion.rs:1791`, `:2027-2029`): the local `Vec<MissingRoot>`
    becomes `Observation`; reached ordinals become warnings returned with the
    stage report and added before `stop_if_blocked` before frontmatter shell
    expansion continues (`pipeline/mod.rs:580`).
  `member_candidates` is per document and is not merged from a child report
  (unlike `unknown_root_candidates`, which are, `report.rs:565`); a child's
  member *warnings* merge with their document through `attribute_to`. Blocking
  changes the outcome, never the severity. Earlier warnings survive a later
  failure through the existing failed-document stash.
- **M19. Surface coverage.** Covered in composition: frontmatter interpolation
  (both passes), body interpolation (including region bodies), `::block when`,
  `::file`/`::code when`, `::map`/`::filter` gates and callbacks, and frontmatter
  `$()` ternary conditions, value branches, and branch `{{ }}` holes (the spec's
  "shell-template holes"; the only shell surface that observes reads today,
  `frontmatter_shell_expansion.rs:1780-1810`). Covered in DMLS: every surface of
  M11, including `expression`-typed frontmatter values. Not covered: lifecycle
  surfaces; body `::shell` command-template holes, which evaluate through the
  unobserved `HoleEvaluator` (`shell_expansion/template/fill.rs:37-73`) and
  refuse a `null` value already; `expression`-typed values in composition, which
  composition never evaluates (passive format) — in DMLS they are checked, which
  is part of the documented reachability difference. Standalone unobserved
  evaluators (`evaluate`, `evaluate_condition`, `Evaluator::eval` without
  observation) keep their behavior.

**Tests and docs**

- **M20. Test files.** Library classifier: new `lib/tests/l1/member_check.rs`.
  Library composition: new `lib/tests/l1/compose_member_warnings.rs`. DMLS: new
  `dmls/tests/l1/undeclared_member.rs`. CLI: new
  `cli/tests/l1/compose_member_warnings.rs`. Register each in its directory's
  `main.rs`. Each composition test starts with a **control** (the same fixture
  spelled `name`) that composes with zero member warnings, so each edit is
  load-bearing. Assertions compare code, severity, line, column, and message
  fields, not rendered text.
- **M21. Fixture.** One `hosts.yaml` per test in a temp dir (three rows, and an
  empty-list variant), plus a pending-template schema: `server: "{ name: string,
  role: enum({{ [\"web\", \"db\"] }}; required) }"`. If dynamic-schema-types
  landed shared fixtures (`lib/tests/dynamic_slots_fixtures/mod.rs`), reuse its
  builder instead of re-creating one.
- **M22. Docs audience.** Each page leads with what the reader can do, gives a
  compact example per rule, and uses a Mermaid diagram for the composition flow
  (precheck candidates → reached nodes → warning). The README stays an overview
  (no change). No doc names a feature or spec. Not-covered surfaces (M19) are
  stated plainly.
- **M23. Spike decision.** None (spec non-goal; M14 adds one push per reached
  access node and one walk per observed expression).
- **M24. Deferral search.** No task says "decide during implementation". The
  places reading cannot settle today (the landed shape of D2/D5/D7) are covered
  by R0 with named dependent rules and by tests that assert the landed result.

### Wave 1 (serial)

- [x] **Rebase verification (R0)** — for each row of R0, find the landed code
  (`rg` for `UnresolvedSlots`, `PendingSchema`, `SlotSession`, `site_schema_for`
  collisions, `SchemaOutcome`, `"try"` in the catalog and `absence.rs`) and write
  one log entry per assumption: holds / differs (with `path:line`). For each
  difference, revise the named rules in this file in the same task. Also confirm
  that the evidence lines cited in M1–M19 still point at the named symbols and
  update any that moved.
  - Done when: the log has an entry per assumption ID and every cited symbol
    resolves.

### Wave 2 (serial)

- [x] **Baseline** — run `just test` and `just lint` in `darkmatter/` before any
  change; record pass/fail in the log. A pre-existing failure is reported to the
  author in the log and not fixed here.

## Phase 2 — Shared classifier and template declarations (library)

Goal: the library returns every unhandled undeclared-member finding for an
expression at a typed site, and the precheck can type a pending document from
its template.

### Wave 3 (parallel; disjoint files)

- [x] **Declaration verdict** — M4 in `scope/projection.rs`, re-exports in
  `scope/mod.rs`. Unit tests in `scope/tests.rs`: `{name}`/`nmae`; optional
  member; `{name}|null`; null-only; two-object union declared/undeclared;
  `{name}|string`; `unknown`; bare `object`; pattern-key dictionary; array arm;
  `Ref` to a closed definition (declared, undeclared); missing `Ref`; a
  self-referential alias chain (exhausted budget); `Imported`.
- [x] **Absence member reads** — M7 in `absence.rs`; update its module doc to say
  members use the same rules. Tests in its test module, one row per spec absence
  table row (`server.nmae`; `|| "none"`; ternary condition; `is_null`;
  `min(…, 5)`; `is_empty(trim(…))`; comparison operand; true-branch proof;
  false-branch read), plus: sibling not covered; ancestor guard covers a
  descendant; `when`-style default scope does not handle; `try(server.nmae,
  "x")` is ordinary (E3); `static_variable_reads` results unchanged for a corpus
  of existing cases.
- [x] **Code and registry** — M1 in `diagnostics.rs` and `expression/mod.rs`; M3
  in `compose/policy.rs` and the `cli/src/args/policy.rs` unit test.

### Wave 4 (after Wave 3)

- [x] **Findings** — `member_check.rs` per M2, M6, M8, M9, M10.
- [x] **Template declarations** — M5 in `schemas/mod.rs` (`DocumentTemplate`,
  `SiteSchema`, `site_schema_for`, shared private helper) and
  `DeclaredTypes::for_site` in `scope/projection.rs`. Unit test beside
  `effective_for`'s tests: a pending document `$schema` gives
  `Template { template: Some(root), .. }` whose `server` shape is a closed object; a concrete one
  gives `Effective(Some)`; no slot evaluator runs (use the work counter
  dynamic-schema-types added for slot runs, else a host stub that panics).

### Wave 5 (after Wave 4)

- [x] **Library classifier tests** — `lib/tests/l1/member_check.rs` through the
  public `expression_member_findings` / `expression_member_diagnostics`:
  - `type_table_rows`: every spec type-table row (control `server.name` first);
  - `access_forms`: `server.nmae`, `(server).nmae`, `server["nmae"]`,
    `fm.server.nmae`, `fm["server"].nmae`, `local.h.nmae` in a callback,
    `{ name: "x" }.nmae`, a typed function return (a catalog function returning
    a closed object, else a host registration with that return), all at the
    same member text;
  - `paths_and_keys`: `a.b.c` with `b` missing → one finding on `b`;
    `server.nmae.length` → one finding; `server["a.b"]` → key `a.b`, not split;
    `server[key]` with undeclared `key` → no member finding; `hosts[0].nmae` over
    a typed array; undeclared root `sever.name` → none;
  - `contracts`: `load_data("h.yaml", "{ name: string }[]")` item typed;
    computed contract `load_data("h.yaml", kind)` → none; a contract naming an
    unresolved import → none; bounded named reference in the site's definitions
    → checked; cyclic alias → none;
  - `try_typing`: `map(try(load_data("h.yaml", "{ a: string }[]"), []), h =>
    h.nmae)` → one finding (E2/E5); `try(server, {}).nmae` → finding (both
    closed arms lack it); `try(server, { nmae: 1 }).nmae` → none;
    `try(server.nmae, "x")` → finding (E3);
  - `narrowing`: `server: "{ name: string } | string"`: `server.nmae` silent;
    `is_string(server) ? "" : server.nmae` → finding (false side narrows, no
    presence proof); `site.assume(...).invalidate(&WriteSet::destination("server"))`
    → silent again;
  - `shadowing`: a callback parameter named like a document property uses the
    parameter's type; two callbacks spelling `h.nmae` over unrelated item types
    → two findings with different `node_ordinal`s; outer capture inside a
    callback uses the outer type;
  - `spans`: multi-segment dotted variable, postfix dotted name, bracket key
    (quotes included), a non-ASCII character before the member (byte offsets);
  - `diagnostic_fields`: code, Warning, message text exactly as M9;
  - `ordinals_mirror_erase`: the M10 enumerator parity corpus.

### Validation checkpoint (end of Phase 2)

- [x] `just test` and `just lint` in `darkmatter/` pass; the existing
  `undeclared_property` and `static_variable_reads` tests pass untouched.

## Phase 3 — DMLS diagnostics

Goal: DMLS shows the shared diagnostic on every surface that already shows call
diagnostics, including documents whose schema is still a template.

### Wave 6 (disjoint files; template layer first)

- [x] **Choke point and code** — M11 in `diagnostics/call_check.rs` and its three
  callers (`providers/dsl.rs`, `providers/collections.rs`,
  `diagnostics/frontmatter.rs`); M13 in `diagnostics/codes.rs`.
- [x] **Template layer** — the overlay half of M12 in `overlay/schema.rs` and
  `overlay/mod.rs` (the `site_schema_for` call, the pending outcome's template
  field, and a `DocumentOverlay::template()` accessor). The choke-point task
  owns `diagnostics/call_check.rs`, so the `declared_types` change waits for
  this accessor: do this task first within the wave.

### Wave 7 (after Wave 6)

- [x] **DMLS tests** — `dmls/tests/l1/undeclared_member.rs`, modeled on
  `dmls/tests/l1/undeclared_property.rs` and `expression_call_diagnostics.rs`:
  - `reports_on_every_surface`: body interpolation, multiline frontmatter
    interpolation, `expression`-typed frontmatter value, `::block when`,
    `::file when`, `::map` callback, `::filter` gate — each at `nmae`;
  - `ac1_editor`: the AC1 expression with a three-row and an empty `hosts.yaml`
    both report at `nmae`; a computed contract reports nothing;
  - `all_branches`: unchosen ternary branch and a callback over a runtime-empty
    collection are reported;
  - `positions`: Unicode before the member (`é`, one UTF-16 unit; an astral
    character, two units) and a body edit that shifts later lines;
  - `strict_independent`: `expressions.strict` on and off give identical member
    diagnostics;
  - `pending_template` (AC3): the M21 pending-template schema, before any
    snapshot is published (registry gate held, `start_with_registry_gate`),
    reports `server.nmae`; and while a refresh runs with a last-good snapshot,
    reports it once (no duplicate);
  - `lifecycle_unchanged`: a lifecycle-stack surface gains no member diagnostic;
  - `no_side_effects`: extend the existing no-I/O/no-handler assertion
    (`call_check.rs` tests `diagnostics_never_enter_a_handler` pattern) with a
    member read inside a `load_data` callback.

### Validation checkpoint (end of Phase 3)

- [x] `just test` and `just lint` in `darkmatter/` pass.

## Phase 4 — Composition transport

Goal: composition warns exactly for reached selectors, once per authored site,
under the precheck's schema.

### Wave 8 (parallel; disjoint files)

- [x] **Observer plumbing** — M14 in `expression/absence.rs` (trait, `Observation`)
  and `expression/mod.rs` (`evaluate_expr`, `evaluate_observed`). Unit tests in
  `absence.rs`'s test module: dotted variable, postfix access, bracket key each
  report their ordinal; unchosen ternary branch and short-circuited `||` operand
  report nothing; a callback over `[]` reports nothing and over three items
  reports its ordinal once; an expression that fails after a read still reports
  it; `()` and `Vec<MissingRoot>` observers unchanged.
- [x] **Candidate store and key** — M15 in new `member_sites.rs` (register in
  `compose/mod.rs`) and the `member_candidates` field on `ComposeReport`
  (`context/report.rs`, not merged in the child merge at `:565`). Unit tests: key
  equality; no key when projection fails; text mismatch yields nothing; repeated
  lookups of one reached site produce one warning after `add_warning`.

### Wave 9 (after Wave 8)

- [x] **Precheck candidates** — M16/M17 in `call_precheck.rs`,
  `schema_validation.rs` (`site_schema_for` wrapper), and
  `pipeline/mod.rs::run_call_precheck`. Existing `compose_call_precheck.rs` tests
  pass unchanged.

### Wave 10 (after Wave 9; parallel, one task per file group)

Each task migrates its surface to `Observation`, computes its runtime key with
the surface's own projector (M15), and routes warnings per M18. Tests for these
surfaces live in Phase 5's file; each task adds a focused unit test in its own
file where the module already has a test module.

- [x] **Interpolation** — `interpolation/evaluator.rs`, `interpolation/rewrite.rs`,
  `inline/interpolation.rs` (body `rewrite_body`, target pass, pending reads),
  `frontmatter_interpolation.rs` (evaluator at `:1073`, drains at `:1168`,
  `:1251`). Body warnings are sorted by authored start before
  `report.add_warnings`, as roots are (`inline/interpolation.rs:238-240`).
- [x] **Conditions** — `conditions.rs` (`ConditionSite::evaluate`,
  `evaluate_condition_observed`), `page_blocks/engine.rs` (`:70`, `:157`),
  `transclusion/engine.rs` (`:686`).
- [x] **Collections** — `collection_blocks/engine.rs` (`:339-340` gates and
  callbacks, and region-body interpolation keys through the copy's origin).
- [x] **Frontmatter `$()`** — `frontmatter_shell_expansion.rs` (`:1791`, `:2029`,
  `:2089`) and the routing at `pipeline/mod.rs:604-607` (the only task in this
  wave that edits `pipeline/mod.rs`).

### Validation checkpoint (end of Phase 4)

- [x] `just test` and `just lint` in `darkmatter/` pass; `undeclared_property`,
  `unknown_identifier_warning.rs`, and the existing compose performance tests
  pass unchanged.

## Phase 5 — Acceptance tests

Goal: every acceptance criterion has a named test through public APIs. Test
in-process; spawn `md` only to prove CLI wiring.

### Wave 11 (parallel; disjoint files)

- [x] **Library composition** — `lib/tests/l1/compose_member_warnings.rs`, through
  the compose entry `compose_call_precheck.rs` uses:
  - `ac1_rows_and_empty`: three rows → one warning at `nmae` (line, column),
    composition succeeds; empty list → none; computed contract → none and no
    inferred names;
  - `ac2_forms`: dot, `(server).nmae`, `fm.server.nmae`, `server["nmae"]`, typed
    function return, literal object; `server.name` silent with `name` declared
    optional and unset;
  - `ac3_pending_template`: with resolution (a `SlotSession` resolving the pure
    slot, D8) → the warning and success; without resolution (a caller-attached
    `SessionMode::Passive` session, revised D5) → the composition fails with
    `dm.schema.slot_pending` and the failure's earlier warnings carry the member
    warning (the frontmatter interpolation site is evaluated before schema
    validation, so it is reached);
  - `ac5_paths`: nested missing step, `server["a.b"]`, computed key with an
    undeclared root (root warning only), `hosts[0].nmae`, no cascade;
  - `ac6_identity`: two authored callbacks → two warnings; many iterations →
    one; `local.h.nmae`; parameter shadowing; one child included twice → one;
    two children → two, each attributed to its document;
  - `ac7_surfaces`: positive and unreached case per M19 composition surface
    (frontmatter interpolation, body interpolation, `::block when`, `::file
    when`, `::map` gate and callback, `::filter` callback, region body, `$()`
    condition and branch hole); unchosen branch, short-circuited operand, and
    empty collection produce none; body edit before the site, multiline
    frontmatter, `é` before the member verify authored line/column;
  - `ac7_try`: `map(try(load_data("missing.yaml", "{ a: string }[]"), []), h =>
    h.nmae)` composes with no warning (fallback `[]`, callback not reached); with
    the file present → one warning;
  - `ac8_later_failure`: a member warning then a later rejected expression
    failure keeps the member warning in the failure's warnings;
  - `discovery_pass_reports_none`: a `defer_expression_failures` request reports
    none;
  - `pass_repetition`: a frontmatter leaf evaluated in both passes → one warning.
- [x] **CLI** — `cli/tests/l1/compose_member_warnings.rs` via
  `CliProcessFixture`: default `md compose` exits 0 and the warning appears on
  stderr for Markdown output; `--deny dm.expression.undeclared_member` exits 1
  with the shared diagnostic and no deliverable; `--deny all` likewise; JSON
  output carries `code`, `severity: warning`, `source.line`/`column` in the
  envelope `diagnostics` with stderr empty; `md compose --help` lists the code.
  Existing `policy_verbs.rs` and `compose_call_warnings.rs` pass unchanged.
- [x] **Agreement** — in `dmls/tests/l1/undeclared_member.rs`, add
  `library_and_editor_agree`: per surface fixture, the DMLS diagnostic and the
  composition warning's `diagnostic` match on code, message, severity, and span
  (composition one-based character column − 1 equals the LSP character for BMP
  text); `reachability_differs`: an unchosen branch is reported by DMLS and not
  by composition.

### Validation checkpoint (end of Phase 5)

- [x] `just test` and `just lint` in `darkmatter/` pass. `just test-l2` is not
  required (no terminal behavior).

## Phase 6 — Documentation and skills

Goal: current-behavior docs describe the feature (M22).

### Wave 12 (parallel; disjoint files)

- [ ] **Expressions topic** — `docs/topics/darkmatter-expressions.md`: a section
  "Misspelled members" after "Variable Access" (`### Bracket Access`): what is
  reported, the union rule with the type table as examples, access forms,
  absence table, one warning per authored site, template declarations for
  schemas with unresolved slots, runtime vs editor reachability, the Mermaid
  flow. Update the root-only sentence at `:114` and the diagram node at `:172`
  so they no longer imply roots are the only reported reads.
- [ ] **DMLS diagnostics** — `dmls/docs/diagnostics.md`: add the code to the
  code tables near `:233`, `:361`, `:400`, with editor behavior (all branches,
  independent of `expressions.strict`, works before a schema snapshot exists,
  lifecycle stacks not covered).
- [ ] **CLI policy** — `docs/cli/policy.md`: name the code beside
  `dm.expression.undeclared_property` (`:116`) and add a `--deny
  dm.expression.undeclared_member` example with its outcome.
- [ ] **Darkmatter skill** — `.claude/skills/darkmatter/expressions.md` (the
  classifier, `expression_member_findings`, `DeclaredTypes::for_site`, the
  template rule), `compose.md` (the candidate/reach join, `member_sites.rs`,
  routing per surface, not-covered list, next to "Undeclared properties"), and
  `dmls.md` (the choke point). Keep `SKILL.md` within its size limit; add one
  line to its CLI paragraph only if the registry sentence needs it.
- [ ] **Drift sweep** — re-read every `///`/`//!` in files this plan edited and
  fix drift in place.

## Phase 7 — Final verification

### Wave 13 (serial)

- [ ] `just lint` and `just test` in `darkmatter/` pass.
- [ ] `git status` shows only files this plan names plus the new tests and the
  log.
- [ ] Record every departure from the spec and every R0 revision in
  `implementation-log.md` (the spec is not edited). Report the terminal state
  **"implementation complete, ready for review"**; do not move the feature.


## Review decisions (2026-11-08)

The author approved the shared template-backed type layer for the call check
(spec Ruling B, with pending constraints treated as unknown) and accepted the
plan's readings: "shell-template holes" means frontmatter `$()`;
expression-typed frontmatter is checked in DMLS only; the code is registered in
the library's table (`policy.rs`); typed host globals are skipped. Spec
acceptance criterion 3 now says composition reports the warning after
resolution; the D5 test records the library's unresolved (`Passive` session) behavior,
since a library composition with no session attaches a resolving one.