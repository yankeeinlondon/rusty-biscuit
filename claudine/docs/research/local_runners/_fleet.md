---
sequence: "@claudine/docs/local-runners.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/local_runners/{{state.file}}"
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
# a research sweep wants the other runners to complete even when one fails —
# the freshness gate makes re-runs cheap
fail_fast: false
# make interrupted fleet runs resumable: skip runners already researched today;
# schema-invalidated documents (e.g. after a breaking schema change) always
# re-enter the research pool
initialize:
    stack:
        - when: "!file_exists(file) || !validate_schema(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **Local Runners**"
        - action:
              - stderr: "The provider **{{state.name}}** has research for **Local Runners** that is current; skipping updates"
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date AND that the frontmatter validates
# before accepting success
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "NOT_STAMPED: research file was not updated"
        - when: "!validate_schema(file)"
          action:
              - stderr: "<b>{{file}}</b> fails <code>md schema validate</code> — the agent's exit-criteria claim was wrong."
              - error: "SCHEMA_INVALID: research frontmatter fails schema validation"
        - action:
              - info: "The **Local Runners** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Local Runners** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Local Runners research on **{{state.name}}** failed to complete!"
    warn: "The Local Runners research on **{{state.name}}** failed to complete! (err: {{err.code}}: {{err.msg}})"
---

## Skills

Use the 'claudine' skill.

## Scope

This topic covers **{{state.desc}}** — a **local model runner**: a server that
loads and serves local LLM models over HTTP. Runners are model *servers*, not
agentic CLIs — the research goal is a machine catalog that answers three
questions:

1. **Detection** — how do we tell this runner is installed and/or running on a
   host (per OS)?
2. **API** — what base URL, paths, and auth do OpenAI-style and
   Anthropic-style clients use, and which metadata endpoints exist (health,
   model list, loaded models)?
3. **Integration** — how does a user wire this runner into an agentic CLI
   (concretely: an OpenCode provider block)?

How each *agentic CLI* consumes local runners is the separate `model-config`
topic; do not research the CLIs here. This document is about the runner itself.

## Document Structure

- `## Introduction to {{state.name}}` Section
    - What the runner is, its platform focus, and its open-source status
    - Homepage, docs, API reference, and repo URLs
- `## Platforms and Installation` Section
    - Per OS (macOS / Linux / Windows): supported natively, via WSL, via a
      separate project, or not at all; exact binary name(s) to check for
      (including historical or renamed binaries); install methods (brew,
      installer app, pip, docker, curl script); daemon vs foreground;
      service management (launchd / systemd / tray app / none)
- `## API Surface` Section
    - Default listening port and bind address
    - OpenAI-compatible API: supported? exact base URL (including whether
      `/v1` is part of it), key paths, auth behavior, known deviations from
      the OpenAI spec
    - Anthropic Messages API: supported? since which version? exact base URL
      shape (Anthropic SDKs append `/v1/messages` themselves), auth variance
      (`Authorization: Bearer` vs `x-api-key`), unsupported features
    - Native API family, when one exists (e.g. Ollama's `/api/*`)
    - Metadata endpoints: health check, version, model list, currently-loaded
      models, model info, metrics — exact method + path each, and any flag
      that gates them
- `## Detection` Section
    - The ordered probes a detector should run per OS: binary on PATH,
      process name, port, HTTP probe with an identifying response marker,
      config file or app-bundle presence
    - Note where a port alone is ambiguous (several runners share defaults)
      and what response marker disambiguates
    - A `### Port identity` subsection: the ranked probe strategy for
      answering "which runner is listening on this port?", mirroring the
      `identity_probes` frontmatter records (ranked list, exact marker per
      probe, and an explicit statement of which probes are NOT identifying)
- `## Configuration` Section
    - Config file(s) with per-OS paths and formats; or the env-var / CLI-flag
      mechanism when no file exists
    - The important environment variables and what each redirects
    - Traps: knobs whose names mislead (env vars that don't do what they
      suggest, pointer files that relocate directories)
- `## Models` Section
    - Model id grammar (how ids are written, including size/quantization
      tags), model formats, acquisition paths (registry pull / HuggingFace /
      manual import), and per-OS model store paths
- `## Capabilities` Section
    - Hardware acceleration backends, multi-model and concurrent serving,
      SSE streaming, tool/function calling, embeddings, reranking, web UI
- `## Agentic CLI Integration` Section
    - A complete, plausible OpenCode provider block for this runner
      (`provider.<id>.npm` + `options.baseURL` + `models`)
    - Where the runner's Anthropic endpoint enables direct Claude Code use
      (`ANTHROPIC_BASE_URL` + `ANTHROPIC_AUTH_TOKEN`), show that too
    - Runner-native integration hooks (e.g. `<binary> launch <tool>`
      commands that wire the runner into a coding agent)
- `## Sources`
    - add all useful resources that you used in your research as Markdown links

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** local runners release rapidly and therefore you should assume
    > the prior research is out of date. You are reading this primarily to be
    > able to effectively report the changes into the `## Changelog` section of
    > the document. Critically, you should never substitute information in the
    > old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section; the entry's heading must begin
  with `### {{ctx.today}}`
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
    - `summary` - one sentence describing the runner
    - `homepage`, `docs_url`, `repo_url`, `api_reference_url` - canonical URLs
    - `open_source` - `full`, `partial` (e.g. closed app with open CLI/SDKs),
      or `closed`
    - `has_official_schema` - "formal" if the runner publishes a
      machine-readable schema for its API or config, "informal" if only
      documented prose/examples exist, otherwise "none". An auto-generated
      OpenAPI document (e.g. FastAPI's `/openapi.json`) counts as "formal" —
      use that path as `schema_url`.
    - `schema_url` - the URL of the (formal preferred) schema; omit when none
    - `default_port` - the default API port (number)
    - `default_bind` - the default bind address (note runners that bind all
      interfaces by default)
    - `auth` - `none`, `optional_api_key`, or `required_api_key`
    - `auth_notes` - nuance the enum cannot carry (e.g. enforced by default
      but skippable for localhost via a settings toggle)
    - `platforms` - one record per OS (macOS, Linux, Windows — include
      unsupported OSes as `support: unsupported` so absence is a stated
      fact): `os`, `support` (`native`/`wsl`/`separate_project`/
      `unsupported`), `binary` (exact name on that OS), `alt_binaries`
      (historical or secondary names — renamed binaries, tray executables),
      `install` (methods), `process_model` (`daemon`/`foreground`/`both`),
      `service` (launchd/systemd/tray/none description), `notes`
        - `native` means a released installable artifact (wheel, binary,
          installer) exists for that OS; experimental build-from-source or a
          plugin/fork project (e.g. vLLM-Metal) is `separate_project`, named
          in `notes`
    - `api_standards` - one record per standard (`openai_compatible`,
      `anthropic_compatible`, `native`): `supported` (`yes`/`no`/`partial` —
      record `no` explicitly rather than omitting), `base_url` (the exact
      client-side value), `key_paths`, `auth`, `since_version`, `deviations`
      (mark experimental/beta endpoints here — never silently drop a real
      endpoint because it is experimental), `docs_url`
        - `since_version` must be an exact release tag or the literal string
          `unknown`. Never hedge ("v0.13.3 or later" is not a value), and
          never leave it as an empty string. **Verification protocol:** open
          that specific tag's release-notes page and confirm the feature is
          named there before citing it; if you cannot find a tag whose notes
          name the feature, write `unknown`. A confidently wrong tag is far
          worse than `unknown` — do not infer a version from "current docs",
          a PR number, or memory.
    - `metadata_endpoints` - one record per non-inference endpoint: `purpose`
      (`health`/`version`/`model_list`/`loaded_models`/`model_info`/
      `metrics`/`load_model`/`unload_model`/`admin_ui`/`other`), `method`,
      `path` (exact), `gated_by` (a flag required for the endpoint to exist,
      e.g. `--metrics`), `auth_gated` (true when the endpoint refuses
      unauthenticated requests while server auth is enabled — detectors need
      the ungated set), `response_hint` (identifying marker in the response),
      `notes`. When no dedicated health endpoint exists, record the
      recommended probe endpoint and say so in `notes`. When the server is
      FastAPI/OpenAPI-based, enumerate endpoints from its `/openapi.json`
      rather than docs prose — it catches aliases and gate flags the docs
      omit. Only record endpoints verified against THIS runner's source,
      README, or live server — runners imitate each other's APIs and it is
      easy to import another runner's path by association (e.g. `/api/tags`
      is Ollama-only; do not attribute it to other runners).
    - `detection` - one record per detection probe: `os` (`macos`/`linux`/
      `windows`/`all`), `method` (`binary`/`process`/`port`/`http`/
      `config_file`/`app_bundle`/`service`), `target` (binary name, process
      name, port number, "GET /path", file path), `expect` (the marker
      confirming identity — essential where default ports collide),
      `confidence` (`source_code`/`observed`/`documented`/`inferred`), `notes`
    - `identity_probes` - the ranked, machine-consumable answer to "which
      runner is listening on this port?" — one record per probe, in the order
      a detector should try them (`rank` 1 first): `request` (e.g.
      "GET /api/version"; "ANY /path" for header fingerprinting), `match_in`
      (`body`/`json_field`/`header`/`status`), `field` (the JSON key or
      header name; empty for `body`/`status`), `marker` (the expected value
      or substring), `uniqueness` (`unique` = this runner only, `strong` =
      near-definitive with the port, `weak` = corroborating only),
      `zero_model_ok` (true when the probe works with no models loaded —
      detectors must not require one), `auth_gated` (true when server auth
      refuses the probe — prefer ungated probes), `confidence`, `notes`.
      Rank at least one `unique` probe first when one exists; always include
      the tempting-but-NON-identifying probes (`/health` returning
      `{"status":"ok"}`, a bare `/v1/models`, generic `server: uvicorn`
      headers) as `weak` records with notes explaining why they fail, and
      note where another runner's mimicry creates a reverse-tell (e.g.
      llama.cpp's `/api/tags` with empty digests vs Ollama's real ones).
    - `version_probe` - how to determine the INSTALLED version without
      starting the server, one record per probe per OS where the answer
      differs: `method` (`cli`/`bundle`/`package`/`http`), `command` (the
      exact invocation, e.g. `ollama --version` or
      `defaults read /Applications/X.app/Contents/Info.plist
      CFBundleShortVersionString`), `pattern` (a regex whose first capture
      group extracts the version), `confidence`, `notes`. Record what the
      output ACTUALLY is — run the command on this host when the binary
      exists. Some CLIs do not print a semver: `lms --version` prints a CLI
      commit hash (not the LM Studio app version — a trap worth flagging),
      `llama-server --version` prints `version: <build> (<sha>)` and may
      emit backend-init noise before the version line (match the line, not
      the first line). When CLI and running-server versions can drift
      (long-lived daemons), say so and cross-reference the identity_probes
      record that reports the live version.
    - `config_mechanism` - `config_file`, `env_vars`, `cli_flags`, `gui`, or
      `mixed`
    - `config_files` - one record per config file: `os`, `path` (template
      form), `format`, `role`, `notes`
    - `env_vars` - one record per environment variable that affects the
      server (bind/port, model store, concurrency, auth): `name` + `effect`
    - `model_id_grammar` - how model ids are written for this runner,
      including size/quantization tag conventions. Enumerate ALL accepted id
      forms — registry-URL forms like Ollama's `hf.co/{user}/{repo}[:quant]`
      are part of the grammar, not just an acquisition method.
    - `model_formats` - served/runtime formats only (e.g. gguf, mlx). A
      format the runner converts at import time (e.g. safetensors imported
      and converted to GGUF) belongs in `model_acquisition` notes, not here.
    - `model_acquisition` - one record per path (`registry`/`huggingface`/
      `manual`/`in_app`) with a concrete `example`
    - `model_store_paths` - one record per OS store location (include legacy
      locations still found in the wild, flagged in `notes`)
    - `hardware_acceleration` - backend list (metal, cuda, rocm, vulkan,
      sycl, cpu, ...)
    - `concurrency` - `multi_model` + `parallel_requests` booleans + `notes`
    - `streaming_sse`, `tool_calling` (+ `tool_calling_notes`), `embeddings`,
      `rerank`, `web_ui_url`
    - `integration_hooks` - one record per runner-native command that wires
      the runner into a coding agent (e.g. `omlx launch codex`,
      `ollama launch claude`): `command` + `effect` + `notes`
    - `traps` - misleading knobs and surprising behaviors a consumer must
      know, one string each (e.g. an env var whose name suggests it sets the
      API port but configures something else; a pointer file that relocates
      the home directory). Set `[]` only after actively looking and finding
      none.
    - `opencode_example` - a complete OpenCode provider block for this runner
      as a JSON string. It MUST follow this exact shape (see
      https://opencode.ai/docs/providers/):

      ```json
      {
        "provider": {
          "<runner-id>": {
            "npm": "@ai-sdk/openai-compatible",
            "name": "<display name>",
            "options": { "baseURL": "http://<host>:<port>/v1" },
            "models": { "<model-id>": { "name": "<display name>" } }
          }
        }
      }
      ```

      `npm` is the AI-SDK adapter package (almost always
      `@ai-sdk/openai-compatible` for local runners) — never the runner's
      own JS client library. `models` is a map keyed by model id, never an
      array. `options`/`models` nest inside the provider entry.
    - **Evidence requirement:** some runners are installed on this host and
      you have read access to `{{state.user_dir}}`. Establish what you can by
      observation before trusting documentation: `which {{state.binary}}`,
      `{{state.binary}} --version`, probe the default port (e.g.
      `curl -s http://localhost:{{state.default_port}}/` and the health /
      model-list endpoints), and read the *actual* config files and model
      store under `{{state.user_dir}}`. Prefer `observed` facts over
      documentation claims, and mark `detection` records you verified locally
      with `confidence: observed`. If the runner is not installed here, note
      that and fall back to `documented`. Negative probes are evidence too:
      an endpoint that returns 404 on this host (e.g. a `/metrics` the docs
      imply exists) is an observed fact — record it in `notes`. Two
      generalization cautions: (a) this host's install may be
      legacy-migrated — distinguish the *fresh-install default* paths from
      what you observe here and record both (legacy flagged in `notes`);
      (b) never infer another OS's paths from this host's conventions —
      per-OS paths you cannot verify from the runner's docs or source get
      `confidence: documented`/`inferred` honestly, or are omitted.
    - **Unanswered ≠ omitted:** when research cannot establish a fact, record
      the closest honest value with `confidence: inferred` (detection
      records) or state the gap in `notes` — never silently drop the field.
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** (or its `sniff` detection surface) based on the changes discovered in your research.
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that
    - **Single-pass discipline:** this sequence verifies your frontmatter
      mechanically (`validate_schema`) after you exit, and a failure burns a
      whole recovery round. Before saving, walk `_schema.yaml` top to bottom
      and confirm every property is present with the right shape — including
      `identity_probes`, `version_probe`, `metadata_endpoints`, `detection`,
      `platforms` (all three OSes, unsupported ones stated), and
      `api_standards` (`supported:
      no` recorded explicitly). Quote any scalar that contains `: ` (colon +
      space) — unquoted values like `notes: Server: llama.cpp ...` are YAML
      parse errors, and a parse failure is indistinguishable from missing
      work. Then run `md schema validate '{{file}}'` yourself and fix
      everything it reports before finishing.

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
