---
area: claudine
status: ready for review
created: 2026-07-22
packages:
    - claudine-cli
review_iterations: 0
---

# Composition setters are swallowed into the provider tail when placed after an unowned switch

## Problem

A shorthand frontmatter setter (`key=value`) passed positionally to a
composition subcommand is silently dropped — forwarded to the underlying agent
as a meaningless argument instead of being applied as a Claudine override —
**whenever it appears after a provider switch Claudine does not own**.

The same setter applied earlier in the same command (before the unowned switch)
works correctly. So the result depends on argument position, which violates the
documented setter contract.

Observed in the wild with the `_implement/implement-plan.md` prompt:

```sh
# works: phase=2 applied, prompt renders "Phase 2 of 8"
claudine compose prompts/_implement/implement-plan.md \
  spec='fixes/2026-07-22-mega-merge/spec.md' \
  phase=2 \
  -c 'model_reasoning_effort="medium"' -y --codex

# BROKEN: phase=2 dropped, prompt still renders "Phase 1 of 8"
claudine compose prompts/_implement/implement-plan.md \
  spec='fixes/2026-07-22-mega-merge/spec.md' \
  -c 'model_reasoning_effort="medium"' \
  phase=2 \
  -y --codex
```

The only difference between the two invocations is that `phase=2` sits after
the codex `-c` switch in the broken case. Because the prompt's `phase`
frontmatter falls back to `1` when no override is supplied, the dropped setter
manifests as "Phase **1** of 8" instead of "Phase **2** of 8".

## Reproduction

Either invocation below composes with `--dry-run` so no agent is launched.
`--silent` prints only the rendered prompt body.

```sh
# control: setter before the unowned switch
claudine compose prompts/_implement/implement-plan.md \
  spec='fixes/2026-07-22-mega-merge/spec.md' phase=2 \
  -c 'model_reasoning_effort="medium"' --dry-run --silent | head -n1
# => # Implement Phase 2 of 8

# regression: setter after the unowned switch
claudine compose prompts/_implement/implement-plan.md \
  spec='fixes/2026-07-22-mega-merge/spec.md' \
  -c 'model_reasoning_effort="medium"' phase=2 --dry-run --silent | head -n1
# => # Implement Phase 1 of 8
```

The behavior is position-dependent and therefore a parsing defect, not a
frontmatter or loop defect. `total_phases` resolves correctly (it is read
directly from the plan frontmatter); the loop seed and iteration engine are
uninvolved.

## Root cause

`claudine compose` (and `inline-compose` / `sequence`) partition the raw argv
**before** clap parses it, splitting tokens into the Claudine argv (for clap)
and a **provider tail** forwarded to the agent. That logic lives in
`claudine/cli/src/argv/partition.rs::partition_composition_tail`.

Tokens after the subcommand are classified left to right:

1. A Claudine-owned flag (derived from the clap surface via
   `OwnedFlags::for_composition`) always belongs to Claudine, even after the
   tail has started.
2. The first **unowned** switch after the composition file starts an *implicit
   provider tail* (`tail_started = true`). The codex `-c` switch is unowned.
3. Once the tail has started, **every subsequent positional is pushed into the
   tail unconditionally** — the branch never consults `looks_like_setter`:

   ```rust
   // claudine/cli/src/argv/partition.rs:307-309
   // Positional token (file, setter, operand) or a non-UTF-8 token.
   if tail_started {
       tail.push(token.to_string_lossy().into_owned());   // phase=2 -> forwarded to codex
   } else if !file_seen {
       // setters honored here ...
   } else {
       // ... and here, but only while tail_started is false
   }
   ```

Trace for the broken invocation
(`… spec=… -c 'model_reasoning_effort="medium"' phase=2`):

| Token                                 | Decision                                   | Bucket   |
| ------------------------------------- | ------------------------------------------ | -------- |
| `prompts/…/implement-plan.md`         | positional → `file_seen = true`            | claudine |
| `spec=…`                              | setter, `tail_started` still false         | claudine |
| `-c`                                  | `Ownership::Unowned` → `tail_started=true` | tail     |
| `model_reasoning_effort="medium"`     | positional while `tail_started`            | tail     |
| **`phase=2`**                         | **positional while `tail_started`**        | **tail** |

`phase=2` is handed to codex and never reaches Claudine's frontmatter
overrides, so the prompt's `phase` stays at its template fallback of `1`.

### Why this contradicts the documented contract

`claudine/cli/src/argv/mod.rs` defines `looks_like_setter` with the explicit
guarantee:

> ```rust
> /// This is the same key validation used by
> /// `crate::commands::compose`'s `parse_compose_setter`; keeping them in lockstep
> /// guarantees the ownership partition classifies a token the same way the
> /// downstream positional parser will.
> pub(crate) fn looks_like_setter(token: &str) -> bool { … }
> ```

The downstream positional parser (`parse_compose_setter` /
`parse_composition_positionals`) treats `phase=2` as a Claudine setter. The
partition honors that lockstep promise only while `tail_started == false`, then
abandons it for the rest of the line.

### Contrast with the retired Rule 3

The partition replaced `claudine/cli/src/argv/rule3_separator.rs` (deleted in
the same commit). Its `apply_composition_separator` explicitly scanned for
`looks_like_setter` tokens and protected them (by inserting a `--` so they
remained positional). The new partition retained the `looks_like_setter`
helper but stopped consulting it once the tail began — a behavioral regression
that the helper's own doc comment still promises.

## When it regressed

- **Commit:** `2c7f98dcf` — *refactor(claudine-cli): replace argv Rule 3 with
  ownership partition*
- **Date:** 2026-07-13
- **Author:** Ken Snyder

`partition.rs` was introduced in this single commit and has not been modified
since (`git log --follow` shows it as the only touch). It is an ancestor of
`HEAD` on `feat/mega-merge`. The greedy `tail_started` branch has therefore
been dropping trailing setters since the day provider-switch forwarding was
introduced.

## Proposed fix

Reclaim setter-shaped tokens for Claudine in the `tail_started` branch before
forwarding them, restoring the "setters belong to Claudine regardless of
position" guarantee:

```rust
// claudine/cli/src/argv/partition.rs
if tail_started {
    if text.map(looks_like_setter).unwrap_or(false) {
        claudine.push(token.clone());
    } else {
        tail.push(token.to_string_lossy().into_owned());
    }
}
```

A setter-shaped token is unambiguous Claudine syntax — no provider accepts
`^[A-Za-z_][A-Za-z0-9_-]*=` as a switch — so reclaiming it is safe and
position-independent.

### Non-goal: greedy value consumption for unowned switches

The existing `reported_command_forwards_config_switch` behavior must be
preserved: a setter-shaped token that is the **direct argument** of an unowned
switch rides with that switch because it is consumed as the switch's value, not
encountered as a free-standing positional. The fix above does not change how
unowned switches consume their following token; it only changes the handling of
**free-standing** positionals encountered after the tail has started. In
`-c model_reasoning_effort=low`, the `model_reasoning_effort=low` token still
belongs to the tail; a *subsequent* free-standing `phase=2` is reclaimed.

> Note: `-c` is classified `Ownership::Unowned`, so the partition does not
> model its arity. The direct-argument case works today only because the value
> happens to be the next positional and falls into the same `tail_started`
> branch. If codex ever requires an explicit-space `--flag value` form for an
> unowned value-bearing switch, that should be addressed separately; it is out
> of scope for this setter-reclamation fix.

## Scope

- **Affected:** `claudine-cli` argv partition (`partition_composition_tail`),
  exercised by `compose`, `inline-compose`, and `sequence`.
- **Behavior change:** free-standing shorthand setters (`key=value`) encountered
  after the implicit provider tail has started are applied as Claudine
  frontmatter overrides instead of forwarded to the agent.
- **No change:** Claudine-owned flags, the explicit `--` boundary, the
  `SwitchBeforeFile` / `SeparatorBeforeFile` errors, and direct-argument
  forwarding of values to unowned switches.

## Acceptance criteria

- [ ] A shorthand setter placed anywhere in a `compose` / `inline-compose` /
      `sequence` invocation — before, between, or after provider switches — is
      applied as a frontmatter override.
- [ ] The regression reproductions above both render "Phase 2 of 8".
- [ ] `reported_command_forwards_config_switch` continues to forward
      `model_reasoning_effort=low` to the agent as the value of `-c`.
- [ ] A new unit test in `partition.rs` asserts that a trailing setter
      (`phase=2`) following an unowned switch (`-c`) lands in the Claudine argv,
      not the provider tail.
- [ ] A new unit test asserts the same setter is reclaimed even when multiple
      unowned switches precede it.
- [ ] No change to `SwitchBeforeFile` / `SeparatorBeforeFile` error semantics.
