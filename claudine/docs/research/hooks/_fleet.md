---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/hooks/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local config examples under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> hooks is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action: 
            - info: "The **Hooks** research on **{{state.name}}** completed successfully: {{ link(file) }}"
            - message: "🎉  the **Hooks** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Hooks research on **{{state.name}}** failed to complete!"
    warn: "The Hooks research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---

## Skills

Use the 'claudine' skill.

## Scope

Research the hook/event system for **{{state.desc}}**. This topic feeds Claudine's
unified lifecycle event model, provider adapters, hook installation, and blocking
behavior. Write the result to `{{file}}` and include `$schema: ./_schema.yaml` in
frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `homepage`, `docs`, `hooks_docs`
- `hooks`
- `config_files`
- `cli_params`
- `payload_fields`
- `response_actions`
- `execution`
- `gaps`
- `changes`
- `requires_claudine_update`
- `reason`

Use `unknown`, `none`, or empty arrays where the provider has no documented support.
Do not omit required fields.

## Frontmatter Field Guide

Use this section as the authoritative meaning of each schema property.

### Identity and Links

- `created`: Date this provider file was first created. Set only on first creation.
  Example: `created: 2026-07-02`
- `last_updated`: Date this research was verified. Always set to `{{ctx.today}}`.
- `agent`: Research runner. Set to `{{env.AGENT}}`.
- `model`: Research model. Set to `{{env.MODEL || 'default'}}`.
- `homepage`, `docs`, `hooks_docs`: Primary URLs for the product, general docs, and
  hook-specific docs. Prefer official docs and source files over third-party summaries.

### Hooks

`hooks` is the native hook/event inventory. Create one record per provider-native hook
or event, not one record per Claudine event. Map to the closest Claudine event when
possible; use `unknown` when the timing is known but the Claudine mapping is unclear,
and `none` when the native hook has no useful Claudine lifecycle equivalent.

`timing` means:

- `pre`: runs before the provider action and may affect whether it happens
- `post`: runs after the provider action
- `around`: wraps an action or can observe both request and result
- `async`: fire-and-forget or background notification
- `unknown`: documented hook exists but timing is unclear

Example:

```yaml
hooks:
  - native_event: PreToolUse
    claudine_event: tool_call
    timing: pre
    blocking: true
    payload_schema: "JSON object with tool_name, tool_input, session_id, cwd"
    return_contract: "Exit 0 allows; exit 2 blocks and returns stderr to the model"
    notes: "Can be scoped by matcher in settings."
  - native_event: Notification
    claudine_event: notification
    timing: async
    blocking: false
    payload_schema: "JSON object with message and transcript_path"
    return_contract: "Ignored"
    notes: "Used for user-facing notifications only."
```

### Config Files

`config_files` records where hooks are configured or installed. Use separate macOS,
Linux, and Windows records for every filesystem path. Do not use `os: all` for paths;
Windows path syntax alone makes that ambiguous.

Example:

```yaml
config_files:
  - os: macos
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User-level hook configuration on macOS."
  - os: linux
    scope: user
    path: "~/.claude/settings.json"
    format: json
    notes: "User-level hook configuration on Linux."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.claude\\settings.json"
    format: json
    notes: "User-level hook configuration on Windows."
  - os: macos
    scope: repo
    path: ".claude/settings.json"
    format: json
    notes: "Repo-level hook configuration; add Linux and Windows records explicitly."
```

### CLI Params

`cli_params` is for commands or switches that install, list, test, disable, or otherwise
affect hooks. Do not list unrelated general CLI flags.

Example:

```yaml
cli_params:
  - flag: "hooks --list"
    description: "Lists configured hooks."
    example: "claude hooks --list"
  - flag: "--disable-hooks"
    description: "Starts a session without executing configured hooks."
    example: "provider run --disable-hooks \"prompt\""
```

### Payload Fields

`payload_fields` describes the event payload shape adapters must parse. Use dot paths
for nested fields and one record per meaningful field. Include fields that are useful
for routing, security decisions, display, or correlation.

Example:

```yaml
payload_fields:
  - native_event: PreToolUse
    field: tool_name
    type: string
    meaning: "Provider-native tool name requested by the model."
  - native_event: PreToolUse
    field: tool_input.command
    type: string
    meaning: "Shell command for bash-like tools."
  - native_event: Stop
    field: transcript_path
    type: string
    meaning: "Path to the session transcript for post-run inspection."
```

### Response Actions

`response_actions` captures what a hook can tell the provider to do. Use provider-native
return values in `native_value`, and describe the actual effect. If the provider uses
process exit codes rather than JSON responses, record the exit code as the native value.

Example:

```yaml
response_actions:
  - action: allow
    native_value: "exit 0"
    effect: "Allows the provider action to continue."
  - action: block
    native_value: "exit 2"
    effect: "Blocks the tool call and sends stderr back to the model."
  - action: modify
    native_value: "{\"decision\":\"approve\",\"updatedInput\":{...}}"
    effect: "Allows the hook to rewrite a tool request before execution."
```

### Execution

`execution` summarizes how hook commands run. This is one object for the provider's
general hook execution model; put event-specific caveats in `hooks[].notes`.

Example:

```yaml
execution:
  shell: "Hook commands run through the user's shell on Unix and PowerShell/cmd on Windows."
  cwd: "Repository root when available; otherwise launch cwd."
  env: "Inherits provider environment plus hook-specific variables."
  timeout: "60 seconds by default; configurable per hook."
  stdin: "Receives JSON payload on stdin."
  stdout: "Usually ignored unless the provider defines a JSON response contract."
  stderr: "Displayed to the user or returned to the model when blocking."
  notes: "Hooks run sequentially for the same event."
```

### Gaps and Change Flags

- `gaps`: Claims that could not be verified, missing provider docs, or behavior Claudine
  cannot model yet.
- `changes`: Update-mode changelog entries. Fresh first-run docs should use `[]`.
- `requires_claudine_update`: Set `true` only when the research implies a Claudine code
  or generated metadata change, not merely because documentation changed.
- `reason`: Required when `requires_claudine_update` is `true`; otherwise use an empty
  string or omit if the schema allows.

## Research Questions

- What native hook or lifecycle events exist, and how do they map to Claudine events?
- Which hooks run before, after, around, or asynchronously relative to the provider action?
- Can a hook block, allow, deny, mutate, replace, or stop execution?
- What payload fields and response contracts are documented or observable?
- Where are hooks configured on macOS, Linux, and Windows, and at which scopes?
- Which CLI switches or commands install, list, test, or disable hooks?
- What shell, cwd, environment, timeout, stdin, stdout, and stderr semantics apply?
- What gaps prevent a faithful Claudine adapter or unified lifecycle mapping?

## Body Structure

- `## Overview`
- `## Native Hooks`
- `## Configuration`
- `## Payloads and Responses`
- `## Execution Semantics`
- `## Claudine Mapping`
- `## Gaps`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.

**IMPORTANT:** DO NOT MAKE THINGS UP. It is far better to admit you don't know something than to make up something just to "complete" the exercise!
