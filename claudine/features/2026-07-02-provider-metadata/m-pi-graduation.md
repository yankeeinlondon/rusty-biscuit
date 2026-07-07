# M-Pi Graduation Report (Phase H, milestone #2)

> Checkpoint **H2** artifact. Pi (earendil-works) onboarded as the 9th compiled
> `Provider` — the first **non-cousin** graduation, with a bespoke stream
> parser + adapter authored from scratch. This report is the process retro:
> scaffold quality (first real facts-skeleton run), the non-cousin behavior
> half, and process accuracy — confirming the onboarding is
> provider-shape-independent.

## Outcome

`Provider::Pi` is live: generation is byte-clean for all 9 providers
(`claudine-gen check` clean across data/catalog/signals/families/roster), the
lib, CLI, and contract crates compile, and `claudine pi` wraps the Pi CLI with a
**bespoke** `--mode json` NDJSON stream parser. Pi entered with **complete
research coverage** — all 17 topic docs. Unlike M-Kilo (an OpenCode fork that
reused OpenCode's parser/adapter/plugin-bridge verbatim), M-Pi reused **nothing**
on the behavior half: a real `PiSemanticStreamParser`, `protocol/pi.rs` typed
models, a `PiAdapter`, and a no-hook `PiConfigurator` were all written from the
wire format. That is the milestone's whole point — proving the data/behavior
seam holds when the provider shape is genuinely new.

## Scaffold quality — the first real facts-skeleton run

M-Kilo built the `--scaffold` flow but never exercised the facts skeleton in
anger (Kilo's facts were hand-copied from OpenCode's up front). M-Pi is its
first real use. Findings:

- **The two-pass scaffold worked as designed.** `generate pi --scaffold` wrote
  `facts/pi.yaml` as a 21-field TODO skeleton (registry order, each field
  annotated `TODO(required|optional)` + its description) and stopped with a
  clear "fill the TODO(required) fields, then rerun" message. The skeleton was
  an accurate, self-documenting fill-in surface — a genuinely good UX. Every
  field mapped to a research doc, so filling it was mechanical.
- **Confirmed gap (M-Kilo follow-up #1): the skeleton omits `acp`.** As
  predicted, the skeleton emitted only the 21 `DeclaredSource::Facts` fields and
  **not** the `acp` sub-record (whose `client_supported`/`events_via_acp` are
  facts-fed while `server_mode` is research-declared). A fully onboarded
  provider still needs the `acp:` block hand-added. This bit exactly as the
  M-Kilo report warned. **Recommendation stands:** teach `scaffold_facts` to
  emit the `acp` keys (or print a "remember to add acp" line). Low effort, real
  papercut — now confirmed on a from-scratch provider, not just theorized.
- **`yolo: null` is a scaffold trap.** The skeleton seeds every field `null`,
  but `yolo`'s "no YOLO mode" encoding is the bare string `none`, not `null`
  (Pi is permissive-by-default with no named mode). Leaving it `null` would fail
  coercion. Minor, but a scaffold that seeded the *enum's* null-equivalent where
  one exists would be friendlier.

Net: the facts-skeleton UX is good and the fill-from-research loop is smooth.
The one concrete defect (missing `acp`) is a known, small follow-up.

## The non-cousin behavior half (parser/adapter authored from scratch)

Pi's `--mode json` wire format is NDJSON with a top-level `type` discriminator
and a nested `assistantMessageEvent.type` for streaming deltas — structurally
unlike any existing provider. What was authored:

- **`stream/protocol/pi.rs`** — a `#[serde(tag="type")] PiEvent` enum over 19
  event families plus their typed payloads. Recognized-but-silent lifecycle
  events (`turn_start`/`turn_end`/`message_start`/`agent_start`/
  `tool_execution_update`/queue/session-mutation) are modeled as `PiIgnore` so
  they are **dropped, not surfaced as `ProviderExtension` noise**; genuinely
  unknown future types still fall through to the `ProviderExtension` fallback.
- **`stream/providers/pi.rs`** — `PiSemanticStreamParser`, a from-scratch
  `SemanticStreamParser`. Key wire-format decisions, each grounded in the
  research:
  - `text_delta` → `OutputText` (accumulated into `assistant_text`);
    `thinking_delta` → `Reasoning`. Block-boundary and model-side `toolcall_*`
    deltas are dropped (tool execution is reported by the `tool_execution_*`
    lifecycle, so rendering `toolcall_*` would double-count).
  - `tool_execution_update.partialResult` is **dropped** — the research is
    explicit that it is an *accumulated snapshot*, not a delta, so appending it
    would duplicate text.
  - Errors are **normalized into assistant messages** (`stopReason: "error"` +
    free-text `errorMessage`); Pi exposes no structured error categories, so
    `classify_error` is text-based (rate-limit/quota/auth/abort → typed
    `SemanticErrorKind`), mirroring OpenCode's message-fallback path. An
    assistant-message error is `terminal: false` (a retry may follow;
    `agent_end` is the true terminal record).
  - `message_end` accumulates per-message usage/cost; `agent_end` emits the
    terminal `TurnComplete`.
  - 19 unit tests cover every routed event, the silent-lifecycle contract,
    usage accumulation, error classification, malformed-JSON warnings, and a
    round-trip serialization-fidelity fixture.
- **`adapters/pi.rs`** — `PiAdapter`. Pi has no native hooks, so this adapter is
  never reached through hook registration; it exists to satisfy the
  per-provider `ProviderAdapter` contract and give `claudine handle --provider
  pi` a best-effort normalization. `detect_from_payload` returns `false` and
  `representative_payload_for(Pi)` returns `None` (like Goose/Qwen/Kilo) — Pi
  delivers no raw hook payloads at all, so shape-based detection is meaningless;
  the wrapper always knows the provider from the `claudine pi` subcommand.
- **`config/pi.rs`** — `PiConfigurator`, the minimal no-hook kind
  (`SkipReason::NoHookSupport`), mirroring `QwenConfigurator`. This is what keeps
  the **hook-support invariant green by construction**: Pi's `event_mapping`
  facts declare zero `Hook`-level entries (all `stream_parse` or `not_supported`
  with `registration_target: false`), so `hook_events_imply_configurator_hooks_supported`
  needs no real configurator, and Pi is correctly **absent from
  `quick_start_supported_providers`** (no code edit needed there — the selection
  is facts-driven).

**Rendering: zero per-provider code.** Pi's event rendering is driven entirely
by its generated `DisplayPolicy` facts (minimal, all-default). No render-path
change — the Phase-G thesis holds on a non-cousin too.

## Process accuracy — what the checklist got right and where it bit

The amended six-step onboarding checklist (design/catalog-generation.md) was
accurate. The compiler-forced sites and the consistency-test sites were exactly
as enumerated. Two items worth recording:

- **The signals-hub trap was real and required a genuine restructure, not a
  swap.** As the phase brief warned,
  `hub.rs::provider_recovers_wired_attribution_only` used `"pi"` as its
  dormant/unwired example, and after Pi wires there is **no** researched-but-
  unwired slug left in `SIGNAL_TABLES`. Fix: the test now builds a **synthetic**
  `ProviderSignalTable { slug: "nonesuch", records: &[] }` in-test and asserts
  `provider()` is `None`. This exercises the "compiled slug with no `Provider`
  variant" path *permanently*, independent of any roster-only slug — so it will
  not need touching at M-Antigravity. The stale "dormant (kilo, pi)" comments in
  `hub.rs`, `signals/mod.rs`, `cli/signals.rs`, and `bespoke.rs` were reconciled
  (both are now wired); `signals/mod.rs`'s dormant-only test was rewritten to
  iterate all compiled tables + assert an unknown slug is `None`.
- **`pi` signals were already researched + generated.** Unlike the "author
  signals from scratch" framing one might assume, `signals/pi.md`, a generated
  `PI_SIGNALS` table, and a bespoke `PiRetriesExhausted` detector already
  existed (Pi was "dormant roster-only"). Graduation was therefore *removing*
  dormancy, not authoring — and the `PiRetriesExhausted` chain now runs at
  runtime for `Provider::Pi` runs.

## Generate-UX friction (all resolvable, none blocking)

- **`model_catalog_source` needed a field-keyed override** (the documented
  dynamic-listing hard-stop). Pi's `pi --list-models` *is* a shell command, but
  it prints a **human-formatted table**, not machine-parseable JSON — so unlike
  Kilo (`kilo models` → JSON), the honest catalog source is `none`
  (`overrides/pi.yaml`), mirroring the codex/kimi precedent. `expected_offerings`
  (research `default_models`) remains the validation baseline. Pinned with a
  reason; revisit if `pi --list-models` output is verified parseable.
- **`model_cli_flag` needed NO override** — a genuine improvement over M-Kilo.
  Pi's `model_selection` records `--model` as the **first bare-flag-token
  cli_flag site**, so the bare-token coercion yields `--model` correctly. (Kilo
  needed a pin because its site was the compound `"--model / -m"`.) This
  confirms the M-Kilo capture-side sidecar tightening reasoning: when research
  records one bare flag token, no override is required.
- **`expected_offerings` hit a duplicate-id error → reconciled the research.**
  Pi is an aggregator, so `default_models` legitimately listed the same model id
  as the per-provider default for multiple providers (`gpt-5.5`, `gpt-5.4`,
  `kimi-k2.6`, `moonshotai/Kimi-K2.6` each twice). Per the design's "collision =
  reconcile the research" rule, the four duplicate-id entries were **merged**
  in `agent-models/pi.md` (folding the second provider into the kept entry's
  notes — no information lost). This is a new-shaped friction M-Kilo did not
  hit (Kilo is a single-vendor fork); **worth adding to the onboarding notes:
  aggregator providers will collide on `default_models` ids and need a
  merge-not-drop reconciliation.**
- **Loud, harmless coercion skips:** `model_env_vars` (Pi has no single bare
  model env var — it is multi-provider) and `non_interactive_conflicting_flags`
  (compound annotated sites) both projected empty. Correct for Pi; no override.

## Facts judgment calls flagged for review

All are valid catalog-types wire forms (they generate) but are accuracy calls
Ken may want to confirm:

- **`platform_kind: agent_aggregator`** — Pi fronts 15+ providers via the user's
  own keys. Defensible; parallels Kilo.
- **`billing_models: [prepaid_credits, subscription, per_token]`** — Pi bills
  through the underlying provider (OpenRouter credits, OAuth subs, API-key
  per-token); Pi itself has no gateway. All three variants apply.
- **`allowed_env_keys` / contract `auth_env_vars`** — Pi has **no single API
  key**. The facts list four representative provider keys (`ANTHROPIC_API_KEY`,
  `ANTHROPIC_OAUTH_TOKEN`, `OPENAI_API_KEY`, `GEMINI_API_KEY`); a wrapper
  preserves whichever matches the selected provider. Pi is Rejected in contract
  v1, so the contract arm is informational.
- **`unmapped_native_events: []`** — `hooks/pi.md` lists 4 extension events with
  `claudine_event: unknown`, but Pi is a no-hook, stream-derived provider
  (Claudine never registers against its TS extension API), so there is no
  dispatchable native-hook surface to remediate — `[]` matches the qwen
  precedent. Flagged in case the 4 events are wanted listed.
- **Resume via `pi --session-id <id>`** — corrected from an initial
  `--session <full-id>` after a live multi-turn smoke (see below). `--session-id`
  is the unattended-safe selector: exact project id, appends to the same session
  file (true resume), never prompts; `--session`'s partial-UUID match risks a
  cross-project fork prompt.

## Live validation against pi 0.80.3 (research corrections)

After the initial from-research build, a real `pi` binary
(`@earendil-works/pi-coding-agent` 0.80.3) became available on the host, so the
wrapper and parser were smoke-tested end-to-end against it (a local `omlx`
model let the run complete offline). This **validated the parser** and
**corrected three wrapper assumptions the research could not reveal** — the
exact value of having a live binary:

- **Parser wire format confirmed.** The real `--mode json` stream matched
  `protocol/pi.rs` exactly: the `session` header, `agent_start`/`turn_start`/
  `message_start` lifecycle, `message_update` →
  `assistantMessageEvent.type: "thinking_delta"` with a `.delta` payload, and the
  `message.usage` envelope (`{input, output, cacheRead, cacheWrite, totalTokens,
  cost:{total}}`). A full `claudine pi "..."` run rendered thinking (coalesced
  BlockQuote), assistant text, and the metrics trailer (input/output tokens,
  duration) — the whole seam works live.
- **Fix 1 — prompt delivery is STDIN, not positional.** The real Pi parser
  **rejects `--`** (`Error: Unknown option: --`) *and* rejects any positional
  message beginning with `-` (`Error: Unknown option: - ...`). The
  Kilo-mirrored `AppendArgs(["--", prompt])` would have failed on every composed
  prompt. Pi reads the prompt from **stdin** in `-p` mode (verified: `echo
  PROMPT | pi -p --mode json` streams the run), so `prompt_delivery` now returns
  `PromptDelivery::Stdin`.
- **Fix 2 — the entrypoint flag is `-p`, not `--mode json`.** The facts wrongly
  placed `[--mode, json]` in the entrypoint `required_flags` (that is the stream
  *selector's* job, sourced from `output_formats`) and omitted `-p`. Without
  `-p`, a non-interactive run opens the TUI. Corrected: `entrypoints.required_flags:
  [-p]`; `--mode json` + the determinism companions come from the stream
  output-format as designed. The spawned argv is now `pi -p --no-approve
  --no-extensions --no-skills --no-prompt-templates --no-context-files --mode
  json` + stdin — verified against the real binary.
- **Fix 3 — system prompt needed a wrapper override + inline delivery.** The
  default `apply_system_prompt` is a stub that always reports "unsupported";
  providers that support it override it (the catalog spec only drives capability
  reporting). Pi's first run warned "does not support append system prompt" and
  skipped it. `PiWrapper` now overrides `apply_system_prompt` (delegating to the
  shared `apply_system_prompt_via_spec`, like Qwen), and the facts moved from
  `file_flag` to **`inline_flag`** (`--append-system-prompt`/`--system-prompt`
  accept inline text per the 0.80.3 help — unambiguous vs the text-or-path
  heuristic). Confirmed: the re-run delivers the prompt (input tokens rose
  2K→3K, no warning).
- **Fix 4 — resume selector is `--session-id`, not `--session`.** A live
  multi-turn smoke (turn 1 establishes a token in an isolated `--session-dir`,
  capture the session id, resume and recall) showed `--session-id <id>` is the
  correct unattended selector: it reused the exact session, **appended to the
  same session file** (2→4 messages = true resume, not a fork), **recalled the
  token**, and exited cleanly with no prompt. The initial `--session <full-id>`
  used the partial-UUID path whose cross-project match can raise a fork prompt.
  `build_resume_args` now emits `pi --session-id <id>`.
- **`--list-models` is a human table**, confirming `model_catalog_source: none`
  (follow-up #3 resolved in favor of `none`).

These are exactly the class of defect the research→codegen path *cannot* catch
(argv-parser quirks, stub-default overrides) and a live binary catches
immediately — a data point for the process retro: **the facts/wrapper layer
benefits from one real-binary smoke per provider before graduation is called
final.**

## Onboarding footprint (the mechanical checklist, as walked)

- **Cross-crate `sniff`:** added `AiCli::Pi` (binary `pi`) — variant +
  `PI_INSTALL` + `AI_CLI_INFO` row + `serde_key` arm. The `AiCli::COUNT ==
  AI_CLI_INFO.len()` guard passes.
- **Compiler-forced (`[T; PROVIDER_COUNT]` arrays):** `provider_id.rs` (variant
  `Pi = 8` + `PROVIDER_COUNT` 8→9 + display order + discriminant assert),
  `provider/registry.rs` (`&PI_INFO`), CLI `WRAPPER_REGISTRY` slot 8.
- **Manual:** `emit::PROVIDER_VARIANTS` (`("pi","Pi")` — the one gen-side edit),
  `provider/mod.rs` (`mod pi`), clap (`args.rs`/`main.rs` ×2/`argv`),
  `telemetry.rs` (×2), `stream::providers::for_provider` arm,
  `stream::protocol/mod.rs`, `adapters/mod.rs` (`mod pi` + `PI_ADAPTER`),
  `config/mod.rs` (`mod pi` + `PiConfigurator`).
- **New files:** `provider/pi/{mod,behavior,data(generated)}.rs`,
  `stream/protocol/pi.rs`, `stream/providers/pi.rs`, `adapters/pi.rs`,
  `config/pi.rs`, `cli .../wrap/profile/pi.rs`, `docs/providers/facts/pi.yaml`,
  `docs/providers/overrides/pi.yaml`.
- **Test updates:** `representative_payload_for` (Pi → `None`),
  `discover_agents_full` count 8→9 + Pi membership, contract `support_matrix`
  len 8→9 + Pi `Rejected` + Pi `auth_env_vars`, signals hub restructure +
  dormant-comment reconciliation, gen `provider_slugs_match_the_wired_set` list.
  Re-blessed `dispatch-inventory.json` (`CLAUDINE_UPDATE_INVENTORY=1`).

## Verification status

- **`claudine-gen check`:** all 9 providers + catalog/signals/families/roster
  **clean** (byte-identity held for the pre-existing 8).
- **`cargo check -p claudine -p claudine-cli -p claudine-contract
  --all-targets`:** clean.
- **`just test`:** **green** — lib, contract 47, cli 1892 (1 flaky in
  `argv_normalization::passthrough_version_flag_still_prints_version_string`,
  retry-passed — the known handle-leak area), gen 89. The 19 new Pi parser
  tests + 7 adapter + 5 configurator tests all pass.
- **`just lint`:** clean (clippy + error-transport guard) across all crates.
- **`sniff`:** clippy clean; `AiCli` count guard passes; the one `just`-run
  timeout (`filesystem::repo::area::detect_area_errors_when_not_in_repo`) is an
  unrelated repo-scan host flake — it passed in isolation (exit 0).
- **End-to-end smoke:** `claudine pi --help` is recognized; `claudine providers`
  renders the Pi row (skills ✅ / slash ✅ / **agents ❌** / **hooks 0**, docs
  link `https://pi.dev/docs/latest`), confirming the no-hook/no-subagent posture
  from `DisplayPolicy` + capability facts with **no per-provider render code**;
  `claudine hooks --support` shows the Pi column.
- **Live end-to-end against `pi` 0.80.3** (see the live-validation section): a
  real `claudine pi "..."` run streamed through `PiSemanticStreamParser` and
  rendered thinking + assistant text + the metrics trailer; the parser wire
  format matched the real binary; three wrapper fixes (stdin delivery, `-p`
  entrypoint, system-prompt override) landed and were re-verified. `just test`
  re-run green after the fixes (cli 1892, gen 89); inventory re-blessed (a
  `Provider::Pi` line-number shift from the two added imports).
- Known host flakes excepted per the phase brief.

## Recommended follow-ups (not blocking H2)

1. **Facts-skeleton should emit `acp` keys** (or a reminder line) — now
   confirmed on a from-scratch provider. Carried over from M-Kilo #1; promote to
   a real fix.
2. **Onboarding notes: aggregator `default_models` id-collision.** Add a note
   that multi-provider providers will hit the `expected_offerings` duplicate-id
   error and need a merge-not-drop reconciliation of the research (as done here).
3. **`pi --list-models` output shape — RESOLVED** to a human table (not JSON),
   so `model_catalog_source: none` stands. Revisit only if Pi adds a
   `--json`/machine-readable listing.
4. **Live smoke — DONE** for prompt delivery, entrypoint, system prompt, the
   parser, and `--session-id` resume (four fixes landed; see the live-validation
   section). No open behavior-half items remain from the live smoke.
5. **agent-models consumption-side work** (retire the recurring
   `model_catalog_source` overrides across providers) remains the standing
   Phase-I / pre-M-Antigravity item; M-Pi did not need a `model_cli_flag`
   override, which is early evidence the capture-side tightening is paying off.

---

**HOLD at ► CHECKPOINT H2 (Ken).** Second retro delivered; the process is
confirmed provider-shape-independent (a non-cousin graduated with a from-scratch
parser/adapter and no render-path change). Not starting M-Antigravity (H3, lands
after the closeout fleets) or Phase I.
