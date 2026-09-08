# Phase 2 Gate Evidence

All commands ran from the named package area on macOS. Tests use nextest via
the repository `just` recipes. No write-mode formatter was run.

## Requirement-to-test execution

| Requirement | Targeted command or test | Result |
|---|---|---|
| Typed diagnostic discovery, restoration, serialization, rendering, and lifecycle routing | Claudine diagnostic library filters plus `diagnostic_discovery`, `effective_diagnostic_render`, `characterization_error_routes`, `error_guards`, and the shipped diagnostic corpus | 50 library tests and 33 CLI/integration tests passed |
| Biscuit File grammar, repository-first precedence, strict explicit-relative behavior, context capture, failures, and repeated persisted representation | Focused `reference_grammar`, `precedence_flip`, `detailed_resolution`, `resolution_context`, `completion_round_trip`, and CLI tests | 59 passed |
| Darkmatter shared reference context across compose/schema/reference/transclusion | `reference_integration` plus focused compose, schema, expression, reference, and transclusion tests | 39 integration and 132 focused tests passed |
| Sequence source variants, JIT behavior, tasks/groups, errors, and stream framing | `sequence_cli`, `sequence_sources_cli`, `sequence_jit`, `sequence_groups`, `sequence_errors_cli`, and focused library tests | 84 CLI and 249 library tests passed |
| Passive shipped-artifact corpus | `shipped_prompt_corpus_parses_frontmatter` | passed |
| Real shipped artifact through the normal CLI path | `shipped_implement_prompt_runs_real_router_target` | passed |
| Original file-resolution input and terminal-level dependent output | `level2_file_resolution_capture` cases for `prompts/_implement/implement-suggestions.md`, its explicit-relative counterpart, and ordered missing candidates | passed |
| Sequence task terminal framing | applicable `level2_sequence_task_stream_capture` cases | passed |
| Root package selection under macOS Bash 3.2 | Exact command `just test biscuit-test-harness` | 85 passed after the `just/devops.just` regression fix |

The exact behavior-to-test mapping, including native/quoted, present/missing,
boundary, malformed, downstream-state, and round-trip variants, is in
`phase2-test-map.md`.

## Broader gates

- `biscuit-file/ just test`: library 383 passed, 4 skipped; CLI 61 passed.
- `biscuit-file/ just lint`: passed.
- `darkmatter/ just test`: the complete L1 set passed as four deterministic
  `--partition hash:N/4` runs after the monolithic invocation crossed the
  non-interactive command ceiling.
- `darkmatter/ just lint`: passed.
- Repository root `just test biscuit-test-harness`: 85 passed.
- Repository root `just _lint biscuit-test-harness`: passed.
- `claudine/rendezvous/ just check`: passed.
- `claudine/rendezvous/ just test`: core 82 passed; daemon 168 passed and 2
  skipped; client 21 passed.
- `claudine/rendezvous/ just lint`: passed.
- `claudine/ just test`: the complete area L1 set passed as eight deterministic
  `--partition hash:N/8` runs after the monolithic invocation crossed the
  non-interactive command ceiling.
- `claudine/ just lint`: passed, including error and lifecycle documentation
  guards and all five Claudine workspace crates.

## Skips, retries, and incomplete optional evidence

- Biscuit File reported 4 pre-existing skipped tests.
- Rendezvous daemon reported 2 pre-existing skipped tests.
- One Claudine test exceeded its initial nextest slow timeout, then passed on
  retry; nextest reported it as flaky. No assertion failure remained.
- A combined L2 invocation passed 11 of 12 selected cases. The final idle-flush
  case was interrupted when the command crossed the session's approximately
  60-second non-interactive ceiling. All Phase 2 file-resolution cases and the
  applicable task-stream cases had already passed. L2 is not a mandatory broad
  Phase 2 completion gate, and the interrupted case is recorded rather than
  represented as passed.
- Initial cold-build attempts for Darkmatter, Rendezvous, and Claudine crossed
  the same ceiling without test failures. Their complete L1 sets subsequently
  passed through warmed or deterministic partitioned invocations as listed
  above.
- The first version of the new shipped-prompt end-to-end test asserted an
  internal log detail beyond the public routing contract. That interim test
  failed, the implementation was left unchanged, and the test was narrowed to
  the observable routed output before passing.

There were no known pre-existing assertion failures in the required L1 or lint
gates.
