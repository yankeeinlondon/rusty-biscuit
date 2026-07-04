---
sequence:
- name: draft
- name: iterate
- name: finalize
prompt: |-
  Local model runners — Ollama, LM Studio, oMLX, Llama.cpp, and vLLM — are servers that host models on the user's own hardware and expose OpenAI- or Anthropic-compatible APIs. They matter to Claudine because agentic CLIs bridge to them for local models, and because runner detection is a likely future sniff surface.

  ## Task

  Your task is to report on local model runners across the local model runners relevant to the providers Claudine supports.

  - your report should start by outlining why local runners matter to agentic CLIs (privacy, cost, offline use, brand-new open-weight models)
  - and then shift its focus to how the runners differ: per-OS binaries and installs, API surfaces (OpenAI/Anthropic compatibility), detection probes, configuration, model-id grammar, and notable traps
  - close with a point of view on the implications for Claudine's model-config bridging story and future runner detection

  As background material we have local-runner research documents for each runner that Claudine supports. They can be found at `@claudine/docs/research/local_runners/*.md`.

  Important: your final response is saved verbatim as the body of this summary document, so it must be the complete document text and nothing else — no preamble, no commentary. Never write to this document yourself.

  ::block when="state.name == 'draft'"
  - Iterate over the first three research documents to develop a point of view on how to write this document and then produce an initial draft of the document
  ::end-block
  ::block when="state.name == 'iterate'"

  - Note: the initial draft has already been created — it is the body of `@claudine/docs/research/summary/local-runners.md` (everything below the frontmatter); read it from there
  - Act as an orchestrator and iterate over each remaining runner's research document:
      - provide the subagent the current draft and ask them to return an improved draft based on the research document they've been assigned
  - Once every remaining runner has been incorporated, your final response is the fully updated draft
  ::end-block

  ::block when="state.name == 'finalize'"

  The document has now gone through several rounds of improvement and your task is just to make sure the document is consistent in tone and detail and that nothing looks incorrect or incomplete. The current draft is the body of `@claudine/docs/research/summary/local-runners.md` (everything below the frontmatter); read it from there, make any adjustments, and your final response will be considered the finalized summary document.
  ::end-block
hash: dc99bbf1562bfb91-6fbcc80ba405ca40
last_updated: 2026-07-03
---
## Why Local Runners Matter to Agentic CLIs

Local model runners matter because agentic CLIs sit directly on the boundary between a user's code, filesystem, tools, prompts, and model provider. Running inference locally changes that boundary.

The obvious benefit is privacy: source code, shell output, plans, and tool traces can stay on the user's machine instead of crossing a hosted API boundary. That does not make an agentic workflow automatically safe, but it gives users and organizations a deployment option where model inference is local and network exposure is explicit.

Cost is the second driver. Agentic CLIs can generate long conversations, repeated tool loops, retries, and background analysis. A local runner moves marginal token cost onto existing hardware. That trade is especially attractive for exploratory coding, repo indexing, test-failure iteration, and lower-stakes automation where hosted frontier-model quality is not always required.

Offline use is the third driver. Local runners let an agentic CLI continue working on airplanes, in restricted networks, during provider outages, or in environments where internet access is intentionally unavailable. The agent may still need network access for package downloads or documentation lookup, but inference itself can be independent.

Finally, local runners are often the fastest way to try brand-new open-weight models. Hosted provider catalogs can lag behind model releases, while runners such as Ollama, LM Studio, oMLX, llama.cpp, and vLLM can expose Hugging Face, GGUF, MLX, ModelScope, or local-path models almost immediately. For Claudine, that matters because model support is no longer only a provider question. It is also a bridge question: can a provider CLI be pointed at a local OpenAI- or Anthropic-compatible server, and can Claudine make that relationship discoverable, configurable, and predictable?

## Runner Shape

The five runners Claudine tracks are all HTTP servers, but they have different product shapes.

| Runner    | Shape                                                                       | Default           | Primary install pattern                                                                        |
|-----------|-----------------------------------------------------------------------------|-------------------|------------------------------------------------------------------------------------------------|
| Ollama    | Open-source app/service plus CLI for GGUF models                            | `127.0.0.1:11434` | macOS app or Homebrew, Linux service/script/Docker, Windows tray app or zip                    |
| LM Studio | Desktop/headless local model app with closed-source server and open tooling | `127.0.0.1:1234`  | macOS/Windows/Linux app, `lms` CLI, `llmster` daemon                                           |
| oMLX      | Open-source macOS Apple Silicon MLX runner                                  | `127.0.0.1:8000`  | macOS app, Homebrew tap, Python/source path                                                    |
| llama.cpp | Low-level open-source inference engine and `llama-server`                   | `127.0.0.1:8080`  | `llama-server` / `llama-server.exe` via package manager, release archive, source build, Docker |
| vLLM      | Open-source high-throughput GPU serving engine                              | `0.0.0.0:8000`    | Linux Python package/container; Windows via WSL2; macOS GPU via separate vLLM-Metal project    |

Ollama and LM Studio are productized local-runner experiences. They manage local model inventories, background services, and user-facing app state. llama.cpp is closer to a direct server binary: extremely portable and explicit, but mostly configured at launch time. oMLX is specialized around Apple's MLX stack and is effectively macOS/Apple Silicon-only. vLLM is the most server-oriented of the group: it is designed for throughput, GPU serving, and deployment-style operation rather than a desktop local-model UX.

That difference affects Claudine in practice. A bridge layer should not treat "local runner" as one uniform integration. Some runners have app bundles, background daemons, model inventories, and launch helpers. Others are just a process with flags and a port.

## API Surfaces

All five runners expose OpenAI-compatible APIs under `/v1`, so the OpenAI-style base URL includes `/v1`:

| Runner    | OpenAI base URL             |
|-----------|-----------------------------|
| Ollama    | `http://localhost:11434/v1` |
| LM Studio | `http://localhost:1234/v1`  |
| oMLX      | `http://localhost:8000/v1`  |
| llama.cpp | `http://localhost:8080/v1`  |
| vLLM      | `http://localhost:8000/v1`  |

All five also expose an Anthropic Messages-compatible surface at `/v1/messages`, so Anthropic-style clients should use a base URL without `/v1`:

| Runner    | Anthropic base URL       |
|-----------|--------------------------|
| Ollama    | `http://localhost:11434` |
| LM Studio | `http://localhost:1234`  |
| oMLX      | `http://localhost:8000`  |
| llama.cpp | `http://localhost:8080`  |
| vLLM      | `http://localhost:8000`  |

This split is important. OpenAI-compatible client configuration usually wants a base URL ending in `/v1`; Anthropic-compatible client configuration usually wants the server root because the SDK or client appends `/v1/messages`.

The compatibility surfaces are not identical to the upstream hosted APIs. Ollama's Anthropic endpoint omits features such as prompt caching, batches, citations, PDF content, token counting, `tool_choice`, and metadata. LM Studio documents Anthropic compatibility for Claude Code-style use, but stateful Anthropic features such as prompt caching, extended thinking, batches, and citations are not supported. llama.cpp routes Anthropic requests through its internal OpenAI-compatible path, and tool behavior depends on Jinja chat templates and model support. vLLM's Anthropic surface requires `Authorization: Bearer` when auth is enabled, not `x-api-key`, and does not establish full Anthropic feature parity. oMLX advertises richer Anthropic behavior, including token counting, vision, thinking, and tool use, but execution still depends on model/template support.

For Claudine, the correct abstraction is "API standard bridge" rather than "provider replacement." A local Anthropic-compatible endpoint may be enough for Claude Code to run, but it is not necessarily Anthropic.

## Detection Probes

Detection needs two layers: installation detection and running-server identification.

Installation detection is platform-specific:

| Runner    | Useful install signals                                                                                  |
|-----------|---------------------------------------------------------------------------------------------------------|
| Ollama    | `ollama` binary, macOS `/Applications/Ollama.app`, Linux `ollama.service`, Windows `ollama.exe`         |
| LM Studio | `lms` binary, macOS `/Applications/LM Studio.app`, `llmster` daemon, LM Studio home pointer             |
| oMLX      | `omlx` binary, macOS `/Applications/oMLX.app`, `omlx-server` process                                    |
| llama.cpp | `llama-server` or `llama-server.exe` binary/process; older `server` / `server.exe` names in some builds |
| vLLM      | `vllm` entry point, Python process, container process, or WSL-side install                              |

Running detection must be HTTP-response based. Ports alone are insufficient.

| Runner    | Strong running probe                                                                                                                                                         |
|-----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Ollama    | `GET /` returns `Ollama is running`; `GET /api/version` returns version JSON                                                                                                 |
| LM Studio | `GET /v1/models` returns OpenAI-style model list with `owned_by: organization_owner` when auth is off; `/api/v1/models` has LM Studio model metadata                         |
| oMLX      | `GET /health` returns JSON with `status: healthy`; `/openapi.json` identifies `oMLX API`                                                                                     |
| llama.cpp | `GET /health` returns `{"status":"ok"}` or a loading status; `/v1/models` often reports `owned_by: llamacpp`; `/props` includes `build_info` and `model_path` when available |
| vLLM      | `GET /version` returns version JSON; `/metrics` contains vLLM Prometheus markers; `/health` is liveness only because it returns an empty 200                                 |

The main detection trap is collision. oMLX and vLLM both default to port `8000`. A port scan cannot distinguish them. Claudine should identify by response marker: oMLX has a JSON `/health` body and `oMLX API` OpenAPI title; vLLM has `/version` and vLLM metrics. A second trap is endpoint overlap: llama.cpp exposes Ollama-style paths such as `/api/tags`, `/api/show`, and `/api/chat`, so an Ollama-looking path is not automatically an Ollama server. A third trap is auth: LM Studio, oMLX, llama.cpp, and vLLM can all require API keys for useful model-list or inference endpoints. Detectors should prefer ungated identity endpoints where they exist and treat auth failures as partial evidence, not as a confirmed runner identity by themselves.

## Configuration

Configuration varies sharply.

Ollama is mostly environment, app state, and native CLI behavior. It has a stable default port, no local auth by default, and a simple local API story. Important knobs include `OLLAMA_HOST`, `OLLAMA_MODELS`, `OLLAMA_CONTEXT_LENGTH`, `OLLAMA_KEEP_ALIVE`, concurrency and queue variables, CORS origins, cloud-disable settings, and GPU/cache controls. It also has runner-native launch helpers that can open agentic CLIs against the local server.

LM Studio has GUI, CLI, daemon, native REST, and JSON-backed app/server state. It is a richer local app, not just a server binary. The `~/.lmstudio-home-pointer` file can relocate the active home directory, and server state such as port, bind address, CORS, auth, and just-in-time loading lives under that home. Its model inventory can include downloaded and loaded models, and just-in-time loading changes what `/v1/models` means.

oMLX is macOS-specific and MLX-oriented. It uses JSON settings under `~/.omlx`, supports environment overrides such as `OMLX_HOST`, `OMLX_PORT`, `OMLX_BASE_PATH`, `OMLX_MODEL_DIR`, `OMLX_API_KEY`, and `OMLX_MCP_CONFIG`, and exposes a substantial admin API. Its default port collides with vLLM, so detection must be careful.

llama.cpp is flag-first. There is no primary server config file by default. Configuration lives in CLI flags, generated `LLAMA_ARG_*` environment variables, optional Web UI JSON via `--webui-config-file`, and optional router presets via `--models-preset`. Important knobs include `LLAMA_ARG_HOST`, `LLAMA_ARG_PORT`, `LLAMA_ARG_API_PREFIX`, `LLAMA_API_KEY`, `LLAMA_ARG_MODEL`, `LLAMA_ARG_MODEL_URL`, `LLAMA_ARG_HF_REPO`, `LLAMA_ARG_HF_FILE`, `LLAMA_ARG_DOCKER_REPO`, `LLAMA_CACHE`, `LLAMA_OFFLINE`, `HF_TOKEN`, `LLAMA_ARG_MODELS_DIR`, `LLAMA_ARG_MODELS_PRESET`, `LLAMA_ARG_EMBEDDINGS`, `LLAMA_ARG_RERANKING`, `LLAMA_ARG_N_GPU_LAYERS`, `LLAMA_ARG_CTX_SIZE`, and `LLAMA_ARG_JINJA`. This makes it portable and scriptable, but detection and bridging should expect many launch shapes.

vLLM is deployment-shaped. It is configured through `vllm serve` flags and optional YAML files passed with `--config`; long-form CLI argument names become YAML keys, and explicit CLI flags override config-file values. Important flags include `--host`, `--port`, `--api-key`, `--served-model-name`, `--chat-template`, `--enable-auto-tool-choice`, `--tool-call-parser`, `--generation-config vllm`, `--download-dir`, `--enable-tokenizer-info-endpoint`, and `--enable-offline-docs`. Its default bind is the biggest security-sensitive difference from the desktop runners: a default vLLM server may listen on `0.0.0.0`, depending on host networking and firewall state. The similarly named `VLLM_HOST_IP` and `VLLM_PORT` are distributed-internal settings, not the HTTP API bind; use `--host` and `--port` for serving.

## Model ID Grammar

Model IDs are not portable across runners.

| Runner    | Common grammar                                                                                                                    |
|-----------|-----------------------------------------------------------------------------------------------------------------------------------|
| Ollama    | `name[:tag]`, `namespace/model`, `hf.co/{user}/{repo}[:quant]`                                                                    |
| LM Studio | `publisher/model`, `publisher/repo/file.gguf`, `id@quant`                                                                         |
| oMLX      | local model directory, `{owner}/{model}`, `<model>:<profile>`                                                                     |
| llama.cpp | `--alias`, GGUF filename/path, Hugging Face repo plus optional quant, direct model URL, Docker Hub model selector, router-mode ID |
| vLLM      | Hugging Face model ID, local model directory, local GGUF path, ModelScope ID, or `--served-model-name` alias                      |

This is the strongest argument against a naive global local-model catalog. The same underlying weights might be named `qwen3:8b` in Ollama, `mlx-community/Qwen...` in oMLX, a GGUF filename in llama.cpp, and a Hugging Face ID in vLLM. Claudine should preserve runner-native model IDs and map them into provider config as explicit bridge entries. Normalization can help display and discovery, but execution needs the exact ID the runner expects.

llama.cpp and vLLM are especially important here. In llama.cpp, `--alias` controls the model ID reported by `/v1/models`; without it, single-model mode may expose the GGUF path or filename. Router mode can expose IDs from `LLAMA_CACHE`, `--models-dir`, or `--models-preset`, and those IDs are selectors. In vLLM, the launch model is the `--model` value, but the client-facing ID is what `/v1/models` returns; `--served-model-name` can replace a raw path or Hugging Face ID with one or more aliases.

## Notable Traps

Base URL construction is easy to get wrong. OpenAI-compatible bridges should generally include `/v1`; Anthropic-compatible bridges should generally omit it.

Compatibility is partial. A successful `/v1/messages` request does not imply support for every Anthropic feature an agentic CLI may try to use. Tool calls, structured output, images, extended thinking, prompt caching, token counting, and stateful responses all need runner-specific expectations.

Auth varies. Ollama has no local auth by default. LM Studio, oMLX, llama.cpp, and vLLM can enable API-key auth. vLLM's built-in auth guards `/v1`, `/v2`, and `/inference`, while probes such as `/health`, `/version`, `/load`, `/metrics`, `/tokenize`, and `/detokenize` remain available. llama.cpp keeps endpoints such as `/health`, `/v1/health`, `/models`, `/v1/models`, `/api/tags`, and `/` public when API-key auth is enabled, but gates inference and some metadata/admin endpoints. LM Studio and oMLX can gate model-list endpoints, so detector confidence should account for auth state.

Bind addresses vary. Most desktop runners bind localhost by default. vLLM defaults to `0.0.0.0`, which is a different exposure posture and should be surfaced if Claudine ever reports detected runners. Ollama can also be intentionally exposed with `OLLAMA_HOST=0.0.0.0:11434`.

Model inventory semantics vary. Ollama can list downloaded models and loaded processes separately. LM Studio may list downloaded or loaded models depending on server settings. oMLX exposes loaded status through model status/admin endpoints. llama.cpp single-model mode is not the same as router mode. vLLM usually serves one base model per server process; multiple model names are aliases, not multiple loaded base models, unless the user uses LoRA adapters or multiple processes/ports.

Runner-native launch support is uneven. Ollama and oMLX have launch helpers for agent CLIs. LM Studio, llama.cpp, and vLLM generally require starting the server and configuring the agent CLI manually.

Several knobs are misleading. llama.cpp uses `LLAMA_ARG_PORT`, not `LLAMA_PORT`. vLLM's `VLLM_PORT` is not the HTTP serving port. LM Studio's home pointer can relocate the entire effective config and model store. llama.cpp's `/api/tags` is not proof of Ollama. vLLM's `/health` is not proof of vLLM when oMLX shares the same default port.

## Implications for Claudine

Claudine's model-config bridging story should treat local runners as bridge targets behind provider CLIs, not as first-class agent providers. The provider still owns the agent loop, tools, prompts, permissions, and stream format. The local runner owns inference. Claudine's job is to make that boundary explicit.

The useful bridge model is:

1. Detect installed and running local runners.
2. Identify the runner by response marker, not port.
3. Discover candidate model IDs through runner-native metadata endpoints where available.
4. Generate provider-specific configuration using the correct API standard and base URL.
5. Preserve runner-native model IDs instead of inventing a universal local-model ID.
6. Surface traps: auth required, non-local bind, port collision, partial API compatibility, no running server, loading state, and single-model versus router/multi-model semantics.

For OpenCode-style bridges, Claudine can write an OpenAI-compatible provider block pointing at `http://localhost:<port>/v1` and enumerate runner model IDs. For Claude Code-style bridges, Claudine can set or document `ANTHROPIC_BASE_URL=http://localhost:<port>` and an appropriate token placeholder when the runner expects one. For runners with launch helpers, Claudine can detect and mention the native path, but it should not depend on those helpers as the only integration path.

Future sniff integration should model runners as a typed detection surface with confidence levels: installed, app present, daemon/process running, HTTP identity confirmed, loading, auth-gated, models visible, bind exposure, and inventory mode. That gives Claudine enough information to say something useful without overclaiming. "Ollama installed" is different from "Ollama running"; "port 8000 open" is different from "oMLX confirmed"; "Anthropic-compatible endpoint exists" is different from "Claude-compatible behavior is complete."

The long-term point of view is that local runner support should be boring and explicit. Claudine should not hide the differences between Ollama, LM Studio, oMLX, llama.cpp, and vLLM. It should turn those differences into clear bridge configuration, accurate detection, and actionable warnings so users can choose privacy, cost control, offline use, or fresh open-weight models without guessing how their agentic CLI is actually wired.
