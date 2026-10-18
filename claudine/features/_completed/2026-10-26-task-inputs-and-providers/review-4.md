---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-18T08:58:53-07:00
spec: 2026-10-26-task-inputs-and-providers/spec.md
implemented: false
description: A **feature** review of `2026-10-26-task-inputs-and-providers/spec.md`
feature: 2026-10-26-task-inputs-and-providers/review-4.md
previous: 2026-10-26-task-inputs-and-providers/review-3.md
---

# Review 4: task inputs and providers

The feature is **production ready** under the specification and the author's three accepted schema decisions in [review 1](review-1.md#human-rulings). There are no new findings or remaining findings. No further human decision is required for this review.

The review checked the specification, the implementation plan, all earlier reviews, and the repair's Finding Ledger in [the implementation log](log.md#implementation-of-review-findings-3). The log's completion claims were checked against source and fresh test runs. This review changes only review documents and specification frontmatter; temporary reproduction tests were removed afterward.

## Previous Finding Verification

R3-F1 is the only finding in the previous review. Its required instances include every child added by the repair ledger, through R3-F1.22. Earlier findings were already closed by review 3; their regression checks are included below for clarity. Earlier observations are not carried as findings, and accepted schema decisions are not reopened.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1: Shell-dependent selection still renders a stale launch identity | R3-F1.1–R3-F1.22, individually accounted for in the table below | verified fixed | All maintained provider CLI cases pass; five additional public CLI probes pass. Read the frontmatter hook, direct/inline preparation, retained input layers, and task launch selection. | None of the required instances. The retry check verifies retention of the selected identity, as the ledger's reproduction requires; it does not claim a new guarantee for every pre-existing retry/provider-switch path. |
| R2-F1: Prompt composition retains an earlier launch identity | R2-F1.1–R2-F1.14 | verified fixed | All fourteen maintained identity cases pass in `sequence_task_providers::`, checking delivered prompt and child identity across dynamic inputs, changed documents, groups, external tasks, nested sequences, and static controls. | None. R3-F1's previously missed shell-dependent siblings are now fixed too. |
| R2-F2: Explicit selection flags do not override circular hints | R2-F2.1–R2-F2.10, including the repair ledger's provider-dependent environment case | verified fixed | Provider CLI precedence cells and all 17 review-selection unit cases pass. Provider/model precedence is applied separately; explicit flags and applicable captured model variables make circular lower hints irrelevant. The unknown-provider environment case retains its previously accepted rejection. | None. |
| R2-F3: Static errors in selection inputs are postponed until execution | R2-F3.1–R2-F3.7, including the runtime-created key | verified fixed | Fresh preflight and CLI runs pass static agent/model failures, indirect failed-input reads, late controls, zero-launch rejection, and the runtime-created-key case. Failed static expressions retain their causes instead of becoming launch-time values. | None. |
| R2-F4: Empty provider hints bypass the canonical fallback rule | R2-F4.1–R2-F4.10, using review 3's corrected authored-empty-list expectation | verified fixed | Null, absent, caller empty-list, installed-list, and authored empty-list controls pass in the provider, preflight, review-selection, and launch-schema runs. Authored empty agent lists fail early; null inherits. | None. The corrected expectation from review 3 stands. |

The following table records the full previous finding's child inventory. Paths name the Claudine package unless Darkmatter is specified. Test names are searchable in [the provider CLI tests](../../cli/tests/l1/sequence_task_providers.rs).

| Required instance | Verification evidence | Result |
| --- | --- | --- |
| R3-F1.1: direct document agent command | `identity_shell_document_agent`: Goose receives a Goose body and child environment; completion names Goose. | Passed |
| R3-F1.2: direct document model command | `identity_shell_document_model`: body, child `MODEL`, and model argument all name `fresh-model`. | Passed |
| R3-F1.3: indirect agent dependency | `identity_shell_indirect_document_agent`: a shell-produced choice read by `agent` reaches body and child. | Passed |
| R3-F1.4: concurrent group leg | `identity_shell_group_leg_agent`: the leg receives its own selected identity. | Passed |
| R3-F1.5: external task reference | `identity_shell_external_task_agent`: the referenced task uses the same corrected preparation. | Passed |
| R3-F1.6: inline nested sequence | `identity_shell_inline_nested_agent`: nested prompt body and child agree. | Passed |
| R3-F1.7: file-reference nested sequence | `identity_shell_file_nested_agent`: referenced sequence prompt body and child agree. | Passed |
| R3-F1.8: no enclosing fallback | `identity_shell_document_agent_without_fallback`: body identifies Goose instead of an empty agent. | Passed |
| R3-F1.9: literal command-looking task model | `identity_literal_command_looking_with_model` and the public library preparation case preserve the literal model as data in body and child. | Passed |
| R3-F1.10: literal command-looking task agent control | `identity_literal_command_looking_with_agent_fails_without_a_launch`: located unknown-provider error, zero launches. | Passed |
| R3-F1.11: explicit provider control | `identity_shell_document_agent_under_a_provider_flag`: body and child use Claude. | Passed |
| R3-F1.12: explicit model control | `identity_shell_document_model_under_an_explicit_model`: body and child use the explicit model. | Passed |
| R3-F1.13: shared preparation decision | Read [Claudine's `compose_selecting`](../../lib/src/composition/prepare.rs:729) and [Darkmatter's hook placement](../../../darkmatter/lib/src/markdown/compose/pipeline/mod.rs:483). Selection runs after frontmatter commands and before body stages; skipping the early read no longer skips this selection. Both single-key tests and a temporary both-key CLI probe pass. | Verified |
| R3-F1.14: child and completion consumers | Maintained cases assert child environment, provider arguments, and relevant root completion lines. The launch log remains the reporting source. | Passed |
| R3-F1.15: fourteen non-shell identity controls | Every R2-F1 identity control passes in the maintained CLI suite. | Passed |
| R3-F1.16: commands execute once | `identity_shell_selection_runs_each_command_once`: the trace is exactly `fm`, then `body`. | Passed |
| R3-F1.17: retained identity on retry/resume reread | `identity_shell_document_agent_on_retry` passes. Read [direct/inline preparation](../../lib/src/composition/prepare.rs:936): selected overrides replace `input_layers.env_overrides`, which the shared retry/resume preparation reads. | Verified |
| R3-F1.18: proxy from shell-agent prompt to static target | Temporary shipped-CLI probe launches Goose, then Claude; both delivered prompts and child environments match. | Passed |
| R3-F1.19: validation/body-only composition sites | Inspected `phase1c`, sequence probes, and body-only composition. Validation probes explicitly disable selection and execute no provider prompt; body-only sites retain their enclosing-document target. These sites do not bypass the new selector for referenced prompt tasks. | Verified |
| R3-F1.20: dry run with no planned provider | All 23 `sequence_sources_cli::` tests pass. Read [the task launch selector](../../cli/src/commands/wrap/sequence/task_run.rs:658): a non-dynamic task without an effective agent preserves its optional planned target. | Passed |
| R3-F1.21: composition ownership guard | `composition_seams::` passes, including the committed allowed-site inventory and preparation-context checks. | Passed |
| R3-F1.22: provider dispatch inventory | `dispatch_inventory::` passes against the committed inventory, without regeneration. | Passed |

Additional probes checked both preparation modes and the content consumers affected by the new environment hook: direct body, inline frontmatter prompt, `::file` transclusion, and `as_markdown` composition. All delivered the selected identity. The inline probe asserts delivery and child identity; it does not claim that its recording stub performs the separate inline file-edit contract. All five temporary probes used the existing isolated recording-provider fixture, completed in under two seconds each, and were removed with the original test file restored byte-for-byte.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No new recurrence. The recurring class from R2-F1 and R3-F1 is repaired for every required instance. No new sibling was found in the additional preparation-mode and nested-content checks.

## Input Robustness Matrix

Default outcome: invalid configuration shapes produce a source-located error and launch no affected target. Statically invalid selection inputs stop the sequence before any task launches. This review does not invent new rejection requirements for formats whose canonical rules intentionally treat null as no hint.

The shared `with_mapping_shape` validator handles task, proxy, retry, and prep mappings. Its exceptions are absence (no overlay) and `{}` (explicit empty overlay, equivalent to no overlay by the spec). Mapping values may contain typed data, including null and collections. Null, scalar, and list whole-field values fail; dynamic keys fail; whole-mapping interpolation receives its specific error before the generic mapping error. Fresh public sequence-parser and lifecycle-parser tests verify located errors and all four routes. The provenance CLI tests verify successful controls, schema-selected file handling, lazy/eager failures, forwarding, caller precedence, chained target schemas, and atomic evaluation failure.

Agent/model hints use the canonical scalar/list parsers. Exceptions are absence/null (no hint), an empty model list (no model), and a caller-supplied empty agent list (no hint). Authored empty agent lists are rejected before execution, preserving review 3's correction. Empty strings remain values rather than absence. Whole-field numbers/objects and non-string list elements fail rather than being silently filtered. Fresh preflight, review-selection, provider CLI, and launch-schema cases cover prediction and its gate, review projection, launch decision, and reporting consumers. Read the complete match arms in [the hint readers](../../lib/src/composition/hints.rs) to confirm that collection element errors cannot be discarded.

No new serialized-file loader was added by the repair: the frontmatter hook consumes the existing parsed document. Existing YAML loading and document-shape policies are unchanged. The sequence-trigger fixture suite checks absent/null activation, malformed non-null activation, typed container levels, and the deeper untyped boundary against the accepted schema decisions. This review does not claim a new duplicate-key or trailing-document test matrix for an unchanged loader.

Searched the changed readers for `serde(default)`, `Option`, `filter_map`, `unwrap_or_default`, and `.ok()`. The dependency scan's filtering does not parse hint-list elements. The early reader's suppressed errors fall through to full preparation, which owns located diagnostics; shell-bearing values now reach the later hook before body rendering. No additional invalid-shape defect resulted.

## Verification level and results

| User-facing requirement | Appropriate level | Fresh verification |
| --- | --- | --- |
| Task `with`, precedence, ordinary `seq.params` data, located shape errors | L1 | Public parser/preflight cases, CLI precedence cases, shared directive-shape tests, maintained-use search |
| File origin, forwarding, caller precedence, atomic overlays | L1 | All 15 provenance CLI cases pass |
| Independent task providers, early failure, provider-free graphs | L1 | Provider CLI and preflight cases pass, including zero-launch assertions |
| Dynamic selection, prompt/child identity, actual attempt reporting | L1 | All 68 provider CLI cases pass; public preparation tests and five additional CLI probes pass |
| Read-only predicted rows and submitted provider/model choices | L2 | All five review-screen tests pass in detached tmux; no focus change |
| Schema discovery/imports, bounded levels, accepted schema/parser differences | L1 | Sequence-schema library fixtures and launch-trigger CLI tests pass; embedded bundle wiring inspected |
| Maintained task-input migration | L1/static inspection | Remaining searched `params:` uses describe ordinary data or historical migration evidence |

No specified behavior in this feature requires OS keyboard injection. The review screen's specified L2 behavior is verified through the real terminal and event loop, rather than inferred from a render buffer.

Fresh commands and outcomes:

| Command | Outcome |
| --- | --- |
| `just test-cli 'sequence_task_providers:: sequence_sources_cli:: review::tests composition_seams:: dispatch_inventory::'` in `claudine/` | 139 passed |
| `just test-library 'preflight prepare:: composition::schema::tests sequence_schema task_with_shape_errors'` in `claudine/` | 489 passed |
| `just test-cli 'with_file_provenance:: launch_trigger_schemas::'` in `claudine/` | 44 passed |
| `just test-cli review4_probe` in `claudine/`, before removing temporary probes | 5 passed |
| `just test-library 'proxy_retry_and_prep_share_the_with_shape_errors task_with_shape_errors_are_the_with_family_at_the_task'` in `claudine/` | 2 passed |
| `just test-l2 sequence_review_screen` in `claudine/` | 5 passed; detached tmux |
| `just test 'frontmatter_hook nested:: frontmatter_shell'` in `darkmatter/` | 152 passed |
| `just check-tier-coverage claudine` at the repository root | 0 stranded tests |

The new CLI cases belong to the existing declared L1 module; the review-screen cases belong to the live L2 target. Recording-provider CLI cases are Unix-gated, while the library checks exercise the portable preparation API. These executions provide macOS evidence; cross-OS evidence remains CI's responsibility and is not a readiness blocker. Full L1 and lint were not repeated for this documentation-only review; the fresh targeted results above, rather than the repair log's full-suite claims, are the verification evidence reported here. The linker emitted its existing compact-unwind size warning; all reported test commands completed successfully.

## Observations

None.
