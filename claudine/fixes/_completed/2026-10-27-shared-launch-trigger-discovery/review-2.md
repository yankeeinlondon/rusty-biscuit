---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-27T17:34:16-07:00
spec: 2026-10-27-shared-launch-trigger-discovery/spec.md
implemented: false
description: "A **feature** review of `2026-10-27-shared-launch-trigger-discovery/spec.md`"
feature: 2026-10-27-shared-launch-trigger-discovery/review-2.md
previous: 2026-10-27-shared-launch-trigger-discovery/review-1.md
---

# Review of shared launch trigger discovery

The feature is **production ready**. The previous finding is verified fixed in every required instance, and this review found no new defects. All 613 selected tests passed. The implementation preserves one discovery per pre-validation call and two independent effective-schema preparations.

## Previous Finding Verification

Read `review-1.md`, the repair log's initial and final Finding Ledgers, and the original implementation log before verification. The following outcomes come from inspecting the current implementation and running the tests, rather than accepting the logs' completion claims. There are no earlier open findings or human rulings in this directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — File-edit and recovery tests rebuild the context they must retain | All eight child rows in the repair's final ledger | **verified fixed** | Every row checked below; repaired tests pass through public pre-validation with retained inputs. | Nothing. |
| R1-F1.1 — Freshness across trigger edits | Absent → created → changed → removed → invalid → repaired, retaining one source/context | **verified fixed** | In claudine's [freshness test](../../lib/tests/l1/invalid_trigger_findings.rs:375), context and source are created before the first call; every verdict uses the closure borrowing them. Required-property errors, successful checks, and warning delivery assertions pass. | Nothing. |
| R1-F1.2 — Failed-scan recovery | Failing pre-validation, standalone cleanup, and recovered pre-validation share source/context | **verified fixed** | In claudine's [recovery test](../../lib/tests/l1/invalid_trigger_findings.rs:412), all three operations use retained inputs. Invalid UTF-8 returns `SchemaLoad` naming the file, counts 1 discovery / 0 preparations, and publishes no partial findings. Cleanup returns unchanged inputs. Repair succeeds with counts 1 / 2. | Nothing. |
| R1-F1.3 — First warning reason survives edits | Retain the first reason and deliver once after changing the invalid trigger | **verified fixed** | Claudine's [first-reason test](../../lib/tests/l1/invalid_trigger_findings.rs:348) now also retains source/context; the original warning and once-only delivery assertions pass. Already clean for its warning contract; the repair strengthened input reuse. | Nothing. |
| R1-F1.4 — Successful work counts | Repeated checks retain source/context and independently count 1 / 2 | **verified fixed** | Claudine's [counter test](../../lib/tests/l1/invalid_trigger_findings.rs:228) constructs inputs before observation and passes both per-call assertions. Previously clean; remains clean. | Nothing. |
| R1-F1.5 — Discovery order and delivery | Repeated checks, standalone cleanup, status, and a fresh warning record | **verified fixed** | Claudine's [delivery test](../../lib/tests/l1/invalid_trigger_findings.rs:180) passes filename order, once-only delivery, and renewed delivery with a new record. Its contract does not require retained-context file freshness. Previously clean; remains clean. | Nothing. |
| R1-F1.6 — Incorrect implementation-log claim | Correct the original claim of same-source/context freshness coverage | **verified fixed** | The Phase 2 correction in `implementation-log.md` explicitly identifies the earlier claim as untrue when recorded and describes the three repaired tests. Current code agrees. | Nothing. |
| R1-F1.7 — Disk/embedded precedence siblings | Schema-root edits in the existing precedence tests | **verified fixed** | Inspected claudine's [precedence test](../../lib/tests/l1/claudine_schemas_embedded.rs:757) and fixture; calls model fresh launches. Removing successive roots exposes the next root and finally the embedded bundle. The embedded-schema suite passes. Previously clean; remains clean. | Nothing. |
| R1-F1.8 — Other schema-test siblings | `schema/tests.rs` and `schema/supplied/tests.rs` | **verified fixed** | Inspected file writes and pre-validation call sites: neither contains a schema edit between repeated pre-validation calls needing this repair. All 126 selected schema tests pass. Previously clean; remains clean. | Nothing. |

This is a complete repair, with no partially repaired instance or newly discovered sibling. The review retains the previous review's accepted implementation choices: authored typing for the removable optional value, the existing missing-file error classification, historical allocation baselines, and Claudine-only completion checks. No new decision requires author input.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec review and acceptance coverage

In the claudine package, [discover_schema_owner](../../lib/src/composition/schema/mod.rs:581) discovers from the document context with the embedded bundle, records skipped-trigger findings, and then applies launch resolution and fallback settings. [pre_validate_verdict](../../lib/src/composition/schema/mod.rs:855) owns that result locally, borrows it for cleanup, replaces caller records with those built from the cleaned overrides, and borrows it for the final preparation. Neither the owner nor its registry is cloned or retained beyond the call.

Both preparations call the existing Darkmatter matching and schema assembly code. Matching keeps the source file and authored `$schema`, while using each pass's current frontmatter. The discovered registry holds resolved payloads and imports, so an outside trigger edit cannot substitute new contents between these preparations. The authored schema still resolves independently for each preparation. Standalone cleanup and status retain independent discovery; authored-only loading remains trigger-free.

| Requirement | Verification and result |
| --- | --- |
| Acceptance 1: one discovery and two preparations, observed around public pre-validation | **Level 1**, matching-trigger counter test passes twice with retained source/context. Fixture and context construction precede observation. |
| Acceptance 2: cleanup changes matching; overrides and caller origins remain correct | **Level 1**, removal of the selecting optional permits the final verdict; the valid-value control still rejects missing `ticket`. Override removal, drop attribution, and the surviving launch-relative file control pass. |
| Acceptance 2: warnings, authored errors, and no applicable schema | **Level 1**, mixed valid/invalid triggers, first-reason/order/delivery, missing and malformed authored schemas, authored references to triggers, and unchanged no-schema outputs pass. CLI tests verify launch rejection/success and warning wiring. |
| Acceptance 2: freshness and recovery with retained inputs | **Level 1**, repaired tests verified row by row above. Separate successful calls also each count fresh discovery. |
| Preserved behavior: root precedence, embedded fallback, validation modes, controls, templates, literal data, and standalone entry points | **Level 1**, schema, embedded-schema, and CLI launch suites pass. Inspection confirms unchanged validation-phase selection and retired-grammar preference. |
| Acceptance 3: allocation savings at 1, 10, and 100 schema files | Inspected the benchmark and six raw rows in `implementation-log.md`, including revision/working-tree state, profile, and toolchain. Recorded bytes are 171,606 / 976,132 / 9,017,650, reductions of 48.3% / 49.7% / 50.0% against the disclosed historical baseline. Counts are 1 / 2; inspection confirms no registry copy. This review did not rerun the allocation experiment. |
| Acceptance 4: area gates and ownership boundary | Implementation and repair logs record `just test` with 11,767 passing tests and `just lint` exit 0. This review independently ran the focused checks below; it did not repeat the full area gates. No Darkmatter production code or public API changed. |
| Acceptance 5: current documentation and comments | Inspected the schema topic's call-lifetime explanation and diagram, finding-record module documentation, claudine skill, and benchmark description. They distinguish a pre-validation call from a whole launch and describe implemented behavior directly. |

All user-observable requirements here concern filesystem discovery, schema results, warning delivery, or launch wiring. In-process and hermetic subprocess Level 1 tests exercise those boundaries. No terminal rendering or keyboard-input requirement needs Level 2 or Level 3 evidence. Cross-OS runtime evidence remains with CI and does not block readiness.

## Verification performed

The first four commands ran from the starting worktree's absolute `claudine/` directory using Nextest; the last ran from the repository root.

| Command | Result |
| --- | --- |
| `just test-library --features work-counters invalid_trigger_findings::` | 14 passed |
| `just test-library --features work-counters composition::schema::` | 126 passed |
| `just test-cli launch_trigger_schemas::` | 57 passed; recipe enables `test-fixtures` |
| `just test-library --features work-counters claudine_schemas_embedded::` | 416 passed |
| `just check-tier-coverage claudine` | Exit 0; no stranded tests |

All 613 selected tests passed; no executed test exceeded five seconds. Compilation emitted the existing compact-unwind linker-size warning. No production or test code was changed during review.

Changed tests are registered in the declared `l1` target despite `autotests = false`, and their names remain selected by L1. Both the local area test recipe and package CI metadata enable `work-counters`. The repaired non-counter branches retain ordinary behavior assertions; the repair log separately records their passing run without that feature.

## Input robustness

This refactor changes orchestration around existing loaders; it adds no field validator, deserializer, format, or default. Field and document shapes continue through the existing Darkmatter loaders. Preserved outcomes are: invalid individual triggers become located warnings while valid rules apply; failed discovery returns a schema error without a partial registry or findings; authored-schema failures remain typed errors after discovery findings are recorded. Standalone cleanup intentionally returns unchanged inputs on discovery/preparation failure.

Inspected the changed adapter for `serde(default)`, absent/null-conflating field options, filtered string collections, `unwrap_or_default`, and discarded parse errors. No new coercion was introduced. The retained best-effort cleanup preparation branch is required by the specification. The executed embedded-schema suite includes its existing JSON/YAML control, field-shape, duplicate-key, and invalid-content cases; this review does not claim an exhaustive new matrix for unchanged parsers.

## Recurrence

None. The previous defect class is fully repaired; no unchanged instance, repair regression, or missed sibling remains within the reviewed boundary.
