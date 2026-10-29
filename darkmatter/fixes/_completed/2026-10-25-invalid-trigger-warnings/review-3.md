---
$schema: feature-review.yaml
ready: true
findings: []
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-25T20:32:28-07:00"
spec: 2026-10-25-invalid-trigger-warnings/spec.md
implemented: false
description: "A **fix** review of `2026-10-25-invalid-trigger-warnings/spec.md`"
fix: 2026-10-25-invalid-trigger-warnings/review-3.md
previous: 2026-10-25-invalid-trigger-warnings/review-2.md
---

# Review 3

The fix is **production ready**. Both findings open in the previous review
are verified fixed across their required instances, including the additional
instances identified by the repair's own sweep. No new finding or observation
remains, and no human decision is required.

This review read both earlier reviews and the repair's `### Finding Ledger`
in `log.md` before examining the implementation. Completion claims were
checked against source and independently executed tests. The earlier accepted
decisions remain settled: source-free JSON when no exact position exists,
passive shell inspection, and Claudine's existing discovery boundaries.
The repair's decisions to keep reference checking free of shell execution,
keep lifecycle initialization before preparation, and retain trigger warnings
from successful siblings require no reconfirmation. In particular, a warning
not discovered until preparation cannot stop an earlier initialization effect.

## Previous Finding Verification

The required instances include the child rows in both repair ledgers.
The second repair extends warning retention through R1-F1.21 and policy
enforcement through R2-F1.12. Earlier closed findings are included as
regression controls rather than reopened requirements.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: early composition exits discard discovered trigger warnings | R1-F1.1–R1-F1.21: inspection, lifecycle exits, approval success/failure, recursive failure, library reports, nested gates, reference refusal, rendering, and Claudine | **verified fixed** | Independent `invalid_trigger`, `preflight`, `nested`, and `reference` runs pass. Collector failure preserves successful siblings; approval results and failures retain findings; both CLI consumers deliver them. Instance evidence follows. | Nothing. The original approval-failure omission and the siblings found in review 2 are repaired. This is completion of the partial repair, not a newly found sibling. |
| R2-F1: preflight-only trigger warnings bypass explicit denial | R2-F1.1–R2-F1.12: approval policy, ordinary/lifecycle CLI, root and nested membership checks, effect controls, opt-out, and passive reference preparation | **verified fixed** | Public API tests check both denial selections, approval-handler counts, typed blocking errors, warning identity, and absent shell markers. CLI tests verify ordinary/JSON delivery and prevention of approved shell effects. | Nothing. The repair's additional nested-gate and reference-check instances are fixed too. |
| R1-F2: inspection and failed clean reports route warnings to the wrong stream | R1-F2.1–R1-F2.10, closed in review 2 | **verified fixed** | Inspection and clean regressions pass in `invalid_trigger`; validation, composition, Claudine, and DMLS controls also pass. Other commands retain their established discovery boundaries. | Nothing. |
| R1-F3: warning projections discard available source information | R1-F3.1–R1-F3.18, closed in review 2 | **verified fixed** | Positioned-envelope, payload/import, alias, embedded-source, symlink, and editor-related-information cases pass in `invalid_trigger` and `trigger_`. Source inspection confirms the exhaustive cause classification remains intact. | Nothing. Accepted location fallbacks remain unchanged. |

### Warning-retention instances

The library tests below are in Darkmatter's
[composition warning tests](../../lib/src/markdown/compose/tests/invalid_triggers.rs).
CLI tests are in darkmatter-cli's
[warning delivery tests](../../cli/tests/l1/invalid_trigger_warnings.rs).
Every cited test ran through the package-area nextest recipe.

| Instance | Verification evidence and result |
| --- | --- |
| R1-F1.1 | `compose_shell_report_keeps_the_warning`: successful inspection retains its normal report and one stderr warning. |
| R1-F1.2 | `compose_shell_report_failure_keeps_the_warning` and `the_warning_is_kept_when_the_authored_schema_fails_to_load`: the earlier root warning accompanies the schema failure. |
| R1-F1.3 | `compose_lifecycle_skipped_by_initialize_keeps_the_bootstrap_warning`: ordinary initialization skip delivers the warning once. |
| R1-F1.4 | The same test's JSON case retains a success report with a null document, one warning, and no stderr warning. |
| R1-F1.5 | `compose_approval_failure_keeps_the_root_warning`: the original unapproved-shell case now retains one warning and the approval error, in both output modes. Ordinary success and other failure controls also pass. |
| R1-F1.6 | `compose_lifecycle_past_initialize_warns_once`: continuing initialization preserves one warning through success and failure, in both modes. |
| R1-F1.7 | Public composition, effective-schema, preflight, authored-schema failure, and transclusion tests retain their warning records. The skipped-child and successful-sibling cases also pass. |
| R1-F1.8 | Claudine's `invalid_trigger` selection passes all 13 tests, including preliminary failure, launch, inline composition, loops, and sequences. |
| R1-F1.9 | Source search confirms Claudine does not enable trigger discovery on these composition/preflight requests or call the approval wrappers; preliminary schema checks continue to own discovery. |
| R1-F1.10 | `compose_lifecycle_call_rejection_keeps_the_warning`: rejection retains discovery findings in ordinary and JSON output. |
| R1-F1.11 | `compose_lifecycle_blocked_call_warning_keeps_the_warning`: a separately denied call warning does not discard the trigger warning. |
| R1-F1.12 | Root and child-only reference-refusal/recovery regressions pass, including lifecycle output. `reference_validation_reports_root_and_child_only_triggers_once` checks the public reference report directly. |
| R1-F1.13 | `compose_render_failure_keeps_the_warning`: failed rendering retains the warning, with and without lifecycle handling. |
| R1-F1.14 | Initialization-error and lifecycle-shell-preparation regressions retain bootstrap findings in both output modes. |
| R1-F1.15 | `a_failed_walk_keeps_an_earlier_siblings_trigger` checks root-before-child order. Both CLI sibling-failure tests retain the child-only finding through inspection and ordinary/lifecycle failures. |
| R1-F1.16 | Root/child approval-required and user-blacklist tests, built-in blacklist, handler denial, and unreadable-whitelist tests all retain findings. The handler test proves a denial actually occurred; the policy-file case checks the typed I/O error. All post-collection errors share the warning-recording boundary. |
| R1-F1.17 | `preflight_approvals_carry_a_skipped_childs_trigger`: successful approval returns the finding even though composition will skip the child. |
| R1-F1.18 | Ordinary CLI cases for skipped-child success, approval failure, and blacklist refusal retain one finding in ordinary and JSON output. |
| R1-F1.19 | Those cases also run with continuing initialization and no root trigger, proving lifecycle delivery independently of bootstrap discovery. |
| R1-F1.20 | `a_pre_approved_composition_reports_a_skipped_childs_trigger` retains the membership walk's warning. The active-child control reports it once despite rediscovery. |
| R1-F1.21 | `a_failed_nested_gate_keeps_a_child_only_trigger`: a frontmatter nested-composition gate rejects the unapproved command and retains its child-only warning on the public composition failure. |

### Policy-enforcement instances

These tests use the same library and CLI files linked above. Effect tests
assert a marker remains absent; a default-policy control proves the command
does execute when the warning is allowed.

| Instance | Verification evidence and result |
| --- | --- |
| R2-F1.1 | `a_denied_child_only_trigger_stops_the_approval_walk_before_any_prompt`: code/all denial returns the typed blocking error, retains the warning, and never invokes the approval handler. The policy check precedes policy-file loading. |
| R2-F1.2 | `compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell`: ordinary CLI stops for both denial selections and output modes; marker absent. Its default control runs the command and warns. |
| R2-F1.3 | The lifecycle counterpart passes the same checks with continuing initialization and a child-only trigger. |
| R2-F1.4 | `a_denied_child_only_trigger_stops_a_pre_approved_body_shell` and its frontmatter-shell counterpart stop before effects through the public API. Warning code, severity, and trigger source remain correct. |
| R2-F1.5 | Root code/all-denial API tests and the CLI denial regression stop before the first effect/document write. |
| R2-F1.6 | `compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell`: ordinary and lifecycle controls stop with the marker absent. |
| R2-F1.7 | Source contract verified: the frontmatter-only membership branch walks no body child; root warning enforcement remains in document preparation. No child-only warning can originate in that branch. |
| R2-F1.8 | Discovery-disabled tests emit no finding. `compose_shell_inspection_under_deny_warns_without_running_anything` preserves passive inspection: success, one warning, no effect. |
| R2-F1.9 | Claudine's unchanged discovery-disabled composition boundary excludes this value; preliminary checks retain their findings. Its invocation regressions pass. |
| R2-F1.10 | `a_denied_child_only_trigger_stops_the_nested_gate`: both denial selections prevent the nested command's marker before the frontmatter call can run it. |
| R2-F1.11 | `a_failed_nested_gate_keeps_a_child_only_trigger` verifies the nested failure handoff. Source inspection confirms failed nested reports contribute trigger warnings before the public failure drains them. |
| R2-F1.12 | The lifecycle effect regression passes. The CLI disables shell expansion during reference checking, and library graph preparation restricts itself to operations the request still enables. The 720-test `reference` selection passes after this change. |

The retained routing and provenance controls cover every earlier closed
instance: R1-F2.1–R1-F2.7 and R1-F3.1/.5/.12/.14 use CLI/public report
assertions; R1-F2.8 uses Claudine invocation tests; R1-F2.9 and
R1-F3.2–R1-F3.10/.18 use DMLS protocol/source tests; R1-F2.10 is the
other-command discovery-boundary check. Public discovery tests cover
R1-F3.2–R1-F3.4/.6–R1-F3.11/.13/.16/.17, including missing-file
observations and normalized self-path exclusion. Embedded discovery and
composition tests cover R1-F3.15 without creating local links or watches.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No finding recurs in this review. The warning-loss class from review 1 and
the preflight-policy class from review 2 are repaired, including the
additional instances identified during implementation. No accepted decision
has been reopened.

## Verification and coverage

Independently executed commands and results:

| Package area | Command | Result |
| --- | --- | --- |
| Darkmatter | `just test invalid_trigger` | 84 passed |
| Darkmatter | `just test trigger_` | 373 passed |
| Darkmatter | `just test preflight` | 159 passed |
| Darkmatter | `just test nested` | 357 passed |
| Darkmatter | `just test reference` | 720 passed |
| Darkmatter | `just test shipped_schema` | 14 passed |
| Claudine | `just test invalid_trigger` | 13 passed |
| Repository root | `just check-tier-coverage darkmatter` | Zero stranded tests |
| Repository root | `just check-tier-coverage claudine` | Zero stranded tests |

Selections overlap; these counts are not a unique-test total. No selected
test exceeded the five-second L1 budget. The repair's full-suite, lint, and
cross-platform statements remain implementer evidence; they are not claimed
as independently repeated here.

A temporary CLI probe reused the existing isolated workspace fixture and
added a root trigger with a different filename from the child's trigger.
It checked first-observed root-before-child warning order in ordinary/JSON
output, ordinary/lifecycle composition, and active/skipped-child cases.
All eight invocations passed. The probe was removed and the original test
file was restored byte for byte. No production source was changed for this
review, and no terminal or browser window was opened or focused.

The changed tests are compiled by declared library or consolidated L1
targets, their modules contain no higher-tier naming marker, and recipe
features enable them. Their execution above proves selection as well as
compilation. Fixtures use temporary repositories and fake providers rather
than the author's incident files or live prompts.

| Spec acceptance requirement | Appropriate verification level and evidence | Assessment |
| --- | --- | --- |
| 1: incident recovery | L1 normal CLI composition of the isolated incident fixture | Passing |
| 2: valid neighbors, ordering, and enforcement | L1 public discovery/validation and CLI assignment controls | Passing |
| 3: invalid envelopes, payloads, imports, cycles, compilation, and strict parsing | L1 public trigger matrices, payload-load tests, and typed-cause/source assertions | Passing |
| 4: all-invalid discovery, matcher-only rules, ignored YAML, shadowing, and infrastructure | L1 disk/embedded discovery and registry tests | Passing |
| 5: invocation identity/order, cached findings, and repair/create/delete refresh | L1 public refresh tests, editor sessions, deduplication controls, and the independent overlapping-pass probe | Passing |
| 6: consuming commands, Claudine launch, reports, and independent failures | L1 CLI/fake-provider tests and DMLS protocol sessions; previous retention defects now pass | Passing |
| 7: explicit schema enforcement and shipped artifacts | L1 public/CLI failure controls and the 14-test shipped-schema selection | Passing |
| 8: editor registry replacement and source diagnostics | L1 registry and DMLS protocol tests covering completion, hover, scopes, refresh, strict authoring, and related sources | Passing |
| 9: denial before the next effect and discovery opt-out | L1 public effect/approval counters and API/CLI marker tests, including skipped-child and nested gates | Passing |
| 10: clean and editor formatting | L1 CLI ordinary/JSON success/failure assertions and editor formatting tests | Passing |

L1 is the appropriate level for these requirements: they concern returned
schema results, warning records, output streams, editor protocol messages,
and prevention of effects. None requires terminal input encoding, keyboard
events, or rendered geometry. There is no L2/L3 verification mismatch.

### Input robustness

This repair changes warning transport and policy decisions, not the trigger
grammar or a configuration loader. The existing public
[trigger input matrix](../../lib/tests/l1/trigger_input_matrix.rs) and
[embedded-envelope matrices](../../lib/tests/l1/schema_embedded_trigger_bundles.rs)
ran in the `trigger_` selection. They use positive controls and source edits
to check the outcome through discovery and effective schema application.

The default outcome for an invalid classified trigger remains one finding
and exclusion of its entire payload; infrastructure failures remain errors.
The explicit exceptions remain: absent or non-trigger string `kind` is
ignored, absent payload is matcher-only, an empty schema mapping is an
explicit empty payload, and omitted optional conditions remove that
predicate. Null, wrong field types, mixed/all-invalid collections, and
invalid empty conditions do not silently turn into missing values.

Duplicate keys, trailing invalid YAML, and embedded block-YAML/JSON syntax
use the existing classifier and loader tests; malformed unclaimed content
retains its accepted ignored outcome. The generated-cell guard remains
declared and selected. The smell search over discovery, assembly, and
finding projection found no new field default, element-dropping validator,
or parse failure converted into valid empty configuration. Assembly's
directory-listing `.ok()` causes a cache miss before strict loading, rather
than accepting invalid input; its tally default and path/name filtering are
not field validators.

## Observations

None.
