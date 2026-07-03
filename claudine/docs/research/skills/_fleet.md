---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/skills/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local skill folders under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> skills is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the research on **{{state.name}}** completed successfully"
---

## Skills

Use the 'claudine' skill.

## Scope

Research first-class or convention-based skill resources for **{{state.desc}}**. This
topic feeds Claudine's linking module and portability classification. Write the result
to `{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `homepage`, `docs`, `skills_docs`
- `support`
- `locations`
- `format`
- `discovery`
- `portability`
- `cli_params`
- `env_vars`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: convention_only` when a provider lacks a named "skills" feature but has a
documented resource convention Claudine could link.

## Research Questions

- Does the provider have first-class skills, reusable instructions, memories, rules, or equivalent resources?
- Where are those resources stored by OS and scope?
- What filenames, frontmatter fields, body formats, and metadata are recognized?
- How are skills discovered, enabled, disabled, inherited, or overridden?
- Which CLI switches or environment variables affect skill loading?
- Which assets are portable across providers, and which need rewriting?

## Body Structure

- `## Overview`
- `## Locations`
- `## File Format`
- `## Discovery and Precedence`
- `## Portability`
- `## Claudine Linking Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
