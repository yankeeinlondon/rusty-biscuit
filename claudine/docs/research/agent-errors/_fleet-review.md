# Fleet Review — remaining-roster research, smell check, and checkpoint

Increment **B3** of spec `2026-07-11-provider-errors-as-data` (Phase 6 of the
execution plan). Phase 5 piloted Codex; this phase researched the remaining nine
roster providers, ran the cross-provider copy-paste smell check, produced the
source-liveness advisory, and reviewed every document. It records the state of
the mandatory human fleet checkpoint.

## How the fleet was run

As in the Codex pilot ([`_pilot-codex.md`](./_pilot-codex.md)), the live fleet
(`claudine sequence` over [`_fleet.md`](./_fleet.md), which spawns a networked
agentic research session) was not runnable in the original non-interactive
research session. The nine documents were authored by the implementing agent acting as
the researcher — grounded in each provider's immutable Phase-A seed baseline
(`docs/research/agent-errors/_seeds/<slug>.yaml`) and its sibling
`signals/<slug>.md` detection research — then run through the **exact** mechanical
verification the fleet's `success` stack runs: `md schema validate` and the
deterministic gate (`claudine-gen agent-errors check <slug>`). Subsequent review
iterations accepted the checkpoint and graduated the verified documents.

## Roster completeness

Exactly **ten** provider research documents exist, one per compiled `Provider`:

`antigravity`, `claude`, `codex`, `gemini`, `goose`, `kilo`, `kimi`, `opencode`,
`pi`, `qwen`.

## Deterministic-gate results (full roster)

Every document passes both gates cleanly; each checker run records an explicit
clean outcome, so no resume was required.

| Provider | `md schema validate` | `agent-errors check` | Seeds preserved | Capacity class |
|---|---|---|---|---|
| claude | valid | clean | all (kind+msg) | covered by seed (`overload`/`overloaded`) |
| codex | valid | clean | all (kind+msg) | accepted `overloaded` + `selected model is at capacity` |
| gemini | valid | clean | all (kind+msg) | **gap** (unpinned CLI phrasing) |
| goose | valid | clean | n/a (parser-less, no seed) | **gap** (no overload variant) |
| kilo | valid | clean | all (kind+msg) | **gap** (native strings are detection) |
| kimi | valid | clean | all (msg+code) | **gap** (overload is 429 `StepRetry` detection) |
| opencode | valid | clean | all (kind+msg) | **gap** (overload lives in stderr detection) |
| pi | valid | clean | all (msg) | covered by seed (`overloaded`/`503`) |
| qwen | valid | clean | all (kind+msg) | **gap** (unpinned CLI phrasing) |
| antigravity | valid | clean | all (msg) | covered by seed (`overloaded`/`503`/`resource_exhausted`) |

**Findings / resumes:** 0 across the roster. Each document was authored seed-first
with provenance and converged in a single pass. The validate-and-resume mechanism
itself was regression-exercised in Phase 5 (broken control fired three findings;
`lib/tests/agent_errors_fleet.rs` locks the `success`-stack shape).

## Cross-provider copy-paste smell check

Comparing every provider's flattened kind+msg needle sets:

- **`opencode` ≡ `kilo` — byte-identical** (both branches). This is the **one**
  exact duplicate and it is **justified**: Kilo speaks an OpenCode-compatible wire
  shape and reuses the OpenCode *parser* with a fixed `Kilo` identity, and its
  Phase-A seed was transcribed as an ordered copy of OpenCode's table. The
  identity is documented in [`kilo.md`](./kilo.md) (`## Shared Parser, Distinct
  Vocabulary`), whose researched surfaces (`PROMOTION_MODEL_LIMIT_REACHED`,
  `PAID_MODEL_AUTH_REQUIRED`, gateway 402) are genuinely Kilo-specific and
  recorded as gaps for a possible future divergence. **Not returned for rerun.**
- **`gemini` vs `qwen` — near-identical, not identical.** Qwen Code is a Gemini
  CLI fork, so their tables share lineage; they differ in the configuration kind
  bucket (`gemini` carries an extra `denied` needle). Each is assessed from its
  own evidence and cross-cites its own `signals/<slug>.md`. Justified lineage
  similarity; not a copy-paste defect.
- **All other pairs — distinct.** `claude` (has `overload`/`credit`/`ratelimit`),
  `codex` (adds `denied`/`config` to configuration and two capacity messages), `kimi`
  (msg+code only), `pi` and `antigravity` (msg-only, capacity-rich), and `goose`
  (empty) each carry a provider-specific shape.

No document was returned for independent re-research on copy-paste grounds.

## Source-liveness advisory (advisory only)

Per Phase 6, source liveness is reported, **not** enforced — a transient network
resolution error must never fail the fleet. This non-interactive session did not
probe the cited URLs over the network. Advisory notes on citation stability:

- **Strong (source-pinned or official docs):** claude (Anthropic API errors +
  Agent SDK), codex (OpenAI error-codes + non-interactive docs), kimi (wire-mode
  docs + `protocol/kimi.rs` constants), pi (repo `json.md`), opencode/kilo
  (`signals/` `LogClassification` source_code records).
- **Documentation-only / weaker:** gemini and qwen (headless docs pages; the
  exact capacity phrasing is an explicit gap, not graduated), antigravity (no
  published implementation source at tag 1.1.0 — surfaces are empirical from the
  installed `agy` binary, recorded as a gap), goose (docs page + `signals/`
  `ProviderError` catalog; research-only, no runtime vocabulary).

None of these blocks the fleet; the weaker-cited capacity/overload phrasings are
deliberately withheld as gaps rather than graduated on a soft citation.

## Per-document review

Every document was reviewed for: stable citations, explicit gaps, collision /
precedence notes, and separation from signal-detection semantics (D9).

- **Citations** — each frontmatter `docs` URL is the best official surface; each
  body `## Sources` cites the provider docs/source, the sibling `signals/<slug>.md`
  detection records (cross-citation, not duplication), and the immutable Phase-A seed.
- **Explicit gaps** — every provider that could not pin the capacity/overload
  class records it as a `gaps` entry; kimi/opencode/kilo additionally explain that
  their overload surface is a *detection* concern, not rendering vocabulary.
- **Collision / precedence notes** — every document carries a `## Collisions and
  Precedence` section flagging its broadest seeds (`api`, `model`, `provider`,
  `auth`, `rate`, bare HTTP numbers) and the late-`api_remote` ordering; numeric
  HTTP substrings are uniformly rejected as unsafe and recorded as gaps.
- **Signal separation (D9)** — no document encodes a detection record as a
  rendering needle. Usage caps, rate-limit extraction, exit-code mapping, and
  Goose/Kilo/OpenCode wire payloads are cited from `signals/`, never duplicated.

## Unresolved gaps (explicitly listed, not silently accepted)

These are deterministic-gate-satisfying **gaps** retained after graduation as
non-executable research scope — recorded, never fabricated into needles:

1. **gemini / qwen — `capacity-overload-phrasing`.** Google-API
   `RESOURCE_EXHAUSTED` / 429 / 503 is passed through, but the exact CLI-rendered
   string was not source-pinned; no capacity needle graduated.
2. **opencode / kilo / kimi — capacity is detection, not rendering.** The overload
   family lives in `signals/` (`LogClassification::ProviderLimit(Overloaded)`,
   Kilo `PROMOTION_MODEL_LIMIT_REACHED`, Kimi 429 `StepRetry`); no rendering needle
   graduated.
3. **kilo — `kilo-native-error-strings`.** Kilo's promotion-limit /
   paid-model-auth phrasing remains deferred rather than diverging from the
   shared OpenCode seed without stronger rendering evidence.
4. **numeric-http-codes (claude, gemini, qwen, and noted elsewhere).** Bare HTTP
   status substrings are unsafe in the case-insensitive substring cascade; they
   need an exact-match / `code_buckets`-style surface (Phase-7 R2a discipline).
5. **antigravity — no published source.** Auth/capacity surfaces are empirical
   from the installed binary; firmer citations await Google publishing source.
6. **goose — `no-stream-parser`.** Research-only, explicitly empty at runtime; its
   typed `ProviderError` catalog is documented for a future parser but not
   encoded as vocabulary.
7. **codex — `numeric-http-codes`.** The exact capacity phrase graduated; bare
   HTTP-code substrings remain withheld because the current matcher cannot
   discriminate them safely.

No provider carries an *unresolved deterministic failure* — every document is
gate-clean; the items above are acknowledged gaps, which the gate accepts by
design.

## Graduated runtime

Research frontmatter is now the sole runtime vocabulary source. Phase 8 deleted
the facts keys, generated `stream/providers/vocabulary.rs` from the ten research
documents, and retained the immutable Phase-A seeds solely for deterministic
identity checks. The two accepted Codex additions are the only classification
deltas: `overloaded` and `selected model is at capacity`, both appended to the
first `api_remote` message bucket with positive and collision fixtures.

## Human fleet checkpoint (◆ B3) — ACCEPTED

Spec B3 and plan Phase 6 require a human (Ken) fleet checkpoint before Phase 7
reconciliation, to record accepted documents, unresolved gaps, and any document
that must be rerun before reconciliation.

The implementation and review iterations accepted all ten documents for C1
reconciliation. No document required rerunning; the gaps above remain explicit,
non-executable follow-up scope. The accepted checkpoint was based on these
mechanical prerequisites:

- All ten `agent-errors/*.md` validate against `_schema.yaml`.
- The deterministic gate is clean for all ten (explicit clean outcomes, no
  findings).
- The cross-provider copy-paste comparison surfaces one justified duplicate
  (`opencode` ≡ `kilo`) and no unjustified copy.
- `claudine-gen check` confirms the research-backed generated artifacts are clean.
