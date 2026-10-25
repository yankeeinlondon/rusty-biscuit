---
kind: implementation-log
---

# Final additional test speed repairs

The initial final-area runs passed functionally: Darkmatter 12,391 tests,
Claudine 11,490, Messenger 773, and Biscuit Speaks 592. Their parallel timings
exposed additional over-budget tests that the smaller per-spec slices did not.
The initial logs are `/tmp/pipeline-final-{area}-l1.log`.

This coordinator-owned slice modifies nine observed slow identities:

| Original test | Before, seconds | Preserved coverage |
| --- | ---: | --- |
| `notify_send_stub::notice_only_timeout_maps_to_timeout_error` | 5.019 | Constructor's production deadline and real helper-process timeout/error mapping |
| `snoretoast_stub::notice_only_timeout_maps_to_timeout_error` | 5.015 | Constructor's production deadline and real helper-process timeout/error mapping |
| `terminal_notifier_stub::notice_only_timeout_maps_to_timeout_error` | 5.014 | Constructor's production deadline and real helper-process timeout/error mapping |
| `burnttoast_stub::notice_only_timeout_maps_to_timeout_error` | 10.015 | Constructor's production deadline and real helper-process timeout/error mapping |
| `executable_constraints_keep_their_researched_unit_and_stage` | 5.286 | All six researched unit/stage fixtures and exact projection assertions |
| `ambiguous_unknown_and_advisory_constraints_never_become_executable` | 5.738 | Ambiguous units, unknown stages, advisory constraints, all three knowledge states, unresolved conditions |
| `sequence_sources_cli::the_three_operators_each_produce_a_usable_name` | 6.740 | Actual CLI results for map, name, and template operators |
| `sequence_sources_cli::file_references_resolve_from_the_authoring_document` | 6.710 | Actual CLI resolution of relative, spaced, and at-sign paths, plus the captured-home request path |
| `sequence_task_providers::a_caller_agent_outranks_with_and_a_nested_sequence_agent_applies_to_its_tasks` | 5.988 | Both native three-task launch traces, with and without the caller override |

The four helper backends retain their production deadlines. The existing
internal helper configuration now carries the deadline, following the Alerter
pattern. Each timeout test first asserts the constructor's production value,
then supplies 100 ms to the same send path and checks the typed timeout.
Registration, send, dismissal, and interactive default behavior are unchanged.

Research cases and CLI matrices are independently scheduled. The knowledge-
state fixture is loaded once for its three related assertions instead of three
times. All original assertions remain; a new unit/stage roster guard checks the
six distinct fixture rows. Provider-precedence scenarios share their immutable
input fixture rather than duplicating it.

This slice adds **15 net identities**: one coverage guard and 14 scheduling
identities. Four existing timeout tests gain explicit production-default
assertions. The combined coordinator verification, final timings, and overall
accounting are recorded in `audit-status.md` after the final checks finish.
