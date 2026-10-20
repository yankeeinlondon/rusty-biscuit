---
total_phases: 4
created: 2026-10-11
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.continued.yaml
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/README.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/dmls/docs/diagnostics.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/compose.md
source_code:
  - darkmatter/lib/src/markdown/schemas/simplified/source.rs
  - darkmatter/lib/src/markdown/schemas/simplified/standalone.rs
  - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
  - darkmatter/lib/src/markdown/schemas/simplified/cursor.rs
  - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
  - darkmatter/lib/tests/l1/main.rs
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/tagged.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/nested.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/root-union.serialized.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/anchors.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-indentless.indentless.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/spec-continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/continued.continued.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.conventional.yaml
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/blank-line.continued.yaml
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/suggestions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/l1/lsp_session.rs
  - darkmatter/dmls/tests/l1/schema_yaml_spellings.rs
  - darkmatter/dmls/tests/l1/main.rs
  - darkmatter/cli/tests/l1/schema_validate.rs
documentation:
  - darkmatter/lib/tests/fixtures/schema_yaml_spellings/README.md
  - darkmatter/docs/topics/schemas/definition.md
  - darkmatter/docs/topics/schemas/dmls-schema-support.md
  - darkmatter/dmls/docs/diagnostics.md
  - .claude/skills/darkmatter/schema.md
  - .claude/skills/darkmatter/compose.md
completed_phase: 4
implemented: true
packages:
  - darkmatter
  - dmls
  - darkmatter-cli
---

# Schema Loader YAML Spellings Implementation Plan

## Phase 1 — Establish Contracts and Fixtures

### Necessary Rules

The specification has no open design questions. The following are implementation rules already decided by the specification or required to preserve existing behavior:

- The plan lives in Claudine, but implementation belongs to the `darkmatter` library, `darkmatter-cli`, and `dmls`. Do not change Claudine behavior or the specification's metadata as part of this work.
- Strict schema authoring gains indentless sequences and continued **plain** scalars only. Split these permissions from tolerant-only block scalars, tags, quoted continuations, and empty values. Preserve conventional compact mappings, mapping-value anchors, aliases, and tolerant frontmatter behavior. Reuse the existing locator and decoder; do not add another locator.
- Decode using `serde_yaml_ng` semantics, including blank lines, comments, and trailing whitespace. Verify decoded text against the semantic value. Projection disagreement is an error, not an approximate successful range.
- Structural spans, suggestion candidates, and grammar-error spans use the same decoded-to-authored coordinates. Nonempty spans use `DecodedScalar::project`; apply a caller's byte offset exactly once afterward. Preserve original LF/CRLF buffers. DMLS performs negotiated LSP position conversion separately.
- Keep `SchemaError::SchemaDocument` for recognized malformed standalone schemas. Recover inner grammar positions in DMLS without redesigning that wrapper. Preserve existing diagnostic codes, severity, and projection-failure fallbacks. Never present the projection error's internal `0..0` sentinel as a successful token span.
- Envelope recognition stays unchanged: pure `$schema` is the sole root key; tagged documents contain exactly `kind: schema` and a `types` mapping. Raw JSON Schema interpretation remains unchanged. The classifier currently returns `Ok(None)` for undecodable YAML or unrecognized envelopes; distinguish that result from acceptance by a schema-loading consumer.
- No performance or exploratory spike is scheduled. The specification explicitly requires existing decoder/scanner reuse and excludes a benchmark gate; the code inspection identifies the relevant seams. Any proposal for wider measurement or another YAML presentation requires an author ruling outside this plan.
- All new tests are portable L1 tests. Use existing isolated CLI/LSP fixtures and nextest recipes; open no terminal or browser windows. Add no CI cells, dependencies, public span representation, or format-normalization step. Do not run `cargo fmt`, commit, or move this feature to `_completed`.

### Work and Success

Extend strict source location and suggestion scanning to consume the two valid YAML spellings, then make DMLS use those coordinates for successful queries and malformed definitions. Success means structurally equal resolved JSON Schema for equivalent presentations, exact authored ranges for library and editor results, and normal CLI validation that accepts a valid instance and rejects an invalid one. The existing strict exclusions, tolerant lookups, shipped schema corpus, and ordinary invocation tests must still pass.

Waves are numbered globally and execute in order. Tasks within a wave may be delegated concurrently when their file ownership is disjoint; a single task remains sequential. Each wave ends at its stated checkpoint. Parallel implementation agents must receive the session's non-interactive constraints. This planning task does not launch implementation agents.

### Wave 1 — Contract Inventory

- [x] **Map Consumers**
  - Trace `source.rs`, `yaml_scalar.rs`, `standalone.rs`, and `cursor.rs` under `darkmatter/lib/src/markdown/schemas/simplified/`, plus DMLS standalone diagnostics in `darkmatter/dmls/src/diagnostics/frontmatter.rs` and its completion/hover callers.
  - Record the minimum shared seam needed for strict/tolerant permissions, complete-scalar suggestion scanning, occurrence identity, cursor lookup, and malformed grammar-range recovery. Prefer existing APIs; any unavoidable accessor must expose existing mapping behavior rather than create another parser.
  - Inspect the existing tests in `schemas_source_projection.rs`, standalone-envelope coverage, CLI `schema_validate.rs`, and DMLS `suggest_constraint_phase1.rs`/`lsp_session.rs`. Identify the existing shipped-corpus test and normal-invocation tests to retain.
  - Checkpoint: every acceptance criterion has a named owning surface and validation path; no implementation depends on an unrecorded design choice.

### Wave 2 — Fixture Contract

- [x] **Build Matrix**
  - Implement the YAML robustness table below as one table-driven L1 test, starting from a captured serializer-produced schema fixture with recorded provenance. Use separate pure, tagged, root-union, and nested-property variants from that seed where a field only exists in one envelope. Reset to the control fixture and make exactly one edit per cell; do not hand-write unrelated cases.
  - Assert public classification and normal loading/validation results, including the effective property's presence and behavior. A semantic YAML value or successful low-level parse alone is insufficient evidence.
  - Add paired conventional/new-presentation fixtures for later tasks: both envelopes, pure root union, nested property union, compact mapping items, mapping-value anchors, and siblings after lists and continued values. Include a serializer-produced indentless fixture and explicitly authored physical continuation variants.
  - Checkpoint: controls prove each tested declaration affects resolved semantics or validation. Capture baseline outcomes without changing envelope recognition; new-presentation cases may remain failing until Phase 2.

### Input Robustness Matrix

Only YAML schema authoring input changes. JSON Schema, TOML/configuration readers, and validated instance fields are not changed readers and do not acquire new matrices. Type-expression constraints, descriptions, and suggestion arguments are text grammar within the definition scalar; test their folded coordinates in Phase 2 rather than treating them as separate configuration fields.

Each column represents a load-bearing envelope field or repeated payload field. Exercise the definition column for named definitions and nested mapping properties; exercise union members in string and mapping arms. These rules also cover sibling definitions used to detect overconsumption.

Legend: **D** = classifier declines (`Ok(None)`); normal loading must not report successful SimplifiedSchema application. **E** = recognized schema returns `SchemaDocument` and normal loading fails. **A** = accepted with the specified observable result. **N/A** = shape cannot apply to that field, with the corresponding containing-field case tested instead.

| Shape | `kind` (tagged seed) | `types` (tagged seed) | `$schema` (pure seed) | Definition value (named/nested) | Property-union members | Root-union members |
| --- | --- | --- | --- | --- | --- | --- |
| Control | A: `schema` claims tagged envelope | A: mapping's definitions apply | A: mapping or root union applies | A: constraint changes validation | A: both declared arms apply | A: both mapping arms apply |
| Absent | D: remaining `types` alone is unrecognized | E: required mapping missing | D: no pure envelope | A: property is undeclared; no implicit empty definition | A if another arm remains: removed arm no longer validates; removing all is empty-row E | A if another arm remains: removed arm no longer validates; removing all is empty-row E |
| Explicit null (`null` and empty YAML value) | D: does not claim tagged envelope | E: not a mapping | E: invalid declaration | E: not an absent definition | E: any null arm invalidates union | E: any null arm invalidates union |
| Wrong whole type | D: number/list does not claim tagged envelope | E: number/list, including indentless list | E: number/bool payload | E: number/bool value | E: numeric replacement of union field | E: numeric replacement of union payload |
| One wrong element | N/A: scalar; list covered above | E: sequence replacing mapping | E: mixed valid/number root union | E: mixed valid/number property union | E: reject whole definition; do not drop member | E: reject whole declaration; do not drop member |
| Every element wrong | N/A: list covered above | E: sequence replacing mapping | E: all-number root union | E: all-number property union | E: never empty accepted union | E: never empty accepted union |
| Empty | D: empty string/container does not claim envelope | A: explicit `{}` is an empty definition catalog; `[]` is E | A: explicit `{}` is empty schema; `[]`/empty reference string is E | A: `{}` is empty object shape; `[]`/empty expression string is E | E: `[]` is empty union | E: `[]` is empty union |
| Duplicate key | D: duplicate `kind` invalid YAML, no last-wins | D: duplicate `types` invalid YAML | D: duplicate `$schema` invalid YAML | D: duplicate named/nested key invalid YAML | D: duplicate key inside mapping arm; repeating scalar arms is not a duplicate key | D: duplicate key inside mapping arm; repeating arms is not a duplicate key |
| Trailing/invalid content | D for invalid YAML or second document | D for invalid YAML or second document | D for invalid YAML or second document | E for valid YAML carrying invalid expression text; D for invalid YAML | E for invalid grammar in an arm; D for invalid YAML | E for invalid arm/reference syntax; D for invalid YAML |

For D cases caused by invalid YAML, assert the normal schema-loading path rejects the buffer rather than accepting a fallback schema. For unrecognized but valid YAML, assert the existing raw-schema interpretation/classification remains unchanged and no SimplifiedSchema application is claimed. Distinct absence/null results are intentional; a null field cannot disappear through `Option<T>`.

Also assert tagged extra root keys are E and extra root keys beside pure `$schema` cause D. Review the touched code for `#[serde(default)]`, absence/null-conflating `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` that could discard load-bearing input. Existing classifier/locator fallbacks are intentional only where their public contracts explicitly say so. A review finding on a cell must record every other cell in that row and column.

## Phase 2 — Extend Shared Source Projection

Prerequisite: Phase 1 contract and fixture inventory. Both tasks edit `source.rs`; keep them in separate waves to avoid conflicting changes.

### Wave 3 — Strict Locator

- [x] **Separate Permissions**
  - Replace the broad `multi_line_scalars` coupling with the smallest explicit strict/tolerant policy needed to share indentless handling and plain continuation decoding. Keep tolerant-only presentations and empty-value handling independently gated.
  - Update both empty mapping-value descent and anchored mapping-value descent to recognize same-indent sequences. Preserve compact-item ownership and stop at sibling keys/items.
  - Decode whole continued plain values in mapping and sequence positions with `decode_scalar_node`; retain blank lines in the original slice even though `source_lines` omits them. Check decoded text against semantic values before projecting structure.
  - Add public-result regressions for both envelopes, nested mapping arms, compact mappings, anchors/aliases, siblings, and each unchanged strict exclusion. Preserve typed projection failures and standalone wrapping.
  - Checkpoint: equivalent schemas resolve to structurally equal JSON Schema; structural spans select continuation tokens and enclosing cross-fold expressions exactly; tolerant tests remain green.

### Wave 4 — Scalar Coordinates

- [x] **Project Suggestions**
  - Make `scan_value_scalars` consume complete continued plain scalars and advance past consumed physical lines. Retain existing flow/quoted/anchor behavior; prevent continuation punctuation from being scanned as another value.
  - Reuse the existing scalar decoder and `DecodedScalar::project` for nonempty suggestion ranges instead of separately projecting endpoints with `raw_offset`. Check semantic agreement and preserve each repeated definition's occurrence, including identical expressions with and without suggestions before the target.
  - Verify cursor lookup through the shared decoded scalar mapping in `cursor.rs`; adjust only seams that fail continuation or indentless-item lookup, retaining existing clamping behavior and partial-expression support.
  - Test folds within type expressions, constraints, descriptions, and suggestion candidates; multiple continuations, a blank line, trailing comments/whitespace, and no final newline. Compare decoded text to `serde_yaml_ng` and semantics to supported equivalent spellings.
  - Assert exact original-buffer byte ranges for tokens on continuation lines and across folds, lint-invalid suggestions that remain advisory, repeated definitions, CRLF, non-ASCII, and nonzero caller offsets. Include a genuine projection mismatch that cannot return a fabricated successful range.
  - Checkpoint: all library presentation and range tests pass, including the Phase 1 matrix; structural and suggestion coordinates agree.

## Phase 3 — Validate CLI and Editor Consumers

Prerequisite: Phase 2 completes the shared mapping/cursor seam. The two Wave 5 tasks can run concurrently with disjoint ownership: DMLS production/tests versus CLI tests. Shared library changes discovered here go back through the Phase 2 owner before integration.

### Wave 5 — Consumer Paths

- [x] **Recover Diagnostics**
  - In DMLS standalone diagnostics, identify the offending scalar occurrence and project the existing grammar error's expression-relative span through its decoded mapping. Preserve inner-definition versus outer-declaration diagnostic classification and existing fallbacks when decoding/projection genuinely fails.
  - Cover the specified `bogus` example through the existing L1 LSP harness: underline exactly `bogus` on physical line 5, displayed columns 5–9 inclusive (zero-based LSP line 4, characters 4..9 for ASCII). Preserve code and severity; do not require malformed schemas to load.
  - Add hover/completion assertions for continued text and indentless union items, including complete and partially typed expressions, sibling node selection, and repeated identical definitions. Compare node/token meaning with conventional spellings and assert authored ranges.
  - Negotiate UTF-16 and use non-BMP text before a target token to distinguish byte, Unicode-scalar, and UTF-16 offsets. Cover CRLF and advisory suggestion diagnostics without normalizing the source.
  - Checkpoint: public LSP results have exact ranges and correct node selection; malformed grammar and genuine projection failure follow their respective paths.

- [x] **Prove Validation**
  - Extend `darkmatter/cli/tests/l1/schema_validate.rs` using `CliProcessFixture` and the test toolkit. Reference isolated schema files containing both new presentations and compare the library's resolved JSON Schema values with conventional forms using structural equality.
  - For each spelling, run `md schema validate` on a valid and invalid instance, asserting reported validation outcome and the specific applied constraint/type. Use real-file references through the existing invocation context; do not introduce another resolver.
  - Include both envelope contracts and an indentless pure root union; retain the `types`-sequence rejection. Keep process cases compact while the library matrix carries exhaustive shape coverage.
  - Checkpoint: successful process exit cannot hide a schema that was never applied; invalid instances demonstrably fail against the referenced schema.

## Phase 4 — Document and Verify Completion

Prerequisite: Phase 3 consumer results agree with library behavior.

### Wave 6 — Behavior Documentation

- [x] **Update Boundary**
  - Update `darkmatter/docs/topics/schemas/definition.md`, especially “Source-Aware Presentation Boundary,” with compact indentless and physically continued plain-scalar examples, sibling behavior, blank-line folding, authored ranges, and unchanged exclusions. Keep docs self-contained and free of feature-directory references.
  - Update locator, suggestion-projector, tolerant-locator, cursor, and DMLS comments wherever behavior changed; remove claims that strict parsing rejects all multiline scalars. Preserve useful contracts rather than narrating implementation steps.
  - Update `.claude/skills/darkmatter/schema.md` if its boundary notes need clarification. Inspect the area README and change it only if a public claim conflicts. No dependency-doc change is needed without a dependency change.
  - Checkpoint: documentation accurately distinguishes supported plain continuations from excluded block/quoted scalars and describes byte ranges versus editor positions.

### Wave 7 — Final Verification

- [x] **Run Gates**
  - Run `just test` and then `just lint` in `darkmatter/`, sequentially; these cover library, CLI, and DMLS through the existing recipes. Do not run a workspace-wide gate or add an L2/L3/browser run for this passive parser change. Respect the prohibition on running `cargo fmt`; if lint exposes formatting drift, correct only touched lines manually.
  - Confirm the shipped schema corpus, supported anchor/alias tests, tolerant-frontmatter tests, strict exclusion tests, robustness matrix, source-range tests, and existing normal-invocation tests actually ran and passed. Use existing recipe filters for focused iteration before the final full area run.
  - Review portability of fixtures: explicit LF/CRLF strings, isolated homes/CWD, runtime fixture paths, and no shell-dependent setup or focus changes. Tests must remain eligible for existing macOS/Linux/Windows/WSL2 coverage; report only environments actually evidenced locally, without adding CI cells.
  - Review the final diff against acceptance criteria 1–7, inspect load-bearing parse smells, and verify no unrelated source, envelope semantics, raw JSON Schema behavior, or public error shape changed. Record any specification departure in an implementation log, not by editing the specification.
  - Checkpoint: all required gates pass and evidence is recorded. Terminal state is **implementation complete, ready for review**; the author owns lifecycle closure and moving the feature.
