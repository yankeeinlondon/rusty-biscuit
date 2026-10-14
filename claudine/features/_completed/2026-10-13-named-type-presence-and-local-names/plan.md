---
total_phases: 6
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/parity_tests.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/schema_interactive/supplied.rs
    - claudine/cli/src/commands/schema_interactive/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/completions/shell-completions.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_phase.rs
docs_updated_during_phase_5:
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/schema-validation.md
    - claudine/docs/topics/composition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/claudine/cli-commands.md
source_files_during_phase_6: []
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
    - darkmatter/lib/src/markdown/schemas/errors.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/simplified/query.rs
    - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/types.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/mod.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/named_type_references.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/supplied/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/parity_tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/src/commands/schema_interactive/supplied.rs
    - claudine/cli/src/commands/schema_interactive/tests.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_validate.rs
    - darkmatter/cli/tests/l1/schema_validate_phase.rs
documentation:
    - claudine/docs/topics/completions/shell-completions.md
    - darkmatter/docs/topics/schemas/local-type-references.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/inline/schema-validation.md
    - claudine/docs/topics/composition.md
completed_phase: 6
implemented: true
packages:
    - darkmatter
    - dmls
    - claudine
    - claudine-cli
    - darkmatter-cli
---

# Named-Type Presence and Local Names Implementation Plan

## Phase 1 — Scope, Success, and Contracts

### Necessary Rules

The finalized specification is the authority. It has no open product questions; the rules below carry its decisions into implementation and identify the remaining engineering choices. Work proceeds without interactive feedback. Record any departure in `implementation-log.md` alongside this plan; leave the specification unchanged.

- **Ownership:** implement schema semantics in Darkmatter, editor behavior in DMLS, and grouped-match consumption in Claudine. This plan stays in the requested Claudine feature directory despite the specification's `area: darkmatter`.
- **Presence:** remove stripping; inherited `required` and `generated` survive scalar, array, object, union-arm, and multi-hop reuse. Required inner fields alone do not require their enclosing object. There is no use-site opt-out. Preserve authored versus effective/runtime validation semantics; generated presence does not supply a value.
- **Namespace:** bare names use exactly the existing `@this` namespace and defining-file context. Built-ins win, names remain case-sensitive, forward references see the complete table, and unknown names are typed errors at the authored reference. Root-union arms do not form a shared namespace; whole-file references and raw JSON Schema gain no exports. Local names never trigger schema-root discovery or create external dependency edges.
- **Constraints:** apply the specification's merge table within a reused definition, without changing tier replacement. Keep item, array-container, and object-bound scopes separate. Merge block `$constraints` and inline object bounds together when they govern the same object. Detect only contradictory bounds, empty scheme intersections, and unequal normalized defaults; equal bounds and numerically equal defaults are valid. Do not add a general satisfiability solver or new default validation policy.
- **Conjunction:** preserve every pattern, including `not-empty`, through lowering and validation. Preserve each `match(...)` group separately, with AND between groups and the existing positive/exclusion rules within each group. Completion and existing root-union matching use the conjunction; ordinary single-schema validation stays passive. Empty intersections require no file search and are accepted.
- **Restrictions:** preserve terminal `@` grammar and all explicit import forms. Reject postfixes, including descriptions, on union-typed named targets and reject a second array suffix on already-array targets. Plain union references and their use as union arms remain supported. Nullable unions, the `null` type, and `|` syntax remain outside this change.
- **Provenance:** conflicts retain typed property paths, source file or inline identity, and individual authored spans through multi-hop expansion and typed error wrappers. DMLS anchors the primary diagnostic at the consuming reference and includes related declaration locations. Examples retain their defining-file origin and declaration order; suggestions retain the property-wide single-suggestion limit, including flattened union arms.
- **Serialization:** canonical local references are bare unless a built-in collision requires `@this`; external targets remain intact. Use existing source-aware editing for localized changes. Do not mass-rewrite working `@this` references.
- **Spike scope:** no performance spike is justified by this specification: it names no measured performance question or OS-specific cost. Run only the one provenance experiment below, once, before choosing its representation. Wider performance measurement requires an author decision and is not scheduled here.
- **Verification:** use L1 nextest recipes and existing fixtures; no new CI cells, real providers, terminal sessions, browser windows, or focus changes. Tests must be portable across macOS, Linux, native Windows, and WSL2. Load the `os` skill before path-comparison or Windows-specific changes, or before ruling out an OS test host. Do not run `cargo fmt`, commit, or move this feature into `_completed`.

### Work and Success

The implementation replaces presence stripping and append/overwrite behavior with target-aware constraint merging, adds bare local references to the shared parser/resolver, and carries those semantics through JSON Schema, DMLS, and Claudine. The highest risks are losing source provenance across imports, confusing array scopes, dropping conjunctions during conversion, and missing consumers of the current single-list file-match API.

Completion means all ten specification acceptance criteria have observable evidence: equivalent bare/explicit resolved properties; correct authored/effective presence; complete merge behavior; typed, accurately located errors; editor assistance and dependency refresh; grouped file completion and root-union validation; serialization parity; recorded shipped-artifact audits; and passing required area checks. Current topic docs and skills must describe the implemented behavior. The terminal state is implementation complete, ready for review.

Waves are globally numbered execution steps. Tasks within a wave may be delegated concurrently only where their file ownership is disjoint; finish the preceding wave and its checkpoint before starting the next. A single-task wave is sequential. Do not run lint alongside tests.

### Wave 1 — Investigate Independent Risks

- [x] **Surface Inventory**
  - Inventory `darkmatter/lib/src/markdown/schemas/{resolve.rs,errors.rs,file_match.rs}` and `simplified/{types.rs,grammar.rs,source.rs,cursor.rs,query.rs,lint.rs,convert.rs,serialize.rs}`. Identify parser entry points, import fast paths, bound interpretation, object `$constraints`, description precedence, and annotation consumers.
  - Trace DMLS from `overlay/schema.rs` through frontmatter assistance, definition lookup, diagnostic publication, and dependency invalidation. Trace Claudine through `cli/src/completion/schema_completion/` and `lib/src/composition/schema/supplied.rs`; supplied-value root-union validation is a consumer too.
  - Inventory shipped `*/schemas/`, `*/docs/schemas/`, the embedded baseline, and inline schemas in shipped Markdown. List presence-stripping tests/comments, duplicate constraints, built-in collisions, and public file-match consumers. Distinguish production artifacts from intentionally invalid test fixtures.
  - Deliver a path/symbol inventory and acceptance-to-test map in the implementation log, including actual additional package areas requiring checks. Use text search and call-graph queries where useful; do not make index refresh a gate.

- [x] **Provenance Spike**
  - Independently inspect the source-map path through the resolver. Today `SchemaSourceMap` records constraint spans, while most `Constraint` variants carry values alone; determine the smallest way to keep a declaration's origin associated with each surviving constraint.
  - Run one bounded fixture experiment: a required bare alias and a two-file, multi-hop alias with a lower bound in the original definition and conflicting upper bound at the use site. Include one quoted or block scalar and one block `$constraints` bound.
  - Demonstrate how the consuming reference, both original constraint spans, structural property path, inline identity, and nested typed error survive to DMLS-related locations. This can use a disposable probe; do not land a competing production pipeline.
  - Deliver the chosen representation, affected public constructors, and a diagnostic example. Reuse the existing source-aware facilities and request context; do not build a YAML syntax tree or repeat this spike later.

**Checkpoint:** the inventory accounts for all consumers; the provenance design has a concrete source-location example; each acceptance criterion has a planned public assertion. Resolve engineering choices here using the finalized rules, documenting any unresolved author ruling explicitly before dependent work.

## Phase 2 — Shared Models and Reference Parsing

Depends on Phase 1. Establish contracts before separate agents edit their consumers.

### Wave 2 — Establish Shared Contracts

- [x] **Model Contracts**
  - Implement the agreed provenance representation in shared schema types/source maps and typed schema errors. Preserve individual bound origins through stricter-bound selection, and retain enough use-site information for consuming-reference diagnostics.
  - Define the local-reference representation, built-in collision predicate, and grouped file-match API/annotation shape. Prefer reusing the existing import expansion representation for a local target rather than adding a second resolver.
  - Assign one owner to `simplified/types.rs`, public exports, and shared constructors. Update compile-required call sites without introducing new semantics in consumers. Record the API and file ownership for subsequent waves.
  - Document the grouped annotation as internal generated-schema metadata, not a new user configuration format. Keep unchanged single-group behavior equivalent; settle the exact encoding here before consumers implement it.

**Checkpoint:** affected crates compile against the agreed shared model, typed locations are inspectable, and local-reference versus external-dependency behavior is explicit. No unresolved shared-file ownership remains.

### Wave 3 — Implement Independent Front Ends

- [x] **Local Grammar**
  - Owner: `simplified/grammar.rs`, `source.rs`, `cursor.rs`, `query.rs`, `serialize.rs`, and their dedicated tests. Parse a non-keyword identifier as a local reference with its authored span; keep the existing identifier and terminal-`@` grammar.
  - Support forward references, descriptions, inline objects, union arms, arrays, and value/container postfixes wherever explicit references support them. Defer applicability and numeric-versus-length interpretation until the target resolves.
  - Teach source projection and cursor/reference discovery about bare names, including documents with no `@`. Meta-type parsing stays passive and accepts reference syntax without requiring a definition or executing expressions.
  - Round-trip bare and explicit references, built-in collisions, descriptions, and external targets. Test plain, quoted, and block source presentations and source-aware edits without whole-document rewriting.

- [x] **Grouped Match Model**
  - Owner: `schemas/file_match.rs` and dedicated model tests. Replace single-list queries/preparation with a representation preserving each group's positives and exclusions and requiring every group to admit a candidate.
  - Reuse `GlobReference` and captured `FileResolutionContext`; preserve value/caller origin, prepared roots, file-name matching, symlink filtering, and existing missing/partial-value behavior. Reuse `FileReference` for resolution, without recapturing ambient state.
  - Preserve the per-group validity rules and prove that constructing an empty intersection performs no file walk. Do not flatten groups into one OR list or add another glob implementation.
  - Test one-group parity, multiple positives/exclusions per group, cross-group conjunction, and valid groups whose conjunction admits nothing.

**Checkpoint:** passive parsing and source projection recognize bare references, serialization preserves meaning, and grouped matching has a stable tested API. These tasks own different files; shared type changes return to the Wave 2 owner.

## Phase 3 — Resolution, Merging, and Lowering

Depends on Phase 2. Resolver and converter tasks may run together against the established model; both must finish before end-to-end parity is assessed.

### Wave 4 — Implement Semantic Pipelines

- [x] **Constraint Resolution**
  - Owner: `schemas/resolve.rs`, resolution-related lint/query integration, and resolver tests. Delete `strip_top_level_presence` and its call and replace the stale explanation that describes stripping as intentional.
  - Expand bare references through the existing named-import path with the complete defining-file table, request-scoped namespace cache, file policy, cycle stack, and depth limit. Fix import detection/fast paths so bare-only files are expanded; local references add no external dependency.
  - Make `apply_import_postfix` merge constraints by resolved target and scope. Preserve existing array item/container constraints, descriptions, required/generated flags, target identity, enum members, and literals. Check object bounds from both `$constraints` and postfixes together.
  - Normalize lower/upper bounds, schemes, and equal defaults; keep all patterns and match groups; deduplicate flags; preserve ordered examples with original file origins; reject duplicate suggestions across complete properties, including unions. Run existing suggestion reinterpretation/lint against the resolved target.
  - Produce typed missing-name and contradiction errors with original authored locations. Preserve imported-error wrappers and mixed bare/explicit cycle chains. Retain all union-postfix and already-array rejection paths.
  - Assert scalar, expression-in-mapping, optional-object/required-inner-field, arrays, union-arm, and multi-hop behavior through resolved schemas and authored/effective validation, not only internal AST values.

- [x] **Conjunctive Lowering**
  - Owner: `simplified/convert.rs`, validation keyword consumers, and converter/validation tests. Lower repeated patterns and `not-empty` as conjunctions, retaining base type, format, presence, descriptions, and Darkmatter completion annotations where consumers can still find them.
  - Read grouped matches when emitting root-union metadata and selecting/validating arms, including simplified arms beside raw JSON Schema arms. Require every group only where matching already governs validation; leave ordinary single-schema matching passive.
  - Check every lowering path for keyword overwrites: bounds, schemes, defaults, examples, patterns, and repeated constraints. Avoid changes to unrelated schema layering and union behavior.
  - Add public validation assertions for values satisfying only one pattern, actual grouped root-union acceptance/rejection, equal/default numeric normalization, equal bounds, and schemas with no affected constraints retaining equivalent output.

**Checkpoint:** compare parsed definitions, resolved fragments, lowered JSON Schema, completion metadata, and authored/effective results for shared fixtures. Bare, `@this`, and cross-file imports agree after property names/origins are accounted for. Conflicts expose both declarations, and conjunctions survive actual validation.

## Phase 4 — Editor and Claudine Integration

Depends on Phase 3. The two tasks have disjoint package/file ownership and may use separate agents.

### Wave 5 — Update Public Consumers

- [x] **Editor Assistance**
  - Owner: `darkmatter/dmls/`. Offer local names bare, colliding built-ins with `@this`, and external references with their targets. Update cursor scanning, reference discovery, hover formatting, definition lookup, and suggestion checks, rather than adding an editor-only grammar.
  - Project missing-name and contradiction errors as schema-definition diagnostics. Assert exact primary ranges and related declaration URIs/ranges, including quoted/block scalars, inline schemas, and multi-hop referenced errors.
  - Preserve dependency edges for imported definitions. Through the existing in-process L1 LSP harness, edit an imported definition and verify refreshed completion, hover, definition, and diagnostics; local-only references must not invent external edges.
  - Verify same-file names in tagged schemas and top-level `$schema` namespaces, forward references, built-in collisions, and mixed-reference cycles. Retain the empty namespace for root unions and no exports for raw/whole-file reference schemas.

- [x] **Claudine Consumers**
  - Owner: `claudine/cli/src/completion/schema_completion/`, `claudine/lib/src/composition/schema/supplied.rs`, and dedicated tests. Update candidate metadata, root walking/preparation, candidate filtering, and supplied-value root-union matching to use all groups.
  - Prove `guide-intro.md` is offered for `*.md` AND `guide*`, while `notes.md` and `guide.txt` are not; test group-local exclusions, caller-origin behavior, and no candidates for an empty intersection.
  - Assert inherited presence and group conjunction through resolved schemas at the consumer boundary, including single-schema passivity and root-union arm selection. Search for remaining consumers expecting one list or one `pattern` keyword.
  - Use `CliProcessFixture` and shared recording/test-toolkit helpers where a child process is needed; no real provider, host environment leakage, audio, or terminal/browser session.

**Checkpoint:** L1 tests demonstrate editor-visible behavior and Claudine's public completion/validation outcomes. The inventory has no remaining single-list consumer, and dependency refresh works without reopening the editor.

## Phase 5 — Compatibility Audits and Current Documentation

Depends on Phase 4. Audit decisions precede corpus acceptance; documentation can proceed concurrently with the audit in separate files.

### Wave 6 — Audit and Document

- [x] **Artifact Audit**
  - Revisit every inventoried shipped schema and inline block, including the baseline. Record files checked, named-type references, required/generated declarations, duplicate constraints, collisions, intended presence, and any changes in `implementation-log.md`.
  - Where stripping previously supplied optionality, define a separate optional variant directly; aliasing the required type is insufficient. Retain required inner fields where appropriate. Do not introduce planned nullable syntax as an implemented workaround.
  - Compare affected parsed/resolved/lowered forms, metadata, and authored/effective results. Review every test and relevant comment expecting stripping; update assertions and explanations in the same behavior change. Keep deliberate invalid fixtures identifiable and avoid shorthand-only churn.
  - Record changed artifact owner areas and source-reading tests, so final validation covers actual readers. Do not relocate artifacts or update specification decisions to conceal a discrepancy.

- [x] **Current Documentation**
  - Update `darkmatter/docs/topics/schemas/{local-type-references.md,authoring-schemas.md,definition.md,dmls-schema-support.md}` and the Darkmatter README if its public schema summary is affected.
  - Remove unsupported-bare-name and stripping status notes; use actual `min(n)`/`max(n)` grammar. Explain namespace/built-in rules, the full merge table, canonical serialization, description precedence, optional outer objects versus required inner fields, generated phases, examples/suggestions, and match-group intersection/passivity.
  - Use compact examples and a Mermaid flow where it helps explain defining-file resolution. Mark nullable unions as planned until implemented. Current docs must not name or link feature/fix snapshots.
  - Update `.claude/skills/darkmatter/schema.md` and relevant skill summaries; update Claudine's completion/schema topic or skill where its single-list description changes. Update dependency docs only if dependencies actually change.

**Checkpoint:** audit disposition exists for every inventoried artifact, changed schemas express intended presence, and current docs teach all changed public behavior without stale grammar or references to this feature directory.

### Wave 7 — Prove Invocation Compatibility

- [x] **Corpus Validation**
  - Add or extend the passive shipped-artifact corpus check covering both schema folders, the baseline, and inline shipped schemas. Assert parse success and no expression execution, network fetch, file search for local names, or document mutation; use existing effects instrumentation rather than timing guesses.
  - Complete the block-metadata robustness matrix below using fixtures derived from real shipped schemas exercised by `md`. Verify outcomes through normal resolution/validation and public diagnostics; report complete row/column coverage if a cell fails.

- [x] **Invocation Fixtures**
  - Independently extend `darkmatter/cli/tests/l1/schema_validate.rs` and fixtures through its `CliProcessFixture`. Run normal `md schema validate` resolution with bare/local/external imports, inherited required/generated values, and constraints that require both patterns.
  - Assert public verdict/exit behavior and diagnostics for absent required values, authored generated absence, supplied invalid values, a multi-hop contradiction, and a grouped root union. Add direct library phase checks where CLI validation does not expose an effective/runtime phase.
  - Pair a shipped-schema positive control with deliberate invalid document values; do not stop at a parser test or snapshot showing an annotation. Keep corpus test files and CLI fixture ownership separate.

### Input Robustness Matrix

Type-expression strings, cursor scanning, and generated JSON Schema metadata are grammar/renderer surfaces and are excluded from the configuration matrix. The affected structured input is block `$constraints` on reused objects: merging must read its container, numeric bounds, and presence flags correctly. The table applies to YAML standalone schemas and YAML frontmatter, and separately to JSON-form schema sources accepted by the existing YAML reader; it does not add a TOML reader. Other unchanged envelope/configuration loaders remain outside this feature's scope.

`B` means the explicit baseline behavior with that field omitted, **not** an empty value: no block metadata, no corresponding bound, or no corresponding presence flag. `E` means a schema-definition error exposed through the public validation result, retaining the authored location where supported. The numeric columns apply independently to both fields; flag columns apply independently to both flags.

| Shape | `$constraints` | `min-keys` | `max-keys` | `required` | `generated` |
|---|---|---|---|---|---|
| Positive control | Valid object metadata is used in merged validation | Lower bound enforced | Upper bound enforced | Inherited presence enforced | Inherited phase semantics enforced |
| Absent | B; inline/use-site constraints still apply | B; use-site bound still applies | B; use-site bound still applies | B; use-site presence still applies | B; use-site flag still applies |
| Explicit null | E | E | E | E | E |
| Wrong whole-field type | E for scalar/list | E for non-integer/list/object | E for non-integer/list/object | E for anything except `true` | E for anything except `true` |
| One invalid element/member | E; valid members do not hide the malformed member | E for a list containing a number and invalid item | E for a list containing a number and invalid item | E for mixed lists | E for mixed lists |
| All invalid elements/members | E; never an empty metadata map | E for any list | E for any list | E for any list | E for any list |
| Empty | `{}` explicitly contributes no block constraints; `[]` is E | E for empty string/list/object; zero is a valid bound | E for empty string/list/object; zero is a valid bound | E for empty string/list/object; `false` is E | E for empty string/list/object; `false` is E |
| Duplicate key | E for duplicated `$constraints` in its object | E for repeated numeric key | E for repeated numeric key | E for repeated flag key | E for repeated flag key |
| Trailing/invalid content | E; malformed source is never a successful simplified-schema validation | E | E | E | E |

Use one data-driven test per accepted source format, walking the entire matrix with one edit per cell to a real-tool fixture and a positive control proving each field changes the public verdict or phase behavior. Include source edits introducing duplicate keys before decoding, since a map AST cannot reveal lost duplicates. Make each numeric bound load-bearing with a document on its violating side; make generated load-bearing through authored/effective comparison. Omitted metadata and `{}` may have equal effective behavior because this format explicitly defines both as adding no constraints, but null and malformed input must never become either.

Before declaring coverage complete, inspect touched paths for `#[serde(default)]`, absence/null-collapsing `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` hiding load-bearing parse failures. Distinguish deliberate unchanged classification from a successful validation that dropped malformed metadata. Fix any in-scope silent loss at the normal public resolution seam, without redesigning unrelated loaders.

**Checkpoint:** the shipped corpus passes, the accepted-format matrices are complete, CLI fixtures prove the changed semantics through the normal invocation path, and audit changes have dedicated regression assertions.

## Phase 6 — Final Verification and Review Handoff

Depends on all earlier phases. No implementation spike is repeated here.

### Wave 8 — Verify Sequentially

- [x] **Acceptance Gates**
  - Run `just test` and then `just lint` in `darkmatter/`, covering the library, CLI, DMLS, and the area's normal crate set. Run `just test` and `just lint` in `claudine/` because its consumers changed, plus `just test` in every additional area whose shipped schema changed. Use nextest, not `cargo test`; do not run workspace-wide gates or terminal/browser tiers for this L1 feature.
  - Load the `rust-testing` skill and test-toolkit guidance before writing the planned tests. If new tests read repository artifacts, declare those inputs in the documented forms so existing CI scope selects their readers; add no new CI matrix cell or gate.
  - Use existing passing OS evidence where it qualifies; otherwise obtain the required environment evidence using the existing workflow and `os` skill. Record which environments were actually verified, and do not claim four-OS execution from a local macOS run.
  - Resolve failures, then rerun the affected checks. Review behavior-changing docs/comments, typed error wrappers, passive/context guards, and all grouped-pattern consumers. Do not run `cargo fmt`.

- [x] **Review Handoff**
  - Close the acceptance-to-test map with concrete test names and results for criteria 1–10. Record corpus/audit coverage, changed files, representation choices, spec departures if any, validation commands/environments, and material limitations in the implementation log.
  - Confirm there is no remaining stripping assertion, local-name schema-root lookup, unresolved consumer migration, speculative nullable/union syntax, or unsupported claim in current docs. Check the plan's phase/task state accurately reflects completed work.
  - Deliver implementation complete, ready for review. Leave lifecycle movement and any commit/push to the author or a separate explicit instruction.

**Checkpoint:** required area gates pass, all acceptance criteria have public evidence, audits and current documentation are complete, and the review handoff makes any remaining environment limitation explicit.
