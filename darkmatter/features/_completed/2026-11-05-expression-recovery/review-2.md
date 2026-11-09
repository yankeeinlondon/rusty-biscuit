---
$schema: feature-review.yaml
ready: false
findings:
    - "R2-F1 (medium): Recursive missing-source format checks use unexpanded environment references"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-11-09T00:55:15-08:00
spec: 2026-11-05-expression-recovery/spec.md
implemented: true
description: "A **feature** review of `2026-11-05-expression-recovery/spec.md`"
feature: 2026-11-05-expression-recovery/review-2.md
previous: 2026-11-05-expression-recovery/review-1.md
next: 2026-11-05-expression-recovery/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/log.md
implemented_by: claude/opus
---

# Expression recovery review, iteration 2

The feature is **not production ready**. All required instances of the three
previous findings are fixed. One new sibling remains: a missing recursive
reference whose filename or extension comes from an environment variable is
mistaken for an unsupported format. This regression was introduced by the
recursive-format repair; it is not an unchanged original instance. It can be
repaired within the existing decisions without a human ruling.

## Previous Finding Verification

The repair's `log.md`, under `### Finding Ledger`, supplies the child inventory.
The following assessment uses inspected implementation, rerun tests, and fresh
command reproductions. Completion flags are not evidence. There are no earlier
reviews or pending human rulings in this feature directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Missing sources bypass invalid-request checks | R1-F1.1–R1-F1.23; individual checks below | **verified fixed** | Missing/present computed-request tests, planner tests, CLI regression, shared validation ordering | No original instance remains. New sibling R2-F1 concerns environment interpolation in the added recursive branch. |
| R1-F2: Stale remote cache hides a denied redirect | R1-F2.1–R1-F2.19; individual checks below | **verified fixed** | Transport-only stale branch, public composition and CLI regressions, fresh warm-cache reader sweep | Nothing |
| R1-F3: Value-only try calls return a generic handler error | R1-F3.1–R1-F3.5; individual checks below | **verified fixed** | Typed rejection and rerun direct-call, evaluator, and lifecycle tests; host-registration invariant | Nothing |

### Request-validation instances

The public composition tests below use computed inputs, ensuring runtime
validation is reached. Named tests were rerun through `just test`.

| Child ID | Verification evidence and result |
|---|---|
| R1-F1.1 | `yaml_delimiter::{missing,present}`: invalid options in both. |
| R1-F1.2 | `json_delimiter::{missing,present}`: invalid options in both. |
| R1-F1.3 | `jsonl_delimiter::{missing,present}`: invalid options in both. |
| R1-F1.4 | `json5_delimiter::{missing,present}`: invalid options in both. |
| R1-F1.5 | `toml_delimiter::{missing,present}`: invalid options in both. |
| R1-F1.6 | `yaml_header::{missing,present}`: invalid options in both. |
| R1-F1.7 | `tsv_semicolon::{missing,present}`: invalid options in both. |
| R1-F1.8 | `csv_headerless_object_rows::{missing,present}`: row-shape conflict in both. |
| R1-F1.9 | `csv_string_rows::{missing,present}`: row-shape conflict in both. |
| R1-F1.10 | `tsv_string_rows::{missing,present}`: row-shape conflict in both. |
| R1-F1.11 | `explicit_format::{missing,present}`: invalid options in both; planner test also checks the override across candidates. |
| R1-F1.12 | `recursive_unsupported_extension::{missing,present}` and fresh `%optional.xyz` CLI pair: unsupported format in both. |
| R1-F1.13 | Existing missing/present unsupported-extension tests pass; nonrecursive planning uses the shared validator. |
| R1-F1.14 | `recursive_supported_extension::{missing,present}` and fresh `%optional.yaml` CLI pair: fallback when missing, loaded values when present. |
| R1-F1.15 | `computed_invalid_options_are_not_recovered` and CLI `computed_invalid_options` pass; delimiter validation precedes resolution. |
| R1-F1.16 | `not_a_container_is_not_recovered` passes; contract preparation precedes resolution, including a miss. |
| R1-F1.17 | `this_import_is_not_recovered` passes; same pre-resolution contract preparation. |
| R1-F1.18 | `missing_contract_import_is_not_recovered` and CLI contract-import case pass; imports are prepared before source lookup. |
| R1-F1.19 | All four `valid_request_recovers_only_a_miss` missing/present pairs pass, as does the valid CLI control. |
| R1-F1.20 | `call_check` and `load_data` suites pass; passive options/row checks do not probe the source. |
| R1-F1.21 | Both executors still use the typed recovery classifier; `try` and recovery suites pass without broadening it. |
| R1-F1.22 | `applicability_outranks_a_missing_file` passes, including computed/literal invalid requests and a valid missing-file control. |
| R1-F1.23 | Inspected remote and present-local paths: reads follow `plan_read`. Other file readers accept no data-format/options/row contract. |

### Cache-refusal instances

Fresh CLI probes used a loopback server, separate temporary cache roots, and
a successful warm-up before changing the same endpoint to redirect to an
unallowed host. Each reader below was checked independently. Object results
were wrapped in an array for `as_json`; scalar readers used ordinary
interpolation. Discovery used `--shell` without a data-output flag.

| Child ID | Verification evidence and result |
|---|---|
| R1-F2.1 | Public `warm_cache_never_hides_a_refused_redirect`, CLI cache regression, and fresh `load_data` probe: refusal propagates. Public test also proves no denied-target request or fallback read. |
| R1-F2.2 | Fresh `file_exists`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.3 | Fresh `frontmatter`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.4 | Fresh `markdown_title`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.5 | Fresh `markdown_body_empty`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.6 | Fresh `validate_schema`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.7 | Fresh `try_frontmatter`: warm-up exits 0; refused revalidation returns `ok: false`, `value: null`, and the policy message, preserving error-as-data behavior. |
| R1-F2.8 | Fresh `::file`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.9 | Fresh `::code`: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.10 | Fresh `--shell` remote-child discovery: warm-up exits 0; denied revalidation exits 1 with the policy message. |
| R1-F2.11 | `redirect_to_denied_host_is_fatal` passes; cold fetch cannot serve stale content. |
| R1-F2.12 | `strict_failure_without_fallback_errors` passes; strict mode cannot enter the stale branch. |
| R1-F2.13 | `fallback_does_not_serve_a_no_cache_entry_after_failed_revalidation` passes. |
| R1-F2.14 | Denied-host warm-cache/memo and fresh-cache tests pass; `check_allowed` precedes store lookup. |
| R1-F2.15 | Public `warm_cache_covers_a_transport_failure`, cache unit control, and CLI transport fallback control pass. |
| R1-F2.16 | Inspected successful revalidation response: UTF-8 decoding returns its typed transport error before the stale-on-fetch-error branch. Invalid-UTF-8 recovery test passes. |
| R1-F2.17 | `fetch_remote_text` reuses the failed fetch slot; `fetch_remote_text_keeps_the_failed_fetch_kind` passes. |
| R1-F2.18 | Inspected `effects::http_post`: uncached request, with no stale response to substitute after refusal. |
| R1-F2.19 | Inspected data source memo: admission follows successful fetch completion; errors return first. Warm-cache/memo denial test passes. |

### Value-only invocation instances

| Child ID | Verification evidence and result |
|---|---|
| R1-F3.1 | `direct_value_call_of_try_is_rejected`: `ContractViolation`, authoring-fatal, nonrecoverable. |
| R1-F3.2 | `direct_value_call_of_evaluator_owned_functions_is_a_contract_violation`: shared rejection for `and`, `or`, `map`, and `filter`. |
| R1-F3.3 | `callback_function_without_inline_callback_is_a_contract_violation`: ordinary evaluator preserves the typed rejection. |
| R1-F3.4 | Lifecycle `callback_function_with_template_operand_is_a_contract_violation`: template-operand value call preserves the rejection. |
| R1-F3.5 | Inspected `call_host`: separate unavailable-implementation path; host registration rejects lazy functions, excluding evaluator-owned host bindings. |

## Unblocked Findings

### R2-F1 (medium): Recursive missing-source format checks use unexpanded environment references

**Authority: defect and regression.** A valid optional reference such as
`%{{DATA_FILE}}`, with `DATA_FILE=optional.yaml`, is rejected as
`dm.data.unsupported_format` while missing, but loads successfully once present.
This violates the spec's recoverable well-formed missing-source boundary and
preservation of reference-resolution rules. Before the R1-F1 repair, recursive
misses bypassed this check and remained recoverable.

**Defect class:** deciding request validity from an authored reference before
expanding its environment placeholders misclassifies a valid source.

In the **darkmatter** package,
[`absent_candidates`](../../lib/src/markdown/compose/expression/data/load.rs:350)
uses `file_ref.raw()` for recursive references. Its comment claims the raw
extension is the one any match would have. The **biscuit-file** package's
[`resolve_core`](../../../biscuit-file/lib/src/file_reference/resolve.rs:100)
expands the captured environment before recursive matching. A variable can
supply the filename or extension, so the representations can name different
formats. `judge_absent` and `plan_read` trust the raw extension and return a
nonrecoverable error. The evaluator correctly propagates that error; its
classifier should remain unchanged.

Save this in a temporary folder as `doc.md`:

```markdown
{{ as_json(try(load_data("%{{DATA_FILE}}", "string[]"), [])) }}
```

Run `DATA_FILE=optional.yaml md compose doc.md --output json`:

- Missing `optional.yaml`: exit 1, unsupported-format diagnostic, no document.
- Write `- loaded` to `optional.yaml`: exit 0, document `["loaded"]`.
- Expected missing result: exit 0, document `[]`, no recovered-cause diagnostic.

Medium priority reflects the unusual combination of recursive lookup and an
environment-driven filename. It is supported reference syntax with a small,
bounded repair; no new syntax, security default, or human decision is needed.

#### Bounded class sweep

Command rows used the normal CLI path with one missing/present pair per shape.
Captured values were `DATA_FILE=optional.yaml`, `DATA_EXT=yaml`, and
`DATA_DIR=nested`. Literal expression arguments preserve the reference
placeholders for `FileReference`, avoiding frontmatter interpolation.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Recursive candidate derivation | `%{{DATA_FILE}}` | Missing: unsupported format; present: loaded array | Missing: `[]`; present: loaded array |
| R2-F1.2 | Recursive extension derivation | `%optional.{{DATA_EXT}}` | Missing: unsupported format; present: loaded array | Missing: `[]`; present: loaded array |
| R2-F1.3 | Downstream option applicability | `%{{DATA_FILE}}`, YAML, `delimiter: ";"` | Missing: unsupported format; present: invalid options | Invalid options in both |
| R2-F1.4 | Recursive directory interpolation | `%{{DATA_DIR}}/optional.yaml` | Missing: `[]`; present: loaded array | Same; clean |
| R2-F1.5 | Nonrecursive candidate planning | `{{DATA_FILE}}` | Missing: `[]`; present: loaded array | Same; clean |
| R2-F1.6 | Nonrecursive extension planning | `optional.{{DATA_EXT}}` | Missing: `[]`; present: loaded array | Same; clean |
| R2-F1.7 | Explicit-format override | `%{{DATA_FILE}}`, `format: "yaml"` | Missing: `[]`; present: loaded array | Same; clean |
| R2-F1.8 | Literal supported extension | `%optional.yaml` | Missing: `[]`; present: loaded array | Same; clean |
| R2-F1.9 | Literal unsupported extension | `%optional.xyz` | Unsupported format in both | Same; clean; retain the author's decision |
| R2-F1.10 | Present-source format selection | Present controls for rows 1–9 | Resolved path supplies the correct format/applicability decision | Same; clean |
| R2-F1.11 | Passive call checking | Placeholder calls in rows 1–7 | Valid syntax reaches execution; no early format rejection | Same; clean; analysis must not read inputs |
| R2-F1.12 | Ordinary recovery decision | Missing-source errors in rows 1–3 | Propagates the loader's nonrecoverable error | Same classifier contract; repair the producer |
| R2-F1.13 | Lifecycle recovery decision | Code invariant: same loader and typed classifier | No independent format derivation; consumes the same loader result | Preserve classifier; repair common loader; clean consumer |

Other recursive roots enter the same helper, with no root-specific format
implementation. Remote reads use URL-path format selection and never enter
this local branch. Contract admission precedes resolution, and other file
readers have no data-format/options/row contract. These invariants bound the
remaining sibling sites.

**Recommended repair:** derive the effective filename/extension through
`FileReference` and the captured context before the shared request checks.
Planning an equivalent nonrecursive target can supply this information without
probing or inventing a resolved identity; retain recursive search and authored
diagnostic identity. Do not add a second environment interpolator or weaken
unsupported-format rejection. Add public composition missing/present tests for
whole-filename and extension interpolation, the invalid-options control, and
one CLI regression. Correct the helper's raw-extension comment in the repair.

## Blocked Findings

None. The recommendation follows existing syntax and recovery decisions.

## Recurrence

R2-F1 repeats the missing-source request-validation class of **review-1.md,
R1-F1: Missing sources bypass invalid-request checks**. It is a **regression
introduced by that repair**, rather than an unchanged original instance.
The original literal recursive child R1-F1.12 is fixed. That repair should
also have swept filenames and extensions after reference interpolation and
the format/applicability decisions consuming them. The table above carries
the full bounded inventory; no new error category or broader catch is needed.

## Verification and acceptance coverage

Commands ran in this session's absolute worktree directory. Filters overlap;
counts are not a distinct-test total. All listed runs passed, with no reported
L1 test at or above five seconds.

| Command from `darkmatter/`, except where noted | Result |
|---|---|
| `just test expression_recovery` | 128 passed |
| `just test try` | 568 passed |
| `just test load_data` | 505 passed |
| `just test remote` | 260 passed |
| `just test direct_value_call` | 2 passed |
| `just test callback_function` | 2 passed |
| `just test data::load::` | 8 passed |
| `just test call_check` | 112 passed |
| `just test recovery` | 243 passed |
| Root: `just check-tier-coverage darkmatter` | Zero stranded tests |
| Debug CLI, temporary local fixtures | Nine missing/present reference-shape pairs above |
| Debug CLI, local HTTP fixtures | Warm-cache refusal sweep of all ten previous reader/discovery sites |

Exploratory commands with failed controls were corrected before drawing
conclusions: `as_json` requires an array, `--shell` has its own output mode,
and authored frontmatter placeholders are not literal reference inputs. An
exploratory frontmatter-input matrix timed out and is not counted as evidence.
The tables report successful corrected controls. The earlier full-suite and
lint results remain implementation-log claims, separate from these reviewer
runs. No production source edits, formatting, commits, real terminal windows,
or live external service requests were needed.

| User-facing requirement | Strongest verification inspected/rerun | Assessment |
|---|---|---|
| Normal CLI optional-data loading and invalid-request propagation | L1 public composition and CLI tests, computed-input matrix, guarded unreadable-source API test | Original instances pass; R2-F1 adds the recursive environment sibling. |
| Falsy successes, lazy fallback, once-only evaluation, nested failures, shared budget | L1 instrumented evaluator/recovery tests | Appropriate level; passed. |
| Fatal request, programming, context, boundary, provider, policy, and resource errors | L1 classifier, composition, binding, CLI, warm-cache tests | Appropriate level; original refusal defect fixed. |
| Body/frontmatter, page guards, transclusion conditions/targets, callbacks, collection regions, shell templates | L1 normal-entry-point surface tests | Appropriate level; passed; restricted surfaces remain restricted. |
| Lifecycle expressions | L1 ordinary/template executor and `try` lifecycle tests; common handler/classifier inspection | Appropriate level; passed. Claudine coverage also inspected in the previous review remains applicable. |
| Schema-slot recovery, permission gating, empty lists, determinism, selected reads | L1 public slot-session tests | Appropriate level; passed. |
| Read sets, memo reuse, distinct fallbacks, invalidation after effects | L1 public composition/data-runtime tests | Appropriate level; passed; accepted missing-probe identity behavior retained. |
| Quiet handled causes, retained/blocking warnings, callback provenance, fallback locations | L1 report assertions and CLI envelopes | Appropriate level; passed. |
| Union types, nullable results, typed empty-array fallback, constrained alternatives | L1 passive typing and DMLS callback completion | Appropriate level; `try` run passed. |
| DMLS completion/hover/diagnostics, no input execution; host-name collision | L1 protocol/passive-analysis and host-registration tests | Appropriate level; `try` run passed. |
| Catalog and generated documentation | L1 catalog checks and inspection of recovery/loading-data/engine/cache docs | No new mismatch beyond R2-F1's faulty format assumption. |

The consolidated L1 targets declare the recovery modules; the tier audit
finds no stranded tests. These requirements concern expressions and structured
reports, with no terminal-emulator or keyboard promise requiring Level 2 or
Level 3. Cross-OS proof remains CI's responsibility as requested.

### Input robustness scope

This work changes a text-expression form, error provenance, request-check
ordering, and cache fallback. It adds no configuration loader or field
validator, so the configuration robustness matrix does not apply. Data
decoders are unchanged. The smell search found the candidate planner's
`.ok()`, existing cache-manifest miss handling, and the table-row iterator;
none adds a configuration-field default. R2-F1 follows the candidate value
through format selection, applicability, and both recovery consumers.

The previous review's accepted choices remain accepted: library-level
unreadability evidence, existing location precision, missing-probe identities,
typed read-failure recovery only, fatal nested composition, and the catalog's
fallibility convention. No new evidence requires reopening them.
