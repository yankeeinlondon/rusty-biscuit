---
kind: plan
total_phases: 4
created: 2026-10-20
phase: 1
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - biscuit-file
  - darkmatter
  - claudine
source_files_during_phase_2:
  - biscuit-file/lib/src/yaml/analyze/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/engine.rs
  - biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/tests/mod.rs
  - biscuit-file/lib/src/yaml/analyze/tests/root_indentation.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/tests/l1/inline_document_text.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
  - darkmatter/cli/tests/l1/clean_frontmatter.rs
  - darkmatter/dmls/tests/l1/formatting_transaction.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/timeline.md
---

# Indented Frontmatter Implementation Plan

## Phase 1 — Establish Contracts and Coverage

### Necessary Rules

Implement `2026-10-20-handle-indented-frontmatter` across biscuit-file, Darkmatter, DMLS, and Claudine. Add a proven root-indentation repair to the existing YAML analyzer, reject indented roots in text-preserving frontmatter editors, and rehearse inline closure operations before prompt composition can execute commands or create guardrails.

Completion means `md clean --save` preserves the parsed frontmatter value while moving supported root keys to column 0; `md hash --save` refuses an uncleaned document clearly without changing its bytes; inline preparation refuses the same baseline before prompt commands or providers run; and the cleaned document completes inline composition with a valid column-0 hash. An agent-introduced indented candidate must still trigger closure refusal and exact baseline restoration.

- The specification resolves the design; no author ruling blocks implementation. Indented mappings remain valid YAML for readers and read-only hash computation. The new rejection is an editor limitation.
- Repair only nonempty block root mappings whose first content line identifies an ordinary mapping key. Exclude flow roots, sequences, scalars, empty/comment-only sources, directives, explicit document markers, complex keys, and tagged-root syntax from the new repair. Do not extend support for unsupported editor shapes.
- Remove the minimum leading ASCII-space count across nonblank, non-comment lines using prefix byte edits. Dedent qualifying comments, retain less-indented comments, and handle whitespace-only `BlockContent` lines as the spec requires. Existing trailing-whitespace repairs own blank lines outside block scalars.
- Preserve the existing combined-edit parse-equivalence proof. If safety cannot be proven, suppress the repair. Do not serialize YAML to normalize it or weaken overlap/equality checks.
- Baseline preflight uses captured `source.original_text`, the existing managed-property list and inline hash options, and a supplied timestamp. It discards results and performs no writes or command execution. It precedes side-effect-capable preparation on ordinary runs and previews, including re-entry through the shared helper.
- Preflight cannot guarantee the safety of later agent edits or external file changes. Retain closure validation and rollback. Ordinary non-inline compose/sequence behavior stays unchanged.
- No spike is needed. The spec already demonstrates the scalar-blank-line hazard, identifies the implementation seams, and explicitly excludes performance spikes and benchmarks. Do not repeat these investigations as later gates.
- Tests are hermetic L1 checks, portable across macOS, Linux, native Windows, and WSL2. Use existing fixture builders and cross-platform provider stubs, private homes/directories, and silent audio. No live agents, terminal/browser focus, L2/L3 runs, or new CI cells are required.

### Wave 1 — Contract Inventory

- [x] **Map Test Boundaries**
  - Inspect biscuit-file's `yaml/analyze/{engine,scan,diagnostic}.rs`, `tests/tab_indentation.rs`, diagnostic inventory tests, and edit application API. Record how prefix edits interact with trailing-whitespace, line-ending, and final-newline edits.
  - Audit Darkmatter's `validate_block_mapping` callers in `markdown/hash/write.rs`, including hash save, both restore inputs, and leaf location. Preserve existing invalid-YAML/root-form errors and no-op validation behavior.
  - Trace Claudine's `prepare_inline_on`, its preview/re-entry callers, closure stamping/restoration, and `composition/error/{mod,render/mod,tests}.rs`. Identify every exhaustive error match and the existing semantic diagnostic conventions.
  - Identify fixture locations and ownership for subsequent waves: analyzer tests; `clean_frontmatter_repair.rs`; CLI `clean_frontmatter.rs`; DMLS `formatting_transaction.rs`; Claudine closure/preparation tests and `inline_compose_hash.rs`/`inline_compose_cli.rs`.
  - Confirm the portable stub pattern before adding platform-specific fixture code; load the `os` skill before changing Windows-specific or path-comparison code. Use the area's skills for implementation work and the shared Test Toolkit for tests.
  - Deliver a requirement-to-test checklist using the robustness scope below. This is a bounded code audit, not a spike or a new abstraction.

### Input Robustness Scope

The changed YAML analyzer and editor shape validator warrant document-shape coverage. Markdown composition itself is a text grammar and does not require a configuration-field matrix. No configuration fields or field validators are added or changed; do not multiply field-type tests across `prompt`, `hash`, and `last_updated` merely because they travel through this path.

Default for an unusable inline baseline: return a source-path-bearing preparation error with a typed Darkmatter cause, execute no prompt command or provider, publish no result, create no guardrail, and leave source bytes unchanged. Editor failures return `FrontmatterTextEdit` before an edit is produced. Exceptions and loader-specific outcomes:

| Boundary | Inputs and required outcome |
| --- | --- |
| YAML analyzer, one existing `serde_yaml_ng` loader | A valid indented ordinary mapping is the positive control: applying the public analysis result preserves its value. Valid column-0 mappings and excluded valid root forms receive no new root-indentation repair. Existing diagnostics may still apply. |
| YAML analyzer invalid documents | Duplicate keys, valid YAML plus invalid trailing content, and a second YAML document retain existing parse/report outcomes; no root-indentation repair is offered. Retain existing recovery semantics for tab indentation and verify cleanup's subsequent rescan enables the new repair. |
| YAML syntax and scalar safety | Literal/folded blocks, explicit indentation indicators, multiline quoted strings, scalar lines beginning with `#`, and scalar blank lines either retain exactly the parsed value after repair or receive no unsafe repair. Explicit YAML document markers/directives remain outside this repair. |
| Editor shared validator | A supported column-0 mapping is the control. Indented mappings fail with the indentation reason; empty/comment-only frontmatter stays accepted. Existing invalid-YAML, scalar/sequence, flow-root, and other unsupported-form behavior is retained. |

Build cases by changing one feature of a representative reproduction/control fixture. Test the analyzer through `analyze_yaml` and its public application result; test editor decisions through exported hash-save, restore, and leaf-location results. Exercise each shared decision once thoroughly, with a control and an indented-input wiring case for each additional public consumer. Avoid a Cartesian product of callers and YAML syntax.

At review, inspect changed parsing paths for load-bearing `#[serde(default)]`, absent/null conflation through `Option<T>`, `filter_map(...as_str())`, `unwrap_or_default()`, and `.ok()` that hide parse failures. Existing intentional uses outside the change do not justify a broad cleanup. A shape-related finding must account for the other cases sharing that validator and document-shape category.

**Checkpoint:** Every success criterion has an assigned public observation; operation ordering and fixture portability are understood. Proceed without a separate approval gate.

## Phase 2 — Implement Shared YAML Safety

### Wave 2 — Parallel Library Changes

Both tasks depend on Wave 1 and may run concurrently in separate file sets. Each task includes its own focused tests and relevant API-comment updates.

- [x] **Repair Root Indentation**
  - Own biscuit-file's YAML analyzer implementation and tests. Add `YamlDiagnosticCode::RootIndentation` with `yaml.root-indentation` serialization, string conversion, `ALL` inventory entry/count, and round-trip coverage.
  - Generate S1 candidates through `s1_candidates`, using the parsed root shape and lexical context to enforce the eligibility rules. Keep the implementation within the existing analyzer rather than creating a second cleanup dispatcher.
  - Emit nonoverlapping leading-prefix removals with correct byte spans. Preserve terminators in these edits; account separately for comments, blank lines outside scalars, and whitespace-only scalar content at/below/above the scalar content indent.
  - Extend/register `tests/root_indentation.rs` beside `tab_indentation.rs`. Cover common indentation, column-0 controls, exclusions, LF/CRLF, less-indented comments, key/order/comment fidelity, all scalar-safety cases, and combined whitespace edits. Assert public applied text, audit acceptance, and parsed-value equality; unsafe candidates must be absent.
  - Preserve the existing proof/fallback behavior even if individually applied prefix edits are insufficient to dedent a multi-line scalar safely.

- [x] **Guard Text Editors**
  - Own Darkmatter's `markdown/hash/write.rs` gate and its editor tests. Preserve the original leading spaces while skipping blank/comment lines to identify the first root key; do not classify nested content as a root key.
  - After respecting existing YAML/root-shape errors, reject indented block mappings with `MarkdownError::FrontmatterTextEdit` and the spec's exact reason, including the measured space count and `md clean --save <file>` guidance.
  - Validate before key lookup/insertion, including otherwise-no-op requests. Test `apply_hash_save_text`, `restore_properties_text` with indentation in either current or snapshot input, and `locate_frontmatter_leaves`; verify unchanged inputs and typed causes.
  - Retain column-0 behavior, empty/comment-only acceptance, read-only parsing, and hash computation. Update editor API documentation to state the column-0 constraint.

**Checkpoint:** Focused analyzer and editor tests pass independently. Repair is parse-equivalent, and direct editor calls reject unsupported indentation before returning changed text. Both contracts are available to Wave 3.

## Phase 3 — Integrate Cleanup and Inline Preparation

### Wave 3 — Parallel Consumer Changes

Both tasks depend on Wave 2. Keep ownership separate: cleanup/DMLS files belong to the first task; Claudine preparation/closure/error files belong to the second.

- [x] **Verify Cleanup Consumers**
  - Extend Darkmatter's `lib/tests/l1/clean_frontmatter_repair.rs` and CLI `tests/l1/clean_frontmatter.rs`. Prove `repair_frontmatter`/`clean_document` consumes the analyzer repair through its existing path, without a new dispatch branch.
  - Verify common indentation combined with whitespace repairs, tab recovery followed by the existing rescan, parsed-value preservation, and stability on a second cleanup. Distinguish the new prefix edits' terminator fidelity from other existing line-ending/body cleanup rules.
  - Through `CliProcessFixture`, first assert `md hash --save` refuses the indented reproduction with the indentation reason and unchanged file bytes. Then persist `md clean --save`, reread the file, run `md hash --save`, and reread/assert the valid column-0 stamp and preserved data.
  - Extend `dmls/tests/l1/formatting_transaction.rs` using the current buffer and provider formatting API: apply the offered edits, verify equivalent frontmatter and column-0 keys, and verify the second formatting request offers no further root repair. Include tab-recovery interaction through the shared transaction.
  - If integration exposes an analyzer defect, hand it back to its owner; do not bypass the proof in a consumer.

- [x] **Preflight Inline Baselines**
  - Add a small closure helper that calls `restore_properties_text(original, original, CLOSURE_OWNED_PROPERTIES)`, constructs Markdown from the returned text, and calls `stamp_baseline` with `inline_hash_options()`, a caller-supplied timestamp, and `Change::Known`. Discard the result; do not call reconciliation or persist anything.
  - Introduce `CompositionError::InlineBaselineUnstampable { path, source }` with a typed `MarkdownError` cause. Integrate the source chain, semantic diagnostic identity/detail, terminal block rendering, and all exhaustive matches. The rendered error must explain preparation refusal, name the source path, and retain the editor's actionable reason.
  - Call the helper on `source.original_text` near entry to `prepare_inline_on`, after side-effect-free early grammar checks and before prompt commands, guardrail creation, and other side-effect-capable preparation. Exercise ordinary preparation and preview modes; confirm retry/proxy preparations using this helper cannot skip the check.
  - Add deterministic helper/preparation tests with fixed timestamps. Assert the dedicated variant, exact path, downcast/matched Darkmatter cause, input immutability, and successful column-0 baseline rehearsal. Cover managed properties present and absent using existing inline closure semantics.
  - Retain candidate validation and rollback; add closure coverage for a valid baseline followed by an indented candidate. Review associated comments so preflight is never described as guaranteeing later editability.

**Checkpoint:** Cleanup and DMLS normalize through the existing shared transaction. Inline preparation returns the new typed error before composition effects, while existing valid inline closure tests pass.

## Phase 4 — Prove Acceptance and Document Behavior

### Wave 4 — Parallel Acceptance and Documentation

Both tasks depend on Wave 3. The documentation task may run while the acceptance owner runs focused tests; avoid concurrent Cargo commands that can replace Claudine's fixture-enabled binary.

- [ ] **Prove Inline Acceptance**
  - Extend the hermetic Claudine CLI tests using `CliProcessFixture` and the existing portable provider-stub pattern. An indented baseline must exit nonzero with its source path and indentation cause, record zero provider launches, remain byte-identical, and leave the expected guardrail path absent.
  - Include a recording prompt-command stub in that document and assert its marker is absent too. Ensure the valid control executes the command/recording seam so absence on refusal is meaningful. Cover the preview path in-process or with one additional wiring check, without duplicating the YAML matrix through CLI spawns.
  - On the same representative document after `md clean --save`, prove inline-compose launches the stub, accepts an actual body edit, and persists a valid column-0 hash. Reuse the established `md` binary fixture/dependency route rather than a host-installed executable.
  - With an initially valid baseline, have a stub produce an indented candidate with a changed body; assert closure failure and exact original-byte restoration. Keep the existing successful column-0 closure checks.
  - Keep fixtures cross-platform, audio silent, and processes bounded. Reuse existing stub infrastructure rather than compiling a new helper per matrix cell.

- [ ] **Document Repair Workflow**
  - Update the current Darkmatter cleanup reference (`darkmatter/docs/cli/clean.md` and related formatting topic only as needed) and `.claude/skills/darkmatter/cleanup.md` with `yaml.root-indentation`, eligibility, scalar-proof limitations, and a compact before/after example.
  - Update biscuit-file's current YAML analyzer documentation alongside the tab-indentation diagnostic. Explain parse-equivalent formatting normalization and the cases where no repair is offered; add an appropriate topic page if no analyzer reference exists.
  - Update `claudine/docs/topics/composition.md` with the launch-time error, `md clean --save` recovery, preview behavior, and the continued need for candidate validation/rollback. Include a small Mermaid flow for preparation → refusal/launch → closure where it clarifies ordering.
  - Add a brief README overview note/link where the public behavior change belongs. Keep detailed command and diagnostic rules in `docs/`; current docs must not refer to the fix directory or spec snapshot.
  - Update the relevant skill guidance for the new preparation invariant. Review touched symbol docs and comments for drift; no dependencies or CI policy changes are expected.

### Wave 5 — Sequential Final Validation

- [ ] **Run Scoped Gates**
  - After all edits merge, run each affected area's L1 suite once using `just test` from `biscuit-file/`, `darkmatter/`, and `claudine/`. Darkmatter's area recipe includes DMLS. Use nextest-backed recipes; do not run workspace-wide suites, live-provider tests, or L2/L3 tiers.
  - Run `just lint` in each affected area after tests, sequentially with Claudine tests and tier listing. Do not run `cargo fmt` or automatic formatting; use lint/check feedback for targeted manual corrections.
  - Run `just check-tier-coverage claudine` after Claudine testing/linting, and confirm any newly added test module is registered in its existing suite. Do not run these commands concurrently with fixture-enabled CLI tests.
  - Reconcile failures against the owning task and rerun only the affected checks after a change. Review the final diff for proof weakening, side effects before refusal, diagnostic cause flattening, unsupported YAML scope expansion, and missing robustness cases.
  - Record commands/results and material limitations in an implementation log alongside this plan. Record any departure from the spec there and correct current docs; leave the spec snapshot intact.
  - Mark completed plan tasks and report implementation complete, ready for review. Do not commit, move the fix to `_completed`, or run `just complete`.

**Final checkpoint:** All four success criteria and the agent-candidate rollback case are proven, scoped L1/lint gates pass, and documentation matches the shipped behavior. Cross-platform compatibility is required; report only environments actually exercised, without adding a new CI matrix or claiming unrun OS evidence.
