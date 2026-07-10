# claudine-contract

A [`biscuit_contract::inference::InferenceAdapter`] implementation backed by
**Claudine**. It runs an agentic CLI (Claude Code, Codex, …) as a **single
non-interactive, tool-free, filesystem-isolated session** and returns its final
assistant text as the inference result.

Deterministic consumers — Reaper first, Darkmatter next — inject it as
`Arc<dyn InferenceAdapter>` and never take a direct dependency on a provider
crate. The adapter reuses the user's existing agentic-CLI authentication and
Claudine's provider-normalization, stream-parsing, and model-catalog machinery.

```rust
use std::sync::Arc;
use biscuit_contract::inference::InferenceAdapter;
use claudine::provider_id::Provider;
use claudine_contract::ClaudineInferenceAdapter;

let adapter: Arc<dyn InferenceAdapter> =
    ClaudineInferenceAdapter::new(Provider::Claude).build();
```

## Why a separate crate

`biscuit-contract` is a tiny, provider-neutral contract; `claudine` is a large
library. This crate is the only place the two meet, so consumers depend on the
contract alone and inject this adapter at their composition root.

```text
biscuit-contract
  ^                      claudine (lib)
  |                        ^
  +---- claudine-contract -+
          ^
          |
          +-- reaper (consumer, via Arc<dyn InferenceAdapter>)
          +-- darkmatter (consumer)
```

## Security posture

Consumers feed **untrusted scraped text**, and an agentic CLI is tool-capable
in its normal mode (it can read files, run shell commands, and call MCP tools).
The `InferenceAdapter` contract authorizes none of that. Every session
therefore runs constrained:

- **No tools / no MCP.** Tool use is constrained by real provider controls
  before the model turn starts, not by the guard prompt alone. The available
  primitive differs by provider, so the guarantee is layered: Claude has a hard
  deny-all (`permissions.deny: ["*"]` written into its shadow home plus
  `--strict-mcp-config`), which forbids every tool outright. Codex has no
  deny-all equivalent — its execution-rules can only forbid named command
  prefixes escaping the sandbox — so its tightest pre-turn lever is
  `--sandbox read-only`, blocking writes. Network denial is treated as a
  defense-in-depth assumption rather than a guarantee. A read-only command
  attempt is still possible under that sandbox, but it runs against the
  isolated empty working directory and shadow home (nothing sensitive is
  reachable), it surfaces as a tool item in the JSONL stream, and it is rejected
  post-hoc (see *Tool-use rejection* below) — so no tool takes effect or has its
  output trusted. The adapter also delivers an adapter-owned guard instruction
  forbidding tool use, file/network access, and shell execution as
  defense-in-depth, and never injects MCP servers.
- **Filesystem isolation.** The process runs in an ephemeral throwaway working
  directory (a `tempfile` dir), never the consumer's repository or CWD. Its
  `HOME` is a separate ephemeral **shadow home** containing only the provider's
  credential file(s) and the deny-all policy — never the user's real provider
  home (config, memory, MCP config, or session state).
- **Environment allowlist.** The child environment is cleared and rebuilt from
  an explicit allowlist: a minimal OS baseline (`PATH`, locale, …), the
  provider's model-selection variables, and the provider's authentication
  variables — and only those actually present. `HOME` is set to the shadow
  tree, never copied from the caller. The consumer's full environment never
  leaks through.
- **Prompt-injection boundary.** The guard instruction is delivered through the
  provider's system-prompt channel where one exists (e.g. Claude's
  `--append-system-prompt`), separately from the untrusted prompt, so injected
  text cannot rewrite the rules. Where no channel exists it is prepended with a
  clear delimiter.
- **Tool-use rejection.** If a completed session nonetheless recorded a tool
  call, permission prompt, or interactive user-input prompt, the request fails
  with `InvalidResponse` rather than returning leaked output.

Secrets, API keys, and full provider payloads never appear in
`InferenceError::message`; provider detail stays in `tracing` only.

## Provider support matrix (v1)

A provider is **enabled** only when it has a non-interactive entrypoint, a
structured stream protocol Claudine can parse (so the final assistant text is
captured deterministically), and has been curated as verified-runnable
tool-free for untrusted input. Everything else is reported as
`InferenceErrorKind::Unsupported` rather than run unsafely.

| Provider | Binary | Non-interactive entrypoint | Tool/MCP control | v1 status |
|----------|--------|----------------------------|------------------|-----------|
| Claude Code | `claude` | `--print --output-format stream-json --verbose` | deny-all `settings.json` (`permissions.deny: ["*"]`) in shadow home + `--strict-mcp-config --mcp-config '{"mcpServers":{}}'`; guard via `--append-system-prompt`; shadow HOME + isolated CWD + env allowlist | **Enabled** |
| Codex | `codex` | `codex exec --json` | `--sandbox read-only` (blocks writes; network denial is defense-in-depth) + post-hoc tool-call rejection; guard via `-c developer_instructions=…`; shadow HOME + isolated CWD + env allowlist | **Enabled** |
| Gemini CLI | `gemini` | `-p` / stream-json | — | Rejected — not yet verified tool-free for untrusted input in v1 |
| Goose | `goose` | run / stream-json | — | Rejected — not yet verified tool-free for untrusted input in v1 |
| Kimi Code | `kimi` | stream-json | — | Rejected — not yet verified tool-free for untrusted input in v1 |
| OpenCode | `opencode` | run / stream-json | — | Rejected — not yet verified tool-free for untrusted input in v1 |
| Qwen Code | `qwen` | stream-json | — | Rejected — not yet verified tool-free for untrusted input in v1 |

The matrix is also available programmatically via
[`claudine_contract::support_matrix`]. Widening the enabled set is a
deliberate, reviewed change — each entry asserts a safety guarantee over
untrusted input.

## Behavior

- **Profile.** [`InferenceProfile`] is best-effort. Reasoning effort maps onto
  the provider's typed reasoning capability and is emitted onto argv where the
  provider exposes a verified non-interactive config-override (Codex
  `-c model_reasoning_effort=…`). Where it does not (Claude's non-interactive
  reasoning control is not reliably verifiable), the preference is recorded on
  the session plan but not emitted, since an unrecognized flag could fail an
  otherwise valid run. An explicit model override is honored; absent one, the
  provider uses its own default and the reported model is filled from the stream
  summary. `InferencePriority` is **not** mapped onto a model: Claudine's static
  model catalog is a flat, untiered id list with no cost/latency/quality
  metadata to map a priority onto.
- **Structured output.** A uniform prompt-and-parse strategy: the JSON Schema
  (Draft 2020-12) is validated up front (`InvalidRequest` if malformed), the
  prompt is augmented to request a single conforming JSON value, the value is
  extracted from the assistant text (tolerating code fences and surrounding
  prose), and validated with the bundled `jsonschema` engine. Invalid JSON, a
  schema violation, or prose where structure was requested is `InvalidResponse`.
- **Cancellation.** Dropping the `infer` future kills the spawned child
  (`kill_on_drop`). The adapter deliberately owns no internal timeout; the
  consumer wraps `infer` with `tokio::time::timeout`.

## Error mapping

| Condition | `InferenceErrorKind` |
|-----------|----------------------|
| No non-interactive entrypoint / not runnable tool-free | `Unsupported` |
| Empty prompt / malformed schema | `InvalidRequest` |
| Provider binary missing or not executable | `Unavailable` |
| Authentication missing or rejected | `Unauthorized` |
| Rate limited | `RateLimited` (+ `retry_after` when known) |
| Overload / 5xx / unavailable | `Unavailable` (+ `retry_after` when known) |
| Session timed out | `Timeout` |
| Empty/garbled output, JSON parse failure, schema mismatch | `InvalidResponse` |
| Tool calls / permission prompts / user-input prompts observed | `InvalidResponse` |
| Any other provider/session failure | `Provider` |

## Testing

- **L1 (default):** deterministic tests drive the adapter through a fake
  session runner returning canned provider stdout fed through Claudine's real
  semantic parser — no agentic CLI is installed or spawned. Run with
  `cargo test -p claudine-contract` or `just test` in this area.
- **`real_` (opt-in):** end-to-end tests against a genuinely installed and
  authenticated provider, gated by `CLAUDINE_CONTRACT_REAL=1` and the provider
  binary being on `PATH`:

  ```sh
  CLAUDINE_CONTRACT_REAL=1 cargo test -p claudine-contract --test real_provider -- --nocapture
  ```

[`biscuit_contract::inference::InferenceAdapter`]: https://docs.rs/biscuit-contract
[`InferenceProfile`]: https://docs.rs/biscuit-contract
[`claudine_contract::support_matrix`]: src/support.rs
