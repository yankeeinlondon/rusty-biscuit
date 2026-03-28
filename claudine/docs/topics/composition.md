# Claudine Composition

Claudine supports the ability to _compose_ content by leveraging the Darkmatter library's powerful composition features and routing the result through a wrapper-grade execution pipeline.

Two canonical commands:

- **`claudine compose <file-ref>`** — direct (chained) composition
- **`claudine inline-compose <file-ref>`** — inline composition

Both commands share the same five-stage pipeline and inherit full wrapper-grade behavior: environment setup, harness detection, structured streaming, and handler-driven recovery.

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
   - If the provider modified frontmatter, Claudine reverts to the original
   - `last_updated` is set to today's date
   - The file is written atomically

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

Shell commands in `shell_command` validations and `deviate`/`handle` declarations share Darkmatter's shell policy files (`.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist`). Commands are tokenized and validated at parse time — before the provider is launched — so users are prompted for approval once rather than mid-execution.

## Retired Interfaces

The following interfaces have been removed and replaced by the two canonical commands above:

| Removed | Replacement |
|---------|-------------|
| `claudine <agent> --compose <file>` | `claudine compose --<agent> <file>` |
| `claudine <agent> --frontmatter-prompt <file>` | `claudine compose --<agent> <file>` or `claudine inline-compose --<agent> <file>` |
| `claudine <agent> --prompt-file <file>` | `claudine compose --<agent> <file>` (prompt loading is now part of composition) |
| `claudine compose inline <file>` | `claudine inline-compose <file>` |
| `claudine compose-inline <file>` | `claudine inline-compose <file>` |
| `AGENT` environment variable | `--claude`, `--codex`, etc. flags or `agent` frontmatter |

## Architecture

Both commands follow the same five-stage pipeline:

```
Resolve → Prepare → Select Provider → Launch → Closure
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Prepare**: `composition::prepare_direct()` or `composition::prepare_inline()` composes through Darkmatter and produces a `PreparedComposition` with `effective_frontmatter`
- **Select**: `composition::select_provider()` applies the precedence chain
- **Launch**: `wrap::composition::execute_composition_request()` runs the provider through the full wrapper pipeline (env, MCP, harness, streaming)
- **Closure**: `composition::closure::rewrite_inline_document()` reconstructs the document for inline mode; direct mode outputs to stdout
