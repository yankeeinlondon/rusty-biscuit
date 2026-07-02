---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/model-config/{{state.file}}"
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
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> is already up to date ({{ctx.today}}) — skipping."
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date before accepting success
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

This topic covers **user-side model configuration** for **{{state.desc}}** — how a user
extends the model set beyond what ships out of the box. The out-of-box model set and
selection mechanisms are covered by the separate `agent-models` topic; do not duplicate
that research here. Focus on:

1. adding **cloud models** the CLI does not (yet) know about
2. adding **local models** served by runners such as Ollama, oMLX, LM Studio,
   llama.cpp, and vLLM

## Document Structure

- `## Introduction to {{state.name}} Model Configuration` Section
    - Which config file(s) accept model configuration, at which scopes (user / repo /
      environment), and in what format
    - Whether a formal schema exists for the config file (e.g. a published JSON Schema)
- `## Adding Cloud Models` Section
    - Walk through a complete, concrete example of adding a model that is not in the
      out-of-box set (config block, where it goes, what each key means)
    - Which API standard(s) the CLI supports for user-added models — OpenAI-compatible,
      Anthropic-compatible, or something bespoke — and how the base URL and auth are
      specified
    - Whether an adapter mechanism is involved (e.g. OpenCode's `npm` ai-sdk package key)
    - What per-model metadata the user can declare (display name, cost, context/output
      limits, modalities, reasoning support, …)
    - How user-added models interact with the built-in catalog: merge, shadow, or
      replace? If the same model id later appears in the CLI's own catalog, whose
      metadata wins?
      > Guidance worth documenting: user config blocks are static while CLI catalogs
      > self-update — best practice is removing a manual block once the catalog covers
      > that model. Note whether {{state.name}} makes this easy or painful.
- `## Adding Local Models` Section
    - For each runner (Ollama, oMLX, LM Studio, llama.cpp, vLLM): is it supported
      natively, via the OpenAI-compatible shim, or not at all? Give a concrete config
      example for at least the two most practical runners
    - How local model ids are written (e.g. `ollama/gemma3:27b` — note size/quantization
      tags)
- `## Environment Overrides` Section
    - ENV variables that redirect model endpoints or selection (base-URL overrides,
      key overrides), and how they interact with config-file settings
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

- Now we will capture other key metadata to the research documents Frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `has_official_schema` - "formal" if the provider publishes a machine-readable
      schema for its config file, "informal" if only documented prose/examples exist,
      otherwise "none"
    - `schema_url` - the URL of the (formal preferred) config schema; omit when none
    - `config_files` - one record per config file that accepts model configuration:
      `scope` (`user`/`repo`/`env`), `path` (template form, e.g.
      `~/.config/opencode/opencode.jsonc`), `format`, `notes`
    - `api_standards` - one record per supported standard for user-added models:
      `standard` (`openai_compatible`/`anthropic_compatible`/`bespoke`),
      `base_url_site` (the config key or env var carrying the base URL), `auth_site`,
      `adapter` (e.g. OpenCode's `npm` key; omit when none), `notes`
    - `metadata_overrides` - the per-model keys a user may declare when adding a model
    - `merge_semantics` - `merge`, `shadow`, `replace`, or `unknown`
    - `local_runners` - one record per runner (`ollama`, `omlx`, `lmstudio`,
      `llamacpp`, `vllm`, `other`): `supported` (`native`/`openai_compatible`/
      `unsupported`), a concrete `example` for supported ones, `notes`
    - `default_model_site` - where the user pins their default model
    - `env_vars` - one record per environment variable that redirects model endpoints
      or selection: `name` + `effect`
    - **Evidence requirement:** you have read access to `{{state.user_dir}}` on this
      host. Inspect the *actual* config files there and prefer what you observe over
      what documentation claims. Real configs regularly contain keys and shapes the
      documentation omits.
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
