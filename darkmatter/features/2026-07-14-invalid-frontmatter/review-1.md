---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T18:10:47-07:00
spec: 2026-07-14-invalid-frontmatter/spec.md
implemented: true
implemented_by: claude/default
log: darkmatter/features/2026-07-14-invalid-frontmatter/log.md
description: "A **feature** review of `2026-07-14-invalid-frontmatter/spec.md`"
feature: 2026-07-14-invalid-frontmatter/review-1.md
---

# Review 1 — Invalid Frontmatter

## Verdict

This feature is **not ready for production**. The source-first YAML analysis in
`biscuit-file` and part of the Darkmatter schema-aware library layer exist, but
the feature is not connected to `md clean`. The flagship invalid-frontmatter
fixture still exits with the legacy parse error, the focused schema-cleaning
test gate fails, and the acceptance matrix's CLI, corpus, performance, and
cross-platform evidence is absent.

Sniff identifies `biscuit-file` and `darkmatter` as the affected package areas.
GitNexus reports a CRITICAL graph blast radius for `analyze_yaml` (171 upstream
symbols, 69 direct), but no production execution process currently reaches it;
the reported callers are overwhelmingly the new analyzer tests. That agrees
with the direct inspection: the implementation has not reached the production
CLI path.

## Findings

### Critical — `md clean` never invokes the invalid-frontmatter pipeline

`darkmatter/cli/src/commands/clean.rs:38` still calls `load_markdown` before any
raw-source frontmatter analysis. Both stdout and `--save` modes therefore
require frontmatter to parse before cleanup can begin. The command then renders
through `Markdown::as_string()` at line 46 or writes it at line 71, so the
specified span-based raw-frontmatter reconstruction is also absent.

The `Clean` arguments in `darkmatter/cli/src/args/command.rs:40-83` expose none
of `--json`, `--schema`, `--baseline-schema`, `--no-baseline-schema`, or
`--no-trigger-schemas`. There is consequently no CLI path for schema-aware
repair, deterministic human suggestions on STDERR, the JSON envelope, or the
schema precedence contract.

Direct Level-1 execution confirms the impact:

```text
md clean .../baselines/invalid-reserved.md
MarkdownError: frontmatter parse failed
EXIT_STATUS=1
```

The flagship `title: @daily-report` case should instead be repaired and emitted
successfully. Implement Phase 6's raw input path, run schema-free analysis
before constructing `Markdown`, resolve schema state only after parsing is
restored, and assemble the output from the repaired raw frontmatter plus the
cleaned body.

### High — The focused schema-aware Level-1 gate has six failures

`cargo nextest run -p darkmatter -E 'test(/clean_quoting/)'` runs 37 tests: 31
pass and 6 fail. Four fixtures use the unsupported schema type `integer` and
panic during setup (`clean_quoting.rs:136-140`, `231-236`, `330-333`, and
`418-423`), so the multiple-problem, coercion, root-union, and determinism
requirements are not actually exercised.

The two S1 result-set tests also fail at `clean_quoting.rs:476` and line 491.
`schema_result_set_identical` compares results from coercing validation
(`clean.rs:323-335`), which can reduce both an authored string and an authored
number to the same empty problem set. Its own tests require those type-changing
cases to compare as different. Reconcile the intended safety contract with the
validator semantics, then make the focused suite green. The separate S1
`serde_yaml_ng::Value` equality check limits the immediate auto-apply risk, but
it does not make a contradictory safety helper and red acceptance tests
releasable.

The test target also has an unused `std::path::Path` import, which will need to
be removed before a warnings-denying lint gate can pass.

### High — Every new user-facing CLI requirement lacks its required Level-1 proof

The acceptance matrix correctly assigns these process/file behaviors to Level
1. The implementation contains existing `md clean` tests for parseable
Markdown cleanup, stdin, save, and flag errors, but none covers the new
invalid-frontmatter behavior. The dedicated targets `clean_json.rs` and
`clean_schema.rs` do not exist.

| User-observable requirement | Strongest verification present | Assessment |
|---|---|---|
| D-1: default file/stdout repair | Library Level 1 for the analyzer; no CLI test | **Gap.** The real command fails the flagship input. |
| D-2: explicit and implicit stdin repair | Existing Level 1 only for parseable Markdown | **Gap.** No invalid-frontmatter stdin proof. |
| D-3/D-4: `--save` repair and invalid-source delta | Existing Level 1 only for parseable Markdown | **Gap.** The raw invalid-source save path is absent. |
| D-5: `--json` envelope and channel contract | None | **Gap.** The flag and golden test target are absent. |
| D-6: human suggestions on STDERR with exit 0 | None | **Gap.** No renderer or spawned-process assertion exists. |
| D-7: unrepairable YAML retains exit 1 and leaves files untouched | Direct manual exit check only | **Gap.** No repeatable Level-1 file-integrity test exists. |
| D-8: absent/empty frontmatter bypasses YAML/schema/trigger work | Existing Level 1 checks ordinary output only | **Gap.** The required counter proof is absent. |
| D-9: byte-level idempotency | Library tests cover individual repairs | **Gap.** No end-to-end CLI fixture loop exists. |
| D-10: schema precedence and all schema flags | Schema library Level 1 is partial and currently red | **Gap.** Flags and CLI tests are absent. |
| D-11: broken YAML in a fenced body remains outside analysis | None | **Gap.** The sentinel CLI fixture/test is absent. |

No requirement depends on a terminal emulator's rendering, input encoder, or
OS keyboard events. Level 2 and Level 3 are therefore not required; the missing
Level-1 process tests are the appropriate blockers.

### High — Safety, corpus, performance, and platform acceptance evidence is incomplete

The acceptance matrix names integration/property targets for YAML safety,
parse counts, corpus cases, mutation fixtures, schema-quoting safety, and clean
counters. The following required targets are absent:

- `biscuit-file/lib/tests/yaml_corpus.rs` and its pinned corpus;
- `biscuit-file/lib/tests/yaml_mutation.rs`;
- `biscuit-file/lib/tests/yaml_safety.rs`;
- `biscuit-file/lib/tests/parse_count.rs`;
- `darkmatter/lib/tests/schema_quoting_safety.rs`; and
- `darkmatter/lib/tests/clean_counters.rs`.

There is likewise no recorded Phase 7 benchmark comparison for the no-
frontmatter and clean-frontmatter hot paths, and no macOS/Windows/Linux CI
evidence for both affected packages. Unit tests cover many normalization,
reserved-indicator, span, CRLF, and report-only cases, but they do not replace
the matrix's suite-wide invariants or the user-visible CLI proofs.

Before release, add the pinned offline corpus and mutation/property suites,
prove the parse/schema/trigger work-count invariants, retain the benchmark
comparison, and execute the affected package gates on all three supported
operating systems.

### Medium — The convenience API can accidentally analyze the same source twice

`Yaml::diagnose()` and `Yaml::repair_candidates()` each call `analyze_yaml`
independently in `biscuit-file/lib/src/yaml/types.rs:345-365`. A consumer that
needs both diagnostics and repairs performs the full scan twice, which conflicts
with the feature's parse-once direction. The CLI integration should retain one
`YamlAnalysis` and derive both views from it. Consider also exposing that
single-analysis path as the documented ergonomic choice so callers do not
naturally select the duplicate-work pair.

## Verification performed

- `biscuit-file/just test`: passed (663 passed, 4 skipped across library and
  CLI test binaries).
- `darkmatter/just test`: the full Level-1 area run was stopped at the
  non-interactive time limit after 2,118 tests passed with no failures observed
  to that point; this is not a completed gate.
- Focused schema-cleaning nextest run: **failed** (31 passed, 6 failed).
- Direct `md clean` execution on `baselines/invalid-reserved.md`: **failed**
  with the legacy frontmatter parse error and exit status 1.
- `md clean --help`: confirmed all new JSON/schema controls are absent.
- The accepted test level for the new CLI contract is Level 1. No Level-2 or
  Level-3 terminal behavior is specified.

`just lint`, doctests, the Phase 7 benchmark comparison, and cross-platform CI
were not treated as meaningful release gates after the core CLI path and
focused tests failed. They remain required after the blockers above are fixed.
