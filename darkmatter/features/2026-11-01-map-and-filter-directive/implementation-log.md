---
spec: /Volumes/coding/wt/rusty-biscuit/feat-map-and-filter/darkmatter/features/2026-11-01-map-and-filter-directive/spec.md
plan: darkmatter/features/2026-11-01-map-and-filter-directive/plan.md
implemented_by: claude/sonnet
started_phase: 1
source_files_during_phase_1:
  - darkmatter/cli/src/commands/render.rs
  - darkmatter/lib/src/markdown/compose/body_origin.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/binding.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/parse_utils.rs
  - darkmatter/lib/src/markdown/compose/perf.rs
  - darkmatter/lib/src/markdown/compose/pipeline/operations.rs
  - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
  - darkmatter/lib/src/markdown/compose/type_tests.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/expression/iteration.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/tests/l1/expression_scopes.rs
  - claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
  - darkmatter/lib/src/markdown/cleanup/protect.rs
  - darkmatter/lib/src/markdown/compose/block_pairs.rs
  - darkmatter/lib/src/markdown/compose/call_precheck.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/types.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/binding.rs
  - darkmatter/lib/src/markdown/compose/expression/call_check.rs
  - darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/expression/iterable.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/prepared.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/lexical.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
  - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/guards.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/parse_utils.rs
  - darkmatter/lib/src/markdown/compose/policy.rs
  - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
  - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
  - darkmatter/lib/tests/l1/binding_contract.rs
  - darkmatter/lib/tests/l1/collection_analysis.rs
  - darkmatter/lib/tests/l1/expression_type_scope.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/l1/scope_site_conformance.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/expression/iteration.rs
  - darkmatter/lib/src/markdown/compose/expression/binding.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/body_origin.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs
  - darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs
  - darkmatter/lib/src/markdown/compose/inline/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/site_scope.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/nested.rs
  - darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
  - darkmatter/lib/tests/l1/collection_execution.rs
  - darkmatter/lib/tests/l1/collection_effects.rs
  - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine
decisions_for_review:
  - |-
    Where the reserved root `local` is registered. The plan lists the `local` descriptor among Phase 1 contracts, but registering it changes how a bare `local` document key classifies in every existing document, so I deferred it to the lexical-frames task in Phase 2. `ScopedLookup` already protects reserved roots and declared globals and resolves `local.<name>` itself. Alternative: register it now (one descriptor plus the host-registration rejection) at the cost of changing existing-document behavior before any local can exist. If the reviewer prefers it now, add the descriptor in `expression/catalog/roots.rs`; nothing else depends on the timing.
  - |-
    Scaffolding scope for the iteration budget, options builders, and diagnostics. I defined the shared value types (limits, usage, iteration path, typed budget error, `CollectionError`, diagnostic code constants) but left the budget authority, `ComposeOptions` limit and recovery builders, the located-diagnostic span/iteration-path fields, and code registration in policy/report/DMLS registries to the phases the plan assigns them (3 and 5). Alternative: add all of them now. That would put unexercised behavior and registry entries (which parity tests assert are emitted) in the foundation. Nothing needs redoing if the reviewer agrees; otherwise those items move forward into Phase 1.
  - |-
    `analyze_collection_blocks` is not defined yet. The plan fixes its signature, and the option/result types it needs exist, but a Phase 1 body could only return an empty analysis, which would silently report "no regions" for documents containing `::map`. I left the function to the phase that implements the scanner. Alternative: a stub returning an error. No consumer exists, so nothing is affected.
  - |-
    A drive-by lint fix in `darkmatter/cli/src/commands/render.rs`: `run_render` has eight arguments and fails `clippy::too_many_arguments` under the repository's current toolchain, which blocked `just lint` for the whole area. I added `#[allow(clippy::too_many_arguments)]` (the pattern used elsewhere in the repo). Alternative: bundle the arguments in a struct, a larger refactor unrelated to this feature.
  - |-
    `ComposeOperation::CollectionBlocks` is registered (index 16, default on, between page blocks and interpolation, with a perf metric and `ComposeStage` variant) but no pipeline arm executes it, so enabling or disabling it currently has no effect. The `--disable collection-blocks` CLI spelling is Phase 5. The new `ComposeStage` variant also extends the public stage list that Claudine's perf tree reads; Claudine's perf tests pass unchanged.
  - |-
    Phase 2 callback bodies parse in interpolation mode, so `||` inside a callback body is the fallback operator, not logical OR. The plan did not say which mode callbacks use. Alternative: condition mode, which would make `a || b` a logical OR in predicates. Reviewing: if predicates should use condition mode, `parse_callback[_spanned]` needs a mode parameter and the collection header parser must pass condition mode for `filter`/`while`/`until`; callers elsewhere are unaffected.
  - |-
    `local` is now a reserved root (`RootMembers::LexicalBindings`). A document or host that already has a frontmatter key named `local` can no longer read it as a bare `local.*` path (`fm.local` still works); a host registering a global named `local` gets the existing reserved-root registration error. Alternative: not reserving it, which would let a document key shadow lexical bindings. Two existing tests used `local` as an ordinary key and were renamed (dmls `expression_scopes`, claudine prewarm classification).
  - |-
    The public block scans (`scan_darkmatter_blocks`, `scan_shell_block_commands`) stay page/shell-only; collection regions are exposed by the new `collection_blocks::parse_collection_regions` and lenient scan. Alternative: add a collection kind to the public `BlockKind`, which would force every DMLS consumer to handle it now. Phase 5 can add it when DMLS projects scopes.
  - |-
    `sort-order` is accepted quoted or bare as long as the value is exactly `asc` or `desc` (the spec shows it quoted but only demands exactness). If bare values should be rejected, change the check in `collection_blocks/parser.rs`.
  - |-
    `::end-block` keeps its old matching rule (an unquoted closer closes any top page or shell region regardless of container path); container-path matching applies to collection closers only, to avoid changing existing page/shell behavior. A quoted `> ::end-block` now closes only a quoted shell block. Alternative: path matching for shell blocks too, which could reject documents that pair today.
  - |-
    The expression `Diagnostic` span/iteration-path fields from plan Rule 22 were not added in Phase 2 (they belong to the report-json work with runtime iteration paths). Authored spans currently ride on `CollectionError` sites and `RecoveredSource`. Nothing else depends on the timing.
  - |-
    A field-pattern local `{name}` and an identifier-pattern member `item.name` do not share narrowing facts (each has its own subject); sharing works between differently named identifier patterns and between callbacks binding the same field names. Bridging needs a canonicalization in `scope/projection.rs`'s subject type. Alternative: add it later if acceptance tests need it.
  - |-
    Collection regions and page blocks run in one traversal. When a body holds an active `::map`/`::filter`, the collection stage also evaluates every `::block when` (inside and outside the collections) and the page-block stage steps aside, because a gate inside a repeated body must see its iteration. Bodies without collections are untouched. Alternative: keep two stages and re-run page blocks per copy, which would rediscover directives in concatenated text (the plan forbids it). Reverting means restoring the page-block stage and giving up scoped gates.
  - |-
    Later stages find an iteration's locals through a `SiteLookup` that is told the offset of each site (a new `SiteHook`), rather than through a new evaluator argument. It is a new `EvaluationLookup` implementation listed in the lookup-parity inventory. Alternative: split the scanned text per copy, which would break Markdown-aware scanning (fences). Only the interpolation, directive-target, shell-expansion, shell-block and transclusion `when=` sites use it.
  - |-
    Byte charging is "once per enclosing expansion": every copied or inserted byte is charged once for each collection invocation that encloses it, which equals the spec's nested double charge. Inserted values (interpolation, directive targets, shell output, child output) are charged at insertion using the recorded owner depth of the copy. Alternative: per-invocation ledgers merged on close; that is the `ExpansionUsage` cache-replay model Phase 4 needs, and can be layered on `ScopeRange::owners`. Child output charged is the replacement written into the parent, not the child's own internal expansion.
  - |-
    The budget accessor on `EvaluationLookup` (plan Rule 5) was not added: nothing in this phase evaluates expression-level map/filter, and nested `as_markdown` composition shares the budget through the pipeline runtime. `EvaluationSession` has its own default-limit budget (`with_iteration_limits`, `iteration_budget`). Add the accessor with the expression feature.
  - |-
    Callback runtime failures surface as `CollectionError::CallbackFailed` carrying the original item path. Failures of ordinary body expressions inside a repeated copy keep the existing interpolation error with its authored line, without an iteration path; the path is recorded on each `ScopeRange` for the Phase 5 report work.
  - |-
    `ScopeRange` no longer records its authored source range (nothing read it); `BodyOrigin` already maps each copy back to authored bytes. Local frames carry `OverrideOrigin::Data`; origin preservation for `set` hand-off belongs to Phase 4.
message_to_agent: |-
  Phase 3 executes `::map`/`::filter`. Notes for Phase 4:
  - Executor: `collection_blocks/engine.rs` (`Execution`, `structure`); stage `inline/collection_blocks.rs`; it also owns page blocks whenever the body has an active collection.
  - Provenance: `body_origin.rs` has `Splice`, `ExpansionWriter`, `ScopeRanges` (`at`, `after_edits`, `charge`); each repeated copy is a `ScopeRange` with `scope`, `iteration` and `owners` (use `owners.len()` for nesting charges and for cache replay).
  - Sites: `site_scope.rs` (`SiteLookup`, `SiteHook::{enter,charge}`) is how interpolation, directive targets, shell expansion, shell blocks and transclusion `when=` read the locals of their copy. Shell local placeholders currently evaluate and run (typing/approval of local holes is still Phase 4's `project_hole` work). Transclusion `set`/child parameters are not yet scoped data; targets and `when=` are.
  - Budget: `IterationBudget` lives on `PipelineRuntime::iteration` (shared by `clone_for_child` and nested `as_markdown`); `IterationBudget::discovery` charges nothing for preflight but no preflight code uses it yet; `reserve_usage` is ready for cache replay. `MarkdownError::Collection` is fatal in `nested::is_structural` and `transclusion_failure_is_fatal`.
  - Options: `ComposeOptions::{with_max_iteration_visits, with_max_expansion_bytes, iteration_limits}` are in both identity encoders; CLI flags are Phase 5.
  - Tests: `tests/l1/collection_execution.rs` and the `effects-instrumentation`-gated `collection_effects.rs`. `just test` in darkmatter: 13,127 tests, ~2 min.
---

# Implementation Log for 2026-11-01-map-and-filter-directive (6 phases)

## Phase 1

Scope: Wave 1 (record contracts, establish interfaces). No runtime feature is
claimed; every scaffolded path is non-executing. Packages: `darkmatter` (lib),
compile-checked `darkmatter-cli`, `dmls`, `claudine`, `claudine-cli`.
Excluded work: the unrelated lifecycle moves already in the tree. The 25 plan
rules are the recorded contracts; no product ruling or spike remains. The
specification is untouched (dated snapshot). Shared ownership with
`2026-11-01-map-and-filter-expression`: callback/pattern, lexical frames, and
iteration types are defined here once, and that feature reuses them.

### Fixed interfaces (all under `darkmatter/lib/src/markdown/compose/`)

| Interface | Location |
| --- | --- |
| `BindingPattern`, `SpannedBindingPattern`, `Callback`, `SpannedCallback` (+ `erase()`) | `expression/ast.rs` |
| `parse_callback`, `parse_callback_spanned`, `INVALID_CALLBACK_CODE` (parser stub: rejects every input) | `expression/parser.rs` |
| `MappedQuotedValue` (decoded→authored boundary map) | `parse_utils.rs` |
| `LexicalScope`, `LocalId`, `ScopedLookup`, `ResolvedBinding::Local`, `EvaluationLookup::lexical_scope` | `expression/binding.rs`, `expression/mod.rs` |
| `BindingId::Local(LocalId)` | `expression/scope/projection.rs` |
| `IterationLimits` (+ defaults), `ExpansionUsage`, `IterationPath/Frame/Position`, `IterationError`, `ExpressionError::IterationLimit` | `expression/iteration.rs`, `expression/error.rs` |
| `ComposeOperation::CollectionBlocks` (index 16, default on, label `collection blocks`), `ComposeOperationPerfMetric::CollectionBlocks`, `PerfMetricKind/ComposeStage::CollectionBlocks` | `pipeline/operations.rs`, `perf.rs` |
| `CollectionKind`, `CollectionRegion`, `BodyScopes`, `CollectionAnalysisOptions`, `CollectionAnalysis` | `collection_blocks/types.rs` |
| `CollectionError`, `CollectionSite`, `MarkdownError::Collection` | `collection_blocks/error.rs`, `types.rs` |
| Diagnostic code constants | `collection_blocks/diagnostics.rs` |
| `ExpansionOwner`, `ScopeRange`, `ScopeRanges`, `BodyProvenance::scopes` | `body_origin.rs` |

`analyze_collection_blocks` keeps the plan's signature
`(source: &str, site: &TypeScope<'_>, options: &CollectionAnalysisOptions<'_>) -> Result<CollectionAnalysis, CollectionError>`
but is not defined yet (see decisions).

### Test mapping

| Behavior | Test |
| --- | --- |
| Operation registry order/phase includes `CollectionBlocks`; unchanged indices 0–15 | `type_tests.rs` `test_compose_operation_default_order_exact`, `test_compose_operation_phase_mapping_is_complete`, `..._all_is_complete_and_ordered` |
| Defaults 100,000 / 16,777,216, builders, overflow, path rendering, code mapping | `expression/iteration.rs` unit tests |
| Implicit/explicit local identity, null shadowing, no fall-through, nesting, siblings | `expression/binding.rs` `lexical_tests` |
| Escaped and multibyte offset mapping | `parse_utils.rs` `mapped_quoted_value_projects_escaped_and_multibyte_ranges` |
| Region keyword pairs | `collection_blocks/types.rs` |

Not tested here by design: callback grammar, collection execution, budgets,
CLI wiring (owned by later phases per the plan). The Input Robustness Matrix
does not apply (plan rule 24).

### Decisions

- `local` was NOT added to `reserved_root_descriptors` / `RootMembers` in this
  phase: registering the root changes how a bare `local` document key
  classifies for every existing document, which contradicts "scaffolding is
  non-executing". `ScopedLookup` protects reserved roots and declared globals
  and resolves `local.<name>`/bare names itself. Phase 2 (lexical frames) owns
  the descriptor and the host-registration rejection.
- `VariableClass` has no `Local` arm yet; Phase 2 adds it with the declared
  lexical layer. `BindingId::Local` exists and projects as `NoProjection`.
- `expression/iterable.rs` and `expression/scope/iterable.rs` are Phase 2
  (Wave 3) per the plan's wave tasks, not scaffolded.
- `IterationBudget`, `ExpansionWriter`, `ComposeOptions` limit builders,
  recovery builders, the `expression::Diagnostic` span/iteration-path fields,
  and diagnostic-code registration in `policy.rs` / `report_diagnostics.rs` /
  DMLS `codes.rs` are Phase 3/5 tasks and were not started.
- `MarkdownError::Collection` currently reports as the generic compose failure
  code in `report_diagnostics.rs` (wildcard arm); Phase 5 maps it to
  `CollectionError::code()`.
- `BodyProvenance::advance` debug-asserts scope ranges are empty; Phase 3 must
  carry them across edits before recording any.

### Verification

- `just test` (darkmatter): 12,918 run, 12,918 passed, 10 skipped (final run).
  Earlier runs failed three guards that this phase legitimately tripped, all
  fixed: `lookup_parity::the_inventory_lists_every_lookup_implementation`
  (added `ScopedLookup` as Probed and the binding fixture), `type_tests::
  default_order_has_sixteen_operations` (now seventeen), and
  `as_block_error_registry` (added the `CollectionError` downcast arm).
- `just lint` (darkmatter, cli, dmls, zed-dmls wasm): passes after the
  `run_render` allow noted in the decisions.
- `just check-tier-coverage darkmatter`: stranded (0).
- `cargo check --all-targets` for `darkmatter`, `darkmatter-cli`, `dmls`,
  `claudine`, `claudine-cli`: clean.
- `claudine` perf tests (`just test perf` in `claudine/`): 72 passed.
- CI scope gate skip does not apply (source files changed). Cross-OS check not
  run: no path, process, or OS-specific code was added.
- Skill update deferred to Phase 6 (the plan assigns `.claude/skills/darkmatter`
  edits there; no behavior is shipped yet).

## Phase 2

Scope: Waves 2–4 (grammar, lexical analysis, passive regions and site analysis).
Work was split across foreground-equivalent subagents per the plan's owners
(callback syntax, lexical frames, Markdown containers; then collection parser
and type projection; then site analysis). Subagents verified their slice only;
the phase gates ran once, here, with no subagent running.
Packages: `darkmatter`, `darkmatter-cli`, `dmls`, `claudine` (compatibility).

### Requirement-to-test mapping

| Behavior | Tests |
| --- | --- |
| Callback grammar: both arrows, patterns, shorthand, rejections, spans | `expression/parser.rs` `tests::callbacks` (19); `parse_utils.rs` `mapped_read_*` (4) |
| Lexical frames: alias equality, null/missing shadowing, nesting, siblings, retained frame, `fm.local`, protected roots, host `local` rejected | `expression/binding.rs` `lexical_tests`; `tests/lookup_parity.rs::assert_local_root_is_not_the_document`; `expression/scope/tests.rs` `lexical`; `absence.rs`; `prepared.rs`; `context/capture/groups.rs`; `catalog/roots.rs` |
| Container-aware pairing (lists, quotes, literal quoted page examples, active quoted shell, crossed closers, shell-body rejection, fences, CRLF, EOF) | `block_pairs.rs` tests (26 new); existing page/shell scan tests unchanged |
| Header validation (every Rule 3 rejection, false/empty regions, disabled op, lenient scan, nesting) | `collection_blocks/parser.rs` tests (22); `directives_api.rs` tests (5) |
| Declared projection, patterns, computed shapes, alias facts, map isolation, gate narrowing, recovery | `expression/iterable.rs` (5), `scope/iterable.rs` (21), `predicates.rs` `file_exists_*` (3) |
| Passive analysis incl. handlers that fail on invocation, all codes with spans, recovery warnings, host globals, frontmatter outside locals | `tests/l1/collection_analysis.rs` (17) |

Boundary and tier: all L1 (pure parsing/analysis through the library's public
API; composition-level rejection through `compose`). No file, network, or
terminal boundary is crossed, so no L2/L3 test applies. The Input Robustness
Matrix does not apply (plan rule 24: text grammar only).

### Fixes made by the coordinator

- `block_pairs.rs`: container path was computed at the `>` prefix instead of the
  directive's first content byte, so every quoted region got an empty path (3
  tests failed). Fixed with the offset past the stripped quote prefix.
- Two clippy lints in `block_pairs.rs` / `shell_blocks/mod.rs` fixed.

### Verification

- `just test` (darkmatter, `--no-fail-fast`): 13,058 run, 13,058 passed, 10 skipped.
- `just lint` (darkmatter, cli, dmls, zed-dmls wasm): clean.
- `just check-tier-coverage darkmatter`: stranded (0).
- claudine `classify|prewarm|perf|binding` tests: 219 passed; `cargo check
  --all-targets` for darkmatter-cli, dmls, claudine clean (subagent run).
- CI scope gate skip does not apply (source files changed). Cross-OS check not
  run: no path, process, or OS-specific code was added (container offsets are
  byte-based over `\n`/`\r\n`; the LF/CRLF cases are tested).
- Skill update deferred to Phase 6 per the plan.

## Phase 3

Scope: Waves 5–6 (budget, retained body scopes, executor, staged wiring).
Packages: `darkmatter` (lib); `darkmatter-cli`, `dmls`, `claudine` compile-checked and covered by `just test`. Work was done directly (no subagents), so the gates ran once, at the end.

### Requirement-to-test mapping

| Behavior | Tests (all L1) |
| --- | --- |
| Map: identity, projection, object transform, result-pattern failure, null vs missing field | `collection_execution::{map_*, a_pattern_field_that_is_missing_*}` |
| Filter: original item, omitted predicate, identity, short-circuiting `filter=` | `collection_execution::filter_*`, `an_omitted_filter_*`; `collection_effects::the_extra_filter_option_*` |
| Sources: missing/null/empty, null element, reached scalar, untyped recovery, key order | `collection_execution::{missing_null_*, a_null_element_*, a_reached_scalar_*, the_untyped_recovery_*, objects_iterate_*}` |
| Gates, laziness, prefix, per-item order, callbacks before body effects | `collection_effects::{a_false_gate_*, while_and_until_*, unsorted_selection_*, every_callback_finishes_*}`, `collection_execution::an_item_rejected_*` |
| Sorting: stability both directions, once-per-item keys, strings/numbers, invalid keys, empty selection | `collection_execution::{sorting_*, strings_sort_*, invalid_sort_keys_*}`, `collection_effects::{sort_keys_*, an_empty_selection_*}` |
| Locals: nesting, shadowing, `local`/`fm`, no leak | `collection_execution::{nested_regions_*, an_inner_local_*, explicit_local_*, a_local_does_not_leak_*}` |
| Page blocks in and around regions, operation subsets | `collection_execution::{a_conditional_block_*, a_collection_inside_*, page_blocks_outside_*, a_disabled_*}`, `collection_effects::the_frontmatter_surface_*` |
| Data provenance across copies | `collection_execution::{inserted_values_are_data_*, text_an_earlier_stage_*}`; `body_origin::tests::expansion::*` |
| Markdown boundaries | `collection_execution::{repeated_bodies_*, an_empty_body_*, block_quoted_*, directive_looking_*, crlf_*}` |
| Failure policy and paths | `collection_effects::{a_callback_failure_*, the_failing_position_*, nested_failures_*, a_failure_inside_a_repeated_body_*}` |
| Budgets: defaults, exactly full, rejected/stopping/sorted/empty visits, nesting, byte double charge, outside-region free, `as_markdown` restoration and sharing | `collection_execution::{defaults_*, an_exactly_full_*, rejected_items_*, nested_regions_share_*, authored_body_bytes_*, nested_output_*, text_outside_*, budget_exhaustion_*, nested_composition_*}`; `expression::iteration::tests` |
| Transclusion and shell sites read their copy | `collection_execution::{a_target_expression_*, a_transclusion_condition_*, a_collection_failure_in_a_child_*, shell::*}` |
| Lookup parity for `SiteLookup` | `tests::lookup_parity` |

Boundary: all in-process through `Markdown::compose_with` (filesystem for temp documents/children, shell via a recording approval handler). No terminal/browser behavior, so no L2/L3. The Input Robustness Matrix does not apply (text grammar).

### Notes

- Preflight remaining uncharged is structural for now: preflight does not run the collection stage; `IterationBudget::discovery` is untested beyond its unit test until Phase 4 discovers collections.
- Cross-OS: no path, process or OS-specific code added; cross-check not run.
- Skill update deferred to Phase 6.

### Verification

- `just test` (darkmatter, with dmls and cli): 13,127 run, 13,127 passed, 10 skipped.
- `just lint`: clean (fixed four clippy findings in new code). `just check-tier-coverage darkmatter`: stranded 0.
- `cargo check --all-targets` for `claudine`, `darkmatter-cli`, `dmls`: clean.
- CI scope gate skip does not apply: `packages` is non-empty (claudine, darkmatter, dmls, test-toolkit, darkmatter-cli).
