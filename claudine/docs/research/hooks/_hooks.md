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
