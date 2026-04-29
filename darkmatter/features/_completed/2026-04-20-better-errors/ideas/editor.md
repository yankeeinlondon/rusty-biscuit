# EditorError — Block Style Error Ideas

Source: `darkmatter/lib/src/editor/mod.rs:45`

## Overview

`EditorError` covers editor discovery, launch, and file-round-trip failures. It has five variants, all triggered from three entry points:

- `resolve_editor_command()` — probes `$EDITOR`, `$VISUAL`, then a priority list of 22 known editors
- `launch_editor_on_path()` — spawns the resolved editor and waits for exit
- `edit_text()` — creates a temp file, launches the editor, then reads the result back

The current `#[error(...)]` messages are plain strings with no context about which file was being edited, which editor was attempted, or what the user can do to fix the problem.

---

## Variants

### 1. `NoEditorFound`

**Current message:**
```
no editor found; set $EDITOR or $VISUAL, or install one of: nvim, vim, code, nano
```

**Trigger:** `resolve_editor_command()` exhausts `$EDITOR`, `$VISUAL`, and the 22-binary `DEFAULT_EDITOR_PRIORITY` probe without finding a usable binary.

**Block Style proposals:**

#### 1a. List the actual detection steps taken

Use a `StatusBlock` with an Error-severity header and a body that explains what was checked, so the user doesn't have to guess whether their `$EDITOR` pointed to a missing binary or was never set.

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>No editor found on this system</b>")
    .body(Prose::new(
        "Checked the following in order:\n\
         ┃ 1. $EDITOR — not set or binary not found\n\
         ┃ 2. $VISUAL — not set or binary not found\n\
         ┃ 3. Probed 22 known editors — none installed"
    ))
    .hint("Set <b>$EDITOR</b> to your preferred editor, e.g.:\n\
           <dim>export EDITOR=nvim</dim>")
```

Rendered (conceptual):

```
⤫ EditorError: No editor found on this system
┃ Checked the following in order:
┃  1. $EDITOR — not set or binary not found
┃  2. $VISUAL — not set or binary not found
┃  3. Probed 22 known editors — none installed
Set $EDITOR to your preferred editor, e.g.:
export EDITOR=nvim
```

#### 1b. Surface which specific env vars were set (but failed)

If `$EDITOR` or `$VISUAL` was set but the binary wasn't found, include the actual value in the body so the user immediately sees the typo or stale path:

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>No editor found on this system</b>")
    .body(Prose::new(format!(
        "<red>$EDITOR={}</red> — binary not found on $PATH\n\
         ┃ <dim>(also checked $VISUAL and 22 known editors)</dim>",
        editor_value
    )))
    .hint("Install the editor, fix the path, or run:\n\
           <dim>export EDITOR=$(which nvim)</dim>")
```

This variant is only applicable when `$EDITOR` / `$VISUAL` was set to something — the general case (1a) covers when neither is set at all.

---

### 2. `NonZeroExit`

**Current message:**
```
editor exited with status {0}
```

**Trigger:** `launch_editor_on_path()` observes a non-zero exit code after the editor process completes.

**Block Style proposals:**

#### 2a. Include the file path and exit code with actionable guidance

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Editor exited with an error</b>")
    .body(Prose::new(format!(
        "Editor <b>{}</b> exited with code <red>{}</red>\\
         \n┃ File: <dim>{}</dim>",
        editor_name, exit_code, path.display()
    )))
    .hint("This usually means the editor crashed or was force-killed.\n\
           ┃ Try opening the file manually to verify it's intact:\n\
           ┃ <dim>{}</dim> {}",
        editor_name, path.display()
    )
```

Rendered (conceptual):

```
⤫ EditorError: Editor exited with an error
┃ Editor nvim exited with code 1
┃ File: /tmp/.tmpX8kF2P.md
This usually means the editor crashed or was force-killed.
┃ Try opening the file manually to verify it's intact:
┃ nvim /tmp/.tmpX8kF2P.md
```

#### 2b. Differentiate signal-based exits from normal error codes

On Unix, a signal-kill produces exit codes like `137` (SIGKILL) or `143` (SIGTERM). Detecting and explaining these specifically removes confusion:

```rust
let explanation = match exit_code {
    127 => "Command not found — the editor binary may have been removed".to_string(),
    130 => "Interrupted by Ctrl-C".to_string(),
    137 => "Killed by the system (SIGKILL — possibly OOM)".to_string(),
    code if code > 128 => format!("Killed by signal {} (exit code {})", code - 128, code),
    _ => format!("Exit code {}", code),
};

StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Editor session ended unexpectedly</b>")
    .body(Prose::new(explanation))
    .hint("If this was unintentional, your file should still be intact.\n\
           Re-run the command to resume editing.")
```

---

### 3. `Missing`

**Current message:**
```
edited file was deleted during editing
```

**Trigger:** `edit_text()` re-checks the temp file path after the editor exits and discovers it no longer exists.

**Block Style proposals:**

#### 3a. Show the temp file path and explain what likely happened

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Edited file disappeared during editing</b>")
    .body(Prose::new(format!(
        "The temporary file was deleted while the editor was open.\n\
         ┃ Expected path: <dim>{}</dim>",
        path.display()
    )))
    .hint("This can happen if:\n\
           ┃ • The editor's save routine writes to a new file and deletes the original\n\
           ┃ • A temp-file cleanup daemon removed it\n\
           ┃ • You manually deleted it inside the editor\n\
           ┃ Re-run the command to start a new editing session.")
```

#### 3b. Suggest an alternative workflow for backup-file editors

Some editors (e.g., Vim with `backupcopy=no`) replace the original file on save, which can break the temp-file pattern. A targeted hint:

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Edited file disappeared during editing</b>")
    .body(Prose::new(format!(
        "Temp file not found after editor closed:\n\
         ┃ <dim>{}</dim>",
        path.display()
    )))
    .hint("If your editor uses a backup-file strategy, try:\n\
           ┃ <dim>set backupcopy=yes</dim> (Vim)\n\
           ┃ or pipe content directly instead of using temp-file editing.")
```

---

### 4. `LaunchFailed`

**Current message:**
```
failed to launch editor {editor:?}: {source}
```

**Trigger:** `Command::status()` returns an `io::Error` before the process even starts — typically `NotFound`, `PermissionDenied`, or similar OS errors.

**Block Style proposals:**

#### 4a. Decode the OS error into human-readable context

```rust
let io_context = match source.kind() {
    std::io::ErrorKind::NotFound => format!(
        "The editor binary <red>{}</red> was not found on $PATH",
        editor
    ),
    std::io::ErrorKind::PermissionDenied => format!(
        "Permission denied when trying to execute <red>{}</red>",
        editor
    ),
    _ => format!("{}", source),
};

StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Could not start editor</b>")
    .body(Prose::new(io_context))
    .hint(format!(
        "Resolved editor command was: <dim>{}</dim>\\
         \n┃ Verify it is installed and executable:\n\
         ┃ <dim>which {} && {} --version</dim>",
        editor_cmd,
        editor_cmd.split_whitespace().next().unwrap_or(&editor_cmd),
        editor_cmd.split_whitespace().next().unwrap_or(&editor_cmd)
    ))
```

Rendered (conceptual):

```
⤫ EditorError: Could not start editor
┃ The editor binary "nano" was not found on $PATH
Resolved editor command was: nano
┃ Verify it is installed and executable:
┃ which nano && nano --version
```

#### 4b. Include the full resolved command string (with flags)

When `$EDITOR` is something like `code --wait --new-window`, the launch failure could be in the flags, not just the binary. Including the full resolved string helps the user debug:

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>Editor launch failed</b>")
    .body(Prose::new(format!(
        "Failed to spawn: <red>{}</red>\n\
         ┃ OS error: <dim>{}</dim>",
        full_command, source
    )))
    .hint("Check that your $EDITOR value is a valid command:\n\
           ┃ <dim>echo $EDITOR</dim>\n\
           ┃ <dim>eval $EDITOR</dim>")
```

---

### 5. `Io`

**Current message:**
```
(transparent — forwards the std::io::Error display)
```

**Trigger:** General I/O failures during temp file creation (`tempfile::Builder`), writing, flushing, or reading the file back. This is a `#[from]` variant that wraps any `std::io::Error`.

**Block Style proposals:**

#### 5a. Add context about which operation failed

The bare `io::Error` gives no indication of whether it was temp-file creation, writing, flushing, or reading. Wrapping with context:

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>File I/O failure during editing</b>")
    .body(Prose::new(format!(
        "{}\n\
         ┃ Operation: <dim>{}</dim>\n\
         ┃ Path: <dim>{}</dim>",
        source,
        operation_description, // e.g., "create temp file", "write initial content", "read edited content"
        path.display()
    )))
    .hint("Check that your temp directory is writable:\n\
           ┃ <dim>echo $TMPDIR</dim>\n\
           ┃ <dim>df -h $TMPDIR</dim>")
```

#### 5b. Surface disk-space and permission diagnostics

Since temp-file I/O errors are commonly caused by a full `/tmp` or permission issues, a targeted hint:

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>EditorError:</b> <b>I/O error during temp file handling</b>")
    .body(Prose::new(format!(
        "<red>{}</red>",
        source
    )))
    .hint("Common causes:\n\
           ┃ • Temp directory is full (<dim>df -h /tmp</dim>)\n\
           ┃ • Permissions prevent writing (<dim>ls -la $TMPDIR</dim>)\n\
           ┃ • Temp directory does not exist\n\
           ┃ Set <b>$TMPDIR</b> to a writable directory to override.")
```

---

## Implementation Notes

- The `EditorError` variants currently lack fields for the file path and editor name on several variants. To produce the rich messages above, the enum should be extended with additional fields (e.g., `NonZeroExit { code: i32, editor: String, path: PathBuf }`, `Missing { path: PathBuf }`).
- The `Io` variant should be split or wrapped so callers can attach an operation tag (`"create temp file"`, `"read edited content"`) instead of forwarding a bare `std::io::Error`.
- All five variants are good candidates for the `BlockError` trait. They are user-facing (CLI `md edit` and Claudine prompt editing), and each has a clear remediation story that benefits from the header + block + hint structure.
- `StatusBlock::new(StatusState::Error)` automatically gets a red `┃` border via `Tailwind::Red500` — no manual `border_color()` call needed for error severity.
- Prose tokens like `<b>`, `<red>`, `<dim>` are available in both the `header()` and `hint()` strings (header goes through `Status::from_prose()`, hint through `Prose::new()`). The `body()` also accepts `Prose` as input.
