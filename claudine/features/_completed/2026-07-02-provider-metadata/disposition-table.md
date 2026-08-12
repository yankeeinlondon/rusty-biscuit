# WrapperProfile Disposition Table (Phase D, item 2)

> **Status:** CHECKPOINT D review artifact (2026-07-04). Classifies every live
> `WrapperProfile` override as **catalog-data** (migrates to a generated field;
> override deleted) or **behavior** (stays), per the OQ5 litmus test
> (design/catalog-generation.md): *data = pure selection among enumerable
> strategies plus string/scalar parameters, no runtime control flow; behavior =
> sequencing, conditionals over runtime state, or side effects.*
>
> **Provenance:** live enumeration of `cli/src/commands/wrap/profile/*.rs`
> (2026-07-04, post-Phase-C tree) joined with the committed mechanical inventory
> (`docs/providers/dispatch-inventory.json`, pattern-set v2: 433 sites, 23
> conditional). Supersedes design/pipeline-dry.md's "57 overrides" count and the
> working estimate of 67: the live count is **66 trait-method overrides**
> (claude 6, codex 11, gemini 15, goose 6, kimi 7, opencode 10, qwen 11).
> OpenCode's presumed 11th — `opencode_default_tui_noise_prefixes` — is a
> module-level helper fn, not a trait override; it is consumed by the
> `stderr_noise_prefixes` override and retires with it.
>
> **Resume ruling applied** (Ken, 2026-07-04): provider-native resume ⇒
> Claudine resume; only the support LEVEL graduates to catalog data
> (`resume: ResumeSupport`, wave-1 field). `build_resume_args` argv mechanics
> stay behavior on all 7 profiles.
>
> **EXECUTED (2026-07-05).** The ratified migrations shipped: 22 overrides +
> 2 helpers (`opencode_default_tui_noise_prefixes`, `push_stream_json_flags`)
> deleted; 6 trait defaults + the new derived `apply_structured_stream` read
> the catalog; the 3 `exec_prep` OpenCode conditionals read
> `model_required_in_non_tty` (inventory re-blessed: 431 sites / 20
> conditional, was 433/23). Remaining impls per provider (incl. the two
> mandatory methods): claude 4, codex 6, gemini 9, goose 5, kimi 4,
> opencode 9, qwen 8 — every remaining override is identity or ratified
> behavior; **static-fact overrides are at zero.** The tables below describe
> the PRE-migration roster (the checkpoint input) — line refs are historical.

## Classification key

| Class | Meaning |
| --- | --- |
| `catalog-data` | Pure static fact; migrates to a generated `ProviderInfo` field (wave-1 fields already landed); the override is deleted and the trait default derives from `provider_info()` |
| `catalog-candidate` | Data-shaped but the migration has a feasibility or semantics caveat — ruled individually below |
| `identity` | `provider()` — the required trait discriminator and catalog join key; not a fact, never migrates |
| `behavior` | Sequencing, argv/env mutation conditional on runtime state, parsing, or side effects; stays in the profile permanently |

## Summary counts

| Class | Overrides |
| --- | ---: |
| catalog-data (clean migration) | 17 |
| catalog-candidate (ruled below) | 8 |
| identity (`provider()`) | 7 |
| behavior (stays) | 34 |
| **Total** | **66** |

Post-migration profile size if all clean + candidate migrations are ratified:
66 → 41 overrides (identity 7 + behavior 34), with every remaining impl
genuinely behavioral — the design's success criterion.

## catalog-data — clean migrations (17)

Trait default changes to read `provider_info(self.provider()).<field>`; the
per-provider overrides below are deleted. All target fields are wave-1 catalog
fields generated from facts/research as of this checkpoint.

| Override | Providers (file:line) | Target catalog field |
| --- | --- | --- |
| `supports_resume` | claude claude.rs:79 · codex codex.rs:104 · gemini gemini.rs:160 · goose goose.rs:71 · kimi kimi.rs:89 · opencode opencode.rs:180 · qwen qwen.rs:110 (×7) | `resume: ResumeSupport` (research:resume `support`; `FirstClass`/`Partial` ⇒ `true`) |
| `allowed_env_keys` | codex codex.rs:91 · gemini gemini.rs:61 · kimi kimi.rs:16 · qwen qwen.rs:60 (×4) | `allowed_env_keys` (facts; hand-ruled security allowlist) |
| `stderr_noise_prefixes` | codex codex.rs:116 · gemini gemini.rs:102 · opencode opencode.rs:198 (×3) | `stderr_noise_prefixes` (facts; curated) |
| `stdout_noise_prefixes` | gemini gemini.rs:92 (×1) | `stdout_noise_prefixes` (facts; curated) |
| `suppress_structured_stderr_on_success` | gemini gemini.rs:106 (×1) | `suppress_structured_stderr_on_success` (facts) |
| `supports_interactive_inline_closure` | codex codex.rs:120 (×1) | `supports_interactive_inline_closure` (facts; Claudine-owned capability) |

Guard rails carried into migration:

- `every_provider_profile_supports_resume` stays green: all 7 research docs say
  `first_class`, so the derived default is `true` everywhere.
- OpenCode's `stderr_noise_prefixes` is simultaneously a promoted
  lifecycle-evidence channel (`--print-logs`) — the facts value is the same
  curated 5-prefix list the helper returned; the summary-triage NIS item about
  reconciling that dual role stays open independently of this migration.

## catalog-candidate — ruled individually (8)

**C1. `apply_structured_stream` ×6** — claude claude.rs:83, codex codex.rs:108,
gemini gemini.rs:164, kimi kimi.rs:93, opencode opencode.rs:184, qwen qwen.rs:114.
OQ7b ruled `structured_stream_flag` is *derived from `output_formats`* (the
Stream record's flag/selector), never a standalone field. The migration is a
generic default that reads the Stream-format record and pushes its flags:

- **Feasible now:** codex (`--json`), kimi (`--wire`), gemini/qwen
  (`--output-format stream-json`) — single selector flags the records carry.
- **Caveat — claude:** pushes companion flags `--print --verbose` beyond the
  format selector; the `OutputFormatSupport` record must express companions or
  claude keeps a thin behavior override.
- **Recommend behavior — opencode:** its "stream flags" (`--format json
  --print-logs --log-level INFO`) bundle log-promotion strategy (the stderr
  evidence channel), which is Claudine policy, not format selection.

*Recommendation:* migrate codex/kimi/gemini/qwen to the derived default;
claude migrates only if the Stream record grows a `companion_flags` slot
(cheap, honest); opencode stays behavior.

**C2. `apply_non_interactive_flags` ×2** — gemini gemini.rs:110, qwen qwen.rs:53.
Data-shaped (bail if a conflicting flag is present) and wave-1's
`non_interactive_conflicting_flags` carries adjacent facts — but the semantics
differ: the research field lists flags conflicting with *Claudine's wrapping
strategy* (includes e.g. `--output-format text`), while the override rejects
only *interactivity* flags (`-i`/`--prompt-interactive`). A mechanical swap
would widen the reject set — a behavior change nobody ruled.
*Recommendation:* stays **behavior** for now; revisit if the NIS schema grows a
typed `interactive_flags` key distinct from strategy conflicts.

## identity (7)

`provider()` — claude.rs:13, codex.rs:12, gemini.rs:15, goose.rs:11,
kimi.rs:12, opencode.rs:12, qwen.rs:14. The required discriminator that makes
every catalog-derived default possible. Permanent.

## behavior — stays (34)

| Override | Providers | Why it fails the litmus |
| --- | --- | --- |
| `prompt_delivery` (required) | all 7 | Delivery mechanics: stdin vs argv position vs wire RPC, size guards (opencode 768 KB bail), `-`-prefix handling. The *selection* enum graduates later (table-A `prompt_delivery`, design pending); impls stay |
| `apply_system_prompt` | all 7 | Tempfile writes, artifacts, env injection (`OPENCODE_CONFIG_CONTENT` merge), append/replace warnings |
| `build_resume_args` | all 7 | Resume argv mechanics (per the resume ruling); templates differ per provider (`-r`/`exec resume`/`--resume --session-id`/`--wire`) |
| `apply_yolo` / `apply_yolo_for_mode` / `reject_direct_yolo` | gemini ×2, qwen ×2, opencode ×2 (6 total) | Conflict detection against user argv, mode-dependent warnings; defaults are already catalog-driven (`YoloSupport`), these six are genuinely bespoke |
| `apply_entrypoint` | codex | Conditional `exec` insertion respecting user-provided entrypoint |
| `apply_sandbox` | codex, qwen | Push-if-absent over user argv; future `sandbox` catalog field informs but does not replace |
| `apply_model` | goose, opencode | Env/argv mutation with fallback ordering (goose env-only; opencode flag+env) |
| `prepare_captured_output` / `parse_captured_output` | gemini ×2 | argv conditioning + stream-json parsing |

(7+7+7+6+1+2+2+2 = 34.)

## Conditional dispatch sites addressable by wave-1 fields

From the 23 conditional inventory sites, these hardcoded provider checks become
catalog reads when the clean migrations land (they burn down the Phase I
allow-list, tag `ws3-profile`):

| Site | Check | Replacement |
| --- | --- | --- |
| `exec_prep/mod.rs:75/89/101` | `== / != Provider::OpenCode` | `provider_info(p).model_required_in_non_tty` |
| `exec_prep/mod.rs:156` | `!= Provider::Codex` | **not a wave-1 swap** — gates `prepare_codex_structured_output` (final-message capture for structured *or* inline-interactive runs). The underlying fact overlaps `supports_interactive_inline_closure` but the gate is capture *mechanics*; leave for Phase G/`FinalMessage` follow-up |

The remaining conditional sites (live_semantic_sink, policy.rs, repo_home.rs,
wrapper_stages.rs, inline.rs, composition) are stream/display/policy dispatch —
Phase G (EventRenderer/DisplayPolicy) and Phase I territory, not profile
migration.

## Checkpoint D questions — status

**RATIFIED (Ken, 2026-07-04) — step-3 migration unblocked:**

1. **The 17 clean catalog-data migrations** (table above) — defaults derive
   from `provider_info()`, overrides deleted. **Approved.**
2. **C1** — derived `apply_structured_stream` default for codex/kimi/gemini/qwen;
   claude via the `companion_flags` slot on the Stream `output_formats` record;
   opencode stays behavior. **Approved as split.**
3. **C2** — `apply_non_interactive_flags` (gemini, qwen) stays behavior
   (semantic mismatch with the research field). **Approved.**

**Ruled (Ken, 2026-07-04):**

4. **`platform_kind` values** — all three undecided providers (gemini/qwen/kimi)
   are `vendor_platform` (aggregator traits are escape hatches, not primary UX);
   field landed facts-fed same day.
5. **`sandbox` enum** — **deferred**: no catalog field until the permissions
   six-axis work (schema v2) provides a real consumer.
6. **`session_log_paths` grammar** — `{snake_case}` ratified; audit found the
   committed catalog already conformant (no migration).
7. **`stream_protocol` vocabulary** — normalize to framing vocab (`ndjson`/
   `jsonl`/`json-rpc`) and graduate from NIS, executed at the NIS-graduation
   moment (variant rename is a shape change).
