# Merge Plan

## Scope

`git status --short` shows two actual unresolved merge conflicts in this worktree:

- `biscuit-terminal/lib/src/terminal.rs`
- `sniff/lib/src/programs/find_program.rs`

Other files found by a raw conflict-marker search are not currently unmerged according to Git. Treat those as normal content until proven otherwise.

## Recommended Resolution Order

1. Resolve `biscuit-terminal/lib/src/terminal.rs`
2. Resolve `sniff/lib/src/programs/find_program.rs`
3. Run focused tests for `biscuit-terminal` and `sniff`
4. Re-check `git status` for any remaining unmerged paths

## Conflict Review

### `biscuit-terminal/lib/src/terminal.rs`

Conflict is in `new_terminal()`.

Current branch (`HEAD`) changes:

- Keeps terminal cell-size probing lazy by setting `cell_size: None`
- Documents why eager CSI 14t probing is unsafe in normal rendering paths

Incoming (`main`) changes:

- Adds `#[tracing::instrument(name = "Terminal::new")]`
- Switches to a local `terminal` variable
- Emits a `tracing::debug!` event with detected terminal metadata
- Reintroduces eager `cell_size: cell_size()`

### Proposed merge

Keep both tracing additions from `main`, but keep the lazy `cell_size: None` behavior from `HEAD`.

Concretely, `new_terminal()` should:

- Keep `#[tracing::instrument(name = "Terminal::new")]`
- Build a local `terminal` variable
- Set `cell_size: None`
- Keep the `tracing::debug!` call
- Return `terminal`

### Reasoning

This is the only conflict with behavioral impact. The file already contains a test asserting that `Terminal::new()` must not eagerly cache cell size because live CSI 14t probes can leak raw tty responses and create `/dev/tty` races. Keeping `main`'s eager `cell_size()` call would directly contradict that intent and likely regress the existing test coverage.

The tracing additions are compatible with the lazy strategy and should be preserved.

### Validation

After resolving, run:

```bash
cargo test -p biscuit-terminal test_terminal_new_does_not_eagerly_cache_cell_size
cargo test -p biscuit-terminal
```

If there is concern about image rendering regressions, also run the `biscuit-terminal` integration tests.

### `sniff/lib/src/programs/find_program.rs`

Conflict is only the `check_bundle_executable` parameter type:

- `HEAD`: `&std::path::Path`
- base: `&PathBuf`
- `main`: `&Path`

### Proposed merge

Keep `main`'s signature:

```rust
fn check_bundle_executable(bundle_path: &Path, binary_name: &str) -> Option<PathBuf>
```

### Reasoning

There is no behavioral change here. `&Path` is the most idiomatic and general API, and the file already imports `Path` at the top. Existing callers passing `&PathBuf` continue to work through deref coercion.

### Validation

After resolving, run:

```bash
cargo test -p sniff
```

If a narrower pass is needed first, run the tests covering program discovery and macOS bundle detection.

## Merge Checklist

- Remove conflict markers from both files
- Preserve lazy `cell_size` initialization in `biscuit-terminal`
- Preserve tracing instrumentation and debug logging in `biscuit-terminal`
- Use `&Path` in `sniff`'s `check_bundle_executable`
- Run focused tests
- Confirm `git status --short` no longer reports `UU` entries

## Expected Final State

After the merge:

- `biscuit-terminal` keeps the newer observability hooks from `main` without reintroducing eager tty probing
- `sniff` compiles with the broader and cleaner `&Path` helper signature
- The worktree is ready for the next review pass with conflict markers removed
