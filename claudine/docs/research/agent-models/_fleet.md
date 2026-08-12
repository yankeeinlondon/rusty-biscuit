---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-models/{{state.file}}"
# NOTE: `grant:` is not implemented yet — until it is, run this sequence with
# `--yolo` so the provider can Read files under {{state.user_dir}}; without it
# OpenCode's external_directory permission is auto-rejected in non-interactive
# mode and the research agent stops prematurely.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
# the frontmatter contract for target documents lives in the schema sidecar
# (./_schema.yaml) so the contract is single-sourced and machine-validated
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
# make interrupted fleet runs resumable: skip providers already researched today
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **Agent Models**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **Agent Models** that is current; skipping updates"
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date before accepting success
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Agent Models** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Agent Models** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent Models research on **{{state.name}}** failed to complete!"
    warn: "The Agent Models research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Agent Model Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

This topic covers the **out-of-box model surface** of **{{state.desc}}**: which models a
stock install offers, which mechanisms select a model at launch time and at runtime, the
precedence between those mechanisms, and whether the catalog can be enumerated
programmatically. The research feeds Claudine's **model ground-truth mapping** — the
expected-offering records Claudine uses to know which model IDs a stock {{state.name}}
install should accept and offer, and to detect drift when the offering changes.

The sibling `model-config` topic owns **user-side extension** of the model set —
registering bespoke cloud models, local-runner models, and gateway/proxy routing. Do not
research or document those mechanisms in depth here; name them only where the body asks
for a classification-level pointer. Reciprocally, `model-config` cedes out-of-box
enumeration, selection mechanisms, and precedence to this topic.

Sibling provider research files in this directory (e.g. `claude.md`, `codex.md`,
`gemini.md`, `opencode.md`) are research **outputs**, not sources — do not open,
paraphrase, or cite another provider's document; your research must be independent.

## Document Structure

- `## Models Available` Section
    - Which models does a stock install of {{state.name}} offer? List every entry with
      its **exact model ID transcribed verbatim from observation** — the precise string
      the CLI/config accepts. Never normalize, abbreviate, or "clean up" an ID; a
      ground-truth mapping built from paraphrased IDs is worse than none
    - Distinguish aliases from the underlying model IDs they resolve to, and say which
      model is used when the user specifies nothing
    - Record documented context windows where the provider states them, and note when
      the default resolution varies by account type or backend
    - > **Anti-pattern — do not do this:** never report a model absent from the
      > interactive picker as unavailable without checking the selection mechanisms and
      > dynamic listing. Pickers are frequently a curated subset; a model can be fully
      > selectable by exact ID via a CLI flag, env var, or config key while never
      > appearing in the menu.
- `## Model Selection` Section
    - Which mechanisms select a model at launch time and at runtime — CLI flags,
      environment variables, config-file keys, interactive slash commands,
      wire-envelope fields? Give the exact flag/variable/key/command name and a
      concrete example for each mechanism that exists
    - What is the highest-wins precedence ordering across those mechanisms? State the
      evidence for the ordering (documented ordering or observed behavior) instead of
      assuming the conventional `cli > env > config` order holds
    - How do session-scoped overrides interact with persisted defaults — does an
      interactive switch persist to config, is a launch flag session-only, what happens
      on session resume?
- `## Configuration Schema` Section
    - Which schema artifacts exist for the model-related configuration surface — a
      formal machine-readable schema (JSON Schema or similar), an informal documented
      shape (prose, tables, examples), or neither? Cite the URL of whatever exists
- `## Dynamic Listing` Section
    - Which mechanisms enumerate the model catalog programmatically — a
      `models`/`list` subcommand, an HTTP API, a config dump, a machine-readable cache
      file? Name the method and show an example invocation with the shape of its output
    - When no mechanism exists, say exactly what was checked (`--help`, subcommand
      help, docs) — a negative probe is a finding, not an omission
- `## Extending the Model Set` Section — pointer only
    - Name each channel through which users register bespoke/local models, one line
      per channel with the config file/key involved — classification only. The
      mechanics (config walkthroughs, gateway setup, local-runner integration) are the
      `model-config` topic's territory; do not duplicate them here
- `## Sources`
    - add all useful resources that you used in your research as Markdown links

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLI's change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information
    in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
    - take your time and make sure to be complete in your research
    - **Evidence requirement:** you have read access to
      `{{state.user_dir || 'the provider user config directory'}}` on this host.
      Inspect the actual config files, caches, and logs there and prefer what you
      observe over what documentation claims — real installs regularly contain model
      IDs and selection keys the documentation omits. State when no local config
      exists to inspect
    - unanswered ≠ omitted: when a question cannot be settled, record `unknown` with a
      note rather than silently dropping it
::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative contract, expressed as a `SimpleSchema`, and `md schema validate`
    > will enforce it against everything you write.

- Now capture the facts you documented above into the document's Frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `has_official_schema` - from `## Configuration Schema`: "formal" when a
      machine-readable schema exists, "informal" when only documented prose/examples
      exist, otherwise "none"
    - `schema_url` - the URL cited in `## Configuration Schema` (prefer formal over
      informal); omit when none exists
    - `default_models` - one record per model documented in `## Models Available`;
      `id` must be the exact observed string, verbatim — never normalized or
      abbreviated. Add `alias` when a short form exists, `context_window` when
      documented, and `is_default: true` on the model used when none is specified
    - `model_selection` - one record per mechanism documented in `## Model Selection`,
      with the `site` and `example` shown there. **One canonical site per record —
      this applies to EVERY method, not just env vars.** `site` is a single token:
      for `cli_flag`, the long-form flag ONLY (`--model`) — never a compound like
      `"--model / -m"`; for `env_var`, a single name (`UPPER_SNAKE_CASE`) — never
      `"VAR_A / VAR_B"`; likewise a single config key or slash command. Put every
      short or alternate form (`-m`, or a second env var in the same family) in the
      record's `aliases` list, and describe the relationship in `notes`. Compound
      sites cannot feed the generated catalog — `claudine-gen` skips them, and a
      semantically-wrong alternate (e.g. a reasoning-effort flag colliding with the
      model flag) can silently win the field
    - `precedence` - the highest-wins ordering you established in `## Model Selection`
      (e.g. "cli_flag > env_var > config_file")
    - `dynamic_listing` - the facts from `## Dynamic Listing`: `available`, plus
      `method` and `example` when a mechanism exists. When the mechanism is a
      shell/subcommand, ALSO capture it structurally so it needs no hand-authored
      override: `list_program` = the executable invoked (usually the CLI binary)
      and `list_args` = its argument tokens as a list — e.g. program `kilo`, args
      `["models"]` for `kilo models`. When the mechanism is an HTTP endpoint, set
      `rest_endpoint` to that URL. Leave all three unset when no programmatic
      listing exists (`available: false`)
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on the changes discovered in your research. 
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document 
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
