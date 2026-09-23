---
kind: plan
created: 2026-09-17
phase: 1
total_phases: 8
agent: claude/default
yolo: true
spec: ./spec.md
source_files_during_phase_1:
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - prompts/_reviews/review-spec-inline.md
  - prompts/_docs.md
  - prompts/documentation.md
  - prompts/_add/add-expressions.md
docs_updated_during_phase_1:
  - darkmatter/features/2026-09-15-dasherized-identifiers/spec.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
docs_created_during_phase_1:
  - darkmatter/features/2026-09-15-dasherized-identifiers/rulings.md
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - darkmatter/lib/tests/dasherized_identifier_compose.rs
  - darkmatter/cli/tests/compose_dasherized_identifiers.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
docs_updated_during_phase_3:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/tests/expression_regression.rs
  - darkmatter/lib/tests/context_functions.rs
  - darkmatter/lib/tests/nested_composition.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/lib/tests/compose_expression_failure_contract.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
  - darkmatter/cli/tests/compose_expression_failures.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/unknown_identifier_warning.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_unknown_identifiers.rs
docs_updated_during_phase_5:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_6:
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/dmls.md
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/compose/subtree.rs
docs_updated_during_phase_7:
  - darkmatter/docs/topics/parsing/index.md
  - darkmatter/docs/topics/parsing/lexing.md
  - darkmatter/docs/topics/parsing/grammar.md
  - darkmatter/docs/topics/parsing/scanning.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/lsp/features.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/os/build-hosts.md
source_code:
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - prompts/_reviews/review-spec-inline.md
  - prompts/_docs.md
  - prompts/documentation.md
  - prompts/_add/add-expressions.md
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/tests/dasherized_identifier_compose.rs
  - darkmatter/cli/tests/compose_dasherized_identifiers.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/tests/expression_regression.rs
  - darkmatter/lib/tests/context_functions.rs
  - darkmatter/lib/tests/nested_composition.rs
  - darkmatter/lib/tests/compose_expression_failure_contract.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
  - darkmatter/cli/tests/compose_expression_failures.rs
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/unknown_identifier_warning.rs
  - darkmatter/cli/tests/compose_unknown_identifiers.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
documentation:
  - darkmatter/features/2026-09-15-dasherized-identifiers/spec.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/rulings.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/docs/topics/parsing/index.md
  - darkmatter/docs/topics/parsing/lexing.md
  - darkmatter/docs/topics/parsing/grammar.md
  - darkmatter/docs/topics/parsing/scanning.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/lsp/features.md
completed_phase: "8"
implemented: true
packages:
  - darkmatter
human_review: false
message_to_agent: >-
  Phase 8 was the final phase: verification only, no source changes. Status:
  implementation complete, ready for review; the feature was not moved to
  _completed. Darkmatter L1 8269/0/14, lint green, L2 green on re-run (one
  cold-WezTerm harness spawn failure, unrelated). Claudine: 7032 passed, 4
  failed. All four are caused by an unrelated uncommitted edit to
  prompts/_implement/implement-plan.md that turns positional `set:` actions
  into mappings, which claudine's action_shape.rs rejects
  (LifecycleObjectDataThroughInterpolationPositional) and the shipped-prompt
  hash pin flags. With the committed prompt, all four pass alongside every
  Darkmatter change. The owner of that prompt edit must resolve it. No remote
  OS evidence was produced: build-win-native's W: disk is full, build-linux's
  cross-check lock has been held by nightly-reward-spike since 2026-09-14, and
  WSL ssh resets. Cross-OS risk was reviewed by reading the tests (sorted
  walk, canonicalize parity, Path::ends_with), so CI is the first multi-OS
  evidence.
---

# Dasherized Identifiers — Implementation Plan

## Summary and Definition of Done

### What this feature actually changes

Five requirements, four crates' worth of touch points, one behavior-breaking
grammar change. The work decomposes into four largely independent engineering
tracks plus a shared safety net:

| Track | Requirement | Primary surface |
| --- | --- | --- |
| **Grammar** | R1 | `lib/src/markdown/compose/expression/lexer.rs` `read_variable` + DMLS cursor scans |
| **Failure policy** | R2 | `lib/src/markdown/compose/interpolation/rewrite.rs` `interpolate_text` and its document-stage callers |
| **Runtime diagnostics** | R4 | evaluator + a new request-scoped candidate accumulator threaded through `ComposeOptions`/`ComposeContext` |
| **Editor diagnostics** | R3 | `dmls/src/overlay/expressions.rs`, `providers/dsl.rs`, `diagnostics/frontmatter.rs`, `providers/code_actions.rs` |
| **Identity (shared)** | R5 | `ComposeReport::merge` / `add_schema_advisory` dedup generalization |

The grammar change is small — roughly twenty lines inside two loops in
`read_variable` — and everything else in this plan exists because that small
change is *breaking* and because the incident it fixes was invisible for months.
The bulk of the effort is the diagnostic and failure-policy scaffolding that
makes the breakage loud instead of silent.

### Verified ground truth (established during planning)

These are facts confirmed by reading the code, not assumptions. They set the
shape of several phases:

- **The boolean-literal case falls out for free.** `read_variable`
  (`lib/src/markdown/compose/expression/lexer.rs:874`) reclassifies `true`/`false`
  only *after* the full name is scanned. `false-1` therefore joins into one
  identifier with no extra work, exactly as the spec's behavior table predicts.
- **`read_variable` has exactly the two loops the spec describes** — a leading
  identifier loop and a dotted-segment loop. Both need the rule; neither is
  shared with `read_number`, so the `4-2` case is untouched by construction.
- **`root_identifier` has exactly five consumers**, matching the spec's GitNexus
  claim: `dmls/src/providers/dsl.rs:398`, `dsl.rs:734`,
  `dmls/src/graph/substrate.rs:402`, `dmls/src/diagnostics/frontmatter.rs:644`,
  and `dmls/src/overlay/expressions.rs:563`. Only two are diagnostic call sites.
- **DMLS has exactly two ad hoc cursor word scans**, both spelling the same
  character class `is_alphanumeric() || '_' || '.'`:
  `completion_partial` (`overlay/expressions.rs:249`) and
  `value_completion_partial` (`overlay/expressions.rs:619`). These are the
  scans that do *not* follow the lexer change automatically.
- **`EvaluationLookup::is_known_variable_root` already exists** with a default of
  `true` (`lib/src/markdown/compose/expression/mod.rs:291`), a real
  implementation on the subtree lookup (`subtree.rs:268`), and a forwarding impl
  on the body-stage `DeferrableLookup` (`inline/interpolation.rs:120`). R4's
  known-root gate has a seam already cut for it.
- **Two divergent AST walks already exist**, and they disagree:
  `collect_variable_roots` (`subtree.rs:536`) is short-circuit-aware — fully
  tolerant inside `Fallback`, descends only a `Ternary`'s condition — while
  `walk_context_variables` (`interpolation/evaluator.rs:396`) is exhaustive.
  R4 needs the former's semantics; R3 needs something closer to the latter's.
  **Neither is directly reusable as-is.** See Ruling R-2.
- **`FunctionBinding` does not exist in the codebase**, and there is **no alias
  registry** for expression functions. The real catalog surface is
  `ExpressionFunctionDescriptor` / `expression_function_descriptors()`
  (`expression/catalog/mod.rs:310`, `:499`). The spec's absence-predicate rule
  names a symbol and a mechanism that must be either built or narrowed. See
  Ruling R-1.
- **`fatality_characterization.rs` is a live pinned matrix**
  (`lib/src/markdown/compose/interpolation/fatality_characterization.rs`) whose
  `expected()` function encodes today's lenient body behavior and asserts
  "fatality drift" on change. R2 will turn it red by design.
- **`ComposeReport::merge` dedups only schema advisories**, via
  `same_schema_advisory` (`context/report.rs:279`, `:407`). Generalizing it is a
  contained change with existing merge tests as the behavior pin.
- **All four corpus directories exist**: root `prompts/` (45 entries),
  `.claude/commands/` (6), `darkmatter/prompts/` (1), `claudine/prompts/` (1).

### Definition of done

The feature is complete when all eight statements below hold. They are the
spec's Success Criteria, restated as observable checks.

1. `just test` and `just lint` are green in `darkmatter/`, and the Claudine
   package area — the only meaningful downstream consumer of Darkmatter's
   expression engine — is green too.
2. The permanent L1 corpus audit test exists in `lib/tests/`, walks all four
   prompt/command directories through the library's own extraction (never
   regex), and asserts both gates: zero breaking-form usages, and every shipped
   executable expression still parses under the new grammar.
3. `{{ key-name }}`, `{{ doc.key-name }}`, and `{{ doc['key-name'] }}` resolve to
   the same value for a kebab-case frontmatter key; `{{ _loop_count - 1 }}`,
   `{{ 4-2 }}`, `{{ f(x)-1 }}`, and `{{ arr[0]-1 }}` still evaluate as
   arithmetic.
4. A body or mixed-frontmatter expression that cannot parse or evaluate fails
   full-document composition **regardless of `fail_fast`**, carries source path
   and span in a typed error, and emits no partial document and no verbatim
   `{{ … }}` on stdout. `compose_subtree(..., Lenient)` is byte-for-byte
   unchanged, proven by a dedicated regression.
5. One bad expression produces exactly one message. One unknown root referenced
   ten times in one source document produces exactly one warning; the same root
   in two transcluded sources produces two.
6. An unresolved root absent from effective state, reserved namespaces, and the
   *effective* schema warns at code `dm.expression.unknown_identifier`,
   composition succeeds at exit 0, and the value still renders empty. A root
   known via `--set`, inherited state, another caller layer, or
   baseline/trigger schema is silent even when its value is explicitly null or
   empty. Every suppression row of the spec's table is pinned by a test.
7. DMLS reports unknown identifiers in operand positions (binary operand,
   ternary branch, function argument, fallback RHS) in both body interpolations
   and frontmatter expression values, at `WARNING` severity, with the
   frontmatter-less editor exception preserved. Hover, completion, and
   go-to-definition resolve at every cursor position inside a kebab identifier
   including on the dash, and do not merge `foo--bar`, a trailing dash, or
   spaced subtraction.
8. A regression test demonstrates that `prompts/_reviews/feature-review.md`
   would have failed to compose at the moment the original `{{spec-name}}` typo
   was introduced.

### Explicitly out of scope

Restating so no phase drifts into them: `--deny-warnings` promotion,
machine-readable warning output (`--format json` stays document-only, warnings
stay stderr-only), schema-aware path analysis for bracket keys and deep object
members, any type-system work, and any change to operator/function result
semantics or namespace membership. All are chartered in
[`2026-09-16-expression-type-system`](../2026-09-16-expression-type-system/spec.md).

### Risk register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Corpus contains a real unspaced-subtraction collision | Low | High — blocks R1 | Phase 1 spike S-1 runs the audit *before* any lexer edit; pre-approved rewrite to `{{ x - 1 }}` |
| R2 turns a large number of existing tests/fixtures red at once | **High** | Medium | Phase 1 spike S-3 measures the blast radius before Phase 4 starts |
| Accumulator threading fights rayon parallelism or the `&ComposeOptions` shared borrow | Medium | High — reshapes Phase 5 | Phase 1 spike S-2 prototypes both options against the real transclusion merge points |
| DMLS cursor scans regress hover/completion subtly | Medium | Medium | Cursor work is bundled *into* Phase 2 with the lexer, per the spec, not deferred |
| Absence-predicate alias mechanism does not exist | **Certain** | Low | Ruling R-1 narrows or builds it in Phase 1 |

---

## Phase 1 — Rulings, Spikes, and the Compatibility Gate

**Goal.** Close every specification ambiguity that would otherwise be resolved
by guesswork mid-implementation, and stand up the permanent corpus audit that
must be green before a single lexer character changes.

Nothing in this phase edits the lexer, the evaluator, or DMLS behavior. It
produces rulings, spike findings, and one new test file.

### Necessary Rulings

These must be decided and recorded (append to the spec's Resolved Decisions, or
to a `rulings.md` beside this plan) before the phases that depend on them start.
Each names its blocking phase.

- [x] **R-1 — Absence-predicate resolution mechanism** *(blocks Phase 5, Phase 6)*
  - The spec requires `is_null(...)`/`is_empty(...)` "**or their registered
    aliases, resolved through the function catalog (`FunctionBinding`), never
    duplicated as string literals in the walker**".
  - **Verified: `FunctionBinding` does not exist, and the expression catalog has
    no alias registry at all.** The real surfaces are
    `ExpressionFunctionDescriptor` (`expression/catalog/mod.rs:310`) and
    `expression_function_descriptors()` (`:499`). The functions themselves are
    `is_null` (`functions/mod.rs:586`) and `is_empty_fn` (`:596`), exposed under
    the names `is_null` and `is_empty` in
    `docs/schemas/expression-functions.yaml:56,84`.
  - **Rule needed:** either (a) narrow the rule to the two exact catalog-resolved
    names, adding a typed `absence_predicate: bool` (or equivalent) marker to
    `ExpressionFunctionDescriptor` so the walker reads the catalog rather than
    hard-coded strings — satisfying the spec's *intent* without inventing an
    alias system; or (b) build a general alias registry first.
  - **Recommendation: (a).** The spec's prohibition is on string literals in the
    walker, not on the absence of aliases; a descriptor flag honors it, is
    smaller, and the rule is explicitly interim (Resolved Decision 9) and due for
    retirement by the successor spec. Building an alias registry for a rule
    scheduled for deletion is speculative work.

- [x] **R-2 — Which walk becomes the shared classifier** *(blocks Phase 5, Phase 6)*
  - Two incompatible walks exist. `collect_variable_roots` (`subtree.rs:536`) is
    short-circuit-aware: **fully tolerant inside any `Fallback` subtree**, and
    descends only a `Ternary`'s condition. `walk_context_variables`
    (`evaluator.rs:396`) is exhaustive and descends everything.
  - R4 needs neither exactly. The spec requires: fallback **primary** suppressed
    but fallback **RHS warns when evaluated**; ternary **condition** suppressed,
    branches warn **when evaluated**; guarded-root branch references suppressed.
    `collect_variable_roots` is too tolerant (it suppresses the fallback RHS);
    `walk_context_variables` is not tolerant at all.
  - **Rule needed:** confirm a *new* shared classifier is written, that
    `collect_variable_roots` keeps its current subtree-strict-mode semantics
    unchanged, and that `walk_context_variables` is either left alone or migrated.
  - **Sub-question with real user-visible consequence:** `collect_context_warnings`
    is exhaustive today, so `{{ ctx.typo || 'default' }}` currently emits a
    `ctx.*` warning. If the new shared classifier absorbs `ctx.*` handling, that
    warning disappears. **Recommendation: leave `ctx.*` warnings exactly as they
    are in this feature** — the spec's suppressions are scoped to R4's unknown-root
    warning and changing `ctx.*` behavior is unrequested scope.

- [x] **R-3 — Fatality scope: which `interpolate_text` call sites become strict**
  *(blocks Phase 4)*
  - The spec names "body interpolation" and "mixed text in frontmatter". Verified
    call sites of `interpolate_text` / `interpolate_value` are: body stage
    (`inline/interpolation.rs:58`), frontmatter interpolation
    (`frontmatter_interpolation.rs:246`), frontmatter shell expansion
    (`frontmatter_shell_expansion.rs:1546`, `:1727`, `:1785`), directive targets
    (`directive_targets.rs:370`), and subtree (`subtree.rs:497`).
  - Subtree is explicitly excluded by the spec. Body and frontmatter are
    explicitly included. **The three shell-expansion sites and the
    directive-targets site are unaddressed.**
  - **Rule needed:** an explicit include/exclude verdict per site.
    **Recommendation: include all four.** They are full-document composition
    surfaces evaluating author-written expressions; excluding them recreates the
    exact silent-passthrough hole this feature exists to close. Record the
    verdict either way so Phase 4 is not a judgment call.

- [x] **R-4 — Shape of the failure-policy parameter** *(blocks Phase 4)*
  - The spec forbids making `interpolate_text` unconditionally strict and
    requires "an explicit expression-failure policy (or use a document-specific
    wrapper)".
  - `interpolate_text` takes `fail_fast: bool` today (`rewrite.rs:101`), and the
    lenient branches are at `rewrite.rs:166-186`.
  - **Rule needed:** choose (a) replace `fail_fast: bool` with a typed
    `ExpressionFailurePolicy { Lenient, Strict }` threaded to every caller, or
    (b) add a separate parameter/wrapper leaving `fail_fast` intact.
  - **Recommendation: (a), a typed two-variant enum.** `interpolate_text` is
    `pub(crate)`, so this is an internal signature with a bounded call-site list
    (seven sites, all enumerated above). A bool that no longer means what its
    name says is the more expensive outcome.

- [x] **R-5 — Accumulator ownership and threading** *(blocks Phase 5)*
  - The spec says the accumulator "travels with `ComposeOptions`/`ComposeContext`"
    and that collection "must either share interior-mutable state or fold
    per-unit at the existing merge points — an implementation constraint, not a
    prescription."
  - `ComposeOptions` is passed as `&ComposeOptions` through the pipeline and is
    read concurrently under rayon during transclusion.
  - **Rule needed:** pick one. Interior mutability (`Mutex`/`RwLock` on a
    candidate vec inside `ComposeOptions`) violates Performance-Posture
    invariant 3 ("no cross-document lock joins the parallel compose hot path")
    if placed naively. Per-unit fold at existing `ComposeReport::merge` points
    preserves it.
  - **Recommendation: per-unit fold**, carrying candidates on `ComposeReport`
    (which already merges) rather than on `ComposeOptions`. Confirm against
    spike S-2's findings before ratifying.

- [x] **R-6 — `fatality_characterization.rs` disposition** *(blocks Phase 4)*
  - This module's `expected()` matrix pins today's lenient body behavior and
    asserts "fatality drift" when it changes. R2 makes it fail by design.
  - **Rule needed:** confirm it is *updated in place* to encode the new contract
    (keeping it as the living drift guard) rather than deleted or weakened. It is
    almost certainly the "fatality characterization matrix" the spec's
    Requirement 2 lists among required doc updates.
  - **Recommendation: update in place**, and treat its new matrix as the
    authoritative statement of the post-change failure contract.

- [x] **R-7 — Corpus audit walk parameters** *(blocks Phase 1 audit task)*
  - Directories confirmed present: root `prompts/` (45), `.claude/commands/` (6),
    `darkmatter/prompts/` (1), `claudine/prompts/` (1).
  - **Rule needed:** recursive or top-level only; which file extensions
    (`.md` only, or also `.yaml` schema/trigger docs carrying Expression-typed
    values); whether `_unscheduled`/`_completed` subtrees are in scope.
  - **Recommendation:** recursive, `.md` plus `.yaml`, all subtrees included —
    the sibling test `shipped_schema_and_trigger_corpus_parses_passively` walks
    directories precisely "so a newly shipped artifact is covered the day it
    lands", and a narrower walk reintroduces the blind spot.

- [x] **R-8 — Quick-fix structured-data carrier** *(blocks Phase 6)*
  - The spec requires structured diagnostic data describing the exact
    replacement, and forbids the code-action provider reparsing the
    human-readable message.
  - **Rule needed:** carry it on `Diagnostic.data` (round-tripped by the client
    into the code-action request) or recompute in `code_actions.rs` from the
    diagnostic range. Confirm the DMLS client contract supports `Diagnostic.data`
    round-tripping; verify against `providers/code_actions.rs:47`, which receives
    `diagnostics: &[Diagnostic]`.
  - **Recommendation: `Diagnostic.data`** — it is the LSP-native carrier and the
    provider already receives the diagnostics.

- [x] **R-9 — Runtime known-root coverage for caller input layers** *(blocks Phase 5)*
  - Success Criterion 6 requires no warning for a root supplied via `--set`,
    inherited parent state, or another caller layer.
  - **Rule needed:** confirm the body-stage lookup chain
    (`DeferrableLookup` → `ResolvingLookup` → `EffectiveState`) already sees all
    those layers through `is_known_variable_root`, or specify exactly what the
    lookup contract must be extended with. The spec is explicit that extension
    must happen on the lookup contract, **not** by reconstructing schema state
    inside the evaluator.
  - Decide before Phase 5 whether *effective schema* (baseline + triggers +
    extensions, not just document `$schema`) is reachable from the lookup or must
    be supplied to the reconciliation step.

- [x] **R-10 — Severity raise blast radius** *(blocks Phase 6)*
  - Raising `dm.expression.unknown_identifier` from `INFORMATION` to `WARNING`
    touches `dsl.rs` (~`:741`) and `diagnostics/frontmatter.rs` (~`:655`).
  - **Rule needed:** confirm no DMLS snapshot, protocol test, or `zed-dmls`
    extension configuration filters on `INFORMATION` for this code. Inventory
    before changing.

### Spikes

Time-boxed investigations that lower risk. Each produces a written finding
appended to the rulings record; none produces shipped code except S-1's test.

- [x] **S-1 — Corpus collision spike**
  - Run the compatibility audit logic (prototype or the real test from Task 1.4)
    across the four corpus directories **before** any lexer edit.
  - Classify hits into: real breaking-form usage; GitHub Actions `${{ }}`
    syntax; triple-brace interpolation literals; fenced documentation examples;
    string-literal contents such as `'review-' + iteration`.
  - **Exit criterion:** a definitive count of real collisions. Zero ⇒ Phase 2 is
    unblocked. Non-zero ⇒ apply the pre-approved rewrite
    (`{{iteration-1}}` → `{{ iteration - 1 }}`) and re-run; escalate only for
    files that cannot be edited.
  - **This is the single highest-value spike in the plan** — it is the gate on
    whether R1 can ship at all.

- [x] **S-2 — Accumulator threading spike**
  - Prototype candidate collection through one real transclusion merge point
    (`transclusion/engine.rs:605` region, folding at `ComposeReport::merge`)
    and, separately, through interior-mutable state on `ComposeOptions`.
  - Measure against Performance-Posture invariant 3: does either introduce a
    lock on the rayon hot path?
  - **Exit criterion:** a recommendation that settles Ruling R-5, with the
    rejected option's specific failure mode named.

- [x] **S-3 — Fatality blast-radius spike**
  - Temporarily flip the lenient branches at `rewrite.rs:166-186` to fatal, run
    `just test` in `darkmatter/` and the Claudine package area, and catalogue
    every failure.
  - Separate genuine latent breakage (which R2 exists to surface — including the
    spec-named `{{absolute-filepath}}` and `{{review-file}}` cases) from tests
    that deliberately assert lenient behavior and must be rewritten.
  - **Exit criterion:** a concrete list sized for Phase 4, and confirmation of
    whether any fixture requires the R1 lexer fix to land first. Revert the
    probe; ship nothing from it.

- [x] **S-4 — DMLS cursor-scan spike**
  - Determine whether `completion_partial` (`overlay/expressions.rs:249`) and
    `value_completion_partial` (`:619`) can reuse token/AST spans from
    `lex_spanned`, or must reimplement the joined-dash rule locally.
  - Both currently do a backward `rfind` over a character class; the joined-dash
    rule is forward-looking and mid-identifier-sensitive, so a backward scan
    cannot express it directly — confirm the shape of the fix.
  - **Exit criterion:** a chosen approach for Phase 2, Task 2.3, including how
    `foo--bar`, a trailing dash, and spaced subtraction stay unmerged.

### Work-group 1A — concurrent

Rulings and spikes are independent of one another and of the audit test.
Run these three tracks in parallel.

- [x] **1.1 Ratify rulings**
  - Decide R-1 through R-10 above.
  - Record each verdict with its rationale in the spec's Resolved Decisions
    section (numbering continues from 18) or a `rulings.md` beside this plan.
  - Dependency: none. Several rulings are informed by spikes — ratify those
    (R-5 by S-2, R-6 and R-3 by S-3) after the corresponding spike reports.

- [x] **1.2 Run spikes S-1 and S-4**
  - S-1 gates Phase 2 entirely; run it first.
  - S-4 informs Phase 2 Task 2.3 only.

- [x] **1.3 Run spikes S-2 and S-3**
  - S-2 gates Phase 5's architecture; S-3 sizes Phase 4.
  - Neither blocks Phase 2 or Phase 3, so these may run behind 1.2 without
    delaying the critical path.

### Work-group 1B — after S-1 reports

- [x] **1.4 Build the permanent corpus audit test**
  - New L1 test in `darkmatter/lib/tests/`, a sibling of
    `shipped_schema_and_trigger_corpus_parses_passively`
    (`lib/tests/schema_phase_validation.rs:548`). Follow its directory-walk
    idiom so newly shipped artifacts are covered automatically.
  - Walk root `prompts/`, `.claude/commands/`, `darkmatter/prompts/`, and
    `claudine/prompts/` per Ruling R-7.
  - Classify executable Darkmatter surfaces **through the library's own
    extraction, never regex**: body and frontmatter interpolations
    (`ExpressionFinder`), `when=` expressions, Expression-typed frontmatter
    values, and `$()` ternary conditions and branches. String literals such as
    `'review-' + iteration` must not count — their dash is never tokenized as an
    operator.
  - Assert **gate A (pre-change)**: zero usages of the breaking form —
    identifier-like operand, unspaced `-`, identifier-continuation follower.
  - Structure the test so **gate B (post-change)** — every shipped executable
    expression still parses under the new grammar, parse-only — can be enabled in
    Phase 2 without rewriting the walk. Typo detection is explicitly *not* this
    test's job; it belongs to R4's runtime warning.
  - Exclude by construction: GitHub Actions `${{ }}`, `{{{ }}}` interpolation
    literals, and fenced documentation examples.

- [x] **1.5 Resolve any corpus collision**
  - Only if S-1 found real collisions.
  - Apply the pre-approved semantics-preserving rewrite
    `{{iteration-1}}` → `{{ iteration - 1 }}`.
  - Escalate to the author only for files that cannot be edited.

### Validation checkpoint — Phase 1

- [x] Corpus audit test is green on the **unmodified** lexer (gate A passes).
- [x] All ten rulings recorded with rationale.
- [x] All four spike findings written down.
- [x] `just test` green in `darkmatter/` (the new test adds no other change).

---

## Phase 2 — Lexer Grammar and DMLS Cursor Consistency

**Goal.** `-` continues an identifier under the spec's exact rule, and every
DMLS surface that scans identifiers agrees with the lexer about where one ends.

**Depends on:** Phase 1 (gate A green; Ruling R-7; spike S-4).

The spec is explicit that cursor/partial handling ships **in the same change**
as the lexer rule, so hover, completion, and definition do not regress. Treat
Phase 2 as one atomic landing.

### Work-group 2A — concurrent

- [x] **2.1 Lexer rule**
  - Modify `read_variable` (`lib/src/markdown/compose/expression/lexer.rs:874`)
    in **both** loops — the leading-identifier loop and the dotted-path-segment
    loop. Missing the second leaves `doc.spec-name` broken while `spec-name`
    works.
  - Rule: consume `-` iff the scanner is already mid-identifier (`name` is
    non-empty for the current segment) **and** the next character satisfies
    `is_identifier_char` (`char::is_alphanumeric() || '_'`).
  - Keep the decision inside `read_variable`, **not** in `is_identifier_char`.
    Resolved Decision 1: a global character class would make `4-2` ambiguous.
  - Do **not** touch the post-scan `true`/`false` reclassification at the end of
    the function. Leaving it in place is what makes `false-1` and `true-value`
    join, per the spec's behavior table.
  - No lookbehind, no token-history inspection. `read_number` handles `4`
    separately, so the scanner is not mid-identifier at the `-` in `4-2`.
  - Verify span correctness through `lex_spanned` (`lexer.rs:~933`): a joined
    identifier's `Spanned<Token>` range must cover the whole dashed name.

- [x] **2.2 Parser object-literal key coverage**
  - No parser change expected: the unquoted-key guard at
    `expression/parser.rs:592` accepts a single `Variable` token with no dot and
    an ASCII alphabetic-or-`_` first character, so `{ spec-name: 1 }` becomes
    legal automatically.
  - Add explicit parser tests for this intentional side effect, **including** that
    a quoted key is still required where the ASCII-first guard demands one
    (e.g. a key starting with a digit or a non-ASCII character).
  - If the guard turns out to reject the joined token, that is a real parser
    change — surface it rather than widening the guard silently.

- [x] **2.3 DMLS cursor and partial scans**
  - Per spike S-4's finding, fix `completion_partial`
    (`dmls/src/overlay/expressions.rs:249`) and `value_completion_partial`
    (`:619`), which both use the backward scan
    `rfind(|c| !(c.is_alphanumeric() || c == '_' || c == '.'))`.
  - **Do not simply add `-` to that character class.** The spec calls this out
    directly: it would merge `foo--bar` and spaced subtraction into one
    identifier.
  - The rule is forward-looking and mid-identifier-sensitive, so a backward
    character-class scan cannot express it. Reuse token/AST spans from
    `lex_spanned`, or implement the exact rule with the correct directionality.
  - `--set` key validation and schema assignment already accept `-`; confirm no
    change is needed there.

### Work-group 2B — after 2A

- [x] **2.4 Lexer and parser tests (L1)**
  - Every row of the spec's Requirement 1 behavior table as a token-stream
    assertion, including the breaking rows (`iteration-1`, `false-1`,
    `true-value`) asserted in their **new** meaning.
  - Edge cases named by the spec: `-` at end of input; `-` followed by
    whitespace; `-` followed by `-` (`foo--bar` must stay `foo - (-bar)`); a
    kebab segment inside a dotted path; a kebab identifier as an unquoted object
    key; `café-name` (the identifier predicates are Unicode, so this must join).
  - Parser: `iteration - 1` still lowers to `Binary`; `4-2`, `f(x)-1`,
    `arr[0]-1`, `(a)-1`, `"x"-1`, `false - 1` still lower to `Binary`;
    `spec-name` lowers to a single `Variable` whose span covers the whole name.

- [x] **2.5 Compose-level kebab tests (L1)**
  - A document with a kebab frontmatter key renders it three ways — bare,
    `doc.<key>`, and `doc['<key>']` — and all three produce the same value
    (Success Criterion 1).
  - Regression: `{{ _loop_count - 1 }}` still evaluates.

- [x] **2.6 DMLS navigation tests (L1 protocol/provider)**
  - Hover, completion, and go-to-definition resolve on a kebab identifier at
    **every** cursor position within it, explicitly including a cursor on the
    dash.
  - Negative assertions: cursor scans do not merge `foo--bar`, a trailing dash,
    or spaced subtraction.

- [x] **2.7 Enable corpus gate B**
  - Turn on the post-change assertion in the Phase 1 audit test: every shipped
    executable expression still parses under the new grammar (parse-only).

### Validation checkpoint — Phase 2

- [x] `just test` and `just lint` green in `darkmatter/`.
- [x] Corpus audit green on **both** gates. *(Gate A was green before the
      lexer change and was retired with it, per spec Resolved Decision 12;
      its fixtures now assert the joined form. Gate B is green.)*
- [x] Claudine package area tests green — it calls Darkmatter's parser and
      evaluator directly and supplies variables through `EvaluationLookup`.
      *(7032/7036; the 4 failures are the pre-existing `implement-plan.md`
      set recorded in Phase 1 and are unrelated to the lexer.)*
- [x] Success Criteria 1, 2, and 7 demonstrably met. *(SC7's navigation
      half; its operand-position diagnostics belong to Phase 6.)*

---

## Phase 3 — Diagnostic Identity and Deduplication

**Goal.** Requirement 5: source-aware diagnostic identity and rescan
deduplication, so every problem is reported exactly once.

**Depends on:** Phase 1 rulings. Independent of Phase 2 — may run concurrently
with it.

**Sequencing note from the spec:** this lands **before** Phase 4's body-failure
change, so the current duplicate-warning behavior remains a useful regression
fixture rather than being masked by the switch to a fatal error. Do not reorder.

### Work-group 3A — concurrent

- [x] **3.1 Diagnostic identity keys**
  - Introduce typed identity for coded warnings. Per the spec's Requirement 5 and
    Resolved Decision 13:
    - unresolved-root family → `(source document identity, code, normalized root name)`
    - expression-failure family → `(source document identity, code, original source span)`
  - Track identity **independently of mutable output offsets**. The rescan loop
    in `interpolate_text` (`rewrite.rs:124`, `for depth in 0..MAX_INTERPOLATION_DEPTH`)
    rewrites the buffer between passes, so an offset captured on pass 1 is
    meaningless on pass 2. This is the mechanism behind the observed duplicate:
    a failing span is left unchanged, a sibling span is replaced, and the next
    depth rescans the unchanged failure.
  - Keep the **first authored location** for display.
  - Replacement-generated expressions take the source identity of the document
    and the first generated span at which they are observed; they must not erase
    or alias an authored diagnostic.

- [x] **3.2 Generalize `ComposeReport::merge` dedup**
  - Today `merge` (`lib/src/markdown/compose/context/report.rs:279`) dedups only
    schema advisories via `same_schema_advisory` (`:407`), and
    `add_schema_advisory` (`:260`) does the same on the add path.
  - Generalize to **one** mechanism keyed by `(source, code, family_key)`, where
    each coded-warning family declares its family key at its definition site.
  - **Behavior-preservation requirement:** the schema-advisory family's extractor
    must return exactly today's `(source, code, path)` triple — behavior-preserving
    by construction. The existing merge tests are the pin; they must pass
    unchanged.
  - A universal `(source, code, message)` key is **rejected** by Resolved
    Decision 13: message is prose, not identity — it over-merges distinct
    advisories and under-merges reworded ones.
  - Apply the same mechanism on both the add path and the merge path so a report
    cannot accumulate duplicates before it is ever merged.

### Work-group 3B — after 3A

- [x] **3.3 Diagnostic-count tests (L1)**
  - **Preserve the original two-span rescan fixture**: one successful
    replacement plus one bad expression produces exactly one issue. This is the
    regression the sequencing exists to protect — write it against today's
    warning behavior so Phase 4 inherits a working fixture.
  - One unknown root referenced ten times in one document → exactly one warning.
  - The same root in two transcluded source documents → exactly two warnings
    (independently actionable, one per source).
  - Two different unknown roots → exactly two warnings.
  - Existing `ComposeReport::merge` schema-advisory tests pass unchanged.

### Validation checkpoint — Phase 3

- [x] `just test` and `just lint` green in `darkmatter/`.
- [x] Schema-advisory dedup behavior provably unchanged.
- [x] Rescan-duplicate fixture in place and asserting exactly one issue.

---

## Phase 4 — Fatal Expression Failures in Full-Document Composition

**Goal.** Requirement 2: a body or mixed-frontmatter expression that cannot be
parsed or evaluated fails full-document composition regardless of `fail_fast`,
with a typed error carrying source path and span, and no partial output.

**Depends on:** Phase 3 (identity in place), Rulings R-3, R-4, R-6, spike S-3.
Independent of Phase 2 — the spec says either order works — but landing after
Phase 2 means fewer latent kebab failures surface at once.

### Work-group 4A — sequential within, concurrent with 4B

- [x] **4.1 Failure-policy plumbing**
  - Per Ruling R-4, introduce the typed expression-failure policy and thread it
    through `interpolate_text` (`interpolation/rewrite.rs:97`) and
    `interpolate_value` (`:~257`).
  - Convert the two lenient branches at `rewrite.rs:166-186` — the
    `EvalResult::Error` warning arm and the `Err(e)` parse-warning arm — to honor
    the policy.
  - **Do not make `interpolate_text` unconditionally strict.** The spec forbids
    it explicitly, because `compose_subtree(..., SubtreeStrictness::Lenient)` is
    an established public contract for best-effort data-tree interpolation and
    Claudine makes explicit subtree choices that must not change silently.
  - Set the policy per call site according to Ruling R-3's verdict:
    - body stage — `inline/interpolation.rs:58` — strict
    - frontmatter interpolation — `frontmatter_interpolation.rs:246` — strict
    - frontmatter shell expansion — `frontmatter_shell_expansion.rs:1546`,
      `:1727`, `:1785` — per R-3
    - directive targets — `directive_targets.rs:370` — per R-3
    - subtree — `subtree.rs:497` — **unchanged**, driven by `SubtreeStrictness`
      exactly as today

- [x] **4.2 Typed error with source projection**
  - The body stage currently constructs an effective-source error **without
    attaching the document's source context**. The spec requires projecting the
    scanner's byte span through that context, not merely prefixing a line number
    to the message.
  - The error must carry original source path, line, and expression span, and
    render through the same typed rich-error surface as other compose errors.
  - **No partially rewritten document and no verbatim failing `{{ … }}` may
    reach stdout.** Ensure the failure path aborts before
    `*markdown.content_mut() = result.output` in
    `inline/interpolation.rs:~125`.

- [x] **4.3 Update the fatality characterization matrix**
  - Per Ruling R-6, update `expected()` in
    `lib/src/markdown/compose/interpolation/fatality_characterization.rs` to
    encode the new contract. Its module docs (lines 12–34) describe today's
    "three lenient cases" in body interpolation and must be rewritten alongside.
  - Keep it as the living drift guard — do not weaken or delete it.

### Work-group 4B — concurrent with 4A

- [x] **4.4 Triage the S-3 blast radius**
  - Work the catalogue produced by spike S-3.
  - For each failing test or fixture, decide: genuine latent breakage that R2
    exists to surface (fix the source), or a test deliberately asserting lenient
    behavior (rewrite the assertion).
  - Expect `{{absolute-filepath}}` and `{{review-file}}` among the genuine
    breakage — both are simultaneously fixed by Phase 2's lexer change, so
    confirm they now resolve rather than merely failing louder.

### Work-group 4C — after 4A and 4B

- [x] **4.5 Failure-contract tests (L1, library boundary)**
  - Body and mixed-text frontmatter parse **and** evaluation failures, with
    `fail_fast` both `false` **and** `true` — all four combinations fatal.
  - The already-strict surfaces stay strict: frontmatter whole-value
    interpolation, `when=` conditions, `$()` ternary conditions and branches.
  - Assert a typed error with source path and span, and no partially rewritten
    output.
  - **Subtree regression:** `SubtreeStrictness::Lenient` stays lenient and
    `Strict` stays strict, asserted directly.
  - **Public condition API regression:** `evaluate_condition` and
    `parse_condition` + `evaluate` keep their exact current contract — failures
    propagate as `Result` errors, each caller owns disposition. Resolved
    Decision 10. Verify Claudine's fatal loop conditions and its
    deliberately warns-and-skips hook `when=` handling both still behave as
    before.

- [x] **4.6 CLI failure tests (L1)**
  - Use `CliProcessFixture` (`cli/tests/common/fixture.rs`) for representative
    `md compose` body and mixed-frontmatter failures. Do not hand-build the
    command — `cli/tests/spawn_site_guard.rs` rejects raw spawns.
  - Assert: exit 1; a source line in the error; **no verbatim failing
    `{{ … }}` on stdout**; and exactly one rendered error.

- [x] **4.7 Motivating-incident regression**
  - Keep one end-to-end composition of `prompts/_reviews/feature-review.md`
    through its normal invocation path.
  - Add the Success Criterion 8 test: the document as it stood with the original
    `{{spec-name}}` typo **fails** to compose.

### Validation checkpoint — Phase 4

- [x] `just test` and `just lint` green in `darkmatter/`; Claudine green.
- [x] Success Criteria 3 and 8 demonstrably met.
- [x] Fatality characterization matrix green against its new expectations.

---

## Phase 5 — Unresolved-Root Warning at Runtime

**Goal.** Requirement 4: a well-formed identifier resolving to nothing warns —
it does not fail — unless the author explicitly handled its absence or the root
is known to effective state, reserved namespaces, or the effective schema.

**Depends on:** Phase 3 (identity keys), Phase 4 (policy plumbing), Rulings R-1,
R-2, R-5, R-9, spike S-2. Benefits from Phase 2 landing first so valid kebab
references already resolve and do not generate false candidates.

This is the largest and most architecturally sensitive phase. Sequence its
work-groups strictly.

### Work-group 5A — foundation, sequential

- [x] **5.1 Candidate accumulator**
  - Introduce the request-scoped diagnostic accumulator per Ruling R-5's chosen
    shape (recommendation: carried on `ComposeReport`, folded at existing merge
    points).
  - Records **source-spanned unknown-root candidates**, not finished warnings.
  - **Observational only**, three hard constraints from the spec: it must not
    execute an expression a surface would not otherwise evaluate; must not
    perform schema or file I/O; must not recapture request context. That last
    one is a Darkmatter non-negotiable — one request-scoped composition context,
    never recaptured downstream.
  - Performance-Posture invariant 2: O(1) work per variable evaluation.
  - Performance-Posture invariant 3: **no cross-document lock on the parallel
    compose hot path.** Transclusion resolution and frontmatter shell expansion
    run under rayon with per-unit reports merged afterward; fold per-unit at
    those merge points.

- [x] **5.2 Shared known-root classifier**
  - One classifier, used by every runtime surface, so they cannot drift.
  - A root is **known** when it is: present in effective state (**including an
    explicit `null` or empty-string value**), supplied by `--set`, inherited
    state, or any caller input layer; a reserved namespace (`ctx`, `env`, `doc`);
    or declared by the **effective** schema — document `$schema` **plus**
    configured baseline **plus** matched schema extensions and triggers. The
    document-local `$schema` is not the only schema authority.
  - **Missing versus falsy is mandatory.** `EvaluationLookup::get` already
    distinguishes `None` from `Some(Value::Null)` and `Some(Value::String(""))`.
    Use presence and known-root information, **never** the rendered empty string.
  - Ride the existing `EvaluationLookup::is_known_variable_root` hook
    (`expression/mod.rs:291`, default `true`) per Ruling R-9. Its default keeps
    third-party lookups silent — preserve that compatibility default. If more
    lookup metadata is needed, **extend the lookup contract explicitly**; do not
    reconstruct schema state inside the evaluator.

- [x] **5.3 Deferred reconciliation**
  - Frontmatter interpolation pass 1 runs **before** schema validation and before
    the final effective state exists (compose pipeline steps 1 and 2). It cannot
    classify a missing root.
  - **Do not reorder composition** and do not resolve schema inside the
    evaluator. Pass 1 records candidates only.
  - Reconcile after the final frontmatter interpolation pass (step 4) and again
    when child/transclusion reports merge: discard any root then known to
    effective state or schema, and emit the remaining deduplicated warnings.
  - Later body and condition evaluations (steps 6–10) may classify immediately —
    their effective state is available.

### Work-group 5B — suppression semantics, after 5A

- [x] **5.4 Structural suppressions**
  - Suppress when the unresolved identifier is the **primary of a `Fallback`**
    or the **condition of a `Ternary`** — both are explicit "this may be absent"
    constructs.
  - When a ternary condition is a bare variable, **matching references to that
    same root in its branches are guarded and suppressed too** (this is what
    keeps the documented `color ? color : "none"` idiom silent). A *different*
    unknown root in either branch still warns.
  - Per Ruling R-2, write a **new** classifier walk. `collect_variable_roots`
    (`subtree.rs:536`) is too tolerant — it suppresses the whole `Fallback`
    subtree including the RHS, but the spec requires `{{ a || x }}` to warn for
    `x` when the RHS is evaluated. Leave that function's subtree-strict-mode
    semantics unchanged.

- [x] **5.5 Absence-predicate suppression**
  - A **direct bare-variable** argument to `is_null(...)` or `is_empty(...)`
    suppresses the warning for that variable.
  - **Nested uses still warn**: `is_empty(trim(x))` warns for `x`, because the
    predicate no longer directly guards the lookup.
  - Resolve predicate names through the catalog per Ruling R-1 — recommendation
    is a typed marker on `ExpressionFunctionDescriptor`
    (`expression/catalog/mod.rs:310`), read by the walker. **Never hard-code
    name strings in the walker.**
  - Mark the rule **interim** in code comments, citing Resolved Decision 9 and
    its retirement trigger (the successor's null-admitting-parameter
    suppression).

- [x] **5.6 Short-circuit reachability**
  - Runtime warnings follow **evaluation** reachability: an unchosen ternary
    branch or a short-circuited fallback is not a runtime issue and must not
    warn.
  - The spec is explicit: "the runtime collector must preserve the evaluator's
    short-circuit behavior rather than blindly walking the whole AST before
    evaluation." This rules out a pre-evaluation AST sweep.
  - Note the contrast with `collect_context_warnings` (`evaluator.rs:377`),
    which *does* walk exhaustively before evaluation. Per Ruling R-2, leave
    `ctx.*` warning behavior alone — do not fold it into this collector.

- [x] **5.7 Wire every runtime surface**
  - Route all of these through the single shared policy so they cannot drift:
    - frontmatter interpolation, **both** passes
      (`frontmatter_interpolation.rs`)
    - body interpolation (`inline/interpolation.rs:58`)
    - page-block `when=` (`page_blocks/engine.rs:40`, `:98`)
    - transclusion `when=` (`transclusion/engine.rs:605`)
    - `$()` ternary conditions and branches
      (`frontmatter_shell_expansion.rs` — `FrontmatterShellTernary`, `:83`)
  - **A bare unknown root in `when="x"` warns.** A misspelled gate silently
    disabling content is the motivating failure class reborn — it is explicitly
    *not* suppressed despite being a condition.
  - **Scope boundary holds:** only full-document composition changes. The public
    condition API keeps its contract; passive schema parsing and DMLS validation
    perform no evaluation at all.

- [x] **5.8 Required-property non-interference**
  - A required property left unset fails schema validation (pipeline step 2)
    before body interpolation and before candidates are reconciled, emitting
    `missing iteration-1: required but not provided`.
  - Requirement 4 must **not** add a second warning for that case. The root is
    *known*, so the schema gate owns it.
  - The known-root gate keys on **known or unknown**, never on required-ness.

- [x] **5.9 Warning emission**
  - Emit as an existing `ComposeWarning` (`context/report.rs:340`) with the
    stable code `dm.expression.unknown_identifier` — cross-surface parity with
    DMLS — and `source`, `path`, and `line_number` set.
  - **No new diagnostic type.** Resolved Decision 11.
  - Dedup through Phase 3's mechanism with family key = normalized root name.
  - Warnings remain **stderr-only**. `--format json` still emits the document
    only. Both CLI follow-ups are explicitly out of scope.

### Work-group 5C — tests, after 5B

- [x] **5.10 Worked-examples tests (L1)**
  - One case per row of the spec's worked-examples table, asserting exit code,
    rendered output, **and** diagnostic count together:
    - `{{ iteration-1 }}`, undeclared → warning, renders empty, exit 0
    - `{{ iteration-1 || "fallback" }}`, undeclared → silent, renders `fallback`
    - `{{ iteration-1 }}`, `iteration-1: number` → silent, renders empty
    - `{{ iteration-1 }}`, `iteration-1: number(required)` → **fatal, exit 1,
      with the schema's own message and NO additional unresolved-identifier
      warning**

- [x] **5.11 Suppression-table tests (L1)**
  - Every row of the spec's suppression table, asserting that evaluated
    branch/RHS positions warn and unreachable ones do not:
    `{{ x }}` warns; `{{ x || "d" }}` silent; `{{ a || x }}` warns when the RHS
    is evaluated; `{{ x ? a : b }}` silent; `{{ x ? x : b }}` silent;
    `{{ a ? x : b }}` warns when the `x` branch is evaluated;
    `{{ is_null(x) }}` silent; `{{ is_empty(trim(x)) }}` warns; bare
    `when="x"` warns.
  - Include a registered-alias case if Ruling R-1 chose to build aliases.

- [x] **5.12 Known-root tests (L1)**
  - Assert **no** warning when a root arrives via `--set`, inherited state,
    another caller input layer, baseline schema, a matched trigger schema, or as
    an explicit `null` / empty-string value.
  - These are the cases that distinguish "missing" from "falsy" — the single
    easiest thing to get wrong in this phase.

- [x] **5.13 Surface-coverage tests (L1)**
  - Cover a body expression, a mixed frontmatter value, a page-block `when=`, a
    transclusion `when=`, and a `$()` ternary, each with a representative
    unknown root, proving every runtime call site is wired to the shared policy.

### Validation checkpoint — Phase 5

- [x] `just test` and `just lint` green in `darkmatter/`; Claudine green.
- [x] Success Criteria 4 and 5 demonstrably met.
- [x] Performance-Posture invariants 2 and 3 verified **by inspection** and
      recorded. No numeric budgets are introduced. If a regression is suspected,
      measure against the existing criterion benches
      (`lib/benches/compose_schema_transclusion.rs`, `phase6_interpolation.rs`) —
      do not add new SLOs.

---

## Phase 6 — DMLS Full-AST Identifier Walk

**Goal.** Requirement 3: DMLS diagnoses every unknown `Variable` node at its own
span, not just the expression's root — plus the dash-separated-key diagnostic
and its quick-fix, and the severity alignment.

**Depends on:** Phase 2 (grammar), Phase 5 (suppression semantics must match, so
editor and runtime do not disagree about intentional absence). Rulings R-1, R-2,
R-8, R-10.

### Work-group 6A — concurrent

- [x] **6.1 Diagnostic-only AST walk**
  - Add a **new helper** in `dmls/src/overlay/expressions.rs` that walks the
    `SpannedExpr` AST and yields every `Variable` node **with its own span**.
  - **`root_identifier` (`overlay/expressions.rs:121`) must not change.** Its
    five consumers — `providers/dsl.rs:398`, `dsl.rs:734`,
    `graph/substrate.rs:402`, `diagnostics/frontmatter.rs:644`,
    `overlay/expressions.rs:563` — include hover, go-to-definition, and graph
    indexing, all of which intentionally want exactly one navigable root and
    would change semantics if handed every operand. Resolved Decision 8.
  - Only the **two diagnostic call sites** adopt the new walk:
    `providers/dsl.rs` `expression_diagnostics` (~`:713`) and
    `diagnostics/frontmatter.rs` (~`:643`).
  - Model the recursion on `walk_context_variables` (`evaluator.rs:396`), which
    already covers Binary, Ternary, Fallback, Index, MemberAccess, FunctionCall
    args, Paren, and unary nodes.
  - Performance-Posture invariant 1: **single-pass, linear in AST nodes, over an
    already-parsed AST. No additional parses.**

- [x] **6.2 Classification and boundaries**
  - Classify each `Variable` by its **first dotted segment**, reusing
    `is_unknown_root` (`overlay/expressions.rs:591`) as the shared authority.
  - `ctx`, `env`, and `doc` remain known namespaces; existing
    namespace-specific diagnostics continue to own invalid members, so this
    feature emits no duplicate unknown-identifier diagnostic for them.
  - A frontmatter root is known when present in the document or declared by
    DMLS's effective schema shape (`providers/frontmatter::known_shape`).
  - **Out of scope, explicitly:** deeper object members (requires schema-aware
    path analysis) and string-literal bracket keys — `doc['typo-key']` is a
    literal, not a `Variable`, so the walk never visits it. Resolved Decision 18.
  - **Preserve the frontmatter-less editor exception**: `is_unknown_identifier`
    (`providers/dsl.rs:935`) returns `false` when the document has no
    frontmatter, because any bare identifier could be a `--set` value DMLS
    cannot see. The runtime has no such blind spot; the editor keeps it.

- [x] **6.3 Apply R4's suppressions statically**
  - DMLS remains static and may diagnose an unknown identifier in **either**
    non-suppressed branch, but it applies the same fallback-primary,
    ternary-condition, guarded-root, and absence-predicate suppressions as
    Requirement 4, so editor and runtime do not disagree about intentional
    absence. Resolved Decisions 6 and 9.
  - The difference is deliberate and narrow: DMLS does **not** apply short-circuit
    reachability (it is static and cannot know which branch runs); the runtime
    does.

- [x] **6.4 Severity alignment**
  - Raise `dm.expression.unknown_identifier` from `INFORMATION` to `WARNING` at
    `providers/dsl.rs` (~`:741`) and `diagnostics/frontmatter.rs` (~`:655`).
  - One condition must not carry two severities across surfaces.
  - Sweep the inventory from Ruling R-10 for snapshot or configuration fallout.

### Work-group 6B — after 6A

- [x] **6.5 Dash-separated-key diagnostic**
  - When the authored source for a subtraction normalizes to a frontmatter key
    that actually exists or is declared — `a- b` matching `a-b`, or `foo--bar`
    matching `foo--bar` — report a mis-referenced key instead of generic unknown
    operands.
  - **Only on an exact source/key match.** All other arithmetic keeps generic
    diagnostics.
  - Emit **one** key-level diagnostic for the subtraction span instead of
    separate generic diagnostics for its operand variables.
  - Membership info is available via `ast.entry_by_dotted` through
    `expression_root_is_unknown` (`diagnostics/frontmatter.rs:666`) and
    `is_unknown_identifier` (`dsl.rs:935`).
  - **Do not** claim support for documents "pinned" to the old grammar — no
    expression grammar-version mechanism exists.

- [x] **6.6 Quick-fix with structured data**
  - Keep the stable `dm.expression.unknown_identifier` code
    (`diagnostics/codes.rs:125`) and attach **structured** diagnostic data
    describing the exact replacement, per Ruling R-8.
  - **The code-action provider must not reparse the human-readable message.**
    `providers/code_actions.rs:47` receives `&[Diagnostic]`; read the structured
    field.
  - Offer exactly one semantics-preserving fix: remove the separating whitespace
    when the resulting key is legal under Requirement 1, otherwise bracket access
    such as `doc['foo--bar']`.
  - If a safe whole-expression replacement cannot be proven, emit the ordinary
    diagnostic **with no quick-fix**.

- [x] **6.7 DMLS tests (L1 protocol/provider)**
  - `dm.expression.unknown_identifier` fires for an unknown identifier in each
    operand position — binary operand, ternary branch, function argument,
    fallback RHS — in **both** a body interpolation and a frontmatter expression
    value.
  - The explicit-absence suppressions hold, and the frontmatter-less editor
    exception stays silent.
  - Dash-separated-key: assert **both** replacement forms, the structured
    diagnostic data, and that ambiguous arithmetic offers no fix.
  - Severity is `WARNING` at both sites.

### Validation checkpoint — Phase 6

- [x] `just test` and `just lint` green in `darkmatter/`.
- [x] Success Criteria 6 and 7 demonstrably met.
- [x] `root_identifier` behavior provably unchanged for all five consumers.
- [x] Editor and runtime suppressions verified consistent — a case suppressed at
      runtime is suppressed in DMLS, modulo the deliberate short-circuit
      difference.

---

## Phase 7 — Documentation and Skill Updates

**Goal.** Every document describing the "before" picture now describes the
"after" picture. CLAUDE.md's authoring discipline applies: an edit that changes
a symbol's behavior includes a pass over its docs in the same change — so some
of this may already have landed inline during Phases 2–6. This phase catches
what remains and covers the standalone prose.

**Depends on:** Phases 2–6 complete (docs describe shipped behavior).

### Work-group 7A — concurrent, one task per document

All targets verified to exist.

- [x] **7.1 Parsing topic docs**
  - `docs/topics/parsing/lexing.md` — the identifier section and the kebab
    workaround paragraph.
  - `docs/topics/parsing/grammar.md` — the language-server section's root-only
    limitation.
  - `docs/topics/parsing/index.md` — remove the known-deviation callout now that
    Requirement 2 has landed.
  - Note: `docs/topics/parsing/scanning.md` also exists; review it for drift
    even though the spec does not name it.

- [x] **7.2 Expression and interpolation docs**
  - `docs/topics/darkmatter-expressions.md` — the identifier principle and the
    unsupported-forms list.
  - `docs/inline/interpolation.md` — the empty-string fallback paragraph now
    carries a warning unless the property is known to effective state/schema or
    the absence is explicitly handled.

- [x] **7.3 DMLS diagnostics registry note**
  - `dmls/docs/diagnostics.md` — add a short `dm.*` registry rules note
    (~15 lines) in the "Sources and codes" area: ownership,
    one-condition-one-code, severity-per-surface, and the procedure for adding a
    new code (a `dmls/src/diagnostics/codes.rs` entry plus a doc row).
  - Reference it from `docs/topics/parsing/index.md`.
  - Resolved Decision 15 defers a shared cross-crate Rust code-constant module
    until a second shared code exists — **do not build one now.**

- [x] **7.4 Public API documentation**
  - `ComposeOptions::with_fail_fast` — document the intentional narrowing: it
    remains meaningful for recoverable non-expression stages such as TOC linking
    and non-structural transclusion, and no longer authorizes malformed or
    unevaluatable expressions in a full document.
  - `ComposeWarning` (`context/report.rs:340`) — its doc comment currently says
    warnings "did not prevent the transform from completing (when
    `fail_fast = false`)", which is now drift. Fix it.
  - Subtree strictness docs (`subtree.rs` module docs, `SubtreeStrictness` at
    `:121`) — distinguish expression authoring failures from other recoverable
    compose failures and state that explicit lenient subtree behavior is
    preserved.
  - The fatality characterization matrix docs — if not already handled in
    Task 4.3.

- [x] **7.5 Darkmatter skill**
  - `.claude/skills/darkmatter/compose.md` — the parsing pointer and error
    policy.
  - Per CLAUDE.md's drift-maintenance rule, also check whether the skill's
    error-handling section and any README describing public compose behavior
    need updating.

### Validation checkpoint — Phase 7

- [x] Every document the spec's Phase 6 lists has been updated.
- [x] No document still describes body interpolation failures as warnings.
- [x] `md hash` re-run on any Markdown file carrying a frontmatter hash property.
- [x] `just lint` green.

---

## Phase 8 — Integration Verification and Handoff

**Goal.** Prove the eight Definition-of-Done statements, confirm no downstream
consumer regressed, and leave the work in the correct terminal state.

**Depends on:** Phases 1–7.

### Work-group 8A — concurrent

- [x] **8.1 Full package-area validation**
  - `just test` and `just lint` in `darkmatter/`.
  - `just test-l2` in `darkmatter/` — confirm no real-terminal behavior
    regressed. No test in this feature belongs at L2; this is a regression sweep
    only. Ensure no terminal or browser window gains focus.
  - Do **not** run workspace-wide Cargo gates for a Darkmatter change.

- [x] **8.2 Downstream consumer validation**
  - Use Sniff and GitNexus to enumerate actual downstream consumers of the
    changed Darkmatter surfaces — Claudine is the known one, and it calls the
    parser and evaluator directly and supplies variables through
    `EvaluationLookup`.
  - Run the Claudine package area's `just test`.
  - Confirm specifically: Claudine's loop conditions are still fatal, its hook
    `when=` handling still warns-and-skips, and its explicit
    `SubtreeStrictness` choices behave unchanged (Resolved Decision 10).

- [x] **8.3 Cross-OS review**
  - No OS-specific code is expected in this feature — it is parser, evaluator,
    and LSP work with no `#[cfg(windows)]` surface.
  - Review any new test for path-comparison or line-ending assumptions. The
    corpus audit test walks directories and reads files, so verify its path
    handling and sorting are deterministic across macOS, Linux, native Windows,
    and WSL2 — the sibling `yaml_files_in`
    (`lib/tests/schema_phase_validation.rs:532`) sorts explicitly for exactly
    this reason.
  - Load the `os` skill before claiming any OS cannot be tested from this host.

### Work-group 8B — after 8A

- [x] **8.4 Success-criteria audit**
  - Walk all eight Definition-of-Done statements and name the test that proves
    each. A criterion with no test is not met.
  - Pay particular attention to Criterion 8 — the
    `prompts/_reviews/feature-review.md` historical-typo regression, which is
    the one criterion most easily assumed rather than demonstrated.

- [x] **8.5 Graph change analysis**
  - Run `detect_changes({scope: "all"})` and
    `detect_changes({scope: "compare", base_ref: "main"})` per CLAUDE.md.
  - `partial: true` or `truncated: true` is not a clean check — re-run.
  - Refresh the index with `just gitnexus` if stale.

- [x] **8.6 Terminal state**
  - Report **implementation complete, ready for review**.
  - Do **not** move the feature to `_completed` and do **not** run
    `just complete` — that is the author's call after the review cycle closes.
  - Do not commit unless explicitly instructed in a separate prompt.

### Validation checkpoint — Phase 8

- [x] All eight Definition-of-Done statements proven by a named test.
- [x] Darkmatter and Claudine both green.
- [x] Graph change analysis clean.
- [x] Rulings record complete and consistent with what shipped.

---

## Dependency Summary

```
Phase 1 (rulings, spikes, corpus gate A)
   │
   ├──────────────┬───────────────────────────────┐
   ▼              ▼                               │
Phase 2        Phase 3                            │
(lexer +       (identity +                        │
 DMLS cursor)   dedup)                            │
   │              │                               │
   │              ▼                               │
   │           Phase 4 ◄── S-3, R-3, R-4, R-6 ────┘
   │           (fatal failures)
   │              │
   └──────┬───────┘
          ▼
       Phase 5 ◄── S-2, R-1, R-2, R-5, R-9
       (runtime warning)
          │
          ▼
       Phase 6 ◄── R-8, R-10
       (DMLS walk)
          │
          ▼
       Phase 7 (docs)
          │
          ▼
       Phase 8 (verification)
```

**Critical path:** Phase 1 (S-1) → Phase 2 → Phase 5 → Phase 6 → Phase 7 → 8.

**Genuine concurrency:**

- Phase 2 and Phase 3 are fully independent — different files, different
  requirements. Run them in parallel.
- Phase 4's work-groups 4A and 4B are independent (implementation vs. triaging
  the existing test fallout).
- Phase 6's work-groups 6A and 6B split cleanly (walk and severity vs.
  dash-key diagnostic and quick-fix).
- Phase 7 is one task per document, all independent.
- Within Phase 1, the three work-group 1A tracks are independent, though S-1
  should start first because it gates the longest chain.

**Non-negotiable ordering** (from the spec, not from convenience):

1. Corpus gate A green **before** any lexer change merges.
2. Phase 3 **before** Phase 4 — so the duplicate-warning fixture survives as a
   regression rather than being masked by the switch to a fatal error.
3. Phase 2 **before** Phase 5 in practice — so valid kebab references already
   resolve and do not generate false unknown-root candidates.
4. Requirement 1 **must not ship without** Requirements 3 and 4's diagnostics.
   The spec states this directly: the grammar change makes `{{iteration-1}}` a
   well-formed reference to a nonexistent key, and the diagnostics are what keep
   that from being a new silent failure. **Do not land Phase 2 in a release
   without Phases 5 and 6.**
