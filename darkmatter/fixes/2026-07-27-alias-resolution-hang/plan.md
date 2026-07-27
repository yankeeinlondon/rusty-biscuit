---
status: ready for implementation
created: 2026-07-27
area: darkmatter
packages:
  - darkmatter
  - darkmatter-cli
  - claudine
implements: ./spec.md
---

# Implementation Plan — Remove Automatic Shell-Alias Resolution

Executes the Chosen Design in [`spec.md`](./spec.md): delete alias resolution
from composition and preflight, leaving the direct-executable contract.

Every file and line reference below was verified against the working tree at
plan time. Line numbers drift as edits land — treat them as locators, and match
on the symbol.

## Success Criteria

Done means all of these hold:

1. `md compose` on a `::shell` document never spawns `$SHELL`.
2. It cannot hang in any terminal / process-group / `$SHELL` combination.
3. An unresolved executable yields the existing typed `CommandNotFound` naming
   the **authored** executable.
4. Preflight and execution report the same executable and argv.
5. `ResolvedAlias`, `resolve_alias`, and `ShellApprovalRequest::alias_name` are
   gone from the public surface, and claudine builds without them.
6. `darkmatter` and `claudine` gates pass, with darkmatter's run performed at
   least once **from a real terminal pane**.

## Blast Radius (verified)

| File | Refs | Action |
|---|---:|---|
| `darkmatter/lib/src/markdown/compose/shell_expansion/alias.rs` | whole file (227 lines) | delete |
| `…/shell_expansion/mod.rs` | 15 | remove module decl, re-export, rewrite `resolve_or_passthrough` + `display_command` |
| `…/shell_expansion/types.rs:495-496` | 1 | drop `alias_name` field |
| `…/compose/preflight/collect.rs` | 6 | drop `resolve_executable` alias branch + import |
| `…/compose/preflight/lifecycle.rs:163` | 1 | drop `alias_name: None` initializer |
| `darkmatter/cli/src/approval.rs:102-111, 211` | 3 | remove the `Alias:` display row |
| `claudine/lib/src/harness/shell.rs:255` | 1 | drop `alias_name: None` initializer |
| `claudine/cli/…/tests/shell_approval.rs` | import | drop field if constructed |

**Do not touch — same name, unrelated:**
`claudine/lib/src/model_catalog/families.rs::resolve_alias` /
`resolve_alias_at` (model-family aliases) and the `resolve_alias_*` **test
names** in `darkmatter/lib/src/markdown/language_grammar.rs` (language-grammar
aliases). A careless `grep -r resolve_alias | xargs sed` breaks both.

## Phase 0 — Confirm scope before editing (required)

Repository policy requires impact analysis before modifying a symbol, and the
table above came from grep, not the call graph.

```
impact({target: "resolve_alias", direction: "upstream"})
impact({target: "resolve_or_passthrough", direction: "upstream"})
impact({target: "ShellApprovalRequest", direction: "upstream"})
```

Then `sniff repo package-areas` / `package-dependencies` to map results to gate
scopes. **Gate:** if impact returns a consumer outside `darkmatter` + `claudine`,
stop and extend the plan before writing code. Record the risk level; warn the
user if HIGH/CRITICAL.

## Phase 1 — Write the failing tests first

The spec's central requirement is that regression tests **fail against today's
code**. Writing them first is the only way to prove that, and it is cheap to
verify now while the bug is live.

### 1a. L2 reproduction — `darkmatter/cli/tests/level2_shell_alias_hang.rs` (new)

Follow the existing CLI L2 pattern (`level2_code_block_styling.rs`):

```rust
use biscuit_test_harness::tmux::TmuxHarness;
use test_toolkit::{Level, require_level};

#[test]
fn level2_compose_shell_directive_completes_in_background_process_group() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    // In a tmux pane (real PTY):
    //   set -m                      -> background jobs get their own pgrp
    //   SHELL=/bin/bash md compose <fixture> &
    // Poll for completion; fail if it has not exited within a few seconds.
}
```

Design constraints, all load-bearing:

- **tmux, not a GUI backend.** Headless, parallel-safe, and does not touch host
  focus (`rust-testing` skill). The tier must never steal focus or inject host
  input.
- **`set -m` is mandatory.** Without job control the child stays in the
  foreground process group and the bug does not reproduce — the test would pass
  against broken code and be worthless.
- **Pin `SHELL` to a bash-family shell.** zsh does not reproduce it. Skip
  cleanly if no bash-family shell exists rather than asserting a fixed path.
- **Assert completion and `CommandNotFound`**, not just non-hang, so it keeps
  covering behavior after the fix.
- **Reap on failure.** A stopped `bash -ic` survives the test; kill the pane's
  process tree in teardown or the suite leaks stopped processes (nextest `LEAK`
  will flag it, and stray `T`-state shells accumulated during investigation).

**Checkpoint:** run it now. It must **fail/hang**. If it passes against current
code, the harness is wrong — fix before continuing.

### 1b. L1 no-spawn test (spec Testing Contract 1)

Point `SHELL` at a fixture script that records invocation (writes a sentinel
file); compose and preflight an unresolved command; assert the sentinel is
**absent** and the error is `CommandNotFound`. Use `test_toolkit::EnvGuard` and
`#[serial_test::serial]` for the env mutation.

This is the durable guard — it needs no PTY, so it runs in CI where the L2 test
may skip.

### 1c. L1 preflight/runtime parity (spec R2)

For a missing executable across **single command, pipeline, and chain**, assert
preflight records the authored executable/argv and execution reports the same.

### 1d. L1 configuration independence (spec R3/R5)

Same compose run under two `$SHELL` values plus a fixture rc file that writes a
sentinel; assert identical results and no sentinel. Serialize.

## Phase 2 — Remove the runtime path

1. **Delete** `shell_expansion/alias.rs`.
2. **`shell_expansion/mod.rs`**: remove `pub mod alias;` (line 28) and
   `pub use alias::{ResolvedAlias, resolve_alias};` (line 37).
3. **Rewrite `resolve_or_passthrough`** (≈line 670-764) to a passthrough
   returning the parsed directive with no alias probe. Prefer deleting the
   function and inlining at its single call site (`prepare_directive`, line 232)
   if nothing else calls it — a function named "resolve or passthrough" that only
   passes through is a stale name.
4. **`display_command`** (≈line 766): drop the `Option<&str>` alias parameter and
   the `(alias: …)` suffix.
5. **`prepare_directive`** (line 232-233, 378): drop `alias_name` binding and the
   struct-literal field.
6. **`types.rs:495-496`**: delete the `alias_name` field and its doc comment.

Fix the rustdoc on `prepare_directive` (line 208, "Resolves aliases, applies
policy checks…") and line 112's "passed alias resolution" — repo policy requires
a doc pass in the same change as the behavior change.

## Phase 3 — Remove the preflight path

`preflight/collect.rs`:

1. Delete the import (line 30).
2. Reduce `resolve_executable` (line 91) to its non-alias behavior. If it becomes
   an identity function, delete it and simplify all three call sites (lines 258,
   295, 607).
3. Fix its rustdoc (line 90: "mirroring the runtime path") — after this change
   both paths trivially agree, which is the point of R2.

`preflight/lifecycle.rs:163`: drop `alias_name: None`.

## Phase 4 — CLI and downstream

1. **`darkmatter/cli/src/approval.rs`**: remove the `Alias:` display block
   (102-111) and the `alias_name: None` test fixture (211). Check the surrounding
   approval-prompt tests for assertions on that row.
2. **`claudine/lib/src/harness/shell.rs:255`**: drop `alias_name: None`.
3. **`claudine/cli/…/loop_control/tests/shell_approval.rs`**: drop the field if
   it constructs the struct.

Claudine is a **required** part of this change, not a follow-up — omitting it
breaks its build.

## Phase 5 — Documentation

1. Search `darkmatter/docs/inline/shell-expansion.md`, `shell-blocks.md`,
   `fm-shell-expansion.md` for alias claims and remove them.
2. **`darkmatter/docs/lsp/features.md`** references "shell aliases from policy
   file" (lines 142, 173) and "whitelist/blacklist/aliases" (line 258). Read
   before editing: these describe *policy-file* aliases, which may be a distinct
   (possibly aspirational) DMLS concept rather than the `$SHELL` probe being
   removed. Correct only what actually refers to shell-alias resolution; if the
   policy-file notion is separate and unimplemented, leave it and note it.
3. Update `.claude/skills/darkmatter/` if any page documents alias resolution.
4. If `::shell` alias support is user-facing in a README, record the removal —
   it is a deliberate behavior change, not a silent one.

## Phase 6 — Verify

```sh
cd darkmatter && just build && just test && just lint && just test-l2
cd claudine  && just build && just test && just lint
```

- **`just test-l2` covers lib, CLI, and DMLS** (verified in `darkmatter/justfile`),
  so the new CLI L2 test is picked up with no recipe change.
- **Run darkmatter's gates once from a real terminal pane.** Every existing test
  passed while this bug was live *because* the harness had no controlling
  terminal. A non-TTY-only verification repeats the exact mistake that let this
  ship.
- Confirm `test_compose_with_nonexistent_command_fails` lands near its 0.49 s
  nominal, well under the 5 s `SLOW` marker — passing under the 45 s kill ceiling
  proves nothing.
- `detect_changes({scope: "compare", base_ref: "main"})` before committing.

## Expected Side Effects

- **Faster composition.** Removes ~110 ms per directive, currently paid four
  times per document (the whole gap between this test at 0.49 s and its
  non-shell siblings at 0.057 s). Record before/after per the spec's acceptance
  criteria.
- **Breaking library API change**, accepted in the spec: no established users.
- **Behavior change for anyone relying on `::shell` aliases** — they must put an
  executable wrapper on `PATH`.

## Risks

| Risk | Mitigation |
|---|---|
| Over-broad delete hits the two false-positive `resolve_alias` symbols | Phase 0 impact analysis; edit by file from the blast-radius table, never a repo-wide sed |
| L2 test passes against broken code (missing `set -m`, or zsh) | Phase 1 checkpoint requires observing it fail first |
| Stopped `bash -ic` leaks from a failed L2 run | Explicit process-tree teardown; nextest `LEAK` as backstop |
| Alias removal breaks an unknown consumer | Phase 0 gate stops the plan if impact reaches beyond darkmatter + claudine |
| Verification done only in a non-TTY context | Phase 6 mandates one terminal-pane run |

## Explicitly Out of Scope

Per the spec: `::shell` execution timeout behavior (already bounded — verified:
`executor.rs:770` `wait_with_timeout` kills the child at line 785 and joins);
alias configuration formats; the repeated shell-expansion stage invocation
(spec open question 1); compose-cache single-flight parks (separate topic); and
the `.config/nextest.toml` 45 s ceiling, which is orthogonal and stays.
