# Managed Codex App-Server Execution

When you run Codex through Claudine without `-i` — `claudine codex "fix the
failing test"`, a `compose`, or a `sequence` step — Claudine can run Codex's
app-server instead of `codex exec` and stay connected to it for the whole run.
The output, final answer, and exit code are the same as `codex exec` gives;
what changes is that the run can be **steered** while it works: by you with
[`claudine steer`](../cli/steer.md), or by Claudine's
[automatic repetition help](automatic-steering.md).

Nothing changes in how you invoke Claudine. Interactive runs
(`claudine codex -i`) still start Codex's own terminal UI.

## Which runs use it

`codex exec` is itself a client of the same app-server, running in-process
(Codex 0.157.1 source). A managed launch therefore runs the same core — your
`~/.codex` configuration, skills, prompts, `AGENTS.md`, and MCP servers — as
long as every `exec` option Claudine would pass has an exact app-server
equivalent:

| `codex exec` option | Managed launch |
| --- | --- |
| `-c`/`--config`, `--enable`, `--disable`, `--strict-config` | passed to `codex app-server` unchanged |
| `-m`/`--model`, `-s`/`--sandbox`, `--ephemeral` | `thread/start` parameters |
| `--dangerously-bypass-approvals-and-sandbox` | sandbox `danger-full-access` |
| `exec resume <id>` | `thread/resume` of that thread |
| `--output-last-message <file>` | written by Claudine from Codex's last agent message |
| `--json`, `--color` | not needed |
| anything else (`-i`, `--output-schema`, `-p`/`--profile`, `--oss`, `--add-dir`, `-C`, a prompt argument, …) | **the run stays on `codex exec`** |

A run that `codex exec` itself would refuse — outside a Git repository without
`--skip-git-repo-check` or the bypass flag — also stays on `codex exec`, which
then refuses it with its usual message. Both decisions are made before
anything starts; a run that stays on `exec` simply cannot be steered.

```sh
# Managed: every option maps.
claudine codex -- --skip-git-repo-check "summarize the repo"

# Stays on codex exec: --ignore-rules has no app-server equivalent.
claudine codex -- --skip-git-repo-check --ignore-rules "summarize the repo"
```

Codex's research selects the app-server as the preferred interface and
`codex exec --json` as its fallback
(`docs/research/non-interactive-sessions/codex.md`); the wrapper reads that
selection from the generated steering catalog.

## A run, start to finish

```mermaid
sequenceDiagram
    participant C as Claudine (owner)
    participant X as codex app-server --listen stdio://
    C->>X: spawn, keep stdin/stdout/stderr
    C->>X: initialize
    X-->>C: userAgent (names the exact Codex version)
    C->>X: initialized, thread/start {approvalPolicy: never, …}
    X-->>C: thread id
    C->>X: turn/start (the task)
    X-->>C: turn id; turn/started … items … turn/completed
    C->>X: thread/read (is anything still running?)
    X-->>C: idle
    Note over C: write --output-last-message
    C->>X: close stdin
    X-->>C: exits 0
```

1. **Readiness.** `initialize` must answer with a user agent, which names the
   exact Codex version (the version a steering grant is checked against).
   Then the thread is started — or resumed — with exec's settings. Its id is
   the conversation the run is bound to.
2. **Task.** One `turn/start`. Its answer names the turn; a refusal fails the
   run, as exec would.
3. **Output.** Codex's notifications are projected onto the same events
   `codex exec --json` prints, so the live display, usage, and summary look as
   before.
4. **Settlement.** Closing stdin while a turn runs makes the app-server exit
   successfully *and abandon the turn* (observed with Codex 0.157.1), so
   Claudine closes it only after a `turn/completed`, when `thread/read` shows
   the thread idle and no turn started meanwhile. A steering message can
   extend the running turn, and an idle message starts another; either keeps
   the run open until it too completes.
5. **Exit.** The app-server exits 0 once stdin closes. Claudine applies
   exec's exit rule itself: a failed turn, an error Codex did not retry, or a
   final interrupted turn exits 1. If Codex lingers five seconds after stdin
   closes, the run is completed by terminating it.

The run keeps every guard ordinary structured runs have: `timeout`,
`step_timeout`, runaway-output guards, Ctrl+C, and the failure lifecycle.
Claudine's own replies (answers to its requests) never count as agent
activity for `step_timeout`.

## Unattended requests

Claudine answers every request Codex makes exactly as `codex exec` does, and
never approves anything:

| Codex request | Answer |
| --- | --- |
| `mcpServer/elicitation/request` | cancel |
| every other request (command, file-change, and permission approvals, `request_user_input`, dynamic tools, …) | JSON-RPC error `-32000`: not supported in a managed run |
| a request Claudine cannot read (for example, no id) | none is possible: the run fails with `error_kind = "input_required"` |

## When the app-server is unavailable

If Codex cannot become ready — it exits, refuses `initialize` or the thread,
answers with something unreadable, or stays silent for 60 seconds — the task
has not been sent. Claudine stops that child, shows its stderr, and runs the
original `codex exec` launch with the task on stdin, with a warning:

```text
⚠ Codex did not initialize: Codex refused it (…). Running `codex exec`
  instead: this run cannot be steered or queried.
```

Once the task's `turn/start` has been written there is no fallback and no
second attempt: Codex does not deduplicate messages, so resending could run
the task twice. A user interrupt during readiness never falls back.

## Steering adapter

The same connection carries steering. The `codex-app-server` adapter
(revision 1, `cli/src/commands/wrap/exec/codex_app_server/executor.rs`)
implements three researched mechanisms:

| Mechanism | When | What it sends | A success means |
| --- | --- | --- | --- |
| `app-server-steer` | Codex is working | `turn/steer` naming the running turn as `expectedTurnId` | **queued** into that turn; the model sees it at the turn's next request, after the running tool batch or the response being generated |
| `app-server-turn-start` | the thread is idle (checked with `thread/read`) | `turn/start` | **accepted**; a new turn starts |
| `app-server-interrupt-then-start` | Codex is working, with explicit consent | `turn/interrupt`, wait for that turn's interrupted completion, `turn/start` | the cancellation and the replacement each report their own outcome |

`expectedTurnId` is Codex's own atomic target guard: if the turn Claudine saw
has ended or changed, Codex refuses the message and nothing reaches the model.
Unlike Pi, nothing but the owner can move a Codex thread, so targeting is safe
without a profile block. Every delivery holds one mutation lock, shared with
the settlement decision. A request that may have reached Codex without an
answer is `unknown` and is never resent. A run that is settling refuses new
steering.

> **Status: enabled on macOS for Codex 0.157.1.** The reviewed policy grants
> `app-server-steer` (working) and `app-server-turn-start` (idle) for a
> managed non-interactive run on macOS at exactly Codex 0.157.1
> ([Steering Activation](steering-activation.md)). Any other version or OS
> lists as unavailable with the reason. Interruption is implemented and
> verified but not granted, because a working run always offers the
> non-interrupting `turn/steer` instead.

What this looks like in practice: a Codex run that starts repeating itself
gets one automatic warning at half the repetition limit (15 identical lines
by default), delivered at its next tool boundary; the model can change course
and the run ends normally. If it keeps repeating, the unchanged limit still
stops it.

## Known limits

- Real-Codex behavior is verified on macOS with Codex 0.157.1 against a
  scripted local model. Linux and native Windows are covered by the
  deterministic fake-Codex tests, not yet by real Codex, so they list as
  unavailable.
- Only the stdio transport is used; Codex's Unix-socket and WebSocket
  listeners are not.
- Natively launched Codex sessions (your own `codex` terminal UI or `codex
  exec`) expose no control endpoint and cannot be steered.

## Testing

| What | Where | Tier |
| --- | --- | --- |
| Strict readers and the exec projection over real 0.157.1 records | `lib/src/stream/protocol/codex/app_server/tests.rs` | L1 |
| Parser handling of an app-server run and exec's exit rule | `lib/src/stream/providers/codex/tests.rs` | L1 |
| Option mapping, the owner, and the adapter over an in-memory wire and a real controller | `cli/src/commands/wrap/exec/codex_app_server/tests.rs` | L1 |
| The wrapper against a compiled fake Codex (`tests/bin/fake_codex`): managed run, fallback, requests, failures, no replay, automatic help | `cli/tests/l1/codex_app_server.rs` | L1 (`test-fixtures`) |
| The session and adapter against the installed Codex: steer at the tool boundary and during generation, stale turn, duplicates, idle turn, consented interruption, stdin EOF, resume | `cli/src/commands/wrap/exec/codex_app_server/tests.rs` (`real_codex_protocol`) | real |
| The wrapper against the installed Codex: parity with `codex exec`, failures, untrusted directory, automatic help and recovery | `cli/tests/real/real_codex_app_server.rs` | real |

Real-tier tests run only when asked:

```sh
just test-real real_codex_   # in claudine/, with `codex` on PATH
```

They use a scripted local model (`tests/common/codex_model.rs`) and a
disposable `CODEX_HOME`; no network beyond loopback, no credentials, and no
existing session is touched.
