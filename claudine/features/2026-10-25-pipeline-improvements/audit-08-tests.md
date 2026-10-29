---
kind: implementation-log
---

# Test Audit: 2026-10-18-fm-namespace

Added **two test identities**: one genuinely new editor-coverage test and one
additional scheduling identity from splitting an existing two-run test. No
identities were removed, so gross and net additions are both **two**. Fixed
**one originally slow test**; no existing test needed stronger assertions.
No production behavior changed.

The final selected runs passed **215 distinct L1 tests** and **eight L2 tests**.
The maximum L1 duration was **2.461 seconds** and maximum L2 duration was
**2.837 seconds**. L1 completed before L2 began; both tiers used the recipes'
parallel execution on this macOS host.

## Coverage against the requirements

| Requirement | Evidence and boundary |
| --- | --- |
| `fm` snapshots, qualified/bracket reads, no fallback, reserved-name escape, overrides | `fm_namespace_contract` exercises public document composition; shared namespace-helper unit tests and the binding contract check the internal lookup semantics. |
| Frontmatter ordering and literal bracket dependencies | Public composition tests place dependents before templated inputs, distinguish a dotted key from a dotted traversal, and compare self-reference behavior with bare-name behavior. Later integrated cycle handling is preserved. |
| Shell-token property selection | The `fm.build` and `fm.echo` public composition case exercises the property branch rather than executable-first lookup. |
| Updated lifecycle state and snapshot independence | Claudine's lifecycle binding test writes `x`, checks bare and qualified reads, and verifies the earlier whole-object snapshot remains unchanged. Reserved roots and declared globals are checked against same-named frontmatter. |
| `doc` and `current_env` are ordinary data | Public composition checks ordinary nested values and undeclared-property advisories. The DMLS protocol tests cover `doc`, `hook`, `fm`, and now schema-only `current_env`. |
| Hook hard rename and unchanged payload shape | Dispatch expression, template, condition, and matcher tests exercise `hook` and bare payload reads, omitted environment, no qualified fallback, nested `extra.doc`, unprefixed fields, and rejected old-root behavior. |
| One environment per document, every consumer agrees | Six `invocation_context::document_tests` cover expression values, transclusion, file-reference expansion, approval, `SCHEMAS_DIR`, model selection, subprocess environment, stable invocation identity, and capture counts using supplied maps. |
| New documents capture; retries, resumes, loops retain their map | Real CLI `document_environment_routes` covers sequence steps, bounded/parallel group legs, proxy targets, transclusion, task shells, provider selection, retry, resume, and finalize. The fixture scripts child captures and does not mutate its parent process environment. |
| Editor roots, hover, diagnostics, and passive behavior | In-memory LSP tests exercise completion/hover/diagnostics through actual request routing. The new schema-only `current_env` test proves field completion, schema hover, absence of a built-in module entry or invented static value, and ordinary undeclared-warning identity. |
| Lazy `current` remains available after removal of `current_env` | Eight detached-tmux cases cover all seven events, fresh observations versus frozen `ctx`, frozen planning/formatting prompts, and negative controls for unknown lazy keys. |
| Migration and authored artifacts | Root-catalog and migration guards, documentation contract, frozen prompt compositions, and current schema/request fixtures retain the renamed expressions. Historical specs and ordinary property names are not rewritten. |

Expression lookup and supplied-map state transitions are transformation
contracts, so focused synthetic inputs (F1) are appropriate. Actual `md` and
`claudine` command invocations exercise their wiring. The LSP harness observes
the real protocol handlers. Detached tmux observes the real terminal run (F2),
and the changed Git branch/file state originates in real Git operations (F3
for those repository observations). Provider stubs select success/failure;
these tests make no claim about a live provider's behavior. No terminal or
browser window gains focus.

## Slow-test repair

The first parallel L2 run passed seven tests functionally, but
`level2_ac28_shipped_prompt_that_references_a_lazy_root_composes` took
**5.284 seconds**. It composed the successful planning prompt and its negative
control serially in one test. These are now independently scheduled tests,
retaining the real frozen prompt, real composition, error assertion, and
private-spool no-audio assertion for both cases. The shared tmux helper also
keeps the exact frame that satisfied readiness instead of sleeping 200 ms and
capturing again.

The final parallel run passed eight tests, including the new negative-control
identity. The positive case took **2.837 seconds**, the negative control
**1.823 seconds**, and the formatting case **2.468 seconds**. All original
functional cases remain; there is no retry, timeout increase, tier change,
reduced matrix, or runner-configuration change.

## Verification

From `darkmatter/`:

```sh
BISCUIT_TEST_FILTER='test(fm_namespace) | test(request_context_builder) | test(undeclared_property::current_env_completes) | test(undeclared_property::fm_) | test(undeclared_property::an_undeclared_doc_path) | test(compose_lazy_roots) | test(current_root_migration_guard) | test(current_root_documentation_contract) | test(catalog::roots::tests) | test(binding_contract)' just test
```

**70 passed**, maximum **0.509 seconds**. Log:
`/tmp/pipeline-s08-darkmatter-final.log`.

From `claudine/`, before L2:

```sh
BISCUIT_TEST_FILTER='test(document_environment_routes) | test(dispatch::expression::tests) | test(dispatch::template::tests) | test(dispatch::matcher::tests) | test(event_time_interpolation) | test(lifecycle::executor::tests::binding_contract)' just test
BISCUIT_TEST_FILTER='test(invocation_context::document_tests)' just test
```

**139 passed**, maximum **2.461 seconds**, then **six passed**, maximum
**0.513 seconds**. Logs: `/tmp/pipeline-s08-claudine-initial.log` and
`/tmp/pipeline-s08-document-consumers.log`.

After all L1 runs finished, from `claudine/`:

```sh
just test-l2 level2_ac28
```

**Eight passed**, maximum **2.837 seconds**, parallel self-spawn with eight
workers. The recipe's `claudine-gen` tail selected zero matching tests.
Log: `/tmp/pipeline-s08-l2-final.log`. The first run's measured slow identity
is retained in `/tmp/pipeline-s08-l2-initial.log`.

The new LSP test's first drafts assumed nested ordinary-property completion
and combined schema/static hover prose, neither of which is the existing
general editor contract. The final case deliberately uses an unset,
schema-declared property to prove schema completion and hover without those
unrelated assumptions. It passed through the real protocol harness.

`git diff --check` passed. The L2 build emitted the existing macOS linker
unwind-table size note. No full-area, live-provider, prompt-opt-in, or
cross-OS run is claimed by this slice; the coordinator owns final package
gates. Current internal prompts were not changed or made normal-suite inputs.
