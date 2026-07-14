---
agent: "claude/"
total_phases: 8
created: 2026-07-14
phase: 1
yolo: true
---
# Invalid Frontmatter — Execution Plan

Derived from [`spec.md`](./spec.md) (authority on v1 scope) and [`research.md`](./research.md)
(menu of opportunities, not a plan). This plan adds frontmatter YAML validation and
deterministic repair to `md clean`, backed by a schema-agnostic diagnose/repair engine in
`biscuit-file` and a schema-aware layer in `darkmatter`.

## Scope reminder (from spec)

- **Target surface:** the frontmatter block only. Body ` ```yaml ` fences are never inspected.
- **v1 auto-applies** three `deterministic` tiers, each behind the hard safety gate:
  source normalization, parse-equivalent whitespace cleanup, schema-proven scalar quoting;
  plus the no-schema flagship parse-shape quoting case.
- **v1 detects-and-reports** (never mutates) everything else: duplicate keys, anchor/alias,
  multi-doc, schema-guided key/shape/type repair, and all `non-deterministic-find` lints.
- **Exit code stays stable.** Non-deterministic findings print to STDERR via `TerminalRenderable`.
- **Performance:** relative no-regression; no-frontmatter → zero cost; reparse only candidates.

## Architecture map (grounded in current code)

- `biscuit-file` — schema-agnostic engine. `YamlError::Parse` currently discards
  `serde_yaml_ng::Error` location; `YamlSource::Path` stores only the path. Both are groundwork.
- `darkmatter` — schema-aware layer, reuses `EffectiveSchema` / `DarkmatterSchemas`,
  span-aware `ValidationProblem`, `extract_frontmatter_block`, `markdown::span::SourceSpan`.
- `md clean` (`darkmatter/cli/src/commands/clean.rs`) — integration point; today runs only
  `cleanup*` passes and does **no** schema work.
- Schema-resolution parity mirrors `md compose` wiring already present in
  `darkmatter/cli/src/commands/compose.rs` (`apply_compose_baseline_schema`,
  `with_trigger_schemas`, `--baseline-schema`, `--no-baseline-schema`, `--schema`,
  `--no-trigger-schemas`).

## Cross-cutting requirements (hold in every phase)

- All three OSes: CRLF / CR / LF / BOM handled and tested on the logic path (host is macOS).
- US English (en-US) for all symbols and docs.
- Never `cargo fmt`. Match surrounding style by hand.
- Run `impact({target, direction:"upstream"})` before editing any shared symbol (esp.
  `YamlError`, `YamlSource`, `Yaml::validate`); warn on HIGH/CRITICAL.
- Tests via nextest (`just test`, `just test-l2`); lint via `just lint`.

---

## Phase 1 — Design lock-down & acceptance criteria

Resolve the spec's design-blocking Open Questions *before* code, and write the DoD. These
decisions shape type signatures and cannot be deferred without rework. Produce recommendations,
get sign-off, record the outcome in this feature directory.

- [ ] Draft `decisions.md` in this feature dir capturing a recommendation + rationale for each
      blocking Open Question below.
- [ ] **`YamlSource::Path` retain-vs-reread** — decide: retain raw source at parse time vs.
      reread on demand, and document the TOCTOU stance. (Recommend: retain at parse; drives G2.)
- [ ] **Ordering** — fix where frontmatter validation runs relative to `clean`'s existing
      body-normalization passes. (Recommend: frontmatter validate/repair *before* body cleanup,
      operating on `extract_frontmatter_block` span, so body passes see repaired frontmatter.)
- [ ] **Unconstrained-key behavior for the schema tier** — silent skip vs. non-deterministic
      suggestion when the effective schema does not constrain a key. (Recommend: silent skip in
      v1; suggestion deferred.)
- [ ] **`--json` field names + stability** — pin exact field names for `code`/`span`/
      `classification`/`message`/`repairs[]` and their v1 stability guarantee.
- [ ] **Idempotency target** — commit to: repeated `md clean` is a fixed point (clean output
      cleans to itself). Record as an acceptance criterion.
- [ ] Write the **Acceptance Criteria / Definition of Done** into `decisions.md`, folding in:
      idempotency, never-mutate-on-non-deterministic, byte-for-byte preservation of untouched
      ranges + comments, safety-gate invariants, CRLF/BOM cross-platform behavior. Name the test
      corpus: YAML Test Suite + mutation tests over real monorepo frontmatter.
- [ ] Confirm the **hard performance budget** stays deferred (post-benchmark), and that the
      no-regression posture governs Phase 7.

**Checkpoint:** `decisions.md` exists, every blocking question has a ratified answer, and the
DoD is written. No code has been touched. Stop for sign-off before Phase 2.

---

## Phase 2 — `biscuit-file` groundwork & shared diagnostic types

Foundational and unambiguous; everything downstream depends on it. (Skill: `biscuit-file`.)

- [ ] Run `impact` on `YamlError` and `YamlSource` (both HIGH-risk shared enums); report blast
      radius before editing.
- [ ] **G1 — preserve structured error location.** Extend the parse-error path so
      `serde_yaml_ng::Error::location` (byte/line/col) is retained as structured data, not only
      a rendered string. Keep the existing `Display` output byte-identical for current callers.
- [ ] **G2 — retain original source for `YamlSource::Path`.** Per the Phase 1 retain-vs-reread
      ruling, make the raw source text available to the repair engine (span-patching needs the
      original bytes; `Text`/`Bytes` already retain input).
- [ ] **Shared types.** Add `YamlDiagnostic`, `YamlRepair`, and `SourceSpan` to `biscuit-file`
      per the research sketch (`code`, `span`, `classification`, `message`, `repairs[]`; repair =
      `span` + `replacement` + `explanation`). Add `YamlDiagnosticCode` and
      `YamlRepairClassification` (the three certainty tiers).
- [ ] Re-export the shared types from `biscuit-file`'s public root so `darkmatter` can emit the
      **same** shape.
- [ ] Reconcile `SourceSpan` with `darkmatter::markdown::span::SourceSpan` (byte-offset
      `Range<usize>`): use one vocabulary or a documented conversion; do not fork two span types
      silently.
- [ ] Unit tests: error location is captured for a known malformed input; `Path`-sourced `Yaml`
      exposes its raw bytes; type round-trips (serde) for the diagnostic shape.

**Checkpoint:** `just test` green in `biscuit-file`; `just lint` clean; existing YAML error
`Display` output unchanged (assert in a test). Shared types compile and are re-exported.

---

## Phase 3 — `biscuit-file` engine + safety gate + deterministic auto-fix tiers

The schema-agnostic diagnose/repair core and the three (schema-independent) auto-fix tiers.
Depends on Phase 2. (Skill: `biscuit-file`; `serde` for value comparison.)

- [ ] **Engine scaffold.** Add `Yaml::diagnose() -> Vec<YamlDiagnostic>` and
      `repair_candidates()`. Implement the pipeline: token/source scan → parse (retain location)
      → per-diagnostic bounded candidate generation → reparse each candidate.
- [ ] **Safety gate (Value-equality half).** Implement: parse original → apply edit → reparse →
      require exact `serde_yaml_ng::Value` equality. Auto-apply **only** candidates whose
      invariant is proven; reparse candidates only (never every document). This half lives in
      `biscuit-file`; the schema-equality half is Phase 5.
- [ ] **Span-patching, not reserialization.** Repairs patch original source spans; never
      serialize the parsed `Value` (it drops comments/anchors/style/whitespace). Assert untouched
      ranges are byte-for-byte unchanged.
- [ ] **Tier A — source normalization (`deterministic`, auto).** BOM removal, CRLF/CR → LF,
      parse-equivalent trailing-whitespace and final-newline cleanup. Each edit passes the gate.
- [ ] **Tier B — parse-equivalent whitespace cleanup (`deterministic`, auto).** Whitespace around
      flow collections, mapping colons, sequence markers — applied only when original and
      candidate parse to equal `Value`. Prove the gate rejects a shape-changing edit (e.g.
      `host:localhost` → `host: localhost` is **not** applied).
- [ ] **Tier C — parse-shape quoting (`deterministic`, auto; no-schema flagship).** An unquoted
      plain scalar that is unparseable or resolves to an unintended shape because it begins with a
      reserved indicator (e.g. `title: @daily-report` → `title: "@daily-report"`). Quote the exact
      lexeme; gate on the result.
- [ ] Tests: each tier auto-applies its positive case and refuses its negative case; comments and
      surrounding lines preserved byte-for-byte; CRLF/BOM inputs handled.

**Checkpoint:** `just test` green in `biscuit-file`; `just lint` clean. Safety gate demonstrably
rejects a value-changing candidate. Three deterministic tiers apply on the flagship examples.

---

## Phase 4 — `biscuit-file` report-only tiers (parallelizable with Phase 5)

Detect-and-report tiers that share the Phase 3 engine but never mutate. Independent of the
schema-aware layer → **can run in parallel with Phase 5** once Phase 3's engine + types land.

- [ ] **Duplicate-key detection** — report both source spans; offer candidate repairs
      (keep-first / keep-last / rename / merge / delete) without selecting one. `classification =
      deterministic-find-non-deterministic-solution`.
- [ ] **Anchor / alias detection** — flag undeclared / forward / misspelled aliases; ranked
      candidate anchors as suggestions only.
- [ ] **Multi-document detection** — count `---`-separated documents; report the single-doc
      incompatibility with candidate repairs (split / select / sequence / reject); never pick one.
- [ ] **Schema-free `non-deterministic-find` lints** — ambiguous scalars, suspicious empty
      values, block-scalar smells, comment-truncation / indicator smells, style/indentation
      inconsistency, similar/misplaced keys. Each emits a diagnostic; none mutate.
- [ ] Tests: each detector fires on its positive case, is silent on clean input, and produces
      **zero** repairs that the auto-apply path would ever accept (assert non-deterministic
      classifications are never auto-applied).

**Checkpoint:** `just test` green; report-only tiers produce diagnostics with correct
classifications; a full-suite run confirms no report-only diagnostic mutates source.

---

## Phase 5 — `darkmatter` schema-aware layer (parallelizable with Phase 4)

Layer schema-proven repairs on top, emitting the **same** diagnostic shape via the re-exported
types. Depends on Phase 3 engine + types; independent of Phase 4. (Skill: `darkmatter`.)

- [ ] **Schema-equality safety-gate half.** When a schema is associated, require identical schema
      results before/after the edit, in addition to the biscuit-file `Value`-equality half.
- [ ] **Schema-proven scalar quoting (`deterministic`, auto).** Reuse `EffectiveSchema` /
      `SimplifiedSchema`. Auto-quote only when: original parses, schema rejects the node *solely*
      for not being a string, node is a plain scalar, quoting the exact lexeme passes the full
      schema, and no other range changes (e.g. `release: 1.20` → `release: "1.20"`). Honor the
      Phase 1 unconstrained-key ruling (skip keys the effective schema doesn't constrain).
- [ ] **Schema-guided key correction (report-only).** `timeuot` → `timeout` candidates via edit
      distance / keyboard adjacency / separator + case normalization; never mutate.
- [ ] **Schema-guided shape / type repair (report-only).** Missing-required, wrong-kind, invalid
      enum, range violations → candidate repairs; never mutate; never insert `default` unless
      explicitly opted in.
- [ ] Ensure Darkmatter diagnostics are the re-exported `YamlDiagnostic` shape — one uniform
      output across both layers (assert shape parity in a test).
- [ ] Tests: schema-proven quoting auto-applies with a live schema and is dormant/skipped when no
      schema constrains the key; report-only schema tiers never mutate.

**Checkpoint:** `just test` green in `darkmatter`; `just lint` clean. Schema-proven quoting fires
under the safety gate; report-only schema tiers produce suggestions only.

---

## Phase 6 — `md clean` integration

Fold validation + repair into `md clean` with full compose-parity schema resolution, lazy
short-circuiting, STDERR rendering, and the `--json` contract. Depends on Phases 3–5.
(Skills: `darkmatter`, `cli`, `biscuit-terminal`.)

- [ ] Run `impact` on `run_clean` / `apply_cleanup` in `clean.rs`; report blast radius.
- [ ] **Frontmatter-only wiring.** Operate on the frontmatter block via
      `extract_frontmatter_block` (byte-accurate `yaml_span`); never touch body ` ```yaml ` fences.
- [ ] **Ordering.** Insert frontmatter validate/repair per the Phase 1 ruling relative to the
      existing `cleanup*` passes.
- [ ] **Schema resolution at compose parity.** Mirror `md compose` wiring: inject Darkmatter
      baseline schema by default; honor inline/file `$schema`; repo-scoped trigger discovery
      (ancestor-walk to Git root for `schemas/*.yaml`). Expose the same escape hatches on `md
      clean`: `--baseline-schema PATH`, `--no-baseline-schema`, `--schema`, `--no-trigger-schemas`.
- [ ] **Lazy + short-circuit (performance).** Schema resolution *and* trigger discovery run only
      when a non-empty frontmatter block is present; a no-frontmatter doc pays nothing. Cache the
      trigger-schema discovery + built validator per `clean` run.
- [ ] **Default auto-apply in place.** Deterministic repairs auto-apply by default (the whole
      point — agents won't opt in). Preserve `--save` semantics and the delta report.
- [ ] **Non-deterministic → STDERR.** Render suggestions with `TerminalRenderable` components to
      STDERR; **do not** change the exit code. Existing exit-code contract stays stable — no new
      failure codes. (Any `--strict` / exit gate is explicitly deferred.)
- [ ] **`--json` diagnostic contract.** Emit the committed shape (`code`, `span`,
      `classification`, `message`, `repairs[]{span,replacement,explanation}`) using the Phase 1
      field-name ruling.
- [ ] Update `md clean` CLI help / arg definitions in `darkmatter/cli/src/args/` for the new flags.
- [ ] Tests: L1 for wiring; L2 for STDERR rendering + auto-apply behavior on a file; assert exit
      code unchanged with non-deterministic findings present; assert body `yaml` fences untouched.

**Checkpoint:** `md clean` on a doc with a broken frontmatter scalar auto-fixes it in place,
prints non-deterministic findings to STDERR, exits with the unchanged code, and leaves body
fences and no-frontmatter docs untouched. `--json` emits the committed shape.

---

## Phase 7 — Validation: corpus, mutation, idempotency, cross-platform, perf

The DoD gate. Depends on Phase 6. (Skills: `rust-testing`, `nextest`.)

- [ ] **YAML Test Suite.** Wire the [yaml-test-suite](https://github.com/yaml/yaml-test-suite)
      as a corpus: valid inputs parse, expected-failure inputs are diagnosed, deterministic
      repairs preserve parsed values.
- [ ] **Mutation tests over real monorepo frontmatter.** Inject each supported mistake into real
      frontmatter samples; verify deterministic repairs preserve parsed values + schema results,
      non-deterministic repairs are never silently applied, and comments / untouched ranges stay
      byte-for-byte unchanged.
- [ ] **Idempotency.** Assert `md clean` output is a fixed point (clean output cleans to itself).
- [ ] **Cross-platform.** CRLF / LF / CR / BOM behavior verified on the logic path; document that
      Windows/Linux parity is covered by unit logic since the host is macOS.
- [ ] **Never-mutate guarantee.** A suite-wide assertion that no `non-deterministic-find` or
      `deterministic-find-non-deterministic-solution` diagnostic ever changes source.
- [ ] **Performance no-regression.** Benchmark the two common cases from the spec — no-frontmatter
      docs and already-clean docs — and confirm no measurable regression vs. baseline `md clean`.
      Record numbers; the hard per-document budget stays deferred.
- [ ] Run `just test` + `just test-l2` for `biscuit-file` and `darkmatter`; `just lint` both.

**Checkpoint:** Every DoD criterion from Phase 1 has a passing test. Perf numbers recorded and
show no regression on the two common cases.

---

## Phase 8 — Documentation, drift, and closure

No source behavior changes. Depends on Phase 7. (Skills: `darkmatter`, `Documenter`.)

- [ ] Update `md clean` CLI docs / README for the new validation behavior, flags, and `--json`.
- [ ] Update `biscuit-file` docs for the diagnose/repair engine and shared types; note the
      general-purpose (non-frontmatter-scoped) engine surface for other callers (`bf`, future).
- [ ] Update `docs/dependencies.md` (root + affected per-area) if any crate was added.
- [ ] Update `.claude/skills/` (`darkmatter`, `biscuit-file`) where the new engine / `md clean`
      behavior changes documented architecture.
- [ ] `md hash --save` any edited skill/docs Markdown that carries a `hash:` frontmatter.
- [ ] Move the feature directory to `_completed` per the features lifecycle convention.
- [ ] Run `detect_changes({scope:"compare", base_ref:"main"})` to confirm only expected symbols /
      flows changed before requesting review.

**Checkpoint:** Docs and skills reflect shipped behavior; feature moved to `_completed`;
`detect_changes` shows no surprises.

---

## Dependency & parallelization summary

```
Phase 1 (decisions)
   └─> Phase 2 (groundwork + shared types)
          ├─> Phase 3 (engine + safety gate + deterministic tiers)
          │      ├─> Phase 4 (report-only tiers) ┐
          │      └─> Phase 5 (schema-aware layer) ┘  (4 and 5 run in PARALLEL)
          │
          └────────────> Phase 6 (md clean integration) [needs 3, 4, 5]
                              └─> Phase 7 (validation/DoD)
                                     └─> Phase 8 (docs + closure)
```

- **Parallel:** Phase 4 and Phase 5 are independent once Phase 3's engine + shared types land.
- **Serial gates:** Phase 1 sign-off precedes any code; Phase 6 needs all of 3/4/5; Phase 7
  gates on the full integration; Phase 8 gates on green validation.
