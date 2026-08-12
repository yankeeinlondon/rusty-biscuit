---
name: model-config
description: Configuring models in agentic CLIs — adding local or cloud models to Claude Code, Codex, Gemini CLI, Goose, Kimi Code, OpenCode, Qwen Code, Pi, or Kilo Code. Use when wiring a local runner (Ollama, LM Studio, oMLX, llama.cpp, vLLM) into an agentic CLI, redirecting a provider's base URL (ANTHROPIC_BASE_URL, OPENAI_BASE_URL, model_providers, provider.<id>.options.baseURL), choosing between OpenAI-compatible and Anthropic-compatible endpoints, bridging a CLI to a different cloud vendor's models, or working with per-provider model config file shapes and merge semantics.
last_updated: 2026-07-02
hash: b612c0ee680a45c0-b57dc298d43fb3c5
---

# Model Configuration in Agentic CLIs

How a user extends an agentic CLI's model set beyond what ships out of the box — cloud
and local. The governing principle: **support is a property of API-standard bridging,
not of a provider "knowing about" a runner or vendor.** Every runner and most gateways
expose an OpenAI-compatible endpoint (many also an Anthropic-compatible one), so any CLI
that allows a base-URL override can use them. The two questions that matter are always:

1. Which API standards can this CLI's model client speak?
2. How is its base URL redirected?

Runner-side facts (ports, endpoint paths, which standards each runner serves) live in
the **local-llm-runners** skill — load it alongside this one for local-model work.

## Cross-provider comparison

| Provider | User config | Format | Client speaks | Base-URL site | Merge |
| --- | --- | --- | --- | --- | --- |
| Claude Code | `~/.claude/settings.json` | JSON | Anthropic only | `ANTHROPIC_BASE_URL` | merge |
| Codex CLI | `~/.codex/config.toml` | TOML | OpenAI Responses only | `[model_providers.<id>].base_url` / `openai_base_url` | replace |
| Gemini CLI | `~/.gemini/settings.json` | JSON | bespoke Gemini only | `GOOGLE_GEMINI_BASE_URL` env (gateway auth mode) | merge |
| Goose | `~/.config/goose/config.yaml` + `custom_providers/*.json` | YAML/JSON | OpenAI, Anthropic, Ollama-native | custom provider `base_url` (or `OPENAI_HOST`/`ANTHROPIC_HOST`/`OLLAMA_HOST`) | merge |
| Kimi Code | `~/.kimi/config.toml` | TOML | OpenAI (legacy+responses), Anthropic, bespoke | `providers.<name>.base_url` (`OPENAI_BASE_URL` for OpenAI types) | shadow |
| OpenCode | `~/.config/opencode/opencode.json(c)` | JSONC | OpenAI (via ai-sdk adapters) | `provider.<id>.options.baseURL` + `npm` adapter | merge |
| Qwen Code | `~/.qwen/settings.json` | JSON | OpenAI, Anthropic, Gemini, Vertex | per-model `baseUrl` in `modelProviders.<type>` (or `OPENAI_BASE_URL`/`ANTHROPIC_BASE_URL`) | merge |
| Pi | `~/.pi/agent/models.json` | JSON | OpenAI, Anthropic, bespoke (`api:` per provider) | `baseUrl` (provider- or model-level) | shadow |
| Kilo Code | `~/.config/kilo/kilo.jsonc` | JSONC | OpenAI, Anthropic (via AI SDK adapters) | `provider.<id>.options.baseURL` | merge |

*Merge* = user entries merge alongside the built-in catalog; *shadow* = a same-key user
entry overrides the built-in/managed entry; *replace* (Codex `model_catalog_json`) = the
user catalog replaces the built-in list entirely.

## Local-runner integration matrix

`FC` = first_class (provider or runner ships an explicit hook) · `BUO` = base-URL
override onto a standard the runner speaks · `proxy` = translation proxy required.
Parenthesized standard is what the path rides on.

| Provider | Ollama | oMLX | LM Studio | llama.cpp | vLLM |
| --- | --- | --- | --- | --- | --- |
| Claude Code | FC `ollama launch claude` (anthropic) | FC `omlx launch claude` (anthropic) | BUO (anthropic) | BUO (anthropic) | BUO (anthropic) |
| Codex CLI | FC `--oss` / built-in provider (openai) | BUO (openai) | FC `--oss --local-provider lmstudio` (openai) | BUO (openai) | BUO (openai) |
| Gemini CLI | proxy (bespoke client) | proxy | proxy | proxy | proxy |
| Goose | FC built-in provider (ollama-native) | BUO (openai) | FC built-in provider (openai) | BUO (openai) | BUO (openai) |
| Kimi Code | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) |
| OpenCode | FC `ollama launch opencode` (openai) | FC `omlx launch opencode` (openai) | BUO (openai) | BUO (openai) | BUO (openai) |
| Qwen Code | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) |
| Pi | BUO (openai) | FC `omlx launch pi` (openai) | BUO (openai) | BUO (openai) | BUO (openai) |
| Kilo Code | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) | BUO (openai) |

Key consequences:

- **Claude Code needs no gateway for local models** — all five runners serve the
  Anthropic Messages API natively, so `ANTHROPIC_BASE_URL=http://localhost:<port>`
  (base URL *excludes* `/v1`) plus a dummy `ANTHROPIC_AUTH_TOKEN` is enough.
- **Codex is Responses-API-only** (`wire_api = "chat"` was removed) — this works for
  all five runners because they all serve `/v1/responses`, but a *cloud* vendor that
  serves only Chat Completions (e.g. Mistral) needs a Responses-translating proxy.
- **Gemini CLI is the lone all-proxy provider** — its client speaks only the bespoke
  Gemini API, which no runner serves; a Gemini-compatible translating proxy is
  required for every local runner and every non-Google cloud vendor.

## Cross-cloud bridging

All nine providers can be routed at a different cloud vendor's models. Open providers
(OpenCode, Kilo, Pi, Qwen, Kimi, Goose) do it natively — add a provider block with the
vendor's (or an aggregator's) OpenAI-compatible URL. Vendor platforms bridge through
their single standard: Claude Code via an Anthropic-compatible gateway on
`ANTHROPIC_BASE_URL`; Codex via a Responses-compatible endpoint or proxy in
`[model_providers.<id>]`; Gemini CLI via a Gemini-compatible gateway on
`GOOGLE_GEMINI_BASE_URL`. When the target vendor's native API doesn't match the standard
the client speaks, put a translating proxy (e.g. LiteLLM) in between — never point the
base URL directly at an incompatible API.

## Whose metadata wins

User config blocks are static while CLI catalogs self-update. In *merge* providers a
duplicate id generally defers to or coexists with the catalog entry; in *shadow*
providers (Kimi, Pi) the user block wins silently. Best practice everywhere: remove a
manual model block once the CLI's own catalog covers that model — no provider automates
this cleanup.

## Per-provider depth

[providers.md](providers.md) carries per-provider reference sections: config file
tables, API-standard sites, a concrete local-runner example, a cloud-bridge example, and
the env vars that redirect endpoints or selection.

Authoritative research (full body + validated frontmatter):
`claudine/docs/research/model-config/<provider>.md`. Runner-side ground truth:
`claudine/docs/research/local_runners/<runner>.md` and the **local-llm-runners** skill.