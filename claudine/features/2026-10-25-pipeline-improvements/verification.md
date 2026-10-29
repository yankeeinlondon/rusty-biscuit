---
kind: implementation-log
---

# Final verification

All 21 three-agent audits finished in pipeline order. This audit created no
commits, moved no specifications, and ran no formatter. External planning
commits advanced the shared branch during verification; their changes were
preserved, and the accounting here describes the audit's changes rather than
only the final uncommitted diff.

## Results

The four canonical area L1 recipes passed before the final speed repairs:

| Area | Passing L1 tests |
| --- | ---: |
| Darkmatter | 12,391 |
| Claudine | 11,490 |
| Messenger | 773 |
| Biscuit Speaks | 592 |
| Total | 25,246 |

Those full parallel runs exposed additional over-budget tests. After the
repairs, one combined parallel run of all affected modules, their guards,
and every previously slow module passed **1,269 tests**, maximum **4.751 s**,
with **zero tests over five seconds**. This includes the unchanged research
write/read/reread interaction and inline preparation chain. The first
combined attempt stopped at a branch-fixture assertion; restoring its real
initial commit fixed that setup mistake. No assertions were weakened.

Earlier relevant L2 runs used detached tmux and followed L1 execution. The
full Claudine tier ran 292 tests, all passing, maximum 4.997 s; the later
model-delivery change was followed by five passing review-screen L2 tests,
maximum 3.197 s. Per-spec reports record other targeted L2 and muted real
speech evidence. Timing and runtime evidence here are from this macOS host.

All four final canonical lint recipes passed: `just lint` in `messenger/`,
`darkmatter/`, `claudine/`, and `biscuit-speaks/`. This includes Darkmatter's
Zed WASM check and Claudine's transport and lifecycle-documentation guards.
Logs are `/tmp/pipeline-final-{area}-lint.log`. The final whitespace and
cross-spec canonical-reference, kind, and specification-body checks passed.

## Reproduce the final combined L1 scope

Run from the repository root. The command below is equivalent to the
coordinator's Python subprocess invocation: the same environment filter,
canonical recipe, package set, and package-qualified features.

```sh
BISCUIT_TEST_FILTER='test(invocation_context::) | test(claudine_schemas_embedded::) | test(compose_prompt_tiers::) | test(ctx_launch_anchor::) | test(ctx_per_run::) | test(document_environment_routes::) | test(sequence_magic_reference::) | test(sequence_overlay_pty::) | test(sequence_schema::) | test(research_sequence_artifacts::) | test(schema_completion::tests::shipped_container_schema) | test(clean_parity::) | test(agent_text_is_data::) | test(authored_text_rendering::) | test(inline_completion_lifecycle::) | test(provider_tail_launch::) | test(provider_tail_notice::) | test(contamination_probes::) | test(generate_ux::) | test(wrap_watchdog_startup_stall::) | test(call_check_agreement::) | test(expression_scopes::) | test(compose_initialize_staged_boot::) | test(compose_schema_cli::) | test(entry_point_parity::) | test(guarded_shell_values::) | test(launch_trigger_schemas::) | test(lifecycle_prep::) | test(shell_missing_values::) | test(sequence_sources_cli::) | test(sequence_task_providers::) | test(desktop_helpers::) | binary(research_validation)' \
just _test_local_all 'claudine; claudine-cli --features test-fixtures; claudine-gen; darkmatter-cli --features test-fixtures; dmls --features effects-instrumentation; messenger --features desktop,research'
```

The measured run is `/tmp/pipeline-final-speed-verified.log`. Initial area
runs are `/tmp/pipeline-final-{area}-l1.log`. Detailed repair accounting is
in the `audit-final-*-speed.md` reports.

## Scope limits

Passing tests do not turn previously recorded implementation departures into
implemented requirements. In particular, callable standalone groups, static
preparation-target approval inventory, and a shared total preparation deadline
remain documented limitations; the loops audit also records the author's
deferred review-loop rehearsal. The ownership audit records existing driver
departures, and conversion-ledger author review remains human-pending.
These are explicit in the corresponding numbered test reports and current
canonical documentation.

