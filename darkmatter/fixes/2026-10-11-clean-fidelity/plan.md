---
area: darkmatter
fix: 2026-10-11-clean-fidelity
total_phases: 6
created: 2026-10-11
phase: 5
agent: "codex/gpt-6.1-sol"
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/style/descriptor.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/src/style/error.rs
    - darkmatter/lib/src/style/schema/mod.rs
    - darkmatter/lib/src/style/coverage_tests.rs
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/lib/tests/l1/clean_rules.rs
    - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/fixtures/clean/style-markdown-control.md
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/tests/l1/layout_style_frontmatter.rs
    - darkmatter/dmls/src/config/mod.rs
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/stdio_subprocess.rs
    - darkmatter/dmls/tests/l1/style_markdown_parity.rs
    - darkmatter/dmls/tests/l1/workspace_formatting_settings.rs
    - darkmatter/dmls/tests/fixtures/formatting/dmls.toml
    - darkmatter/dmls/tests/fixtures/formatting/settings.json
docs_updated_during_phase_2:
    - darkmatter/dmls/docs/editors/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_3:
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/src/markdown/cleanup/compare.rs
    - darkmatter/lib/src/markdown/cleanup/edits.rs
    - darkmatter/lib/src/markdown/cleanup/protect.rs
    - darkmatter/lib/src/markdown/cleanup/structure.rs
    - darkmatter/lib/src/markdown/cleanup/transform.rs
    - darkmatter/lib/src/markdown/cleanup/wrap.rs
    - darkmatter/lib/src/markdown/cleanup/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/perf_profile.rs
    - darkmatter/lib/src/markdown/cleanup/tests/mod.rs
    - darkmatter/lib/src/markdown/cleanup/tests/body_support.rs
    - darkmatter/lib/src/markdown/cleanup/tests/directive_fixed_points.rs
    - darkmatter/lib/src/markdown/cleanup/tests/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/tests/fidelity_repro.rs
    - darkmatter/lib/src/markdown/cleanup/tests/guard.rs
    - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
    - darkmatter/lib/src/markdown/cleanup/tests/rules.rs
    - darkmatter/lib/src/markdown/cleanup/tests/source_fidelity.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
docs_updated_during_phase_3:
    - darkmatter/docs/cli/clean.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_4:
    - claudine/cli/src/commands/wrap/inline.rs
    - claudine/lib/src/composition/closure/tests.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/src/commands/clean/json.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/render.rs
    - darkmatter/cli/tests/l1/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/cli/tests/l1/clean_frontmatter.rs
    - darkmatter/cli/tests/l1/clean_json.rs
    - darkmatter/cli/tests/l1/clean_parity.rs
    - darkmatter/cli/tests/l1/clean_rules.rs
    - darkmatter/cli/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/providers/formatting.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/tests/l1/formatting_transaction.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/fuzz/fuzz_targets/markdown_parser.rs
    - darkmatter/lib/src/markdown/cleanup/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/src/markdown/cleanup/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/lists.rs
    - darkmatter/lib/src/markdown/cleanup/markers.rs
    - darkmatter/lib/src/markdown/cleanup/mod.rs
    - darkmatter/lib/src/markdown/cleanup/opaque.rs
    - darkmatter/lib/src/markdown/cleanup/perf_profile.rs
    - darkmatter/lib/src/markdown/cleanup/protect.rs
    - darkmatter/lib/src/markdown/cleanup/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/cleanup/tables.rs
    - darkmatter/lib/src/markdown/cleanup/tests/blockquote.rs
    - darkmatter/lib/src/markdown/cleanup/tests/brackets.rs
    - darkmatter/lib/src/markdown/cleanup/tests/emphasis.rs
    - darkmatter/lib/src/markdown/cleanup/tests/escapes.rs
    - darkmatter/lib/src/markdown/cleanup/tests/guard.rs
    - darkmatter/lib/src/markdown/cleanup/tests/lists.rs
    - darkmatter/lib/src/markdown/cleanup/tests/mod.rs
    - darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs
    - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
    - darkmatter/lib/src/markdown/cleanup/tests/source_fidelity.rs
    - darkmatter/lib/src/markdown/cleanup/tests/tables.rs
    - darkmatter/lib/src/markdown/compose/block_pairs.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/directives_api.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/render_tree/disclosure_scan.rs
    - darkmatter/lib/tests/common/clean_parity/mod.rs
    - darkmatter/lib/tests/l1/clean_compose_equivalence.rs
    - darkmatter/lib/tests/l1/clean_parity.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_4:
    - darkmatter/lib/README.md
    - darkmatter/dmls/docs/editors/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/cleanup.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/document.rs
    - darkmatter/lib/src/markdown/cleanup/body.rs
    - darkmatter/lib/tests/l1/clean_real_world_prompt.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/common/clean_parity/mod.rs
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md
    - darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.provenance.yaml
    - darkmatter/cli/src/args/command.rs
    - darkmatter/cli/src/commands/clean.rs
    - darkmatter/cli/tests/l1/clean_fidelity.rs
    - darkmatter/dmls/src/providers/formatting.rs
docs_updated_during_phase_5:
    - darkmatter/README.md
    - darkmatter/lib/README.md
    - darkmatter/cli/README.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/cli/clean.md
    - darkmatter/docs/inline/cleaning.md
    - darkmatter/docs/lsp/features.md
    - darkmatter/docs/darkmatter-compose-pipeline.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/layouts-and-style.md
    - darkmatter/dmls/docs/editors/README.md
    - darkmatter/dmls/docs/features.md
docs_created_during_phase_5:
    - darkmatter/docs/topics/markdown-formatting.md
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/cleanup.md
    - .claude/skills/darkmatter/compose.md
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
---

# Clean Fidelity Implementation Plan

## Phase 1: Establish Contracts and Reduce Design Risk

### Necessary Rules

This plan implements the specification's invariants, ten formatting rules,
shared raw-document cleanup transaction, and CLI/DMLS parity. Success means
supported documents actually format without changing meaning; returning the
original with a diagnostic is the safety response to a defect, not proof that
the formatter works. All fourteen acceptance criteria must have recorded
evidence before handoff.

The following rulings govern implementation. They distinguish decisions already
made in the specification from implementation choices and remaining design work.

1. **Defaults and provenance — specified.** Preserve authored soft breaks and
   uniform/sequential ordered markers. Resolve each leaf independently as
   explicit CLI override → document → DMLS workspace → runtime default.
   Retain whether padding was explicitly selected and which list boundaries it
   affects; a global permission to ignore tight/loose differences is unsafe.
   Schema defaults never become inserted frontmatter values.
2. **Validation and repair — specified.** Resolve settings once from original
   parseable YAML, or a syntax-repaired candidate when necessary. Invalid or
   executable document settings invalidate the entire `style.markdown` object,
   with authored diagnostics; other precedence levels remain usable. Unrelated
   schema findings do not invalidate valid rules. Candidate repairs used to
   discover `frontmatter: false` are discarded. Unrepaired invalid YAML means
   `invalid`, no write/edit, CLI exit 1; a fidelity refusal means `declined`,
   original output, no applied repairs, CLI exit 0.
3. **Byte preservation — specified.** No global newline, blank-line, emphasis,
   or whitespace normalization remains. The all-body-rules-off path returns
   exact body bytes. Preserve BOM, empty frontmatter, delimiters, comments,
   boundary spacing, mixed terminators, and final-newline presence except for
   explicitly authorized edits. Inserted body lines use the local neighboring
   terminator, with LF only for an empty body. Code bytes and protected spans
   are checked separately from normalized parser events.
4. **Protection and comparison — specified.** Use existing directive-family
   recognizers, including disclosure and quoted shell-block behavior. Ordinary
   content within conditional regions remains formatable. Do not split at
   directives and clean chunks independently. Compare full protected bytes,
   neighboring blank-line state, order, and container placement separately from
   the masked semantic view. Code examples resembling directives are code.
5. **Diagnostic ownership — specified.** Move the clean diagnostic model into
   Darkmatter; do not add a Markdown code to biscuit-file's YAML enum. Preserve
   version-1 JSON fields and old spellings. A fallback has
   `clean.fidelity-fallback`, `non_deterministic_find`, `repairs: []`, and a
   whole-document authored span; its message locates the first affected input
   block. Use nearest input boundary/EOF for output-only blocks. Keep stable
   source order and distinguish malformed regions, serialization errors, and
   semantic mismatches.
6. **DMLS migration — implementation ruling.** Validate and translate each
   authored configuration source before merging it. Serialized defaults must
   not manufacture legacy keys or make valid new settings appear mixed.
   Within one source, the presence of either legacy key alongside
   `formatting.markdown`, even an empty object, is a mixed shape and is rejected.
   Across sources, normalize first, then preserve editor-overlay-over-file
   precedence. Emit one migration warning per authored legacy key per load or
   configuration change, not on every formatting request. Invalid overlays
   retain the valid file settings; invalid files contribute no overrides and
   produce a visible diagnostic. Unknown formatting keys are rejected; do not
   broaden this change into unrelated server configuration policy.
7. **Null and duplicate inputs — implementation ruling.** Absent and explicit
   null formatting values are recorded separately, although the specification
   deliberately gives both the same no-override outcome. TOML has no null.
   YAML/TOML duplicate keys are rejected, never last-wins. DMLS's public editor
   settings input is already `serde_json::Value`, which cannot contain duplicate
   keys; raw JSON duplicate-name behavior belongs to the existing LSP transport
   and is not changed by this fix. Test and document that boundary rather than
   claiming the settings loader detects information the transport discarded.
   Widening this fix to reject duplicate names in raw LSP messages requires an
   author ruling; it is not a prerequisite for formatting changes.
8. **Rule validation — specified plus implementation ruling.** Integer widths
   are 1–1000; `true` means 120. Integer indentation is 2/4/8 or the string
   `preserve`; quoted numerals are invalid. Reject unsafe language strings,
   unknown rule names, and arrays instead of scalar leaves. Null object roots
   mean no overrides; empty objects are valid explicit objects with no leaves.
   Legacy `fixed_width` migrates only if it is an integer in the supported width
   range; absent/null means no width override. Unsupported legacy values are
   reported, never coerced.
9. **Parse work — specified.** Plain body cleanup has at most two counted
   parses; cleanup with existing separate reflow has at most three. Verify only
   the final candidate, after reflow/reassembly. Instrument scanner parses
   separately or reuse instrumented parse data; never report the cleanup
   counter as total work. No performance spike or benchmark gate is needed.
10. **Scope and execution — specified/repository rules.** No CLI flag redesign,
    heading-level feature, composition during cleanup, network access, or
    recapture of CWD/home/repository. Use existing contexts, `FileReference`,
    biscuit-file YAML utilities, and schema-cleaning machinery. Tests are L1
    text, filesystem, and protocol checks unless a real display behavior is
    newly introduced. Do not commit, run `cargo fmt`, move the fix, or run
    `just complete` as part of implementation.

### Wave 1 — Sequential contract preparation

- [x] **Map entrypoints**
  - Inventory `Markdown::cleanup*`, `cleanup_content*`, fixed-width and standalone
    strip/reflow helpers, `md clean`, top-level `md FILE --save`, and DMLS's
    provider/router. Record which are guarded operations and their parse ceilings.
  - Trace `cleanup/mod.rs`, `lists.rs`, `blockquote.rs`, `opaque.rs`, and
    `reflow/semantic.rs`; classify every existing pass as artifact repair,
    rule-owned transform, or removal. Confirm the root-cause hypotheses rather
    than treating them as established findings.
  - Inventory raw repair responsibilities in
    `cli/src/commands/clean/frontmatter_repair.rs`, shared `schemas/clean.rs`,
    style parser/descriptors, baseline inclusion, DMLS config sources, and
    source-map/notification plumbing. Begin `implementation-log.md` with these
    maps and an acceptance-criterion-to-test checklist.

- [x] **Prototype fidelity**
  - Run one bounded correctness spike, before production transforms: prototype
    a source-span/container representation against E1/D1/C1/A1/B1/F1, a quoted
    shell block, an expression, an unused reference definition, and mixed line
    endings. Determine how directive-aware parsing avoids lazy continuation
    while preserving lists spanning directive boundaries.
  - Prove that edits can retain complete list-child prefixes, protected adjacency,
    raw code bytes, and original source coordinates. Reuse one input event/range
    collection; identify extra scanner parses explicitly.
  - Record the chosen approach and discarded prototype in the log. Prefer bounded
    edits to authored spans where serialization cannot preserve disabled regions;
    do not commit to a serializer replacement without this evidence. This spike
    is run once and is not scheduled as a later re-check.

- [x] **Define shared contracts**
  - Finalize public raw-source options/result types: text, `unchanged`/`changed`/
    `declined`/`invalid`, positioned diagnostics, and only emitted repairs.
    Define internal optional overrides separately from resolved defaults and
    explicit padding permissions. Body-only result APIs and string wrappers
    must share the protected transform/guard implementation.
  - Freeze module ownership and signatures before parallel work: repair/result
    modules, formatting rules, source protection, semantic comparison, and thin
    consumer adapters. State dependencies without adding a generic pipeline
    framework. Reconcile the robustness tables below with existing loader
    behavior; record departures before editing.
  - Checkpoint: the spike demonstrates feasibility, every pass has a disposition,
    all acceptance criteria have an owner, and any new author-only decision is
    explicit in the log. In a non-interactive run, do independent work but do not
    silently implement an unresolved widening of scope.

### Input Robustness Matrix

These tables cover new/changed configuration readers, not the Markdown grammar,
renderers, or the existing YAML editor's full input domain. Each leaf is
load-bearing. Use a checked-in normal-tool fixture for each format: an authored
Markdown document used by `md`, a real `.dmls.toml`, and settings captured from a
normal editor configuration request. Each matrix test runs the unedited positive
control, then exactly one mutation per applicable cell through public cleanup or
configuration/formatting behavior. Assert text/outcome and visible diagnostics,
not merely deserialization success. Select input content for which each setting
changes the public result; use repairable YAML for the `frontmatter` control.

Cell notation: **F** = fall through without an override; **N** = explicitly null,
tracked separately and intentionally falling through; **D** = diagnostic and
discard the entire document markdown object; **W** = configuration diagnostic,
reject that source's formatting overrides and use valid lower settings;
**I** = invalid YAML with no emitted document when safe repair cannot resolve it;
**X** = not representable/applicable at this boundary. Controls name the positive
fixture value; full legal-value coverage belongs to Phase 3.

#### YAML document leaves: `style.markdown`

| Shape | frontmatter | table-layout | fixed-width | unwrap | indentation | ordered-lists | list-padding | list-item-padding | list-interior-padding | default-language |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | false | false | 80 | true | 2 | false | false | one | all | sh |
| Absent | F | F | F | F | F | F | F | F | F | F |
| Explicit null | N | N | N | N | N | N | N | N | N | N |
| Wrong whole type | D | D | D | D | D | D | D | D | D | D |
| One wrong element | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array |
| Every element wrong | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array | D: array |
| Empty | D | D | D | D | D | D | D | D | D | `""`: no language; collections D |
| Duplicate key | I | I | I | I | I | I | I | I | I | I |
| Invalid/trailing YAML | I | I | I | I | I | I | I | I | I | I |

Use unrepaired duplicate/trailing cases to prove no arbitrary winner or accepted
garbage; ordinary Markdown after the closing frontmatter delimiter is valid body,
not trailing YAML. If the existing safety-proven repair machinery accepts a case,
assert the explicit repair and final valid document rather than labeling it I.

#### TOML workspace leaves: `formatting.markdown`

| Shape | frontmatter | table-layout | fixed-width | unwrap | indentation | ordered-lists | list-padding | list-item-padding | list-interior-padding | default-language |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | false | false | 80 | true | 2 | false | false | one | all | sh |
| Absent | F | F | F | F | F | F | F | F | F | F |
| Explicit null | X | X | X | X | X | X | X | X | X | X |
| Wrong whole type | W | W | W | W | W | W | W | W | W | W |
| One wrong element | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array |
| Every element wrong | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array |
| Empty | W | W | W | W | W | W | W | W | W | `""`: no language; collections W |
| Duplicate key | W | W | W | W | W | W | W | W | W | W |
| Invalid/trailing TOML | W | W | W | W | W | W | W | W | W | W |

#### JSON editor leaves: `formatting.markdown`

| Shape | frontmatter | table-layout | fixed-width | unwrap | indentation | ordered-lists | list-padding | list-item-padding | list-interior-padding | default-language |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Control | false | false | 80 | true | 2 | false | false | one | all | sh |
| Absent | F | F | F | F | F | F | F | F | F | F |
| Explicit null | N | N | N | N | N | N | N | N | N | N |
| Wrong whole type | W | W | W | W | W | W | W | W | W | W |
| One wrong element | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array |
| Every element wrong | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array | W: array |
| Empty | W | W | W | W | W | W | W | W | W | `""`: no language; collections W |
| Duplicate key | X: Value | X: Value | X: Value | X: Value | X: Value | X: Value | X: Value | X: Value | X: Value | X: Value |
| Invalid/trailing JSON | reject message | reject message | reject message | reject message | reject message | reject message | reject message | reject message | reject message | reject message |

Exercise malformed JSON through the transport with a subsequent valid request
to observe rejection and unchanged effective rules. Record raw duplicate-name
transport behavior explicitly under ruling 7; it is not a loader matrix pass.

#### Object roots and legacy inputs

Each format has separate columns for each relevant load-bearing container or
legacy field. **M** = mapping/object accepted, with leaves validated separately;
**L** = recognized legacy value, translated with one migration warning.

| YAML shape | style | style.markdown |
| --- | --- | --- |
| Control | M containing markdown | M with the ten control leaves |
| Absent | F | F |
| Explicit null | N | N |
| Wrong whole type | D for cleanup rules | D |
| One wrong element | D: array | D: array |
| Every element wrong | D: array | D: array |
| Empty | `{}`: M; `[]`/`""`: D | `{}`: M; `[]`/`""`: D |
| Duplicate key | I | I |
| Invalid/trailing YAML | I | I |

| TOML shape | formatting | formatting.markdown | formatting.cleanup | formatting.fixed_width |
| --- | --- | --- | --- | --- |
| Control | M | M with control leaves | compact: L to none | 80: L to width 80 |
| Absent | F | F | F | F |
| Explicit null | X | X | X | X |
| Wrong whole type | W | W | W | W |
| One wrong element | W: array | W: array | W: array | W: array |
| Every element wrong | W: array | W: array | W: array | W: array |
| Empty | `{}`: M; other empty types W | `{}`: M; other empty types W | W | W |
| Duplicate key | W | W | W | W |
| Invalid/trailing TOML | W | W | W | W |

| JSON shape | formatting | formatting.markdown | formatting.cleanup | formatting.fixed_width |
| --- | --- | --- | --- | --- |
| Control | M | M with control leaves | compact: L to none | 80: L to width 80 |
| Absent | F | F | F | F |
| Explicit null | N | N | N plus migration warning | N plus migration warning |
| Wrong whole type | W | W | W | W |
| One wrong element | W: array | W: array | W: array | W: array |
| Every element wrong | W: array | W: array | W: array | W: array |
| Empty | `{}`: M; `[]`/`""`: W | `{}`: M; `[]`/`""`: W | W | W |
| Duplicate key | X: Value | X: Value | X: Value | X: Value |
| Invalid/trailing JSON | reject message | reject message | reject message | reject message |

Add semantic-invalid mutations for unknown names/enums, quoted indentation,
fractional/out-of-range width, unsafe language strings, pending executable YAML
values, and mixed old/new settings. A null leaf is intentionally fallthrough,
never confused with explicit `false`, `preserve`, or `""`. Prove null editor
overrides release the leaf to lower-precedence workspace/default values rather
than erasing an unrelated valid setting. Whole editor `settings: null` retains
its existing clear-overlay contract; the optional `dmls` wrapper remains the
existing input envelope.

Review new readers for `#[serde(default)]`, absence-erasing `Option<T>`,
`filter_map(..as_str())`, `unwrap_or_default()`, and `.ok()` on load-bearing
parses. Any matrix defect triggers review of the full affected row and column.

## Phase 2: Build Shared Rules and Repair Surfaces

Depends on Phase 1's contracts. Tasks in Wave 2 are parallelizable with separate
module owners; one integrator owns exports and shared module declarations.

### Wave 2 — Parallel foundation modules

- [x] **Implement rule model**
  - Add typed optional rule inputs, resolved defaults, source provenance, leaf
    validation, and precedence resolution. Normalize width implication only
    after precedence; false width does not cancel an independent true unwrap.
  - Register markdown in `lib/src/style/schema/`, `parse.rs`, and
    `descriptor.rs`, plus the embedded `docs/schemas/darkmatter.yaml` baseline.
    Preserve existing rendering style behavior; valid cleanup settings neither
    run cleanup nor produce unknown/inactive claims under strict rendering.
  - Add YAML matrix and rule-resolution tests, including baseline disabled/
    replaced, document-schema restrictions, all-default and all-off profiles.

- [x] **Extract repair pipeline**
  - Move raw frontmatter orchestration, authored coordinate projection, and
    clean result/diagnostic data from the CLI into library-owned modules.
    Keep terminal reporting and JSON source/output adaptation in the CLI.
  - Accept existing document context/schema controls; retain baseline → triggers
    → document schema and CLI overrides. No file-trigger discovery for stdin
    or untitled buffers; missing/unreadable referenced schemas use typed errors.
  - Preserve existing deterministic repair proofs and diagnostic spellings.
    Add direct raw-source repair tests and disabled-repair preservation cases.

- [x] **Migrate workspace settings**
  - Implement `formatting.markdown` in `dmls/src/config/mod.rs` using shared
    rule validation, source-preserving normalization, mixed-shape rejection,
    and migration warnings for compact/loose/default and legacy width.
  - Apply TOML/JSON matrices through config loading and effective formatting
    behavior. Test file/editor merging, malformed overlay fallback, warning
    visibility/count, unknown keys, and null/empty distinctions.
  - This task depends on the frozen Phase 1 rule contract, not on another
    agent's unfinished implementation; integrate with the completed shared
    rule module at Wave 3.

### Wave 3 — Sequential foundation integration

- [x] **Verify foundation parity**
  - Integrate exports and shared types; test shipped-schema corpus validation,
    runtime descriptors, strict-style acceptance, and DMLS completion/validation
    agreement. Defaults must match without schema coercion of quoted numerals.
  - Checkpoint: rule precedence and malformed-config behavior are observable,
    raw repairs work before `Markdown` construction, and no new environment
    capture or execution sites exist. Run targeted package-area L1 recipes.

## Phase 3: Implement Protected Transforms and Fidelity Comparison

Depends on Phase 2. Wave 4 separates protection from semantic comparison;
Wave 5 integrates them before one owner changes the interacting body passes.

### Wave 4 — Parallel protection and comparison

- [x] **Protect authored regions**
  - Implement the spike's source representation using existing scanners and
    shared input events/ranges. Preserve directive lines/terminators, quoted
    shell regions, opaque bodies, expression spellings, neighboring blank
    status, order, and container identity. Refuse malformed/unpaired regions.
  - Retain code payload spans and used/unused reference definitions with labels,
    destinations, titles, and source coordinates. Preserve BOM/newline metadata.
  - For every rule, add directive-family coverage after blockquote paragraphs,
    ordered/unordered list paragraphs nested two levels, and tables. Assert
    exact directive bytes and no added neighboring blank line; ensure
    directive-like code stays unprotected code.

- [x] **Compare semantic meaning**
  - Build a normalized event comparison using identical cleanup parser options
    without smart punctuation/definition lists. Compare containers, nesting,
    list start and tightness, headings, tables, footnotes, extension blocks,
    inline structure/values, HTML bytes, task state, and exact decoded text.
  - Coalesce text events; allow ordinary prose soft-break changes only under
    resolved unwrap/reflow. Compare code source bytes, info strings, references,
    protected spans, and preserved markers separately. Allow only authorized
    unlabeled-fence additions and affected-list spacing exceptions.
  - Expose first-input-divergence diagnostics and a test-only broken-pass seam
    accessible to consumer tests without enabling mutation in production.

### Wave 5 — Sequential transform integration

- [x] **Repair body transforms**
  - Integrate the source model and comparator. Repair prefixes for every child
    block in list items, marker-width minimum continuation indentation, and
    blockquote/fence/nested-list placement; eliminate introduced whitespace.
  - Implement all ten rules according to their ownership boundaries: external
    list padding, sibling padding, interior padding, indentation, ordered marker
    policy, table layout, safe language addition, unwrap, and width; frontmatter
    is supplied by Phase 2. Respect directive adjacency and required separators.
  - Remove ambient `PREFER_ITALICS` restyling and unconditional boundary/final
    newline changes; retain only proven artifact restoration. Disabled regions
    must not undergo incidental serializer normalization.
  - Route reflow through the same protections, count prefixes in display width,
    and preserve hard breaks, expressions, code, and HTML. Propagate serialization
    failures as declined results rather than silent string fallbacks.

- [x] **Verify rule fidelity**
  - Add E1–F1 exact output assertions: defaults are byte-identical except D1's
    `text` info addition at column 8; all-off is exact. Exercise E1/C1/D1 under
    indentation 2/8/preserve, unwrap, width 80, and every padding value, including
    multi-digit markers and code children requiring a safer indent.
  - Test every rule's default and every non-default value, all list-spacing
    examples exactly, required paragraph separators under both `none` rules,
    width true→120, width implication, empty language, and ordered sequences
    `1,1,1`, `3,3,3`, `3,4,5`, `1,3,8`, and `2,2,5`.
  - Checkpoint: valid fixtures are accepted, every criterion 1–4 fixture is
    idempotent, and injected semantic/protected mutations are rejected.

## Phase 4: Integrate the Final Transaction and Consumer Routes

Depends on Phase 3. Consumer adapters may run in parallel only after the final
library result contract and transaction behavior pass Wave 6.

### Wave 6 — Sequential transaction assembly

- [x] **Finalize cleanup transaction**
  - Assemble raw-source repair preparation, one-time rule resolution, protected
    body transforms/reflow, original-byte frontmatter reconstruction, and one
    final comparison. Body comparison is independent of YAML repair proof.
  - On refusal discard every candidate edit and applied repair. On invalid YAML
    return invalid without emitting repaired/partially cleaned text. Reports
    describe only emitted repairs and original authored coordinates.
  - Update all `Markdown::cleanup*` and body wrappers to shared defaults and
    guarding; compact/loose are explicit sibling-rule overrides. String-returning
    wrappers return original text on refusal; diagnostic consumers use results.
  - Test decline after a candidate YAML repair and corruption introduced by
    reflow. Enforce parse ceilings and separately report scanner counts, with
    short-circuit cases and standalone helper policy from Phase 1.

### Wave 7 — Parallel consumer adapters

- [x] **Wire CLI routes**
  - Replace CLI repair/body orchestration with the raw library transaction.
    Retain `open_argument` and document-context derivation. Convert only explicit
    flags into overrides; compact/loose map to none/one, and ignore-incidental-
    newlines overrides both unwrap and width with existing mutual exclusion.
  - Wire both `md clean` and top-level `md FILE --save`. Preserve JSON v1 and
    empty JSON stderr; render human diagnostics to stderr with terminal
    components. Declines emit original in nonsave mode, perform no save, and
    exit 0; invalid YAML exits 1 with no save.
  - Use `CliProcessFixture` to prove stdin/JSON/save behavior, schema overrides,
    ordinary changes, and injected fallback including empty `applied`.

- [x] **Wire DMLS formatting**
  - Use raw current unsaved buffer and existing document/schema context in the
    provider. Return edits only for changed outcomes, no edits for unchanged,
    declined, or invalid. Keep library code free of protocol side effects.
  - Route warnings through `router.rs`/client notifications, converting authored
    byte spans with negotiated encoding. Test UTF-8/UTF-16 positions, Unicode
    before the affected block, and one Warning `window/showMessage` per refusal.
  - Prove formatting request behavior without range/on-type/save capabilities,
    default formatter changes, disk rereads, content execution, or host focus.
    Correct provider module comments to the actual transaction guarantees.

### Wave 8 — Sequential consumer parity

- [x] **Verify invocation parity**
  - Run normal CLI and DMLS protocol fixtures on identical raw input/options:
    repairable malformed frontmatter, repair disabled, empty frontmatter, BOM,
    CRLF/mixed endings, missing final newline, references, and unpaired directives.
  - Prove CLI > document > workspace > runtime precedence and whole-object
    invalid-rule fallback. Prove A1/B1 compose equivalence with `when` true/false
    using fixture-owned data and no shell/network effects.
  - Checkpoint: criteria 5–9, 12, and 13 pass across public invocation paths;
    injected mutations cover links/titles, emphasis, hard breaks, HTML, task
    state, table alignment, references, expressions, code/info, and list tightness.

## Phase 5: Validate Real Documents and Publish Current Behavior

Depends on Phase 4. Corpus triage owns code fixes; documentation owns separate
docs/skill files. Coordinate any discovered behavior departure before docs close.

### Wave 9 — Parallel corpus and documentation work

- [x] **Validate repository corpus**
  - Retrieve `prompts/commit.md` from blob `464e2d90e` with a non-interactive
    read-only Git command and check its exact bytes into a cleanup fixture.
    Verify default acceptance, preserved wraps, and successful fixture-isolated
    composition. Record the original blob provenance in the fixture metadata.
  - Scan every tracked `*.md` without saving, composing, or executing content.
    Report processed/changed/declined/invalid counts, categorized refusal reasons,
    and per-file findings in `implementation-log.md`. Account for unreadable
    inputs explicitly rather than silently omitting them.
  - Separate intentionally malformed examples from valid supported syntax.
    Fix supported fallback defects or create an `_unscheduled` fix with a minimal
    reproduction as the specification permits. Record each remaining valid
    refusal as a defect; do not count fail-closed output as successful formatting.
    This is local acceptance evidence, not a new CI gate.

- [x] **Document formatting rules**
  - Add a `docs/` topic for every `style.markdown` key with compact examples,
    invariants, defaults, and a Mermaid precedence diagram. Explain tight/loose
    rendering for newcomers; include the boundary diagram and same-input output
    for every value of both padding rules.
  - Update `docs/cli/clean.md`, `docs/inline/cleaning.md`,
    `docs/lsp/features.md`, and the DMLS configuration documentation to describe
    transaction behavior, repair parity, migration, precedence, and warnings.
    Update affected READMEs and `.claude/skills/darkmatter/` cleanup guidance.
  - Review behavior-changing symbol comments alongside code and remove stale
    guarantees. Docs describe current behavior and never link to/name the fix.
    Update dependency docs only if a crate is actually added or removed.

### Wave 10 — Sequential corpus closure

- [x] **Close corpus findings**
  - After fixes, rerun affected fixtures and repeat the passive scan only when
    changed behavior makes the prior results stale. Finish counts and refusal
    dispositions; reconcile docs with accepted behavior and log spec departures.
  - Checkpoint: criteria 10, 11, and 14 have artifacts, every corpus refusal has
    a classification/disposition, and no repository Markdown was rewritten.

## Phase 6: Complete Validation and Review Handoff

Depends on all previous checkpoints. No new performance study or corpus CI cell.

### Wave 11 — Validation with independent environment work

- [ ] **Run scoped checks**
  - Run `just test` and `just lint` in `darkmatter/` through the area's nextest
    recipes. Include actual direct consumers affected by public API changes,
    identified with Sniff/call-site searches; avoid workspace-wide gates.
  - Confirm tests are registered in consolidated binaries and repository fixture/
    schema reads use CI-indexable paths. Review complete robustness rows/columns,
    shipped-schema validation, JSON compatibility, idempotence, and parse counts.
  - Reuse qualifying environment evidence; load the `os` skill and run required
    missing coverage through supported non-interactive host workflows for macOS,
    Linux, native Windows, and WSL2. Record evidence or an explicit blocker for
    each. Independent host validations can run concurrently after code is stable.
    Do not mistake cross-compilation for execution evidence or broaden CI scope.
  - L2/browser checks are needed only if implementation changes a real terminal
    or browser behavior; use canonical recipes and no-focus harnesses if so.
    Never run `cargo fmt` or substitute `cargo test` for nextest suites.

### Wave 12 — Sequential handoff

- [ ] **Prepare review handoff**
  - Complete the log with criterion-to-test evidence, changed surfaces, pass
    dispositions, scanner/cleanup parse accounting, corpus counts, deferred
    reproductions, and any implementation rulings or departures.
  - Verify the plan's six phases and all task outcomes against the specification;
    leave incomplete checks unchecked and report blockers precisely. Review the
    final diff for unintended schema, formatting, or fixture changes.
  - Terminal state is **implementation complete, ready for review** when required
    evidence is present. Leave the fix in its active directory; the author owns
    lifecycle closure. Commit/push/signing are outside this plan's authorization.
