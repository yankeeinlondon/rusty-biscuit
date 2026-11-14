---
$schema: feature-review.yaml
ready: false
findings:
  - 'R2-F1 (high): Required defaults are skipped beneath pattern keys'
  - 'R2-F2 (high): Initialized composition can panic when performance reporting is enabled'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-14T06:48:52-08:00
spec: 2026-11-12-registration/spec.md
implemented: true
description: 'A **feature** review of `2026-11-12-registration/spec.md`'
feature: 2026-11-12-registration/review-2.md
previous: 2026-11-12-registration/review-1.md
next: 2026-11-12-registration/review-3.md
log: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/log.md
implemented_by: claude/opus
---

# Registration review, iteration 2

The feature is not production ready. All eight findings from review 1 are verified fixed, including their required instances and controls. Two additional defects remain: required defaults do not work beneath pattern keys, and a preparation timer can make ordinary initialized composition panic with `--perf`. Both have repairs within the existing contract; neither requires an author decision.

## Previous Finding Verification

I read review 1 and the repair log's Finding Ledger before checking the implementation. There are no earlier reviews or human rulings in this feature directory. The ranges below account for every child instance in that ledger; completion labels were not used as proof.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Foreign composition discards host-event schema errors | R1-F1.1–R1-F1.5: foreign `start`, `blocked`, `success`, `failure`, and Claudine `loop`; R1-F1.6–R1-F1.11: shared events, rejected md spellings, numeric timeout | verified fixed | Read the selected-host branch in [Darkmatter's composition verdict](../../lib/src/markdown/compose/schema_validation.rs:675) and its shared pending-value filter. Ran `compose_foreign_documents`, `foreign_document_compose`, and Claudine `installed_registration`, including the actual published-profile loop case. Wrong shapes reject, shared events retain md's parser, and numeric timeout conversion remains accepted. | None of the original instances. |
| R1-F2 — Sequence-only Claudine documents miss the main host schema | R1-F2.1–R1-F2.9: all nine identifying keys; R1-F2.10–R1-F2.11: sequence/agent renamed-event checks; R1-F2.12: shared-only control | verified fixed | Read [Claudine's main selector](../../../claudine/schemas/claudine.yaml), which now includes non-null `sequence`. Ran the published registration tests for every identifying key through passive validation and composition, the renamed-event controls, shared-only selection, and `claudine_schema_bundle`. The selection guard compares the identifying and main selectors directly. | None. |
| R1-F3 — Claudine leaves configured URL transclusion disabled | R1-F3.1–R1-F3.4: file/code/prologue/epilogue; R1-F3.5–R1-F3.6: expression/md controls; R1-F3.7: inline, loop, sequence, preflight builders | verified fixed | Read [Claudine's common options builder](../../../claudine/lib/src/composition/mod.rs) and its callers in preparation, wrapper discovery, sequence discovery, and system prompts. Remote transclusion is enabled before the captured network policy decides each read. Ran all `network_transclusion` cases, including zero-request denial, redirects, and shell discovery, plus Darkmatter's network tests and the shared permission tests. | None. The deliberate library operation switch is preserved. |
| R1-F4 — Per-run host approvals bypass invalid permissions | R1-F4.1–R1-F4.4: five invalid network shapes across four URL surfaces; R1-F4.5–R1-F4.9: valid policies, field/YAML loaders, local slots, Claudine controls; R1-F4.10–R1-F4.14: invalid whole block, list entry, shell rule, missing reference, omitted block | verified fixed | Read [the remote gate](../../lib/src/markdown/compose/remote_fetch.rs:568), [the slot gate](../../lib/src/markdown/schemas/slots/grants/gate.rs:134), and `NetworkPermission::fetch_policy`: invalid configuration now precedes run grants and projects deny-all. Ran `network_permission`, `config_notices`, `config_fields`, `config_load`, permission/reference tests, and Claudine parity cases. The public-result matrix asserts located failures and zero requests; valid deny plus exact-host consent still fetches. | None. |
| R1-F5 — DMLS ignores template shell approvals | R1-F5.1–R1-F5.5: template/literal, exact, prefix, exclusion decisions; R1-F5.6–R1-F5.7: md execution and Claudine parity | verified fixed | Read [DMLS's passive verdict](../../dmls/src/overlay/shell.rs:129) and directive parsing. They use the shared authored-template parser, type projection, coverage, and exclusion checks. Ran `shell_permission_classification`, including real in-process LSP hover decisions for the shared fixture, and the md/Claudine permission cases. Literal commands do not gain template approval, and both literal and shape exclusions remove pre-approval. | None. Conservative unknown results for values the editor does not evaluate are reasonable and do not require a ruling. |
| R1-F6 — Union validation skips conflicting default candidates | R1-F6.1–R1-F6.4: strict/soft root and nested alternatives; R1-F6.5–R1-F6.7: empty, identical, discriminated candidates; R1-F6.8: Darkmatter projection | verified fixed | Read `Prepared`'s acceptance shortcut, `prepare_defaults`, nested/root settlement, and `EffectiveSchema::converted_candidate`. Ran all 41 `required_defaults` tests and the CLI root/nested ambiguity regression. An unchanged candidate now competes with successful completed candidates. The repair correctly clarifies that the original closed nested example has only one successful candidate; the new open-arm case exercises the actual nested ambiguity. | None of the arbitration defect. R2-F1 is a different traversal omission, not a partial repair of this finding. |
| R1-F7 — Recorded Windows source-scan timings fail the L1 speed gate | R1-F7.1–R1-F7.2: production scan; R1-F7.3: bounded fixture; R1-F7.4: glob guard | verified fixed | Read the single-threaded [migration guard and Git prefilter](../../lib/tests/l1/current_root_migration_guard.rs:105), including fallback scanning and modified/untracked/index-flag handling. Its equivalence fixture passes. This review measured the production scan at 0.532 s, the bounded fixture at 0.076 s, the prefilter fixture at 0.172 s, and the related glob scan at 1.663 s within the targeted run. The repair also records Windows filtered evidence of 1.085 s for the production scan. | No outstanding speed failure in these instances. This review does not claim a new full Windows package run; cross-OS confirmation remains with CI. |
| R1-F8 — Eager default failures lose their schema-default origin | R1-F8.1–R1-F8.4: strict, soft, md validate, md compose; R1-F8.5–R1-F8.7: existing eager target, lazy target, invalid numeric default | verified fixed | Read `AppliedDefault`, `attribute_problems`, effective-schema provenance, both composition passes, formatted hints, output schemas, and DMLS projection. Ran the eager-file strict/soft and composition controls, CLI schema-validation cases, and DMLS `required_defaults`. Errors identify the default and its schema pointer instead of inventing an authored value position. Imported-type and post-shell-trigger cases are covered; the latter passed in this review's run. | None of the original instances. |

The repair log also contains discoveries that were not findings in review 1. Its test-feature correction is wired into a declared test binary and a CI-enabled feature. The three recorded 5–10 s cases include the required loaded-host measurements and isolated results below five seconds. Its remaining timing panic, called `X3` in the repair ledger, was never repaired: it is now R2-F2 below. A successful rerun did not clear it.

## Unblocked Findings

### R2-F1 (high): Required defaults are skipped beneath pattern keys

**Authority — spec, section 3.2:** “A `required` property whose definition has `default(...)` satisfies its contract when absent,” including “inside an object … only if the object exists.” Section 3.3 requires composition to apply those defaults before downstream reads. An existing dictionary entry is such an object; the spec makes no exception for entries selected by pattern keys.

**Defect class:** the required-default walker skips supported object-value containers, so the validator rejects values whose missing children have valid defaults.

In the Simplified Schema package, [`Walk::object`](../../../simplified-schema/lib/src/defaults.rs:357) visits literal properties but never visits `patternProperties`. It also disables **all** `additionalProperties` traversal whenever any pattern exists, including for keys that match no pattern. This is reachable with ordinary typed dictionaries, such as per-job settings selected by an `x-` prefix.

For example, save this schema as `defaults.yaml`:

```yaml
$schema:
  "<starting::x->": "{ retries: number(required; default(3)) }"
```

Then compose or validate:

```markdown
---
$schema: ./defaults.yaml
x-build: {}
---
Retries {{ x-build.retries }}
```

Both `md schema validate doc.md --output json` and `md compose doc.md --output json` exit 1 with `"retries" is a required property` under `/x-build`. Writing `retries: 3` explicitly makes both succeed and composition returns `Retries 3`. Replacing only the selector with literal `x-build` or catch-all `<string>` also makes the omitted-child case succeed. These controls establish that the default is valid and load-bearing.

The class sweep kept the child contract and input constant while changing the enclosing container. An in-process probe in the existing `required_defaults` test module exercised the public `SimplifiedSchema::validate_strict` and `validate_soft` results. The temporary probe and editor probe were removed after collecting evidence; they are not claimed as permanent regression coverage.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Shared object walker, prefix pattern | Existing `x-build: {}` under `<starting::x->` | Strict and soft reject missing `retries` | Strict accepts and reports default; soft inserts `3` |
| R2-F1.2 | Shared object walker, suffix pattern | Same input under `<ending::-build>` | Both reject | Same default behavior |
| R2-F1.3 | Shared object walker, regex pattern | Same input under `<pattern::^x-.*$>` | Both reject | Same default behavior |
| R2-F1.4 | Catch-all alongside a pattern | Same input under catch-all plus unrelated `^other-` pattern | Both reject; even the nonmatching pattern suppresses traversal | Use catch-all and supply default |
| R2-F1.5 | Literal property control | Same input under literal `x-build` | Both accept; strict reports default and soft inserts it | Same; clean |
| R2-F1.6 | Catch-all without patterns | Same input under `<string>` | Both accept with default | Same; clean |
| R2-F1.7 | Literal property with unrelated pattern | Same literal child plus `^other-` pattern | Both accept with default | Same; clean |
| R2-F1.8 | Array-item traversal | `entries: [{}]`, same child schema in `items` | Both accept; default path `entries.0.retries` | Same; clean |
| R2-F1.9 | Tuple-item traversal | Same array, child in `prefixItems` | Both accept with default | Same; clean |
| R2-F1.10 | Local reference traversal | Same child supplied through `$defs`/`$ref` | Both accept with default | Same; clean |
| R2-F1.11 | Conjunctive traversal | Same child beneath `allOf` | Both accept with default | Same; clean |
| R2-F1.12 | Darkmatter passive validation | Prefix/suffix/regex Markdown fixtures above | CLI exits 1, missing child | Accept and report applied default |
| R2-F1.13 | Darkmatter composition | Same fixtures and body read | CLI exits 1, no document | Compose `Retries 3` |
| R2-F1.14 | DMLS schema diagnostics | Prefix fixture through an in-process language-server session | Publishes `dm.schema.missing_required` for `/x-build` | No missing-child diagnostic |

Root/nested union settlement and recursive-reference controls were also checked in the passing required-default suite; their arbitration is covered in the R1-F6 verification above. `Prepared`, Darkmatter's effective-schema completion, composition, and editor validation all delegate to this shared walker, so the fix belongs there.

**Recommended repair:** visit each existing key's applicable pattern schemas, and apply `additionalProperties` only when neither a literal property nor a pattern covers the key. Preserve the existing literal-key precedence and regex semantics; the package already has pattern matching support, including [`KeyMatcher`](../../../simplified-schema/lib/src/coercion/shape.rs:375). No new schema syntax or separate default engine is needed. Add public strict/soft regressions for the affected selectors and mixed catch-all case, plus one CLI/editor wiring case. Include the explicit-value and literal/catch-all controls.

### R2-F2 (high): Initialized composition can panic when performance reporting is enabled

**Authority — defect:** the existing Claudine `wrap_perf::composition_setup_and_provider_handoff_order_matches_phase_1_baseline` fixture already produced this panic in the repair's recorded run. Inspection and a fresh reproduction confirm that a normal initialized document with a modest body-composition cost crashes with `--perf`. A diagnostic flag must not turn successful composition into a panic.

**Defect class:** a child timing is assigned to a parent window that ended before that work started, breaking the performance report's duration invariant.

In the Claudine CLI package, [the initialized route records `prep_phase`](../../../claudine/cli/src/commands/compose/prep.rs:1738) before calling `run_staged_single`. That helper runs initialization, re-reads the document, and performs the actual preparation [afterward](../../../claudine/cli/src/commands/compose/prep.rs:2027). The report then places this later `compose_perf.total` inside the earlier preparation window as a structural child in [`build_perf_tree`](../../../claudine/cli/src/perf/tree.rs:67). When the child is sufficiently large, [`debug_assert_reconciles`](../../../claudine/cli/src/perf/tree.rs:444) panics. Normal scheduling can expose it on the existing tiny fixture; it is not merely an arbitrary timeout in a test.

I reused `CliProcessFixture`, seeded its user configuration, installed a local fake `goose` executable, and allowed only `exact sleep 0.1`. No real agent was launched. The direct document was:

```markdown
---
agent: goose
prompt: Hi
initialize:
  - {stdout: initialized}
---
# Body

::shell sleep 0.1
```

`claudine compose --goose --perf doc.md` exited **101**, after the fake provider completed, with `perf report failed TR-1 reconciliation` at `cli/src/perf/tree.rs:445`. The final reproduction reported headline `773.120042ms`. Removing only `--perf` succeeded. Removing only `initialize` also succeeded and rendered the report. For inline composition, moving the same shell directive into the Markdown `prompt` reproduced the panic with a fake provider that correctly updates the document.

The sweep followed both initialized callers of the shared single-document helper and both report collectors; it included the ordinary routes and sequence placement as controls.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F2.1 | Direct initialized preparation → single-document report | Initialized body with 0.1 s command, `compose --perf` | Exit 101, reconciliation panic | Exit 0 with consistent report |
| R2-F2.2 | Inline initialized preparation → same report | Initialized prompt with same command, `inline-compose --perf`; fake provider updates body | Exit 101, same panic; headline `528.109917ms` | Exit 0 with consistent report |
| R2-F2.3 | Direct ordinary preparation | Same body without `initialize`, `--perf` | Exit 0 and report | Same; clean |
| R2-F2.4 | Inline ordinary preparation | Same prompt without `initialize`, `--perf` | Exit 0 and report | Same; clean |
| R2-F2.5 | Initialized route without performance report | Original direct document, no `--perf` | Exit 0 | Same; clean |
| R2-F2.6 | Sequence preparation → sequence report | One initialized inline step, same prompt command, updating fake provider, `sequence --perf` | Exit 0 and aggregate report | Same; clean: composition is placed under its step, inside that step's wall-clock window |
| R2-F2.7 | Wrapper-only report construction | `CommandPerfCollector::new` invariant | It carries no composition child, so this misplaced child cannot arise; existing wrapper/report tests pass | Clean for this class |

Loop execution returns through its own iteration engine before this single-document timing branch; it does not supply this report's composition child. The two affected entry points share the same faulty measurement boundary, so they should be repaired together.

**Recommended repair:** keep the preparation measurement open through the initialized re-read and actual composition, stopping it before provider setup, as the ordinary route already does. Preserve the report invariant rather than increasing its tolerance or removing the assertion. Add a bounded regression for direct and inline initialized preparation whose composition duration reliably exceeds the preliminary window; a deterministic timing seam is also reasonable. The existing ordering test can remain as an independent control. The defect is present in debug/test builds; release builds omit the assertion but still receive the incorrectly attributed timing.

## Blocked Findings

None. Both findings have a recommended repair within the specification and existing APIs. No human ruling is required.

## Recurrence

Neither finding repeats a defect class from review 1. R2-F1 concerns missing traversal, whereas R1-F6 concerned arbitration among candidates that were traversed. R2-F2 is the repair ledger's unresolved timing panic, not a recurrence of R1-F7's slow repository scan. The earlier review did not list that panic as a finding. The original eight findings are not being reopened.

## Input robustness and verification levels

The feature adds configuration and manifest readers. Their default invalid-input outcome is a located error without authorization or effects. Exceptions remain: absent configuration/settings use schema defaults; explicit empty policy lists replace their defaults; a malformed registration produces a diagnostic and omits that registration while independent valid permissions remain usable.

I checked the shared admitted-YAML loader and public `config_fields`, `config_load`, reference, registration, migration, and writer cases. They cover absent versus null, whole-field and element type errors, explicit empty values, duplicate keys, malformed/trailing YAML, and multiple documents. The malformed-network matrix exercises all four fetch surfaces after parsing, including exact-host grants and zero-request assertions. The load-bearing parse smell check found no new permissive fallback: filtered iteration is downstream of validation, registration recovery retains diagnostics, and Claudine's serde defaults remain behind its schema check. R2-F1 concerns schema-driven default application, not a configuration loader silently accepting an invalid shape; the field-format matrix is not applied to the Markdown or schema grammar.

| Requirement | Appropriate level | Evidence and result |
|---|---|---|
| Configuration shapes, scope/default rules, committed-policy intersection, references, migration, preserving writes | L1 public API and isolated CLI | Targeted configuration/permission suites pass; reader and writer paths inspected |
| Shell rules, exclusions, blacklist, templates/chains, URL consent and redirects | L1 public API, loopback server, CLI and LSP protocol | Targeted parity/network/editor suites pass; original R1-F3–R1-F5 instances fixed |
| Strict/soft defaults, nesting, unions, composition ordering, prompting, editor presence | L1 public API plus normal CLI/LSP paths | Existing default suites pass; new pattern-container probes fail as R2-F1 describes |
| Host identification, registration publication, layering, unavailable runtime values, shared-only events, editor invalidation | L1 public API and CLI/LSP protocol | Registration/foreign-document suites pass; actual published Claudine profile checked |
| Performance reporting preserves successful command execution | L1 isolated CLI with local fake provider | Existing performance tests pass on this run, but initialized direct/inline reproductions panic: R2-F2 |
| Real terminal rendering or physical keyboard behavior | No new such requirement in this spec | No L2/L3 mismatch identified. Passive configuration/editor semantics do not need focus or OS keyboard injection. |

The changed test modules are declared by their consolidated test binaries. The fixture-dependent Claudine tests use the enabled `test-fixtures` feature; no new stranded tier was found. This review leaves no temporary test additions or production changes.

## Verification performed

- `darkmatter/`: `just test registration config_ permission required_defaults composition_defaults foreign_document schema_validate current_root_migration_guard glob_implementation_guard claudine_schema_bundle` — **688 passed**, no test at or above five seconds.
- `simplified-schema/`: `just test required_defaults` — **41 passed**, then the recipe's full minimal build **1,828 passed**; no slow tests observed.
- `claudine/`: `just test-library installed_registration network_transclusion shell_permission_parity` — **40 passed**, no test at or above five seconds.
- `claudine/`: `just test-cli wrap_perf perf::` — **60 passed**, no test at or above five seconds. This passing run does not supersede the deterministic initialized-route reproductions.
- After restoring the temporary probes, the affected Darkmatter modules passed **100/100** and the Claudine performance modules passed **60/60** again, with no test at or above five seconds.
- Temporary public-API probes checked all eleven default-container rows; strict/soft failed exactly for prefix, suffix, regex, and catch-all with a pattern. A temporary LSP probe confirmed the editor's wrong missing-field result. The CLI controls reproduced the same affected selectors with explicit values and literal/catch-all alternatives.
- A temporary isolated CLI probe checked the six executable performance-report rows above in 3.75 seconds total. Its harness completed successfully while recording the two child exit-101 failures; that harness result is not a passing verdict for those commands.

Existing tests were restored byte-for-byte after probing, and the affected existing modules were rerun afterward. No formatting, commit, CI dispatch, host publication, terminal window, or real agent launch was part of this review.

## Implementation record corrections

The repair log's `X3` disposition and completion summary called the timing panic clean because it passed on rerun. I corrected those statements in place and added a dated explanation of the actual preparation-window defect and the direct/inline CLI reproductions. Historical passing test totals remain intact: they describe those runs accurately, but do not prove this defect was repaired. The correction is a record correction, not an additional finding or human-review item.
