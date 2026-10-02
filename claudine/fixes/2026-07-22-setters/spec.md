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
area: claudine
status: proposed
created: 2026-07-22
packages:
    - claudine-cli
related:
    - 2026-07-13-cli-switches
depends-on:
    - 2026-07-13-cli-switches
review_iterations: 0
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-01
---

# A shorthand setter after a provider switch is forwarded to the agent

## Problem

A shorthand setter (`key=value`) supplies a caller override for a prompt's
frontmatter. Currently, when it appears after a provider switch that Claudine
does not own, it is passed to the agent instead. Nothing tells the user. The
prompt renders with the key's default, and the agent receives a stray argument.
This affects `compose`, `inline-compose`, and `sequence` in claudine-cli.

```sh
# phase=2 is applied: it comes before the first provider switch
claudine compose plan.md phase=2 --codex -c 'model_reasoning_effort="medium"'

# phase=2 is sent to Codex: it comes after the first provider switch
claudine compose plan.md --codex -c 'model_reasoning_effort="medium"' phase=2
```

## Root cause and current status

In claudine-cli,
[`partition_composition_tail`](../../cli/src/argv/partition.rs) separates
Claudine's arguments from those sent to the provider. Its current implementation
treats the first unowned switch as the start of a provider tail and forwards
every later non-Claudine token, setter-shaped ones included. It cannot distinguish
these tokens by shape alone: in `-c model_reasoning_effort=low`, the `key=value`
is Codex's configuration value, while `phase=2` after it is a caller setter.
The distinction requires knowing how many values `-c` consumes.

As reviewed on 2026-10-01, the source still has that behavior. The existing
`reported_command_forwards_config_switch` unit test in the same file protects
forwarding of Codex's configuration value, but does not cover a later setter.
The routing changes below are planned, not implemented by this review.

## Dependency and design decision

This fix depends on `2026-07-13-cli-switches`, which defines the shared
[Token ownership contract](../2026-07-13-cli-switches/spec.md#token-ownership).
That contract covers researched provider-switch types, argument classification,
and checks against the provider that actually launches. This spec adds
setter-specific requirements and regression checks; it does not introduce a
second parser, switch catalog, warning mechanism, or implementation workstream.
Its cases should be implemented with the dependency's researched switch metadata
and type-aware ownership work.

**Reader's note:** an earlier revision proposed a warning while leaving routing
unchanged. Applying setters correctly addresses the silent wrong-prompt problem,
so the type-aware design replaces that warning. This intentionally changes
routing before an authored `--`. Callers who intend `key=value` as provider
data can put it after `--`, where it remains untouched. Before this fix lands,
placing setters before provider switches or supplying them with `--set` avoids
the defect.

## Setter ownership

A **candidate provider** is one whose researched switch types are considered
when classifying arguments: the explicitly named provider, otherwise the
providers named by the document's literal `agent`, otherwise all supported
providers. The dependency defines ambiguity handling and later checks against
the provider selected for each launch.

Apply its precedence rules without exceptions specific to this fix. Unless a
row says otherwise, the fixture has no schema claiming the provider value key
`x`; a row with no provider hint also has no authored `agent` hint.

| Input before an authored `--` | Required result |
| --- | --- |
| `--codex -c x=y phase=2` | Forward `-c x=y`; apply `phase=2` as a setter. |
| `-c x=y phase=2`, with no provider hint | Forward `-c x=y` because a candidate takes a string; apply `phase=2`. A resolved provider whose `-c` takes no value must fail before launch. |
| `--codex --yolo phase=2` | Claudine owns `--yolo`; apply `phase=2`. This tests Claudine option ownership, not a provider-switch type. |
| A provider-only switch researched as taking no value, followed by `phase=2` | Forward only the switch; apply `phase=2`. Choose a fixture from the generated metadata, rather than treating an owned Claudine switch as provider data. |
| `--claude --add-dir a b phase=2` | Forward `--add-dir a b`; apply `phase=2`. A setter ends the switch's contiguous value run. |
| `--claude --add-dir x=y phase=2`, with neither key declared by the schema | Forward `--add-dir x=y`; apply `phase=2`. Only the first adjacent setter-shaped value belongs to this variadic switch. |
| `--codex -c phase=2`, with `phase` declared in the authored `$schema` | Keep `phase=2` as a setter; reject the missing value for `-c` before launch. |
| An unrecognized provider switch followed by `phase=2` | Apply `phase=2`. Unknown switch types do not justify consuming a setter-shaped token. |
| `--codex --config=x=y phase=2` | Preserve the attached provider value as one token; apply `phase=2`. This uses the researched Codex `--config` spelling. |

Schema precedence applies to declared top-level parameter names in every union
arm, including statically declared raw JSON Schema properties. It uses the
**authored snapshot**: the document and its schema as read before caller overrides.
A setter changing `agent` or `$schema` changes subsequent preparation, not
classification already performed. External schemas use composition's existing
source-relative resolver. If schema names cannot be established, use the
dependency's fail-before-launch behavior for a contested setter-shaped value;
do not silently treat it as provider data or execute expressions to decide.
Schema membership never steals a value already consumed by a Claudine option.

The original argument adjacency must survive classification. For example,
with `phase` in the schema, `--codex -c phase=2 x=y` leaves `-c` without a
value. Removing `phase=2` must not cause `x=y` to become that value. The error
must name the switch, the conflicting setter key where applicable, and the
provider, with guidance to supply a separate provider value or place intentional
provider arguments after `--`. Diagnostics follow the dependency's value-redaction
contract; they need not expose the setter's value.

An authored `--` remains the escape hatch:

```sh
# Neither setter-shaped token is applied to frontmatter.
claudine compose plan.md --codex -- -c x=y phase=2
```

The first boundary after the file is consumed; all later tokens are provider
data, even schema keys or Claudine flag spellings. Missing-value checks apply
to the implicit provider arguments, not to this opaque portion.

## Preserve setter semantics

Routing a token back to Claudine must feed the existing override path in
claudine-cli's
[`parse_composition_positionals`](../../cli/src/commands/compose/setters.rs),
which collects caller setters, rather than creating a separate merge path.

- Keep the existing key grammar: an ASCII letter or `_` first, then ASCII
  letters, digits, `_`, or `-`. A dotted key such as `foo.bar=baz` is not a
  shorthand setter; use the dependency's ordinary argument rules for it.
- Split at the first `=`. Parse the remainder as JSON5, falling back to a
  string; an empty remainder is an empty string. Thus `count=3`,
  `enabled=true`, `phase=`, and `label=a=b` retain their existing meanings.
  Values assigned to provider switches remain unchanged strings.
- Preserve input order: the last shorthand setter for a repeated key wins,
  including when occurrences straddle provider arguments. Shorthand setters
  override matching `--set` keys regardless of their placement. Sequence's
  reserved per-step overlay keys retain their established precedence.
- Keep caller override provenance and file-reference anchoring through the
  existing path. Routing must not turn a caller value into a document-authored
  value. Sequence steps and retry, resume, and proxy launches retain the
  original ownership decision and caller inputs; they never reclassify a
  setter as provider data.
- `inline-compose` does not persist caller setters merely because they were
  applied as invocation overrides. The agent's intentional file edits retain
  the existing inline-composition behavior.
- The dependency reserves `argv` for bare positional arguments. Before `--`,
  `argv=...` and a `--set` object containing `argv` remain errors with guidance
  to pass bare words. This fix adds no exception.

## Reproduction

This manual example uses a macOS/Linux shell. Automated checks must use portable
Rust fixtures on macOS, Linux, native Windows, and WSL2.

`--dry-run` launches no agent. For this shell-free fixture, it writes the composed
body to stdout and frontmatter plus metadata to stderr without modifying the
file. General dry runs can execute document composition shell spans; the
reproduction deliberately has none.

```sh
dir=$(mktemp -d)
cat > "$dir/plan.md" <<'EOF'
---
phase: 1
---
Phase {{ phase }}
EOF

# control: renders "Phase 2"
claudine compose "$dir/plan.md" phase=2 --codex -c 'model_reasoning_effort="medium"' --dry-run

# defect: renders "Phase 1" today; renders "Phase 2" after the fix
claudine compose "$dir/plan.md" --codex -c 'model_reasoning_effort="medium"' phase=2 --dry-run
```

In both cases the stderr "Provider args" row must contain the `-c` switch and
its configuration value, with no `phase=2`. Assert the token list rather than
terminal table spacing. Remove the temporary directory after inspection.

## Scope and verification

**Changed:** setter ownership after provider switches for all three composition
commands, implemented by the dependency's shared classifier and preserved caller
override path. The dependency remains responsible for switch research, shared
completion classification, positional `argv`, and provider checks on every
launch path; completing this narrower spec does not complete that larger fix.

**Preserved:** Claudine-owned option precedence, the explicit `--` boundary,
forwarding of a switch's own setter-shaped value, setter parsing and merging,
and the switch-before-file and separator-before-file errors. Direct wrappers
are outside this fix's routing scope.

Use focused unit cases beside the partition and setter parser, plus compiled
binary checks for the resulting frontmatter and exact provider arguments.
Binary checks use `CliProcessFixture` with isolated home, working directory,
private audio policy, and fake providers; use Test Toolkit helpers for shared
environment state. No real provider, network, or focused terminal window is
needed. Run the implementation's canonical `just test`, `just test-l2`, and
`just lint` from `claudine/`, as required by the dependency. Tests use nextest.

No performance spike or new CI matrix is needed. The dependency already uses
in-process generated switch metadata and composition's existing document/schema
resolution. If implementation reveals a concrete new cost, use one host and a
quick sample; broader measurement is for the author to decide.

## Acceptance criteria

- [ ] The existing `reported_command_forwards_config_switch` test passes:
      `-c model_reasoning_effort=low` still reaches the provider unchanged.
- [ ] Every ownership-table case asserts both the caller setters and exact
      forwarded tokens. The no-provider case uses a document with no `agent`
      hint, so the candidate union is actually exercised.
- [ ] `--codex -c x=y -m gpt5 phase=2` keeps `-m gpt5` with Claudine,
      forwards `-c x=y`, and applies `phase=2`.
- [ ] A Claudine option interrupting a variadic provider value run ends that
      run; later tokens cannot reconnect to the earlier switch.
- [ ] Inline schemas, a root schema union, and a source-relative external
      schema protect a declared `phase` directly after `-c`. Missing-value
      errors name `-c`, `phase`, and Codex and launch no provider. An
      unestablished schema follows the dependency's contested-value error.
- [ ] `--codex -c phase=2 x=y` with a declared `phase` fails without
      reconnecting `x=y` to `-c`; no fake provider is launched.
- [ ] With no schema claiming `x`, `-c x=y phase=2` classified across candidates
      but resolved to Claude fails before spawn, rather than rerouting `x=y`
      into frontmatter. Use controlled metadata and fake provider fixtures.
- [ ] `--codex -- -c x=y phase=2` forwards both setter-shaped values unchanged,
      even when the schema declares `phase`; neither becomes a caller setter.
- [ ] Reclaimed numeric, boolean, empty, and string-containing-`=` setters
      retain their types. Duplicate shorthand setters keep last-occurrence
      precedence, and shorthand still wins over `--set`.
- [ ] Compiled-binary coverage proves a post-switch setter affects `compose`
      output, `inline-compose`'s effective launch input, and every applicable
      step of `sequence`; it is absent from provider arguments. Inline launch
      overlays are not automatically written to the source file.
- [ ] Existing caller propagation coverage includes a reclaimed setter through
      retry, resume, and proxy adoption, without adding a second classifier or
      changing its original ownership.
- [ ] The reproduction renders "Phase 2" and reports only the Codex
      configuration switch/value as provider arguments. Dry-run does not
      require Codex to be installed.
- [ ] `docs/topics/composition.md`, under Positional Arguments and Provider
      Argument Forwarding, explains setters after provider switches, the
      schema-priority missing-value error, and `--` as the provider-data escape
      hatch. Update argument-normalization docs and the Claudine skill with
      the dependency; current-behavior docs must not link back to a fix spec.

## Open Questions

No unresolved setter-specific behavior decision remains. The dependency owns
broader argument-classification decisions.

The existing frontmatter status is `proposed`, which is outside the requested
`$schema` status enum. It is preserved because this review was instructed to
leave other frontmatter properties unchanged. Before validating this spec against
that schema, the author must choose a supported status:

- **`draft-spec` (recommended):** accurately describes a reviewed design awaiting
  implementation planning; allows further edits. It does not declare the
  design final.
- **`finalized-spec`:** clearly signals design approval and readiness for a plan.
  It would imply approval that this agent review alone does not establish.
- **`planned`:** fits if the author adopts a concrete implementation plan for
  this fix through the dependency. The dependency having a plan does not alone
  establish that this narrower spec has reached that stage.
