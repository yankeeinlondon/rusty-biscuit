# M-Antigravity Graduation Report (Phase H, milestone #3)

> Checkpoint **H3** artifact — the spec's **Goal-1 acceptance test**. Antigravity
> (Google's headless `agy` CLI) onboarded as the **10th** compiled `Provider`,
> the first genuinely-new provider taken through the entire
> research → codegen → behavior → live-smoke pipeline end to end. This is the
> process retro: what the pipeline got right, the first **buffered-JSON**
> behavior half, the live-binary findings, and the mechanical footprint.

## Outcome

`Provider::Antigravity` is live. Generation is byte-clean for all **10**
providers (`claudine-gen check` clean across data / catalog / signals / families
/ roster), the lib, CLI, and contract crates compile and test green, and
`claudine antigravity` wraps `agy` with a **bespoke single-envelope JSON**
parser. A real `claudine antigravity "…"` run against the installed **agy 1.1.0**
streamed through the new `AntigravitySemanticStreamParser`, rendered the
assistant text + the metrics trailer, reused the cached keyring/OAuth login (no
browser prompt), and exited clean.

**Goal-1 is met.** A provider that did not exist in the compiled set at the
start of the session — sourced only from the 17 research topic docs — is now a
fully wired, tested, live-smoked `Provider`, with **zero per-provider rendering
code** (the Phase-G thesis holds on a third, differently-shaped provider). The
end-to-end research → production pipeline is closed.

## Antigravity is a genuinely new shape (the first buffered-JSON provider)

Every previously-wired provider streams a line-delimited event protocol
(stream-json / NDJSON / JSONL / wire-JSON-RPC). agy's structured print mode is
different: `agy --print <prompt> --output-format json` emits **one buffered JSON
object** on stdout *after* the run completes —

```json
{"conversation_id":"…","status":"SUCCESS","response":"…",
 "duration_seconds":1.79,"num_turns":1,
 "usage":{"input_tokens":23791,"output_tokens":6,"thinking_tokens":0,"total_tokens":23797}}
```

— not a stream of deltas. This forced the one type-system extension of the
milestone: a new **`StreamProtocol::Json`** variant ("a single buffered JSON
document, not line-delimited"). Blast radius was small and contained: the enum
+ its serde round-trip test, the reporting label match, and the gen coercion
(`"json" → Json`). Authoring the honest wire shape rather than mislabeling agy
as `stream-json`/`ndjson` follows the M-Pi philosophy (build the real format).

Behavior half authored from scratch (nothing reused):

- **`stream/protocol/antigravity.rs`** — `AntigravityEnvelope` + `AntigravityUsage`,
  the typed single-object model.
- **`stream/providers/antigravity.rs`** — `AntigravitySemanticStreamParser`, a
  from-scratch `SemanticStreamParser` that **accumulates** every fed line into a
  buffer and parses it as one object as soon as it is valid JSON (handles both
  the normal compact single line and a pretty-printed split). It emits
  `OutputText` (the `response`), a terminal `TurnComplete` (usage/duration/
  status), and — on a non-`SUCCESS` status or a top-level `error` — a terminal
  text-classified `Error`. `conversation_id` is captured into the summary's
  `session_id` for resume. A run that produces no parseable envelope (e.g. a
  plain-text auth failure) is recorded as a classified error in `finish`. 6 unit
  tests cover success, pretty-print, error-status, non-JSON stdout, empty
  stdout, and the lenient-reparse (below).
- **`adapters/antigravity.rs`** — `AntigravityAdapter`, a real hook-payload
  parser for `claudine handle`. agy hook payloads are camelCase JSON with **no**
  event-name field, so `parse_event` infers the Claudine event from the field
  shape (`toolCall` → before_tool, `fullyIdle`/`terminationReason` →
  turn_complete, `invocationNum` → before_model, bare `stepIdx` → after_tool)
  and `format_response` emits agy's stdout decision JSON (`{decision: allow|deny|
  ask}` for PreToolUse, `{decision: continue}` for Stop, `{terminationBehavior}`
  for the invocation events). `detect_from_payload → false` /
  `representative_payload_for → None` (agy's payloads share common fields and
  are not uniquely shape-detectable; the hook path passes the provider
  explicitly). Documented limitation: PreInvocation and PostInvocation have an
  identical payload shape and cannot be distinguished from the payload alone —
  the ambiguous case maps to `before_model`.
- **`config/antigravity.rs`** — `AntigravityConfigurator`, a **real** configurator
  (`hooks_supported() → true`). It registers Claudine's command handlers into
  the global `~/.gemini/config/hooks.json` (the path agy loads in print mode)
  under a single `"claudine"` named-hook key, using agy's schema:
  `PreToolUse`/`PostToolUse` as matcher-groups (`[{matcher:"*", hooks:[…]}]`),
  and `PreInvocation`/`PostInvocation`/`Stop` as direct handler arrays; each
  handler is `{type: command, command: "claudine handle <event> --provider
  antigravity", timeout: 30}`. Whole-key replace preserves other named hooks.
  This satisfies the hook-support invariant (Hook-level `event_mapping` ⟺ real
  configurator) and puts Antigravity in `init --quick` (it registers a `Stop`
  hook for the recommended `turn_complete` event).
- **`cli .../wrap/profile/antigravity.rs`** — `AntigravityWrapper`. The only
  overrides over the catalog defaults: `prompt_delivery` (append `--print
  <prompt>` LAST so the value stays adjacent to its flag for Go's parser),
  `apply_structured_stream` (push `--output-format json`, since the catalog
  records the format as `OutputFormat::Json`, not the `Stream` record the
  default keys on), `build_resume_args` (`agy --conversation <id>`),
  `apply_sandbox` (`--sandbox`), and `apply_model` (`--model` dedup + `MODEL`
  env). Model selection, YOLO, and reject-direct-yolo are catalog-driven.

**Hooks — wired (corrected mid-graduation).** The first pass wrongly treated agy
as no-hook ("unverified"). On review (Ken, 2026-07-08) a live probe proved the
hook subsystem **loads and parses `~/.gemini/config/hooks.json` during a
`--print` run** (`hooks_manager.go` parsed the config live), and the binary's
embedded docs gave the authoritative schema. So agy is wired as a real hook
provider — five `Hook`-level events (PreToolUse→before_tool,
PostToolUse→after_tool, PreInvocation→before_model, PostInvocation→after_model,
Stop→turn_complete), a real configurator, and a real payload adapter — while the
`--output-format json` envelope remains the wrapper-mode rendering source.
**One caveat, Ken-approved (wire-now, verify-later):** hook LOADING in print mode
is confirmed, but a hook handler's command was **not yet observed EXECUTING** in
one-shot `--print` (the probing destabilized the local agy backend before a
clean firing could be captured). Wired now on the strength of the research +
loaded-in-print evidence; a single clean firing check on a stable agy is the
outstanding confirmation (facts `known_gaps` + follow-up #7). *(Correction to my
earlier claim: the research doc's hook schema was actually right — the
named-hook wrapper is documented; my first probe misread it by putting the event
at the top level.)*

**Other surfaces still unwired (documented gaps):** MCP (shared config files but
no safe runtime injection — a shadow HOME would break keyring auth), subagents
(`/agents` is interactive-only, no `--agent` selector), and slash commands
(skill-derived, no separate dir, no arg placeholders). Skills are the one
first-class linkable resource. All are recorded in the facts `known_gaps` with
trackers.

## Scaffold + generate UX

- **Two-pass scaffold worked cleanly** (third real exercise): pass 1 wrote the
  21-field TODO facts skeleton and stopped; after the facts were filled, pass 2
  wrote the `mod.rs`/`behavior.rs` stubs (never overwriting) and generated
  `data.rs`. The behavior stub compiled as-is.
- **Confirmed follow-up (M-Kilo #1 / M-Pi #1): the skeleton still omits `acp`.**
  As on Pi, the `acp:` sub-record had to be hand-added. Third confirmation on a
  from-scratch provider — promote to a real fix.
- **`model_catalog_source` override required** (the documented dynamic-listing
  hard-stop): `agy models` *is* a shell command but prints a **human text list**
  of display labels (verified against agy 1.1.0), not machine-parseable JSON,
  and requires an authenticated session. Honest source is `none`
  (`overrides/antigravity.yaml`), mirroring the pi/codex/kimi precedent;
  `expected_offerings` remains the validation baseline.
- **`model_cli_flag` needed NO override** — `--model` is a single bare-token
  flag, so the coercion yields it directly (same win as Pi).
- **Loud, harmless coercion skips** (all correct): `config_paths` skipped two
  templated entries (`~/.gemini/config/projects/<project-id>.json`,
  `<repo>/.agents/`); `non_interactive_conflicting_flags` skipped two prose
  entries; `expected_offerings` joined 1/9 to the models-catalog artifact (the 8
  Antigravity display-label model ids — "Gemini 3.5 Flash (Medium)", "Claude
  Sonnet 4.6 (Thinking)", "GPT-OSS 120B (Medium)", … — are Google-gateway labels
  with no unchained-ai family entry; informational, no drift).
- **No `expected_offerings` duplicate-id collision.** Unlike Pi (a BYO-key
  aggregator that listed the same id for several providers), Antigravity's model
  list is Google's own gateway catalog with distinct display labels, so the
  aggregator merge-not-drop reconciliation was not needed. Recorded
  `platform_kind: vendor_platform` (Google bills and serves every model —
  Gemini, Claude, GPT-OSS — through one authenticated backend; the user brings
  no keys), distinct from Pi/Kilo's `agent_aggregator`.

## Live-binary findings against agy 1.1.0 (what codegen could not reveal)

The four wrapper facts the research cannot settle, all resolved against the real
binary:

1. **`--output-format json` exists and is the right structured selector** — the
   research docs disagreed (agent-cli/resume said a hidden JSON envelope flag
   existed; non-interactive-sessions said print mode was plain-text only). The
   binary settled it: `agy --help` hides it, but the binary strings carry
   `--json-schema can only be used when --output-format is 'json'`, and a real
   `agy --print "…" --output-format json` returned the envelope above. The
   non-interactive doc under-reported it.
2. **Prompt delivery is the `--print` flag value** (argv, not stdin — agy
   ignores stdin). `AppendArgs(["--print", prompt])` appended last keeps the
   value adjacent. Verified with single-line and multi-line prompts.
3. **System-prompt override is genuinely unsupported** — agy 1.1.0 exposes no
   `--append/replace-system-prompt` flag (append is only possible via workspace
   rule files, which is not a flag Claudine can drive today). So the facts leave
   both modes `Unsupported` and the default `apply_system_prompt` stub is left
   in place; the wrapper correctly warns "does not support append system prompt;
   this flag was skipped" and continues. Honest, not a defect.
4. **Resume selector is `agy --conversation <id>`** — a live two-turn smoke
   (store a codeword, resume by the envelope's `conversation_id`, recall it)
   confirmed it appends to the same conversation and recalled the codeword. The
   `-c`/`--continue` alternative selects the latest conversation and is unsafe
   for parallel wrappers.

Plus two findings only a live run surfaces:

- **HOME/keyring is preserved by construction.** agy authenticates via the OS
  keyring + Google Sign-In OAuth with **no headless API-key mode** (no auth env
  var honored → `allowed_env_keys: []`, contract `auth_env_vars → &[]`). A
  wrapped run must reuse the cached login. Claudine's env sanitization only
  rewrites HOME under a **shadow HOME**, and `needs_shadow_home` is
  `repo_only || (Codex && …)` — false for Antigravity by default — so HOME (and
  the Keychain session) is inherited untouched. The live run confirmed: no OAuth
  browser prompt, cached login reused. No wrapper override was needed.
- **agy occasionally emits an unescaped control character** inside the
  `response` string (observed once on an off-rails Gemini-Flash response). Strict
  `serde_json` rejects that, which would surface the whole run as a parse error.
  The parser now does a **lenient reparse**: strict first (the normal case —
  agy emits compact, correctly-escaped JSON), and only on failure re-escapes
  bare control chars and retries. Since agy's output is one compact line with no
  structural whitespace, this cannot corrupt a genuinely-valid document. Covered
  by a unit test.

## Onboarding footprint (the mechanical checklist, as walked)

- **Cross-crate `sniff`** (vendored `sniff/lib` inside the claudine worktree):
  `AiCli::Antigravity` (binary `agy`, `alternate_binary_names: &[]` — the GUI
  launcher `antigravity` is a *different* binary and must NOT be an alternate) +
  `ANTIGRAVITY_INSTALL` + `AI_CLI_INFO` row + `serde_key` arm. The `AiCli::COUNT
  == AI_CLI_INFO.len()` guard passes.
- **Type-system extension:** `StreamProtocol::Json` (lib) + reporting label + gen
  coercion.
- **Compiler-forced (`[T; PROVIDER_COUNT]` arrays):** `provider_id.rs`
  (`Antigravity = 9` + `PROVIDER_COUNT` 9→10 + display order + discriminant
  assert), `provider/registry.rs` (`&ANTIGRAVITY_INFO`), CLI `WRAPPER_REGISTRY`
  slot 9.
- **Manual (gen-side + wiring):** `emit::PROVIDER_VARIANTS` (`("antigravity",
  "Antigravity")`), `signals::SIGNAL_SLUGS` (`"antigravity"` — the signals
  builder iterates its own slug list, easy to miss: without it the generated
  signals table is silently omitted despite 4 researched records), `provider/mod`,
  `adapters/mod`, `config/mod`, `stream/protocol/mod`, `stream/providers/mod`
  (+ `for_provider` arm), clap `args.rs`/`main.rs` (×2)/`argv::WRAPPER_SUBCOMMANDS`,
  `telemetry.rs` (×2), `cli .../profile/mod.rs` (mod + use + static + registry).
- **New files:** `provider/antigravity/{mod,behavior,data(generated)}.rs`,
  `stream/protocol/antigravity.rs`, `stream/providers/antigravity.rs`,
  `adapters/antigravity.rs`, `config/antigravity.rs`,
  `cli .../wrap/profile/antigravity.rs`, `docs/providers/facts/antigravity.yaml`,
  `docs/providers/overrides/antigravity.yaml`.
- **Roster correction:** `user_dir`/`repo_dir` were provisional and **wrong** —
  corrected `user_dir → ~/.gemini/antigravity-cli` and `repo_dir → .agents` (the
  provisional `~/.antigravity` / `.antigravity` is the separate desktop-IDE
  app-shell dir, not the CLI's state). `docs_url` left as the verified product
  page.
- **Test updates:** `representative_payload_for` (Antigravity → `None`),
  `discover_agents_full` count 9→10 + membership, contract `support_matrix` len
  9→10 + Antigravity `Rejected` + `auth_env_vars` (`&[]`), gen
  `provider_slugs_match_the_wired_set_in_order` (+antigravity) and
  `provider_variant_rejects_unknown_slug` (swapped its example `"antigravity"` →
  `"nonesuch"` — it is now wired). Re-blessed `dispatch-inventory.json`
  (`CLAUDINE_UPDATE_INVENTORY=1`, no drift). The signals dormant test needed
  **no** change (M-Pi's synthetic-slug rewrite made it provider-independent).
- **Hook wiring (second pass, after the no-hook correction):** facts
  `event_mapping` flipped 5 events to `Hook`-level (`registration_target: true`);
  `behavior.rs` `hooks_supported() → true`; `config/antigravity.rs` rewritten
  from a no-op stub to a real `hooks.json` configurator; `adapters/antigravity.rs`
  rewritten from an envelope stub to a real hook-payload parser;
  `init_defaults.rs` quick-start test updated (+Antigravity, via its `Stop`
  hook for `turn_complete`). Regenerated `data.rs`/`catalog.json` (clean).

## Verification status

- **`claudine-gen check`:** all 10 providers + catalog / signals / families
  **clean** (byte-identity held for the pre-existing 9); roster reports every
  active entry wired.
- **`just test` (repo-root, all curated packages):** the full Antigravity set is
  green — `claudine`, `claudine-catalog-types`, `claudine-cli`,
  `claudine-contract`, `claudine-gen` all "all tests passing". The 15 new
  Antigravity lib tests (parser 6 + adapter 5 + configurator 4) pass. sniff
  recompiled clean.
- **`just lint`:** clean — clippy across `claudine` / `claudine-cli` /
  `claudine-gen` / `sniff` finished with no warnings; the error-transport and
  lifecycle-doc-facets guards pass.
- **Live end-to-end against agy 1.1.0:** `claudine antigravity "…"` streamed
  through `AntigravitySemanticStreamParser` and rendered the response + metrics
  trailer; single-line, multi-line, and two-turn resume smokes all succeeded;
  the cached keyring login was reused with no browser prompt. `claudine
  providers` renders the Antigravity row (skills ✅ / slash ❌ / agents ❌ /
  **hooks 5**) and `claudine hooks --support` shows the Antigravity column with
  BeforeTool/AfterTool/BeforeModel/AfterModel/TurnComplete ✅ — both from
  `DisplayPolicy` + capability facts with **no per-provider render code**.
- **Hook subsystem (loaded, not yet fired):** a live probe confirmed agy
  parses `~/.gemini/config/hooks.json` during a `--print` run; a hook handler's
  command was **not** observed executing in one-shot print before the local agy
  backend was destabilized by the probing. Wired on that evidence per the
  wire-now/verify-later decision; the clean firing check is follow-up #1.

## Recommended follow-ups (not blocking H3)

1. **Confirm print-mode hook EXECUTION (the wire-now/verify-later item).** The
   hook subsystem loads in `--print`, but a handler firing was not cleanly
   observed. On a stable agy (restart it first — repeated force-kills poison its
   local backend), run ONE clean probe: a `PreInvocation` or `Stop` command hook
   in `~/.gemini/config/hooks.json` under a named key + a plain `agy --print`
   from an approved workspace. If it fires, done. If it does not, print mode
   skips hooks like `claude --bare` — revert the 5 events to `not_supported` and
   the configurator to no-op.
2. **Facts skeleton should emit `acp` keys** (or a reminder line) — now confirmed
   on three from-scratch onboardings (Kilo/Pi/Antigravity). Promote to a fix.
2. **System-prompt via workspace rule files.** agy has no system-prompt flag but
   reads `AGENTS.md` / `.agents/rules/*.md`; a future `apply_system_prompt`
   override could write a temp rule file into the workspace + `--add-dir` it.
   Deferred (needs workspace-lifecycle care); v1 honestly reports unsupported.
3. **agy JSON robustness.** The lenient reparse handles the observed unescaped
   control char; if agy proves to emit invalid JSON regularly, consider a proper
   streaming-tolerant decode. Watch-item.
4. **MCP / subagents / slash commands.** Deferred by design (interactive/IDE
   surfaces, no safe headless injection). Revisit if agy gains a headless MCP
   subcommand or an `--agent` selector.
5. **`--print-timeout` companion.** agy's internal print timeout defaults to 5m;
   Claudine's `step_timeout` default is 30m. Long runs could hit agy's cap
   first. Consider forwarding a larger `--print-timeout` as a companion flag.
6. **agent-models consumption-side work** (retire the recurring
   `model_catalog_source` overrides) — the standing cross-provider Phase-I item;
   Antigravity added one more `none` pin (human-table listing).

---

**► CHECKPOINT H3 — APPROVED (Ken, 2026-07-08).** Goal-1 acceptance test
delivered and accepted: a genuinely new provider (the first buffered-JSON shape)
graduated end to end through the research → codegen → behavior → live-smoke
pipeline, with a from-scratch parser/adapter/configurator and zero render-path
change. Phase H closed; the recommended follow-ups above are tracked separately
(see the `more-struture` kickoff Workstream B).
