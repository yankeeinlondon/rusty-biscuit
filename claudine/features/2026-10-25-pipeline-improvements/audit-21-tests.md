---
kind: implementation-log
---

# Test audit: 2026-10-31-type-aware-evaluation

Added **one genuine behavioral test function/Nextest identity** (gross and net
increase: one). No scheduling splits, existing test replacements, strengthened
tests, slow-test modifications, or production behavior changes were needed.

The final scoped runs passed **713 distinct L1 tests**: 540 in Darkmatter,
169 in Claudine, and four in Biscuit File. The maximum was **3.804 seconds**.
The Biscuit File recipe additionally ran its nine existing path-text unit
checks successfully; those observations are reported separately. The earlier
304-test discovery run and six-test host-diagnostics run overlap the final
Darkmatter slice and are not additional distinct identities.

## Added coverage

`darkmatter/dmls/tests/l1/host_function_diagnostics.rs` now verifies host-facts
changes throughout a single running editor session. Existing tests started a
fresh server for each valid, absent, stale, or incomplete artifact, so they
could not detect retained trusted or untrusted facts after a change.

The new test opens a document with a host catalog but no facts artifact, then
sends real LSP watched-file notifications while it creates the matching
artifact, changes the catalog to invalidate it, restores the matching catalog,
and deletes the artifact. At every publication it checks the exact set and
locations of incompatible-call, uncertain-call, and incomplete-analysis
diagnostics. This proves that trusted facts restore guarded file narrowing and
proven call errors, and that stale or absent facts withdraw those verdicts.
The document and server remain the same throughout. Synchronization uses the
server's completion barrier and published diagnostics, with no sleeps.

The focused new-test run took **0.351 seconds**. The test is portable: it uses
isolated workspace paths and file URLs, explicit notifications, and no shell,
network, terminal, browser, or installed host-function executable.

## Requirement coverage

| Criteria | Existing and added evidence |
| --- | --- |
| 1–6, 31–32: projection, layers, globals, indices, recursive paths | `variable_type_projection`, `recursive_call_check`, and Claudine's lifecycle call-check tests cover root unions, normalization, baseline/import/trigger/document layers, raw schemas, optional parents, constant versus dynamic indices, catalog globals, and bounded recursive lookup. |
| 7, 12, 22: passivity and state/privacy invariants | Instrumented projection/narrowing/shared-diagnostic tests assert no handler entry or network attempt, unchanged stored state and source bytes, and records without evaluated argument or environment values. Existing null-materialization tests separate normalization from analysis. |
| 8, 19: body/lifecycle/editor agreement | `variable_type_projection::lifecycle_calls`, `compose_lifecycle`, `shared_diagnostics`, `expression_call_diagnostics::shared_fields`, `compose_report_json`, and Claudine adapters compare located shared verdicts through library composition, CLI JSON, and LSP. |
| 9–11, 16–17, 33–34: narrowing and fact lifetime | `flow_sensitive_narrowing`, `expression_type_scope`, DMLS narrowing cases, host-function handler counters, and Claudine `scope_site_conformance` cover true-iff versus true-implies, short circuit/negation/contradiction, block guards, invalidating writes, event boundaries, lexical shadowing, and shell-hole typing. |
| 13–15, 35: passive file predicates | Biscuit File's structural corpus, `file_predicates`, and the real `md compose`/LSP cases cover accepted and malformed references, context-free classification, aliases, preserved existence outcomes, and always-false literal warnings. |
| 18, 21, 36: configuration and absence analysis | Expression diagnostic configuration checks, unknown-identifier tests, host nullable-parameter cases, binding tests, and ownership guards cover the absence of a file-specific setting, direct-null-admitting suppression, nested-call exceptions, and the shared analysis authority. |
| 20, 23–29, 38–39: policy, stopping, and reports | `policy_verbs` and `compose_report_json` cover precedence and conflicts, registry/completion mappings, removed flags, recovery without shell/host authorization, disabled checks, style severity, pre-effect stops, no rollback of prior effects, no failed document publication/viewer output, exact single-object JSON fields, failure classes, warning retention, and transcluded provenance. |
| 30: performance evidence | The existing implementation log contains the required representative composition and editor before/after observations and explanations. This audit measured current scoped test duration; it did not invent a new benchmark or rerun the historical comparison as a CI gate. |
| 37: host registration and artifact trust | `host_functions` covers collisions, facts validation, catalog/registration parity, deterministic artifact freshness, and zero handler entries during analysis. Fresh-session LSP cases cover each failure form; the added running-session test closes create/edit/restore/delete invalidation coverage. |

These are transformation and host-integration contracts. Direct public-API
assertions are sufficient for projection and narrowing; instrumented handlers
prove passivity. Real CLI invocations and the LSP protocol harness prove the
consumer wiring. No requirement depends on terminal input/rendering, browser
layout, live hardware, or an external provider, so **L2 is not applicable**.
No windows were opened. Earlier unrelated L2 runs are not counted as new
evidence for this slice.

## Verification

From `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(variable_type_projection) | test(flow_sensitive_narrowing) | test(file_predicates) | test(design_time_call_check) | test(expression_type_scope) | test(host_functions) | test(host_function_diagnostics) | test(expression_call_diagnostics) | test(call_check_agreement) | test(recursive_call_check) | test(compose_report_json) | test(policy_verbs) | test(report_diagnostics) | test(shared_diagnostics) | test(unknown_identifier_warning) | test(type_scope_ownership_guard) | test(call_binding) | test(function_binding_registry) | test(absence_scope) | test(lifecycle_passive) | test(compose_lifecycle)' just test
```

Passed **540**, maximum **3.804 seconds**. Log:
`/tmp/pipeline-s21-darkmatter-final.log`.

From `claudine/`:

```sh
BISCUIT_TEST_FILTER='test(call_check) | test(type_scope_ownership_guard) | test(host_catalog) | test(scope_site_conformance) | test(compose_removed_grammar) | test(compose_lifecycle_call_check)' just test
```

Passed **169**, maximum **1.482 seconds**. Log:
`/tmp/pipeline-s21-claudine.log`.

From `biscuit-file/`:

```sh
BISCUIT_TEST_FILTER='test(structural_validation)' just test
```

Passed **four Nextest identities**, maximum **0.011 seconds**, plus the
recipe's nine path-text unit checks. Log: `/tmp/pipeline-s21-file.log`.

Each test run used the canonical recipe's parallel L1 scheduler. Lint ran
after the test runs, with no competing build during final timing measurements.

From the repository root, exact changed-target lint passed:

```sh
cargo clippy -p dmls --test l1 --features effects-instrumentation -- -D warnings
```

Log: `/tmp/pipeline-s21-lint.log`. The new test took **0.362 seconds** in the
final expanded parallel run. `git diff --check` passed. No `cargo fmt`, commit,
or CI configuration change was made.

## Scope corrections and limits

One host-diagnostics invocation initially ran `just test` from the repository
root with only a test filter. That selected workspace compilation. Its owned
process group was interrupted with SIGINT (exit 130) before test execution;
the command was rerun from `darkmatter/`. No workspace-wide tests are claimed
or counted. The corrected targeted command passed all six host-diagnostics
tests in 0.372 seconds of execution.

The completed implementation's recorded decisions remain: raw recursive JSON
references stay conservatively unknown; guard refinement of a parent object
from its member discriminant is outside this feature; the preserved remote
existence behavior can return access/fetch errors. Neither `md` nor Claudine
registers a production host expression function, so registration and handler
passivity are proven by the public test host and separately running editor,
not by adding a production function solely for testing.

Local execution evidence is macOS. The new test adds no OS-specific code or
shell syntax. This slice makes no cross-OS execution claim; existing schema
and file-reference portable corpora remain exercised. The coordinator owns
the final affected-package gates after all serial audits finish.
