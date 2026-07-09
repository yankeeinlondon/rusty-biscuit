# Kickoff prompt — `more-struture` epic + provider-metadata follow-ups

> Paste this into a fresh session (or say "read
> `claudine/features/2026-07-06-more-struture/kickoff-prompt.md` and begin").
> Written 2026-07-08 at the close of the 2026-07-02 provider-metadata epic.

## Load first

- Use the **`claudine`** agent skill (load it before anything). Pull in `sniff`,
  `darkmatter`, `biscuit-file` as the task needs them.
- You are in the **claudine** package area of the rusty-biscuit monorepo.

## Where things stand (established context — do not re-litigate)

- The **2026-07-02 provider-metadata epic is fully closed and committed** —
  Phases A–I done. Generated `data.rs` is the only source of provider data
  (drift-tested); the legacy `AgentCapabilities` tree is gone; WrapperProfile
  static-fact overrides are zero; signals flow through compiled tables + one
  sink; the model-catalog boundary is live; render components carry
  `DisplayPolicy`; and the **unified `Provider`-dispatch drift guard**
  (`claudine-cli/tests/dispatch_inventory.rs`, covering lib+cli) holds the line
  (18 governed sites, all `keep`). The spec is stamped RATIFIED & IMPLEMENTED.
- **10 compiled providers** (`PROVIDER_COUNT = 10`): claude, codex, gemini,
  goose, kimi, opencode, qwen, kilo, pi, antigravity. Roster == enum.
- **Checkpoint H3 was APPROVED by Ken on 2026-07-08** (the Antigravity Goal-1
  acceptance test). The graduation report still says "HOLD at H3" — recording
  that approval is task B1 below.
- The sister **`2026-07-06-model-metadata`** spec is already IMPLEMENTED
  (Parsera → models.dev). Nothing to do there.

## Conventions (repo rules)

- Never run `cargo fmt`. Never commit unless explicitly told. Subagents never
  commit. Prefer US English.
- Any `catalog-types` / `ProviderInfo` change follows the **one-change gen
  discipline**: registry + `gen/src/emit.rs` + regen-all + `catalog.json` +
  both field-list guards, together. Never hand-edit a `provider/<slug>/data.rs`.
- Any catalog-types enum / signal-record change: catalog-types + sidecar +
  mirror test + fleet doc edits + regen + `claudine signals check` green, in one
  change per cluster. No parallel mapping mechanism.
- Tests (from the `claudine/` dir): `just test`, `just lint`, `just test-gen`,
  `just signals-check`. `cd` is shimmed to zoxide on this host — if not already
  in `claudine/`, invoke recipes as
  `just -f claudine/justfile -d claudine <recipe>` (or pass absolute paths).
- Re-bless artifacts after intentional changes: dispatch inventory
  (`CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`),
  skill `hash:` (`md hash --save <SKILL.md>`).
- Known host flakes (ignore ONLY on exact match): the 3
  `level2_tmux_*_chooser_detail`; `argv_normalization` handle-leak; sniff
  `detect_area_errors_when_not_in_repo`.

---

## Workstream A (primary) — the `more-struture` epic

Read `claudine/features/2026-07-06-more-struture/spec.md`. It is a DRAFT
"parking lot with a designed exit" that was gated on *"the 2026-07-02 spec fully
complete (Phases E–I closed, ladder validated)"* — **that gate is now met, so
it is unblocked.** First edit: lift its blocked/DRAFT banner (it is no longer
"do not implement").

**Goal:** review the structured research the fleets capture but the typed system
does not hold, and decide **item by item** whether to map it explicitly or keep
it research-side (with the *why* recorded). It graduates surplus into the typed
system through the established regime — it does **not** invent a new one.

**The spec's five inventory categories (verify against reality; the corpus has
moved since the spec was written — E6 harvest promotions, later rounds):**

1. **36 supplementary extraction fields with no `SignalEvent` slot** (the E4
   headline) — resolved at runtime, `debug!`-logged, `signals check`-asserted,
   but no variant field. Per field-family, decide: widen a variant (e.g.
   `TokensConsumed` gaining `cache_read`/`cache_write`/`cost`/`reasoning`), OR a
   generic supplementary payload map on `ObservedSignal`, OR stop resolving
   (delete the debug path). See `lib/src/signals/event_builder.rs`.
2. **Never-compiled record fields** (`vocabulary:`/`locator:`/`distinguish:`/
   `confidence:`/`notes:`) — stay research-side by design, EXCEPT closed
   `vocabulary` sets that bespoke code re-encodes as string needles (qwen
   `LoopType` ×10, goose `ProviderError` prefixes ×12, opencode five-branch
   classifier). Candidate: graduate closed vocabularies into catalog-types enums
   with sidecar mirrors so bespoke code + records share one source.
3. **Observed vocab values with no variant** — e.g. claude `rateLimitType:
   "usage"` → `UsageWindow::Unknown` today; either add a taxonomy member or
   document the mapping as intentional.
4. **`gaps:` prose that is really structured backlog** — a `gap` record class
   with a disposition (mirroring the summary-triage pattern): qwen exit
   53/55/130 (harvest-supplied), goose `retries_exhausted` (wire-invisible at
   65eed515, re-check on future versions), kimi `unsupported_protocol_version`,
   codex app-server richer vocab, gemini dynamic `stats.models.<model>` keys, pi
   `responseModel`, kilo step-finish presence check.
5. **Grammar/operator gaps** implied by the corpus — negation (kimi), field-to-
   field comparison (pi A≠B), dynamic key segments (gemini). Each is a
   `MatchOp`/path-grammar extension; only worth it when a second producer
   appears.

**Method (follow the spec's own §"Method (when activated)"):**

1. **Scripted audit, not memory** — regenerate the extraction-field × variant-
   slot list mechanically; sweep `vocabulary:`/`gaps:` across ALL live research
   topics (not just signals); diff against the spec's inventory.
2. **Classify each item** into: new variant field | new variant/enum member |
   new vocab enum | supplementary payload map | grammar extension | stays
   research-side (with *why*).
3. **► CHECKPOINT with Ken on the classification BEFORE any code.** This is a
   hard gate.
4. **Implement per cluster** through the established regime (see conventions);
   `signals check` green per change.

**Non-goals:** no new research fleets (consume existing accumulation only); no
runtime behavior before the classification checkpoint; harvest-promotion
mechanics stay owned by E6/provenance rules.

Recommended shape: produce the scripted audit + classification table first,
present it, get Ken's ruling, then implement cluster-by-cluster.

---

## Workstream B — small tracked follow-ups (independent; good warm-ups)

Do any/all; each is self-contained. Commit only when told.

- **B1. Record the H3 approval.** Update
  `features/2026-07-02-provider-metadata/m-antigravity-graduation.md` (HOLD →
  APPROVED, Ken, 2026-07-08) and the Phase H checkpoint note in
  `features/2026-07-02-provider-metadata/implementation-plan.md`.
- **B2. Skill event-support matrix.** `.claude/skills/claudine/architecture.md`'s
  Event Support Matrix has only 7 columns; add **Kilo, Pi, Antigravity** from
  the authoritative generated `lib/src/provider/<slug>/data.rs` `event_mapping`,
  then remove the "not yet columns" caveat blockquote I left under the legend.
  (architecture.md has no `hash:`; if you also touch SKILL.md, re-run
  `md hash --save`.)
- **B3. Antigravity graduation follow-ups** (from that report's "Recommended
  follow-ups"): promote the facts-skeleton `acp`-keys reminder to a real fix
  (confirmed across Kilo/Pi/Antigravity onboardings); consider forwarding a
  larger `--print-timeout` companion flag (agy's internal print timeout is 5m vs
  Claudine's 30m `step_timeout`); note the agy JSON-robustness watch-item.
- **B4. Standing cross-provider item** — retire the recurring
  `model_catalog_source` overrides (agent-models consumption side); Antigravity
  added one more `none` pin (human-table listing).
- **B5 (optional, larger).** Antigravity **app-log bespoke classifier.** `.txt`
  signal fixtures now load as opaque per-line payloads (commit `c6affd21a`), so
  the two `app_log` records (`app_log-provider_version-language-server`,
  `app_log-auth_invalid-not-logged-in`) in `docs/research/signals/antigravity.md`
  currently `[SKIP] (bespoke, pending emitter)`. Writing a glog-line classifier
  + registering a bespoke replayer (`signals.rs`) turns them into live
  detection. Gated by `requires_claudine_update: true`; needs the app-log to be
  an actual runtime source Claudine reads (verify before building — do not
  guess). This overlaps `more-struture` category 5, so consider sequencing it
  there.

## Success

- `more-struture`: audit + classification presented, Ken-approved, and the
  approved clusters implemented with `just test` / `just lint` / `just test-gen`
  green (only known host flakes excepted).
- Follow-ups: each landed with the suite green and the relevant artifact
  (skill hash, dispatch inventory, catalog) re-blessed.
