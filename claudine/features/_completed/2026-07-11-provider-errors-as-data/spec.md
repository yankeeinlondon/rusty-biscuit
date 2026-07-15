---
created: 2026-07-11
review_iterations: 6
status: complete
reviewed: true
reviewed_by: ken
reviewed_on: 2026-07-14
---

# Provider Error Vocabulary as Data

> **Status:** COMPLETED 2026-07-14. The live ten-provider Phase-B fleet
> converged with ten clean first attempts and no resumes. Ken accepted all 55
> evidence-backed append-only additions at C1; C2 regenerated the runtime
> vocabulary with exhaustive Level-1 addition, precedence, code, and near-miss
> coverage. C3 retained the live telemetry and closed the feature.

Move the per-provider error-classification vocabulary out of the stream
parsers and into the provider-metadata knowledge base, then **do it right**:
stand up an `agent-errors/` fleet-research topic so the vocabulary is
researched, provenance-attested, and generated — joining the same pipeline
that already owns `stream_protocol`, event mappings, signals, and expected
offerings.

Three phases:

- **Phase A — Plumbing (byte-identical migration).** Transcribe the current
  in-code tables into facts, generate, cut the parsers over. No behavior
  change; the existing tests prove parity.
- **Phase B — Research.** Author the `agent-errors/` topic (sidecar + fleet
  prompt), pilot one provider, then run the fleet over the roster.
- **Phase C — Graduation + reconciliation.** Re-point generation from facts
  to research and adjudicate every vocabulary delta as a deliberate,
  test-covered behavior change under the reconciliation rules (D8).

> **Reader's note (inline review, 2026-07-11):** the standalone artifact is
> deliberately *not* registered as a `ProviderInfo` field. The catalog mapping
> registry covers serialized `ProviderInfo` fields and rejects unrelated
> entries; this emitter therefore owns a typed source loader with equivalent
> facts-to-research collision and delete-on-graduate guarantees. The review
> also makes parser identity explicit so Kilo can use Kilo research while
> sharing OpenCode's wire parser, and adds a temporary table-to-table parity
> gate because example-based tests cannot prove an exact transcription.

> **RULINGS (Ken, 2026-07-11):**
> **Q1** — Kimi's JSON-RPC numeric codes migrate as `code_buckets` (D5).
> **Q2** — the generated artifact is a standalone stream-layer module, not a
> `ProviderInfo` field (D3).
> **Q3** — the `agent-errors/` fleet-research topic is **in scope** for this
> workstream ("add the new fleet research and do this right"), structured as
> Phases B/C so Phase A's migration safety is preserved.

> **IMPLEMENTATION OUTCOME (2026-07-13):** all three phases are complete.
> Research frontmatter is the sole executable vocabulary source, the Phase-A
> facts keys have been deleted, and generation is drift-checked. C1 accepted
> two append-only Codex message additions: `overloaded` and the narrow phrase
> `selected model is at capacity`; both classify as `ApiRemote` and carry
> parser-level positive and collision coverage. Other recorded provider gaps
> remain non-executable research scope and do not block this graduation.

> **RECOVERY FOLLOW-UP (2026-07-13):** D10 was amended after review found that
> the fleet resumed deterministic findings but aborted other agent-correctable
> conditions. The vocabulary graduation above remains complete; the recovery
> amendment is specified below and implemented in the fleet lifecycle, checker
> outcome contract, and focused integration coverage.

## Motivation

A standing goal of the fleet-research → metadata-knowledge-base workstream
(`2026-07-02-provider-metadata`, RATIFIED & IMPLEMENTED) is to minimize
per-provider implementations: per-provider **code** should exist only where
providers genuinely differ in *behavior*; per-provider **facts** belong in
the generated catalog.

The 2026-07-11 module-structure work (`2026-07-11-module-structure`,
Phase 6c) got the stream layer most of the way there. Error classification
now runs through one shared cascade —
`stream/providers/common.rs::classify_error_by_keywords` — and the only
per-provider remnant is a pure-data constant in each of the 8 parser files:

```rust
const ERROR_KEYWORDS: super::common::ErrorKeywords = super::common::ErrorKeywords {
    kind_buckets: &[
        (SemanticErrorKind::ApiRemote, &["rate", "quota", "billing"]),
        ...
    ],
    msg_buckets: &[ ... ],
};
```

These eight constants were facts, not behavior — and they were also
**incomplete facts**: they encoded what had happened to be observed live, not
what each provider documents. Before this feature, Codex's *"Selected model is
at capacity"* incident matched no Codex needle. The research phase closed that
exact gap with a source-pinned, narrow phrase rather than the collision-prone
`capacity` substring. The per-provider ground truth of those baseline tables is
recorded in
[`../2026-07-11-module-structure/phase6-discovery.md`](../2026-07-11-module-structure/phase6-discovery.md)
(§`classify_error`).

**Provider-ladder payoff:** before this workstream, onboarding a new provider
(the Phase H ladder pattern) required hand-writing its keyword table inside a
new parser file. Error vocabulary is now a research
deliverable like every other catalog field — researched, schema-validated,
generated, and drift-checked.

## Starting state

| Artifact | Location | Nature |
|---|---|---|
| `ErrorKeywords` struct + `classify_error_by_keywords` cascade | `lib/src/stream/providers/common.rs` | Shared code (stays) |
| `ERROR_KEYWORDS` const ×8 | `lib/src/stream/providers/<slug>.rs` (all 8 parser files) | **Data, embedded in code — the migration target** |
| Kimi JSON-RPC numeric-code match | `lib/src/stream/providers/kimi.rs::classify_jsonrpc_error` | Data-shaped prelude (`code → SemanticErrorKind`), migrates per Q1 |
| `classify_error_*` tests | each parser's test module | The behavior contract (stay put) |

Two invariants every phase must preserve:

1. **Bucket order is the behavior contract.** The cascade walks the
   provider's buckets in order and the first substring hit wins. Order
   encodes real precedence quirks: Gemini checks Configuration before
   ApiRemote in the kind branch; Codex/OpenCode/Qwen/Gemini run a *second*
   ApiRemote pass after Interrupted; Antigravity checks auth keywords first
   in the message branch; Pi/Antigravity match `"abort"` where the others
   require `"aborted"`. Every storage layer (facts, research frontmatter,
   generated code) must be order-preserving and order-auditable.
2. **Needles are matched against lowercased input.** Every needle must be
   lowercase; the generator validates this rather than trusting authors.

## Goals

- G1 — Each provider's error vocabulary lives in the metadata knowledge
  base and is validated at generation time (known kinds, non-empty ordered
  buckets, lowercase needles).
- G2 — The 8 in-parser `ERROR_KEYWORDS` constants (and Kimi's code match)
  are deleted; parsers consume generated data through one accessor.
- G3 — **Phase A is byte-for-byte behavior-preserving**: every existing
  `classify_error_*` test passes unchanged, and A2 mechanically compares
  every generated ordered bucket and Kimi code mapping with the still-present
  hand-written tables before A3 deletes them. Example-based tests cannot
  prove that an untested needle was transcribed. (Behavior changes happen
  only in Phase C, individually adjudicated under D8.)
- G4 — A new provider's error vocabulary is authored as research, never as
  parser code.
- G5 — CI drift-checks the generated artifact exactly like the rest of the
  catalog (`claudine providers generate` / `--check`).
- G6 — Post-Phase-C, every needle carries **provenance** (documented /
  source-code / issue-tracker / empirical / seed) so future disputes about a
  keyword resolve by citation, not archaeology.
- G7 — The research topic follows the provider-metadata recipe verbatim
  (sidecar rules, `_fleet.md`, pilot technique, lifecycle verification
  stacks) — no parallel research format.
- G8 — Shared wire parsers select vocabulary by the provider being wrapped,
  not by the parser module's name. Kilo retains OpenCode wire parsing but
  consumes Kilo's researched vocabulary.

## Non-goals

- **`finish()` field-population capability as metadata** (Class 2 item #9
  from the review discussion). Considered and rejected: the *facts* (who
  can report `rate_limit`, `context_usage`, `permission_prompts`) are
  declarable, but the *population* is parsing code that must exist per
  provider regardless. Post-6b each parser's summary literal already
  expresses its capability set minimally; metadata would duplicate what the
  code states.
- **Other error-text surfaces.** `stream/logs/opencode/errors.rs`
  (stderr-log classification), `cli/src/output/error_report.rs`
  (`classify_native_cli_error`), and `cli/src/output/api_errors.rs` carry
  their own provider-flavored vocabulary. Different consumers, different
  shapes; fold them in later only if this migration proves out (Future
  Directions). The research topic *may* surface vocabulary relevant to
  them — record such findings in the research docs' prose, not in the
  structured frontmatter.
- **Signal detection records.** The `signals/` research topic and
  `signals/generated.rs` own wire-level *detection* (SignalKind events).
  This workstream owns the `SemanticErrorKind` *rendering/summary*
  vocabulary. See D9 for the coordination contract between the two.
- **Changing classification behavior in Phase A.** No keyword additions,
  removals, or re-ordering ride along with the migration.
- **Changing the matching algorithm.** Classification remains
  ASCII-lowercased, case-insensitive substring matching. Boundary, regex,
  token, or Unicode-normalization semantics require a separate fix with
  collision fixtures; research must propose literals safe for this matcher.

## Design

### D1 — Data model (internal + facts shape)

The vocabulary for one provider, as gen consumes it and as Phase A's facts
files carry it (facts are serde-parsed gen inputs, so nested YAML is fine
here):

```yaml
# ordered — sequence order IS the cascade order
error_vocabulary:
  kind_buckets:                # checked against the structured error-kind discriminator
    - kind: api_remote
      needles: [rate, quota, billing]
    - kind: configuration
      needles: [auth, config, permission, denied]
    - kind: interrupted
      needles: [interrupt, cancel, abort]
    - kind: api_remote         # the "late ApiRemote" second pass — order expresses it
      needles: [api, upstream, server]
  msg_buckets:                 # checked against the free-form message
    - kind: api_remote
      needles: [rate limit, quota, billing, api error]
    - kind: configuration
      needles: [api key, authentication, not authorized, permission denied, config]
    - kind: interrupted
      needles: [interrupt, cancel, aborted]
  code_buckets:                # numeric wire codes (Kimi JSON-RPC; ruled in, Q1)
    - code: <AUTH_EXPIRED value from protocol/kimi.rs>
      kind: configuration
    - code: <CHAT_PROVIDER_ERROR value>
      kind: api_remote
```

`kind` values are the snake_case serde names of `SemanticErrorKind`
(`configuration | agent_native | api_remote | interrupted | unknown`) — the
same spelling the summary already serializes. Repeated `kind` across buckets
is legal and meaningful (the late-ApiRemote pass). Absent `kind_buckets`
(Pi, Antigravity — message-only classifiers) and absent `code_buckets`
(everyone but Kimi) serialize as omitted keys. Seeding copies Kimi's real
numeric constants from `protocol/kimi.rs` with a comment naming each.

### D2 — Source-of-truth lifecycle: facts (Phase A) → research (Phase C)

Follow the established **facts-file bootstrap** pattern
(`docs/providers/facts/<slug>.yaml`, "seeded once from the hand-written
provider constants… delete-on-graduate when a research topic lands"):

- **Phase A:** add an `error_vocabulary:` key to each provider's facts
  file, seeded by **transcribing the current in-code tables verbatim**
  (they are the current runtime ground truth; several were empirically
  hardened — OpenCode's `providermodelnotfound`, Antigravity's
  OAuth-flavored `sign in`/`401`/`403`). Register the source declaration in
  the vocabulary emitter's typed loader, **not** the general mapping registry:
  that registry is exhaustive over serialized `ProviderInfo` fields and this
  vocabulary intentionally is not one.
- **Phase C:** the vocabulary loader's declared source re-points to the
  `agent-errors/` research frontmatter. Its collision check errors while a
  facts file still carries `error_vocabulary`, reproducing the standard
  delete-on-graduate guarantee. Before deleting those keys, copy each Phase-A
  value byte-identically into
  `docs/research/agent-errors/_seeds/<slug>.yaml` as a direct, immutable
  `ErrorVocabulary` baseline for future deterministic checks. Unit tests cover
  facts-only, research-only, missing declared input, and facts+research
  collision cases.

### D3 — Generated artifact: one lib module (signals precedent)

> **RULING (Ken, 2026-07-11): standalone module confirmed.**
> `lib/src/stream/providers/vocabulary.rs`, `pub(crate)` within the stream
> layer. Revisit only if a catalog-surface consumer (e.g. `claudine
> providers` matrix) materializes — the move is an emitter change plus one
> regeneration, with the source-of-truth files untouched.

`claudine-gen` gains an emitter (sibling of `gen/src/signals.rs`) that
produces a single generated module:

```
lib/src/stream/providers/vocabulary.rs   // GENERATED by claudine-gen — DO NOT EDIT BY HAND.
```

containing one `ErrorKeywords` const per provider (extended with
`code_buckets` per Q1) plus one accessor:

```rust
pub(crate) fn error_keywords(provider: Provider) -> &'static ErrorKeywords
```

Rationale (why not a `ProviderInfo` field): `ErrorKeywords` references
`SemanticErrorKind` (a `stream::semantic` type), the vocabulary has exactly
one consumer, a `ProviderInfo` shape change costs `emit.rs` + ten
regenerated `data.rs` files on every evolution, and the exhaustive accessor
makes "no vocabulary yet" explicit (a provider without a stream parser,
e.g. Goose, gets an explicit empty table rather than silently
`AgentNative`-everything).

Generator validations at emit time: every `kind` is a known
`SemanticErrorKind` name; every needle is non-empty and lowercase; every
bucket has ≥1 needle; `msg_buckets` is non-empty for every provider that
has a stream parser. Exact duplicate needles and prefix/substring shadowing
within one input branch are included in the C1 delta report; shadowing is
accepted only when research explains the intended precedence. Empty tables
are valid only for providers without a structured stream parser.

### D4 — Runtime wiring

- Delete the 8 in-parser `ERROR_KEYWORDS` consts.
- Each parser owns or receives its runtime `Provider` identity and delegates
  with `vocabulary::error_keywords(self.provider)`. Dedicated parser
  constructors stamp their fixed provider; the shared OpenCode parser
  receives `Provider::OpenCode` or `Provider::Kilo` from `for_provider` and
  rejects any other provider. This prevents Kilo research from becoming
  generated-but-unreachable data.
- `common.rs` (struct + cascade) gains the `code_buckets` field and a
  `code: Option<i32>` cascade input checked before the kind branch (D5).
- The `classify_error_*` tests in each parser module are **not moved** —
  they keep asserting through the public parser behavior and become the
  regression gate for both the Phase A cutover and every Phase C delta.
- Add an end-to-end Kilo error fixture whose winning classification differs
  from OpenCode's test vocabulary so shared parsing cannot regress to a
  hard-coded OpenCode lookup.

### D5 — Kimi numeric codes

> **RULING (Ken, 2026-07-11): migrate.** Kimi's JSON-RPC code mapping
> (`AUTH_EXPIRED → Configuration`, `CHAT_PROVIDER_ERROR → ApiRemote`,
> parse/request/method/params/internal → `AgentNative`) moves into
> `code_buckets`; the named constants stay in `protocol/kimi.rs` as
> wire-protocol definitions used by parsing. The numeric values appearing
> in both places is accepted (small, comment-linked, and drift-checked by
> the existing kimi classify tests).

Code matching remains first and exact. All currently explicit standard
JSON-RPC codes map to `AgentNative` in `code_buckets`, not only the two
Kimi-specific codes; otherwise a standard code carrying auth/rate-limit prose
would newly fall through to message matching. Unknown codes continue to fall
through. The generator rejects duplicate codes within one provider.

### D6 — Drift and enforcement

- The generated module carries the standard `GENERATED — DO NOT EDIT`
  header and is covered by the existing `claudine providers generate`
  drift check in CI.
- The `dispatch_inventory` guard will see the per-provider references move
  from 8 parser files into `vocabulary.rs`; regenerate the committed
  inventory in the same change (`CLAUDINE_UPDATE_INVENTORY=1 cargo nextest
  run -p claudine-cli --test dispatch_inventory`).
- One gen-side unit test per validation rule in D3 (unknown kind, uppercase
  needle, empty needle, empty bucket, duplicate code, missing required parser
  vocabulary → generation error naming the provider + branch/bucket).
- During A2, while local constants still exist, a temporary parity test
  compares the complete generated tables against all eight local tables and
  Kimi's full numeric mapping, preserving order and duplicates. A3 may remove
  this migration-only test with the old constants; parser tests remain the
  durable behavior suite.

### D7 — The `agent-errors/` research topic (Phase B)

New topic at `claudine/docs/research/agent-errors/`, built strictly from
the provider-metadata recipe (start from `docs/research/_TEMPLATE.md` and
the `signals/` topic as the closest sibling):

**Research question per provider:** *how does this CLI report errors in its
non-interactive structured output* — documented error strings, structured
error-kind discriminators, numeric wire codes, rate-limit / quota / billing
/ auth-failure / capacity / interruption message text — from official docs,
the CLI's source (most are open source), and issue trackers. The deliverable
is a proposed ordered vocabulary with per-needle evidence.

**`_schema.yaml` sidecar (SimplifiedSchema).** The sidecar mirrors the
nested facts shape directly — SimplifiedSchema supports nested inline
objects (to `MAX_INLINE_OBJECT_DEPTH = 32`, grammar Decision #11), so
buckets nest their needle lists and each needle is an object carrying its
own provenance:

```yaml
$schema: ./_schema.yaml
# bucket sequence order IS the cascade order (invariant #1); needle objects
# carry the per-needle provenance contract inline.
kind_buckets: "{ kind: enum(configuration,agent_native,api_remote,interrupted,unknown; required), needles: { text: string(required), evidence: enum(documented,source_code,issue_tracker,empirical,seed; required), source: string, empirical: { fixture: string(required), capture_notes: string(required) } }[] }[]"
msg_buckets: "{ kind: enum(configuration,agent_native,api_remote,interrupted,unknown; required), needles: { text: string(required), evidence: enum(documented,source_code,issue_tracker,empirical,seed; required), source: string, empirical: { fixture: string(required), capture_notes: string(required) } }[] }[]"
code_buckets: "{ kind: enum(configuration,agent_native,api_remote,interrupted,unknown; required), codes: { code: number(required), name: string, evidence: enum(documented,source_code,issue_tracker,empirical,seed; required), source: string, empirical: { fixture: string(required), capture_notes: string(required) } }[] }[]"
# gaps: error surfaces the researcher could not confirm either way.
gaps: "{ area: string(required), notes: string(required) }[]"
```

Provenance lives only in the research layer: graduation (Phase C) projects
`needles[].text` into the runtime vocabulary, and the generated
`ErrorKeywords` carries no evidence fields. Exact field spelling is
finalized against the then-current Darkmatter grammar in increment B1
(newer Darkmatter schema updates are landing; B1 re-verifies rather than
trusting this sketch).

For non-seed evidence, `source` is required by the deterministic coherence
check even if the sidecar cannot express that conditional constraint. It must
be a stable citation: an official documentation URL, a commit-pinned source
permalink with record identity, or an issue URL with provider version/date.
Search-result URLs and unversioned repository homepages are not evidence.
`seed` may omit `source`; `empirical` requires a typed `empirical` object whose
`fixture` is an existing, scrubbed `./_fixtures/...` file reference and whose
`capture_notes` are non-empty. Research documents retain citations after
runtime projection.

**`_fleet.md` prompt document** instructs each research session to:
1. Read the provider's **seeded vocabulary** from the immutable Phase-A
   baseline at `docs/research/agent-errors/_seeds/<slug>.yaml` as
   the starting point — every seeded needle must reappear in the output
   with `evidence: seed` unless upgraded to a stronger evidence class with
   a citation.
2. Research documented error surfaces and propose additions/orderings with
   per-needle evidence.
3. Explicitly check the **motivating-incident class**: capacity/overload
   vocabulary (`overloaded`, `at capacity`, `resource_exhausted`, 429/503
   phrasings).
4. Record unresearchable areas in `gaps` rather than guessing.
5. Not research signal *detection* surfaces (exit codes → SignalKind, wire
   event shapes) — that is the `signals/` topic's territory (D9).
6. Check proposed substrings against representative success/non-error prose
   and earlier buckets. Broad fragments such as `rate`, `model`, `auth`,
   `401`, and `403` require a collision/precedence note; evidence that a
   phrase exists does not prove it is a safe substring classifier.

**Pilot technique:** run ONE provider first, review with Ken, harden the
sidecar/prompt from what the pilot exposes, then fleet the remaining
roster. Pilot = Codex per P3 (motivating incident, open-source CLI).

**Verification:** per the recipe, the fleet prompt carries lifecycle
verification stacks; each output doc's frontmatter is sidecar-validated at
compose time. This topic additionally pilots the **deterministic
validate-and-resume** pattern (D10): the lifecycle doesn't just report
progress, it gates on mechanical checks and resumes the research session to
fix what the checks catch.

### D8 — Reconciliation rules (Phase C)

Research output does not silently become behavior. Graduation applies these
rules, with a Ken checkpoint on the consolidated delta report:

- **R1 — Seeds are sticky.** A needle carried from the current tables
  (`evidence: seed` or `empirical`) is never removed or re-kinded by
  research. Docs saying "that string doesn't exist" loses to the fact that
  we observed it. Removals happen only via a dedicated fix with a
  reproducing case. *(P1)*
- **R2 — Additions require evidence.** A new needle/bucket lands only with
  `evidence: documented | source_code | issue_tracker` + `source`, and each
  addition (or coherent bucket of additions) gets a `classify_error_*` test
  in the same change.
- **R2a — Additions require discrimination evidence.** Each accepted addition
  includes a positive fixture and, for broad or overlapping substrings, a
  negative/collision fixture. Tests assert the winning bucket, not merely
  that the needle appears in generated output.
- **R3 — Ordering changes are behavior changes.** A researched re-ordering
  of buckets (or a new bucket inserted mid-cascade) needs explicit
  justification in the delta report; default is to append new buckets after
  existing ones of the same branch.
- **R4 — Kind reassignment of an existing needle** is individually
  adjudicated (it silently changes rendering/summary classification for
  already-observed errors).
- **R5 — Cross-provider consistency is advisory, not forced.** The fleet
  will surface that providers classify similar strings differently; do not
  homogenize without per-provider evidence — the vocabulary is
  per-provider by design.

### D9 — Coordination with the signal-assurance workstream

Two live workstreams touch provider error text. The boundary:

- **`signals/` topic + `signals/generated.rs`** own *detection*: wire-level
  records that fire `SignalKind` events (the signal-assurance spec's
  "detected, researched and attested absent, or loudly flagged" contract).
- **`agent-errors/` (this spec)** owns the *rendering/summary* vocabulary:
  `SemanticErrorKind` classification of error kind/message text.

The signal-assurance plan already schedules Codex overload research (its
phase 2). Sequence the Codex **pilot** (B2) after — or explicitly
coordinated with — that work so the two research passes over the same
provider surface cite each other instead of duplicating. If the signals
topic's overlap-exclusion mechanism (`signals/_overlap-exclusions.yaml`
precedent) is the right tool, add an equivalent exclusion note to the
`agent-errors/` topic. A future shared vocabulary source serving both
layers stays out of scope (Future Directions).

### D10 — Deterministic validate-and-resume in the fleet lifecycle

Today's fleet lifecycles use hooks to *communicate* progress. The upgrade:
run **deterministic checks** in the `success` stack and, when a check
catches a surprising or known-untrue outcome, **`resume` the same research
session** with the findings as the prompt — so the model corrects its own
output with full session context instead of a human (or a fresh session)
re-deriving it.

**Mechanism (already supported, verified 2026-07-11):** a lifecycle
`resume` control seeds `next_resume_session_id` + `next_prompt_override`
(`control_dispatch.rs` Resume arm), is budgeted by `max_attempts`
(`ControlBudgets`), gated on the *wrapper agent's* resume capability
(`supports_resume` metadata — note: what matters is the CLI running the
research, not the provider being researched), and the message is
late-binding interpolated (DM2) so it can carry check findings. On budget
exhaustion the resume dispatch falls through (`Exhausted → Fallthrough`) only
to `finalize`. The non-clean finalize guard raises `error`; the dispatcher maps
that `StackControl::Error` to `Abort`, so the fleet/compose command exits
non-zero while preserving both the machine-visible findings report and the
authored finalize reason for C1 human review.

**Recovery-policy amendment (2026-07-13):** validation is useful only when the
lifecycle attempts repairs that are both safe and likely to converge. Every
negative condition is therefore classified by *who can correct it*, not merely
by the phase in which it was detected:

| Condition | Owner | Lifecycle response |
|---|---|---|
| The provider returned success but did not create/update the research document | research session | `resume` the same session with the missing postcondition |
| Deterministic report has `status: findings` | research session | `resume` with the durable findings report |
| Deterministic report has `status: gate_error` and `error_scope: research_document` | research session | `resume` with the document/schema error |
| Provider execution failed with a transient disposition | runtime/provider | bounded `retry` with exponential backoff; a timeout may `resume` when the wrapper captured a resumable session |
| Seed loading, checker inputs, or other authoritative gate state failed | maintainer/infrastructure | fail closed; the research session must not edit immutable authority to make the gate pass |
| Checker process or outcome-report persistence failed | maintainer/infrastructure | fail closed before reading any possibly stale report |
| Completed checker produced no report or an unknown status/scope | checker protocol | fail closed |
| Authentication, configuration, billing, interruption, runaway, or a non-waitable cap failed | operator/policy | fail closed; blind replay is unsafe or predictably ineffective |
| A recovery budget was exhausted | maintainer | fall through to `finalize`, preserve the last durable report, and exit non-zero |

`resume` and `retry` budgets are run-scoped by control type, not independently
reset for each conditional branch. All same-session remediation branches use
the same `max_attempts: 2` ceiling, allowing at most two additional agent turns
in total whether the first defect was a missing file, invalid document, or
deterministic finding. Transient fresh-run recovery has its own bounded retry
budget. A later branch consumes the remaining budget rather than opening a new
one; this prevents a sequence of different failed checks from multiplying the
attempt ceiling.

The outcome report retains the `clean | findings | gate_error` status contract.
A `gate_error` additionally carries `error_scope: research_document |
gate_input`. `research_document` covers the provider-authored document being
missing, malformed, schema-invalid, or incompatible with the research
vocabulary shape and is repairable by resume. `gate_input` covers immutable
seed or other authoritative checker-input failures and is terminal. The field
is omitted for `clean` and `findings`. Report persistence failure remains an
ordinary non-zero checker error because no new report can be trusted.

**Shape in the fleet doc's `success` stack:** first, a missing/stale-document
guard resumes the live research session. Otherwise an approved `shell` check
script (early-binding, reads the output doc from its static path, atomically
writes an explicit outcome report) runs. `when:`-guarded branches then resume
for `findings` and repairable `gate_error`, report success for `clean`, and fail
closed for absent, unknown, or non-repairable outcomes. Fleet conditions read
the report through Markdown frontmatter read-side functions.

**Shape in the fleet doc's `failure` stack:** timeout recovery prefers a
bounded same-session `resume` because it retains research context. Other errors
whose diagnostic disposition projects `err.is_transient == true` use bounded
`retry` with exponential backoff. The stack deliberately has no catch-all
recovery: every other failure falls through to its terminal report and
`finalize` guard.

Completed checks exit zero after the outcome is durably persisted. Report
persistence failures exit non-zero and stop the stack, so lifecycle processing
never consumes stale state. Replacement must not remove the prior failure
report first: a synced sibling temporary file atomically replaces it only when
the new report is ready. Clean is never inferred from absence. The original B2
coverage remains responsible for check failure → one resume → corrected
document → no second resume, schema failure, report-write failure, and resume
budget exhaustion. The recovery follow-up adds missing/stale document recovery,
transient execution retry, authoritative gate-input failure, unknown outcome
protocol, and retry-budget exhaustion. Exhaustion must leave a machine-visible
failing validation result and make the fleet fail; it may fall through to
**human adjudication**, but must not turn a known-bad research document into a
successful fleet result.

**Deterministic checks for `agent-errors` (initial set):**

| Check | Catches | On failure |
|---|---|---|
| Seed removal | a seeded needle/code absent from its original branch (mechanical half of R1) | resume, listing the removed rows |
| Seed re-kind | a seeded needle/code moved to another semantic kind (R4) | resume, listing the old and new kinds for adjudication |
| Seed reorder | a seeded needle/code changed bucket or item position (R3) | resume, listing the old and new positions for adjudication |
| Needle hygiene | non-lowercase / leading-trailing-whitespace needles | resume with the offending needles |
| Provenance coherence | non-seed evidence with empty `source`; empirical evidence without a resolvable scoped fixture and capture notes; `evidence: seed` on a needle not in the seed (invented provenance) | resume |
| Motivating-class coverage | no overload/capacity vocabulary in any bucket **and** no `gaps` entry acknowledging it | resume ("research capacity/overload error surfaces or record the gap") |
| Cross-provider copy-paste smell | needle set verbatim-identical to an already-completed provider's doc (independent research should not be byte-identical) | flag in B3 review, not resume (needs the fleet's other outputs) |
| Source liveness | cited URLs unresolvable | advisory only (network-flaky; never blocks) |

The pilot (B2) explicitly evaluates this pattern — how many resumes fire,
whether corrections converge, whether the budget is right — and the B2
checkpoint decides whether the pattern graduates into the general fleet
recipe (tracked as an unscheduled feature; see P5).

## Implementation increments

Each lands independently, `just test` + `just lint` green between. Ken
checkpoints marked ◆.

**Phase A — plumbing (byte-identical):**

1. **A1 Facts seeding** — `error_vocabulary:` in the 8 dedicated
   stream-parser providers' facts files (verbatim transcription, bucket order
   preserved, Kimi's complete explicit code mapping included per Q1); add the
   source declaration to the vocabulary loader. Kilo receives an explicit
   seed copied from OpenCode because it shares that parser today; Phase B
   researches the copy independently.
2. **A2 Generator** — emitter + validations + `vocabulary.rs`; generated
   consts mechanically compared with the still-present parser constants and
   Kimi code match before cutover (an eyeball diff is supplementary).
3. **A3 Cutover** — parsers consume `vocabulary::error_keywords(...)`;
   delete the 8 local consts + Kimi's code match; thread provider identity
   through the shared OpenCode/Kilo parser; add `code_buckets` to the cascade;
   full `classify_error_*` suite green; regenerate dispatch inventory.
4. **A4 Docs** — architecture.md (skill) stream section; provider-ladder
   onboarding notes.

**Phase B — research:**

5. **B1 Topic authoring** — `agent-errors/_schema.yaml` +
   `agent-errors/_fleet.md` per D7; sidecar shape finalized against the
   then-current SimplifiedSchema grammar; author the deterministic check
   script(s) + the `success`-stack validate-and-resume wiring per D10.
6. **B2 Pilot (Codex)** — one research run with the D10 checks live;
   ◆ review output **and the validate-and-resume telemetry** (resumes
   fired, convergence, budget fit) + harden sidecar/prompt/checks;
   coordinate with signal-assurance phase 2 (D9). This checkpoint also
   decides whether the D10 pattern graduates to the general fleet recipe
   (unscheduled feature, P5).
7. **B3 Fleet** — remaining roster providers (all 10 per P2), including
   the cross-provider copy-paste check over the accumulated outputs; ◆
   review the fleet's outputs.

**Phase C — graduation + reconciliation:**

8. **C1 Delta report** — consolidated diff of researched vocabulary vs
   immutable Phase-A seed baselines, classified per D8 (R1 conflicts, R2 additions, R3
   orderings, R4 re-kinds); ◆ Ken adjudicates.
9. **C2 Graduation** — mapping registry re-points to research frontmatter;
   facts entries deleted (delete-on-graduate); accepted deltas land with
   their tests; regenerate + full suite green.
10. **C3 Docs + closeout** — skill docs, research topic listed in the
    skill's Topic Research section, spec status → completed.

## Future directions (explicitly out of scope)

- Folding the other error-text surfaces (`stream/logs/opencode/errors.rs`
  classification, `cli/output/error_report.rs`, `api_errors.rs`) into the
  same vocabulary model if their shapes permit.
- A shared vocabulary source serving both the `SemanticErrorKind` rendering
  layer and the `SignalKind` detection layer (would tangle two in-flight
  workstreams today; revisit after both land).

## Rulings and open proposals

Ruled:

- **Q1 (D5):** migrate Kimi's numeric JSON-RPC codes as `code_buckets`.
  **RULED 2026-07-11: migrate.**
- **Q2 (D3):** standalone generated module vs `ProviderInfo` field.
  **RULED 2026-07-11: standalone module.**
- **Q3:** facts-only v1 vs bundled research. **RULED 2026-07-11: bundle the
  fleet research** (Phases B/C added; Phase A keeps migration parity).

Proposals awaiting review (defaults applied unless overruled):

- **P1 (D8/R1):** seeded/empirical needles are sticky — research never
  removes or re-kinds them; removals need a dedicated fix with a
  reproducing case.
- **P2 (B3):** research the full 10-provider roster. Kilo's vocabulary is
  consumed through the provider-aware shared OpenCode parser. Goose has no
  structured stream parser, so its findings remain research-only and its
  runtime table is explicitly empty until a parser lands; parser onboarding
  is when accepted Goose records become executable and gain classifier tests.
- **P3 (B2):** pilot provider = Codex (motivating incident, open source),
  explicitly sequenced/coordinated with the signal-assurance plan's Codex
  overload research (D9).
- **P4 (D7):** ~~research frontmatter uses flat id-joined lists with
  explicit `position` fields (SimplifiedSchema one-nesting-level
  constraint)~~ **CORRECTED 2026-07-11 (Ken):** the one-nesting-level
  constraint was stale — the current grammar supports nested inline objects
  to depth 32 (Decision #11). The sidecar mirrors the nested facts shape
  with needle-level provenance objects (see D7); bucket order rides on YAML
  sequence order, same as facts. B1 re-verifies against the Darkmatter
  version current at implementation time.
- **P5 (D10):** this topic pilots deterministic recovery in its fleet
  lifecycle: one run-scoped, two-additional-turn `resume` budget repairs a
  missing/stale output document, repairable document/schema gate errors, and
  deterministic findings; a separate bounded `retry` budget with exponential
  backoff handles transient execution failures. Checker persistence,
  authoritative gate inputs, unknown outcome protocol, hard operator/policy
  failures, and exhausted budgets fail closed. Exhaustion falls through only
  to `finalize`, whose non-clean `error` guard aborts the fleet/compose command
  with a non-zero exit while preserving the findings report and authored
  reason for C1 human review. The B2 checkpoint decides whether the pattern
  graduates into the general fleet-research recipe — tracked as
  `features/_unscheduled/fleet-validate-and-resume.md`.
