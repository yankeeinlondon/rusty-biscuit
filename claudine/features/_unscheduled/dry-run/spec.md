the `--dry-run` flag has been available for `claudine compose`, `claudine inline-compose` and `claudine sequence` for some time but NEVER used because it was never actually asked for and to this day I still have no idea what it was designed to do!

Ironically, now what we need is a "working" `--dry-run` feature for all of the composition commands in Claudine:

## Pipeline Scope

`--dry-run` exercises the **full composition pipeline up to but not including provider launch**. This means:

- Schema validation runs normally
- Interactive prompts (including shell-command approval) behave exactly as in normal mode
- Harness pre-checks run normally
- Provider selection and agent resolution run normally
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

> ⚠️ **Warning**: Because `--dry-run` executes shell commands for real (required to produce accurate shell expansion), users must understand that file modifications, network requests, and other side effects from shell commands **will occur**. This is not a sandbox.

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

### `--silent` mode

When `--silent` is passed alongside `--dry-run`:

- Suppress all stderr output (frontmatter rendering, metadata table, and sequence section dividers)
- Emit **only** the composed document body to stdout

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

- [ ] `claudine compose --dry-run doc.md` composes the document through the full pipeline but does not launch a provider
- [ ] Shell commands referenced in the document are executed for real; their output is available for interpolation and expansion
- [ ] A warning is displayed when any shell command is executed during a dry-run, informing the user that side effects are real
- [ ] The composed document body is written to stdout
- [ ] The rendered frontmatter and metadata table are written to stderr
- [ ] `claudine compose --dry-run --silent doc.md` emits only the composed body to stdout and nothing to stderr
- [ ] Interactive approval prompts for unapproved shell commands appear exactly as in normal mode
- [ ] In non-TTY mode, an unapproved shell command causes dry-run to exit non-zero with the error: "Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration."
- [ ] Schema validation failures, missing files, and other composition errors are rendered to stderr and the process exits non-zero
- [ ] The metadata table includes Document (with OSC8 link), Description, Agent, Model, YOLO, and Area (when in a monorepo) fields
- [ ] `claudine sequence --dry-run` composes and renders all documents in the sequence
- [ ] Each document in a sequence dry-run is separated by a section divider on stderr (e.g., `=== Document N of M ===`)
- [ ] All composed document bodies from a sequence dry-run are concatenated to stdout in order
- [ ] If any document in a sequence dry-run fails, the error is rendered to stderr and the sequence stops immediately
- [ ] `claudine sequence --dry-run --silent` suppresses all stderr output (section dividers, frontmatter, and metadata tables)
