# Snapshot Redaction

Snapshots become noisy when they include host-specific or time-varying values. Redact those inputs before snapshotting.

## Common Redaction Targets

- temp directories
- `$HOME`-derived paths
- timestamps
- UUIDs and session IDs
- ANSI escape sequences
- generated filenames

## String Redaction

If the snapshot target is plain text, pre-process it first:

```rust
fn redact_temp_home(input: &str) -> String {
    input.replace("/var/folders/xyz/tmp", "<temp>")
}

assert_snapshot!(redact_temp_home(&output));
```

## JSON Redaction

Use Insta redaction selectors for structured output:

```rust
use insta::assert_json_snapshot;

assert_json_snapshot!(value, {
    ".id" => "[id]",
    ".session_id" => "[session]",
    ".timestamp" => "[timestamp]",
    ".paths[].home" => "[home]",
});
```

## TUI Buffer Snapshots

For Ratatui `Buffer` snapshots:

- prefer deterministic terminal sizes
- disable color when the snapshot does not need style data
- snapshot cloned buffers, not terminal handles

## Review Workflow

Use:

```bash
cargo insta pending-snapshots
cargo insta review
cargo insta accept
```

Do not accept snapshots blindly. First confirm the change is a real output improvement rather than a temp-path or timestamp regression.
