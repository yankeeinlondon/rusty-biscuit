# Vocabulary Delta Report — research projection vs. Phase-A seed baselines

Increment **C1** of spec [`2026-07-11-provider-errors-as-data`](../../../features/2026-07-11-provider-errors-as-data/spec.md)
(Phase 7 of the [execution plan](../../../features/2026-07-11-provider-errors-as-data/plan.md)).
This is the consolidated, order-aware diff of the researched `agent-errors`
vocabulary against the immutable Phase-A seed baseline, every difference classified under the
D8 reconciliation rules and the mandatory adjudication checkpoint. Iteration 1
accepted the initial overload delta, and iteration 2 accepted the exact capacity
phrase; Phase 8 graduated both with parser tests.

## How the delta was computed

For each of the ten compiled providers, this report compares two projections of
the same shape, **branch by branch and in sequence order**:

- **Seed projection** — the ordered direct `ErrorVocabulary` in
  `docs/research/agent-errors/_seeds/<slug>.yaml` (the Phase-A runtime ground truth,
  transcribed verbatim from the parser constants).
- **Research projection** — the ordered `needles[].text` / `codes[].code` in
  `docs/research/agent-errors/<slug>.md` frontmatter (validated against
  `_schema.yaml` and clean under the deterministic gate,
  `claudine-gen agent-errors check <slug>`, per Phase 6).

Order matters because **bucket sequence order is the behavior contract**
(spec invariant #1: first substring hit wins). A row is a delta only when the
research projection **adds, removes, re-kinds, or re-orders** a needle/code
relative to the seed. Provenance (`evidence:`) rides only on the research layer
and is dropped at graduation, so an `evidence: seed` needle that reappears in
its seeded position is **not** a delta — it is a preserved seed.

The `gaps` entries are **not** deltas: they are researched-but-not-graduated
surfaces (remaining capacity phrasings without a pinned citation, numeric-HTTP
substrings unsafe in the cascade, detection-owned overload families). They are carried
forward as explicit non-executable items for Ken's awareness, not as proposed
behavior.

## Completeness accounting (every seed, every researched row)

Every seeded needle/code is accounted for, and every researched row resolves to
exactly one disposition (preserved seed **or** a classified delta). There are
**no unclassified diffs**.

| Provider | Seed rows (kind/msg/code) | Research rows | Seeds preserved | Deltas | Runtime status |
|---|---|---|---|---|---|
| claude | 14 / 13 / 0 | 27 | 27 / 27 | 0 | parser-backed |
| codex | 13 / 12 / 0 | 27 | 25 / 25 | **2** (msg adds `overloaded`, `selected model is at capacity`) | parser-backed |
| gemini | 13 / 11 / 0 | 24 | 24 / 24 | 0 | parser-backed |
| goose | 0 / 0 / 0 | 0 | n/a | 0 | **research-only (empty at runtime)** |
| kimi | 0 / 14 / 7 | 21 | 21 / 21 | 0 | parser-backed (msg + code) |
| opencode | 14 / 15 / 0 | 29 | 29 / 29 | 0 | parser-backed |
| kilo | 14 / 15 / 0 | 29 | 29 / 29 | 0 | parser-backed (shared OpenCode parser, `Kilo` identity) |
| pi | 0 / 18 / 0 | 18 | 18 / 18 | 0 | parser-backed (msg-only) |
| qwen | 12 / 11 / 0 | 23 | 23 / 23 | 0 | parser-backed |
| antigravity | 0 / 18 / 0 | 18 | 18 / 18 | 0 | parser-backed (msg-only) |
| **Total** | **214 rows** | **216 rows** | **214 / 214** | **2** | — |

**214 seeds in, 214 seeds preserved (100%), 2 evidence-backed additions, 0
removals, 0 re-kinds, 0 reorderings, 0 duplicates.** The two extra research
rows are the Codex capacity additions classified below.

## The delta (D8 classification)

### Δ1 — codex · `msg_buckets` · api_remote · **evidence-backed addition** (R2 / R2a)

The iteration-1 needle-level delta.

| Field | Value |
|---|---|
| Provider | `codex` |
| Branch | `msg_buckets` |
| Bucket | first `api_remote` bucket (`rate limit`, `quota`, `billing`, `api error`) |
| Change | **append** `overloaded` after the existing seeds |
| Evidence | `documented` |
| Source | <https://platform.openai.com/docs/guides/error-codes> — OpenAI documents the HTTP 503 `overloaded` response; Codex passes API errors through into its `error` / `turn.failed` message prose. |
| D8 class | **evidence-backed addition** (R2). Not a sticky-seed conflict, not a re-kind, not a reorder. |
| Ordering (R3) | **append-within-branch** — added at the *end* of the existing bucket, so all seeded needles (`rate limit`/`quota`/`billing`/`api error`) still match first. No mid-cascade insertion, no reordering. Precedence of every existing needle is unchanged. |
| Shadowing | `overloaded` is a narrow, unambiguous capacity marker; it is neither a substring of, nor shadowed by, any earlier codex needle in any branch. In the msg cascade the `api_remote` bucket is checked first, so `overloaded` classifies to `ApiRemote` as intended. |
| Motivation | Closes the **motivating-incident class** — Codex's observed "Selected model is at capacity" / 503 overload matched no seeded Codex needle (spec §Motivation). |

**Proposed tests (R2a — required before Phase 8 implements this):** target module
`lib/src/stream/providers/codex.rs` `mod tests`, exercising the private
`classify_error(error_kind, message)` helper (the same seam the existing
`classify_error_*` tests use). Expected winning kind: `SemanticErrorKind::ApiRemote`.

- **Positive fixture** — `classify_error_overloaded_message_maps_to_api_remote`:
  `classify_error(None, Some("the selected model is overloaded, retry"))`
  ⇒ `SemanticErrorKind::ApiRemote`.
- **Collision/precedence fixture** — `classify_error_overloaded_does_not_disturb_seed_precedence`:
  assert the append leaves earlier seeds winning and does not misroute
  configuration/interrupted prose — e.g.
  `classify_error(None, Some("rate limit reached"))` still `ApiRemote` (seed wins
  first), and a benign non-error message without the substring is unaffected.
  The fixture asserts the **winning bucket**, not mere presence of the needle in
  generated output.

**Disposition:** **ACCEPTED** for iteration 1. Graduated with the named positive
and precedence/collision fixtures.

### Δ2 — codex · `msg_buckets` · api_remote · **evidence-backed addition** (R2 / R2a)

| Field | Value |
|---|---|
| Provider | `codex` |
| Branch | `msg_buckets` |
| Bucket | first `api_remote` bucket, after the accepted `overloaded` addition |
| Change | **append** `selected model is at capacity` |
| Evidence | `issue_tracker` |
| Source | <https://github.com/openai/codex/issues/17014> — exact terminal error observed by the reporter on Codex CLI 0.118.0 on 2026-04-07. The reporter interprets it as a transient model-capacity/admission problem rather than quota exhaustion; the issue contains no official provider confirmation of that interpretation. |
| D8 class | **evidence-backed addition** (R2). Not a sticky-seed conflict, not a re-kind, not a reorder. |
| Ordering (R3) | **append-within-branch** — the new phrase follows all seeds and the accepted `overloaded` addition, preserving all existing precedence. |
| Discrimination (R2a) | The full `selected model is at capacity` clause avoids broad `capacity` / `at capacity` matches. A parser-level positive fixture uses the current message-only `turn.failed.error.message` shape; a collision fixture keeps ordinary capacity-planning prose `AgentNative`. |
| Motivation | Classifies the exact production incident that motivated the feature as `SemanticErrorKind::ApiRemote` when Codex supplies no structured error kind. |

**Disposition:** **ACCEPTED** for iteration 2. The source-pinned narrow phrase
and parser-level positive/collision fixtures close review-2 finding 2.

## D8 class tally (whole roster)

| D8 class | Count | Detail |
|---|---|---|
| Sticky-seed conflict (R1 — removal / re-kind of a seed) | **0** | No research doc removes or re-kinds any seed; the deterministic gate's seed-preservation check enforced this for all ten docs (Phase 6, 0 findings). |
| Evidence-backed addition (R2 / R2a) | **2** | Δ1 Codex `overloaded`; Δ2 Codex `selected model is at capacity`. |
| Ordering / insertion change (R3) | **0** | Δ1 and Δ2 are appends (default honored); no mid-cascade insertion or reorder anywhere. |
| Kind reassignment (R4) | **0** | No existing needle changes its bucket kind. |
| Exact duplicate needle within a branch | **0** | No branch carries a repeated identical needle. |
| Prefix / substring shadowing within a branch | **0 new** | See the advisory seed-shadowing note below — all shadowing is pre-existing seed behavior, not introduced by research. |

## R1 sticky-seed enforcement

Per D8/R1, seeds are sticky: **research may not silently remove or re-kind a
needle carried from the current runtime tables.** This report confirms
mechanically that all 214 seeded rows reappear in their seeded branch and kind.
No claimed correction, removal, or re-kind is proposed by any research document;
should one ever be, R1 routes it to a **separate reproducing fix**, not this
graduation. Nothing here triggers that path.

## R3 ordering discipline

The default is to **append** new buckets/needles after existing ones of the same
branch. Both accepted deltas (Δ1 and Δ2) obey this exactly. No document requests a
mid-cascade insertion or a bucket reordering; had one, this report would flag it
as an explicit behavior change requiring standalone justification. None applies.

## R5 cross-provider consistency (advisory only)

Consistency is assessed **advisory**, never forced — the vocabulary is
per-provider by design and each provider is adjudicated from its own evidence.

- **`opencode` ≡ `kilo` — byte-identical (both branches).** Justified: Kilo
  reuses the OpenCode wire parser under a fixed `Kilo` identity, and its Phase-A
  seed was transcribed as an ordered copy of OpenCode's table. Distinct provider
  records by design (so Kilo can diverge later); see the `kilo-native-error-strings`
  gap. **Not homogenized, not a delta.**
- **`gemini` ≈ `qwen` — near-identical (fork lineage).** Qwen Code is a Gemini
  CLI fork; the two share table lineage and differ only in Gemini's extra
  `denied` needle in the configuration kind bucket. Each is assessed from its own
  evidence. **Not a copy-paste defect, not a delta.**

No provider vocabulary is changed to match another.

## Advisory: pre-existing intra-branch shadowing (D3 disclosure, not a delta)

Spec D3 requires the C1 report to surface exact-duplicate and prefix/substring
shadowing within an input branch. The following are **seed** behaviors preserved
verbatim from Phase A — disclosed for Ken's awareness, **not** proposed changes:

- **antigravity · msg · configuration** — `authentication failed` precedes the
  broader `authentication`; a string with "authentication" but not "failed"
  falls through to `authentication` (both classify `configuration`, so the
  outcome is unchanged — the shadow is benign). The bucket also carries the
  numeric substrings `"401"` / `"403"`.
- **kimi · msg · configuration** — `authentication` precedes the broader `auth`
  (both `configuration`; benign shadow).
- **Broad seeds** — `api` / `model` / `provider` (kind branches of
  codex/gemini/opencode/kilo/qwen) and bare HTTP numbers (`"503"` in
  pi/antigravity, `"401"`/`"403"` in antigravity) are broad substrings whose
  precedence is already fixed by Phase A. Research uniformly **declines** to add
  new bare-numeric substrings (recorded as the `numeric-http-codes` gap) — they
  belong behind an exact-match / `code_buckets` surface, not the substring
  cascade.

None of these is graduated, removed, or reordered by this report.

## Gaps carried forward (non-executable; Ken awareness)

Every researched-but-not-graduated surface, recorded as an explicit `gaps` entry
rather than fabricated into a needle. These change **no** runtime behavior and
are candidates for a *future* live research run or a separate exact-match
surface, **not** for Phase 8 unless Ken explicitly accepts one.

| Gap | Providers | Why not graduated |
|---|---|---|
| `capacity-overload-phrasing` | gemini, qwen, opencode, kilo, kimi, goose | Exact CLI-rendered capacity string not source-pinnable in a non-interactive run, **or** the overload family is a `signals/` **detection** concern (opencode/kilo/kimi), not rendering vocabulary. |
| `numeric-http-codes` | claude, codex, gemini, qwen, opencode | Bare HTTP status substrings collide with token counts/IDs in the case-insensitive cascade; need an exact-match / `code_buckets` channel. |
| `structured-error-kind-discriminator` | codex, pi, antigravity | No stable machine error-kind enum on the stream; classification leans on the message branch. |
| `kilo-native-error-strings` | kilo | Whether `PROMOTION_MODEL_LIMIT_REACHED` / `PAID_MODEL_AUTH_REQUIRED` / gateway-402 should diverge from the shared OpenCode seed is a deliberate Phase-7 decision (currently detection records in `signals/kilo.md`). |
| `no-stream-parser` | goose | Research-only; no parser, no Phase-A seed baseline; explicitly empty at runtime until a parser lands. |
| Antigravity has no published source at tag 1.1.0 | antigravity | Auth/capacity surfaces are empirical from the installed `agy` binary; firmer citations await Google publishing source. |

## Empty / runtime-inactive providers

- **goose** — the one parser-less provider: no Phase-A seed baseline, empty research
  buckets, explicitly empty at runtime (spec acceptance criterion). Its typed
  `ProviderError` catalog is documented in `goose.md` as a research-only starting
  point should a parser ever land. **No delta; runtime table stays empty.**

## Ken adjudication checkpoint (◆ C1) — ACCEPTED

The iteration-1 implementation request authorized the initial Codex overload
delta, and review iteration 2 authorized the exact capacity follow-up. The
mechanical prerequisites were green:

- Every seed (214/214) is preserved; every researched row is classified; **no
  unclassified diff** (completeness table above).
- Exactly **two** behavior deltas (Δ1 Codex `overloaded`; Δ2 Codex
  `selected model is at capacity`), both append-within-branch evidence-backed
  additions with stable citations and tests asserting the winning
  `SemanticErrorKind`.
- Zero sticky-seed conflicts, re-kinds, reorderings, or duplicates across the
  roster.
- All ten documents validate against `_schema.yaml` and are clean under the
  deterministic gate (`claudine-gen agent-errors check`, Phase 6).
- Phase 8 projects research into runtime vocabulary and covers the accepted
  classification behavior with Level 1 parser tests.

**Disposition ledger:**

| # | Delta | Proposed | Disposition | Notes |
|---|---|---|---|---|
| Δ1 | codex `msg` api_remote += `overloaded` (documented) | R2/R2a addition, append | **accepted** | Iteration 1; positive + precedence/collision tests required and landed. |
| Δ2 | codex `msg` api_remote += `selected model is at capacity` (issue tracker) | R2/R2a addition, append | **accepted** | Iteration 2; exact message-only `turn.failed` fixture + narrow-needle collision fixture landed. |

Gaps above are informational; Ken may additionally direct that any specific gap
(e.g. `kilo-native-error-strings`) be promoted to a live research run before
Phase 8, but none is a proposed behavior change in this report.

No gap was promoted by this disposition; gaps remain non-executable as recorded.
