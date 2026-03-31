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

## Validation Types

Validations support list form (ordered, repeatable) and map form (compact shorthand). Path resolution is document-centric: absolute stays absolute, `@foo/bar.md` resolves from repo root, other relative paths resolve from the source document's directory.

### Filesystem / Data Shape

`file_exists`, `dir_exists`, `json_file_exists`, `yaml_file_exists`, `toml_file_exists`, `has_write_permission` -- typed file checks distinguish "exists" from "exists and is structurally valid."

### Repository State

`no_dirty_source_code`, `has_dirty_source_code` -- clean baseline or local edits expected.

### Shell

`shell_command` -- goes through centralized approval and policy system, not an uncontrolled bypass.

### Post-Run File / Frontmatter

`file_changed`, `file_unchanged`, `frontmatter_prop_changed`, `frontmatter_prop_unchanged`, `frontmatter_prop_equals` -- especially useful for document-maintenance workflows.

### Response

`response_length_at_least`, `response_length_at_most`, `response_includes`, `response_missing` -- evaluated against final non-thinking assistant response, character-based lengths.

## Timeouts

Declared via `timeout: 5m` in frontmatter. Treated as a first-class failure event: can be reported, matched by `handle_timeout`, and recovered with retry/resume/redirect.

## Handlers

Four declarative recovery actions plus a programmatic fallback:

### retry

Fresh provider session. Can append prompt text, apply `set` overlay to frontmatter, emit terminal messaging, cap retries. Falls back to generic "previous attempt failed" prompt if no addition supplied.

### resume

Continue from captured `session_id`. Requires a prompt (author must be explicit about what the agent should do). Best for timeouts and partial completions.

### redirect

Switch to a different Markdown document. Can optionally resume the existing session. Best for fallback prompts and document-structured recovery branching.

### deviate

Run an approved external command before retrying. Declared in frontmatter, screened through shell approval. Useful for generating prerequisites, running formatters, narrow repairs. Declarative-only -- programmatic handlers cannot return deviate.

### Programmatic handle

Script inspects failure context, returns: no action, default retry, or a typed recovery action (retry/resume/redirect, not deviate).

## Handler Resolution Order

1. Subject-specific YAML handler (e.g. `handle_response_includes`)
2. Generic YAML handler (e.g. `handle_timeout`)
3. Programmatic `handle`
4. Unhandled failure

## set Overlays

Handlers can adjust frontmatter-derived state in-memory for the next attempt via `set`. Recovery builds the next attempt plan (source document, prompt text, frontmatter overlay, timeout context, launch mode) rather than merely incrementing an attempt counter.

## Inline Composition

`inline-compose` is the most stateful mode: checks body changes, preserves/restores frontmatter layout, updates `last_updated`, and can recover+retry inside the harness loop.

## Example

```yaml
---
pre_checks:
  - file_exists: "@docs/brief.md"
  - has_write_permission: "@docs/brief.md"
post_checks:
  - file_changed: "@docs/brief.md"
  - response_includes: "Updated brief"
timeout: 10m
handle_timeout:
  resume:
    prompt: "Continue from where you stopped and finish the brief."
    retries: 2
handle_response_includes:
  retry:
    prompt: "Your final response must explicitly say 'Updated brief'."
    set:
      strict_summary: true
---
```
