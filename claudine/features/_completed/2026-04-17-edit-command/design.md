# `--edit` Switch for Claudine Wrappers — Design

Author: Ken Snyder
Date: 2026-04-17
Status: Proposed

## Summary

Add a `--edit` boolean switch to every Claudine wrapper subcommand (`claudine
claude`, `codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`) that opens the
user's preferred editor, blocks until the editor closes, and then continues the
normal wrap pipeline with the edited text as the prompt.

The flag works with or without a seed prompt on the command line:

```text
claudine claude --edit
claudine claude "starting the prompt" --edit
claudine claude --edit "starting the prompt"
```

Editor resolution follows the same precedence Darkmatter's `md edit` already
uses (`$EDITOR` → `$VISUAL` → `sniff`-detected default priority list). The
only behaviour the two commands disagree on is the **target file**: Darkmatter
edits a user-supplied markdown document in place; Claudine edits a
throwaway temp file whose final contents become the prompt.

To avoid duplicating policy across the monorepo, the reusable half of
Darkmatter's `run_edit` — editor resolution, wait-flag selection, and the
launch-and-read-back loop — is extracted to a new `darkmatter::editor`
library module. Darkmatter's existing CLI delegates to it; Claudine's new
flag consumes it.

## Goals

1. Let a user compose a prompt in their editor for any wrap command.
2. Accept optional seed text from the command line and carry it into the
   editor buffer unchanged.
3. Block the wrap pipeline until the editor closes, then resume with the
   edited text as the prompt.
4. Reuse Darkmatter's editor resolution rules so users get one consistent
   experience across `md edit` and `claudine <wrapper> --edit`.
5. Detect installed editors via `sniff::programs::InstalledEditors` so the
   fallback list adapts to the actual host.
6. Factor the editor launch into `darkmatter::editor` so Darkmatter's
   existing `run_edit` and Claudine's new flag share one implementation.

## Non-Goals

1. Not changing Darkmatter's `md edit` user-facing behaviour (it still
   edits a user-supplied file in place).
2. Not adding `--edit` to the composition commands (`compose`,
   `inline-compose`, `sequence`). Those have their own markdown bodies
   and override grammars; adding editor support there is deferred to a
   follow-up feature.
3. Not persisting the edit buffer between sessions.
4. Not providing a comment/header template inside the buffer. What the
   user types (minus trailing whitespace) is exactly what reaches the
   provider.
5. Not implementing per-provider prompt-format hints (e.g. Claude vs.
   Codex). The prompt is plain text regardless of provider.
6. Not introducing a new top-level Claudine subcommand. `--edit` is
   strictly a modifier on existing wrapper subcommands.

## Current Baseline

### Darkmatter `md edit`

The reference implementation lives in
[`darkmatter/cli/src/commands.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/cli/src/commands.rs):

1. `run_edit(raw_file)` (lines 1056–1158) — orchestrates the whole flow.
2. `resolve_editor_command()` (lines 1165–1195) — `$EDITOR` → `$VISUAL` →
   `DEFAULT_EDITOR_PRIORITY` fallback, each candidate verified with
   `sniff::programs::find_program(cmd)`.
3. `wait_args_for_editor(binary)` (lines 1205–1223) — per-binary static
   table of flags that make GUI editors block (`--wait` for VS Code,
   Sublime, Zed, TextMate, BBEdit, JetBrains; `--block` for Kate; empty
   for terminal editors).
4. `DEFAULT_EDITOR_PRIORITY` (lines 1041–1048) — ordered
   `sniff::programs::Editor` list used when env vars are unset.

The editor pieces are **not currently exposed as a library API** —
everything is private to the `darkmatter-cli` binary. Claudine cannot
call them today without re-implementing or depending on the CLI crate.

### Claudine wrapper arg plumbing

All wrapper subcommands route through `WrapperArgs` in
[`claudine/cli/src/commands/wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs)
(lines 605–701). A few wrapper-level booleans (`--yolo`, `--interactive`,
`--quiet`, `--silent`, `--repo`, `--verbose`) are declared twice:

1. once as typed fields on `WrapperArgs`, and
2. once inside `extract_wrapper_flags_from_passthrough` (lines 3411+) so
   they can also be detected after the `trailing_var_arg` positional has
   started capturing.

The two sources are OR-merged in `run_provider_wrapper` around line 769.
`--edit` must plug into both.

Prompt extraction is handled by
`profile::extract_prompt_source_from_passthrough`
([`claudine/cli/src/commands/wrap/profile.rs:1878`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/profile.rs)).
It returns `(cleaned_args, PromptSource)` where `PromptSource` is:

```rust
pub(crate) enum PromptSource {
    None,
    Inline(String),
    InheritStdin,
}
```

This is the natural seam where the edit flow intercepts: replace
`PromptSource::None | Inline(seed)` with `PromptSource::Inline(edited)`.

### Existing dependencies

`claudine-cli` already depends on:

- `darkmatter` (lib) — so exposing the editor API from `darkmatter/lib`
  adds zero new cross-crate coupling.
- `sniff` — `InstalledEditors` is already available.
- `tempfile` — the staging file crate is already in scope.

No new workspace dependencies are introduced.

## User Experience

### CLI contract

```text
claudine <wrapper> [flags] [prompt] [-- agent-args...]
```

becomes, with the new flag:

```text
claudine <wrapper> [flags] [seed-prompt] --edit [-- agent-args...]
```

Concrete examples:

| Command | Behaviour |
|---------|-----------|
| `claudine claude --edit` | Empty buffer; user types the full prompt. |
| `claudine claude "draft of my question" --edit` | Buffer pre-seeded with `draft of my question`. |
| `claudine claude --edit "draft of my question"` | Same as above — flag order doesn't matter. |
| `claudine codex --edit --model gpt-5` | Edit first, then forward with the `--model` override. |
| `claudine claude --edit -- --dangerously-skip-permissions` | Edit first; post-`--` args are opaque passthrough. |

`--edit` is always interpreted by Claudine when it appears **before** the
`--` separator. If a provider CLI also exposes an `--edit` flag and the
user wants to forward it, the **sole** escape hatch is to place it after
`--`. Claudine does not inspect provider argument grammars and does not
expose an alternate flag name; pre-`--` `--edit` is unconditionally
Claudine-owned.

### Editor lifecycle visible to the user

1. Claudine first verifies that both stdin and stdout are attached to a
   TTY. If either is not (piped stdin, redirected stdout, etc.),
   Claudine exits with `--edit requires an interactive terminal`
   **before** any editor is launched.
2. Claudine prints a one-line status to stderr before launching the
   editor: `✎ opening <editor> for prompt…`.
3. Editor opens with the seed (if any) already in the buffer.
4. Claudine's process blocks. GUI editors receive the
   `wait_args_for_editor` flag so the CLI wrapper doesn't detach.
5. When the user closes the buffer, Claudine reads the file, trims
   trailing whitespace, and resumes the wrap pipeline.
6. A non-zero editor exit or a deleted buffer file aborts the session
   with a formatted error (see "Error handling" below). An empty
   (whitespace-only after `trim_end()`) buffer is a **clean abort**:
   Claudine exits `0` with a single stderr line `prompt empty; aborted`
   and does not invoke the provider. This follows the `git commit`,
   `visudo`, and `crontab -e` conventions.
7. The temp file is deleted on return via
   `tempfile::NamedTempFile::drop`.

### Section ordering (stderr)

The live semantic sink's canonical section ordering (see the claudine
skill's "Claudine CLI Output" memory) is **not** modified. The edit
status line prints **before** the execution header — it describes a
pre-session hand-off to the editor, not a session event. No blank-line
separators inside the existing 9-section model change.

### Dry run

`--edit` + `--dry-run` launches the editor (because the edited text is
an input to the dry-run preview), then prints the preview without
invoking the provider. A `--silent` run suppresses the `✎ opening …`
line but still blocks.

## Editor Resolution

Semantics are identical to Darkmatter's `resolve_editor_command()`:

1. `$EDITOR`: if set, take the first whitespace-separated token as the
   binary and verify with `sniff::programs::find_program`. If found,
   use the full `$EDITOR` value verbatim (preserves user flags like
   `code --new-window`).
2. `$VISUAL`: same rule if `$EDITOR` was unset or its binary missing.
3. Fallback list (Darkmatter's `DEFAULT_EDITOR_PRIORITY`, lightly
   reordered to bias terminal editors for CLI use):
   `nvim, vim, code, sublime, zed, nano, helix, vi, emacs,
   code-insiders, codium, bbedit, textmate, phpstorm, idea, pycharm,
   webstorm, clion, goland, rider`.
4. Wait-flag injection per `wait_args_for_editor(binary)` — unchanged
   from Darkmatter's static table. Terminal editors get no extra flag.

If all three paths fail, Claudine returns an error with the
`sniff software editors install` hint (see "Error handling").

## Temp File Workflow

```text
1. Build NamedTempFile in the platform temp dir with suffix ".md"
     (extension chosen so GUI editors pick up markdown highlighting)
2. Write seed text to the file (empty string if no seed)
3. Flush + sync; close the write handle so editors see a complete file
4. Launch editor with the file path and any wait flags
5. Block on Command::status()
6. On editor exit(0):
     a. Re-read the file as UTF-8
     b. Validate existence (a missing file is `EditorError::Missing`).
        Emptiness-after-trim is NOT an error: it is a clean-abort
        signal the caller handles by returning `Ok(None)`.
     c. trim_end() the content (we preserve leading whitespace —
        some users deliberately lead with a blank line)
     d. If the trimmed content is empty, return `Ok(None)`; otherwise
        return `Ok(Some(content))`.
7. NamedTempFile drops → temp file removed
```

Rationale for using `tempfile::NamedTempFile` over a fixed path:

- The buffer is ephemeral by contract; there is no recovery story for a
  crashed editor session (future work may add one).
- `NamedTempFile` guarantees cleanup even if Claudine is killed before
  the editor returns, minimising clutter in `/tmp`.
- The `.md` suffix is applied via `tempfile::Builder::new().suffix(".md")`.

## Library Reuse Plan — `darkmatter::editor`

### Motivation

The reusable parts of `run_edit` are:

1. `resolve_editor_command()` — env + sniff detection.
2. `wait_args_for_editor(binary)` — per-binary wait flags.
3. The spawn-and-wait loop around `std::process::Command`.

The non-reusable parts are target-file resolution (`FileReference`) and
the Darkmatter-specific "print canonical path" success action.

We factor the reusable parts into a new module.

### New API surface

`darkmatter/lib/src/editor/mod.rs`:

```rust
/// Resolve the editor command using $EDITOR, $VISUAL, then sniff-detected
/// installed editors (in the Darkmatter-curated priority order).
pub fn resolve_editor_command() -> Result<String, EditorError>;

/// Static wait flags for GUI editors. Empty slice for terminal editors.
pub fn wait_args_for_editor(binary: &str) -> &'static [&'static str];

/// Open a file path in the resolved editor and block until it closes.
/// Does not touch the file contents.
pub fn launch_editor_on_path(path: &Path) -> Result<(), EditorError>;

/// Edit text in an ephemeral temp file. Seed the buffer with `initial`,
/// launch the editor, and return the edited text trimmed at the end.
///
/// `suffix` is appended to the temp file name (".md" by convention for
/// Markdown-flavoured prompts).
///
/// Returns `Ok(Some(text))` when the user saved non-empty content, and
/// `Ok(None)` when the buffer is empty after `trim_end()` — callers
/// should treat `Ok(None)` as a clean user-initiated abort, not an
/// error.
pub fn edit_text(initial: &str, suffix: &str) -> Result<Option<String>, EditorError>;

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("no editor found; set $EDITOR or $VISUAL, or install one of: nvim, vim, code, nano")]
    NoEditorFound,
    #[error("editor exited with status {0}")]
    NonZeroExit(i32),
    #[error("edited file was deleted during editing")]
    Missing,
    #[error("failed to launch editor {editor:?}: {source}")]
    LaunchFailed { editor: String, #[source] source: std::io::Error },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

`DEFAULT_EDITOR_PRIORITY` moves into the new module. Darkmatter's CLI
retains a thin `run_edit(raw_file)` that resolves the file path via
`FileReference`, then delegates to
`darkmatter::editor::launch_editor_on_path`. The CLI still owns the
"print canonical path on success" policy.

### Backwards compatibility

Darkmatter's `md edit` behaviour is unchanged from the user's
perspective. `run_edit`'s tests continue to pass because the observable
contract (file path resolution, canonical-path output, empty-file error
text) is retained in the CLI handler.

## Claudine Integration

### 1. Add typed flag to `WrapperArgs`

In [`claudine/cli/src/commands/wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/wrap/mod.rs)
around line 687 (after the `strict` flag, before `passthrough`):

```rust
/// Open the user's preferred editor to compose the prompt. Any inline
/// prompt on the command line is used as the initial buffer content.
#[arg(long, conflicts_with = "interactive")]
pub edit: bool,
```

The `conflicts_with = "interactive"` is deliberate — see "Interaction
rules" below.

### 2. Extend the passthrough extractor

In `ExtractedWrapperFlags` (line 3368):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ExtractedWrapperFlags {
    yolo: bool,
    interactive: bool,
    repo: bool,
    quiet: bool,
    silent: bool,
    verbose: bool,
    edit: bool,            // NEW
    operation: Option<String>,
}
```

Add the match arm in `extract_wrapper_flags_from_passthrough_with_boundary`
(around line 3431):

```rust
"--edit" => {
    extracted.edit = true;
    remove_indices.push(i);
}
```

### 3. OR-merge in `run_provider_wrapper`

Around line 775:

```rust
let edit_requested = args.edit || extracted.edit;
```

### 4. Invoke the editor between prompt extraction and delivery

In `run_provider_wrapper`, immediately **after** the call to
`extract_prompt_source_from_passthrough` at line 791 and **before** the
`has_prompt` flag is computed at line 797, insert:

```rust
if edit_requested {
    // Preflight: --edit requires an interactive TTY on both ends.
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(eyre!("--edit requires an interactive terminal"));
    }

    // InheritStdin is already impossible here because stdin is a TTY,
    // but keep the match for defence-in-depth.
    match &prompt_source {
        profile::PromptSource::InheritStdin => {
            return Err(eyre!("--edit requires an interactive terminal"));
        }
        _ => {}
    }

    let seed = prompt_source.as_inline().unwrap_or("").to_string();

    if !silent_requested {
        // Status line printed before the execution header.
        eprintln!("✎ opening editor for prompt…");
    }

    match darkmatter::editor::edit_text(&seed, ".md")
        .wrap_err("failed to edit prompt")?
    {
        Some(text) => {
            prompt_source = profile::PromptSource::Inline(text);
        }
        None => {
            // Clean abort: empty buffer after trim_end().
            if !silent_requested {
                eprintln!("prompt empty; aborted");
            }
            return Ok(());
        }
    }
}
```

Everything downstream — system prompt resolution, MCP injection, live
sink rendering — is unchanged. Because the effective `PromptSource` is
now always `Inline`, `has_prompt` becomes true and the session defaults
to non-interactive (matching the existing wrap semantics and the
`conflicts_with` rule above).

### 5. Help text & completions

- Update the synthesised help block in `print_wrapper_help` near
  `mod.rs` line 3345 to mention `--edit` in the "Wrapper Options"
  section.
- Regenerate shell completions by running `just` at the workspace root
  (completion generation already happens during `just build`).

## Interaction Rules

| Other flag | Behaviour |
|------------|-----------|
| `--interactive` / `-i` | Hard conflict. Clap rejects at parse time via `conflicts_with`. Most provider CLIs can't seed an initial user turn into an interactive TTY session, so silently falling back to interactive with a "first turn injected" hack is fragile. Force users to choose. |
| `--quiet` / `-q` | Suppresses info lines but still prints the `✎ opening editor…` status. |
| `--silent` | Suppresses the `✎ opening editor…` status. Editor still launches. |
| `--dry-run` | Editor launches; dry-run preview renders with the edited text. |
| Non-interactive session | Hard-fail at preflight. `--edit` requires both stdin and stdout to be attached to a TTY; piped stdin, redirected stdout, or any non-TTY handle produces `--edit requires an interactive terminal` before the editor is launched. |
| Provider-level `--edit` | Provider CLIs that expose their own `--edit` flag must receive it **after** the `--` separator. Pre-`--` `--edit` is always interpreted by Claudine; there is no provider-awareness and no alternate flag name. |
| `--append-system-prompt` / `--replace-system-prompt` | Independent. Both flags still apply to the system prompt; `--edit` only affects the user prompt. |
| `--mcp`, `--use`, `--strict` | Independent. MCP tag resolution runs on the edited prompt exactly as it does for any inline prompt. |
| `--timeout` | Works normally. Editor time is not counted against the provider timeout; the timeout starts when the child process launches. |
| `--yolo`, `--sandbox`, `--operation` | Independent. |

## Error Handling

All editor errors bubble up via `color_eyre::Result` and are rendered
through the existing error formatter (`<red><b>Error:</b></red>` +
deduplicated cause chain), consistent with the homelab CLI error
conventions.

Specific error messages:

1. **No editor found** — `EditorError::NoEditorFound` renders as:
   > Error: no editor found; set $EDITOR or $VISUAL, or install one
   > of: nvim, vim, code, nano
   >
   > Tip: run `sniff software editors install` to pick one interactively.
2. **Editor non-zero exit** — `EditorError::NonZeroExit(code)` renders
   as `editor exited with status <code> (prompt abandoned)`.
3. **Missing file** — `EditorError::Missing` renders as `prompt buffer
   was deleted during editing`.
4. **Launch failure** — `EditorError::LaunchFailed { editor, source }`
   renders as `failed to launch <editor>: <io error>`.
5. **Non-interactive session** — raised before the editor launches when
   stdin or stdout is not a TTY; renders as `--edit requires an
   interactive terminal`.

Note: an empty buffer is **not** an error. It is a clean abort handled
directly in `run_provider_wrapper` (stderr `prompt empty; aborted`,
exit `0`); see "Claudine Integration → step 4" and the editor
lifecycle.

None of these cases leak a backtrace or location line (per the
homelab-style error handling memory).

## Testing

### Library tests (`darkmatter/lib/src/editor/`)

1. `resolve_editor_command` priority tests using `serial_test::serial`
   to isolate `$EDITOR` / `$VISUAL` mutation:
   - env-only: `EDITOR=echo` → returns `"echo"`.
   - env missing binary: `EDITOR=/non/existent` → falls through to
     `$VISUAL` or the sniff list.
   - both env unset: first installed editor from the fallback list.
2. `wait_args_for_editor` lookup table test (one assert per known
   binary).
3. `edit_text` happy path with a mock `EDITOR` that appends a known
   string:
   ```bash
   EDITOR='sh -c "echo appended >> \"$0\""'
   ```
   The test asserts the returned value is
   `Ok(Some(seed + "appended\n"))` (trimmed at the end).
4. `edit_text` empty-buffer path: mock `EDITOR` that truncates the
   file. The test asserts the returned value is `Ok(None)` — this is
   a clean abort, not an error.
5. `edit_text` non-zero exit path: mock `EDITOR` that `exit 1`s.

### Claudine CLI integration tests

Use `assert_cmd` + `predicates` (already the convention per the
monorepo memory):

1. `--edit` with piped stdin → exits non-zero with
   `--edit requires an interactive terminal` on stderr; editor is not
   launched. (Specific case of the broader non-TTY preflight rule.)
2. `--edit` with stdout redirected to a file → exits non-zero with the
   same `--edit requires an interactive terminal` message; editor is
   not launched.
3. `--edit` + `--interactive` → clap rejects at parse time with
   `cannot be used with '--interactive'`.
4. `--edit` with `EDITOR` set to a script that writes `"edited
   prompt"` → wrap pipeline continues in `--dry-run` mode and the
   preview shows `edited prompt` as the prompt.
5. `--edit "seed text"` with a mock editor that appends `" + more"` →
   preview shows `seed text + more`.
6. `--edit` with a mock editor that truncates the file → Claudine
   exits `0` with `prompt empty; aborted` on stderr, no provider is
   invoked, and no preview is rendered.
7. Post-`--` `--edit` is the **sole** escape hatch for forwarding the
   token to the provider. The extractor must not consume it, and
   Claudine's own edit flow must not activate. Test:
   `claudine claude -- --edit` → preview's passthrough contains `--edit`
   verbatim, Claudine's `edit_requested` is `false`, and no editor is
   launched.

### Mock editor pattern

Tests set `EDITOR` to a shell script path via `tempfile` + chmod +x, or
to an inline `sh -c` that reads `$0` as the file path argument. The
`serial_test` crate keeps env-var-mutating tests from colliding.

## Out of Scope / Future Work

1. **`--edit` on composition commands** (`compose`, `inline-compose`,
   `sequence`): the natural target is the composed body or the
   frontmatter `prompt`, not the command line. Different semantics,
   separate feature.
2. **Edit-buffer recovery** after Claudine crashes (write to a stable
   path under `~/.claudine/drafts/` instead of `NamedTempFile`).
3. **Interactive seeded turns**: feeding an edited prompt into an
   interactive session as the first user turn. Requires per-provider
   hacks (stuffing the TTY, `expect`-style injection) that are out of
   scope.
4. **Per-provider buffer templates** (e.g. thinking-budget header for
   Claude). Today the buffer is pure text.
5. **Editor picker**: no interactive "which editor?" prompt on first
   run. Resolution is fully automatic.

## File-Level Change List

### New

- `darkmatter/lib/src/editor/mod.rs` — new module with
  `resolve_editor_command`, `wait_args_for_editor`, `launch_editor_on_path`,
  `edit_text` (returns `Result<Option<String>, EditorError>`; `Ok(None)`
  signals a clean empty-buffer abort), `EditorError` (no `Empty`
  variant — empty buffers are not errors), and the
  `DEFAULT_EDITOR_PRIORITY` constant.
- `darkmatter/lib/src/editor/tests.rs` (or `#[cfg(test)] mod tests` in
  the module) — library-level tests per the plan above.
- `claudine/features/2026-04-17-edit-command/design.md` — this file.

### Modified

- `darkmatter/lib/src/lib.rs` — `pub mod editor;`.
- `darkmatter/cli/src/commands.rs` — `run_edit` keeps file-reference
  resolution and the canonical-path print, but delegates editor launch
  to `darkmatter::editor::launch_editor_on_path`. Remove the now-unused
  `resolve_editor_command`, `wait_args_for_editor`, and
  `DEFAULT_EDITOR_PRIORITY` from the CLI crate.
- `claudine/cli/src/commands/wrap/mod.rs`:
  - `WrapperArgs::edit` field (clap-declared, `conflicts_with = "interactive"`).
  - `ExtractedWrapperFlags::edit` field.
  - `extract_wrapper_flags_from_passthrough_with_boundary` — new
    `"--edit"` arm.
  - `run_provider_wrapper` — OR-merge `edit_requested`; insert the
    editor invocation block between `extract_prompt_source_from_passthrough`
    and the downstream `has_prompt` computation. The block performs a
    `std::io::IsTerminal` preflight (both stdin and stdout must be
    TTYs) before launching the editor, and handles `Ok(None)` from
    `edit_text` as a clean abort (stderr `prompt empty; aborted`,
    exit `0`, provider not invoked).
  - `print_wrapper_help` — document the new flag.
- No change to `claudine/lib/` (pure CLI feature).

### Dependency additions

None. `claudine-cli` already depends on `darkmatter` (lib),
`sniff`, and `tempfile`.

## Open Questions

1. Should the fallback editor priority list live in Claudine config so
   users can override it globally? **Proposed answer:** not in v1.
   `$EDITOR` / `$VISUAL` already cover explicit preference; the
   fallback is a last resort. Revisit if users report churn.
2. Should we include a commented header in the buffer (e.g.
   `# Close the file to send. Leave empty to abort.`)? **Proposed
   answer:** no. Most providers don't strip markdown comments, so the
   header would leak into the prompt. Users who need guidance can
   learn once.
3. Should `--edit` be a single-character alias (e.g. `-e`)? **Proposed
   answer:** no, `-e` is too likely to collide with provider flags.
   Long-only is safer.
