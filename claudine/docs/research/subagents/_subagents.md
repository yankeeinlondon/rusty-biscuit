---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/subagents/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local agent folders under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> subagents is already up to date ({{ctx.today}}) — skipping."
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

Research agent/subagent definition support for **{{state.desc}}**. This topic feeds
Claudine's agent linking module and future lifecycle resume/proxy behavior. Write the
result to `{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `homepage`, `docs`, `subagent_docs`
- `support`
- `locations`
- `format`
- `runtime`
- `observability`
- `portability`
- `cli_params`
- `env_vars`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: convention_only` when the provider has reusable agent definitions under a
different name but no explicit "subagent" feature.

## Research Questions

- Does the provider support user-defined agents, subagents, modes, personas, workers, or equivalent resources?
- Where are definitions stored by OS and scope?
- What filenames, metadata, and body formats are recognized?
- How are subagents invoked, selected, or delegated to at runtime?
- What context, permissions, model, tool, and turn-limit inheritance applies?
- Are subagent starts/stops visible in streams, hooks, logs, or session IDs?
- Which definitions are portable across providers, and which need rewriting?

## Body Structure

- `## Overview`
- `## Locations`
- `## Definition Format`
- `## Runtime Behavior`
- `## Observability`
- `## Portability`
- `## Claudine Linking Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
