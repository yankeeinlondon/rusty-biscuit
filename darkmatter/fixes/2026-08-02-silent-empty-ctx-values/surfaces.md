# `ctx.*` Evaluation Surface Matrix

Phase 1 deliverable (Spike B) for `2026-08-02-silent-empty-ctx-values`. Paths are
relative to `darkmatter/lib/src/markdown/compose/`; line numbers are as of
`914ac2188` and will drift. Phase 3 implements against this matrix, and the
fatality characterization tests extend it.

## How a lookup becomes `""` today

- `EvaluationLookup` (`expression/mod.rs:185`) has one read channel:
  `fn get(&self, path) -> Option<Value>`. **It has no error channel**, so a
  checked lookup needs a new trait method (see `error-taxonomy.md`).
- `Expr::Variable` → `lookup.get(path).unwrap_or(Value::Null)`
  (`expression/mod.rs:391`); `Null` → `""` in `Evaluator::eval`
  (`interpolation/evaluator.rs:262`), `scalar_string` (`expression/mod.rs:340`),
  `EffectiveState::get_string` (`context/effective_state.rs:254`), and
  `FrontmatterSeedState::get_string` (`frontmatter_interpolation.rs:141`).
- `is_valid_context_variable` answers only "is the name in the catalog"
  (`effective_state.rs:361`, `frontmatter_interpolation.rs:167`), so a cataloged
  key from an uncaptured group produces no warning at all.
- No production code calls `ComposeContext::capture_requirements()` or
  `extend_with_evidence` today.

## Lookup implementations

| Lookup | Location | `ctx.*` source | Reaches `capture_requirements()`? |
|---|---|---|---|
| `EffectiveState` | `context/effective_state.rs:352` (`get` `:213`, `get_context_value` `:290`) | merged `data["ctx"]` → `ComposeContext::get` → legacy date fields | yes, via `self.context` |
| `ResolvingLookup` | `context/effective_state.rs:394` (impl `:411`) | delegates to `EffectiveState` | yes |
| `FrontmatterSeedState` | `frontmatter_interpolation.rs:56` (impl `:96`) | `ComposeContext::get_effective` (`:105`) | yes, owns a `ComposeContext` |
| `LayeredLookup` | `subtree.rs:152` (impl `:213`) | globals → `EffectiveState::get` | yes |
| `ShortcutLookup` → `CtxLookup` | `conditions.rs:345` → `expression/ctx.rs:40` | lazy per-group capture, private cache | **no**: tracks its own `HashSet<ContextGroup>` |
| `ResolutionContext.ctx_values` | `expression/resolve_ctx.rs:70` | plain `Map` from `context.as_object()` | **no** (function-internal reads only, such as the `agent` fallback at `:195`; not an authored `ctx.*` lookup, so out of scope) |

## Surface matrix

"FF-gated" means that today, whether an evaluation error is fatal depends on
`fail_fast`.

| # | Surface | Entry → evaluating call | Lookup | FF-gated today | Error carrier | Phase 3 action |
|---|---|---|---|---|---|---|
| 1 | FM interpolation **pass 1** | `pipeline/mod.rs:186` → `interpolate_frontmatter` (`frontmatter_interpolation.rs:351`) → `rewrite_value` `:605`/`:681` → `interpolate_value` (`interpolation/rewrite.rs:254`) | `FrontmatterSeedState` | Whole value: always fatal (`rewrite.rs:264-276`). Mixed text: `fail_fast \|\| is_authoring_fatal()` (`rewrite.rs:165`). | `MarkdownError::Interpolation{key:Some}` | Checked lookup + `is_authoring_fatal` → fatal in both shapes |
| 2 | FM interpolation **pass 2** | `pipeline/mod.rs:315` (only when FM shell produced replacements) | `FrontmatterSeedState` | same as #1 | same as #1 | same as #1 |
| 3 | Schema-adjacent composed values | `schema_validation.rs:212`, `verify_projection_stability:414` | none: consumes pass-1/2 output | n/a | `SchemaValidationFailed` | None needed: covered because #1/#2 fail first. Today a blanked `""` is validated as final (`value_pending_composition:1236`). |
| 4 | FM shell expansion `$()` plain pipeline | `pipeline/mod.rs:290` → `frontmatter_shell_expansion.rs:1278` | none: text already interpolated by #1 | always fatal | `MarkdownError::ShellExpansion` | Covered by #1 |
| 5 | Page-block `when=` | `page_blocks/engine.rs:17` → `conditions::evaluate_condition` `:40` (nested `:98`) | `ResolvingLookup` | always fatal (`?`) | `ConditionError` → `PageBlockError::Condition` → `MarkdownError::PageBlock` | Checked lookup; the error must carry the typed cause, not only a string |
| 6 | Transclusion `when=` | `transclusion/engine.rs:477` (`prepare_block_transclusions`) → `:582` | `ResolvingLookup` | always fatal (serial prepare step, outside the tolerant resolve path) | `TransclusionError::ConditionEval/Parse` → `MarkdownError::Transclusion` | Checked lookup; same typed-cause note as #5 |
| 7 | Body interpolation, plus whole-value `::file/::code/::url` targets | `pipeline/phases.rs:49` → `inline/interpolation.rs:22` → `directive_targets.rs:346`/`:370`, then `interpolate_text` (`rewrite.rs:97`) | `ResolvingLookup` + presentation values | Text: `fail_fast \|\| is_authoring_fatal`; otherwise a warning and the `{{…}}` is left in place. Targets: always fatal. A `Null`/`""` target becomes `Absent` (`directive_targets.rs:378`). | `MarkdownError::Interpolation{key:None}` | Checked lookup + `is_authoring_fatal` |
| 8 | `$()` ternary condition / branches | `frontmatter_shell_expansion.rs:1534` (condition), `:1712` (value branch), `:1771` (text branch) | `FrontmatterSeedState` | hard-coded `fail_fast=true` → always fatal; `rewrite.warnings` (including ctx typo warnings) **dropped** | `ShellExpansionError::ParseDirective` (**stringly**: `format!("… failed: {err}")`) → `MarkdownError::ShellExpansion` | Checked lookup. **Typed-cause gap:** the `ExpressionError` is flattened into a string, so this wrapper needs a typed source. |
| 9 | Bare-name ctx fallback (`when="repo"`) | `EffectiveState::get` `:235` (`get_nested_value(path).or_else(get_context_value(path))`); `ShortcutLookup::get` `conditions.rs:368` | `EffectiveState` / `ShortcutLookup`; **not** `FrontmatterSeedState` | silent | none | See Ruling 8 (open). `ContextRequirements` scans only literal `ctx.KEY` (`capture/groups.rs:141`), so a bare name never causes its group to be captured. |
| 10 | Recursive **local** transclusion | `engine.rs:1422` `render_markdown_transclusion` → `get_or_compute_compose` `:1478` → `options.clone()` `:1484` → `run_compose_pipeline_internal` `:1519` | child runs #1–#9 on the parent's `ComposeContext` (Arc clone) | **Child errors tolerated** into notice + `ComposeWarning("transclusion")` unless structural or `fail_fast` (`phases.rs:376-410`) | whatever the child raises | **Must add** missing-capture/invariant causes to the structural set, or they are swallowed. Phase 4: requirement handoff before the cache key. |
| 11 | Recursive **remote** transclusion | `engine.rs:1028-1116` (`options.clone()` `:1105`); not cached via `get_or_compute_compose` | same as #10 | same as #10 (`RemoteFetchFailed` is structural) | same | same as #10 |
| 12 | Standalone `evaluate_condition_against` | `conditions.rs:257` → `evaluate` `:281` | `ShortcutLookup` → `CtxLookup::resolve_ctx` | every error returned; no warnings | `ConditionError::{Parse,Eval}` | Phase 2 parity: unknown → `None`; captured group missing a cataloged key → invariant error; capture diagnostics (dropped at `ctx.rs:61`) unchanged |
| 13a | Pre-flight `interpolate_frontmatter_best_effort` | `preflight/collect.rs:543` → `frontmatter_interpolation.rs:390` | `FrontmatterSeedState` over the **non-upgraded** `options.context()` | per-key errors swallowed; results and warnings discarded | none | Ruling 3: stays tolerant |
| 13b | Pre-flight ternary branch discovery | `collect.rs:585` → `frontmatter_shell_expansion.rs:1464` → `:1496` | `FrontmatterSeedState` over the non-upgraded context | fatal (`fail_fast=true`) | `ShellExpansionError` | **Conflict with Ruling 3:** under a non-upgraded ambient context this pass would now raise missing-capture where the terminal pass would not. Swallow per-expression missing-capture here, or upgrade before pre-flight. See the notes below. |
| 13c | Shell-command discovery inline compose | `collect.rs:228-251`: `options.clone().only([FmInterp, TextReplacement, Interpolation])` → `compose_with` | full pipeline on a clone (gets the root upgrade at `pipeline/mod.rs:36`) | inherits `fail_fast` (`?`) | as #1/#7 | Behaves like the terminal pass; Ruling 3 wants tolerance here |
| 13d | Deferred schema verdicts | `schema_validation.rs:240,285,425,481` | no ctx evaluation | verdict suppressed | none | none |
| 14 | Reference graph `when=` (`md graph`, not compose) | `markdown/reference/graph.rs:354-370` | `ResolvingLookup` over a non-upgraded `options.compose.context()` | `.unwrap_or(false)` swallows | none | Stays tolerant: a passive graph inspection is not a compose verdict. Document it. |
| 15 | Subtree compose (event-time) | `subtree.rs:54` → `interpolate_value` `:474`; strict pre-pass `:490` | `LayeredLookup` | strict: `fail_fast=true`; lenient: as #7 | `MarkdownError::Transform` / `Interpolation` | Inherits the checked path via `EffectiveState` |

## Unknown-variable walker (`collect_context_warnings`)

- Defined at `interpolation/evaluator.rs:365` (walk `:384-427`). It inspects
  only `Expr::Variable` names with a `ctx.` prefix and checks their first
  segment. It does **not** descend into array or object literals.
- It emits `ComposeWarning(stage, "unknown context variable '…'" + did-you-mean)`
  only when `!is_valid_context_variable`.
- Callers: `rewrite.rs:139`, `rewrite.rs:272`, and `conditions.rs:182` (via page
  blocks `:36/:94` and transclusion `:576`).
- Warnings are lost in: `$()` ternary (#8), pre-flight (#13a), and
  `evaluate_condition_against` (#12, never called).
- Ruling 6: this walker gains **no** missing-group diagnostic. Unknown keys
  have no owning group, so the checked lookup must return "unknown", which
  keeps today's `None` behavior and avoids duplicate emission.

## Surfaces whose lookup cannot reach `capture_requirements()` today

1. **`CtxLookup` / `ShortcutLookup` (#12).** They own no `ComposeContext`, so
   Phase 2 needs a classification adapter that reuses
   `ContextGroup::for_key` + `context_variable_descriptors()`.
2. **`FrontmatterSeedState` (#1, #2, #8, #13a/b).** It *can* reach the context,
   but it reads through `get_effective`, which has no requirements check, so it
   needs the adapter too. It is not an `EffectiveState` and has no bare-name
   fallback.
3. **`ResolutionContext.ctx_values` (#16 in the survey).** Out of scope: it is a
   function-internal read, not an authored lookup.

## Cross-cutting findings Phase 2–4 must handle

- **Typed-cause gaps.** #5/#6 (`ConditionError`) and #8 (`ShellExpansionError`)
  carry string messages. Spec §2 requires a typed cause reachable from
  `MarkdownError`.
- **Transclusion tolerance (#10/#11).** Without a structural classification,
  a child's missing-capture error becomes a notice-filled section plus a
  warning. That is a partially composed document, which violates spec §2.
- **Pre-flight epoch.** #13a/#13b/#14 evaluate against the context *before*
  the root upgrade (pre-flight does not enter `run_compose_pipeline`). This
  already happens today. Once lookups are checked, these passes must tolerate
  missing-capture, or pre-flight must receive the upgraded context.
- **Cache identity.** `context_hash` (`cache/hashing.rs:103`) hashes values and
  env but not `capture_requirements()`. That is sufficient only while every
  captured group projects all of its keys (a captured group's `null` keys are
  present in the map, while an uncaptured group's keys are absent). The child
  key (`engine.rs:1444`) uses the parent phase's hoisted `PhaseStateIdentity`
  (`phases.rs:331`), which is taken before any child-introduced group exists.
  See the Spike A decision in `plan.md`.
- **Merged `data["ctx"]`.** `EffectiveState::get_context_value` reads
  user-merged `data["ctx"]` *before* the context. Ruling 1 requires the
  captured-group check to run first.
