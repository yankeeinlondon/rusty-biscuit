# Disposable Steering Verification

The first pass uses a real Pi RPC subprocess with a deterministic model registered
by a test extension. The model exposes the input it receives through allowlisted
fixture markers. This tests Pi's actual command, queue, tool, extension, and event
machinery without cloud inference or changes to existing sessions.

## Pi 0.84.4 on macOS

[Recorded result](pi-macos-0.84.4.json): one combined opt-in test passed, producing
two scoped research verification records for active steering and idle prompting.
The final nextest run was `93f3ef43-26ef-40d5-8bf1-f34653f85571`.

- Steering was acknowledged while a tool remained held; both tool calls completed
  before steering reached the next model request.
- Delivery and later idle prompts retained the same session ID.
- Reusing a request ID delivered duplicate text twice. Correlation is not deduplication.
- Context, skill, and template sentinels reached the model through normal loading.
- An extension command returned success without invoking the model.
- `new_session` changed the current conversation within the same child.
- An extension confirmation remained pending while the controller answered nothing.
- `agent_settled` was observed after the tested runs. Retry/compaction settlement
  ordering was not exercised.

The fixture project has its own Pi configuration and session directory and trusts
only its explicitly created project resources. It does not pass `--no-extensions`,
`--no-skills`, `--no-prompt-templates`, or `--no-context-files`. The Rust owner kills
and reaps its child and joins its pipe readers when the test finishes or panics.
Diagnostics are drained but not retained; the saved event projection excludes
message content, system prompts, environment values, paths, and credentials.

## Repeat the test

Use `sniff software agents --json` to identify the intended Pi binary, then run
from the repository root with the binary's absolute path:

```sh
CLAUDINE_PI_BINARY=/absolute/path/to/pi \
CLAUDINE_PI_REPORT=/tmp/pi-steering-result.json \
cargo nextest run -p claudine-cli --features real-tests \
  --test real_pi_steering --run-ignored all --no-fail-fast
```

On PowerShell, set `$env:CLAUDINE_PI_BINARY` and `$env:CLAUDINE_PI_REPORT` before
running the same nextest arguments. The test is ignored unless explicitly selected;
missing binaries or required responses fail instead of being counted as verified.
The native Windows and Linux paths have not been executed in this pass.

Implementation: [Rust harness](../../../cli/tests/real_pi_steering.rs) and
[Pi extension fixture](../../../cli/tests/fixtures/steering/pi-probe.ts).

## Next experiments

1. Bounded generation stalls and sequential tool batches in addition to this batch.
2. Additional subprocess descendants, detached tools, and post-acceptance replacement failure beyond the built-in bash experiment below.
3. Full-duplex loss, controller/host failure, restart, and broader concurrent scheduling beyond the controlled switch/provider-kill cases below.
4. Real cloud-model execution and representative configured user extensions, without
   weakening their resource or approval policy to make tests pass.
5. The same fixtures on native Linux and Windows with exact version records.

These results enable targeted design decisions, not a production adapter. They
do not establish an ordinary-session attachment endpoint, race-safe targeting,
crash persistence, safe retry, or compatibility with arbitrary user profiles.

## Abort and stdin EOF follow-up

[Saved failure results](pi-macos-0.84.4-failures.json), nextest run
`1f9371f1-5dae-4ef9-88e5-b892f9b39151`: both opt-in tests passed, zero skipped.
The new test covers four scenarios with the same real Pi/local-model fixture:

| Scenario | Steering in saved transcript | Model consumption | Outcome |
| --- | --- | --- | --- |
| Abort with queued steering | One user message | On an explicit next prompt, not during abort | Tools canceled; session retained |
| Clear queue, then abort | None | None on the next prompt | Pending text returned by clear operation |
| Close stdin without waiting for acknowledgment | None | None | Exit 0; acknowledgment observed during later output drain |
| Close stdin after acknowledgment | None | None | Exit 0 despite undelivered acknowledged text |

Both abort scenarios also reject a malformed replacement independently, then
accept a valid explicit next prompt. This proves pre-acceptance rejection and
cooperative tool cancellation, not cleanup of external subprocesses or handling
of a cloud error after replacement acceptance.

The disconnect boundary is explicitly stdin EOF with stdout retained for
observation. It does not simulate full-duplex loss or killing the provider.
Not waiting for a response does not establish that provider acceptance had not
already occurred. No ambiguous request is automatically retried.

Use `CLAUDINE_PI_FAILURE_REPORT` to choose the new test's JSON output path;
`CLAUDINE_PI_REPORT` still applies to the original contract test. The repeat
command above now runs both tests. Lint and the maintained research validator
also pass. Four scoped Pi verification records now exist, including expected
failure behavior; their presence is not a production activation decision.

## Controlled session switching and provider kill

[Saved results](pi-macos-0.84.4-switch-crash.json), nextest run
`ebe8a54f-2fdb-4cdf-8639-9244126eb755`: three opt-in Rust tests passed, zero
skipped; scoped Clippy passed. Set `CLAUDINE_PI_RACE_REPORT` to save this test's
output when repeating the command above.

| Scenario | Acceptance | Saved steering | Model consumption |
| --- | --- | --- | --- |
| Queue steering while an extension holds a session switch | Acknowledged against the still-observable old session | None after switch | None |
| Queue steering after switch acknowledgment | Acknowledged | Once, in the new session | Observed on explicit next prompt |
| Kill provider after acknowledgment during held tool batch | Acknowledged before kill | None | None |

The switch hook creates a controlled interleaving; it does not disable normal
extension, skill, template, or context discovery. This tests the real provider's
handling of an implicit current-session target. It does not prove every scheduling
order, concurrent sender arbitration, or protection from arbitrary extensions.
The kill test terminates only its owned Pi child. It does not test controller or
host failure, restart recovery, or disk durability.

Claudine must coordinate session changes and submissions. A fresh `get_state`
alone cannot protect a later send. An independently initiated switch requires
an effective provider guard or a profile limitation; a wrapper lock cannot
control actors outside that lock. Queue acceptance must remain distinct from
saved history and model delivery after a crash. No automatic replay is justified.

## External tool process cleanup

The [built-in bash results](pi-macos-0.84.4-bash-cleanup.json) passed both contract
assertions in a separate disposable runner. A deterministic local model calls
Pi's real built-in bash tool, which replaces its shell with a marked external
`sleep` process. The command expires after 20 seconds as a final fixture safeguard.

| Action | Process observed after a 750 ms delay | Fixture cleanup |
| --- | --- | --- |
| RPC abort, then acknowledgment | Absent | Unnecessary |
| Kill Pi with SIGKILL | Still running | Required; process then confirmed absent |

The runner checks the fixture process marker and process-group identity before
the deliberate failure experiment. It preserves normal resource discovery and
confirms fixture context, skill, and template availability. The evidence tests
one external process, not additional descendants or detached work. No cloud
inference or existing user session is involved. The POSIX experiment rejects
Windows before launching; native Windows requires a separate process-lifetime
fixture, and Linux has not been executed.

```sh
CLAUDINE_PI_BINARY=/absolute/path/to/pi \
CLAUDINE_PI_CLEANUP_REPORT=/tmp/pi-cleanup-result.json \
node claudine/features/2026-09-08-steering/verification/run-pi-bash-cleanup.mjs
```

Claudine must preserve ownership sufficient to clean up tools after provider
failure; normal provider cancellation alone cannot establish that guarantee.
There are now seven scoped Pi verification records. The expected-loss and
cleanup assertions remain distinct from successful steering delivery.
