---
area: claudine
status: proposed
created: 2026-07-22
packages:
    - claudine-cli
related:
    - 2026-07-13-cli-switches
depends-on:
    - 2026-07-13-cli-switches
review_iterations: 2
---

# A shorthand setter after a provider switch is forwarded to the agent

## Problem

A shorthand setter (`key=value`) placed after a provider switch Claudine does
not own is passed to the agent instead of being applied to the prompt's
frontmatter. Nothing tells the user. The prompt renders with the key's default,
and the agent receives a stray argument.

```sh
# phase=2 is applied: it comes before the first provider switch
claudine compose plan.md phase=2 --codex -c 'model_reasoning_effort="medium"'

# phase=2 is sent to Codex: it comes after the first provider switch
claudine compose plan.md --codex -c 'model_reasoning_effort="medium"' phase=2
```

## Root cause

`partition_composition_tail` (`cli/src/argv/partition.rs:307-309`) treats the
first unowned switch as the start of an agent tail and forwards every later
non-Claudine token, setter-shaped ones included. It cannot do better without
knowing switch types. In `-c model_reasoning_effort=low`, the `key=value` is
the value of Codex's `-c`. Telling it apart from a free-standing `phase=2`
requires knowing that `-c` takes exactly one string.

The partition has not changed since `2c7f98dcf` (2026-07-13), and no test
covers a setter after an unowned switch.

## Fix

The fix is the type-aware token ownership contract in `2026-07-13-cli-switches`
(Token ownership, with work items R8 and R9). This spec does not define a
separate mechanism. It records the setter cases that contract must get right
and the tests that prove it.

Under those rules:

- `phase=2` in `-c x=y phase=2` follows a value, not a switch, so it is a
  setter (rule 3).
- A `$schema` parameter is always a setter, even directly after a switch
  (rule 2). If that leaves a switch without a required value, Claudine fails
  before launch instead of letting the provider take the wrong token.
- A `key=value` after a switch that takes no value for every candidate
  provider, such as `--yolo phase=2`, is a setter.
- A `key=value` that is not the first value after a variadic switch is a
  setter.

An earlier revision of this spec proposed a warning that left routing
unchanged. The type-aware contract replaces it, so no warning is added.

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

# control: renders "Phase 2"
claudine compose "$dir/plan.md" phase=2 --codex -c 'model_reasoning_effort="medium"' --dry-run

# defect: renders "Phase 1" today; renders "Phase 2" after the fix
claudine compose "$dir/plan.md" --codex -c 'model_reasoning_effort="medium"' phase=2 --dry-run
```

In both cases the dry-run "Provider args" row must show only
`-c model_reasoning_effort=…`.

## Scope

- **Changed:** where a setter-shaped token after a provider switch lands, as
  part of R9.
- **Unchanged:** Claudine-owned flag reclaim, the explicit `--` boundary,
  `SwitchBeforeFile` / `SeparatorBeforeFile`, and forwarding of a switch's own
  `key=value` value.

## Acceptance criteria

These are setter-specific checks of `2026-07-13-cli-switches` criteria 4, 16,
17, 18, and 21.

- [ ] `reported_command_forwards_config_switch` passes unchanged:
      `-c model_reasoning_effort=low` reaches the agent.
- [ ] Partition tests, with `--codex` and with no provider named:
      `-c x=y phase=2` forwards `-c x=y` and applies `phase=2`.
- [ ] Partition test: `-c x=y -m gpt5 phase=2` (a Claudine flag between the
      provider switch and the setter) applies `phase=2` and keeps `-m gpt5`
      with Claudine.
- [ ] Partition test: `--claude --add-dir a b phase=2` forwards
      `--add-dir a b` and applies `phase=2`.
- [ ] Partition test: with `phase` in the file's `$schema`, `--codex -c phase=2`
      fails before launch with an error naming `-c` and `phase`.
- [ ] The defect reproduction above renders "Phase 2" and its "Provider args"
      row lists only `-c`.
- [ ] `docs/topics/composition.md` (Positional Arguments and Provider Argument
      Forwarding) shows a setter after a provider switch being applied.
