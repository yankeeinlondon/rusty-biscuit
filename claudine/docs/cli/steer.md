---
blast_radius:
  - claudine/cli/src/commands/steer/
  - claudine/cli/src/steering/
  - claudine/lib/src/steering/discovery.rs
  - claudine/lib/src/steering/eligibility.rs
---
# Steering a running agent (`claudine steer`)

`claudine steer` sends one message to one agent session that is already
running, to course-correct it without stopping it. You pick the session from a
list, or name it exactly by ID. The command never broadcasts, never guesses a
target, and never interrupts a session without asking you first.

> **What works today.** The command, listing, selection, consent, and receipts
> are implemented. Which sessions can actually receive a message depends on
> reviewed provider evidence
> ([Steering Activation](../topics/steering-activation.md)). Today that is a
> non-interactive Codex run Claudine launched through Codex's app-server on
> macOS with Codex 0.157.1
> ([Managed Codex app-server execution](../topics/codex-app-server.md)): while
> it works the message joins the running turn; while it is idle it starts a
> turn. A managed Pi RPC run is listed with a reviewed block, and every other
> run is listed with its own reason.

## Usage

```sh
# Pick a session interactively, then send.
claudine steer "Recheck the failing test before changing the implementation."

# Send to an exact session (IDs come from --list).
claudine steer --session managed:1b4e28ba-2fa1-11d2-883f-0016d3cca427 "Recheck the failing test."

# List sessions (sends nothing), for people or for scripts.
claudine steer --list
claudine steer --list --json

# Scripted send with a typed result.
claudine steer --session <id> --json "Recheck the failing test."

# A message that starts with a dash.
claudine steer --session <id> -- "--dry-run first, please"
```

There are no `send`/`list` subcommands and no flag that skips interruption
consent.

## What happens when you send

```mermaid
flowchart TD
    A[claudine steer MESSAGE] --> V{message valid?}
    V -- no --> U[exit 2]
    V -- yes --> T{--session given?}
    T -- no, --json or no terminal --> U
    T -- no, terminal --> L[list sessions, show reasons]
    L --> P{pick one}
    P -- cancel --> C[exit 130, nothing sent]
    T -- yes --> F{ID in the listing?}
    F -- no --> N[exit 1, nothing sent]
    P --> R
    F -- yes --> R{availability}
    R -- unavailable --> N
    R -- interruption required --> Q{terminal and not --json?}
    Q -- no --> N
    Q -- yes --> K{you consent?}
    K -- no --> C
    K -- yes --> X
    R -- non-interrupting --> X[look the session up again if a human paused]
    X -- changed or ended --> N
    X -- same --> S[send once; report the receipt]
```

1. **Validate.** The message is sent byte for byte: it is never trimmed,
   truncated, or split. It is rejected (exit 2) when it is empty or only
   whitespace, contains a NUL character, or is longer than 64 KiB of UTF-8.
2. **Choose.** Without `--session`, the command shows the listing and asks you
   to pick, even when only one session is selectable. Unavailable sessions are
   shown but cannot be picked. `--json`, or stdin/stderr that is not a
   terminal, cannot ask, so a send without `--session` fails with a pointer to
   `claudine steer --list`.
3. **Consent.** A session that can only be reached by stopping its current
   turn is marked `interruption required`. Before anything happens the command
   explains what stopping means — the turn stops first; a tool it is running
   may stop with it; messages already waiting are kept, not cleared; if the
   stop works but the message is not confirmed you get a partial result and
   the old work is not restarted — and asks. An explicit `--session` skips
   only the picker, never this question. `--json` and non-terminal runs refuse
   instead of asking.
4. **Check again.** If you spent time in the picker or the consent prompt, the
   session is looked up again before sending. If it ended, its conversation was
   replaced, or it now needs a different action (for example it went idle, so
   the message would start a new turn), nothing is sent and the command says
   why. If it now needs interruption, you are asked again. The message is never
   redirected to another session.
5. **Send once and report.** The command returns as soon as the provider
   answers. It reports only what the provider established and never resends.

Sending never changes configuration: `steer` does not run the setup wizard,
install anything, or edit provider settings.

## Reading the listing

```text
 # │ Provider │ Session  │ Directory      │ State   │ Steering
 1 │ pi       │ fixture  │ /work/project  │ working │ unavailable
 ...
#1 managed:1b4e28ba-2fa1-11d2-883f-0016d3cca427
- Why: steering is blocked for this launch profile: …
- Seen by Claudine wrapper; launch profile retained-rpc; provider version unknown.
```

- **Steering** is always spelled out: `non-interrupting`, `interruption
  required`, or `unavailable`. On a color terminal unavailable rows are also dim
  and struck through; the words carry the meaning without styling.
- Each row's details give the full ID to pass to `--session`, what sending
  would do ("adds the message to the running turn", "the session is idle, so
  the message starts a new turn"), the reason it is unavailable or needs
  interruption, and any **setup for future launches**. Setup never changes a
  session that is already open.
- A terminal too narrow for the table shows each row as one wrapped summary
  line instead. IDs are never wrapped or shortened.
- Rows are ordered by provider (research-roster order), then directory, then
  ID. Sessions are deduplicated only when their identities prove they are the
  same session.
- If a discovery source fails, the other sources' rows are still listed and
  the failure is a warning on stderr. For example, with no local Rendezvous
  daemon running, managed sessions are missing but native Claude Code
  sessions are still listed. Only when every source fails does the command
  fail.

## Receipts

| Outcome | Exit | Meaning |
| --- | --- | --- |
| `accepted` | 0 | The provider accepted the message. Not proof it reached the conversation or that the agent acted on it. |
| `queued` | 0 | The provider will deliver it at its next boundary. Not in the conversation yet. |
| `delivered` | 0 | In the conversation. Not proof the agent acted on it. |
| `held` | 1 | Held by the provider's inbound policy and **not delivered**. Changing that policy is separate setup. |
| `refused` | 1 | Refused; nothing was delivered. |
| `unavailable` | 1 | Nothing was sent (no route, ended or changed session, consent unavailable). |
| `busy` | 1 | Nothing was sent: the session's queue was full or the request expired first. |
| `partial_interruption` | 1 | The turn was stopped but the message was not confirmed. |
| `unknown` | 1 | It may have been submitted. It was **not retried**, to avoid a duplicate; check the session before sending again. |

Other exit codes: **2** for invalid usage (bad message, malformed ID, `--json`
or no terminal without `--session`, conflicting flags), **130** when you
cancelled the picker or declined interruption. Cancelling never sends
anything.

## JSON

Both documents carry `schema_version: 1`; unknown values are explicit `null`.
Status messages and warnings go to stderr, so stdout holds only the document.

`claudine steer --list --json`:

```json
{
  "schema_version": 1,
  "observed_at": "2026-09-28T17:02:11Z",
  "sessions": [
    {
      "id": "managed:1b4e28ba-2fa1-11d2-883f-0016d3cca427",
      "provider": "pi",
      "name": null,
      "cwd": "/work/project",
      "state": "working",
      "origins": ["managed"],
      "launch_profile": "retained-rpc",
      "provider_version": null,
      "availability": "unavailable",
      "operation": null,
      "reason": "steering is blocked for this launch profile: …",
      "setup_requirements": [],
      "observed_at": "2026-09-28T17:02:11Z"
    }
  ],
  "discovery_errors": [{ "source": "codex", "message": "…" }],
  "coverage_gaps": [{ "provider": "codex", "discovery_id": "…", "method": "provider_api" }]
}
```

- `id` is the exact value `--session` accepts. IDs are opaque; prefixes are
  never accepted.
- `availability` is `non_interrupting`, `interruption_required`, or
  `unavailable`; `operation` is what sending would do (`steer_active_turn`,
  `queue_follow_up`, `start_idle_turn`, `interrupt_then_submit`) and is `null`
  exactly when unavailable.
- `discovery_errors` lists sources that failed (secret-masked);
  `coverage_gaps` lists researched ways to find sessions that this build
  cannot run yet, so sessions they would find are missing.
- Sessions started outside Claudine are listed where this build can find
  them: today, Claude Code sessions from Claude's own session registry
  ([Steering Routing — Native Claude Code sessions](../topics/steering-routing.md#native-claude-code-sessions)).
  They appear with `origins: ["native"]` and, for now, as unavailable with
  the reason.
- Without a local Rendezvous daemon, managed sessions cannot be listed; the
  listing still succeeds with the native sessions it found and a
  `discovery_errors` entry for the `managed` source.

`claudine steer --session <id> --json "<message>"`:

```json
{
  "schema_version": 1,
  "request_id": "0b9c2d3e-5f60-4a71-8b92-a3b4c5d6e7f8",
  "target": "managed:1b4e28ba-2fa1-11d2-883f-0016d3cca427",
  "operation": "steer_active_turn",
  "mechanism": "rpc-steer",
  "outcome": "queued",
  "receipt": "queued",
  "interruption": null,
  "detail": null
}
```

- `receipt` is `accepted`, `queued`, `delivered`, or `unknown`; it is never
  stronger than `outcome`.
- `interruption`, when an interruption was attempted, keeps its two phases
  separate: `{"cancellation": "established", "replacement": "unknown"}`.
- `detail` explains any non-confirmed outcome. It is secret-masked, and the
  message text is never echoed.
- A send that fails before reaching the session (unknown ID, unavailable,
  interruption without consent) still prints this document, with outcome
  `unavailable` and a `detail`.

## How it is built

`claudine steer` is the requester in
[Steering Routing](../topics/steering-routing.md): it lists managed sessions
through the local Rendezvous daemon and routes one request to the wrapper that
owns the session, which revalidates it and submits it through the provider's
adapter. The code is `cli/src/commands/steer/` (command, rendering, and the
picker/consent prompts) over `cli/src/steering/requester.rs`.
