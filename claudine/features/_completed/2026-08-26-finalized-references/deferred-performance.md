---
feature: 2026-08-26-finalized-references
created: 2026-09-08
---

# Deferred performance measurements — `2026-08-26-finalized-references`

Performance verification that a review asked for and an implementation cycle
could not legitimately produce. Each entry names the finding it maps back to,
the review that raised it, and precisely what has to happen before it can close.

## 1. `level2_perf_tree_renders_styled_in_tmux` — relative-timing assertion under host contention

- **Maps back to:** finding 2 of [review-3.md](review-3.md) (high) — "AC10
  remains incomplete; the Windows/WSL Level 2 CI legs it requires are not yet
  provisioned", whose final-tree half requires a green `just test-l2` in the
  Claudine package area.
- **Acceptance criterion:** AC10.
- **Deferred during:** implementation cycle 3, 2026-09-08.

### What is required

`just test-l2` must pass in the `claudine/` package area on the final tree. The
test `level2_perf_tree_renders_styled_in_tmux`
(`claudine/cli/tests/level2_perf_capture.rs:204-212`) composes a document that
contains a deliberately slow `::shell · sleep` directive, captures the rendered
performance tree from a real tmux pane, and asserts that the `▇ HOT` marker
lands on that directive:

```rust
let hot = plain
    .lines()
    .find(|l| l.contains("▇ HOT"))
    .unwrap_or_else(|| panic!("expected a `▇ HOT` marker.\nplain:\n{plain}"));
assert!(
    hot.contains("shell · sleep"),
    "HOT must flag the dominant `::shell` directive; got: {hot:?}",
);
```

This is a **relative-timing** assertion: it does not check a fixed budget, it
checks which of two measured spans is larger. It therefore depends on the host
being quiet enough that a fixed `sleep` still outweighs Claudine's own
environment-setup work.

### What happened

During the full-suite run of 2026-09-08 the assertion failed:

```
thread 'level2_perf_tree_renders_styled_in_tmux' panicked at
claudine/cli/tests/level2_perf_capture.rs:209:5:
HOT must flag the dominant `::shell` directive; got:
"▌ ├─ environment setup                     346.3ms   50% ▇ HOT"
```

The host was saturated at that moment:

- load average **72.56** (1-minute), 62.14 (5-minute);
- a concurrent `claudine` test binary from a *different* worktree
  (`.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests`) at ~61% CPU;
- an `opencode` process at ~57% CPU;
- a second `cargo-nextest` at ~27% CPU.

Environment setup — process spawn, filesystem work, repository observation — is
exactly the work that inflates under that kind of contention, while a `sleep`
does not. The measurement inverted for a host reason, not a product reason.

### Evidence that it is contention, not a regression

- Re-run in isolation on the same tree via `just test-l2
  level2_perf_tree_renders_styled_in_tmux`: **1 passed** in 6.9 s.
- Re-run as part of the complete suite with `--no-fail-fast` a few minutes
  later, with load easing to ~59.8: **passed**, and the suite finished 238/239
  with a single unrelated failure (the host Atuin first-run prompt wedging
  `level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`).
- The change under review in this cycle touches only
  `claudine/cli/tests/spawn_inventory.rs`, a source-inventory guard that runs
  `syn` over text and spawns no child process, so it cannot affect a
  perf-tree measurement.

### What has to happen before it closes

A single `just test-l2` run in the `claudine/` package area on a quiet host —
no concurrent worktree test suites, no other agentic CLI sessions — showing the
Claudine L2 tier green. CI's `claudine-cli / test-l2` legs on `ubuntu-latest`
and `macos-latest` were `success` in run `34192299897`, so the assertion is not
inherently unstable; it needs a local host that is not being shared by several
concurrent builds.

### Note for whoever picks this up

If the test proves flaky on shared CI runners as well, the durable fix is to
make the assertion measure what it means — that the `::shell` span's *own*
duration exceeds a threshold derived from the `sleep` it was given — rather
than that it outranks a sibling whose cost is a property of the host. That
would be a test-design change in `claudine/cli/tests/level2_perf_capture.rs`,
not a product change, and it is out of scope for a file-reference feature.
