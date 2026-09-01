---
title: Preserve inline-compose frontmatter and authorize generated properties
status: ready
created: 2026-09-01
phase: 1
total_phases: 6
agent: codex/default
yolo: true
spec: claudine/fixes/2026-09-01-inline-compose-frontmatter/spec.md
packages:
  - darkmatter
  - darkmatter-cli
  - claudine
  - claudine-cli
---

# Inline-compose frontmatter execution plan

## Objective

Make `claudine inline-compose` preserve authored frontmatter text through the
final hash-stamped write, accept generated frontmatter only through an authored
`response_frontmatter` allowlist, migrate shipped guardrails without replacing
customizations, and treat mid-run source drift as detection input only: the
run always completes, the closure's snapshot rebuild restores the authored
bytes, and drift is reported without attributing a writer. Adopt the same
text-preserving hash-save path in `md hash --save`.

## Execution constraints and dependency map

- Preserve unrelated worktree changes, including the existing
  `.claude/memory/commits.md` modification and the specification directory.
- Before editing each production symbol, rerun GitNexus upstream impact for
  that exact symbol. The planning-time index rated `prepare_inline` and
  `load_or_create_guardrails` CRITICAL, and `apply_inline_closure` and
  `try_inline_closure` HIGH; review direct callers before changing signatures.
- Treat raw document text as the byte authority. Parsed YAML maps are for
  validation, hash planning, and values, never for re-emitting untouched
  authored frontmatter.
- Keep the existing map-based `Markdown::apply_hash_save` for map-owning
  callers. Document its reserialization behavior and route only text-fidelity
  callers through the new fallible textual API.
- Preserve `FileReference` resolution in `md hash --save`, reject stdin for
  save mode, retain LF/CRLF, and use atomic writes where the owning package
  already requires them.
- Keep terminal status output on existing `TerminalRenderable` components;
  do not introduce direct styled `println!`/`eprintln!` output.
- Use nextest-backed `just` recipes. Do not run `cargo test` or `cargo fmt`.
- Phase 3 depends only on Phase 1 and is parallelizable with Phase 2. Phase 4
  depends on Phases 2 and 3. Phase 5 depends on Phase 4; its documentation work
  may run in parallel with its end-to-end test work after public behavior and
  symbol names settle.

## Acceptance coverage map

| Acceptance criteria | Primary coverage |
|---|---|
| AC1–AC4 | Claudine closure L1 tests in `composition/closure/tests.rs` |
| AC5 | Darkmatter hash-writer unit tests and `darkmatter-cli` L1 CLI tests |
| AC6–AC8, AC12 | Claudine prepare/extraction/closure L1 tests |
| AC9 | Claudine guardrail loader L1 tests with an injected failing writer |
| AC10 | Existing `wrap_inline_compose.rs` provider-stub integration tier |
| AC11 | Claudine closure L1 mid-run-drift restore tests plus CLI status assertions |

## Phase 1 — Lock the contracts with regression tests

- [ ] **Task 1.1 — Reconfirm blast radius and baseline behavior.** Run
  GitNexus `impact(..., direction: "upstream")` for `apply_hash_save`,
  `prepare_inline`, `load_or_create_guardrails`, `extract_replacement_body`,
  `apply_inline_closure`, and `try_inline_closure`; record direct callers and
  any HIGH/CRITICAL warnings in the implementation handoff. Run the existing
  focused hash, closure, prepare, guardrail, and wrapper-inline tests to prove
  the baseline is green before introducing red regressions.

- [ ] **Task 1.2 — Add Darkmatter textual-save contract tests.** In
  `darkmatter/lib/src/markdown/hash/write.rs` tests, add table-driven LF and
  CRLF cases covering a trailing-space multiline property, default and custom
  managed property names, plain and quoted managed keys, absent frontmatter,
  existing scalar and block-mapped values, every `StoredHash` representation,
  structured-to-Simple replacement, comments at node boundaries, preserved
  `last_updated` quote style, duplicate semantic keys, and unsupported
  flow-style roots. Assert no bytes outside the managed hash and
  `last_updated` nodes change. Covers AC3–AC5.
  **Parallelizable with Tasks 1.3–1.5.**

- [ ] **Task 1.3 — Add authored-allowlist preparation tests.** In
  `composition/prepare/tests.rs`, add cases proving `response_frontmatter` is
  snapshotted from the authored source in declaration order; setters,
  interpolation, schema defaults, and effective-frontmatter changes cannot
  create or expand it; non-list, empty, non-string, duplicate, and reserved
  names fail before provider launch; and Claudine-interpreted allowed names
  produce a prepare-time warning. Assert the generated protocol clause lists
  the exact authorized keys. Covers the preparation half of AC6–AC7.
  **Parallelizable with Tasks 1.2, 1.4, and 1.5.**

- [ ] **Task 1.4 — Add response parsing and closure regression tests.** Replace
  the body-only assumptions in `composition/closure/tests.rs` with red tests
  for parsed response parts, strict YAML mapping validation, duplicate keys,
  non-map roots, well-delimited blocks without bodies, conservative unclosed
  delimiters, undeclared-key warnings, silent `hash`/`last_updated` rejection,
  missing allowed keys, declaration-order insertion, in-place refresh of
  scalar/sequence/mapping/multiline nodes, YAML-significant key names, and the
  unchanged-body rule. Covers AC6–AC8 and AC12.
  **Parallelizable with Tasks 1.2, 1.3, and 1.5.**

- [ ] **Task 1.5 — Add byte-fidelity, guardrail, and mid-run-drift regressions.**
  Add the exact AC1 prompt fixture (four-space `|-`, trailing whitespace, blank
  line, and literal `\"`), structured-hash downgrade, hash comparison and
  repeated-run idempotence assertions, both historical shipped guardrail
  migration cases, custom guardrail preservation, simulated migration-write
  failure, and body/frontmatter mid-run-drift cases — the canonical one
  rewrites `prompt: |-` to an inline `\n`-escaped string — proving the run
  completes, the response is applied, the written frontmatter is restored
  byte-identical from the snapshot, value-drifted properties are reported
  without attribution, and value-preserving reformats restore silently.
  Covers AC1–AC4, AC9, and AC11.
  **Parallelizable with Tasks 1.2–1.4.**

- [ ] **Validation checkpoint 1.6 — Capture the red proof.** Run only the new
  Darkmatter and Claudine tests through each package area's `just test` filter
  support. Confirm existing tests remain green and new behavior tests fail for
  the expected reserialization, missing response channel, stale guardrail, or
  overwrite behavior—not because fixtures are malformed or platform-specific.

## Phase 2 — Build Darkmatter's text-preserving hash-save path

- [ ] **Task 2.1 — Introduce typed text-frontmatter editing.** Add the smallest
  source-preserving top-level YAML node locator/editor needed by hash save. It
  must recognize plain and quoted keys by parsed semantic value, return exact
  byte ranges for scalar and block-mapped nodes, retain zero-indent surrounding
  comments, treat indented comments as part of a replaced node, detect multiple
  semantic occurrences, detect newline style, and reject unsupported root
  layouts with a typed `MarkdownError`. Keep this a focused frontmatter-text
  facility rather than a general YAML formatter.

- [ ] **Task 2.2 — Implement `apply_hash_save_text`.** Add the fallible textual
  save API under Darkmatter's hash surface with the specification's signature
  or an equivalent owned-text form. Return `Ok(None)` immediately when
  `SaveDecision::new_stored` is absent; otherwise serialize the complete
  `StoredHash` value as one YAML node, replace or append
  `options.property`, and apply `bump_last_updated` while preserving existing
  quote style and document newline style. A document without frontmatter must
  gain a minimal valid block while retaining the body byte-for-byte.

- [ ] **Task 2.3 — Preserve the map-owned API contract.** Keep
  `Markdown::apply_hash_save` behavior intact, update its module/rustdoc to
  state that it reserializes frontmatter, and direct text-authoritative callers
  to the new API. Review all changed comments against the implementation and
  remove the now-incomplete body-only fidelity claim.

- [ ] **Task 2.4 — Adopt raw text in `md hash --save`.** Refactor
  `darkmatter/cli/src/io/mod.rs` and `commands/hash.rs` so save mode rejects
  stdin before loading, resolves the input once through `FileReference`, reads
  raw UTF-8 once, parses the `Markdown` from that same string, plans the save,
  and writes the textual result to the resolved path. Leave non-save callers
  on the existing `load_markdown` behavior and preserve error context.

- [ ] **Task 2.5 — Add CLI-level fidelity coverage.** Exercise real
  `md hash --save` invocations for LF/CRLF, multiline trailing whitespace,
  longhand structured/detailed values, `HASH_PROPERTY`, quoted keys, and an
  unsupported flow mapping. Assert successful saves are hash-clean under the
  equivalent of `md hash --diff`, repeated saves are byte-idempotent, and
  rejected inputs remain byte-identical. Completes AC5.

- [ ] **Validation checkpoint 2.6 — Verify Darkmatter independently.** From
  `darkmatter/`, run focused hash tests, then `just test` and `just lint`.
  Confirm no map-based caller changes behavior and all textual-save fixtures
  preserve body and non-managed frontmatter bytes.

## Phase 3 — Prepare the authored allowlist and migrate guardrails

- [ ] **Task 3.1 — Add the closure authorization snapshot.** Extend
  `InlineClosurePlan` with an ordered `response_frontmatter` collection and
  update every constructor/fixture. In `prepare_inline`, read the declaration
  from `source.markdown.frontmatter()` before composing or applying input
  layers, validate it as unique non-empty strings, and reject `prompt`,
  `response_frontmatter`, `hash`, and `last_updated` with typed composition
  errors rooted at the authored property.

- [ ] **Task 3.2 — Warn on future execution semantics.** Classify allowed keys
  that Claudine interprets (`agent`, `model`, `$schema`, lifecycle keys, and
  other existing control properties) and append a source-aware
  `ComposeWarning` during preparation. Keep these names authorized—the warning
  is advisory because the author explicitly opted in.

- [ ] **Task 3.3 — Teach the dynamic response protocol.** Replace
  `DEFAULT_GUARDRAILS` with the specified conditional-frontmatter language.
  After loading built-in or customized guardrails, append a Claudine-owned
  clause only when the allowlist is non-empty, listing the exact names in
  declaration order. Repository customization may tighten instructions but
  must never add authorization.

- [ ] **Task 3.4 — Migrate only known shipped defaults.** Teach
  `load_or_create_guardrails` to recognize both historical default byte
  sequences and atomically replace only those with the new default. Preserve
  any other bytes exactly. Introduce a narrow write seam so tests can prove a
  migration failure returns the new in-memory protocol, emits a warning, and
  leaves the old file untruncated for a later retry. Update this repository's
  materialized `.claudine/inline-compose.md` only because it byte-matches a
  known shipped default.

- [ ] **Validation checkpoint 3.5 — Verify preparation is fail-fast.** Run the
  focused prepare and guardrail tests, then the Claudine library L1 suite.
  Confirm invalid declarations never reach selection/provider launch, custom
  guardrails remain unchanged, and dry-run remains mutation-free.

## Phase 4 — Integrate response harvesting, textual closure, and drift restore

- [ ] **Task 4.1 — Parse replacement parts without silent loss.** Replace
  `extract_replacement_body` with `extract_replacement_parts` (or a compatible
  sibling) returning the trimmed body plus optional parsed leading
  frontmatter and source key locations. Treat an exact, closed leading `---`
  block as an explicit metadata attempt: malformed YAML, duplicates, non-map
  roots, or missing non-empty body return `InvalidInlineResponse`; an unclosed
  delimiter remains ordinary body text.

- [ ] **Task 4.2 — Harvest only authorized response keys.** Compare response
  keys with the snapshotted allowlist, ignore closure-owned `hash` and
  `last_updated` silently, produce source-accurate warnings for undeclared
  proposals, and report allowed-but-missing keys without failing an otherwise
  valid body. Serialize each harvested key/value as a one-entry YAML mapping so
  significant keys and scalar, multiline, sequence, and mapping values remain
  valid YAML.

- [ ] **Task 4.3 — Replace generated nodes textually.** Update
  `rewrite_inline_document`/closure helpers so missing generated keys are
  inserted in declaration order immediately before `last_updated`, while
  existing authorized keys are replaced as complete top-level nodes in their
  original positions. Do not change undeclared authored nodes or adjacent
  comments. Removing authorization must leave the existing property untouched.

- [ ] **Task 4.4 — Stamp the reconstructed text without reserialization.** In
  `apply_inline_closure`, delete the `with_frontmatter` reconstruction, parse
  `doc_string` only to read the stored hash and compute `SaveDecision`, and
  pass the unchanged `doc_string` to Darkmatter's textual save API. Preserve
  Simple-hash forcing, cleanup-before-hash, unchanged-body rejection,
  self-consistency, and the single final atomic write.

- [ ] **Task 4.5 — Add mid-run drift detection feeding the restore report.**
  Immediately before `atomic_write`, reread `target_path` and compare it with
  `InlineClosurePlan::original_document_text`. Drift is detection input only —
  never a merge source and never a reason to refuse: the snapshot rebuild
  proceeds to the single atomic write unconditionally. Report each
  value-drifted frontmatter property as "changed on disk during the run —
  restored the authored value", emit one non-attributing informational line
  for body drift, and restore value-preserving reformats silently. A failed
  reread degrades to writing without a drift report. No typed conflict error
  exists on this path.

- [ ] **Task 4.6 — Rewire wrapper status reporting.** In
  `try_inline_closure`, remove post-run on-disk frontmatter harvesting as a
  merge source (Task 4.5's pre-write reread is detection-only), pass parsed
  response parts into closure, and render statuses for inserted, refreshed,
  ignored, and missing response properties through existing
  `TerminalRenderable` status components. Delete the agent-attribution
  wording; render the D4 drift-restore reports (per-property frontmatter
  restores, the body-drift informational line) through the same components
  while retaining the truthful preservation and cleanup success lines.

- [ ] **Validation checkpoint 4.7 — Run the full closure matrix.** Run focused
  closure, error-rendering, wrapper-inline, preparation, and lifecycle-loop
  tests. Confirm AC1–AC4, AC6–AC8, AC11, and AC12 pass, including repeated
  refresh, byte-idempotence, source-accurate statuses, the mid-run-drift
  restore, and the non-mutating invalid-response path.

## Phase 5 — Prove the end-to-end protocol and align documentation

- [ ] **Task 5.1 — Add the obedient-provider end-to-end test.** Extend the
  existing `claudine/cli/tests/wrap_inline_compose.rs` provider-stub harness
  with an `ap.md`-shaped document declaring
  `response_frontmatter: [access_points, generated_by]`. Have the stub obey the
  dynamic guardrails by returning only those properties plus a replacement
  body. Assert both values are present, the authored multiline `prompt` bytes
  are unchanged, `hash` and `last_updated` are closure-managed, and a second
  invocation refreshes generated values. Keep the harness non-interactive and
  ensure no terminal or browser gains focus. Covers AC10.
  **Parallelizable with Tasks 5.2–5.4.**

- [ ] **Task 5.2 — Update authoritative Claudine behavior docs.** Revise the
  inline-compose sections of `claudine/docs/topics/composition.md` and any
  execution-flow page that still describes body-only responses, on-disk
  merge/revert behavior, or agent attribution. Document authored authorization,
  generated-key ownership/refresh, invalid response behavior, the mid-run
  drift restore contract (execution never stops; the authored `|-` rendition
  is authoritative and comes back byte-for-byte), and textual hash write-back.
  **Parallelizable with Tasks 5.1, 5.3, and 5.4.**

- [ ] **Task 5.3 — Update property and guardrail references.** Add
  `response_frontmatter` to
  `claudine/docs/topics/frontmatter-properties.md`, explain warnings for
  Claudine-interpreted names, and update default guardrail examples and the
  `hash`/`last_updated` fidelity claims. Use a metadata name such as
  `generated_by` in examples rather than overloading `agent`.
  **Parallelizable with Tasks 5.1, 5.2, and 5.4.**

- [ ] **Task 5.4 — Refresh the portable skill snapshot.** Mirror the finalized
  composition contract into `.claude/skills/claudine/composition.md` and check
  the main Claudine skill/timeline for statements that became false. Update
  only drifted material. For every modified Markdown document with a managed
  `hash:` frontmatter property, stamp it using Darkmatter's `md hash --save`
  path and verify `md hash --diff` is clean.
  **Parallelizable with Tasks 5.1–5.3.**

- [ ] **Validation checkpoint 5.5 — Verify public behavior and docs.** Run the
  new provider-stub test, existing inline-compose integration tests, doctests
  affected by the public Darkmatter API, and any shipped-artifact/doc drift
  checks. Confirm `--dry-run` still launches no provider and mutates no source.

## Phase 6 — Cross-package validation and scope review

- [ ] **Task 6.1 — Run package-area gates.** From `darkmatter/`, run
  `just test` and `just lint`; from `claudine/`, run `just test` and `just lint`.
  Run `just test-l2` only if the final placement includes an L2 target; the
  provider-stub AC10 test should remain in the existing non-focusing tier.

- [ ] **Task 6.2 — Check cross-platform compilation assumptions.** Run the
  available Claudine and Darkmatter Windows check recipes/targets and inspect
  all new filesystem, newline, path, and test-stub code for macOS, Windows, and
  Linux compatibility. Confirm Unix-only executable stubs are correctly gated
  and portable library/CLI tests still compile on Windows.

- [ ] **Task 6.3 — Run the repository-local pre-push equivalent.** From the
  repository root, run `just ci-local claudine darkmatter` (or the current
  package-precise equivalent exposed by the root justfile) after focused gates
  are green. Do not commit or push as part of this plan.

- [ ] **Task 6.4 — Review affected execution scope.** Run GitNexus
  `detect_changes(scope: "compare", base_ref: "main")`, inspect every changed
  symbol and execution flow, and confirm changes are limited to textual hash
  save, inline preparation/closure/status, guardrails, tests, and required
  documentation. Investigate any unexpected lifecycle, sequence, effect-verb,
  or non-save hash consumer before declaring completion.

- [ ] **Validation checkpoint 6.5 — Close against all acceptance criteria.**
  Record evidence for AC1–AC12, including byte comparisons, hash-diff exit-0
  equivalence, repeated-run idempotence, invalid-response non-mutation, the
  mid-run-drift restore with its non-attributing reports, guardrail failure
  fallback, and obedient-provider integration. Confirm the non-goals remain
  untouched: effects-verb fidelity, historical document repair, writer
  attribution/three-way merge, protection of mid-run human edits, and
  YAML-emitter replacement.
