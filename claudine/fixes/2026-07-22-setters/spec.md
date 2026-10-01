---
area: claudine
status: proposed
created: 2026-07-22
packages:
    - claudine-cli
related:
    - 2026-07-13-cli-switches
review_iterations: 1
---

# A shorthand setter after a provider switch is forwarded without notice

## Problem

A shorthand frontmatter setter (`key=value`) placed **after** a provider switch
Claudine does not own is forwarded to the agent rather than applied as a
frontmatter override. Nothing tells the user. The prompt renders with the
key's fallback value, and the agent receives a stray operand.

```sh
# phase=2 applied: setter precedes the provider tail
claudine compose plan.md phase=2 --codex -c 'model_reasoning_effort="medium"'

# phase=2 forwarded to codex: setter follows the provider tail's first switch
claudine compose plan.md --codex -c 'model_reasoning_effort="medium"' phase=2
```

Forwarding is the intended routing. The defect is that it is **silent**. The
user wrote a token that is valid setter syntax and names a key the prompt
declares, and Claudine dropped it without saying so.

## Decided contract this fix must keep

`2026-07-13-cli-switches` decided how composition argv is routed, and
`partition.rs:15-30` and `docs/topics/argv-normalization.md` (Provider-argument
partition) document it:

1. A token on Claudine's clap surface belongs to Claudine everywhere before an
   explicit `--`, even after an implicit tail has started.
2. The first unowned switch after the file starts the implicit agent tail.
   **Every non-Claudine token from there, setter-shaped values included, is
   forwarded in original order.**
3. **Routing never depends on researched switch metadata.** Claudine does not
   model how many values an unowned switch takes, so it cannot tell `-c` and
   its value apart from `--yolo` followed by a free-standing setter. Guessing
   would let a switch steal the composition file or a real setter.
4. Setters meant for Claudine go before the tail. This follows from rule 1
   rather than being stated separately: `--set` is a Claudine-owned flag, so it
   is reclaimed anywhere before `--`, which makes it the escape hatch for a
   setter that has to come later.

Any fix that moves a setter-shaped tail token back to Claudine breaks rule 2,
that spec's acceptance criterion 4 ("a setter-shaped token after tail start is
forwarded"), and `reported_command_forwards_config_switch`.
In `-c model_reasoning_effort=low`, the partition does not take a value for
the unowned `-c`, so `model_reasoning_effort=low` reaches the partition as a
free-standing positional. A `looks_like_setter` check in the `tail_started`
branch would pull it back into Claudine. Leaving the token right after a
space-form unowned switch in the tail would still guess at arity (rule 3).

## Root cause

`partition_composition_tail` (`cli/src/argv/partition.rs:307-309`) pushes every
positional to the tail once `tail_started` is set. That matches rule 2. The
defect is downstream: no surface checks the forwarded tail against the keys
the composition can set, so a misplaced setter fails silently.

Two pieces of supporting text add to the confusion:

- The `looks_like_setter` doc comment (`cli/src/argv/mod.rs`) says the
  partition "classifies a token the same way the downstream positional parser
  will". That is true only for positionals before the tail starts. The comment
  should be scoped to that case.
- `docs/topics/composition.md` (Provider Argument Forwarding) shows a
  setter-shaped *value* being forwarded, but never says that a free-standing
  setter after the tail is forwarded too, or how to apply one there.

The partition has not changed since `2c7f98dcf` (2026-07-13), and no test
covers a free-standing setter after an unowned switch.

## Fix

Keep the routing. Add a reporting-only diagnostic, which the 2026-07-13
contract allows to use any information without changing ownership.

### Misplaced-setter warning

After the composition file's frontmatter is loaded, check each **forwarded**
tail token. If a token:

- comes from an **implicit** tail. An explicit `--` tail is opaque and is
  never checked. Read the source from the tail itself: today that is
  `provider_args_explicit`, and after cli-switches R6 it is the typed tail
  descriptor's implicit/explicit source;
- satisfies `looks_like_setter`; and
- has a key that is a top-level frontmatter key of the composition file (for
  `sequence`, the sequence file),

then print one warning per token, before the dry-run seam, so `--dry-run`
prints it as well:

```text
warning: [setter] `phase=…` follows a provider switch, so it was forwarded to codex and not applied to frontmatter.
         Move it before the first provider switch, or pass --set '{"phase": …}'.
```

Rules for the warning:

- It never shows the value: the key is printed and the value is replaced by
  `…`, consistent with the tail-redaction policy.
- It follows the frontmatter `model` mismatch warning's contract: it does not
  block, it is suppressed by `--silent`, and `--dry-run` prints it.
- It is deduplicated per `(provider, tail)` in the same state the forwarding
  notice uses, so a `sequence` or loop does not repeat it on every step. Do
  not add a second process-wide static: cli-switches R3 moves that state into
  per-command execution state, and the warning goes with it.
- It stays in the composition path even after cli-switches R5 moves the
  forwarding notice into a module shared with direct wrappers. Direct wrappers
  have no frontmatter, so the check does not apply there.
- It does not change argv. The token is still forwarded.

Because the warning does not guess arity, it also fires for a switch's value
whose key matches a frontmatter key. For example, `-c model=o3` warns when the
prompt declares `model`. The warning is still accurate in that case: the token
was forwarded and not applied. A key the prompt does not declare does not warn;
a setter that introduces a new key after the tail is a known gap that this
fix leaves open.

### Docs and comments

- `docs/topics/composition.md`, in both Positional Arguments and Provider
  Argument Forwarding: state that a setter after the first provider switch is
  forwarded, give the supported order
  (`<file> [key=value ...] [CLAUDINE_OPTIONS] [AGENT_ARGS ...]`), show the
  `--set` form for a late setter, and describe the warning.
- `docs/topics/argv-normalization.md`, in Provider-argument partition: add a
  free-standing-setter example next to the `-c model_reasoning_effort=low`
  example, and note that the partition never warns. The warning is a
  composition-time report.
- `cli/src/argv/partition.rs:24-27`: no change to the rule. Add one clause
  noting that a free-standing setter is included.
- `looks_like_setter` doc comment: scope the "same classification" guarantee
  to positionals before the tail starts.

## Reproduction

Self-contained; `--dry-run` launches no agent and writes the composed body to
stdout.

```sh
dir=$(mktemp -d)
cat > "$dir/plan.md" <<'EOF'
---
phase: 1
---
Phase {{ phase }}
EOF

# control: setter before the tail => "Phase 2"
claudine compose "$dir/plan.md" phase=2 --codex -c 'model_reasoning_effort="medium"' --dry-run

# defect: setter after the tail => "Phase 1", with no warning today
claudine compose "$dir/plan.md" --codex -c 'model_reasoning_effort="medium"' phase=2 --dry-run

# escape hatch: --set is reclaimed after the tail => "Phase 2"
claudine compose "$dir/plan.md" --codex -c 'model_reasoning_effort="medium"' --set '{"phase":2}' --dry-run
```

After the fix, the second command still renders "Phase 1" and its dry-run
metadata still lists `phase=2` in the forwarded tail (redacted), but stderr
carries the `[setter]` warning for `phase`.

## Scope

- **Changed:** a new composition-time warning; documentation and doc-comment
  corrections.
- **Unchanged:** `partition_composition_tail` routing, Claudine-owned flag
  reclaim, the explicit `--` boundary, `SwitchBeforeFile` /
  `SeparatorBeforeFile`, the forwarding INFO notice, and the child argv.
- **Non-goal:** arity-aware routing for unowned switches. If research-backed
  switch metadata (R8 of `2026-07-13-cli-switches`, including `value_arity`) arrives, it may make
  the warning more precise. For example, it could skip the declared value of a
  known one-value switch. It must not change routing.

## Acceptance criteria

- [ ] `reported_command_forwards_config_switch` passes unchanged.
- [ ] A new partition unit test pins the current routing: in
      `compose file.md -c x=y phase=2`, both `x=y` and `phase=2` land in the
      tail.
- [ ] A new partition unit test pins the escape hatch: in
      `compose file.md -c x=y --set '{"phase":2}'`, `--set` and its value land
      in the Claudine argv.
- [ ] A setter-shaped implicit-tail token whose key is a top-level
      frontmatter key prints one `[setter]` warning naming the key and
      provider, with no value. A token whose key is not declared prints none.
      A token after an explicit `--` prints none.
- [ ] The warning is printed under `--dry-run`, suppressed by `--silent`, and
      printed once per `(provider, tail)` across `sequence` steps.
- [ ] The second reproduction command above prints the warning; the first and
      third do not.
- [ ] `composition.md`, `argv-normalization.md`, `partition.rs` module docs,
      and the `looks_like_setter` doc comment are updated as described in
      Docs and comments.
