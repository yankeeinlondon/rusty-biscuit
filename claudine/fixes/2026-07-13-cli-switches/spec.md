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

On 2026-10-01 the token-ownership contract was revised: the split between
Claudine and the agent is now decided by switch **types** researched for every
provider, not by "everything after the first unknown switch". The revised rules
are [Token ownership](#token-ownership) and the work is R8 and R9. Until R9
lands, the code still uses the original rule, described under
[Current partition](#current-partition-replaced-by-r9).

The rest of this spec states the contract and describes only the work that
remains. [Remaining work](#remaining-work) lists the gaps, and
[Acceptance criteria](#acceptance-criteria) records which criteria already
pass. `plan.md` still applies to R1–R7 but predates R9 and treats R8 as
reporting-only, so it needs new phases for R8/R9 before they are executed. Its Phase 1 audit is
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
- **A setter after a provider switch is lost.** In
  `compose plan.md --codex -c x=y phase=2`, `phase=2` is forwarded to Codex
  instead of being applied, with no warning (`2026-07-22-setters`).

## Contract

These rules are implemented unless a [remaining-work](#remaining-work) item
says otherwise.

### Token ownership

Ownership is decided **per token, using the type of each provider switch**. The
supported order is unchanged:

```sh
claudine compose <file> [key=value ...] [CLAUDINE_OPTIONS] [AGENT_ARGS ...]
```

What Claudine always knows, with no research data:

- its own switches (`OwnedFlags::for_composition`, derived from clap);
- that a `key=value` token is a Claudine setting unless rule 3 below gives it
  to a provider switch; and
- that any other `-name` / `--name` switch belongs to a provider.

What it needs research data for is the **type** of each provider switch: does
it take no value, a string, a number, or a variadic list of strings? R8
supplies those types.

#### Rules, applied left to right

1. **Claudine switches first.** A token on Claudine's clap surface, before an
   explicit `--`, is Claudine's, with its value if it takes one. This holds
   anywhere on the line.
2. **Schema parameters always win.** If the composition file declares a
   `$schema`, a `key=value` whose key is a parameter of that schema (in any
   union arm) is always a Claudine setter, wherever it appears.
3. **A `key=value` goes to a provider only when** the token immediately
   before it is a forwarded switch in space form, and that switch takes a
   string (or a variadic list) for at least one candidate provider. Every
   other `key=value` is a Claudine setter.
4. **Each provider switch takes values according to its type** for the
   candidate providers (see [Candidate providers](#candidate-providers)):
   - **none:** takes nothing;
   - **string:** takes the next token;
   - **number:** takes the next token if it parses as a number;
   - **variadic:** takes every following token up to the next switch, except
     a `key=value` that rule 3 does not give it (a `key=value` that is not the
     first token after the switch);
   - **union** (the candidates disagree): takes the next token if at least
     one arm matches it. If one arm is "none" and another arm matches a bare
     word (not `key=value`, not a switch), such as `-c foo` when Claude types
     `-c` as none and Codex as string, the token is **ambiguous**; see
     [Ambiguity](#ambiguity).
5. **Unrecognized switches** (in no provider's data, such as a switch added in
   a newer provider release, or a typo) take the next token if it is a bare
   word, and nothing otherwise. A `key=value` after one is a setter (rule 3).
   The INFO notice names the switch as unrecognized (R3/R8).
6. **Values that start with `-` are never taken from the next token.** They
   must be attached: `--temperature=-0.5`.
7. **A bare word that no switch takes is a Claudine positional** and goes
   into the `argv` frontmatter property (see
   [Positional arguments](#positional-arguments-argv)). Provider operands
   therefore still need an explicit `--`.
8. **`--` and file order are unchanged.** An authored `--` after the file is
   consumed and starts an opaque tail that is never classified. An unowned
   switch or a `--` before the file is an error with ordering guidance.

Forwarded tokens keep their original relative order. Claudine never rewrites
a forwarded token.

```sh
# -c takes a string for Codex, so model_reasoning_effort=low is forwarded;
# phase=2 follows a value, not a switch, so it is a setter.
claudine compose plan.md --codex -c model_reasoning_effort=low phase=2

# Same line with no provider named: -c is "none" for Claude and "string" for
# Codex. Rule 3 still forwards the key=value, because one candidate takes a string.
claudine compose plan.md -c model_reasoning_effort=low phase=2

# --add-dir is variadic for Claude: a and b are forwarded, x=y is a setter.
claudine compose plan.md --claude --add-dir a b x=y
```

#### Positional arguments (`argv`)

Every bare word left over after the composition file, Claudine switches and
their values, setters, and provider switches and their values is a positional
argument. Claudine sets them, in original order, as the `argv` frontmatter
property:

```sh
claudine compose plan.md alpha --codex -c x=y beta phase=2
# argv: ["alpha", "beta"]   phase: 2   provider tail: -c x=y
```

```markdown
Arguments: {{ argv }}
```

- `argv` is an array of strings. Values are not JSON5-parsed the way setter
  values are.
- The composition file is not in `argv`, and neither is anything after an
  authored `--`.
- With at least one positional, `argv` overrides an authored `argv` the same
  way a setter does. With none, an authored `argv` is left alone.
- `argv` is reserved for positionals: it can never be set as a named
  parameter. An `argv=…` setter, or a `--set` object containing `argv`, is
  always an error, whether or not positionals are given. The error says to
  pass the values as bare words instead.
- For `sequence`, `argv` is applied to every step, like any caller setter.
- This replaces the multiple-file error for a second bare word.

This is the starting point. Declaring positional parameters (names, types,
arity) is left to a later spec.

#### Candidate providers

The candidate set is the providers whose switch types apply:

1. the provider named on the command line (`--provider`, `--codex`, …), if
   any; otherwise
2. the provider or providers named by the composition file's frontmatter
   `agent`; otherwise
3. every provider Claudine supports.

With one candidate, only that provider's types are used. With several, each
switch's type is the union across them. Per-step providers in a `sequence`
do not narrow the set; it comes from the command line and the sequence file.

Because rule 2 and the candidate set need the composition file's frontmatter,
ownership is decided after that file is resolved and its frontmatter is read.
It uses the same file resolver as composition, not a second one.

#### Ambiguity

When rule 4 finds an ambiguous token:

- if Interactive Mode is allowed (the same conditions as missing-property
  prompting), Claudine asks which agent is intended, uses that provider's
  types to decide, and runs with that provider;
- otherwise it fails before launch with an `ambiguous provider argument`
  error. The error names the switch, says which providers type it
  differently, and tells the user to name the provider or to put the agent
  arguments after `--`.

#### A switch left without its value

A switch can end up without its required value. For example, the schema
declares `phase`, so in `--codex -c phase=2` rule 2 takes `phase=2` and leaves
Codex's `-c` empty. A trailing `-c` at the end of the line has the same
problem. Claudine never lets the provider take the wrong token in that case.

If the switch requires a value for **every** candidate provider, ownership
fails before launch with an error naming the switch and the setter that took
its place. If some candidate accepts it with no value, ownership succeeds, and
the check runs again against the resolved provider before each spawn
(including each `sequence` step). It fails there if that provider requires a
value.

#### Current partition (replaced by R9)

Until R9 lands, `partition_composition_tail` uses the original rule: the first
unowned switch after the file starts an implicit agent tail, and every
non-Claudine token after it is forwarded, setter-shaped tokens included. Rules
1 and 8 above already hold. That rule causes the lost-setter bug, and R9
replaces it.

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

### R8. Research-backed switch types (not started)

This data drives token ownership (R9) and enriches the R2/R3 messages.

- Extend `docs/research/agent-cli/_schema.yaml` `cli_switches[]` (today
  `{ flag, value, scope, default, description, example, notes }`) with:
  - `aliases`;
  - `value_type`: `none`, `string`, `number`, or `variadic` (a list of
    strings);
  - `value_optional`: a value may be omitted (it types as a union with
    `none`); and
  - a normalized invocation scope (global, or exact native command paths).

  Keep `value` as a human placeholder. The type is never inferred from it or
  from `notes`.
- Update the fleet prompt, re-research all roster providers, and validate them
  against the sidecar. A switch whose type research cannot establish is
  recorded without a type and treated as unrecognized (rule 5), never guessed.
- Project the catalog through `claudine-gen` into each generated
  `lib/src/provider/<slug>/data.rs` as typed static metadata. The generator
  validates alias/canonical uniqueness per scope, legal types, non-empty
  descriptions, and deterministic order. The existing drift check covers the
  output.
- One lookup, keyed by provider and effective entrypoint, serves both
  ownership (R9) and messages. It can also return the union type for a
  candidate set. In messages:
  - known switch: `-c is Codex's --config switch (override a configuration value); forwarding to Codex.`
  - unrecognized switch: Claudine does not recognize it and forwards it anyway.
  - explicit tail: still opaque.

### R9. Type-aware token ownership

Replace the current partition with the [Token ownership](#token-ownership)
rules. This depends on R8 data.

- Ownership needs the CLI provider and the composition file's frontmatter
  (`$schema` parameters and `agent`). Resolve and read the file first,
  through composition's own resolver, then decide the remaining tokens. File
  identification stays as it is today: the first bare non-setter token, which
  must come before any provider switch.
- The candidate set, union types, ambiguity prompt or error, and
  missing-value checks are as specified in the contract. The per-spawn
  missing-value check uses the resolved provider for each launch, retry,
  proxy target, resume, and `sequence` step.
- The owned surface stays `OwnedFlags::for_composition`. Switch types come
  only from the R8 lookup; there is no handwritten list.
- The ownership result keeps the R6 typed tail descriptor.
- Update `looks_like_setter`'s doc comment: setter shape alone no longer
  decides ownership. Rules 2 and 3 do.
- Collect leftover bare words into `argv` in
  `parse_composition_positionals` (`cli/src/commands/compose/setters.rs`),
  which today rejects a second bare word as a second file. Remove that error.
  It comes from this parser, not from clap, despite the partition's comment
  calling it "clap's existing multiple-file diagnostic".

## Out of scope

- Declaring positional parameters beyond the `argv` array (names, types,
  arity, schema integration). A later spec covers it.
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
describe the current partition and must be rewritten for the R9 rules when
R9 lands. Each remaining item updates those pages in the
change that lands it: resume carry-over (R1), correlated errors (R2), notice
scope and wording (R3), completion behavior (`docs/topics/completions/`, R4),
direct-wrapper parity in the CLI reference (R5), and type-aware ownership with
its ambiguity and missing-value errors (R9). R9 also documents the `argv`
property in `composition.md` (Positional Arguments) and
`frontmatter-properties.md`. Until R8 lands, no page promises
switch recognition.

## Acceptance criteria

The numbering matches the original spec, and `plan.md`'s traceability table
uses these numbers.

| # | Criterion | Status |
| --- | --- | --- |
| 1 | `sequence`/`compose`/`inline-compose <file> --codex -c 'model_reasoning_effort=low'` launches Codex with exactly that tail. The value is not applied as frontmatter | Implemented. Exact-argv binary proof outstanding (R7) |
| 2 | `compose <file> --codex -- -c value` consumes `--`, forwards `-c value`, and does no collision extraction | Implemented. Binary proof outstanding (R7) |
| 3 | `compose --unknown <file>` fails with file-before-tail guidance | Done |
| 4 | A shorthand setter is applied wherever it appears, unless rule 3 gives it to the string switch directly before it | Revised 2026-10-01. Open (R9). Today a setter after tail start is forwarded |
| 5 | A Claudine flag before `--` stays Claudine's even after the first provider switch. The same spelling after `--` is forwarded | Done (unit) |
| 6 | Bare provider operands require `--` | Done |
| 7 | The exact tail survives sequence steps, retries, proxy runs, and **resume**. Multi-provider sequences classify messages per provider without changing argv | Open: resume (R1), coverage (R7) |
| 8 | INFO is emitted once per distinct provider/tail pair **per command**, is suppressed by `--quiet`/`--silent`, and reports explicit tails as a unit | Open: scope and wording (R3) |
| 9 | INFO reveals no values. Debug, dry-run, metadata, and correlated surfaces reveal no unredacted secret | Open: binary proof (R7), correlated surface (R2) |
| 10 | A fixture-backed native rejection produces one correlated error. Auth, timeout, interruption, API, and ambiguous failures are not misattributed | Open (R2) |
| 11 | Direct wrappers share the tail descriptor, notice, classification, and reporting, with no child-argv change | Open (R5) |
| 12 | Completion never fails inside the tail, stops Claudine suggestions after `--`, and keeps pre-tail file/setter completion | Open (R4) |
| 13 | No synthetic separator can be mistaken for an authored boundary. Rule 3 is retired | Done |
| 14 | Generated metadata recognizes Codex `-c` as `--config` with type `string`, enriches the message, and rejects alias/type drift | Open (R8) |
| 15 | A non-UTF-8 tail token is refused with a targeted error, never rewritten | Open (R6) |
| 16 | A `$schema` parameter is always a Claudine setter, including directly after a provider switch | Open (R9) |
| 17 | `-c model_reasoning_effort=low phase=2` forwards `-c model_reasoning_effort=low` and applies `phase=2`, both with `--codex` and with no provider named | Open (R9) |
| 18 | A variadic switch takes tokens up to the next switch and never takes a `key=value` that is not its first value | Open (R9) |
| 19 | The candidate set narrows to the CLI provider, then frontmatter `agent`, then all providers. A single candidate uses only its own types | Open (R9) |
| 20 | An ambiguous "none or string" bare word prompts for the agent in Interactive Mode, and otherwise fails with a targeted error | Open (R9) |
| 21 | A switch left without a required value fails before launch for all candidates, or before the spawn of a resolved provider that requires one. The provider never takes the wrong token | Open (R9) |
| 22 | An unrecognized switch takes a following bare word, never a `key=value`, and the notice names it as unrecognized | Open (R8, R9) |
| 23 | Leftover bare words become the `argv` frontmatter array in order, excluding the file and anything after `--`. They override an authored `argv`. An `argv=…` setter or `--set` key is always an error. A second bare word is no longer a multiple-file error | Open (R9) |
