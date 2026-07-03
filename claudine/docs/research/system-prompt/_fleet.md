---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/system-prompt/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local prompt/config examples under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> system prompts is already up to date ({{ctx.today}}) — skipping."
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

Claudine wants to provide a consistent universal way to either append to or replace
the system prompt for **{{state.desc}}**. Research how this provider handles system
prompts, project instructions, agent/subagent prompts, prompt replacement, prompt
append, and prompt export/inspection. This topic feeds Claudine's `SystemPromptSpec`
and wrapper-level `--append-system-prompt` / `--replace-system-prompt` delivery.

Write the result to `{{file}}` and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`, `system_prompt_docs`
- `append_support`, `replace_support`
- `cli_params`
- `config_sources`
- `env_vars`
- `prompt_layers`
- `agent_prompting`
- `claudine_delivery`
- `format_recommendations`
- `recent_changes`
- `quirks`
- `gaps`
- `changes`
- `requires_claudine_update`
- `reason`

Use `unknown`, `none`, empty arrays, or a clear `gaps` entry when the current provider
documentation does not prove the answer. Do not invent support from adjacent providers.

## Research Questions

- What CLI switches are involved in affecting the system prompt? What does each switch do?
- What other ways, besides CLI switches, can manipulate the effective system prompt?
- Can agents or subagents have their own system prompt distinct from an orchestrator?
- What quirks and workarounds do developers discuss for this provider's system prompts?
- Have there been recent changes to how system prompts can be manipulated? If so, when?
- What format works best when appending to the system prompt?
- What format works best when replacing the system prompt?
- Does the provider offer a way to inspect or export the effective built-in prompt?
- Which strategy should Claudine use for append and replace without permanently mutating user config?

## Body Structure

- `## Overview`
- `## CLI Parameters`
- `## Configuration and Discovery`
- `## Prompt Layers and Precedence`
- `## Agents and Subagents`
- `## Format Recommendations`
- `## Recent Changes`
- `## Quirks and Workarounds`
- `## Claudine Delivery Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.

Do not add thinking or preparatory statements to the document body. Those can go to
stdout during the run, but the saved Markdown body must contain only the research.
