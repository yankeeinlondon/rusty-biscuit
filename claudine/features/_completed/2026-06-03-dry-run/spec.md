the `--dry-run` flag has been available for `claudine compose`, `claudine inline-compose` and `claudine sequence` for some time but NEVER used because it was never actually asked for and to this day I still have no idea what it was designed to do!

Ironically, now what we need is a "working" `--dry-run` feature for all of the composition commands in Claudine:

## Pipeline Scope

`--dry-run` exercises composition through provider/model resolution, then
returns before launch wiring. This means:

- Schema validation runs normally
- Interactive prompts (including shell-command approval) behave exactly as in normal mode
- Harness pre-checks run normally
- Provider selection and agent resolution run normally
- Selected-executable availability validation and path resolution do not run;
  the selected provider need not be installed or present on `PATH`
- Installed-provider inventory may still run when automatic selection or the
  rendered agent-resolution breakdown requires it
- **The provider is never launched** — no request is sent to any agentic CLI

This makes `claudine compose --dry-run` suitable for CI gating and reliable rehearsal.

## Core Behavior

1. Using `--dry-run` with any compositional claudine command should NOT send anything to an agentic CLI
2. The target document is composed normally through the full pipeline, with the following clarifications:
    - Shell commands in the compose document graph are **executed for real**; they produce actual side effects. All unapproved commands go through the same interactive approval process as normal mode.
    - The YAML frontmatter has:
        - all state from caller and document merged together,
        - all interpolation completed
        - all shell expansion completed
    - The document's _compose_ body content has:
        - all interpolation, conditional blocks, shell expansions, etc. completed

## Non-TTY Behavior

In non-TTY environments (e.g., CI pipelines), interactive approval prompts cannot be displayed. When `--dry-run` encounters an unapproved shell command in non-TTY mode:

- The process exits immediately with a non-zero status
- A clear error message is written to stderr:

```
Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration.
```

where `X` is the shell command that would have required approval.

This behavior ensures that CI gating tests the exact same path as production. If production would prompt for approval, the dry-run fails — that is the gate working correctly.

To bypass this in non-TTY environments, either:
- Run with `--yolo` to auto-approve all shell commands
- Pre-approve the command in your configuration

## Output Behavior

Dry-run output is split between `stdout` and `stderr` to follow Unix conventions:

- **stdout**: the composed document body (the data product of the command)
- **stderr**: the rendered YAML frontmatter and the metadata table (ancillary status information)

This design makes `claudine compose --dry-run > output.md` capture only the composed body.

### `--quiet` and `--silent` flags

Both `--quiet` and `--silent` flags have **no effect** when `--dry-run` is active. The full dry-run output (composed body to stdout, frontmatter and metadata to stderr) is always rendered regardless of these flags.

## Sequence Behavior

When `--dry-run` is used with `claudine sequence`, the dry-run exercises the entire sequence as a single logical command:

- Each document in the sequence is composed and rendered with its own frontmatter and metadata
- Documents are processed in order, with section dividers written to stderr between each document's metadata block (e.g., `=== Document 2 of 3 ===`)
- **stdout** contains all composed document bodies concatenated in order
- **stderr** contains the rendered frontmatter and metadata table for each document
- If any document fails during composition, the error is rendered to stderr and the sequence stops immediately (same behavior as normal mode)

This makes `claudine sequence --dry-run > output.md` capture all composed bodies in sequence order.

## Rendering

When stderr is not suppressed (default dry-run mode):

- The finalized YAML frontmatter will be highlighted and rendered to **stderr**
- A metadata table will be rendered to **stderr** after the frontmatter, containing:
    - Document:
        - if the Frontmatter defines a `name` property then we'll show this here
        - if `name` is not set then we'll use the relative path to the markdown document
        - in both cases we'll render this in blue and create an OSC8 link to the document
    - Description:
        - if the Frontmatter defines a `description` property then we'll show it in italics and dimmed
    - Agent:
        - the agent chosen (if one was)
        - `<i><yellow>interactive</yellow></i>` if no agent has been identified yet
    - Model:
        - the model chosen (if one was)
        - `<i><dim>default</dim></I>` if no model was chosen
    - YOLO:
        - a boolean `<green>true</green>`/`<red>false</red>` indicator of whether YOLO mode was used
    - Area:
        - this key/value is ONLY shown if called inside of a monorepo
        - shows the currently focused "area" (same as `ctx.area` context variable)

## Error Handling

If an error occurs during composition, the error is rendered to **stderr** instead of the frontmatter and metadata table, and the process exits with a non-zero status.

## Acceptance Criteria

- [x] `claudine compose --dry-run doc.md` composes the document through provider/model resolution without requiring or launching a provider executable — `compose_dry_run_body_only_on_stdout_metadata_on_stderr`, `compose_dry_run_perf_renders_report_without_agent_execution`
- [x] Shell commands referenced in the document are executed for real; their output is available for interpolation and expansion — `compose_dry_run_yolo_bypasses_shell_gate` (body on stdout contains the executed command output), `level2_pty_dry_run_shell_approval_prompt_appears_and_allows`
- [x] The composed document body is written to stdout — `compose_dry_run_body_only_on_stdout_metadata_on_stderr`
- [x] The rendered frontmatter and metadata table are written to stderr — `compose_dry_run_body_only_on_stdout_metadata_on_stderr`
- [x] `claudine compose --dry-run --quiet doc.md` and `claudine compose --dry-run --silent doc.md` render the full dry-run output; `--quiet` and `--silent` have no effect in dry-run mode — `compose_dry_run_quiet_and_silent_are_no_op`
- [x] Interactive approval prompts for unapproved shell commands appear exactly as in normal mode — `level2_pty_dry_run_shell_approval_prompt_appears_and_allows` (PTY byte-injection: prompt fires under `--dry-run` and approving runs the command) and `level2_pty_dry_run_approval_prompt_matches_normal_mode` (PTY byte-injection: captures the rendered prompt region in both normal and `--dry-run` mode and asserts they are byte-identical), plus the real terminal-emulator complement `level2_dry_run_approval_prompt_matches_normal_mode_in_tmux` (drives the prompt through `tmux`, captures the displayed pane, and asserts the normal-mode and `--dry-run` prompt surfaces match and carry styling)
- [x] In non-TTY mode, an unapproved shell command causes dry-run to exit non-zero with the error: "Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration." — `compose_dry_run_non_tty_unapproved_shell_emits_gate_error`
- [x] Schema validation failures, missing files, and other composition errors are rendered to stderr and the process exits non-zero — `compose_dry_run_missing_file_errors_to_stderr_with_clean_stdout`, `inline_compose_dry_run_schema_error_to_stderr_with_clean_stdout`
- [x] The metadata table includes Document (with OSC8 link), Description, Agent, Model, YOLO, and Area (when in a monorepo) fields — `dry_run.rs` unit tests (`table_shows_description_when_present`, `table_shows_area_when_present`, `agent_*`, `model_*`, `yolo_true_and_false`, `document_uses_name_when_set`/`..._path_when_name_absent`; the Document cell is built as a blue `<a href="file://…">` OSC8 link) + `compose_dry_run_body_only_on_stdout_metadata_on_stderr` (rows on stderr) + L2 real-terminal capture: `level2_dry_run_metadata_table_renders_styled_in_tmux` (tmux: blue Document, italic+dim Description, red `false` YOLO asserted from `frame.raw`) and `level2_dry_run_document_cell_renders_osc8_link_in_wezterm` (WezTerm: the Document cell emits a real OSC8 `file://` hyperlink)
- [x] `claudine sequence --dry-run` composes and renders all documents in the sequence — `sequence_dry_run_concatenates_bodies_with_dividers`
- [x] Each document in a sequence dry-run is separated by a section divider on stderr (e.g., `=== Document N of M ===`) — `sequence_dry_run_concatenates_bodies_with_dividers`
- [x] All composed document bodies from a sequence dry-run are concatenated to stdout in order — `sequence_dry_run_concatenates_bodies_with_dividers`
- [x] If any document in a sequence dry-run fails, the error is rendered to stderr and the sequence stops immediately — `sequence_dry_run_fail_fast_on_composition_error`
- [x] `claudine sequence --dry-run --quiet` and `claudine sequence --dry-run --silent` render the full dry-run output; `--quiet` and `--silent` have no effect in dry-run mode — `sequence_dry_run_quiet_and_silent_are_no_op`
