# Claudine Composition

Claudine supports the ability to _compose_ content by leveraging the Darkmatter library's powerful composition features and routing the result through a wrapper-grade execution pipeline.

Two canonical commands:

- **`claudine compose [flags] <arg>...`** — direct (chained) composition
- **`claudine inline-compose [flags] <arg>...`** — inline composition

Both commands share the same five-stage pipeline and inherit full wrapper-grade behavior: environment setup, harness detection, structured streaming, and handler-driven recovery.

Because composition flows through the same execution path as `claudine claude` / `codex` / etc., it inherits every behavior of the live stderr surface documented in [Non-Interactive Sessions](non-interactive-sessions.md):

- **Tool call rendering** — `→ Name(summary)` / `← Name(slot)` with shell-name prefixing for `Bash` / `shell` / `run_command` and `description → subject → prompt → task` field order for `Task`.
- **Idle flush** — buffered assistant markdown is flushed before the next heartbeat status line whenever the block buffer has been idle for at least the heartbeat silence window (default 30 s), so a dangling final paragraph never sits invisible while a slow-to-close provider waits to exit.
- **Typed error rendering** — `SemanticEvent::Error` is rendered as a colored `BlockQuote` whose label and border come from `SemanticErrorKind` (`Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, `Unknown`).
- **Reasoning / thinking** — provider reasoning (Claude, Codex, OpenCode, Gemini, Qwen) renders into `Section::Thinking` as a `BlockQuote` with the wider `▌ ` border that matches the System Prompt and Agent Prompt sections.

### Positional Arguments

Each command accepts exactly one file reference plus zero or more `key=value`
setters, in any order:

```sh
claudine compose @prompts/review.md review=review.md
claudine compose review=review.md @prompts/review.md
claudine inline-compose draft=false @notes/update.md
```

A token is a setter when it contains `=` and its key starts with an ASCII
letter or `_` and contains only letters, digits, `_`, or `-`. Dot-paths and
path-like tokens (for example `foo.bar=baz`) are not setters and are treated
as file-reference candidates.

Setter values are parsed as JSON5 first and fall back to strings when JSON5
parsing fails, so `count=3`, `enabled=true`, `tags=["a","b"]`, and
`review=review.md` all resolve to their natural types.

Inline setters override matching keys from `--set`. For `sequence`, reserved
per-step overlay keys still win over both `--set` and shorthand setters.

## Direct Composition

Direct composition takes a Markdown file, composes it through Darkmatter, and sends the composed content as a prompt to an agentic CLI. No files are mutated.

```sh
claudine compose @commit.md
claudine compose --codex @commit.md
```

Steps:

1. **Resolve** — resolve the file reference using `biscuit-file::FileReference` (supports `@` magic paths, repo-relative, monorepo-package-relative, and absolute paths)
2. **Compose** — run the Markdown through Darkmatter's compose pipeline (transclusion, interpolation, shell commands, conditionals)
3. **Prepare** — extract the effective (composed) frontmatter; this is the single source of truth for all downstream decisions
4. **Select provider** — choose which agentic CLI to use (see Provider Selection below)
5. **Execute** — run a non-interactive session (or interactive with `-i`) through the wrapper-grade pipeline

The composed prompt is sent to the provider. Output streams to the terminal with Markdown-to-terminal rendering in non-interactive mode.

## Inline Composition

Inline composition uses the `prompt` frontmatter property as input and replaces the document's body with the provider's output.

```sh
claudine inline-compose @research.md
claudine inline-compose --claude @research.md
```

Steps:

1. **Resolve** — resolve the file reference
2. **Validate permissions** — confirm read + write access to the file
3. **Compose** — extract the `prompt` property, compose through Darkmatter, append inline guardrails
4. **Prepare** — extract effective frontmatter, capture pre-execution hashes for closure
5. **Select provider** — choose the agentic CLI
6. **Execute** — run the provider session
7. **Closure** — Claudine rewrites the file:
   - The provider returns replacement body content only (no frontmatter)
   - Original frontmatter properties are preserved byte-for-byte
   - If the provider modified an existing frontmatter property, Claudine reverts it to the original value and emits a warning
   - If the provider added a new frontmatter property, Claudine merges it into the document (inserted before `last_updated`)
   - `last_updated` is set to today's date (local time, `YYYY-MM-DD`)
   - The file is written atomically
   - A cleanup pass normalizes the body markdown without touching frontmatter

### Inline Conventions

- **`prompt`** (required) — the prompt text; composed through Darkmatter before execution
- **`last_updated`** — auto-updated by Claudine on each successful write
- **`agent`** — optional provider hint (see Provider Selection)
- **`policy`** — content freshness policy (coming soon)
- **`blast_radius`** — list of source files that trigger re-generation when changed

## Provider Selection

Both commands use a deterministic precedence chain:

1. **Explicit flag** (`--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`) — highest priority
2. **Single installed** — if only one provider remains after `--exclude` filtering
3. **Frontmatter hint** — the `agent` property in the effective (composed) frontmatter, fuzzy-matched against provider names
4. **Config favorite** — `settings.linking.preference[0]` from `~/.claudine/config.json` or `<repo>/.claudine/config.json`
5. **Interactive chooser** — if a TTY is available, prompt the user; otherwise error

### The `--interactive` Flag

`-i` / `--interactive` controls the **provider session mode**, not provider selection. The composed prompt is still prepared first, then passed as the initial message for an interactive session.

> **Note:** `inline-compose -i` is provider-gated. Claudine allows it only when the selected provider can recover the final assistant message for the inline rewrite path.

### The `--exclude` Flag

`--exclude <PROVIDER>` removes a provider from automatic selection (repeatable). Explicit flags (`--codex`, etc.) override exclusions.

## Harness: Validations and Handlers

Composed documents can declare **pre-checks**, **post-checks**, **timeouts**, and **handlers** in their frontmatter. When present, Claudine activates a harness that gates provider execution behind validation rules and can recover from failures automatically.

The harness reads from the **effective (composed) frontmatter** — not from the raw source file. This means composition can inject harness properties dynamically via Darkmatter transclusion or interpolation.

### Pre-checks and Post-checks

Pre-checks run before the provider launches; post-checks run after:

```yaml
pre_checks:
  - file_exists: "@docs/plan.md"
  - dir_exists: "@src/components"
post_checks:
  - file_changed: "@docs/plan.md"
  - response_includes: "## Summary"
```

Available validations include filesystem checks (`file_exists`, `dir_exists`, `json_file_exists`, `yaml_file_exists`, `toml_file_exists`, `has_write_permission`), git checks (`no_dirty_source_code`, `has_dirty_source_code`), post-only file comparisons (`file_changed`, `file_unchanged`), frontmatter comparisons (`frontmatter_prop_changed`, `frontmatter_prop_unchanged`, `frontmatter_prop_equals`), response checks (`response_length_at_least`, `response_length_at_most`, `response_includes`, `response_missing`), and shell commands (`shell_command`).

### Timeouts

The `timeout` frontmatter property sets a per-execution deadline:

```yaml
timeout: 5m
```

Accepts `s`/`sec`/`seconds`, `m`/`min`/`minutes`, `h`/`hr`/`hours` units.

### Handlers

Handlers define recovery actions when failures occur:

```yaml
handle_timeout:
  resume:
    prompt: "Continue from where you stopped."

handle_agent_failure:
  retry:
    prompt_suffix: "The previous attempt failed. Please try again."
    retries: 3

handle_file_exists:
  "@docs/plan.md":
    redirect:
      file: "./fallback.md"
```

Four handler actions are available:

- **retry** — re-run the same prompt with optional modifications
- **resume** — continue from the previous session (provider must support session resume)
- **redirect** — switch to a different source document
- **deviate** — execute a shell command, then re-evaluate post-checks

A programmatic `handle` property accepts a shell command that receives failure context on stdin and returns a handler action as JSON on stdout.

### Shell Policy

All shell commands — `::shell` directives in the template, top-level frontmatter `$(cmd)` expressions, `shell_command` validations, and `deviate`/`handle` declarations — are approved upfront during the pre-flight phase, before the provider session starts. See [Pre-Flight Shell Approval](pre-flight-checks.md) for the full flow.

## Retired Interfaces

The following interfaces have been removed and replaced by the two canonical commands above:

| Removed | Replacement |
|---------|-------------|
| `claudine <agent> --compose <file>` | `claudine compose --<agent> <file>` |
| `claudine <agent> --frontmatter-prompt <file>` | `claudine inline-compose --<agent> <file>` |
| `claudine compose inline <file>` | `claudine inline-compose <file>` |
| `claudine compose-inline <file>` | `claudine inline-compose <file>` |
| `AGENT` environment variable | `--claude`, `--codex`, etc. flags or `agent` frontmatter |

**Removed without replacement:**

| Removed | Reason |
|---------|--------|
| `claudine <agent> --prompt-file <file>` | Sent file content verbatim as a prompt. `claudine compose` performs full Markdown composition (frontmatter, template substitution, `::shell` directives) so it is not a drop-in replacement. Callers that need raw prompt delivery should use the provider CLI directly. |

## Sequence Composition

Sequence composition runs a single source document multiple times, once per step in a defined list, with step-specific state injected into the composition context on each run.

```sh
claudine sequence @deploy.md
claudine sequence --fail-fast false @batch.md
```

### When to Use Sequence

Use `claudine sequence` when you have a fixed list of items and need to compose the same template document against each item independently. Each step is a full one-shot composition run — with its own provider selection, harness evaluation, lifecycle notifications, and pre-flight shell approval. The sequence command is serial; steps do not run in parallel.

### Inline Sequence Definition

Sequences can be defined directly in the source document's frontmatter as a scalar list or an object list.

**Scalar list** — each step value is a plain string:

```yaml
sequence:
  - one
  - two
  - three
fail_fast: false
```

**Object list** — each step value is an object; `name` is required:

```yaml
sequence:
  - name: one
    color: red
  - name: two
    color: blue
```

### External YAML Sequence Definition

When the `sequence` frontmatter property is a string, Claudine resolves it as a file reference relative to the source document.

**Plain list form** — the external file contains a `sequence:` key:

```yaml
# steps.yaml
sequence:
  - name: Codex CLI
    site: https://developers.openai.com/codex/cli
  - name: Claude Code
    site: https://claude.ai/code
```

**Template form** — the external file uses `kind/list/template` to apply a shared template across all items:

```yaml
# steps.yaml
kind: sequence
template:
  desc: "{{name}} (_site: {{site}}, repo: {{repo || 'n/a'}}_)"
list:
  - name: Codex CLI
    site: https://developers.openai.com/codex/cli
    repo: https://github.com/openai/codex
  - name: Claude Code
    site: https://claude.ai/code
```

Template rules:

- `kind: sequence` is optional; when present it must equal `sequence`
- `list` must be a non-empty list of objects, each with `name`
- `template` is only supported in the `kind/list/template` external-file form
- Template values must be strings; each template string is rendered against the item's own fields
- Rendered template fields are merged into the item; they may not overwrite reserved step keys

### Template Evaluation

Each step runs the source document through Darkmatter's composition pipeline with a set of reserved variables injected as overrides. These variables are always set by the sequence runner and cannot be overridden by `--set`:

| Variable | Type | Description |
|---|---|---|
| `state` | string or object | The current step value (scalar string or full object) |
| `previous_state` | string, object, or null | The previous step's value, or null for the first step |
| `next_state` | string, object, or null | The next step's value, or null for the last step |
| `is_first` | boolean | `true` when this is the first step |
| `is_last` | boolean | `true` when this is the last step |
| `step` | integer | One-based index of the current step |
| `total_steps` | integer | Total number of steps in the sequence |

For object steps, fields are accessed through `state`: `{{state.name}}`, `{{state.color}}`, etc. Field values are not promoted to top-level variables to avoid collisions with reserved keys or other frontmatter properties such as `agent` or `timeout`.

The `FAIL_FAST` environment variable is also injected per step so that `{{env.FAIL_FAST}}` and `::shell` directives see the same policy as the child provider process.

### Fail-Fast Behavior

By default, a sequence stops on the first failed step. Failure means any of: pre-flight failure, preparation failure, non-zero provider exit, or harness resolution failure.

The effective fail-fast policy is determined by:

1. **`--fail-fast` CLI flag** — overrides the document default for this invocation
2. **`fail_fast` frontmatter property** — document-level default; must be a boolean
3. **Built-in default** — `true` when neither is specified

```yaml
# document default: continue on failure
fail_fast: false
```

```sh
# CLI override: stop on first failure regardless of document default
claudine sequence --fail-fast true @batch.md
```

The `--fail-fast` flag accepts boolish values: `true`, `false`, `1`, `0`, `yes`, `no`.

### The `FAIL_FAST` Environment Variable

Claudine injects `FAIL_FAST=true` or `FAIL_FAST=false` into the composition environment for each step. This makes the effective policy visible to `{{env.FAIL_FAST}}` interpolation inside the template and to any `::shell` directives that inspect the environment.

### Error Handling Semantics

When `fail_fast` is `true` (the default), Claudine stops immediately after the first failed step and exits with code `1`. Steps after the failure are not executed.

When `fail_fast` is `false`, Claudine records each step's result and continues through all steps regardless of failures. After the last step, Claudine exits with `0` if all steps succeeded, or `1` if one or more steps failed.

Harness recovery actions (`retry`, `resume`, `redirect`, `deviate`) apply within a single step only. There is no cross-step recovery mechanism.

> **Note:** The `fail_fast` frontmatter key is reserved for sequence control. It is not passed to Darkmatter's internal compose options.

## Architecture

Both commands follow the same six-stage pipeline:

```
Resolve → Pre-Flight → Prepare → Select Provider → Launch → Closure
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Pre-Flight**: `composition::resolve_shell_approvals()` discovers every shell command in the document graph — template `::shell` directives, top-level frontmatter `$(...)` expressions, and harness `shell_command` validations / `deviate` / `handle` actions — checks whitelists, and prompts the user to approve any unapproved commands before proceeding (see [Pre-Flight Shell Approval](pre-flight-checks.md))
- **Prepare**: `composition::prepare_direct()` or `composition::prepare_inline()` composes through Darkmatter with the pre-approved command set and produces a `PreparedComposition` with `effective_frontmatter`
- **Select**: `composition::select_provider()` applies the precedence chain
- **Launch**: `wrap::composition::execute_composition_request()` runs the provider through the full wrapper pipeline (env, MCP, harness, streaming)
- **Closure**: `composition::closure::rewrite_inline_document()` reconstructs the document for inline mode; direct mode outputs to stdout
