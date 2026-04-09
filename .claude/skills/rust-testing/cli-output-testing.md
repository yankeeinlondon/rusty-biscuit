# CLI Output Testing

Rust CLIs should test more than success or failure. High-value coverage usually includes:

- stdout vs stderr channel separation
- `NO_COLOR`, `FORCE_COLOR`, and explicit `--plain` handling
- shell completion generation
- stable dry-run or help output snapshots

## Shared Pattern

Put common tempdir, config, git, executable, and ANSI helpers in `tests/common/mod.rs`, then import them from each integration test crate with `mod common;`.

## Output Channels

Machine-readable or pipeable output should be asserted on `stdout`. Status, progress, warnings, and errors should be asserted on `stderr`.

```rust
let output = cargo_bin_cmd!("my-cli")
    .args(["completions", "bash"])
    .assert()
    .success()
    .get_output()
    .clone();

assert!(output.stderr.is_empty());
assert!(!output.stdout.is_empty());
```

For failure paths, assert the inverse when appropriate:

```rust
let output = cargo_bin_cmd!("my-cli")
    .args(["run", "/tmp/missing.file"])
    .assert()
    .failure()
    .get_output()
    .clone();

assert!(output.stdout.is_empty());
assert!(!output.stderr.is_empty());
```

## Color Modes

Test the three common modes independently:

- `NO_COLOR=1`: no ANSI sequences
- `FORCE_COLOR=1`: ANSI sequences even in non-TTY integration tests
- `--plain`: explicit CLI no-style mode; should usually override forced color

```rust
let stdout = String::from_utf8(
    cargo_bin_cmd!("my-cli")
        .env("NO_COLOR", "1")
        .args(["providers"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone(),
).unwrap();

assert_eq!(stdout, strip_ansi(&stdout));
```

## Shell Completions

Prefer shell-specific markers over full snapshots:

- bash: `_my_cli()`
- zsh: `#compdef my-cli`
- fish: `complete -c my-cli`
- elvish: `edit:completion:arg-completer[...]`

## Snapshot Targets

Good CLI snapshot candidates:

- grouped help output
- stable dry-run output
- tables or summaries without live timestamps
- error messages after redaction
