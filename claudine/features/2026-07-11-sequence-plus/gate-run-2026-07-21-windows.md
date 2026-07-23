# Gate run — 2026-07-21 — Windows cross-target compile

Current compile-only evidence for review 11 finding 3. This record does not
claim Windows runtime behavior.

## Candidate

```
8876d1b80078e990af7d90fba1eb93e4fd756a71
```

The command ran from a detached clean worktree at this exact commit.
`git status --short` produced no output. No dirty authoring-tree file was part
of the candidate or the gate.

Host: macOS 26.5.2 (25F84), arm64.

```
rustc 1.96.0 (ac68faa20 2026-05-25)
cargo 1.96.0 (30a34c682 2026-05-25)
x86_64-w64-mingw32-gcc (GCC) 16.1.0
```

## Gate

Run from `claudine/`:

```
just check-windows
```

Result: exit `0`.

Verbatim cargo closing line:

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 56.28s
```

The recipe type-checked the `claudine` and `claudine-cli` libraries and test
targets for `x86_64-pc-windows-gnu`. Warnings were unused/dead-code artifacts
under the Windows configuration; there were no errors.

## Verification boundary

This is compile/type-check evidence only. `cargo check` did not link or run a
Windows binary, so it proves no Job Object, console-control, process-tree,
pipe-closure, or keyboard behavior. Native Windows execution remains absent.

Linux Level 3 was not rerun. This session ran on macOS without the required
Linux Xvfb/WezTerm/XTEST environment. The 2026-07-19 R3 Linux result remains
useful historical evidence but is not acceptance evidence for this candidate.
No host keyboard injection or desktop focus was attempted.
