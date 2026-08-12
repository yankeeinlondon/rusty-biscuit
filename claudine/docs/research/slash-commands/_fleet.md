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
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **slash commands**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **slash commands** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Slash Commands** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Slash Commands** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Slash Commands research on **{{state.name}}** failed to complete!"
    warn: "The Slash Commands research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Slash Commands Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research user-defined **slash commands and equivalent reusable command resources** for
**{{state.desc}}**. Prior-generation research in `../cross-referencing/` is a
validation asset for humans — do not open, paraphrase, or cite it; your research must
be independent. This topic feeds Claudine's command linking and portability
classification, so the research must describe both the authoring format and runtime
invocation behavior.

For this topic, slash-command support means any durable command a user or repo can define
and invoke in an agent session: slash commands, prompt commands, custom commands, command
macros, workflow commands, extension commands, or documented conventions that behave like
named reusable commands. Do not count built-in commands alone as support unless the
provider also lets users define their own.

Boundary against the skills topic: this topic owns invocation grammar and command-shaped
entries — including providers that unify commands into skills, where the command surface
is still this topic's ground; the skills topic owns packaging, activation, and discovery.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Research Deliverables

Write prose specific enough that Claudine can link or transform command definitions
without guessing. Prefer exact syntax, file names, precedence rules, and invocation
examples over general descriptions.

In the body, cover:

- What the provider calls user-defined commands, or the closest equivalent.
- Whether built-in commands and user-defined commands share one namespace.
- Storage locations by OS and scope: user, repo/project, workspace, system,
  extension/plugin, or other.
- Command file format: file names, frontmatter/config fields, body format, argument
  placeholders, shell execution rules, prompt insertion rules, and examples.
- Invocation behavior: command name, namespace prefix, argument parsing, quoting,
  multi-word arguments, default arguments, command discovery, autocomplete, and disable
  mechanisms.
- Output handling: whether the command body is inserted into the conversation, executed
  as shell, sent as a prompt, expands files, mutates context, or streams output.
- Precedence and trust: how user/repo/extension commands interact, whether repo commands
  require trust, and how name conflicts are resolved.
- CLI flags, environment variables, config files, safe mode, profiles, or extensions
  that affect command loading.
- Portability: which command files can be linked as-is, which need metadata or argument
  rewrites, and which are provider-specific enough to avoid linking.
- Claudine integration notes: how the command linker should classify the provider and
  whether code or generated-metadata changes are needed.

## Frontmatter Contract

Read `./_schema.yaml` before writing. It is the machine-validated contract. Populate
frontmatter as follows:

- `$schema` - set to the string `./_schema.yaml`.
- `created` - first-run date, `{{ctx.today}}`. Preserve the existing value on update.
- `last_updated` - set to `{{ctx.today}}`.
- `agent` - set to `{{env.AGENT}}`.
- `model` - set to `{{env.MODEL || 'default'}}`.
- `homepage` - provider homepage URL, when useful for identification.
- `docs` - best general official documentation URL for this provider's CLI/config.
- `slash_docs` - best official URL specifically covering user-defined slash/custom
  commands. Omit only when no such page exists and explain that gap in the body.
- `support` - one of:
  - `first_class`: the provider has named, documented user-defined commands.
  - `partial`: user-defined commands exist but with major limits such as one scope,
    no arguments, unstable format, or no automatic discovery.
  - `convention_only`: there is no formal command feature, but documented reusable
    prompt/config files can be invoked like commands.
  - `none`: user-defined commands or equivalents are clearly absent.
  - `unknown`: current sources do not prove the answer.
- `locations` - one record per command storage location: `os`, `scope`, `path`, and
  optional `notes`. Use template paths like `~/.provider/commands` or
  `.provider/commands`.
- `format` - summarize the command artifact:
  - `file_names`: accepted names or glob patterns such as `*.md` or `commands/*.toml`.
  - `frontmatter`: whether frontmatter is recognized.
  - `required_fields`: metadata keys required by the provider.
  - `optional_fields`: recognized metadata keys.
  - `argument_syntax`: exact placeholder or argument grammar, such as `$ARGUMENTS`,
    positional `$1`, a double-braced args placeholder token (literal form shown fenced
    below), YAML fields, or "none".

    ```text
    {{args}}
    ```

  - `body_format`: `markdown`, `yaml`, `json`, `toml`, `text`, `other`, or `unknown`.
  - `notes`: include examples, directory-to-namespace mapping, shell behavior, and
    undocumented constraints.
- `command_model` - describe runtime behavior:
  - `invocation`: how a user invokes the command, including prefixes and examples.
  - `namespacing`: how directories, scopes, extensions, or command names map to the
    visible command namespace, and how conflicts resolve.
  - `arguments`: how arguments are parsed, quoted, substituted, validated, and passed.
  - `output_handling`: how command content/output enters the active session.
  - `disabled_mechanism`: supported disable/hide mechanisms, or "none documented".
  - `notes`: trust gates, autocomplete, built-in conflicts, or mode-specific behavior.
- `portability` - Claudine's linking classification:
  - `portable`: true only when a command can be linked/copied to another provider with
    no semantic rewrite beyond path placement.
  - `non_portable_assets`: provider-specific placeholders, shell features, tools,
    frontmatter, file references, scripts, or extension hooks.
  - `rewrite_needed`: true when content or metadata must be transformed.
  - `notes`: describe the exact rewrite or why no safe rewrite exists.
- `cli_params` - every CLI flag/subcommand that affects command discovery, profiles,
  extensions, trust, safe mode, or disabling. Use `[]` only after checking docs and
  `--help`.
- `env_vars` - environment variables that influence command paths, config roots,
  profiles, trust, extensions, or disabling. Use `[]` only when verified absent.
- `changes` - on first run, `[]`; on update, concise strings describing changes since
  the previous research. Do not use old research as proof for current facts.
- `requires_claudine_update` - `true` only when Claudine code, schemas, generated
  metadata, or linking rules should change because of the research.
- `reason` - required when `requires_claudine_update` is true; otherwise a short
  explanation is still useful.

## Useful Examples

These examples show the expected specificity. Do not copy them unless verified for
{{state.name}}.

```yaml
support: first_class
locations:
  - os: macos
    scope: user
    path: "~/.provider/commands"
    notes: "Commands are loaded at startup on macOS."
  - os: linux
    scope: user
    path: "~/.config/provider/commands"
    notes: "Example Linux/XDG location; verify exact provider behavior."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\commands"
    notes: "Example Windows location; verify exact provider behavior."
  - os: macos
    scope: repo
    path: ".provider/commands"
    notes: "Repo commands require a trusted workspace; add Linux and Windows records explicitly."
format:
  file_names: ["*.md"]
  frontmatter: true
  required_fields: []
  optional_fields: ["description", "argument-hint"]
  argument_syntax: "$ARGUMENTS is replaced with the raw argument string."
  body_format: markdown
  notes: "Nested directories become slash-command namespaces."
```

```yaml
command_model:
  invocation: "Type /project:review src/lib.rs in an interactive session."
  namespacing: "User commands use /user:name; repo commands use /project:name; built-ins win on exact conflict."
  arguments: "Arguments are passed as one raw string; quoting is not parsed by the provider."
  output_handling: "The expanded Markdown is appended to the conversation as a user prompt."
  disabled_mechanism: "No per-command disable flag; remove or rename the file."
  notes: "Autocomplete lists commands after startup only."
portability:
  portable: false
  non_portable_assets: ["$ARGUMENTS placeholder", "provider namespace prefixes"]
  rewrite_needed: true
  notes: "Body is mostly portable, but placeholders and metadata need mapping."
```

## Research Questions

- Does the provider support user-defined slash commands or equivalent reusable commands?
- Where are command files stored by OS and scope?
- What file names, metadata, argument syntax, and body formats are recognized?
- How are commands invoked, namespaced, disabled, trusted, and passed arguments?
- How does command content or output feed the active conversation?
- Are command definitions allowed to execute shell, call tools, read files, or include
  other prompt files?
- Which CLI switches, environment variables, config files, or extensions affect command
  discovery?
- Which commands are portable across providers, and which need rewriting?

## Body Structure

- `## Overview` — what the provider calls the feature and how complete the support is.
- `## Locations` — exact template paths per OS and scope, noting which were observed
  locally versus documented only.
- `## File Format` — file names, metadata fields, argument grammar, body format, and a
  small real example of a command file.
- `## Invocation Model` — how commands are invoked, namespaced, argument-parsed, and how
  their content enters the conversation.
- `## Portability` — which command files link as-is, which need rewriting, and why.
- `## Claudine Linking Notes` — what the command linker should do and avoid for this
  provider.
- `## Changelog` when `update` is true
- `## Sources`

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`.

    > Prior research may be stale. Use it to preserve useful topics and write the
    > changelog, not as proof of current behavior.

::end-block
- Research the current behavior using official documentation first, then source code,
  release notes, `--help`, and local inspection where useful.
- Inspect `{{state.user_dir}}` when it exists and the provider stores command resources
  there. State what you observed, including when no local config/resources exist.
::block when="update"
- Update `{{file}}` with current research and add a `## Changelog` entry.
::end-block
::block when="!update"
- Write and save the new research document to `{{file}}`.
::end-block
- Set all frontmatter required by `./_schema.yaml`.
- Cite sources as Markdown links in `## Sources`.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done when `{{file}}` has been saved with complete prose research, all
frontmatter fields populated appropriately, `$schema: ./_schema.yaml`, and
`md schema validate '{{file}}'` returns `true`.

- You do not need to run tests or lints.
- This task has no code modifications.
