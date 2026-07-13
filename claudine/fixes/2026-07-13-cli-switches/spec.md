---
reviewed: false
---
# Composition rejects provider CLI switches instead of proxying them

## Problem

Composition subcommands (`compose`, `inline-compose`, `sequence`) reject any
CLI switch that isn't part of Claudine's own surface, instead of forwarding it
to the underlying agent CLI. The wrapper subcommands (`claudine codex …`)
already forward unknown flags to the child; composition never adopted that
behavior.

Observed in the wild:

```sh
💻❯ sequence docs/research/agent-errors/_fleet.md -y --codex -c 'model_reasoning_effort="low"'
error: unexpected argument '-c' found

  tip: to pass '-c' as a value, use '-- -c'

💻❯ sequence docs/research/agent-errors/_fleet.md -y --codex -- -c 'model_reasoning_effort="low"'

Error: expected at most one file reference, but got multiple: docs/research/agent-errors/_fleet.md, -c
```

`-c model_reasoning_effort="low"` is Codex's `--config` short form (a valid,
research-documented switch). The user's expectation — **any switch Claudine
does not own is proxied to the agent** — holds for the direct wrappers but was
never wired into composition.

## Reproduction

```sh
# Any non-Claudine switch on a composition subcommand:
claudine sequence <file> --codex -c 'model_reasoning_effort=low'
claudine compose  <file> --codex --some-codex-flag
```

Both fail at the CLI layer before the agent is ever launched.

## Root cause

Two distinct failures, both rooted in composition using **strict** parsing
where the wrappers use **lenient** parsing:

1. **No passthrough bucket.** `ComposeArgs` / `InlineComposeArgs` /
   `SequenceArgs` declare a single greedy positional
   (`#[arg(value_name = "ARG", num_args = 1..)]`) that is documented as "one
   file reference plus `key=value` setters." Composition subcommands are
   parsed *strictly* (no `ignore_errors(true)`), so an unknown flag like `-c`
   is a hard clap error. The wrapper subcommands avoid this with
   `ignore_errors(true)` + a `trailing_var_arg`, `allow_hyphen_values`
   `passthrough: Vec<String>` field (`cli/src/commands/wrap/flags.rs`).

2. **The value is setter-shaped.** Even past clap, `parse_composition_positionals`
   (`cli/src/commands/compose/setters.rs`) classifies each positional token as
   file-ref vs `key=value` setter. `-c` has no `=`, so it is treated as a
   *second file reference* → the "expected at most one file reference" error.
   Worse, its value `model_reasoning_effort=low` **does** match the setter
   grammar (`^[A-Za-z_][A-Za-z0-9_-]*=`), so a naive forward-everything fix
   would silently swallow the agent's config value as a Claudine frontmatter
   override. Correct grouping of a switch with its value is therefore
   load-bearing, not cosmetic.

There is no provider-passthrough channel anywhere in the composition path:
`CompositionExecutionRequest` (`lib/src/composition/types.rs`) has no field for
it, and `commands/wrap/composition/mod.rs` builds the child argv entirely from
Claudine-owned flags plus MCP injection.

## Design decisions (ratified)

- **No `--` required.** The user never has to type a separator. Claudine
  parses what it recognizes into its own slots and forwards the rest.
  An explicit `--` is still **honored** as a POSIX "everything after this is
  opaque agent args, stop classifying" boundary (mirrors the wrapper's
  `find_passthrough_dash_boundary`), but it is optional.
- **Two-camp classification of forwarded switches:**
  1. **Known switch** — not a Claudine switch, but recognized from
     `agent-cli` fleet research for the target provider. Claudine knows its
     name/description and its value arity.
  2. **Unknown switch** — unrecognized. Still forwarded; the user is told with
     a lower-information message.
- **Unknown-switch arity: greedy.** For an unknown switch, if the next token
  is not itself flag-shaped, consume it as the switch's value and forward
  both. (Known switches use their researched arity.)
- **Scope: composition + wrapper unified.** Composition adopts lenient
  forwarding, and the classify + INFO enrichment applies to *both*
  composition and the direct wrappers (the wrappers forward silently today).
- **Phased rollout:** correct forwarding first, research-backed enrichment
  second (see Phasing).

## Required behavior

### Forwarding (both surfaces)

1. A switch Claudine does not own is forwarded verbatim to the agent's argv,
   in the position Claudine assembles for provider flags (before prompt
   delivery, so it lands in the harness-loop base argv).
2. A forwarded switch that takes a value carries its value:
   - **Known switch** — arity comes from research (`cli_switches[].value`).
   - **Unknown switch** — greedy: the next non-flag-shaped token is treated as
     the value. `--flag=value` is always a single token.
3. Value tokens consumed by a forwarded switch are **removed from** the
   file/setter positional stream, so a setter-shaped value
   (`model_reasoning_effort=low`) is never misapplied as a Claudine
   frontmatter override.
4. An explicit `--` forces everything after it to be forwarded opaquely with
   no classification.
5. Claudine's own flags continue to win: a token that *is* a Claudine switch
   is never forwarded (existing precedence; document the collision list).

### Communication (INFO status)

For each forwarded switch, emit an INFO-level status message (suppressed under
`--silent`, consistent with existing status output):

- **Known switch:** name it and describe it, e.g.
  `-c is codex's --config switch (override a configuration value); forwarding to codex.`
- **Unknown switch:** lower-information, e.g.
  `unrecognized switch --foo is not used by Claudine; forwarding to codex.`

Exact wording is an implementation detail; the information tiers (known vs
unknown) are the contract.

### Refuses-to-start correlation (post-spawn)

Because unknown switches are always forwarded, a genuinely invalid switch will
make the agent exit during its own argument parsing. Today that renders as a
generic provider failure with no link to the forwarded switch.

When **all** of the following hold, Claudine surfaces a *correlated* error
rather than a bare provider failure:

- Claudine forwarded one or more switches this run, **and**
- the child exited non-zero with a signature classified as an
  argument-parse rejection (gate on the exit-source payload + the
  provider-errors error vocabulary — `exit_code`, `stderr_tail`; do **not**
  attribute auth/missing-binary/other failures to forwarded switches).

The correlated message must:

- name the forwarded switch(es),
- quote the agent's own stderr (already captured as `summary.stderr_text`),
- state plainly that Claudine did not recognize the switch either (when it was
  an unknown switch), so a stale-research false-negative reads as "likely
  cause" rather than a Claudine bug.

This is the honest safety net for the trade-off Ken called out: forwarding
unknown switches means a valid-but-unrecognized switch still works, at the cost
of the agent possibly rejecting a truly invalid one — and when it does, the
failure explains itself.

## Phasing

### Phase 1 — correct forwarding + generic INFO + correlation

- Composition adopts the wrapper's lenient parse model (bring the passthrough
  bucket + `ignore_errors(true)` to `compose`/`inline-compose`/`sequence`, or
  an equivalent that keeps clap as the primary parser).
- Thread forwarded switches through `CompositionExecutionRequest` into the
  composition child argv; keep the file/setter positional split correct
  (values consumed by forwarded switches removed from it).
- **Resolve the argv-normalizer coupling.** Rule 3 in
  `cli/src/argv/` currently inserts a *synthetic* `--` to protect trailing
  setters. That mechanism must not collide with the new forwarding (a
  synthetic `--` must not misroute setters into the agent, and an explicit
  user `--` must be honored as a passthrough boundary). Update the rule and
  its documented invariants (`argv-normalization.md`, `cli-pre-parsing.md`)
  and keep the pass-through tests green.
- INFO messages use the **generic** tier only (known-vs-unknown distinction
  not yet available at runtime — see Phase 2). Every forwarded switch still
  gets a "forwarding to {agent}" INFO.
- Post-spawn refuses-to-start correlation, gated on exit-source + error
  vocabulary.

Phase 1 alone makes the reported command work.

### Phase 2 — research-backed enrichment

- Project `agent-cli` research `cli_switches[]` into the generated
  `lib/src/provider/<slug>/data.rs` (same pipeline shape as the
  agent-errors → `stream/providers/vocabulary.rs` projection): a compiled,
  per-provider switch table carrying at least `flag`, short-form, value arity,
  and a short description. This requires `claudine-gen` (`gen/`) + `emit.rs`
  changes and a regen; it is drift-checked in CI like the rest of `data.rs`.
- Short-forms are currently recorded in research `notes` prose (e.g.
  "Short form: -c"). Phase 2 must give short-form a first-class home — either a
  new `cli_switches[].short_flag` schema field (preferred; re-research/backfill)
  or a documented extraction — so `-c` resolves to `--config` deterministically.
- Known-switch arity from the table replaces the greedy heuristic for
  recognized switches (greedy remains the fallback for unknown switches).
- INFO messages gain the **known** tier (name + description); the correlation
  message can state "`--foo` is a known codex switch" vs "unknown to Claudine."

## Out of scope

- Validating agent switch *values* (e.g. that `model_reasoning_effort` accepts
  `low`) — Claudine forwards; the agent validates.
- Rewriting or normalizing agent switches (short↔long expansion beyond what's
  needed to recognize/describe them). Forwarded switches reach the agent
  byte-for-byte.
- Any change to which switches Claudine *owns*. This spec forwards the
  complement of the existing Claudine surface; it does not add or remove
  Claudine flags.
- MCP injection argv (`mcp_extra_args`) — unaffected; forwarded switches are a
  separate, additive argv contribution.

## Acceptance criteria

1. `claudine sequence <file> --codex -c 'model_reasoning_effort=low'`
   (no `--`) launches Codex with `-c model_reasoning_effort=low` in its argv,
   and `model_reasoning_effort` is **not** applied as a Claudine frontmatter
   override. Same for `compose` and `inline-compose`.
2. An explicit `--` still works and forwards its tail opaquely with no
   classification.
3. A setter-shaped value after a forwarded switch
   (`--config model_reasoning_effort=low`) is grouped with the switch, not
   parsed as a Claudine setter; a genuine setter with no preceding
   value-taking switch is still applied as a Claudine override.
4. Each forwarded switch emits an INFO status naming the target agent;
   suppressed under `--silent`.
5. Forwarding an invalid switch that the agent rejects produces a correlated
   error that names the forwarded switch and quotes the agent's stderr —
   distinct from Claudine's generic provider-failure surface, and gated so
   non-arg-parse failures are **not** mis-attributed to forwarding.
6. Direct wrappers exhibit the same INFO/classification behavior for forwarded
   switches (scope unification), with existing wrapper passthrough tests still
   green.
7. Argv-normalizer Rule 3 changes land with updated docs
   (`argv-normalization.md`, `cli-pre-parsing.md`) and green pass-through
   tests; no synthetic `--` misroutes a setter to the agent.
8. **Phase 2:** `cli_switches` is projected into `data.rs` under the codegen
   drift guard; `-c` resolves to Codex's `--config` and the INFO/correlation
   messages carry the switch name + description.
```
