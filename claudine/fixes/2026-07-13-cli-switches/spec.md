---
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-01
review_iterations: 3
completed: true
refreshed_on: 2026-10-01
human_review: false
message_to_agent: |-
    All seven phases are implemented; read "## Phase 7" in implementation-log.md. Every
    acceptance criterion (1-29) is Done. Facts a reviewer or follow-up agent needs:
    - Completion (R4) asks ownership through claudine::composition::owner_of_last_argument
      with the cursor word as the last argument (cli/src/completion/engine/ownership.rs).
      It offers nothing for the agent's word, on any OwnershipError, on an unreadable file or
      $schema once a provider switch follows the file, and after an authored `--`
      (CompletionTarget::Declined, which now also suppresses clap's fallback for wrappers).
      It never prompts and reads the file only when a provider switch follows it.
    - is_value_bearing_flag is gone; the classifier skips Claudine option values through
      argv::OwnedFlags::for_composition().consumes_next. A Claudine option's value now
      classifies as Other (clap) instead of the setter-name completer.
    - provider::lookup_candidates / CandidateSwitch were removed (unused since Phase 6);
      dispatch-inventory.json was regenerated (1784 -> 1780 sites).
    - Native Windows cross-check of claudine-cli: 3 failures, all "batch file arguments are
      invalid" from a .cmd provider stub given a multi-line prompt (loop_gate_ambient x2,
      pr_flow_rehearsal x1); none carries a provider tail and Phase 7 touched no spawn
      path. Not proven to predate the branch; fix forward on CI's Windows leg if they recur.
    - Do not move this fix to _completed; the author does that after review.
implemented: true
---

# Composition forwards provider CLI switches to the agent

## Status (2026-10-02)

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
are [Token ownership](#token-ownership) and the work is [researched switch metadata](#r8-research-backed-switch-types-not-started) and [type-aware ownership](#r9-type-aware-token-ownership). Status 2026-10-02: both have
landed (Phases 5 and 6); [Current partition](#current-partition) describes
the rule they replaced. Completion (R4) landed on 2026-10-02 (Phase 7): shell
completion reads the words after the file through the same ownership function
and offers nothing where ownership gives the word to the agent or cannot
decide. Every acceptance criterion is now Done; the implementation is ready
for review.

The rest of this spec states the contract and describes only the work that
remains. [Remaining work](#remaining-work) lists the gaps, and
[Acceptance criteria](#acceptance-criteria) records which criteria already
pass.

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

The **provider tail** is the ordered list of arguments Claudine sends to the
underlying agent CLI. **Ownership** determines whether each input belongs to
Claudine or that agent. **Preflight** checks a planned launch before an agent
starts. A **scalar** switch takes one value; a **variadic** switch takes several.

This section describes the intended behavior. The type-aware ownership rules,
`argv` positionals, and resolved-provider checks are **planned**, not current
behavior. The status section and acceptance table distinguish implemented
behavior from remaining work.

**Reader's note:** the ownership revision intentionally changes two existing
behaviors: setters after a provider switch can return to Claudine, and a second
bare word becomes a positional argument instead of a multiple-file error.
Callers that intend those tokens for the provider must put them after `--`.
Direct wrappers retain their existing parsing and child argument order; they
share reporting, not composition's new ownership rules.

### Token ownership

Ownership is decided **per token, using the type of each provider switch**. The
composition file must precede provider switches:

```sh
claudine compose <file> [CLAUDINE_INPUTS_OR_PROVIDER_SWITCHES ...] [-- OPAQUE_AGENT_ARGS ...]
```

After the file, Claudine options, setters, positionals, and implicit provider
switches may be interleaved. The displayed order is an example, not a required
ordering of those groups.

What Claudine always knows, with no research data:

- its own switches ([`OwnedFlags::for_composition`](../../cli/src/argv/partition.rs), the claudine-cli option inventory derived from clap);
- that a `key=value` token is a Claudine setting unless rule 3 below gives it
  to a provider switch; and
- that any other `-name` / `--name` switch belongs to a provider.

What it needs research data for is the **type** of each provider switch: does
it take no value, a string, a number, or a variadic list of strings? [researched switch metadata](#r8-research-backed-switch-types-not-started)
supplies those types.

#### Rules, applied left to right

1. **Claudine switches first.** A token on Claudine's clap surface, before an
   explicit `--`, is Claudine's, with its value if it takes one. This holds
   anywhere on the line. Schema and provider rules never reclaim a value
   already consumed by a Claudine option. Preserve clap's validation of owned
   options and values.
2. **Schema parameters always win.** If the composition file declares a
   `$schema`, a `key=value` whose key is a parameter of that schema (in any
   union arm) is always a Claudine setter before `--`. The reserved key
   `argv` is also Claudine-owned and produces the positional-guidance error,
   even directly after a provider switch; after `--` it is opaque provider data.
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
     first token after the switch). A setter or Claudine-owned switch ends
     that value run; later bare words do not reconnect to the earlier switch;
   - **union** (the candidates disagree): takes the next token if at least
     one arm matches it. If one arm is "none" and another arm matches a bare
     word (not `key=value`, not a switch), such as `-c foo` when Claude types
     `-c` as none and Codex as string, the token is **ambiguous**; see
     [Ambiguity](#ambiguity).
5. **Unrecognized switches** (in no provider's data, such as a switch added in
   a newer provider release, or a typo) take the next token if it is a bare
   word, and nothing otherwise. A `key=value` after one is a setter (rule 3).
   The INFO notice names the switch as unrecognized ([forwarding notices](#r3-fix-the-info-notice)/[researched switch metadata](#r8-research-backed-switch-types-not-started)).
6. **Exact switch spellings and researched attached forms only.** Claudine
   does not expand provider short-switch clusters. An attached value
   (`-cfoo`, `--config=x`) stays one token and takes nothing from the next
   token. Recognize
   `--name=value` only by an exact canonical name or alias. Recognize a short
   attached form only when the exact two-character switch is researched as
   value-bearing and permits attachment; never interpret `-abc` as `-a` plus
   `bc` when `-a` takes no value. Exact whole-token matches take precedence.
   Any other token is unrecognized (rule 5).
7. **Values that start with `-` are never taken from the next token.** They
   must be attached: `--temperature=-0.5`.
8. **A bare word that no switch takes is a Claudine positional** and goes
   into the `argv` frontmatter property (see
   [Positional arguments](#positional-arguments-argv)). Provider operands
   therefore still need an explicit `--`.
9. **`--` and file order are unchanged.** An authored `--` after the file is
   consumed and starts an opaque tail that is never classified. Any later
   literal `--` is forwarded as a provider token; only the first boundary is
   consumed. An unowned
   switch or a `--` before the file is an error with ordering guidance.

Forwarded tokens keep their original relative order. Claudine never rewrites
a forwarded token. Each value run is contiguous in the original arguments:
removing a Claudine token must not cause a later token to become a value for an
earlier provider switch. For example, `--codex -c phase=2 x=y`, where the
schema declares `phase`, leaves `-c` without a value; it must fail rather than
silently attaching `x=y`.

Additional value rules:

- Required scalar switches take exactly one value. Optional scalar switches
  take the adjacent eligible token when present, and may take none. A provider
  that permits an optional value only in attached form must record that rule;
  it must not consume a separate bare word.
- Variadic switches need a researched minimum value count. Optionality
  alone does not establish how many values are required. An attached first
  value is self-contained under rule 6; Claudine does not extend it with later
  bare words.
- For ownership, a number is a finite decimal with an optional fraction and
  exponent; hexadecimal, `NaN`, and infinity do not qualify. A separate
  negative value remains subject to rule 7. This determines ownership only;
  provider-specific ranges and enums remain the provider's responsibility.
- An empty argument is a valid string value and stays empty in the child
  arguments. An empty attached value also stays attached and empty.
- A provider missing a switch entry, or having an entry with unknown type,
  contributes **unknown**, not **none**, to a candidate comparison. Apply rule
  5 for that candidate. If known and unknown candidates would consume a bare
  word differently, use the same ambiguity handling as other disagreements.
- Type-aware checks apply only to the implicit part of the tail. Tokens after
  an authored `--` are never locally checked for ownership, value count, or
  switch support. A native rejection still uses correlated reporting.

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
  an error before `--`, whether or not positionals are given. The error says to
  pass the values as bare words instead.
- For `sequence`, `argv` is applied to every step, like any caller setter.
  It remains caller input through retries and proxy adoption; it is not
  recomputed from provider arguments. Use the existing caller-overlay path
  rather than adding a separate propagation mechanism.
- The effective array participates in ordinary schema validation. If a
  document declares `argv`, its declaration must accept the supplied string
  array. Inline composition must not persist caller positionals merely because
  they were applied as an overlay.
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
It uses the same file resolver as composition, not a second one. This early
read must use composition's existing file-reference and source-relative schema
resolution. It must not run templates, shell expansion, lifecycle actions,
provider discovery, or network research. Help and version requests must still
work without opening a composition file; completion must remain read-only.

Use the existing claudine library
[`parse_selection_hints_from_frontmatter`](../../lib/src/composition/hints.rs)
to interpret literal `agent` hints without inventing stricter selection rules.
An unresolved expression cannot narrow candidates; retain all supported
providers and let ordinary preparation resolve the actual provider. Invalid
hints retain the existing diagnostic behavior.

##### The authored snapshot

Ownership is decided from the file **as authored**, before any caller
overrides are applied. This keeps parsing a single read-only pass with no
circular dependency on the setters it is classifying.

- The snapshot holds the literal `$schema` reference or inline declaration,
  resolved relative to the source document, and the literal `agent` hints.
  Explicit `--provider` wins regardless of where it appears before `--`.
- Caller overrides do not change ownership. A setter or `--set` that changes
  `agent` or `$schema` takes effect for preparation, provider selection, and
  validation as usual, but the tokens were already classified against the
  authored values. For example, in
  `compose plan.md agent=codex -c foo`, `agent=codex` does not narrow the
  candidate set for `-c`; name the provider with `--codex` instead.
- Expressions in `agent` or `$schema` are not evaluated for ownership. An
  unresolved `agent` leaves every supported provider as a candidate.
- **SimplifiedSchema** contributes its declared property names, from every
  union arm.
- **Raw JSON Schema** contributes the statically declared top-level property
  names, including union branches, read through the existing schema loader.
  Its property types are never used to type provider switches.
- If schema parameter names cannot be established from the snapshot, a
  setter-shaped token that rule 3 would give to a provider fails before
  launch: Claudine cannot tell a schema parameter from the switch's value.
  The error says to put the provider value after `--`, or to pass the setter
  with `--set`. Choosing an agent would not resolve it, so there is no
  prompt.
- A schema read failure is an error for execution, not permission to guess
  ownership. Completion offers no suggestions instead.
- Final schema validation still runs against the effective frontmatter after
  overrides, unchanged.

Ownership is fixed once per invocation. A retry, changed file, proxy target,
or sequence step may select a different provider, but cannot reassign the
caller's setters or positionals to that provider. Recheck the preserved
implicit assignments against the actual launch entrypoint instead.

#### Ambiguity

When candidate providers disagree about whether or how many adjacent bare
words a switch consumes, Claudine must not silently choose a meaning. This
includes scalar-versus-variadic disagreements, not just none-versus-string.
Setter-shaped values keep the explicit precedence rules above.

- **Interactive:** Claudine asks which agent the arguments are intended for
  and uses that provider's types to decide who owns the token. The prompt says
  it is resolving how the arguments are read. The answer decides **ownership
  only**: it does not select the provider for the run, a `sequence` step, or a
  proxy target, which each still resolve their own provider. A provider that
  disagrees with the decision is caught by the
  [resolved-provider check](#resolved-provider-check).
- **Prompt eligibility** reuses Interactive Mode's environment conditions
  without its missing-property condition: `prompt_for_missing` is `true`,
  stdin and stderr are TTYs, and `--silent` is not set. Completion never
  prompts.
- **Otherwise** execution fails before launch with an
  `ambiguous provider argument` error naming the switch and the providers'
  different interpretations. It suggests naming the provider or placing
  provider arguments after `--`. Completion offers no suggestions for the
  ambiguous slot.

#### Resolved-provider check

Ownership with several candidates is decided against a union of types, so the
provider that finally runs can disagree with it in two ways:

- **Missing value.** A switch that provider types as taking a value got none.
  For example, the schema declares `phase`, so in `--codex -c phase=2` rule 2
  takes `phase=2` and leaves Codex's `-c` empty. A trailing `-c` at the end of
  the line has the same problem.
- **Extra value.** A switch that provider types as taking nothing was given
  one. For example, `-c model_reasoning_effort=low` with no provider named
  forwards the value because Codex takes a string. If the run resolves to
  Claude, where `-c` takes nothing, Claude would receive a stray operand.

Either way Claudine fails before the spawn with an error naming the switch,
the token, and the resolved provider. It never lets a provider take the wrong
token or receive a stray one **when researched metadata establishes the
mismatch**. Unknown switch types defer to the provider; absence from the
catalog is not grounds to reject a forwarded switch. Explicit tails bypass
this check.

Check minimum value counts, required scalar values, and researched attachment
rules without validating provider-specific value contents. A candidate union
must preserve each switch's original value assignments so the check can name
which assignment disagrees; a flattened vector alone loses that information.
Use the actual provider profile and native command path, including resume,
when selecting metadata. Known command-specific differences may fail locally;
unresearched differences remain eligible for a native correlated error.

When the check runs:

- **During ownership,** if the mismatch holds for **every** candidate (for
  example a switch that takes a value for all of them gets none).
- **During preflight,** for every launch whose provider can be known before
  anything runs: the command itself, and each `sequence` step whose provider
  resolves statically. A sequence therefore fails before step 1, not after
  step 2.
- **Before each spawn,** for a provider known only at launch time (a retry,
  a proxy target, a resume, or a step whose provider is decided at runtime).

#### Current partition

Until [type-aware ownership](#r9-type-aware-token-ownership) lands, `partition_composition_tail` uses the original rule: the first
unowned switch after the file starts an implicit agent tail, and every
non-Claudine token after it is forwarded, setter-shaped tokens included. Rules
1 and 9 above already hold. That rule causes the lost-setter bug, and [type-aware ownership](#r9-type-aware-token-ownership)
replaces it.

### Launch threading

The tail is request-level launch state on the claudine library's
[`CompositionExecutionRequest`](../../lib/src/composition/types.rs),
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
  the claudine-cli [`redact_sensitive_args`](../../cli/src/commands/wrap/env/sanitize.rs)
  policy. The child receives the original tokens.
- Correlated errors are never suppressed by quiet or silent modes. Status
  and diagnostic text go to stderr; stdout retains its existing pipeable data.
- For notices, strip attached values using the ownership metadata, not just
  `=value`: `-csecret` must display only `-c`. Unrecognized short tokens cannot
  be safely split, so describe them without echoing the full token. Explicit
  tokens remain opaque and are never listed as switch names.
- Native diagnostic excerpts are prose, not an argument vector. Mask them with
  the shared claudine secret recognizer and mask echoes of values recognized
  in the original tail, including a value printed without its flag. Escape
  terminal control characters and rendering markup before display. Redaction
  is heuristic; the requirement covers recognized secrets, not every possible
  string a caller might consider private.
- The new descriptor must have a redacted `Debug` representation. Raw tails,
  ownership records, and notice keys must never reach tracing through derived
  debug output. Preserve the unmodified values in memory only for launching.

### Native-exit classification

The claudine-cli [`NativeCliCause` and `classify_native_cli_cause`](../../cli/src/output/error_report.rs)
classify a provider's process exit for user-facing reports. They are
deliberately separate from the structured-stream vocabulary in
`lib/src/stream/providers/vocabulary.rs` and must stay that way.

## Remaining work

### R1. Resume carries the forwarded tail

The claudine-cli helper
[`append_resume_passthrough_args`](../../cli/src/commands/wrap/resume.rs)
currently copies a fixed
allowlist (`--json`, `--verbose`, `--print-logs`, `--approve`/`--no-approve`,
`--output-format`, `--format`, `--output-last-message`, `--log-level`, `--mode`)
from the base argv into the resume argv. `harness_orch/launch.rs` calls it on
every resume attempt, so any forwarded switch outside that list is dropped.

Required:

- The resume argv receives the request's provider tail exactly once, at the
  position the profile's resume entrypoint expects. Read the tail from the
  typed request state ([typed tail state](#r6-one-typed-tail-descriptor-and-no-silent-byte-changes)), not by pattern-matching the base argv.
- The transport/safety allowlist keeps its job for Claudine's own injections.
  The tail is not added to it as a second mechanism. Feed that carry-over from
  arguments identified as Claudine injections, not from a base vector that
  already contains the provider tail: a user-supplied `--json` or `--format`
  must not be copied once by the allowlist and again by tail append. Preserve
  authored repetitions and order; do not deduplicate user tokens by spelling.
- If a provider's resume entrypoint does not accept the tail (for example a
  subcommand that rejects root switches), handle it through the
  resolved-provider check or [native error correlation](#r2-correlate-argument-rejection-on-every-launch-path), rather than
  silently dropping it. Claudine does not filter the tail per entrypoint. When
  researched metadata proves an implicit assignment invalid for resume, the
  resolved-provider check fails before spawn; otherwise native rejection uses
  correlated reporting. Explicit tails always reach the native entrypoint.
- `harness_orch/session_key.rs` compares the **canonical** argv and documents
  the allowlist as an intentional drop. Update that comment. The tail is
  invocation-fixed, so it must not make a resume look incompatible.

### R2. Correlate argument rejection on every launch path

The claudine-cli report builder
[`AgentErrorReport::correlated_with_forwarded_tail`](../../cli/src/output/error_report.rs)
is currently unused and marked `#[allow(dead_code)]`.
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
  composition attempt. Reuse existing capture bounds; do not retain complete
  streams or route provider stdout to stderr just to classify it. Capture an
  interactive stream only where the current launch path already observes it;
  unavailable evidence leaves the failure unclassified.
- One report builder that both paths call exactly once per terminal failure.
  It produces the correlated report only when the launch had a non-empty tail,
  the exit was non-zero, and the classifier returned `ArgumentRejected`.
  Otherwise it preserves the existing failure category and remediation. If
  a rejection names a switch, correlate only when that name belongs to the
  forwarded tail; a rejection naming Claudine's own injected switch stays a
  generic native error. An unnamed, fixture-backed parser rejection may use
  the cautious tail wording.
- The correlated report names the redacted switch names, or says the tail was
  opaque. It includes a redacted excerpt of the provider's diagnostic and says
  "likely caused by the forwarded arguments". Its wording must not claim
  Claudine failed to recognize the switch (that claim waits for [researched switch metadata](#r8-research-backed-switch-types-not-started)).
- Do not duplicate the native excerpt as a raw stderr echo and again inside
  the report. A handled retry may keep existing attempt diagnostics; it must
  not emit a terminal correlated report before recovery is exhausted. An
  explicit tail containing only operands still qualifies as a non-empty tail.
- Presentation only: exit code, termination state, lifecycle
  `failure`/`finalize`, and retry policy are unchanged.
- Remove the `dead_code` allowances that this wiring makes stale.

### R3. Fix the INFO notice

The claudine-cli notice implementation in
[`provider_args.rs`](../../cli/src/commands/wrap/composition/provider_args.rs) needs these changes:

- **Scope.** Deduplication uses a process-wide `static ANNOUNCED`. Move it into
  per-command execution state owned by the top-level command and threaded to
  the launch. It holds at most one notice per distinct `(provider, tail)` pair,
  and nothing leaks between invocations or between tests in one process.
  Include the explicit-boundary position in the key so equal token vectors
  with different ownership do not share misleading notices. Parallel sequence
  tasks share command-owned notice state and atomically claim each notice;
  do not hold its lock while rendering or launching.
- **Wording.** "not recognized by Claudine" is forbidden until [researched switch metadata](#r8-research-backed-switch-types-not-started) can know it.
  Implicit tail: `Forwarding provider arguments to Codex: -c`. Explicit tail:
  `Forwarding an opaque argument tail to Codex (passed after --).`
- **Location.** The notice moves out of `wrap/composition/` into a module both
  launch paths can use ([direct-wrapper reporting](#r5-direct-wrappers-share-reporting)).

### R4. Completion uses the owned surface

The claudine-cli completion helper
[`is_value_bearing_flag`](../../cli/src/completion/engine/tokens.rs) maintains a
handwritten list of value-bearing switches. Its doc comment points at
`crate::argv::COMPOSITION_FLAGS_WITH_VALUE`, which `2c7f98dcf` deleted.

Required:

- Completion's cursor scan uses `OwnedFlags::for_composition`, the partition's
  surface, so the two cannot drift. Delete the list and the stale comment.
- Completion classifies the tokens before the cursor with the same
  ownership function as the partition ([type-aware ownership](#r9-type-aware-token-ownership)), including [researched switch metadata](#r8-research-backed-switch-types-not-started) types and the
  file's `$schema` and `agent`. Until [type-aware ownership](#r9-type-aware-token-ownership) lands it uses the current partition.
- Completion never fails. When ownership cannot decide (an ambiguous token,
  an unreadable file, a missing-value or extra-value error), it offers no
  suggestions instead of erroring or prompting.
- Claudine suggestions stop after an authored `--`. File and setter
  completion keep working wherever a setter or positional can appear.
  Completing provider switches stays out of scope.

### R5. Direct wrappers share reporting

Today direct wrappers share only `classify_native_cli_cause`. They forward
their passthrough positional with no INFO notice and no correlation. After this
work both launch paths use the same tail descriptor ([typed tail state](#r6-one-typed-tail-descriptor-and-no-silent-byte-changes)), notice ([forwarding notices](#r3-fix-the-info-notice)),
redaction, and correlated report ([native error correlation](#r2-correlate-argument-rejection-on-every-launch-path)). Direct-wrapper child argv must not
change. `tests/l1/wrap_direct_argv.rs` is the guard for that.

### R6. One typed tail descriptor, and no silent byte changes

- The claudine library
  [`CompositionExecutionRequest`](../../lib/src/composition/types.rs) and
  claudine-cli [`SharedComposeArgs`](../../cli/src/commands/compose/mod.rs) carry the
  tail as parallel fields, `provider_args: Vec<String>` and
  `provider_args_explicit: bool`. Replace both with one typed descriptor in the
  library, holding the ordered args and the index where an authored `--`
  starts the opaque suffix (`None` when there is no boundary). An index of
  zero means fully explicit; an index equal to the argument count preserves
  an authored empty suffix. The implicit prefix retains switch/value
  assignments for later checks. The CLI's
  `argv::ProviderArgs` becomes that type or converts into it at one place.
  For example, `-c x=y -- --native z` contains a typed implicit prefix
  `-c x=y` and an opaque suffix `--native z`; neither can be described by one
  Boolean without losing information. Render a value-free notice for the
  prefix plus an opaque-tail summary, as one notice. Constructors that build
  an empty tail (for example the test helper in `commands/sequence.rs`) use
  `Default`. Direct wrappers populate the same descriptor from their existing
  passthrough parsing, without running composition ownership checks.
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
  resume ([resume forwarding](#r1-resume-carries-the-forwarded-tail)), and each step of a multi-provider sequence;
- that a secret-shaped tail value (`--api-key sk-…`, `--token=…`) reaches the
  fake provider unchanged but appears in none of the INFO, dry-run, debug,
  `AGENT_PARAMS`, or correlated-error output;
- one INFO notice per distinct pair, and none under `--quiet` or `--silent`;
- one correlated report for a fixture-backed rejection, no correlation for
  auth, timeout, interruption, API, or ambiguous failures, and the exit code
  preserved.

### R8. Research-backed switch types (not started)

This data drives token ownership ([type-aware ownership](#r9-type-aware-token-ownership)) and enriches the [native error correlation](#r2-correlate-argument-rejection-on-every-launch-path)/[forwarding notices](#r3-fix-the-info-notice) messages.

- Extend `docs/research/agent-cli/_schema.yaml` `cli_switches[]` (today
  `{ flag, value, scope, default, description, example, notes }`) with:
  - `aliases`;
  - `value_type`: `none`, `string`, `number`, `variadic` (a list of
    strings), or `unknown`;
  - `value_optional`: whether a scalar value may be omitted;
  - a variadic minimum count and researched acceptance of space, equals, and
    short-attached forms; and
  - a normalized invocation scope (global, or exact native command paths).

  Keep `value` as a human placeholder. The type is never inferred from it or
  from `notes`.
- Update the fleet prompt, re-research all roster providers, and validate them
  against the sidecar. A switch whose type research cannot establish is
  recorded with explicit `unknown` type and a described evidence gap, and
  treated as unrecognized (rule 5), never guessed. Record observed provider
  version and evidence for value consumption and attachment forms. Repeatable
  scalar switches remain scalar; repeated use is not a variadic value list.
- Follow the existing [research contract standard](../../../.claude/skills/claudine/research-contracts.md):
  define switch and invocation-scope records as named types in the topic's
  `_types.yaml`, with closed enums and a description for every property.
  Increment `schema_revision`; run both shape and relation checks from the
  fleet's success lifecycle, and use revision-aware refresh instead of allowing
  a recently dated document to skip the changed contract. Pilot one provider
  before refreshing the fleet. Respect roster entries that pause research;
  every compiled provider still needs metadata or an explicit unknown gap.
- Invocation scope records use exact native command paths, with an empty path
  for the root entrypoint and an explicit global marker. Existing labels such
  as `config` and `model_selection` are topic categories, not command paths;
  do not reinterpret them mechanically. Lookup includes global and applicable
  exact-path entries. Conflicting canonical names or aliases in that effective
  set fail generation instead of depending on insertion order.
- Project the catalog through `claudine-gen` into each generated
  `lib/src/provider/<slug>/data.rs` as typed static metadata. Shared vocabulary
  belongs in the existing `claudine-catalog-types` crate; do not duplicate it
  in the CLI or hand-edit generated provider files. The generator
  validates alias/canonical uniqueness per scope, legal types, non-empty
  descriptions, and deterministic order. The existing drift check covers the
  output.
- One lookup, keyed by provider and effective entrypoint, serves both
  ownership ([type-aware ownership](#r9-type-aware-token-ownership)) and messages. It can also return the union type for a
  candidate set. In messages:
  - known switch: `-c is Codex's --config switch (override a configuration value); forwarding to Codex.`
  - unrecognized switch: the compiled catalog has no established type for it
    at this entrypoint, and Claudine forwards it anyway. This does not claim
    that the installed provider rejects it.
  - explicit tail: still opaque.

### R9. Type-aware token ownership

Replace the current partition with the [Token ownership](#token-ownership)
rules. This depends on [researched switch metadata](#r8-research-backed-switch-types-not-started) data.

- Ownership needs the CLI provider and the composition file's frontmatter
  (`$schema` parameters and `agent`). Resolve and read the file first,
  through composition's own resolver, then decide the remaining tokens. File
  identification stays as it is today: the first bare non-setter token, which
  must come before any provider switch.
- The candidate set, union types, ambiguity prompt or error, and
  resolved-provider check are as specified in the contract. The check runs
  at ownership, at preflight (including every statically resolvable
  `sequence` step), and before each spawn whose provider is known only at
  launch.
- Ownership is one shared function. The partition and completion ([shared completion ownership](#r4-completion-uses-the-owned-surface)) both
  call it; neither keeps its own rules.
- The owned surface stays `OwnedFlags::for_composition`. Switch types come
  only from the [researched switch metadata](#r8-research-backed-switch-types-not-started) lookup; there is no handwritten list.
- The ownership result keeps the [typed tail descriptor](#r6-one-typed-tail-descriptor-and-no-silent-byte-changes).
- Update the claudine-cli [`looks_like_setter`](../../cli/src/argv/mod.rs)
  doc comment: setter shape alone no longer
  decides ownership. Rules 2 and 3 do.
- Collect leftover bare words into `argv` in
  the claudine-cli parser
  [`parse_composition_positionals`](../../cli/src/commands/compose/setters.rs),
  which today rejects a second bare word as a second file. Remove that error.
  It comes from this parser, not from clap, despite the partition's comment
  calling it "clap's existing multiple-file diagnostic".

## Out of scope

- Declaring positional parameters beyond the `argv` array (names, types,
  arity, schema integration). A later spec covers it.
- Validating provider-specific values or whether an unknown switch exists.
  The planned ownership/value-count checks are explicitly in scope; rewriting,
  normalizing, or expanding forwarded provider tokens is not.
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
- For implementation changes, run `just test`, `just test-l2`, and `just lint`
  from `claudine/`. Research/generator changes also need
  `cargo run -p claudine-gen -- check` and the affected generator/catalog-types
  L1 checks through their canonical area recipes. Tests use nextest.
- Add focused ownership fixtures for optional and variadic counts, attached
  and empty values, aliases, mixed implicit/explicit tails, unknown catalog
  entries, numeric boundaries, and a Claudine token interrupting a value run.
  Assert both ownership and exact forwarded tokens.
- Include schema references relative to the source document, a schema union,
  a templated `agent`, command-scoped resume metadata, and unresolved file or
  schema reads. Help without a valid file must still succeed. Completion must
  distinguish an unfinished value slot from a terminal missing-value error;
  it offers no Claudine suggestions while the cursor belongs to a provider.
- Include rejection on stdout, an injected-switch rejection, explicit
  operand-only rejection, repeated authored switches through resume, and
  secrets in short-attached arguments and echoed diagnostic prose.
- L1 binaries use `CliProcessFixture`'s isolated home, working directory,
  provider stubs, and dry-run audio policy. Use Test Toolkit helpers for shared
  environment state. Any L2 terminal sessions stay in the background and
  never take focus; no real provider or network is needed for these checks.
- No performance spike is required: generated lookup is in-process, and the
  early document/schema read reuses composition's resolution. Measure only
  if implementation reveals a concrete new cost, using one host and a quick
  sample unless the cost is specific to an OS. Do not add a benchmark gate
  for this parsing change.

## Documentation

Update `docs/topics/argv-normalization.md`, `docs/topics/cli-pre-parsing.md`,
and the Provider Argument Forwarding section of `docs/topics/composition.md`
alongside the behavior each implementation change lands. Describe resume
carry-over, notice wording and command scope, redaction, and correlated errors.
Update `docs/topics/completions/` for shared completion ownership and the CLI
reference for direct-wrapper reporting.

When type-aware ownership lands, document the authored snapshot and its
effect on overridden `agent`/`$schema`, schema precedence, ambiguity,
entrypoint checks, and escape through `--`. Document the `argv` string array in
`composition.md` and `frontmatter-properties.md`, including its reserved setter
name and propagation through sequence steps and proxy runs. Update the claudine
skill where the shared descriptor or parsing workflow changes. Until the
catalog lands, no page promises switch recognition. These topic pages describe
behavior directly and must not link back to a dated feature or fix.

## Acceptance criteria

Criteria 1–24 retain their numbering; 25–29 were added later. Status links
name the corresponding work in [Remaining work](#remaining-work).

| # | Criterion | Status |
| --- | --- | --- |
| 1 | `sequence`/`compose`/`inline-compose <file> --codex -c 'model_reasoning_effort=low'` launches Codex with exactly that tail. The value is not applied as frontmatter | Done |
| 2 | `compose <file> --codex -- -c value` consumes `--`, forwards `-c value`, and does no collision extraction | Done |
| 3 | `compose --unknown <file>` fails with file-before-tail guidance | Done |
| 4 | A shorthand setter is applied wherever it appears, unless rule 3 gives it to the string switch directly before it | Done (type-aware ownership) |
| 5 | A Claudine flag before `--` stays Claudine's even after the first provider switch. The same spelling after `--` is forwarded | Done (unit) |
| 6 | Bare provider operands require `--` | Done |
| 7 | The exact tail survives sequence steps, retries, proxy runs, and **resume**. Multi-provider sequences classify messages per provider without changing argv | Done |
| 8 | INFO is emitted once per distinct provider/tail pair **per command**, is suppressed by `--quiet`/`--silent`, and reports explicit tails as a unit | Done |
| 9 | INFO reveals no values. Debug, dry-run, metadata, and correlated surfaces reveal no unredacted secret | Done |
| 10 | A fixture-backed native rejection produces one correlated error. Auth, timeout, interruption, API, and ambiguous failures are not misattributed | Done |
| 11 | Direct wrappers share the tail descriptor, notice, classification, and reporting, with no child-argv change | Done |
| 12 | Completion uses the shared ownership function, never fails (offering nothing when ownership cannot decide), stops Claudine suggestions after `--`, and keeps file/setter completion | Done |
| 13 | No synthetic separator can be mistaken for an authored boundary. Rule 3 is retired | Done |
| 14 | Generated metadata recognizes Codex `-c` as `--config` with type `string`, enriches the message, and rejects alias/type drift | Done |
| 15 | A non-UTF-8 tail token is refused with a targeted error, never rewritten | Done |
| 16 | A declared `$schema` parameter before `--` is always a Claudine setter, including directly after a provider switch | Done |
| 17 | `-c model_reasoning_effort=low phase=2` forwards `-c model_reasoning_effort=low` and applies `phase=2`, both with `--codex` and with no provider named | Done |
| 18 | A variadic switch takes a contiguous run up to the next switch or setter, and never takes a `key=value` that is not its first value | Done |
| 19 | The candidate set narrows to the CLI provider, then frontmatter `agent`, then all providers. A single candidate uses only its own types | Done |
| 20 | Disagreement over bare-word consumption never silently chooses ownership. When eligible, Claudine asks which agent is intended and the answer decides ownership only; otherwise execution fails with guidance. Completion never prompts | Done |
| 21 | A researched mismatch in an implicit switch's value count fails before spawn; explicit tails remain opaque. The check runs at ownership when it holds for every candidate, at preflight for every statically known launch and `sequence` step, and before each spawn otherwise | Done |
| 22 | An unrecognized switch takes a following bare word, never a `key=value`, and the notice names it as unrecognized | Done |
| 23 | Leftover bare words become the `argv` frontmatter array in order, excluding the file and anything after `--`. They override an authored `argv`. An `argv=…` setter or `--set` key before `--` is an error. A second bare word is no longer a multiple-file error | Done |
| 24 | Exact names and aliases take precedence. Only researched value-bearing switches accept attached forms; unknown clusters are not split. Attached tokens remain unchanged | Done |
| 25 | Mixed implicit and explicit tails preserve the authored boundary; only the implicit prefix receives ownership checks. Resume preserves repeated authored switches exactly once | Done |
| 26 | Missing/unknown metadata does not mean a no-value switch. Optional values, variadic minimum counts, and different candidate consumption lengths follow the documented rules | Done |
| 27 | Help needs no readable file. Ownership reads are side-effect free and use existing source-relative resolution; completion never prompts or launches a provider | Done |
| 28 | A rejection naming only an injected switch is not attributed to the tail; stdout rejection and operand-only explicit rejection are reported once, with echoed recognized secrets masked | Done |
| 29 | Ownership uses the authored snapshot: a setter or `--set` that changes `agent` or `$schema` does not change ownership, raw JSON Schema contributes only top-level property names, unestablished names make a contested setter-shaped token an error, and final validation still uses the effective frontmatter | Done |
