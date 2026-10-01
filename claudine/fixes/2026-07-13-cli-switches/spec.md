---
reviewed: false
refreshed_on: 2026-10-01
---

# Composition forwards provider CLI switches to the agent

## Status (2026-10-01)

The headline bug is fixed. Commit `2c7f98dcf` landed the ownership
partition, the implicit and explicit (`--`) agent tail, the switch-before-file
error, Claudine ownership of colliding switches, the retirement of argv Rule 3,
request-level threading of the tail, the dry-run "Provider args" row, and a
dormant typed native-exit classifier. This command now works:

```sh
claudine sequence docs/research/agent-errors/_fleet.md -y --codex -c 'model_reasoning_effort="low"'
```

The rest of this spec states the contract and describes only the work that
remains. [Remaining work](#remaining-work) lists the gaps, and
[Acceptance criteria](#acceptance-criteria) records which criteria already
pass. `plan.md` still applies to the remaining work. Its Phase 1 audit is
answered by this status section, and most of its Phase 2 partition tasks are
done (see the criteria table).

## Problem

Composition subcommands (`compose`, `inline-compose`, `sequence`) rejected any
switch outside Claudine's own surface instead of forwarding it to the agent CLI,
the way direct wrappers (`claudine codex ...`) already did. The parsing half of
that is solved. What remains is making the forwarded tail survive every launch,
report honestly, and fail understandably:

- **Resume drops the tail.** A lifecycle `resume` rebuilds the provider's argv
  from its resume entrypoint and carries over only a hardcoded allowlist, so
  `-c model_reasoning_effort=low` is silently lost on the resumed attempt.
- **A provider that rejects the tail gets a generic error.** The correlated
  report exists in code but nothing calls it.
- **The INFO notice says something false.** It claims the switches are "not
  recognized by Claudine" before any switch catalog exists.
- **Completion keeps a second, already drifted ownership list.**

## Contract

These rules are implemented unless a [remaining-work](#remaining-work) item
says otherwise.

### Token ownership

A single pass in `cli/src/argv/partition.rs` (`partition_composition_tail`)
runs after `argv::normalize` and before clap. It splits composition argv into
two explicit vectors: the Claudine argv clap parses, and a `ProviderArgs` tail.
The tail is never rebuilt later from clap matches or `std::env::args()`.

| Token, scanned left to right | Owner |
| --- | --- |
| A switch on Claudine's clap surface, before an explicit `--` | Claudine, even after an implicit tail has started |
| A bare token or `key=value` before the tail starts | Composition grammar: one file plus setters, in any order |
| The first unowned switch **after** the file | Starts the implicit agent tail |
| Every non-Claudine token after the tail starts | Agent, in original order. Setter-shaped tokens are agent values |
| An authored `--` after the file | Consumed by Claudine. Everything after it is an opaque agent tail |
| An unowned switch or `--` **before** the file | Error with ordering guidance |

```sh
claudine compose <file> [key=value ...] [CLAUDINE_OPTIONS] [AGENT_ARGS ...]

claudine compose doc.md --codex -c model_reasoning_effort=low   # tail: -c model_reasoning_effort=low
claudine compose doc.md --codex -c x -m gpt-5                   # -m stays Claudine's; tail: -c x
claudine compose doc.md --codex -- -m gpt-5 --help              # tail: -m gpt-5 --help (opaque)
claudine compose --unknown doc.md                               # error: the file must come before agent switches
```

The owned surface is `OwnedFlags::for_composition`, derived from the clap
definitions, never from a handwritten list. Bare provider operands need an
explicit `--`. Two ordinary bare positionals keep clap's multiple-file error.
Routing never depends on researched switch metadata.

### Launch threading

The tail is request-level launch state on `CompositionExecutionRequest`,
separate from MCP arguments and Claudine-owned flags. It seeds the child argv at
the same stage as direct-wrapper passthrough (`LaunchPlanInputs::provider_args_tail`),
before entrypoint, model, transport, system-prompt, MCP, and prompt-delivery
injections. Fresh retries and proxy targets rebuild the launch plan from the
same inputs, so they carry the tail. Every `sequence` step receives the same
tail token for token, whichever provider the step resolves to.

### Reporting and redaction

- One INFO status before launch, rendered with `TerminalRenderable` and
  suppressed by `--quiet` and `--silent`. It shows switch names only and strips
  any `=value` suffix.
- Every surface that shows more of the tail (dry-run, debug traces,
  `AGENT_PARAMS`, correlated diagnostic excerpts) passes it through
  `redact_sensitive_args`. The child receives the original tokens.
- Correlated errors are never suppressed by quiet or silent modes.

### Native-exit classification

`NativeCliCause` and `classify_native_cli_cause` in
`cli/src/output/error_report.rs` classify a provider's process exit. They are
deliberately separate from the structured-stream vocabulary in
`lib/src/stream/providers/vocabulary.rs` and must stay that way.

## Remaining work

### R1. Resume carries the forwarded tail

`commands/wrap/resume.rs::append_resume_passthrough_args` copies a fixed
allowlist (`--json`, `--verbose`, `--print-logs`, `--approve`/`--no-approve`,
`--output-format`, `--format`, `--output-last-message`, `--log-level`, `--mode`)
from the base argv into the resume argv. `harness_orch/launch.rs` calls it on
every resume attempt, so any forwarded switch outside that list is dropped.

Required:

- The resume argv receives the request's provider tail exactly once, at the
  position the profile's resume entrypoint expects. Read the tail from the
  typed request state (R6), not by pattern-matching the base argv.
- The transport/safety allowlist keeps its job for Claudine's own injections.
  The tail is not added to it as a second mechanism.
- If a provider's resume entrypoint does not accept the tail (for example a
  subcommand that rejects root switches), that is an R2 correlated error, not
  a silent drop. Claudine does not filter the tail per entrypoint.
- `harness_orch/session_key.rs` compares the **canonical** argv and documents
  the allowlist as an intentional drop. Update that comment. The tail is
  invocation-fixed, so it must not make a resume look incompatible.

### R2. Correlate argument rejection on every launch path

`AgentErrorReport::correlated_with_forwarded_tail` is `#[allow(dead_code)]`.
The direct wrapper (`commands/wrap/mod.rs`) renders
`from_exit_code_with_source`, and the composition attempt path
(`harness_orch/attempt.rs`) echoes stderr without building a native-cause
report at all. The classifier has three defects:

1. **It reads stderr only.** Some providers print usage errors on stdout.
   Classification takes the exit code, termination state, and bounded tails of
   both streams.
2. **The precedence is wrong.** It checks `ArgumentRejected` before
   `AuthOrPermission` and before `FileNotFound`, `classify_exit` checks API
   errors after both, and timeouts are invisible to it. The required order is:
   interruption → timeout → missing binary → authentication/permission → API
   failure → model not found → argument rejected → missing argument → none.
3. **A signature is too broad.** `invalid argument` matches auth and API
   messages (`invalid argument: api key`). Keep only signatures backed by a
   positive fixture and a near-miss fixture. An uncertain exit returns `None`.

Required:

- A typed native-exit input carrying the exit code, `ProcessTermination`, and
  bounded stdout/stderr tails, produced by both the direct wrapper and the
  composition attempt.
- One report builder that both paths call exactly once per terminal failure.
  It produces the correlated report only when the launch had a non-empty tail,
  the exit was non-zero, and the classifier returned `ArgumentRejected`.
  Otherwise it produces the existing report unchanged.
- The correlated report names the redacted switch names, or says the tail was
  opaque. It includes a redacted excerpt of the provider's diagnostic and says
  "likely caused by the forwarded arguments". Its wording must not claim
  Claudine failed to recognize the switch (that claim waits for R8).
- Presentation only: exit code, termination state, lifecycle
  `failure`/`finalize`, and retry policy are unchanged.
- Remove the `dead_code` allowances that this wiring makes stale.

### R3. Fix the INFO notice

`commands/wrap/composition/provider_args.rs`:

- **Scope.** Deduplication uses a process-wide `static ANNOUNCED`. Move it into
  per-command execution state owned by the top-level command and threaded to
  the launch. It holds at most one notice per distinct `(provider, tail)` pair,
  and nothing leaks between invocations or between tests in one process.
- **Wording.** "not recognized by Claudine" is forbidden until R8 can know it.
  Implicit tail: `Forwarding provider arguments to Codex: -c`. Explicit tail:
  `Forwarding an opaque argument tail to Codex (passed after --).`
- **Location.** The notice moves out of `wrap/composition/` into a module both
  launch paths can use (R5).

### R4. Completion uses the owned surface

`completion/engine/tokens.rs::is_value_bearing_flag` is a handwritten list of
value-bearing switches. Its doc comment points at
`crate::argv::COMPOSITION_FLAGS_WITH_VALUE`, which `2c7f98dcf` deleted.

Required:

- Completion's cursor scan uses `OwnedFlags::for_composition`, the partition's
  surface, so the two cannot drift. Delete the list and the stale comment.
- Composition completion never fails while the cursor is inside an implicit
  tail. Claudine suggestions stop after an authored `--`. File and setter
  completion before the tail are unchanged. Completing provider switches stays
  out of scope.

### R5. Direct wrappers share reporting

Today direct wrappers share only `classify_native_cli_cause`. They forward
their passthrough positional with no INFO notice and no correlation. After this
work both launch paths use the same tail descriptor (R6), notice (R3),
redaction, and correlated report (R2). Direct-wrapper child argv must not
change. `tests/l1/wrap_direct_argv.rs` is the guard for that.

### R6. One typed tail descriptor, and no silent byte changes

- `CompositionExecutionRequest` (lib) and `SharedComposeArgs` (cli) carry the
  tail as parallel fields, `provider_args: Vec<String>` and
  `provider_args_explicit: bool`. Replace both with one typed descriptor in the
  library, holding the ordered args and an implicit/explicit source. The CLI's
  `argv::ProviderArgs` becomes that type or converts into it at one place.
  Constructors that build an empty tail (for example the test helper in
  `commands/sequence.rs`) use `Default`.
- The partitioner converts non-UTF-8 tail tokens with `to_string_lossy`, so
  the child can receive different bytes than the user typed. Child argv is
  `String`-based throughout. Refuse a non-UTF-8 tail token with a targeted
  partition error instead of changing it. Clap already refuses non-UTF-8
  passthrough for direct wrappers, so this makes the two paths consistent.

### R7. Compiled-binary coverage

Binary-level coverage of the tail is currently two dry-run cases in
`tests/l1/argv_normalization.rs`. Add L1 cases under `claudine/cli/tests` that
use a deterministic fake provider through `CliProcessFixture` and assert:

- the exact child argv for the headline command under `compose`,
  `inline-compose`, and `sequence`, with the setter-shaped value not applied to
  frontmatter;
- that the tail is present exactly once on a retry, a proxy target, a
  resume (R1), and each step of a multi-provider sequence;
- that a secret-shaped tail value (`--api-key sk-…`, `--token=…`) reaches the
  fake provider unchanged but appears in none of the INFO, dry-run, debug,
  `AGENT_PARAMS`, or correlated-error output;
- one INFO notice per distinct pair, and none under `--quiet` or `--silent`;
- one correlated report for a fixture-backed rejection, no correlation for
  auth, timeout, interruption, API, or ambiguous failures, and the exit code
  preserved.

### R8. Phase 2: research-backed enrichment (not started)

This is advisory and must never change argv.

- Extend `docs/research/agent-cli/_schema.yaml` `cli_switches[]` (today
  `{ flag, value, scope, default, description, example, notes }`) with
  `aliases`, `value_arity` (`none`, `one`, `optional`, `variadic`), and a
  normalized invocation scope (global, or exact native command paths). Keep
  `value` as a human placeholder. Arity is never inferred from it or from
  `notes`.
- Update the fleet prompt, re-research all roster providers, and validate them
  against the sidecar. An ambiguous alias is recorded as unknown, never guessed.
- Project the catalog through `claudine-gen` into each generated
  `lib/src/provider/<slug>/data.rs` as typed static metadata. The generator
  validates alias/canonical uniqueness per scope, legal arity, non-empty
  descriptions, and deterministic order. The existing drift check covers the
  output.
- A read-only lookup keyed by resolved provider and effective entrypoint
  enriches the R3 notice and R2 report:
  - known switch: `-c is Codex's --config switch (override a configuration value); forwarding to Codex.`
  - unknown switch: Claudine does not recognize it and forwards it anyway.
  - explicit tail: still opaque.

## Out of scope

- Validating, rewriting, normalizing, or expanding provider switches.
- Bare provider operands without `--`.
- Changing which switches Claudine owns.
- Completing provider switches.
- Folding native-exit signatures into the structured-stream `agent-errors`
  vocabulary.
- Provider arguments in Markdown frontmatter.

## Verification and test placement

- Partition tests stay beside `argv/partition.rs`. Classifier tests stay inline
  in `error_report.rs` until the file crosses the package's placement
  thresholds (`tests/l1/test_placement.rs`), then move to a sibling test module.
- Binary tests follow the L1 spawn contract (`CliProcessFixture`) and need no
  real provider or network.
- Run `just test`, `just test-l2`, and `just lint` from `claudine/`. R8 also
  needs `cargo run -p claudine-gen -- check`.

## Documentation

`docs/topics/argv-normalization.md`, `docs/topics/cli-pre-parsing.md`, and
the "Provider Argument Forwarding" section of `docs/topics/composition.md`
already describe the partition. Each remaining item updates those pages in the
change that lands it: resume carry-over (R1), correlated errors (R2), notice
scope and wording (R3), completion behavior (`docs/topics/completions/`, R4),
and direct-wrapper parity in the CLI reference (R5). Until R8 lands, no page
promises switch recognition.

## Acceptance criteria

The numbering matches the original spec, and `plan.md`'s traceability table
uses these numbers.

| # | Criterion | Status |
| --- | --- | --- |
| 1 | `sequence`/`compose`/`inline-compose <file> --codex -c 'model_reasoning_effort=low'` launches Codex with exactly that tail. The value is not applied as frontmatter | Implemented. Exact-argv binary proof outstanding (R7) |
| 2 | `compose <file> --codex -- -c value` consumes `--`, forwards `-c value`, and does no collision extraction | Implemented. Binary proof outstanding (R7) |
| 3 | `compose --unknown <file>` fails with file-before-tail guidance | Done |
| 4 | A shorthand setter before the tail is applied. A setter-shaped token after tail start is forwarded | Done (unit). Binary proof outstanding (R7) |
| 5 | A Claudine flag before `--` stays Claudine's even after the first provider switch. The same spelling after `--` is forwarded | Done (unit) |
| 6 | Bare provider operands require `--`. The multiple-file diagnostic remains | Done |
| 7 | The exact tail survives sequence steps, retries, proxy runs, and **resume**. Multi-provider sequences classify messages per provider without changing argv | Open: resume (R1), coverage (R7) |
| 8 | INFO is emitted once per distinct provider/tail pair **per command**, is suppressed by `--quiet`/`--silent`, and reports explicit tails as a unit | Open: scope and wording (R3) |
| 9 | INFO reveals no values. Debug, dry-run, metadata, and correlated surfaces reveal no unredacted secret | Open: binary proof (R7), correlated surface (R2) |
| 10 | A fixture-backed native rejection produces one correlated error. Auth, timeout, interruption, API, and ambiguous failures are not misattributed | Open (R2) |
| 11 | Direct wrappers share the tail descriptor, notice, classification, and reporting, with no child-argv change | Open (R5) |
| 12 | Completion never fails inside the tail, stops Claudine suggestions after `--`, and keeps pre-tail file/setter completion | Open (R4) |
| 13 | No synthetic separator can be mistaken for an authored boundary. Rule 3 is retired | Done |
| 14 | Phase 2: generated metadata recognizes Codex `-c` as `--config`, enriches the message, rejects alias/arity drift, and never changes argv | Open (R8) |
| 15 | A non-UTF-8 tail token is refused with a targeted error, never rewritten | Open (R6) |
