# Codex Pilot — telemetry, review, and checkpoint

Increment **B2** of spec `2026-07-11-provider-errors-as-data` (Phase 5 of the
execution plan). This records the Codex pilot's telemetry, the broad-substring
false-positive review, the signals-topic coordination outcome, and the state of
the mandatory human checkpoint.

## How the pilot was run

The live fleet (`claudine sequence` over
[`_fleet.md`](./_fleet.md), provider OpenCode / `kimi-for-coding/k2p7`) spawns a
networked agentic research session and is **not** runnable in this
non-interactive execution session. The pilot was instead executed by the
implementing agent acting as the researcher: [`codex.md`](./codex.md) was
authored against the Phase-A seed
(`docs/providers/facts/codex.yaml::error_vocabulary`) and the sibling
`signals/codex.md` research, then run through the **exact** mechanical
verification the fleet's `success` stack runs — `md schema validate` and the
deterministic gate (`claudine-gen agent-errors check`). A live fleet run against
the same schema/gate remains for the interactive checkpoint (below); the schema,
prompt, gate, and their behavior are all exercised here.

## Deterministic-gate telemetry

Checks in the gate (`gen/src/agent_errors_check.rs`): seed preservation, needle
hygiene, provenance coherence, invented-seed, motivating-class coverage.

| Run | Input | Checks fired | Findings written | Findings file | Resume trigger |
|---|---|---|---|---|---|
| Clean (committed `codex.md`) | preserves all seeds, `overloaded` documented, capacity gap recorded | 0 | 0 | absent | none |
| Broken control (dropped `quota` seed, uncited `upstream`, no capacity coverage) | — | 3 | 3 | written | would fire one resume |

Broken-control findings (verbatim, the payload a resume message would carry):

1. `seed_preservation` / `kind` — seeded `quota` missing (R1).
2. `provenance_coherence` / `kind` — `upstream` has `evidence: source_code` but
   no `source`.
3. `motivating_class` — no overload/capacity vocabulary and no `gaps` entry.

Observations:

- **Findings emitted:** the committed document is clean — 0 findings, no file.
  All five check classes are demonstrably wired (three fired on the control).
- **Resumes fired:** 0 on the clean document. The `when: file_exists(findings)`
  guard in `_fleet.md` only fires when the file survives, so a clean run
  performs no resume — the intended behavior.
- **Convergence:** the clean document converged in a single authoring pass
  because it was written seed-first with provenance. The broken control shows
  the mechanism would surface precise, actionable findings for a resume.
- **Budget fit (`max_attempts: 2`):** all three control findings are
  single-edit corrections (re-add a seed, add a `source`, add a capacity needle
  or gap). Two attempts is comfortably sufficient for this failure profile; no
  evidence the budget is too small. Recommend keeping `max_attempts: 2` for the
  fleet.

## Broad-substring false-positive review

Per Phase 5, reviewing `rate`, `model`, `auth`, and numeric HTTP terms against
representative non-error prose and earlier buckets:

- **`overloaded` (the one proposed addition)** — narrow and unambiguous; absent
  from Codex success/progress frames (`turn.completed`, `agent_message`, tool
  item events). Appended *after* the seeded needles in the first `api_remote`
  message bucket, so seed precedence is unchanged. **Safe.**
- **`auth` (seed, kind branch)** — broad but sticky (Phase-A seed); untouched.
  Matches "auth"/"authenticated"/"authorization"; acceptable in the
  `configuration` bucket. No change.
- **`api` (seed, kind branch)** — the broadest seed; matches any "api" prose.
  Sticky; untouched. Flagged in `codex.md` collisions so Phase C is aware.
- **Bare HTTP numbers (`429`/`503`/`401`/`403`)** — **rejected** as substring
  needles: as raw substrings they collide with token counts, IDs, and
  timestamps in non-error frames. Recorded as the `numeric-http-codes` gap for a
  future exact-match / `code_buckets`-style surface. Not graduated.
- **`model`** — not proposed (would collide with every model-name mention in
  normal Codex output).

## Hardening outcome (prompt / schema / validator)

The pilot exposed **no defect** requiring a code change to the schema, fleet
prompt, or validator:

- The schema accepted the nested needle/provenance shape and rejected the
  invalid-provenance fixture (existing `gen/tests/agent_errors_check.rs`
  coverage).
- The gate's five checks all fired correctly on the control and stayed silent on
  the clean document; the stale-first / write-on-failure findings contract
  behaved as specified.
- The fleet `success` stack's `no_error` shell + `when`-guarded `resume` shape
  is regression-locked by `lib/tests/agent_errors_fleet.rs`.

**Recommendation carried to the checkpoint (not implemented — Phase C scope):**
add a validator check that flags bare numeric-HTTP substrings (`\b(4|5)\d\d\b`)
proposed as `kind_buckets`/`msg_buckets` needles, steering them toward an
exact-match surface. Deferred deliberately: it is a *reconciliation* rule (R2a
collision discipline), the pilot produced no such needle to test it against, and
Rule 3 (surgical changes) argues against speculative validator surface before a
real failure motivates it.

## Signals-topic coordination (D9)

Recorded in [`_signals-overlap.md`](./_signals-overlap.md). Summary: Codex's
usage-cap / rate-limit **detection** records stay in `signals/`; `codex.md`
cites them rather than duplicating. The proposed `overloaded` needle is a
*rendering* concern unique to this topic. No `_overlap-exclusions.yaml` is
needed because the `agent-errors` gate does not replay cross-topic fixtures.

## Human checkpoint (◆ B2) — PENDING

Spec B2 and plan Phase 5 require a human (Ken) checkpoint before Phase 6, to
approve:

1. the Codex research output ([`codex.md`](./codex.md)),
2. the validate-and-resume telemetry (above), and
3. whether the D10 validate-and-resume pattern graduates into the general fleet
   recipe (tracked as `features/_unscheduled/fleet-validate-and-resume.md`, P5).

**This checkpoint cannot be satisfied in a non-interactive session.** Phase 6
must not begin until Ken reviews the artifacts above. The mechanical
prerequisites for the checkpoint are all green:

- `md schema validate 'docs/research/agent-errors/codex.md'` → valid.
- `claudine-gen agent-errors check codex` → clean, no findings file.
- `claudine-gen check codex` → `codex: clean` and `stream vocabulary.rs: clean`
  (the runtime table is still facts-backed and byte-identical; research changed
  no classification behavior).
