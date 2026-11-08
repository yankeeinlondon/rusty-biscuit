---
kind: plan
created: 2026-11-08
phase: 1
total_phases: 6
agent: claude/sonnet
yolo: true
spec: 2026-11-05-member-property-check
area: darkmatter
packages:
    - darkmatter
    - darkmatter-lsp
---

# Plan: Member Property Check

## Summary and Definition of Done

**What is built.** A new advisory, `dm.expression.undeclared_member`, reports a
constant member selector (`h.nmae`, `(server).nmae`, `server["nmae"]`,
`fm.server.nmae`, `local.h.nmae`) whose base has a known type that declares no
such member. One shared classifier in the `darkmatter` library decides the
verdict for a given expression and typed site. Two consumers adapt it:

- **DMLS** (`darkmatter-lsp`) runs the classifier over *every* branch and
  callback body of each authored expression, exactly like the existing call
  diagnostics.
- **Composition** warns only for selectors the evaluator *actually evaluated*.
  Composition joins two facts per authored site: the classifier's static
  findings (computed once per document by the existing passive precheck, under
  the prepared effective schema) and a runtime "this selector node was
  evaluated" record from the evaluator. A warning exists only where both hold.

Member lookup still returns `null`; nothing about evaluation changes. The code
is registered in the compose policy catalog so `--deny dm.expression.undeclared_member`
and `--deny all` block it through the existing checkpoints.

**Definition of done.**

1. Every row of the spec's type table and absence table is pinned by a library
   test that goes through `expression_member_findings` / `expression_member_diagnostics`.
2. `md compose` on `{{ map(load_data("hosts.yaml", "{ name: string, port: number }[]"), h => h.nmae) }}`
   composes successfully under the default policy with exactly one
   `dm.expression.undeclared_member` warning ranged on `nmae`, however many
   rows the data has; with an empty data file it reports no warning; DMLS
   reports the authored typo in both cases; a computed contract reports
   neither.
3. Composition and DMLS agree on code, message, severity, and member-name span
   for equivalent sites; the documented reachability difference is separately
   asserted.
4. `--deny dm.expression.undeclared_member` and `--deny all` stop composition
   on each covered surface; structured CLI output carries the shared diagnostic;
   stderr stays clean for data output; earlier warnings survive a later failure.
5. Existing `dm.expression.undeclared_property`, namespace/host-global
   diagnostics, and unobserved evaluators behave unchanged (their existing
   tests pass untouched).
6. `docs/topics/darkmatter-expressions.md`, `dmls/docs/diagnostics.md`,
   `docs/cli/policy.md`, and the Darkmatter skill describe the new behavior
   (union rule, absence handling, site identity, reachability difference) with
   examples, and none links back to the spec. Drifted doc comments in the files
   this plan touches are fixed in the same change.
7. `just test` and `just lint` pass in `darkmatter/` (this covers the library,
   CLI, and DMLS packages of the area).

**Not in this plan.** No performance spike (the spec rules it out and no
OS-specific cost is named). No change to `simplified-schema`. No input-format
parser is added or changed (the expression grammar is a text grammar), so the
Input Robustness Matrix does not apply.

## Phase 1: Shared classifier in the library

Goal: one library function that, given a `SpannedExpr` and a site `TypeScope`,
returns every unhandled undeclared-member finding with its member-name span,
subject, message, and a stable node ordinal. No consumer changes yet.

### Necessary Rules

Each rule is a decision already made; evidence follows it.

**R1 — Code, severity, home.** The code is the constant
`UNDECLARED_MEMBER_CODE = "dm.expression.undeclared_member"`, defined in
`lib/src/markdown/compose/expression/diagnostics.rs` next to
`INCOMPATIBLE_CALL_CODE` and `UNCERTAIN_CALL_CODE`, re-exported from
`expression/mod.rs` where those are re-exported (line ~130 re-exports
`expression_call_diagnostics`). Severity is always `Severity::Warning`,
independent of `expressions.strict`. It is registered once in
`COMPOSE_DIAGNOSTIC_CODES` (`lib/src/markdown/compose/policy.rs`) with the
description "a member read the known object type does not declare".
Evidence: spec "Report one warning per authored access site";
`md compose --help` lists the registry (`docs/cli/policy.md`), and
`the_catalog_lists_each_code_once` pins uniqueness.

**R2 — No AST change in `simplified-schema`.** Member-name spans are derived
from existing node spans. A postfix `MemberAccess` node's span ends at the name
token (`parser.rs` `parse_postfix`: `expr.span.start..name_tok.span.end`) and the
name is one lexer token pushed byte-for-byte, so the name's span is
`node.span.end - name.len()..node.span.end` and segment *i* of a dotted name
starts after the preceding segments plus one `.` each. A dotted `Variable`
token is contiguous too (`lexer.rs` builds `name` char by char, no whitespace),
so segment *i* starts at `span.start + Σ(len of earlier segments + 1)`. A
bracket key's span is the `StringLiteral` index node's span (quotes included),
which is "the key's string token". Evidence: `parser.rs` lines 522-565,
`lexer.rs` lines 575-606.

**R3 — Eligible bases.** A read is eligible only when its root classifies as a
document property (bare or `fm.`-prefixed) or a lexical local (bare, or
`local.`-prefixed), or when it is a postfix/bracket access on any expression
whose type the projection knows. Every other root (`ctx`, `current`, `env`,
`null`, any reserved namespace other than `fm`/`local`, any declared host
global) is skipped entirely. Evidence: spec "Preserve existing namespace and
host-global diagnostics: namespace wrappers such as `fm` and `local` are not
closed data objects, but a typed value selected through them is eligible".
Use `TypeScope::resolves_to_local(path)` (public) for locals and
`DeclaredTypes::classify_variable` (`pub(crate)` in `scope/projection.rs`, so
usable inside the library) for the document-property class. An undeclared root
is a document-property read with no type; its type is `Unknown`, so it yields
no member finding (the root warning stays the only one).

**R4 — The declaration verdict.** Add `pub(crate) enum MemberDeclaration { Declared, Undeclared, Unknown }`
and `pub(crate) fn declares_member(shape: &Shape, name: &str, definitions: &DefinitionTable) -> MemberDeclaration`
in `lib/src/markdown/compose/expression/scope/projection.rs`, beside
`member_shape`/`arm_member` (same file, same alias-bound idiom). Rules:

- `Shape::Unknown` → `Unknown`.
- `Shape::Arms` with no non-null arm (null-only) → `Unknown`.
- Each non-null arm classifies independently:
    - `atom.is_array` → not an object → makes the whole verdict `Unknown`.
    - `TypeExpr::InlineObject(shape)` with a non-empty `shape.pattern_keys`
      → open → `Unknown`; otherwise closed, declares `name` iff
      `shape.properties.contains_key(name)`.
    - `TypeExpr::Ref(key)` with remaining alias budget (`definitions.len()`,
      decremented per hop exactly as `arm_member` does) and a definition present:
      classify the definition body's arms recursively; a missing definition,
      an exhausted budget (cycle), or `TypeExpr::Imported` → `Unknown`.
    - Every other `TypeExpr` (`Primitive` incl. bare `object`, `Tuple`, `Union`
      already flattened, `Function`) → `Unknown`.
- Verdict: `Unknown` if any arm is `Unknown`; else `Declared` if any closed
  arm declares `name`; else `Undeclared`. Optionality never matters
  (`properties` holds optional members too).

Evidence: spec type table and "An open object, pattern-key object,
unresolved reference, or any unknown alternative prevents a warning";
`SchemaShape` has only `properties` and `pattern_keys` (`simplified-schema/lib/src/simplified/types.rs:37`);
`unresolved_as_unknown` in `call_check.rs` already turns a literal
`load_data` contract's references into `Unknown`, which satisfies "named
references retain their current unknown projection".

**R5 — Typing the base.** The base type of a selector is
`expression_type(base_spanned, &scope)` (`call_check.rs:157`) where `scope` is
`branch_scope(expr, &node.span, site.clone())` (`absence.rs:417`) for the access
node. For a dotted `Variable` step *i* (path prefix `p`), the base type is
`scope.type_of_expr(&Expr::Variable(prefix))`. After a step, the next base type
is `member_shape(shape, Member::Field(name), definitions)`; make
`member_shape` `pub(super)` → `pub(crate)` for this. Once a step's verdict is
`Undeclared` or `Unknown`, later steps of the same read are not checked (no
cascade). Evidence: spec "Once a missing step projects to unknown ... do not emit
cascading warnings"; `member_shape` already yields `unknown` for an
undeclared member.

**R6 — Access forms and subjects.** Three node kinds are checked:

1. dotted `Variable` (`server.nmae`, `fm.server.nmae`, `local.h.nmae`): the
   member steps start after the root (`server`) or after the namespace plus root
   (`fm.server`, `local.h`); subject = the authored prefix up to the step's base
   (`server`, `fm.server`, `local.h`).
2. postfix `MemberAccess { base, name }` (after `)`, `]`, a call, or `}`): steps
   are the `.`-split segments of `name`; subject of the first step = the
   authored text of `base` (`&text[base.span]`), later steps = authored text up
   to the previous segment.
3. `Index { base, index }` whose `index` is a `StringLiteral`: one step, key =
   the literal verbatim (`server["a.b"]` is the single key `a.b`, never split);
   span = the index node's span; subject as in (2).

`Index` with a number or computed index is not itself checked; its children
(base and index expression) are still walked. The finding stores the authored
subject text, so a non-path base uses its authored expression (spec). Message
(library-owned, raw text): `Member '<name>' is not declared by the known object type of '<subject>'.`
Evidence: spec "Cover equivalent access forms" and "Report one warning".

**R7 — Absence handling extends the existing classifier, no second walker.**
In `expression/absence.rs`, the single recursive `walk_static` gains a second
output. Introduce a private `struct StaticSink<'e> { variables: Vec<StaticVariableRead<'e>>, members: Vec<StaticMemberRead<'e>> }`
and make `walk_static`/`callback_reads` write to it (public
`static_variable_reads*` keep their signatures and return `sink.variables`,
still dropping lexical variable reads). A new `pub(crate) fn static_member_reads(expr: &SpannedExpr) -> Vec<StaticMemberRead<'_>>`
returns member records, **including** those whose root is a callback local
(locals are exactly what is checked). A `StaticMemberRead` holds the access
node (`&SpannedExpr`), the step index, the step's authored path prefix (for
variables) or `None`, and `handles_absence: bool`. `handles_absence` =
`scope.handles_absence flag` (new accessor on `AbsenceScope`) `||` the
facts say the path/node is guarded: for a variable path use the existing
`AbsenceScope::handles(prefix_through_step)`; for postfix/bracket nodes add
`AbsenceScope::handles_expr(&Expr)` that asks `facts.subject(expr)` then
`facts.guards_path_or_ancestor`. Because the member scope passes through
`MemberAccess`/`Index` bases unchanged (`walk_static` already does
`walk_static(base, scope, …)`), direct-position, nullable-parameter, overload,
consumed-by-call, and branch-fact rules apply to members with no new rules. A
`when=` condition is walked with `AbsenceScope::default()` (ordinary operand),
so a bare `when="server.nmae"` is not an absence check. Evidence: spec
"Preserve intentional absence handling" table; `absence.rs` module docs.

**R8 — Findings API.** New file `lib/src/markdown/compose/expression/member_check.rs`,
declared and re-exported from `expression/mod.rs`:

```rust
pub struct MemberFinding {
    pub member: String,
    pub subject: String,
    pub span: SourceSpan,       // member-name span, relative to the expression text
    pub node_ordinal: usize,    // pre-order index of the access node (R9)
    pub step: usize,
}
pub fn expression_member_findings(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<MemberFinding>;
pub fn expression_member_diagnostics(expr: &SpannedExpr, text: &str, site: &TypeScope<'_>) -> Vec<ExpressionDiagnostic>;
```

`text` is the exact text `expr` was parsed from (needed for subject slices).
Findings are returned in source order of `span.start`. The diagnostic uses
`code: UNDECLARED_MEMBER_CODE`, `Severity::Warning`, `expected_type: None`,
`actual_type: None`. Evidence: `expression_call_diagnostics` is the precedent
(`diagnostics.rs:304`).

**R9 — Node ordinals (the composition join).** The runtime evaluates the
span-erased `Expr`, which has no spans, and `SpannedExpr::erase` is a node-for-node
mirror. Define two pre-order enumerators with the same child order, in
`member_check.rs`: `spanned_nodes(&SpannedExpr) -> Vec<&SpannedExpr>` and
`expr_nodes(&Expr) -> Vec<&Expr>`. Child order per variant: `ArrayLiteral` items;
`ObjectLiteral` values; `UnaryNot`/`UnaryMinus`/`Paren` inner; `Binary` and
`Comparison` left then right; `Index` base then index; `MemberAccess` base;
`LogicalOr` left then right; `Ternary` condition, then, else; `FunctionCall`
args; `CallbackCall` args in authored order, a callback contributing its body.
`MemberFinding.node_ordinal` is the access node's index in `spanned_nodes`. A
test asserts both enumerators produce the same variant sequence and length for
a corpus of expressions (every `Expr` variant). Evidence:
`Expr`/`SpannedExprKind` mirror each other (`ast.rs` 66-188 vs 279-365) and
`erase()` is exact (module doc: "the two are produced by one grammar").

**R10 — Spike decision.** None. The spec states no performance spike; the join
adds a hash lookup per *candidate* node only (R14), so there is no measurement
to take.

### Wave 1 (parallel; disjoint files)

- [ ] **Declaration verdict** — implement R4 in `scope/projection.rs`; make
  `member_shape` crate-visible (R5); re-export `MemberDeclaration` as
  `pub(crate)` from `scope/mod.rs`.
    - Unit tests in `scope/tests.rs`: each row of the spec type table that is a
      shape question (`{name}`/`nmae`, optional member, `{name}|null`, null-only,
      union of two objects, `{…}|string`, `unknown`, bare `object`, dictionary via
      a pattern key, array arm), plus a `Ref` to a closed definition (declared and
      undeclared), an unresolved `Ref`, a self-referential alias chain, and an
      `Imported` arm.
- [ ] **Absence member reads** — implement R7 in `absence.rs`. Update the
  module doc comment of `absence.rs` to say members are classified by the same
  rules.
    - Tests in `absence.rs`'s test module: reuse the `STATIC_ROWS` style with
      member rows from the spec absence table (`server.nmae`, `|| "none"`, direct
      ternary condition, `is_null`, `min(…, 5)`, `is_empty(trim(…))`, comparison
      operand, true-branch proof, false-branch non-proof, sibling non-coverage,
      ancestor-guard coverage of a descendant read).
- [ ] **Code and registry** — implement R1 (constant, re-export, policy entry).

### Wave 2 (depends on Wave 1)

- [ ] **Member findings** — write `member_check.rs` per R2, R3, R5, R6, R8, R9.
    - For every `StaticMemberRead` that is not `handles_absence` and whose base is
      eligible (R3), compute `branch_scope(expr, &node.span, site.clone())`, type
      the base (R5), call `declares_member`, and emit a finding only for
      `Undeclared`.
    - Ordinal helper: `fn node_ordinal(expr: &SpannedExpr, node: &SpannedExpr) -> usize`
      by span-and-pointer equality over `spanned_nodes`.
- [ ] **Library tests** — new `lib/tests/l1/member_check.rs` (register in
  `lib/tests/l1/main.rs`), going through the public `expression_member_findings`
  and `expression_member_diagnostics`:
    - every row of the spec type table and absence table (control row first:
      `server.name` silent);
    - access forms: `server.nmae`, `(server).nmae`, `server["nmae"]`,
      `fm.server.nmae`, `local.h.nmae` inside a callback, `{ name: "x" }.nmae`,
      a typed function return as base, `server["a.b"]` as one key, nested
      `a.b.c` where `b` is missing (one finding, on `b`), `server.nmae.length`
      (one finding), computed key `server[key]` with an undeclared root inside the
      key (no member finding), `hosts[0].nmae` over a typed array element, and an
      undeclared root `sever.name` (no member finding);
    - lexical shadowing: a callback parameter named like a document property; two
      separate callbacks spelling `h.nmae` over unrelated types each produce a
      finding with different `node_ordinal`;
    - narrowed branches use the narrowed type; a `TypeScope::invalidate` of the
      property before the expression drops the narrowing;
    - spans: multi-segment dotted variable, postfix dotted name, bracket key, and
      a non-ASCII character earlier in the expression (byte spans are UTF-8
      offsets);
    - message text and `Severity::Warning` are identical with `strict` irrelevant
      (the API has no strict parameter);
    - R9 enumerator parity test.
- [ ] **Comment pass** — per the repo drift rule, fix any `///`/`//!` text in the
  touched files that the change makes stale (`absence.rs` header, `policy.rs`
  catalog doc).

### Phase 1 checkpoint

- [ ] `just test` and `just lint` in `darkmatter/` pass, including the untouched
  `dm.expression.undeclared_property` tests.

## Phase 2: Editor diagnostics (DMLS)

Goal: DMLS shows the shared diagnostic in every expression surface that already
shows call diagnostics.

### Rules (Phase 2)

**R11 — Single choke point.** `dmls/src/diagnostics/call_check.rs::call_diagnostics`
is called by `providers/dsl.rs:1080` (body interpolations and `when=` conditions,
typed under `guards.scope_at`), `providers/collections.rs:72` (`::map`/`::filter`
gates and callbacks, typed by the region analysis), and
`diagnostics/frontmatter.rs:1191` (`FrontmatterCalls::check`, frontmatter
interpolations and expression-typed values outside lifecycle stacks). It gains an
extra `text: &str` argument and appends `expression_member_diagnostics(expr, text, site)`
after the call diagnostics, regardless of `strict`. Each call site passes the
exact text its `SpannedExpr` was parsed from (`interpolation.text`,
`value.value`, `authored.site.text`, `expression`). Lifecycle-stack surfaces
(`diagnostics/frontmatter.rs` branch `Some(surface)` and
`stack_expression_diagnostics`) are **not** extended: their typing comes from
`lifecycle_call_checks`, and the spec says no host observation API change is
required. Evidence: the call-site grep above; spec "Lifecycle and embedding hosts
can use the shared classifier with their own typed site".

**R12 — Code constant.** `dmls/src/diagnostics/codes.rs` gets
`EXPRESSION_UNDECLARED_MEMBER` defined as the darkmatter constant
(`pub const EXPRESSION_UNDECLARED_MEMBER: &str = darkmatter::markdown::compose::expression::UNDECLARED_MEMBER_CODE;`),
following `EXPRESSION_UNKNOWN_FUNCTION`. The LSP severity comes from
`lsp_severity(found.severity)` (WARNING); `source` is the one the sibling call
diagnostics use at that site.

### Wave 1

- [ ] **Wire call_check** — apply R11/R12; update the three call sites and the
  doc comment of `call_check.rs` (the module doc says "argument-type
  diagnostics"; add that the same entry point also carries member diagnostics).
- [ ] **DMLS tests** — new `dmls/tests/l1/undeclared_member.rs` (register in
  `dmls/tests/l1/main.rs`), modeled on `dmls/tests/l1/undeclared_property.rs` and
  `expression_call_diagnostics.rs`:
    - body interpolation, multiline frontmatter expression value, `::block when`,
      `::map` callback, and `::filter` callback each report at `nmae`;
    - `server.nmae` in an unchosen ternary branch and in a callback over a
      collection that is empty at runtime is still reported (reachability
      difference);
    - Unicode text before the member (LSP columns use UTF-16 per
      `byte_range_to_lsp`) and a body edit that shifts later positions;
    - a computed `load_data` contract is silent; `expressions.strict` on/off gives
      the same member diagnostics;
    - a lifecycle-stack surface is unchanged (no member diagnostic).

### Phase 2 checkpoint

- [ ] `just test` and `just lint` in `darkmatter/` pass.

## Phase 3: Composition transport

Goal: composition warns exactly for selectors the evaluator evaluated, once per
authored site, under the prepared effective schema.

### Rules (Phase 3)

**R13 — Architecture: static candidates joined with runtime reach.** The
document-level precheck (`call_precheck::check_document`, run once per pipeline
node, before any expression is evaluated, with `DeclaredTypes::for_schema` of
the prepared effective schema, `pipeline/mod.rs:1045`) already walks every
authored composition surface with the right typed scope (frontmatter leaves,
body interpolations under `::block when` guards, region locals via `BodyScopes`,
transclusion and page-block conditions). It additionally calls
`expression_member_findings` next to `check_expression_calls` (`call_precheck.rs:618`
and the sibling surface walkers) and stores each finding as a *candidate* in a
new `ComposeReport` field. The evaluator separately records which candidate-kind
nodes it evaluated. A candidate becomes a warning only when its site key and
ordinal were reached. Because candidates come from the prepared effective
schema, no provisional-schema decision can happen in frontmatter's repeated
passes. Evidence: spec "Carry the site's declared types ... into observation",
"Frontmatter's repeated passes must not report ... using a provisional schema",
"Do not use a static scan to emit warnings for composition branches that never
ran" (a static finding alone never emits).

**R14 — Runtime reach record.**

- `MissingRootObserver` (`expression/absence.rs`) gains
  `const OBSERVES_MEMBERS: bool = false;` and
  `fn member_read(&mut self, _node: usize) {}` and `fn finish_expression(&mut self, _root: &Expr) {}`.
  `()` and `Vec<MissingRoot>` keep the defaults, so unobserved evaluators and
  every surface not migrated behave exactly as today.
- `evaluate_expr` (`expression/mod.rs:599`) calls
  `observer.member_read(expr as *const Expr as usize)` (an address, not a
  pointer, so observers stay `Send`) when `O::OBSERVES_MEMBERS` and the node is
  `Expr::Variable` whose path contains `.`, `Expr::MemberAccess`, or
  `Expr::Index` whose index is a `StringLiteral`. `evaluate_observed` calls
  `finish_expression(expr)` after evaluation whatever the result, so a failing
  expression still reports what it reached first.
- New `pub(crate) struct Observation { pub roots: Vec<MissingRoot>, reached: Vec<ReachedMember>, addresses: Vec<usize> }`
  implements the observer with `OBSERVES_MEMBERS = true`; `finish_expression`
  converts the recorded addresses to pre-order ordinals by one walk of
  `expr_nodes(root)` (R9) and appends `ReachedMember { ordinal, span: None }`
  (deduplicated per expression evaluation). Per-item repetition inside a
  callback body therefore costs one `Vec` push per node visit and one walk per
  expression, and an expression with no candidate reads pays nothing extra.
- `Evaluator` (`interpolation/evaluator.rs`): `observing_missing_roots()` now
  installs an `Observation` (same RefCell). `evaluate()` (line ~262) and the
  bare-variable fast path in `eval()` (line ~306, where `Expr::Variable("a.b")`
  bypasses `evaluate`; record ordinal 0 there for a dotted name) feed it.
  `locate_missing_roots(mark, span)` additionally stamps the span on reached
  members recorded since the matching member mark (add
  `reached_member_mark()`), and add `take_reached_members()` and
  `reached_members_since(mark)` mirroring the root accessors.
- Surfaces that pass a bare `Vec<MissingRoot>` as the observer today (page-block
  `engine.rs:70,157`, transclusion `engine.rs:686`, collection
  `engine.rs:339`, the three `frontmatter_shell_expansion.rs` helpers at 1791,
  2029, 2089) switch their local to `Observation` and drain `roots` where they
  drained the `Vec`.

**R15 — Candidates and the site key.** New file `lib/src/markdown/compose/member_sites.rs`:

```rust
pub(crate) struct MemberSiteKey { pub scope: Option<String>, pub start: usize }
pub(crate) struct MemberCandidate { key: MemberSiteKey, ordinal: usize, warning: ComposeWarning }
pub(crate) struct MemberCandidates { /* HashMap<(MemberSiteKey, usize), ComposeWarning> */ }
```

`scope` is the frontmatter key-path label (`path_label` in `call_precheck.rs`) or
`None` for body, page-block, transclusion, and collection expressions. `start`
is the authored byte offset in the document's loaded source of the construct
start: the `{{` for an interpolation, the first byte of the condition text for a
`when=` value, the first byte of the expression text for a collection gate or
callback. Both sides compute it through one helper
`authored_expression_start(...)` that reuses what each side already has:
precheck projects `expression_start`/`condition_start` through
`BodyOrigin::project` (body) or `FrontmatterExpressionLocation::project`
(frontmatter, same type the runtime uses in `frontmatter_interpolation.rs`); the
runtime projects the `MissingRoot`-style span of the `{{ … }}` it evaluated
through the surface's `BodyOrigin`/frontmatter location exactly as `locate` does
in `inline/interpolation.rs:199`. If either side cannot project, the site has no
key and produces no member warning (silence, never a guessed line; spec "unknown
authored positions remain absent"). A repeated `::map` copy projects to the one
authored region site, which is how iterations and repeated inclusion dedupe.

**R16 — Warning construction.** The candidate's `ComposeWarning` is built in the
precheck with the same machinery as call warnings (`advisory_warnings` in
`call_precheck.rs`): `source = ComposeWarning::EXPRESSION_SOURCE`, `code =
UNDECLARED_MEMBER_CODE`, `diagnostic = Some(finding.at(document, position))` with
position from the authored member-name offset (projected like the call
advisories; extract that projection into a shared helper rather than copying it),
`identity = WarningSubject::Expression { document: None, scope, origin:
ExpressionOrigin::Authored(authored member-name range) }` (attributed to the
document at merge, like call warnings), `stage` = the stage name the surface
already uses. Message = the library message with its trailing `.` removed,
escaped with `Prose::escape_text`, followed by the same location text
call-precheck uses (` at {doc}:{line}`, ` in {doc}`, ` at line {n}`, or none),
followed by `.`. Identity is per document plus authored member span, so separate
sites with identically named locals keep separate warnings, and repeated
reaches of one site collapse in `ComposeReport::add_warning`'s existing
`WarningIndex`. Evidence: `WarningSubject::Expression` doc ("once per source
document no matter how many times it is reported"); `call_precheck.rs:314-328`.

**R17 — Emission and policy.** A reach is converted to a warning immediately
after the expression that reached it is evaluated, by
`ComposeReport::observe_reached_members(options, key_for, reached)`: it is a
no-op when `options.defer_expression_failures` is set (non-terminal discovery
passes never report; same flag `unknown_identifiers::undeclared_warnings` honors),
and otherwise looks up `(key, ordinal)` and calls `add_warning` with the candidate
warning. Each surface routes the resulting warning to the same checkpoint its
other warnings reach, so `BlockingPolicy` stops the document before the next
stage/command/child:

- body interpolation: through the `guard` warning stream in
  `interpolation::interpolate_text_selected` (`inline/interpolation.rs:222-236`);
- frontmatter interpolation and expression-typed frontmatter: through the
  `add_frontmatter_candidates` call sites (`pipeline/mod.rs:478, 607, 635`) and the
  policy check that follows them;
- page-block, transclusion, and collection conditions: through the `(self.stop)(self.report)`
  / `blocking_guard` checks that already follow `add_unknown_root_candidates`.

Candidate storage (`member_candidates` on `ComposeReport`, `pub(crate)`) is
*never merged from a child report*: each document's pipeline decides its own
sites before its report merges, like `unknown_root_candidates`; merged child
*warnings* keep their child document through `attribute_to`. Severity stays
Warning; the policy only changes the outcome.

**R18 — Surface coverage.** Covered (they observe roots today and have a typed
precheck walker): frontmatter interpolation, expression-typed frontmatter
(`frontmatter_shell_expansion.rs`), body interpolation (including the directive
target pass: the target pass keeps its existing `drop(evaluator.take_missing_roots())`
at `inline/interpolation.rs:114` and also drops its reached members, so a target
already handled by nullable-target diagnostics gets no second warning),
`::block when` (page blocks), `::file`/`::code` `when=` (transclusion),
`::map`/`::filter` gates, callbacks, and body surfaces inside region bodies.
**Not covered:** shell-template holes (`shell_expansion/template/fill.rs`
evaluates through a `HoleEvaluator` that records no roots; `grep MissingRoot`
finds nothing in `shell_expansion/`) and lifecycle surfaces; "preserve each
surface's existing observation policy" forbids adding observation there. The
shell-template and lifecycle gaps are documented in Phase 5.

### Wave 1 (parallel; disjoint files)

- [ ] **Observer plumbing** — R14 in `expression/absence.rs`, `expression/mod.rs`,
  `interpolation/evaluator.rs`. Unit tests next to the evaluator: a dotted
  variable, a postfix access, a bracket key each report their ordinal; an
  unchosen ternary branch and a short-circuited `||` operand report nothing;
  a callback over `[]` reports nothing and over three items reports each
  ordinal once; `()` observer unchanged (existing evaluator tests pass).
- [ ] **Candidate store and key** — R15 in `member_sites.rs`, the
  `ComposeReport` field and `observe_reached_members` (R17) in
  `context/report.rs`; register the module in `compose/mod.rs`. Unit tests:
  key equality, no key when projection is absent, dedup of repeated reaches,
  `defer_expression_failures` no-op, candidates not merged from a child.
- [ ] **Precheck candidates** — R13/R16 in `call_precheck.rs`: every place that
  calls `check_expression_calls`/`checked`/`rejected_condition` also computes
  member findings for that expression under the same scope and records a
  candidate with the surface's key (R15). Do not change the existing call
  warnings or the rejection order. Extract the shared authored-position
  projection helper used by `advisory_warnings`.

### Wave 2 (depends on Wave 1; one task per surface, parallel)

Each task: migrate the surface's observer to `Observation`, compute the runtime
key with the surface's own projection (R15), call `observe_reached_members`
(R17), and make the blocking checkpoint see the warning. Each adds a surface test
in Phase 4's file (listed there) — write the test in the same task.

- [ ] **Body interpolation** — `inline/interpolation.rs` (`rewrite_body`, target
  pass, `resolve_pending`), `interpolation/rewrite.rs:369` (`locate_missing_roots`
  caller).
- [ ] **Frontmatter interpolation** — `frontmatter_interpolation.rs` (evaluator at
  ~1073, drains at ~1168/1251), `pipeline/mod.rs:478, 635`. Pass-1/pass-2
  repetition must yield one warning (identity dedup) and must not read the
  schema.
- [ ] **Expression-typed frontmatter** — `frontmatter_shell_expansion.rs`
  (1791-2105), `pipeline/mod.rs:607`.
- [ ] **Conditions** — `page_blocks/engine.rs` (70, 157), `transclusion/engine.rs:686`
  via `conditions.rs` (`ConditionSite::evaluate`/`evaluate_condition_observed`
  take the new observer type).
- [ ] **Collections** — `collection_blocks/engine.rs:339` (gates and callbacks) and
  region-body body surfaces; lexical item identity comes from the authored
  region site, so iterations collapse.

### Phase 3 checkpoint

- [ ] `just test` and `just lint` in `darkmatter/` pass; the unchanged
  `undeclared_property` tests still pass; `compose_perf.rs` (existing) still passes.

## Phase 4: Policy, CLI, and acceptance tests

Goal: pin every spec acceptance criterion through public APIs. Test in-process;
spawn the CLI only to prove wiring (controls plus a few cases).

### Rules (Phase 4)

**R19 — Test files.** Library composition tests in a new
`lib/tests/l1/compose_member_warnings.rs`; CLI wiring in a new
`cli/tests/l1/compose_member_warnings.rs`; editor/library agreement in
`dmls/tests/l1/undeclared_member.rs` (extend from Phase 2). Register each in its
directory's `main.rs`. Use the repo test toolkit and L1 fixtures; no terminal or
browser is opened. Each test builds the fixture with one edit per case and
includes a **control** (the unedited fixture with the correct spelling `h.name`
composes with zero member warnings) so every edit is known load-bearing.

**R20 — Field-shape matrix for the one external input, `load_data` data files.**
The only configuration-like input exercised is the YAML data file; its loader is
the existing `load_data` path, not new code. Default outcome (existing): a
malformed file is an existing located error. Exceptions this feature defines:
an *empty* typed input (`[]`) → no composition warning (nothing evaluated),
DMLS still warns; rows with a missing `name` key do not change the verdict
(declaration check only).

### Wave 1 (parallel)

- [ ] **Library composition tests** — in `compose_member_warnings.rs`, through the
  compose public API (the same entry the neighbouring `compose_call_precheck.rs`
  uses):
    - AC1: three rows → one warning ranged on `nmae`, default policy composes;
      empty file → none; computed contract → neither warning nor inferred names;
    - AC2: `server.nmae` in dot, `(server).nmae`, `fm.server.nmae`, bracket forms;
      `server.name` silent including an optional unset member; typed function
      return and literal object as bases;
    - AC4: nested dotted path, `server["a.b"]`, computed key with an undeclared
      root inside (root warning only), typed array element then misspelled member;
    - AC5: two separately authored callbacks with `h.nmae` → two warnings;
      iterations add none; `local.h.nmae`; shadowing by a callback parameter; a
      repeated include of one file → one warning; two different files → two;
    - AC6 surfaces, each with a positive and an unreached case: frontmatter
      interpolation, body interpolation, expression-typed frontmatter, `::block
      when`, `::file when`, `::map`/`::filter` gates, callbacks, and region body;
      body edits before the site, multiline frontmatter, Unicode before the
      member verify the authored line/column;
    - reachability: unchosen ternary branch, short-circuited `||` operand, and an
      empty collection produce no warning;
    - AC7: a later failure (a rejected call after a member warning) keeps the
      earlier member warning;
    - a discovery (`defer_expression_failures`) pass reports none.
- [ ] **CLI tests** — in `cli/tests/l1/compose_member_warnings.rs`: default
  `md compose` succeeds and prints the warning on stderr for text output;
  `--deny dm.expression.undeclared_member` fails with the shared diagnostic and
  writes no output; `--deny all` likewise; `--deny all --allow` of an unrelated
  recovery still blocks; structured (JSON) output carries `code`, `severity`,
  and the located position with an empty-of-diagnostics stderr for data output;
  `md compose --help` lists the new code.
- [ ] **Agreement test** — in `dmls/tests/l1/undeclared_member.rs`, one fixture per
  surface compares the DMLS diagnostic to the composition warning's `diagnostic`
  for code, message, severity, and member-name span, and separately asserts the
  reachability difference.

### Phase 4 checkpoint

- [ ] `just test` and `just lint` in `darkmatter/` pass. `just test-l2` is not
  required (no real-terminal behavior was added).

## Phase 5: Documentation and skill

Goal: the current-behavior docs describe the feature; none refers to the spec.

### Rules (Phase 5)

**R21 — Doc audience and placement.** The reader is a developer with no
repository experience: each page leads with what the author can do, gives one
compact example per rule, and uses a Mermaid diagram for the composition flow
(candidates → reached → warning). README stays an overview (no change). No doc
links to or names a feature/fix directory. Behavior that is not covered
(shell-template holes, lifecycle surfaces, host-global bases) is stated plainly as
not checked.

### Wave 1 (parallel; disjoint files)

- [ ] **Expressions topic** — `docs/topics/darkmatter-expressions.md`: new section
  "Misspelled members" (what is reported, the conservative union rule with the
  spec's type table as examples, access forms, absence handling table, one
  warning per authored site, runtime vs editor reachability, the diagram); update
  the existing sentence near line 114 and the diagram node near line 172 that
  describe only root reads.
- [ ] **DMLS diagnostics** — `dmls/docs/diagnostics.md`: add the code to the three
  code tables (lines ~233, ~361, ~400) with the editor behavior (all branches,
  independent of `expressions.strict`, lifecycle stacks not covered).
- [ ] **CLI policy** — `docs/cli/policy.md`: name the code in the "diagnostic code"
  bullet (line ~116) and add a `--deny dm.expression.undeclared_member` example
  with its expected outcome.
- [ ] **Darkmatter skill** — `.claude/skills/darkmatter/expressions.md` (member
  classification, `expression_member_findings`, the join), `compose.md`
  (observation surfaces and the not-covered list), and `dmls.md` (the new
  diagnostic and its choke point `call_check::call_diagnostics`). Keep
  `SKILL.md` under its size limit, linking to those files.
- [ ] **Comment and doc drift sweep** — re-read every `///`/`//!` in files this plan
  edited; fix drift in place (code is correct, comments are wrong).

## Phase 6: Final verification

- [ ] `just lint` and `just test` in `darkmatter/` pass from a clean tree;
  `git status` shows only the files this plan names plus new tests.
- [ ] Re-run the spec's five-line scenario (AC1) through `md compose` by hand with
  a two-row `hosts.yaml` and confirm one warning, exit code 0; then with
  `--deny dm.expression.undeclared_member` and confirm a non-zero exit.
- [ ] Record any departure from the spec in an `implementation-log.md` in this
  feature's directory (the spec is not edited); the terminal state is
  "implementation complete, ready for review". Do not move the feature to
  `_completed`.

## Closure check (planning)

Deferral search done: the plan contains no "decide during implementation"
or conditional-on-trace tasks (`docs/dependencies.md` needs no change: no crate is added). The one place reading code could not
settle detail — the exact warning-checkpoint call in each surface — is covered
by R17's named checkpoint per surface plus a per-surface blocking test in Phase 4,
so an implementer who wires the wrong checkpoint fails a test rather than
guessing.
