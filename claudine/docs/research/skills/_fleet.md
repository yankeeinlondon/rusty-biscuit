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
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **skills**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **skills** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Agent Skills** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Agent Skills** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent Skills research on **{{state.name}}** failed to complete!"
    warn: "The Agent Skills research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Agent Skills Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Research **Agent Skills** for **{{state.desc}}**. Prior-generation research in
`../cross-referencing/` and `../skillsets/` are validation assets for humans — do not
open, paraphrase, or cite them; your research must be independent. This topic feeds
Claudine's shared-resource linking module and portability classification, so the goal is to
understand how this provider implements, discovers, scopes, loads, and applies Agent
Skills.

For this topic, Agent Skills are durable user- or repo-authored resources that package
reusable agent behavior for later discovery and use by an agent. Record the exact
implementation: storage locations, file names, metadata, discovery rules, loading
behavior, and runtime behavior. Do not dilute the topic into generic memories, ordinary
project instructions, chat history, transient session context, or provider documentation
pages unless the provider explicitly loads those surfaces as Agent Skills.

Boundary against the slash-commands topic: slash-commands owns invocation grammar and
command-shaped entries — including providers that unify commands into skills, where the
command surface is still slash-commands' ground; this topic owns packaging, activation,
and discovery.

Write the result to `{{file}}`. Include `$schema: ./_schema.yaml` in frontmatter so the
document can be validated, but treat the instructions below as the source of what
high-quality research must contain.

## Research Deliverables

Write prose specific enough that Claudine can implement linking behavior from it without
guessing. Prefer exact paths, file names, metadata keys, precedence rules, and verified
limitations over broad statements.

In the body, cover:

- The provider's Agent Skills implementation.
- Which scopes exist: user, repo/project, workspace, system, extension/plugin, or other.
- The exact storage locations on macOS, Linux, and Windows. Use separate OS records for
  every filesystem path; do not use `os: all`.
- The file format: file names, directory structure, frontmatter or config keys,
  recognized metadata, body format, attachments/assets, and examples.
- Discovery and precedence: how resources are found, ordered, inherited, shadowed,
  enabled, disabled, trusted, or ignored.
- Whether CLI flags, environment variables, config files, safe mode, trust settings, or
  extensions affect skill loading.
- Portability: which artifacts can be linked as-is, which need provider-specific
  rewrites, and which are non-portable because they depend on provider-only tools,
  metadata, file references, or bundled assets.
- Claudine integration notes: what the linker should do, what it should avoid, and
  whether the research implies code or generated-metadata changes.

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
- `skills_docs` - best official URL specifically covering Agent Skills. Omit only when
  no such page exists and explain the documentation gap in the body.
- `support` - classify the provider's Agent Skills implementation:
  - `first_class`: documented, discoverable Agent Skills with clear storage locations,
    file format, metadata, and loading behavior.
  - `partial`: Agent Skills work, but one or more important implementation details are
    limited, unstable, undocumented, or only available in one scope.
  - `convention_only`: Agent Skills are implemented through documented files,
    directories, rules, or conventions rather than a dedicated command or branded
    feature.
  - `unknown`: use only when current sources are unavailable or contradictory after
    serious research; explain the gap in the body's Gaps/Notes prose.
  - `none`: schema compatibility value only. Do not use it for this fleet unless the
    provider has been removed, is not actually an agentic CLI, or cannot load any
    user-authored behavior at all. If research seems to point here, keep digging and
    document why the expected Agent Skills mechanism could not be found.
- `locations` - one record per storage location: `os`, `scope`, `path`, and optional
  `notes`. Use template paths like `~/.claude/skills` or `.claude/skills`, not absolute
  host-specific paths unless reporting observed local evidence in prose.
- `format` - summarize the artifact shape:
  - `file_names`: accepted file names or glob patterns, such as `SKILL.md` or `*.md`.
  - `frontmatter`: whether frontmatter is recognized.
  - `required_fields`: metadata keys required by the provider, not by this research.
  - `optional_fields`: recognized metadata keys.
  - `body_format`: `markdown`, `yaml`, `json`, `toml`, `text`, `other`, or `unknown`.
  - `notes`: include directory layout, attachment rules, examples, or undocumented
    behavior.
- `discovery` - explain `mechanism`, `precedence`, `enable_disable`, and `notes`.
  Include whether repo resources override user resources, whether names shadow, whether
  trusted-workspace gates apply, and whether extension resources participate.
- `portability` - Claudine's linking classification:
  - `portable`: true only when a resource can be linked/copied to another provider with
    no semantic rewrite beyond path placement.
  - `non_portable_assets`: provider-specific attachments, scripts, config references,
    tool names, frontmatter keys, or other assets that cannot be shared directly.
  - `rewrite_needed`: true when content or metadata must be transformed.
  - `notes`: describe the exact rewrite or why no safe rewrite exists.
- `cli_params` - every CLI flag/subcommand that influences skill discovery, trust,
  scope, extension loading, profile selection, or disabling. Use `[]` only after checking
  docs and `--help`, and state the absence in the body.
- `env_vars` - environment variables that influence skill paths, config roots, trust,
  extension loading, or disabling. Use `[]` only when verified absent.
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
    path: "~/.provider/skills"
    notes: "User skills are discovered automatically at startup on macOS."
  - os: linux
    scope: user
    path: "~/.config/provider/skills"
    notes: "Example Linux/XDG location; verify exact provider behavior."
  - os: windows
    scope: user
    path: "%APPDATA%\\Provider\\skills"
    notes: "Example Windows location; verify exact provider behavior."
  - os: macos
    scope: repo
    path: ".provider/skills"
    notes: "Repo skills shadow user skills with the same directory name; add Linux and Windows records explicitly."
format:
  file_names: ["SKILL.md"]
  frontmatter: true
  required_fields: ["name", "description"]
  optional_fields: ["allowed-tools"]
  body_format: markdown
  notes: "A skill is a directory containing SKILL.md plus optional assets."
```

```yaml
discovery:
  mechanism: "Startup scans user and repo skill directories, then exposes matching skills by description."
  precedence: "Repo scope shadows user scope by skill name; extension skills load last."
  enable_disable: "Skills can be disabled by removing the directory; no documented per-skill disable flag."
  notes: "Untrusted workspaces ignore repo skills until the folder is trusted."
portability:
  portable: false
  non_portable_assets: ["allowed-tools metadata", "provider-specific tool names"]
  rewrite_needed: true
  notes: "Markdown body is portable, but frontmatter must be mapped or removed."
```

## Research Questions

- How does the provider implement Agent Skills?
- Where are those resources stored by OS and scope?
- What file names, frontmatter fields, body formats, and metadata are recognized?
- How are skills discovered, enabled, disabled, inherited, shadowed, trusted, or
  overridden?
- Do extensions/plugins contribute skills?
- Which CLI switches, environment variables, or config files affect skill loading?
- Can resources include local files, scripts, media, MCP references, tools, or other
  assets?
- Which artifacts are portable across providers, and which need rewriting?

## Body Structure

- `## Overview`
- `## Locations` — exact template paths per OS and scope, noting which were observed
  locally versus documented only.
- `## File Format` — file names, frontmatter keys, body format, and a small real example
  of a skill artifact.
- `## Discovery and Precedence` — how skills are found, ordered, shadowed,
  enabled/disabled, and trust-gated.
- `## Portability` — which artifacts link as-is, which need rewriting, and why.
- `## Claudine Linking Notes`
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
- Inspect `{{state.user_dir}}` when it exists and the provider stores Agent Skills
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
