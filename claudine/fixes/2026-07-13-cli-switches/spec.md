---
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-13
---

# Composition rejects provider CLI switches instead of proxying them

## Problem

Composition subcommands (`compose`, `inline-compose`, `sequence`) reject any
CLI switch that is not part of Claudine's own surface instead of forwarding it
to the underlying agent CLI. Direct wrappers (`claudine codex ...`) already
forward provider arguments; composition never adopted that launch contract.

Observed in the wild:

```sh
💻❯ sequence docs/research/agent-errors/_fleet.md -y --codex -c 'model_reasoning_effort="low"'
error: unexpected argument '-c' found

  tip: to pass '-c' as a value, use '-- -c'

💻❯ sequence docs/research/agent-errors/_fleet.md -y --codex -- -c 'model_reasoning_effort="low"'

Error: expected at most one file reference, but got multiple: docs/research/agent-errors/_fleet.md, -c
```

`-c model_reasoning_effort="low"` is Codex's `--config` short form. The
expected contract is that a switch Claudine does not own is proxied to the
agent, as it is on the direct-wrapper surface.

> **Reader's note (inline review, 2026-07-13):** the draft proposed routing
> unknown switches by greedily consuming their next non-flag token. That is
> not deterministic: `compose --unknown file.md` can consume the required
> file as the switch value, while `compose file.md --boolean key=value` can
> consume a real shorthand setter. This revision makes the first unowned
> switch start an agent-owned **tail** after the file has been identified.
> Correct forwarding therefore does not depend on researched arity. Phase 2
> metadata enriches reporting only; it never changes token ownership.
>
> The draft also referenced the generated `agent-errors` vocabulary for
> post-spawn correlation. That vocabulary classifies structured stream error
> events and expressly excludes `cli/src/output/error_report.rs`. Native CLI
> argument rejection must use a typed native-process classifier at the CLI
> reporting boundary instead; the two vocabularies must not be conflated.

## Reproduction

```sh
claudine sequence <file> --codex -c 'model_reasoning_effort=low'
claudine compose <file> --codex --some-codex-flag
claudine inline-compose <file> --codex --some-codex-flag=value
```

All three fail during Claudine argument parsing, before the agent is launched.

## Root cause

Two independent parsing decisions cause the failure:

1. **No provider-argument bucket.** `ComposeArgs`, `InlineComposeArgs`, and
   `SequenceArgs` expose one greedy positional (`num_args = 1..`) containing
   one file reference plus `key=value` setters. Composition uses strict clap
   parsing, so an unknown switch such as `-c` is rejected. Direct wrappers use
   a lenient command tree plus a trailing passthrough positional.
2. **Provider values enter the composition grammar.** Even if clap accepts the
   tokens, `parse_composition_positionals` classifies `-c` as a second file
   reference and `model_reasoning_effort=low` as a Claudine setter. A naive
   "accept unknown flags" change can therefore launch with the wrong prompt
   state even when it does not error.

There is no provider-argument channel in `CompositionExecutionRequest`, and
the composition wrapper currently builds child argv only from Claudine-owned
flags, provider-profile injections, and MCP injections.

## Design decisions

### One ownership pass before clap

Composition argv is partitioned into three categories before the normal clap
parse:

1. **Claudine-owned options.** The root and active composition command's clap
   definitions are the source of truth, including aliases and value arity.
   These options and their values remain in the Claudine argv.
2. **Composition positionals.** Before an agent tail starts, non-switch tokens
   retain the existing one-file-plus-setters grammar and may appear in either
   order.
3. **Agent tail.** After the composition file has been identified, the first
   switch not owned by Claudine starts an agent-owned tail. Every non-Claudine
   token from that point onward is forwarded in original order. In particular,
   setter-shaped tokens in the tail are agent values, not frontmatter
   overrides.

The ownership pass produces two explicit vectors: normalized Claudine argv
for clap, and the provider tail for execution. It must not reconstruct the
provider tail from clap matches or from `std::env::args()` later in the run.

This yields an intentional ordering rule: the composition file must precede
the first implicit provider switch. Shorthand setters intended for Claudine
should also precede the provider tail; `--set` remains available as an
unambiguous Claudine-owned option anywhere before an explicit `--` boundary.
If an unowned switch appears before the file, fail before clap with a targeted
error that shows the supported order:

```sh
claudine compose <file> [key=value ...] [CLAUDINE_OPTIONS] [AGENT_ARGS ...]
```

This is preferable to guessing whether the first non-switch token is a file,
a provider-switch value, a provider subcommand, or a prompt operand.

### Explicit `--` remains the escape hatch

An explicit `--` is optional. When present after the composition file, it
starts the agent tail immediately. The delimiter itself is consumed by
Claudine and is not inserted into the child's argv; every following token is
opaque and is never extracted as a Claudine flag, even when it collides with
one.

An explicit `--` before the file is an error because the opaque tail cannot
contain the composition source. This keeps file resolution independent from
provider argv.

### Claudine owns collisions before `--`

Before an explicit boundary, a token matching Claudine's clap surface always
belongs to Claudine, even after an implicit agent tail has started. This
preserves the direct wrapper's established precedence for flags such as
`-i`, `-m`, `-o`, `-q`, `-y`, `--model`, `--silent`, and `--help`. A user who
intends a colliding native switch must place it after `--`.

The collision set must be derived from the clap command definitions and
covered by a drift test; do not maintain a second handwritten list. Generate
the user-facing collision reference from the same data and document that
`--help` before the boundary is Claudine help while `-- --help` is agent help.

### Routing never depends on researched arity

Both known and unknown switches are forwarded from the already-partitioned
tail. Phase 2 metadata may identify switch names, aliases, descriptions, and
arity for reporting, but it must not regroup, drop, or reorder argv. This
keeps stale research from changing execution behavior and handles provider
operands, variadic values, negative values, and setter-shaped values without
heuristics.

### Forwarded arguments are request-level launch state

The provider tail is threaded through `CompositionExecutionRequest` as a
distinct field, separate from MCP arguments and Claudine-owned flags. It
seeds the provider-profile argv pipeline at the same point as direct-wrapper
passthrough, before Claudine applies entrypoint, model, structured-transport,
system-prompt, MCP, and prompt-delivery requirements.

Claudine's required transport and safety injections retain their existing
precedence. A user tail must not disable structured output, prompt delivery,
sandboxing, or another Claudine-owned behavior accidentally. Known conflicts
should use the existing provider-profile validation/reporting path rather than
silently relying on a provider's first-wins or last-wins behavior.

The same tail applies to:

- every `sequence` step, including steps that resolve to different providers;
- every fresh retry or proxy attempt; and
- resume attempts, inserted through the provider profile's resume-aware argv
  assembly rather than the current transport-only carry-forward allowlist.

For a multi-provider sequence, classification and status wording are resolved
against each step's actual provider. Forwarding remains token-for-token the
same for every step; a provider that rejects the tail is handled by the
correlated error contract below.

### Composition and direct wrappers share classification and reporting

The tokenizer is composition-specific because only composition has a file and
setter grammar. After token ownership is resolved, both composition and direct
wrappers use the same provider-tail descriptor, status renderer, metadata
lookup, redaction, and correlated-error path. Direct-wrapper launch behavior
must remain unchanged.

## Required behavior

### Forwarding

1. A non-Claudine switch after the composition file starts the implicit agent
   tail; the switch and every provider-owned token following it reach the
   provider in original order.
2. A setter-shaped provider value such as `model_reasoning_effort=low` remains
   in the agent tail and is never applied to frontmatter.
3. A literal `--` after the file starts an opaque agent tail. Its contents are
   not scanned for Claudine collisions, setters, or switch/value grouping.
4. Bare provider operands with no preceding provider switch require explicit
   `--`; otherwise they retain the existing second-file error.
5. The provider tail survives sequence iteration, loop retries, proxy runs,
   and resume reconstruction.
6. `--dry-run` never launches the provider, but its metadata report includes a
   redacted provider-argument tail so the proposed launch can be audited.
7. Provider arguments are distinct from `mcp_extra_args`; both contributions
   are preserved and provider-profile ordering remains valid.

### Communication

Before launch, render an INFO status through `TerminalRenderable` components.
It is suppressed by both `--quiet` and `--silent`, matching the existing
status-output contract.

- **Phase 1, implicit tail:** report once that a provider argument tail
  beginning with the first switch is being forwarded to the resolved
  provider. Because the switch catalog is not compiled yet, do not claim that
  the switch itself is unknown.
- **Phase 1, explicit boundary:** report once that an opaque argument tail is
  being forwarded. Do not claim individual switch classification.
- **Phase 2, known switch:** identify its canonical name and concise
  description, for example: `-c is Codex's --config switch (override a
  configuration value); forwarding to Codex.`
- **Phase 2, unknown switch:** state only that Claudine does not recognize it
  and is forwarding it.

Sequence and loop execution must not emit the same INFO line on every attempt.
Emit at most once per distinct `(provider, provider-tail)` pair per top-level
command. Correlated errors are never suppressed by quiet/silent modes.

INFO messages display switch names only and strip any `=value` suffix. Every
surface that renders more of the tail (provider diagnostic excerpts, debug
traces, dry-run output, and `AGENT_PARAMS`-style metadata) must pass it through
the existing `redact_sensitive_args` policy and must never expose an
unredacted sensitive value. The actual child argv remains unchanged.

### Refuses-to-start correlation

Forwarding an invalid switch can cause the provider to reject its argv. When
all of the following hold, render a correlated provider-argument error instead
of the generic provider-failure report:

- this launch had a non-empty provider tail;
- the child exited non-zero;
- the typed native CLI classifier returns `ArgumentRejected` from the
  provider's captured process exit (`exit_code` plus the captured stdout/stderr
  tails); and
- no stronger failure classification, such as authentication, missing binary,
  timeout, interruption, or API failure, won first.

Refactor `classify_native_cli_error` so classification returns a typed cause
before rendering. Do not consult `stream/providers/vocabulary.rs`: that table
classifies structured semantic error events, not provider process argv
rejections. Keep the initial argument-rejection signatures narrow and backed
by positive and collision fixtures; an uncertain result must fall back to the
generic provider error rather than misattribute the failure.

The correlated report must:

- identify the redacted forwarded switch names, or identify an opaque tail
  without attempting to parse it;
- include a redacted excerpt of the provider's own diagnostic;
- distinguish a Phase 2 known switch from one unknown to Claudine; and
- say "likely caused by the forwarded arguments" rather than asserting
  causality when the provider only supplied a generic parse error.

Correlation changes presentation only. Preserve the provider's exit code,
termination state, lifecycle `failure`/`finalize` behavior, and retry policy,
and render exactly one final error report.

## Phasing

### Phase 1 - deterministic forwarding, generic INFO, and correlation

- Add the composition ownership pass before clap. Keep clap authoritative for
  all Claudine-owned options, values, conflicts, and diagnostics.
- Replace Rule 3's synthetic-separator coupling with the explicit partitioned
  result. A synthetic `--` must never become an agent boundary, and an authored
  `--` must remain distinguishable from any internal clap protection.
- Thread the provider tail and its implicit/explicit source through every
  `CompositionExecutionRequest` construction and the sequence request-copy
  path.
- Seed composition child argv at the same provider-profile stage used by
  direct-wrapper passthrough, including retry/proxy/resume reconstruction.
- Add generic INFO rendering, deduplication, redaction, and dry-run reporting.
- Introduce the typed native CLI `ArgumentRejected` classification and the
  correlated error renderer.
- Update the completion command tree and the `__complete` context classifier
  so composition remains lenient after an agent tail starts. Claudine
  completions stop at an explicit boundary; provider-switch completion is not
  added in Phase 1.
- Update `argv-normalization.md`, `cli-pre-parsing.md`, CLI reference/help, and
  the argv module docs. Remove obsolete claims that Rule 3's separator is the
  only owner of trailing composition values.

Phase 1 alone makes the reported command work safely.

### Phase 2 - research-backed enrichment

- Extend `agent-cli` research `cli_switches[]` with first-class machine fields
  rather than inferring behavior from prose:
  - `aliases` (including short forms such as `-c`),
  - `value_arity` (`none`, `one`, `optional`, `variadic`), and
  - a normalized invocation scope sufficient to disambiguate aliases whose
    meaning changes by provider subcommand.
- Retain the current human-facing `value` placeholder for documentation, but
  never derive arity from strings such as `<FILE>...` or from `notes`.
- Re-research/backfill all providers and update the topic sidecar before
  enabling runtime enrichment. Ambiguous or out-of-scope aliases fall back to
  the unknown tier; they never guess.
- Project the switch catalog through `claudine-gen` into each generated
  `lib/src/provider/<slug>/data.rs` and expose it as typed, static provider
  metadata. Add generator validation for canonical/alias uniqueness within an
  invocation scope, valid arity, non-empty descriptions, and deterministic
  ordering. The existing generated-data drift check covers the output.
- Use the resolved provider and effective native entrypoint to enrich INFO and
  correlation messages. Metadata lookup remains read-only with respect to the
  already-partitioned argv.

## Out of scope

- Validating provider-switch values. Claudine forwards; the provider validates.
- Rewriting, normalizing, or expanding provider switches. Recognition does not
  alter argv.
- Adding bare provider operands to the implicit grammar. Use explicit `--`.
- Changing which switches Claudine owns.
- Provider-switch shell completion in Phase 1.
- Folding native CLI rejection signatures into the structured-stream
  `agent-errors` vocabulary.
- Adding provider arguments to Markdown frontmatter. This specification is
  limited to invocation-level CLI forwarding.

## Verification and test placement

- Keep tokenizer/partition tests beside the argv module. Cover file/setter
  ordering, implicit and explicit tails, Claudine flags after tail start,
  collisions after `--`, help behavior, non-UTF-8 pass-through at the
  normalizer boundary, and missing-file diagnostics.
- Keep small metadata/classifier tests inline; split them into sibling test
  files when the production file or test module crosses the package's
  architecture thresholds.
- Add compiled-binary integration coverage under `claudine/cli/tests` using a
  deterministic fake provider executable. Assert exact argv, stdout/stderr
  separation, INFO suppression, redaction, exit-code preservation, and both
  positive and negative correlation cases. Do not require a real provider or
  network access.
- Use nextest through the package `just test`/`just test-l2` recipes; do not
  introduce `cargo test` instructions.

## Acceptance criteria

1. `claudine sequence <file> --codex -c 'model_reasoning_effort=low'`
   launches Codex with the exact tail `-c model_reasoning_effort=low`, and the
   value is not applied as frontmatter. The same holds for `compose` and
   `inline-compose`.
2. `claudine compose <file> --codex -- -c value` consumes Claudine's `--`,
   forwards `-c value`, and performs no collision extraction from the tail.
3. `claudine compose --unknown <file>` fails with the targeted file-before-tail
   guidance rather than guessing that `<file>` is a switch value.
4. A genuine shorthand setter before the provider tail is applied; a
   setter-shaped token after tail start is forwarded.
5. A Claudine flag before an explicit boundary remains Claudine-owned even
   when it follows the first provider switch. The same spelling after `--` is
   forwarded.
6. Bare provider operands require `--`; the existing multiple-file diagnostic
   remains for two ordinary non-setter positionals.
7. The exact provider tail survives sequence steps, loop retries, proxy runs,
   and resume launches. A multi-provider sequence classifies messages against
   each resolved provider without changing argv.
8. Generic INFO output is emitted once per distinct provider/tail pair and is
   suppressed by `--quiet` and `--silent`. Explicit opaque tails are reported
   as a unit.
9. INFO output reveals no provider argument values, and debug, dry-run,
   metadata, and correlated-error surfaces reveal no unredacted sensitive
   values or recognizable secret tokens.
10. An invalid forwarded switch with a fixture-backed native argv-rejection
    signature produces one correlated error naming the redacted switch and
    provider diagnostic. Auth, timeout, interruption, API, and ambiguous
    failures are not misattributed.
11. Direct wrappers use the shared reporting/classification path without any
    change to the child argv they forward today.
12. Completion remains non-failing while entering provider args, stops
    Claudine suggestions after explicit `--`, and preserves existing file and
    setter completion before the tail.
13. Rule 3 documentation and tests are replaced or revised so no synthetic
    separator can be mistaken for an authored provider boundary.
14. **Phase 2:** generated provider metadata recognizes Codex `-c` as
    `--config`, enriches its message, rejects schema alias/arity drift, and has
    no effect on the exact forwarded argv.
