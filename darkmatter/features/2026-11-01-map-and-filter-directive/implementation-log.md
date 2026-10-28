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
packages:
  - darkmatter
  - darkmatter-cli
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
message_to_agent: |-
  Phase 1 added only non-executing contracts; nothing in the pipeline calls them yet. Notes for Phase 2:
  - `parse_callback` / `parse_callback_spanned` (expression/parser.rs) are stubs that reject every input; the AST types (`Callback`, `BindingPattern` and spanned forms) are real.
  - `local` is not yet a reserved root (see decisions_for_review); add the `RootMembers::LexicalBindings` descriptor and the host-registration rejection with the lexical-frames task, plus a `VariableClass::Local` arm and declared lexical type layer. `BindingId::Local(LocalId)` exists and projects as NoProjection.
  - `ScopedLookup` is in `expression/binding.rs` and is registered as a Probed lookup in `tests/lookup_parity.rs`; any new `EvaluationLookup` impl must be added to that INVENTORY.
  - `MarkdownError::Collection` exists and is registered in `markdown/errors/mod.rs::as_block_error`; `report_diagnostics.rs` still maps it to the generic compose failure code.
  - `BodyProvenance::scopes` is always empty and `advance` debug-asserts that; carry ranges across edits before recording any.
  - `just test` in darkmatter takes about 80-100 s; run `just test --no-fail-fast` once to see every failing guard rather than one per run (the default stops at the first failure).
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
