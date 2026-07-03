---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/slash-commands/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local command folders under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> slash commands is already up to date ({{ctx.today}}) — skipping."
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

Research custom slash-command support for **{{state.desc}}**. This topic feeds
Claudine's command linking and portability classification. Write the result to
`{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `homepage`, `docs`, `slash_docs`
- `support`
- `locations`
- `format`
- `command_model`
- `portability`
- `cli_params`
- `env_vars`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when user-defined slash commands or equivalent command
resources are clearly absent.

## Research Questions

- Does the provider support user-defined slash commands or equivalent reusable commands?
- Where are command files stored by OS and scope?
- What filenames, metadata, argument syntax, and body formats are recognized?
- How are commands invoked, namespaced, disabled, and passed arguments?
- How does command output feed the active conversation?
- Which CLI switches or environment variables affect command discovery?
- Which commands are portable across providers, and which need rewriting?

## Body Structure

- `## Overview`
- `## Locations`
- `## File Format`
- `## Invocation Model`
- `## Portability`
- `## Claudine Linking Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
