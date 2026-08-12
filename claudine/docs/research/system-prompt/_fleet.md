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
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **system prompts**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **system prompts** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **System Prompt** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **System Prompt** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the System Prompt research on **{{state.name}}** failed to complete!"
    warn: "The System Prompt research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# System Prompt Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Claudine wants to provide a consistent and universal way to either **append to** or
**replace** the system prompt for **{{state.desc}}**. Your job is to research in detail
how {{state.name}} handles system prompts, project instructions, agent/subagent prompts,
prompt replacement, prompt append, prompt inspection, and prompt export.

This topic feeds Claudine's `SystemPromptSpec` and wrapper-level
`--append-system-prompt` / `--replace-system-prompt` delivery. The output must be useful
to someone implementing a wrapper, not merely a summary of docs. Boundary: the agent-cli
topic's switch inventory records the *existence* of system-prompt-related flags; this
topic owns their semantics and delivery behavior.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Research Deliverables

All research and observations should be written to the body of the Markdown document
while preserving frontmatter data. The Markdown must be standards-based and isomorphic:
use Markdown tables for tables, Markdown links for links, and Mermaid code blocks for
important data visuals when a diagram clarifies precedence or prompt layering.

The body should answer at least these questions:

- What CLI switches are involved in affecting the system prompt? What does each switch
  do?
- What other ways, other than CLI switches, can manipulate the effective system prompt?
- Can agents or subagents have their own system prompt distinct from an orchestrator?
- What quirks and workarounds do developers discuss for {{state.name}} system prompts?
- Have there been recent changes to how system prompts can be manipulated? If so, when?
- What format works best when appending to the system prompt: pure Markdown, XML-wrapped
  Markdown, YAML, JSON, plain text, or something else?
- What format works best when replacing the system prompt?
- Does the provider offer a supported way to inspect or export the effective built-in
  prompt?
- Which strategy should Claudine use for append and replace without permanently mutating
  user config?

Prefer current official documentation, provider source code, release notes, and observed
CLI behavior. Use local config inspection under `{{state.user_dir}}` when available.
Developer discussions, issues, and workarounds are useful for the `Quirks and
Workarounds` section, but do not let them override official behavior unless you explain
the discrepancy.

Do not invent support from adjacent providers. Use `unknown`, `none`, empty arrays, or a
clear `gaps` entry when current sources do not prove the answer.

## Frontmatter Contract

Read `./_schema.yaml` before writing. It is the machine-validated contract. Populate
frontmatter as follows:

- `$schema` - set to the string `./_schema.yaml`.
- `created` - first-run date, `{{ctx.today}}`. Preserve the existing value on update.
- `last_updated` - set to `{{ctx.today}}`.
- `agent` - set to `{{env.AGENT}}`.
- `model` - set to `{{env.MODEL || 'default'}}`.
- `docs` - best general official documentation URL for {{state.name}} CLI/config
  behavior.
- `system_prompt_docs` - best official URL specifically covering system prompts,
  custom instructions, project instructions, agent specifications, prompt files, or the
  closest prompt-control surface. Omit only when no such page exists and record that in
  `gaps`.
- `append_support` - classify how Claudine can add instructions without replacing the
  provider's base prompt:
  - `native`: a first-class append flag/API exists.
  - `config`: a config key can append or layer instructions.
  - `env`: an environment variable can append or layer instructions.
  - `file`: a discovered instruction file can append or layer instructions.
  - `indirect`: append is possible only through a workaround, such as wrapping a custom
    agent spec around the provider default.
  - `none`: current evidence shows append is not supported.
  - `unknown`: current evidence is insufficient.
- `replace_support` - classify how Claudine can replace the provider's base prompt:
  - `native`: a first-class replace flag/API exists.
  - `config`: a config key can replace the prompt.
  - `env`: an environment variable can replace the prompt.
  - `file`: a file path or discovered file can replace the prompt.
  - `agent_spec`: a provider-native agent/subagent specification can define a distinct
    system prompt.
  - `indirect`: replacement is possible only through a workaround.
  - `none`: current evidence shows replacement is not supported.
  - `unknown`: current evidence is insufficient.
- `cli_params` - every CLI flag, command, or config override switch that affects system
  prompts. Include flags for append, replace, modify/layer, inspect/export, disable, and
  adjacent prompt surfaces such as agent selection. Each record should include:
  - `flag`: exact flag or command, such as `--agent-file <FILE>`.
  - `mode`: `append`, `replace`, `modify`, `inspect`, `disable`, `other`, or `unknown`.
  - `value_shape`: expected value, such as inline text, file path, agent name, JSON, or
    boolean switch.
  - `description`: what the switch does.
  - `example`: a working-looking invocation.
  - `notes`: precedence, mode-specific behavior, limitations, or undocumented behavior.
- `config_sources` - non-CLI files or config surfaces that affect the effective prompt.
  Use one record per OS/scope/path/mode combination. File paths must be recorded
  separately for macOS, Linux, and Windows — never use a single record claiming to
  cover all OSes (Windows paths always differ):
  - `os`: `macos`, `linux`, or `windows`.
  - `scope`: `user`, `repo`, `system`, `agent`, `subagent`, `extension`, `other`.
  - `path`: template path, such as `AGENTS.md`, `.provider/config.json`, or
    `~/.provider/agents/*.yaml`.
  - `mode`: whether the source appends, replaces, modifies/layers, inspects, disables,
    or has unknown behavior.
  - `format`: `markdown`, `yaml`, `json`, `jsonc`, `toml`, `text`, `other`, or
    `unknown`.
  - `notes`: discovery, precedence, trust gates, merge behavior, or examples.
- `env_vars` - environment variables that affect prompt discovery, replacement, export,
  formatting, config roots, agent selection, or disabling. Include `name`, `effect`, and
  `mode` when known.
- `prompt_layers` - ordered effective-prompt layers when the provider documents or
  reveals them. Include:
  - `source`: provider-native source name, such as built-in base prompt, user
    instructions, repo instructions, agent spec, subagent spec, skills, memory, or
    extension.
  - `mode`: append, replace, modify, inspect, disable, other, or unknown.
  - `scope`: affected scopes, such as `builtin`, `user`, `repo`, `agent`, `subagent`,
    `extension`, or provider-native terms.
  - `order_notes`: relative order and conflict behavior.
  - `notes`: whether the layer is visible, mutable, trusted, cached, or model-dependent.
- `agent_prompting` - whether provider-native agents/subagents can carry their own
  system prompt:
  - `supported`: true only when user-defined agents/subagents can define distinct prompt
    text.
  - `definition_surface`: file, config key, CLI flag, extension manifest, or other
    surface.
  - `inheritance`: whether child agents inherit, append to, or replace orchestrator
    prompts.
  - `isolation`: whether child prompts are isolated from parent context and whether
    results return to the parent.
  - `limitations`: missing features, unsupported scopes, undocumented behavior, or
    automation risks.
- `claudine_delivery` - best known wrapper strategy:
  - `append_strategy`: `inline_flag`, `file_flag`, `env_var_file`,
    `config_key_inline`, `config_key_file`, `shadow_home_file`, `agent_spec`,
    `unsupported`, or `unknown`.
  - `replace_strategy`: same enum, for replacement.
  - `temp_file_required`: true when Claudine must write a temporary prompt/config file
    rather than pass inline text.
  - `argv_limit`: note shell/argv length concerns or "not applicable".
  - `notes`: how to avoid mutating user config, whether a shadow HOME/config root is
    needed, and what is risky.
- `format_recommendations` - recommended prompt formats:
  - `append_format`: `markdown`, `xml_wrapped_markdown`, `yaml`, `json`, `text`,
    `other`, or `unknown`.
  - `replace_format`: same enum.
  - `rationale`: explain why that format works best for this provider and whether the
    recommendation differs for append vs replace.
- `recent_changes` - one record per recent provider change affecting prompt
  manipulation. Include `date`, `version` when known, `change`, and `impact`. Use `[]`
  only after checking release notes/changelog sources.
- `quirks` - provider quirks, workarounds, caveats, community-reported traps, and known
  failure modes. Keep each item concrete.
- `gaps` - claims that could not be verified, missing docs, unavailable local config,
  untested flags, or areas requiring follow-up.
- `changes` - on first run, `[]`; on update, concise strings describing changes since
  the previous research. Do not use old research as proof for current facts.
- `requires_claudine_update` - `true` only when the research implies a Claudine wrapper,
  schema, generated-metadata, or documentation change.
- `reason` - required when `requires_claudine_update` is true; otherwise a short
  explanation is still useful.

## Useful Examples

These examples show the expected specificity. Do not copy them unless verified for
{{state.name}}.

```yaml
append_support: file
replace_support: agent_spec
cli_params:
  - flag: "--agent-file <FILE>"
    mode: replace
    value_shape: "YAML file path"
    description: "Loads an agent specification whose system_prompt_path supplies the agent prompt."
    example: "provider --agent-file /tmp/claudine-agent.yaml"
    notes: "Best replacement path; requires a temporary YAML file and prompt file."
config_sources:
  - os: macos
    scope: repo
    path: "AGENTS.md"
    mode: append
    format: markdown
    notes: "Discovered from the working directory and appended as project instructions; repo-relative prompt source on macOS — add Linux and Windows records explicitly."
```

```yaml
prompt_layers:
  - source: "built-in base prompt"
    mode: replace
    scope: ["builtin"]
    order_notes: "Lowest layer unless an agent spec replaces it."
    notes: "Not directly exportable."
  - source: "repo instructions"
    mode: append
    scope: ["repo"]
    order_notes: "Injected after built-in base prompt."
    notes: "Markdown works best; XML wrapping is unnecessary."
agent_prompting:
  supported: true
  definition_surface: "Agent YAML with system_prompt_path"
  inheritance: "Subagents may inherit a base agent and override system_prompt_path."
  isolation: "Child prompt is distinct; final result returns to orchestrator."
  limitations: "Prompt layering for nested agents is not fully documented."
```

```yaml
claudine_delivery:
  append_strategy: config_key_file
  replace_strategy: agent_spec
  temp_file_required: true
  argv_limit: "Avoid inline prompt text; use temporary files for long prompts."
  notes: "Use a shadow config root or temporary agent spec so user config is not mutated."
format_recommendations:
  append_format: markdown
  replace_format: markdown
  rationale: "Provider prompt files are Markdown-centric; XML wrappers add tokens without documented benefit."
```

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

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`.

    > Prior research may be stale. Use it to preserve useful topics and write the
    > changelog, not as proof of current behavior.

::end-block
- Research the current behavior using official documentation first, then provider source
  code, release notes, `--help`, local inspection, and developer discussions where
  useful.
- Inspect `{{state.user_dir}}` when it exists and the provider stores prompt/config
  examples there. State what you observed, including when no local config exists.
::block when="update"
- Update `{{file}}` with current research and add a `## Changelog` entry.
::end-block
::block when="!update"
- Write and save the new research document to `{{file}}`.
::end-block
- Set all frontmatter required by `./_schema.yaml`.
- Cite sources as Markdown links in `## Sources`.
- Provide a summary to stdout: one short paragraph plus bullets is ideal. The saved
  Markdown body must contain only the research, not preparatory thinking or run notes.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done when `{{file}}` has been saved with complete prose research, all
frontmatter fields populated appropriately, `$schema: ./_schema.yaml`, and
`md schema validate '{{file}}'` returns `true`.

- You do not need to run tests or lints.
- This task has no code modifications.
