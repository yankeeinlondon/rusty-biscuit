# Summary-Surfaced Behavior-Gap Triage Backlog

> Seeded 2026-07-03 from the cross-provider summaries (`docs/research/summary/*.md`).
> Governing rule (spec, "Expectation, 2026-07-03"): every item gets an explicit
> disposition — **implement / schedule / won't-do** — at the Phase D checkpoint for its
> topic; surfacing alone is not completion. Add new items as later summaries/fleets
> surface them; never delete, mark disposition instead.
>
> Disposition legend: `[ ]` untriaged · `[I]` implement now · `[S]` scheduled · `[W]` won't-do

## MCP (`summary/mcp.md`)

- [ ] Goose/Kimi/Qwen have provider-native MCP support with no Claudine import/export/runtime path
- [ ] Claude Code native `--mcp-config` one-run mechanism has no Claudine runtime injector
- [ ] Plugin-declared MCP servers (Kimi plugins, Gemini/Qwen extensions, Kilo Marketplace `MCP.yaml`) are a fourth discovery source for the MCP catalog — import candidates with credential stripping; catalog stays authoritative, plugin entries are never authoritative
- [ ] ACP `session/new` `mcpServers` injection is a candidate MCP delivery path (cleaner than shadow config where wired; NOT wired for Pi, partial for Codex)

## Agent Logging (`summary/agent-logging.md`)

- [ ] Provider-log **evidence adapters** feeding `claudine logs` as a federated observability layer: WAL-aware SQLite readers (OpenCode, Kilo CLI, Goose, Codex state DBs), Gemini `$set`-aware transcript parsing, Pi tree-preserving transcript reader
- [ ] Kimi Wire protocol-version tolerance — observed 1.9 parser pin vs 1.10 server is live breakage (**triage early**)
- [ ] Shared path/volatility inventory: resume storage (Codex `state_5.sqlite`, session indexes, Kimi state files) overlaps the shadow-home volatile-SQLite skip list and the evidence-adapter plan — one inventory, multiple consumers

## Agent Permissions (`summary/agent-permissions.md`)

- [ ] Decide the Goose wrapper's default posture — Goose's own default `GOOSE_MODE=auto` is effectively YOLO (auto-approves all visible tools, ignores `permission.yaml`)
- [ ] Model OpenCode `--auto` as auto-reply-**once**, not a standing approval
- [ ] Kimi's only precise programmatic approval transport is ACP (feeds the ACP-adoption disposition below)

## System Prompt (`summary/system-prompt.md`)

- [ ] Kimi `--rsp` disposition: current Kimi Code has NO per-session replace surface; only legacy `kimi-cli --agent-file` can replace — split by implementation, version-scoped (`since`/`until`)
- [ ] Goose `--rsp` disposition: no clean per-launch replace (persistent template config mutates user config; `GOOSE_SYSTEM_PROMPT_FILE_PATH` undocumented; ACP `set_session_system_prompt` is `goose acp`-only)
- [ ] Goose `--asp`: `goose run --system` only (inline text, argv limits, conflicts with `--recipe`, no `goose session` equivalent) — interactive fallback is a weaker context file
- [ ] `OPENCODE_CONFIG_CONTENT` merge contract: system-prompt injection, MCP injection, and permission overlays all write ONE env var — overwriting breaks the others (also lands in plan Phase C shared prep)
- [ ] Qwen/Gemini: inline-only prompt flags (argv limits), `QWEN_SYSTEM_MD`/`GEMINI_SYSTEM_MD` missing-file is FATAL; Gemini replace can drop dynamic substitution slots (tools/skills/subagents)
- [ ] Kimi legacy Jinja2 `StrictUndefined`: stray `${...}` in Claudine-composed prompt content can abort the session
- [ ] Docs: define `--rsp` as "replace the provider base prompt where possible" — never context isolation (memory files/skills/MCP instructions still load; OpenCode replaces slot 0 only)

## Non-Interactive Sessions (`summary/non-interactive-sessions.md`)

- [ ] Kimi Wire peer obligations: verify Claudine answers/rejects blocking `ApprovalRequest`/`QuestionRequest`/`HookRequest`/`ToolCallRequest` by policy (`--afk` = safe unattended posture); print mode is a lossy projection, not an equivalent fallback
- [ ] OpenCode promoted stderr (`--print-logs --log-level INFO`) is CONTRACT, not noise — reconcile with `stderr_noise_prefixes` / `suppress_structured_stderr_on_success` semantics; the model-mismatch guard depends on this channel
- [ ] Goose invocation hardening: `--quiet` mandatory for parse-safe stdout; generate stable `--name` run-id; no init event (capture launch metadata wrapper-side); `error`-then-`complete` must taint the run (`complete` carries no status)
- [ ] Qwen: accept both `system/init` and `system/session_start`; classify exit codes 53 (max turns) / 55 (wall-clock) / 130 (interrupt) which bypass `result` (also Phase E declarative-record candidates)
- [ ] Codex exec JSONL omits launch metadata (cwd/model/provider/auth/version/MCP/sandbox/approval) — wrapper-side capture required
- [ ] Cross-provider wrapper-metadata capture layer (argv, cwd, prompt-delivery mode, sandbox/approval, provider version, session/resume ids) — one systematic capability, not eight patches

## Hooks (`summary/hooks.md`)

- [ ] **Codex is no longer notify-only**: first-class 10-event hook system (blocking, `updatedInput` mutation, permission allow/deny) — Claudine's Codex registration under-covers the canonical events (**triage early**)
- [ ] No `after_compact` canonical event — Codex/Kimi/Qwen `PostCompact` + OpenCode `session.compacted` all collapse into `notification` (canonical-enum change; Phase A1 catalog-types)
- [ ] Tool-selection phase (Gemini `BeforeToolSelection`, OpenCode `tool.definition`) is unmappable to `before_tool` — fires before a concrete tool call exists
- [ ] OpenCode blocking authority requires dedicated plugin hooks, NOT the observe-only event bus — verify which mechanism Claudine bridges
- [ ] Goose `SubagentStart`/`SubagentStop` appear in the Open Plugins spec but are NOT emitted — don't register as active
- [ ] Goose hook registration = authoring an Open Plugin (`~/.agents/plugins/<root>/hooks/hooks.json`, two config files to enable, `sh -c` execution — Windows needs Git Bash/MSYS2)
- [ ] Kimi undocumented client-side wire-hook channel (JSON-RPC `HookRequest` at ACP/wire init) — no Claudine last mile
- [ ] Registration-health detection: per-provider kill switches silently defeat Claudine hooks (Qwen `disableAllHooks`/`--bare`/safe-mode, Codex feature flags + `/hooks` trust disable, OpenCode `OPENCODE_PURE`)
- [ ] `human_in_the_loop` has no uniform provider hook — dispatchability review

## Session Resumption (`summary/session-resumption.md`)

- [x] **Resume parity (ruled by Ken, 2026-07-04):** if the provider natively supports resume, Claudine must support using it. The gemini/goose/opencode wrapper gap (stale legacy `resume_supported: false` despite first-class provider support) was closed same-day: all 7 profiles now implement `supports_resume` + `build_resume_args` from the research-ratified argv forms, guarded by `every_provider_profile_supports_resume`. Remaining half: graduate `ResumeSpec` into the typed catalog (Phase D field expansion) so support derives from catalog data and only argv mechanics stay behavior
- [ ] Handle capture + persistence is THE prerequisite: capture a stable session handle early (structured output/hooks/list APIs), persist in run metadata, resume by explicit handle — "continue latest"/pickers are not recovery primitives
- [ ] Resume does not restore launch environment (model/sandbox/approval/cwd/MCP may be recalculated) — record launch metadata and reapply
- [ ] Serialize resume attempts per provider session id (Claude warns of interleaved transcript writes)
- [ ] Kimi current surface: `KIMI_CODE_HOME` → `~/.kimi-code` (not `~/.kimi`), follow-up via `-p/--prompt` not legacy `--print` — version-scope the facts
- [ ] Goose: plain `--resume` is GLOBAL, not repo-scoped — automation must use explicit `--session-id` (also preserves provider/model metadata); wrapper restores cwd
- [ ] Record non-resumable launch conditions at launch time (Codex `--ephemeral`, Claude `--no-session-persistence`, Goose `--no-session`, archived Qwen sessions)
- [ ] Two-step API resume paths (Codex app-server `thread/resume`+`turn/start`, Qwen daemon `load`/`resume`+`prompt`, OpenCode server endpoints) — the durable-HITL surfaces, no Claudine last mile
- [ ] Claude durable HITL: `PreToolUse` defer in print/SDK flows (capture `tool_deferred` + ids, resume same session after out-of-band input) — feeds `human_input_requested`/`session_resumable` signals

## ACP (`summary/acp.md`)

- [ ] **Adoption disposition (proposed by summary)**: hybrid — keep CLI wrappers as baseline; add ACP provider-by-provider as typed streaming + permission backend first; capability-gate every reverse-request handler; record ACP event streams into reporting before ACP replaces any execution path
- [ ] Enforcement-plane framing is valid per delegation TIER, not per protocol — only Gemini and Goose route fs+terminal through reverse requests; Claude/Codex adapters keep execution internal
- [ ] Adapter dependency drift: Claude/Codex/Pi need external `npx` adapter packages; Pi has two divergent adapter lines
- [ ] Gemini `--experimental-acp` is deprecated → `gemini --acp`; purge stale references
- [ ] ACP `session/update` stream is a missing `source` value in the signal detection-record enum (Phase E)

## Skills / Slash Commands / Plugins (`summary/agent-skills.md`, `summary/slash-commands.md`, `summary/agent-plugins.md`)

- [ ] Generic `.agents/skills/` root (scanned by 8 of 9 providers) is an unexploited single projection target; linking into brand paths on providers that also scan `.claude/skills/` risks double-discovery
- [ ] Precedence/conflict semantics are provider-divergent (first-wins / last-wins / tiered) — linked skills can be silently shadowed; linker assumes none of them
- [ ] Activation dependencies can silently disable linked skills (Gemini consent gate, Goose Summon dependency, OpenCode/Kilo `skill`-tool permission, Pi `read`-tool gating) — "Linked But Degraded" warnings needed
- [ ] Inventory-only resource class: managed/bundled/plugin/built-in/URL/marketplace scopes must be inventoried, never synced
- [ ] Format-convertible command artifacts (Gemini TOML, Goose config+recipe, OpenCode JSON entries) break link-as-file — linker needs parse-and-emit-native conversion
- [ ] Codex (`$name` skills only) and Kimi (`/skill:`/`/flow:`) have no user `/name` surface — don't present linked commands as native slash commands there
- [ ] Argument/interpolation grammar rewrite map (`$ARGUMENTS`/`$N` vs `{{args}}` vs Jinja vs append-only; shell/file injection syntaxes) — cross-linking without it silently corrupts behavior
- [I] Reconcile the two portability taxonomies (skills 5-class vs commands 4-case) into ONE classification enum — **ratified 2026-07-03 (Ken)**: 5-class skills enum, `PortableWithProviderMapping` widened to cover deterministic format conversion, conversion-ness carried as `artifact_kind` + `conversion: none/mechanical/semantic` facts (see spec table-B row)
- [ ] Plugin-carried resources are invisible to the linker — extraction with provenance (source provider, plugin name/version, namespace) is the ruled approach
- [ ] Plugin bundles as a Claudine distribution channel: summary answers "yes, selectively" (generated per-provider manifests; never one shared package) — needs explicit disposition
- [ ] Runtime-executable plugin surfaces (OpenCode/Kilo `plugin/` dirs, Pi TypeScript extensions) must be codified as linking exclusions

## Subagents (`summary/subagents.md`)

- [ ] **Subagent observation adapters** with one normalized child record (parent session id, child agent/session id, type, invocation prompt, start/stop-or-idle markers, final text, transcript/store handle, termination reason, permission mode): Claude/Codex/Qwen/legacy-Kimi map to real `SubagentStart`/`Stop`; Gemini must be synthesized from `tool_use`/`tool_result`; OpenCode from `subtask` part → `session.created parentID` → `session.idle` (this is the recipe for the known OpenCode subagent-invisibility blind spot); Goose from `subagent_tool_request` MCP notifications + `load()` result metadata
- [ ] **Observability-strength gating** for child recovery: `strong_child_identity / session_child_identity / tool_call_with_captured_output / tool_call_only` — only strong identity can support future child `resume`/`proxy`/transcript reporting; don't promise uniformly
- [ ] Kimi subagent linking is version-sensitive: legacy `kimi-cli` YAML `--agent-file` per-session only (no user/repo discovery dirs — NOT a filesystem-linking target); newer `kimi-code` drops that surface
- [ ] Compatibility-origin tagging: Goose scans `.claude/agents/` (same file discoverable via two providers — tag as compatibility-origin, report ignored Claude-only frontmatter); Qwen deliberately bridges Claude agent schema but the reverse direction is lossy — represent as ASYMMETRIC compatibility
- [ ] Codex agent files are full TOML config layers (model/sandbox/approval/MCP/skills) with no `--agent` main-session mode — never equivalent to a Claude/OpenCode primary agent; linking requires body→`developer_instructions` + frontmatter→TOML translation
- [ ] Goose frontmatter `model` is metadata-only (runtime model comes from parent/delegate/recipe) — don't project model intent into it

## Local Runners / Model Config (`summary/local-runners.md`, `summary/model-config.md`)

- [ ] Typed runner-detection surface (sniff) with confidence ladder + identify-by-response-marker never by port (oMLX and vLLM share :8000; llama.cpp serves Ollama-style `/api/tags`); include bind-exposure reporting (vLLM defaults `0.0.0.0`)
- [ ] Bridge-config generation: Claudine writing OpenAI-compatible provider blocks (`http://localhost:<port>/v1`) / `ANTHROPIC_BASE_URL` env for detected runners
- [ ] Partial-compatibility warnings: per-runner Anthropic feature gaps (Ollama: no prompt caching/token counting/`tool_choice`; LM Studio: no extended thinking; vLLM: `Bearer` not `x-api-key`) — a working `/v1/messages` ≠ full feature support
- [ ] Gemini CLI cannot bridge to local runners without a Gemini-protocol translating proxy — exclude or document
- [ ] OpenAI dialect split: Codex is Responses-only (`/v1/responses`) — bridge generation must distinguish `openai_chat` vs `openai_responses` and know when a proxy is required
- [ ] Codex `model_catalog_json` is replacement-shaped (touch it → own the whole catalog); provider ids `openai`/`ollama`/`lmstudio` are RESERVED — generated Codex config must respect both
- [ ] Stale-manual-block detection: shadow-semantics providers + Kilo's hourly Models.dev refresh make stale user model blocks a detectable drift condition (fits Phase F drift-channel SignalEvents)
- [ ] Qwen project-scope settings REPLACE user-level `modelProviders` (not merge) — config inspection/generation trap

## Cross-Topic

- [W] **Roo refresh sweep** — Roo research is missing/stale across six topics (agent-cli, agent-logging, hooks, resume, acp, subagents) and absent from skills/permissions rosters; one consolidated sweep, not per-topic patches. **Won't-do (2026-07-04): superseded by Checkpoint B's full Roo Code removal** (enum variant, roster entry, research documents, facts file all deleted); there is no Roo surface left to refresh
