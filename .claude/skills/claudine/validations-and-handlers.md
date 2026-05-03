# Validations and Handlers

The `harness` module provides a typed job harness for Markdown-backed prompts in Claudine's composition pipeline. It lets a document describe prerequisites, success criteria, timeouts, and recovery strategies in frontmatter.

## Where It Activates

The harness is scoped to:

1. `claudine compose --<provider> <file>` -- chained prompt pipelines
2. `claudine inline-compose --<provider> <file>` -- deterministic document rewrite
3. Wrapper passthrough prompts that resolve to Markdown with harness frontmatter (`pre_checks`, `post_checks`)

## Three Phases

### Before (pre_checks)

Should this run even start? Checks for files, directories, clean repo state, shell-based setup checks. Failures are real failure events that handlers can recover from.

### During

Normal provider execution: prompt preparation, argument shaping, structured/captured output, session ID capture, timeout enforcement. Non-zero exit or timeout becomes a failure event.

### After (post_checks)

Did the run accomplish what the document claimed? File-diff checks, frontmatter comparisons, response-based checks. Success is "the provider exited AND the state matches the contract."

## Path Resolution

All path values in validations use document-centric resolution with three rules:

1. **Absolute** -- returned as-is (`/usr/local/bin/tool`)
2. **`@`-prefixed** -- resolved from repository root (`@docs/brief.md` -> `<repo-root>/docs/brief.md`). Errors if no repo root is detected.
3. **Relative** -- resolved from the source document's parent directory (`../sibling.md` -> `<source-dir>/../sibling.md`)

## Validation Types

Validations support **list form** (ordered, repeatable) and **map form** (compact shorthand). Each validation can include an optional `msg` field for custom failure messages.

### Filesystem / Data Shape

#### `file_exists`

Checks that a file exists at the given path. Usable in both pre and post checks.

```yaml
# Shorthand
pre_checks:
  - file_exists: "@docs/brief.md"

# With custom message
pre_checks:
  - file_exists:
      file: "@docs/brief.md"
      msg: "Brief document must exist before generation"
```

#### `dir_exists`

Checks that a directory exists. Usable in both pre and post checks.

```yaml
pre_checks:
  - dir_exists: "@output/"

# Explicit field form
pre_checks:
  - dir_exists:
      dir: "@output/"
      msg: "Output directory is required"
```

#### `json_file_exists`

Checks that a file exists and contains valid JSON. Optionally validates the top-level shape (`scalar`, `array`, or `object`). Usable in both pre and post checks.

```yaml
pre_checks:
  - json_file_exists: "@config/settings.json"

# With shape constraint
post_checks:
  - json_file_exists:
      file: "@output/results.json"
      shape: array
```

#### `yaml_file_exists`

Checks that a file exists and contains valid YAML. Optionally validates the top-level shape. Usable in both pre and post checks.

```yaml
pre_checks:
  - yaml_file_exists: "@config/pipeline.yml"

# With shape constraint
pre_checks:
  - yaml_file_exists:
      file: "@data/catalog.yml"
      shape: object
```

#### `toml_file_exists`

Checks that a file exists and contains valid TOML. Usable in both pre and post checks.

```yaml
pre_checks:
  - toml_file_exists: "@Cargo.toml"
```

#### `has_write_permission`

Checks that the current process can write to the given path. Usable in both pre and post checks.

```yaml
pre_checks:
  - has_write_permission: "@docs/brief.md"
```

### Repository State

#### `no_dirty_source_code`

Checks for a clean git working tree at the given root (defaults to `.` if omitted). Usable in both pre and post checks.

```yaml
pre_checks:
  - no_dirty_source_code: "@."

# Default root (source document's directory)
pre_checks:
  - no_dirty_source_code: "."
```

#### `has_dirty_source_code`

Checks that local edits are present -- the inverse of `no_dirty_source_code`. Usable in both pre and post checks.

```yaml
post_checks:
  - has_dirty_source_code: "@."
```

### Shell

#### `shell_command`

Runs a command through the centralized approval and policy system. Not an uncontrolled bypass. Usable in both pre and post checks.

```yaml
# Shorthand (stdout/stderr shown by default)
pre_checks:
  - shell_command: "test -f /tmp/lockfile"

# With options
pre_checks:
  - shell_command:
      cmd: "node --version"
      show_stdout: true
      show_stderr: false
      msg: "Node.js must be installed"
```

### Post-Run File / Frontmatter

These validations are **post-only** -- they compare state before and after provider execution.

#### `file_changed`

Asserts that the file differs from its pre-run snapshot.

```yaml
post_checks:
  - file_changed: "@docs/brief.md"
```

#### `file_unchanged`

Asserts that the file is identical to its pre-run snapshot.

```yaml
post_checks:
  - file_unchanged: "@config/settings.json"
```

#### `frontmatter_prop_changed`

Asserts that a specific frontmatter property was modified by the provider run.

```yaml
post_checks:
  - frontmatter_prop_changed: "status"
```

#### `frontmatter_prop_unchanged`

Asserts that a specific frontmatter property was NOT modified.

```yaml
post_checks:
  - frontmatter_prop_unchanged: "author"
```

#### `frontmatter_prop_equals`

Asserts that frontmatter properties match expected values. Takes a mapping of property names to values (the `msg` key is reserved for the custom message).

```yaml
post_checks:
  - frontmatter_prop_equals:
      status: "complete"
      reviewed: true
      msg: "Document must be marked complete and reviewed"
```

### Response

Evaluated against the final non-thinking assistant response. Lengths are character-based. All are **post-only**.

#### `response_length_at_least`

```yaml
post_checks:
  - response_length_at_least: 100
```

#### `response_length_at_most`

```yaml
post_checks:
  - response_length_at_most: 5000
```

#### `response_includes`

```yaml
post_checks:
  - response_includes: "Updated brief"
```

#### `response_missing`

```yaml
post_checks:
  - response_missing: "TODO"
```

### Built-in Inline Events

These are never declared as pre/post checks. They are produced automatically by the `inline-compose` closure path. Handler keys (`handle_inline_response_empty`, `handle_inline_body_unchanged`) can still target them.

- **`inline_response_empty`** -- the provider returned an empty response during inline composition
- **`inline_body_unchanged`** -- the document body was identical after the provider run

## Map Form (Shorthand)

Checks can also be declared as a flat mapping instead of a list. This is more compact but does not support ordering or repeating the same check type.

```yaml
# List form (ordered, repeatable)
pre_checks:
  - file_exists: "@docs/brief.md"
  - file_exists: "@docs/outline.md"
  - has_write_permission: "@docs/brief.md"

# Map form (compact shorthand)
pre_checks:
  file_exists: "@docs/brief.md"
  has_write_permission: "@docs/brief.md"
```

## Timeouts

Declared via a human-friendly string in frontmatter. Accepts `{number}{unit}` with optional whitespace. Treated as a first-class failure event that handlers can recover from.

Supported units: `s`/`sec`/`second`/`seconds`, `m`/`min`/`minute`/`minutes`, `h`/`hr`/`hour`/`hours`.

```yaml
timeout: 30s
timeout: 5 min
timeout: 2h
timeout: 1.5 hours
```

## Failure Events

The harness recognizes four categories of failure:

| Event | Trigger |
|-------|---------|
| `agent_failure` | Non-zero exit from provider |
| `timeout` | Execution exceeded declared timeout |
| `shell_audit_denied` | Shell command denied by approval policy |
| `<validation_event>` | Any validation check failure (e.g. `file_exists`, `response_includes`) |

## Failure Reporting

Passing checks render as a single compact `Status` line. A failing check renders a four-section block on stderr:

1. **Status header** — red glyph plus a phase label (`Pre-validation failed`, `Post-validation failed`, `Agent execution failed`, or `Shell audit failed`).
2. **Source line** — `in <path>` pointing at the markdown file that declared the rule, OSC8-linked when the terminal supports hyperlinks.
3. **YAML snippet** — the rule's frontmatter entry, syntax-highlighted via the same path that renders fenced ` ```yaml ` blocks in markdown.
4. **Reason line** — the underlying diagnostic (e.g. `file does not exist: /path/to/missing.toml`), rendered in muted styling because the glyph already carries severity.

Each `ValidationRule` carries an optional `RuleSource { file, line_range, yaml_snippet }` populated by `parse_harness_plan` and cloned forward onto every `ValidationCheckOutcome`. Programmatically constructed rules without a markdown origin (such as the system-owned inline-compose writability pre-check) fall back to the legacy single-line failure rendering.

## Handlers

Four declarative recovery actions plus a programmatic fallback. All declarative actions support these common fields:

- **`msg`** -- terminal message displayed before recovery
- **`say`** -- text-to-speech announcement (via biscuit-speaks)
- **`set`** -- frontmatter overlay applied in-memory for the next attempt

### retry

Fresh provider session. Falls back to a generic "previous attempt failed" prompt if no `prompt` is supplied.

```yaml
handle_agent_failure:
  retry:
    prompt: "The previous attempt failed. Focus on the core requirements only."
    retries: 3
    msg: "Retrying with simplified prompt..."
    say: "Retrying"
    set:
      strict_mode: true
```

### resume

Continue from a captured `session_id`. Requires a `prompt` (author must be explicit about what the agent should do). Best for timeouts and partial completions.

```yaml
handle_timeout:
  resume:
    prompt: "Continue from where you stopped and finish the brief."
    retries: 2
    msg: "Resuming timed-out session..."
```

### redirect

Switch to a different Markdown document. Can optionally resume the existing session. Best for fallback prompts and document-structured recovery branching.

```yaml
handle_response_includes:
  redirect:
    file: "@prompts/fallback-brief.md"
    resume: false
    msg: "Switching to fallback prompt..."
    set:
      fallback: true
```

### deviate

Run an approved external command before retrying. Declared in frontmatter only -- programmatic handlers cannot return deviate. Screened through shell approval at parse time.

```yaml
handle_file_exists:
  deviate:
    cmd: "mkdir -p output"
    msg: "Creating missing output directory..."
    set:
      directory_created: true
```

### Subject-Specific Handlers

Handlers can be scoped to a specific subject (e.g., a particular file path). Subject-specific handlers take priority over generic handlers for the same event.

```yaml
handle_file_exists:
  # Subject-specific: different recovery per file
  "@docs/brief.md":
    retry:
      prompt: "Create the brief document at docs/brief.md"
  "@docs/outline.md":
    redirect:
      file: "@prompts/create-outline.md"

# Generic handler (no subject key)
handle_file_changed:
  retry:
    prompt: "You must modify the target file."
    retries: 2
```

### Programmatic `handle`

An external script that inspects failure context and returns a recovery decision. Receives JSON on stdin and environment variables. Cannot return `deviate`.

```yaml
# String form
handle: "node scripts/recovery-handler.js"

# Object with command string
handle:
  command: "python3 scripts/handler.py --strict"

# Object with command array
handle:
  command: ["node", "scripts/recovery-handler.js", "--verbose"]
```

**Environment variables** passed to the handler:

| Variable | Description |
|----------|-------------|
| `CLAUDINE_PROVIDER` | Provider name (e.g., "claude") |
| `CLAUDINE_ATTEMPT` | Current attempt number |
| `CLAUDINE_FAILURE_EVENT` | Event name (e.g., "timeout", "file_exists") |
| `CLAUDINE_FAILURE_PHASE` | Phase (e.g., "pre_check", "post_check", "agent") |
| `CLAUDINE_SESSION_ID` | Provider session ID (empty if none) |
| `CLAUDINE_TERMINATION` | How execution ended (e.g., "completed", "timed_out") |
| `CLAUDINE_SOURCE_FILE` | Path to the source document |

**JSON stdin payload:**

```json
{
  "provider": "claude",
  "source_file": "/path/to/prompt.md",
  "attempt": 1,
  "session_id": "abc123",
  "termination": "timed_out",
  "failure_event": "timeout",
  "failure_phase": "agent",
  "message": "Execution exceeded 5m timeout",
  "check": null,
  "response": { "text": "partial output..." }
}
```

**Response protocol:**

| stdout | Meaning |
|--------|---------|
| empty / `null` / `false` | Unhandled -- fall through |
| `true` | Default retry (no prompt suffix, no overlay) |
| JSON with `action: "retry"` | Retry with optional `prompt_suffix`, `set`, `msg`, `say`, `retries` |
| JSON with `action: "resume"` | Resume with required `prompt`, optional `set`, `msg`, `say`, `retries` |
| JSON with `action: "redirect"` | Redirect with required `file`, optional `set`, `msg`, `say`, `resume` |

Example programmatic response:

```json
{
  "action": "retry",
  "prompt_suffix": "Focus only on updating the summary section.",
  "retries": 2,
  "set": { "narrow_scope": true }
}
```

## Handler Resolution Order

1. Subject-specific YAML handler (e.g., `handle_file_exists` with a path key matching the failed check)
2. Generic YAML handler (e.g., `handle_timeout`)
3. Programmatic `handle`
4. Unhandled failure

## `set` Overlays

Handlers can adjust frontmatter-derived state in-memory for the next attempt via `set`. Recovery builds the next attempt plan (source document, prompt text, frontmatter overlay, timeout context, launch mode) rather than merely incrementing an attempt counter.

```yaml
handle_response_includes:
  retry:
    prompt: "Your final response must explicitly say 'Updated brief'."
    set:
      strict_summary: true
      max_tokens: 2000
```

## Inline Composition

`inline-compose` is the most stateful mode: checks body changes, preserves/restores frontmatter layout, updates `last_updated`, and can recover+retry inside the harness loop. The built-in events `inline_response_empty` and `inline_body_unchanged` fire automatically when the provider fails to modify the document.

```yaml
---
timeout: 10m
post_checks:
  - file_changed: "@docs/brief.md"
  - response_includes: "Updated brief"
handle_timeout:
  resume:
    prompt: "Continue from where you stopped and finish the brief."
    retries: 2
handle_inline_body_unchanged:
  retry:
    prompt: "You must modify the document body. Re-read the instructions."
    retries: 1
handle_inline_response_empty:
  retry:
    prompt: "You returned an empty response. Produce the full updated document."
---
```

## Full Example

```yaml
---
pre_checks:
  - file_exists: "@docs/brief.md"
  - has_write_permission: "@docs/brief.md"
  - json_file_exists:
      file: "@config/settings.json"
      shape: object
  - shell_command:
      cmd: "node --version"
      show_stderr: false
      msg: "Node.js is required"
post_checks:
  - file_changed: "@docs/brief.md"
  - response_includes: "Updated brief"
  - response_length_at_least: 200
  - frontmatter_prop_equals:
      status: "complete"
timeout: 10m
handle_timeout:
  resume:
    prompt: "Continue from where you stopped and finish the brief."
    retries: 2
    msg: "Resuming after timeout..."
handle_response_includes:
  retry:
    prompt: "Your final response must explicitly say 'Updated brief'."
    set:
      strict_summary: true
handle_file_exists:
  "@docs/brief.md":
    deviate:
      cmd: "touch docs/brief.md"
      msg: "Creating empty brief..."
handle: "node scripts/fallback-handler.js"
---
```
