---
created: 2026-07-11
status: ready for planning and implementation
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-16
depends-on: ../_completed/2026-07-11-provider-errors-as-data/spec.md
---

# Fleet Research: Deterministic Validate-and-Resume Lifecycles

Upgrade the fleet-research recipe from lifecycles that merely report progress
to lifecycles that gate on deterministic content checks and resume the same
research session when the session can safely correct the result.

> **Reader's note (inline review, 2026-07-16):** the original draft described
> this as an unscheduled idea gated on the `agent-errors` pilot. That dependency
> is now complete. The pilot established the outcome and recovery contracts,
> but its ten-provider live run produced no resumes, so it did not by itself
> prove real-world correction convergence. This reviewed spec therefore keeps
> rollout opt-in, adds a second-topic pilot, and makes the reusable protocol
> explicit. It also closes a stale-success hole: every run first writes a
> run-scoped `pending` outcome, so a prior run's `clean` report can never satisfy
> the current run's `finalize` guard.

## Motivation

Schema sidecars validate the output document's shape, but many important
research invariants are semantic and cross-record:

- seeded facts must not silently disappear, move, or change meaning;
- a provenance label must agree with its citation or empirical fixture;
- required coverage must be researched or explicitly recorded as a gap;
- references between two frontmatter collections must resolve;
- ordered data that feeds first-match-wins runtime behavior must retain its
  intended precedence.

These checks are deterministic. When one fails because the provider-authored
research document is incomplete or inconsistent, the best recovery context is
the session that just performed the research. A bounded lifecycle `resume`
can return the findings to that same session. Failures in authoritative gate
inputs, checker execution, authentication, policy, or report persistence are
not author-correctable and must fail closed.

The `agent-errors` fleet proved the underlying lifecycle mechanism in
[`../_completed/2026-07-11-provider-errors-as-data/spec.md`](../_completed/2026-07-11-provider-errors-as-data/spec.md)
(D10). This feature generalizes that implementation into a stable authoring
standard and reusable gate surface.

## Goals

1. Define one versioned, run-scoped outcome protocol for deterministic research
   gates.
2. Provide a cross-platform Rust gate runner and reusable gate infrastructure;
   fleet prompts must not depend on POSIX-only check scripts.
3. Standardize safe recovery by the owner of a failure: same-session `resume`,
   fresh-run `retry`, or fail-closed maintainer/operator intervention.
4. Update `docs/research/_TEMPLATE.md` with a copy-ready lifecycle shape that
   invalidates stale success, validates the current report envelope, shares
   bounded control budgets, and fails non-clean finalization.
5. Migrate `agent-errors` to the shared protocol without weakening any existing
   checks, then prove reuse with one `signals` provider pilot.
6. Make future topic adoption incremental: add content checks when a topic is
   refreshed or when a known failure class justifies them, not by mass-editing
   every fleet prompt.

## Non-goals

- **Replacing `_schema.yaml`.** Darkmatter `SimplifiedSchema` remains the sole
  shape contract. The deterministic gate loads and validates that same sidecar
  before running content checks.
- **Treating model output as trusted.** A clean deterministic report is a
  mechanical result, not human ratification of sources or conclusions.
- **Adversarial sandboxing.** The gate is correctness automation, not a security
  boundary. Until `grant:` is implemented, human diff review remains mandatory
  and the prompt must tell the agent not to edit seeds, checker code, or gate
  reports.
- **Network-dependent blocking checks.** URL liveness and source freshness are
  advisory because network failure is not evidence that research is wrong.
- **Automatically freezing the entire prior frontmatter document.** Refreshes
  are expected to change facts. Only explicit, human-owned seed assertions are
  sticky.
- **Resuming aggregate fleet checks.** Once a provider session has completed,
  cross-provider checks cannot assume that its live session is still available.
- **A general-purpose validation DSL.** v1 extracts the protocol and proven
  primitives. Topic semantics remain small typed Rust adapters; a declarative
  rule language should be considered only after repeated adapters demonstrate
  a stable common shape.

## Established Contracts

This specification builds on, rather than changes, these Claudine contracts:

- `resume` re-enters the wrapper agent's current session with a prompt override.
  Capability is determined by the **agent running the research**, not the
  provider being researched.
- `resume` and `retry` attempt budgets are run-scoped by control type. Separate
  conditional branches using `max_attempts: 2` share one two-additional-attempt
  resume ceiling; retry has its own ceiling.
- exhausted control dispatch falls through to `finalize`; `finalize` must turn a
  non-clean outcome into command failure.
- lifecycle `when:` predicates are expressions. Lifecycle action values are
  literal by default and use `{{{ ... }}}` for event-time interpolation. Shell
  actions are the early-binding exception and execute the exact command approved
  during preflight.
- a lifecycle shell action that cannot durably write a trustworthy report must
  return non-zero and must not use `no_error: true`.
- target research documents are schema-validated **after the provider writes
  them**. Composing the fleet prompt does not validate a future output file.

## Design

### D1 — Two validation scopes

Checks are classified by the scope in which their evidence is available:

| Scope | Examples | Can recover with same-session `resume`? |
|---|---|---|
| Per-document | schema validity, seed preservation, provenance coherence, joined-record integrity, required coverage-or-gap | Yes, while processing that provider's `success` event |
| Fleet aggregate | byte-identical copy-paste smell, provider-to-provider contradictions, fleet-wide coverage holes | No; write an aggregate report for human review after the sequence |

Only deterministic, per-document checks belong in the resume loop. Aggregate
checks must never reopen a fresh context-free session while describing that as
same-session correction.

### D2 — Shared command surface and ownership

Add a public command family:

```text
claudine research gate begin <topic> <subject> --run-id <id>
claudine research gate check <topic> <subject> --run-id <id>
```

`subject` is the roster slug. The neutral name avoids baking `provider` into a
protocol also used by fleets such as `local_runners`.

The deterministic implementation lives in `claudine-gen`, beside the current
`agent-errors` checker and other research-to-catalog validation. It retains the
generator's bootstrap rule: it must not depend on `claudine` or `claudine-cli`.
The `claudine` CLI delegates through the existing installed-binary/development-
checkout fallback, but the fleet prompt uses only the public `claudine research
gate` surface.

`begin` and `check` derive the research document, sidecar, seed, and report paths
from the validated `<topic>` and `<subject>` identifiers. Fleet shell commands
must not interpolate filesystem paths or contain platform-specific `sh`, `cmd`,
or PowerShell syntax. Internally, all file references resolve through
`biscuit_file::FileReference`.

The old `claudine providers agent-errors check` surface and its topic-specific
report plumbing are removed when the fleet migrates; retaining two internal
gate protocols would create drift with no compatibility benefit in this new
codebase.

All human-facing output from both binaries uses `TerminalRenderable`
components. Use `Prose` for status/detail and `UnorderedList` for findings; raw
`println!` formatting is not part of this feature.

### D3 — Versioned, run-scoped outcome protocol

The transient report lives at:

```text
docs/research/<topic>/.findings/<subject>.md
```

All `docs/research/**/.findings/` directories are gitignored. Reports are
machine state for the active run, not research artifacts.

The report frontmatter has this v1 envelope:

```yaml
protocol_version: 1
run_id: "<opaque value supplied by the fleet prompt>"
topic: agent-errors
subject: codex
status: pending # pending | clean | findings | gate_error
findings: []
# error: present for gate_error
# error_scope: research_document | gate_input
```

Rules:

1. After a freshness `skip` decision but immediately before provider launch,
   the `start` stack calls `gate begin`. It atomically replaces any previous
   report with `status: pending` for the current `run_id`.
2. `gate check` refuses a missing or mismatched pending envelope. A topic or
   subject mismatch is a protocol/input failure, never permission to reuse the
   file.
3. A completed check atomically replaces `pending` with exactly one terminal
   status: `clean`, `findings`, or `gate_error`.
4. `clean` is explicit. Absence, `pending`, an unknown status, a wrong protocol
   version, or a wrong `run_id` is never clean.
5. Each finding carries a stable `check` identifier and an actionable `detail`;
   optional structured location fields may identify a frontmatter path, row, or
   seed identity.
6. `gate_error` carries `error_scope: research_document | gate_input`.
   `research_document` means the live session can repair its authored output.
   `gate_input` means a sidecar, seed, checker configuration, or other
   authoritative input requires maintainer action.
7. A successfully persisted `findings` or `gate_error` report is a completed
   gate execution, so the checker exits zero and lets lifecycle policy branch
   on the report. Failure to execute or persist a trustworthy replacement exits
   non-zero and stops the stack before any report condition is evaluated.
8. Replacement uses a synced sibling temporary file and atomic persistence.
   A failed replacement leaves the last valid report for the **current run**
   readable; it must never delete first and create a window in which absence
   could be mistaken for success.

The fleet prompt derives `run_id` once in ordinary frontmatter from stable
per-execution context plus `state.slug`, passes it to both commands, and checks
the complete envelope in `success` and `finalize`. Concurrent runs of the same
topic and subject are unsupported in v1; `begin` must fail loudly if it detects
an active conflicting run rather than allowing two writers to race.

### D4 — Authority and seed policy

Each adopted topic may carry human-owned seed assertions under:

```text
docs/research/<topic>/_seeds/<subject>.yaml
```

Seeds contain only facts whose preservation is an invariant. They are not an
automatic copy of the previous frontmatter and need not mirror the whole topic
schema. This avoids two bad outcomes: freezing facts that research is supposed
to refresh, and relying on a racy pre-run copy of a mutable output document.

The checker validates the output document against `_schema.yaml` first, then
loads the topic's seed and evaluates topic rules. A malformed output document
is `gate_error/research_document`; a missing or malformed required seed or
sidecar is `gate_error/gate_input`.

Research sessions may read seeds but must be instructed not to modify them.
Because the current `--yolo` fleet posture cannot enforce that write boundary,
the implementation must not claim that seeds are tamper-proof. The pilot
checkpoint requires a clean human-reviewed diff outside the target research
document and explicitly verifies that seed and checker inputs were unchanged.
When `grant:` becomes enforceable, topic prompts should grant writes only to
the target document and approved fixture directory.

### D5 — Reusable engine, typed topic adapters

The shared Rust gate engine owns:

- report envelope parsing, state transitions, run identity, and atomic writes;
- Darkmatter frontmatter parsing and sidecar validation;
- deterministic finding ordering;
- topic/subject/path validation and `FileReference` resolution;
- common seed-row identity and ordered-preservation helpers;
- common conditional-provenance and coverage-or-gap helpers;
- terminal rendering of clean, findings, and gate-error outcomes.

Each topic supplies a small typed adapter that loads its structured
frontmatter/seed shape and returns findings. Complex topic meaning stays in the
adapter. In particular, provenance coherence is reusable only where that
topic's sidecar actually declares provenance; the original draft's claim that
it generalizes to every topic was too broad.

v1 does **not** create `docs/research/_checks/` shell scripts. Compiled Rust is
cross-platform, gives findings stable types, keeps authoritative logic out of
the provider's ordinary edit target, and reuses the existing checker boundary.
Extract a new common semantic helper only when at least two topic adapters need
the same behavior.

### D6 — Canonical lifecycle order

`docs/research/_TEMPLATE.md` gains a deterministic-gate variant with this
normative order:

1. **`initialize`:** apply the normal freshness guard and `skip` before any
   report mutation.
2. **`start`:** call `gate begin` for this topic, subject, and run ID. Any
   failure blocks provider launch.
3. **`success`, postcondition:** if the output is missing or its
   `last_updated` stamp is not today, resume the same session.
4. **`success`, gate:** call `gate check`. Do not set `no_error`.
5. **`success`, envelope validation:** reject a missing report, wrong protocol
   version, wrong run ID/topic/subject, `pending`, unknown status, or unknown
   error scope before accepting any status branch.
6. **`success`, recovery:** resume `findings` and
   `gate_error/research_document`; fail `gate_error/gate_input`; report
   `clean`.
7. **`failure`:** resume timeouts when a session handle exists; retry only
   `err.is_transient` failures with exponential backoff. No catch-all retry.
8. **`finalize`:** require the current v1 report envelope and `status: clean`.
   Exhausted remediation therefore exits non-zero while preserving the last
   durable report.

All same-session remediation branches, including timeout recovery, use the
same `max_attempts: 2` resume ceiling. Transient fresh-run recovery uses its own
`max_attempts: 2` retry ceiling with exponential backoff and a documented base
delay. A later branch consumes the remaining budget; it does not open a new
budget because the reason changed.

The template must use triple-brace interpolation in lifecycle action values,
for example `message: "Read {{{ findings }}} and correct {{{ file }}}"`.
`when:` remains an expression without interpolation braces. The current
`agent-errors/_fleet.md` double-brace action values are corrected as part of
its migration so its messages comply with the lifecycle literal-by-default
contract.

### D7 — Recovery policy

| Condition | Owner | Lifecycle response |
|---|---|---|
| Provider reports success without a current output document | research session | bounded same-session `resume` |
| Current report is `findings` | research session | bounded `resume` with durable report path and finding classes |
| Current report is `gate_error/research_document` | research session | bounded `resume`; instruct it to edit only the output/approved fixtures |
| Timeout with captured resumable session | research session | bounded `resume` |
| Other `err.is_transient` provider failure | runtime/provider | bounded fresh-run `retry` with exponential backoff |
| Sidecar, seed, or authoritative input failure | maintainer | fail closed |
| Checker execution or report persistence failure | maintainer/infrastructure | fail closed before report branching |
| Missing/mismatched/unknown/pending report envelope | checker protocol | fail closed |
| Auth, configuration, billing, policy, interruption, runaway, or non-waitable cap | operator/policy | fail closed; blind replay is unsafe or predictably ineffective |
| Recovery budget exhausted | maintainer | fall through to `finalize`, preserve report, exit non-zero |

### D8 — Pilot and rollout

The completed `agent-errors` fleet is the mechanism pilot, but its live fleet
had ten clean first attempts and zero resumes. Generalization therefore has two
checkpoints:

1. **Migration checkpoint:** refactor `agent-errors` onto the shared protocol
   and prove byte-equivalent semantic findings plus the new stale-clean
   regression behavior.
2. **Second-topic checkpoint:** adapt the existing deterministic `signals`
   validation for one provider. Deliberately exercise one correctable fixture
   in test, then run one real provider refresh. Review whether the resumed
   session corrected only the research output, converged within the shared
   budget, and left authoritative inputs unchanged before applying the pattern
   to the rest of that fleet.

After both checkpoints, other topics adopt gates only on their next refresh or
after a documented failure class. Every adoption must name its deterministic
checks, authority inputs, correction owner, and a false-positive escape path
(`gaps` or human adjudication). Network liveness stays advisory.

## Implementation Increments

### Phase A — Shared protocol and `agent-errors` migration

1. Add the shared gate report types, `pending` state, run identity, atomic
   transition logic, path validation, and common renderer in `claudine-gen`.
2. Add `claudine research gate begin|check` and its generator passthrough.
3. Move the `agent-errors` checker behind a typed topic adapter; preserve its
   seed removal/re-kind/reorder, needle hygiene, invented-seed, provenance, and
   capacity-or-gap checks.
4. Update `agent-errors/_fleet.md` to initialize the current run in `start`,
   validate the complete envelope, use triple-brace lifecycle action
   interpolation, and require current-run clean finalization.
5. Gitignore transient findings directories and remove the old topic-specific
   command/report surface.

### Phase B — Authoring standard

1. Add a deterministic-gate variant to `docs/research/_TEMPLATE.md`, including
   the owner/recovery matrix and warnings against stale reports, shell-script
   checkers, whole-prior-document freezing, and network blocking checks.
2. Update the Claudine skill lifecycle/research documentation so the template,
   CLI reference, and architecture module map agree with the implementation.
3. Document how a topic declares seed authority and how a topic adapter
   distinguishes research-document errors from gate-input errors.

### Phase C — Second-topic pilot and incremental adoption

1. Wrap the existing `signals` deterministic validation in a topic adapter and
   v1 report without weakening its fixture replay or overlap checks.
2. Pilot one provider, review remediation telemetry and the git diff, then
   decide whether to enable the lifecycle for the remaining signals fleet.
3. Record eligible checks for other topics as refresh-time work; do not mass
   migrate unchanged fleet prompts.

## Verification

All automated coverage in this feature is Level 1; it needs subprocesses and
temporary files but no real terminal, browser, device, or external service.

- Unit-test every allowed and rejected report transition, including
  `previous clean → current pending → exhausted missing-output resume →
  finalize failure`.
- Test missing, stale, wrong-version, wrong-run, wrong-topic, wrong-subject,
  unknown-status, unknown-scope, and still-pending envelopes.
- Preserve the existing `agent-errors` checker corpus and prove deterministic
  finding order and semantic parity before/after extraction.
- Test schema-invalid output as `research_document`, invalid seed/sidecar as
  `gate_input`, and report persistence failure as non-zero with no stale report
  consumption.
- Parse the committed fleet lifecycles through production lifecycle parsing;
  assert one shared resume budget, a separate retry budget, terminal control
  placement, and a clean-only current-run finalize guard.
- Run an in-process lifecycle test for finding → resume → corrected document →
  clean, plus persistent findings through budget exhaustion.
- Run a CLI process test proving the initial invocation plus two resumes exits
  non-zero when findings persist and preserves the final report.
- Add Windows-safe path and command-construction coverage; no test or runtime
  path may assume `/bin/sh`, single-quote semantics, or Unix path separators.
- Run `just test` and `just lint` from the Claudine package area.

## Acceptance Criteria

- A prior `clean` report cannot satisfy a later run under any early-exit or
  exhausted-resume path.
- Only an explicit current-run v1 `clean` report can complete a gated fleet item
  successfully.
- Research-document findings resume the same session at most twice in total;
  gate-input and protocol failures never resume.
- `agent-errors` retains every existing deterministic check and uses the shared
  command/report protocol.
- One `signals` provider completes the second-topic checkpoint with reviewed
  telemetry and unchanged authority inputs.
- The canonical fleet template, CLI reference, Claudine skill docs, and
  implementation describe the same lifecycle and protocol.
- The feature works on macOS, Windows, and Linux without topic-owned shell
  scripts.

## Open Questions

None. The inline review resolves the original location and findings-threading
questions in favor of a typed Rust gate runner plus a durable, versioned
Markdown outcome report. Future pressure for a declarative rule format must be
supported by at least two genuinely equivalent topic adapters before it becomes
a design proposal.
