---
title: Research metadata pipeline fixture matrix
date: 2026-09-17
status: phase-1-draft
---

# Fixture Matrix

Maps each of the specification's 37 acceptance criteria to named fixtures,
the test boundary that proves it, and the phase that lands it. Every row is L1
in the [rust-testing](../../../.claude/skills/rust-testing/SKILL.md) taxonomy:
hermetic in-process, filesystem, and fake-subprocess tests. No row needs a
terminal, browser, network, credential, or live recipient; none needs an L2/L3
tier. Phase 8's checkpoint links each row to passing evidence.

## Fixture roots

| Root | Owner | Contents |
|---|---|---|
| `messenger/lib/tests/fixtures/research/contract/` | Phase 2 | Positive documents, roster, mappings; one fixture per required representation |
| `messenger/lib/tests/fixtures/research/negative/` | Phase 2 | One defect per file; file stem names the expected diagnostic code |
| `messenger/lib/tests/fixtures/research/interaction/` | Phase 2 | Inbound, question, and form records plus sanitized answer payloads |
| `messenger/lib/tests/fixtures/research/diagnostics/` | Phase 2 | Sanitized envelopes, signatures, near misses |
| `messenger/lib/tests/fixtures/research/lifecycle/` | Phase 4/6 | Before/after snapshots, candidates, approvals, publication trees |
| `claudine/cli/tests/fixtures/sequence-budget/` | Phase 5 | Fake-provider scripts, ledgers, sequence documents |

Test entry points (names fixed in Phase 3–6, one per boundary):

| Test | Boundary |
|---|---|
| `messenger/lib/src/research/**/tests.rs` | Table-driven semantic rules over the corpus (unit) |
| `messenger/lib/tests/research_corpus.rs` | Passive corpus: every fixture and every shipped `docs/research/platforms/*.md` through the real loader |
| `messenger/lib/tests/research_publication.rs` | Snapshot writer with fault injection in fixture-owned directories |
| `messenger/lib/tests/research_lifecycle.rs` | Selection, delta, approval, promotion, retention with fake agent output |
| `messenger/cli/tests/research_cli.rs` | Real `messenger research …` binary against fixture trees (exit codes, JSON, drift) |
| `claudine/cli/tests/sequence_budget.rs` | Budgeted sequence runs with fake providers and fixture-owned home/PATH |

## Criteria

| # | Criterion (short) | Fixtures | Proof boundary | Phase |
|---|---|---|---|---|
| 1 | Roster: 5 platforms, 7 adapters, companions distinct, paused/excluded explicit | `contract/roster.yaml`; `negative/roster-missing-adapter`, `roster-companion-as-adapter`, `roster-email-active` | unit + corpus over shipped `docs/platforms.yaml` | 2, 3 |
| 2 | Every active document passes schema + semantic validation with prose, citations, gaps, changes | shipped documents | `research_corpus.rs` + `messenger research validate` e2e | 3 (harness), 7 (content) |
| 3 | Every emitted text surface has a bound, a supported no-bound claim, or an actionable gap | `negative/surface-uncovered`, `surface-empty-array-as-unlimited` | unit (coverage engine) + corpus | 3 |
| 4 | Field, recommendation, aggregate, byte, count, conditional bridge rule — no executable logic | `contract/constraints-{field,recommended,aggregate,bytes,count,bridge-conditional}` | unit | 2, 3 |
| 5 | UTF-8 / scalars / UTF-16 / combining / parsed vs markup; ambiguity non-executable | `contract/units-*`; `negative/unit-unspecified-executable` | unit (eligibility) | 2, 3 |
| 6 | Reject missing interface, duplicate fact, broken source ref, unknown key/enum, state/value mismatch, incompatible aggregate, ambiguous applicability, stale override | `negative/*` (one file each) | schema via Darkmatter library + unit semantic pass | 2, 3 |
| 7 | Byte-identical double generation; `--check` drift; invalid/interrupted generation preserves prior catalog | `lifecycle/generate-*` | `research_cli.rs` (two real runs, drift) + `research_publication.rs` (fault injection) | 4 |
| 8 | Fleet lifecycle: missing, unchanged/unverified, malformed, contradictory, bounded recovery, skip current, resume | `lifecycle/refresh-*` with fake agent outputs | `research_lifecycle.rs` | 6 |
| 9 | Changed bound → fact diff + implementation gap, no delivery change | `lifecycle/delta-raised-limit` | lifecycle + assert `CapabilitySet` unchanged | 4, 6 |
| 10 | Catalog/report retain unknowns, conflicts, freshness, provenance; unenforceable fails executable gate | `contract/catalog-mixed-states` | unit (projection) + CLI JSON | 4 |
| 11 | Summary + skill projection agree with metadata; truncation handoff lists unresolved questions | `lifecycle/publish-consistency` | `research_publication.rs` manifest agreement check | 4, 8 |
| 12 | Every interface: envelope + error inventory or gap; content-limit errors resolve to constraints | `contract/errors-*`; `negative/error-dangling-constraint` | unit | 2, 3 |
| 13 | HTTP-200 app failure, plain text, nested fields, local validation, success warning, overlap, unknown code, ambiguous timeout, changed envelope | `diagnostics/*` | unit signature replay | 2, 3 |
| 14 | No secrets/message content; sanitized, bounded external text; remediation only when matched | `diagnostics/*`; secret/control-sequence scanner over all fixtures | corpus scan test | 2, 3 |
| 15 | Surface → format profile + binding; mode switch vs separate fields vs fallback vs entities vs exclusive; unsupported constructs stay unsupported | `contract/bindings-*`; `negative/markdown-family-implies-support` | unit | 2, 3 |
| 16 | Image-role matrix; inline vs media, shared budgets, ordered vs provider-selected, caption vs alt, previews, exact vs approximate; unimplemented stays visible | `contract/images-*`; `negative/image-role-missing`, `provider-placement-as-caller` | unit + report model | 2, 3, 4 |
| 17 | Attribution: platform-derived, config default, conditional override, content-author label | `contract/attribution-*`; `negative/label-implies-identity` | unit | 2, 3 |
| 18 | Location: auto author, optional coordinates, shared place, live, text fallback; report answers who/when/separate message | `contract/location-*`; `negative/place-as-author` | unit + report model | 2, 3, 4 |
| 19 | Expression: API vs app-only, reactions, emoji/stickers, automatic effects; all-unsupported valid | `contract/expression-*`; `negative/app-only-as-api` | unit | 2, 3 |
| 20 | Inbound scope, question kinds, form packaging per integration; no inheritance from platform name | `interaction/send-only`, `callback-only`, `conditional-visibility`, `companion-combo` | unit | 2, 3 |
| 21 | Questions: yes/no vs cancel, single vs multi, stale option IDs, correlation, anonymous aggregate, callback vs link button | `interaction/question-*` | unit answer normalization over sanitized payloads | 2, 3 |
| 22 | Forms: atomic multi-field, independent controls, sequential, external; absent/empty/canceled; deadlines | `interaction/form-*` | unit with fake transport payloads | 2, 3 |
| 23 | `just test` / `just lint` in messenger + feature matrix; claudine package checks | — | recipes + feature matrix below | 3–8 |
| 24 | Discovery gets identification + questions/schema only; reconciliation gets discovery + curated + prior; suggestions not authoritative | `sequence-budget/discovery-isolation` | `sequence_budget.rs` input-marker assertions + `research_lifecycle.rs` suggestion handling | 5, 6 |
| 25 | Stable vs preview, API vs SDK/bridge, unknown dates, unversioned vs unresearched; chronology preserved | `contract/versions-*`; `lifecycle/chronology-preserved` | unit + lifecycle | 2, 3, 6 |
| 26 | Delta after validation, before promotion; fixed flags; initial baseline; same-URL evidence change; prose-only change; agent vs mechanical separated | `lifecycle/delta-*` | `research_lifecycle.rs` | 4, 6 |
| 27 | Curated cap 10 across interfaces; contributions explained; replacement at capacity; human approval | `contract/roster-cap-10`; `negative/roster-cap-11`, `retained-without-contribution` | unit + lifecycle | 2, 3, 6 |
| 28 | Initial baseline rejects missing categories, incomplete handoffs, placeholder unknowns; complete gap record; three passes done | `negative/gap-placeholder`, `gap-missing-searches`; `lifecycle/baseline-incomplete-pass` | unit + lifecycle | 2, 3, 6 |
| 29 | Automatic acceptance only for verified unchanged renewal | `lifecycle/renewal-{unchanged,timestamp-only,failed-check,source-added,prose-changed}` | lifecycle approval policy | 6 |
| 30 | Partial refresh publishes successes + compatible prior docs with real dates; incompatible/missing blocks publication | `lifecycle/partial-*` | `research_publication.rs` + lifecycle | 4, 6 |
| 31 | CHANGELOG summaries vs separate review artifacts; no candidate/rejected entries; no no-change entries; no transcripts | `lifecycle/publish-changelog-*` | lifecycle | 6 |
| 32 | Assessment reuse on fingerprint match; revision provenance; unrelated commit keeps; input change → `unassessed`; proposed not accepted | `contract/mappings-*`; `lifecycle/fingerprint-*` | unit + lifecycle | 3, 6 |
| 33 | Missing limits rejected; every invocation counted; one platform at a time; stop at exhaustion; no reset | `sequence-budget/*`; `negative/run-config-missing-limit` | `sequence_budget.rs` + messenger run-config unit test | 5, 6 |
| 34 | Public vs approval-required access; renewal preserves prose; interrupted publication leaves consistent snapshot | `lifecycle/access-*`, `renewal-prose-preserved`; publication fault points | lifecycle + publication | 4, 6 |
| 35 | Retention: durable accepted evidence; local records; exact preview at 30 days; resumability disclosed; protections; no unattended deletion/Git | `lifecycle/cleanup-*` | lifecycle + CLI dry-run e2e | 6 |
| 36 | Every curated source has an attempt record; inaccessible ≠ rechecked; failed check blocks renewal; exhaustion incomplete | `lifecycle/source-access-*` | lifecycle | 6 |
| 37 | Active clock charges; suspension does not; retry/restart/crash preserve consumption; extra budget recorded | `sequence-budget/{charge-*,suspend,restart,crash,grant}` | `sequence_budget.rs` | 5 |

## Feature combinations beyond `just test`

`just test` and `just lint` in `messenger/` build the library with only
`desktop`; `messenger-cli` enables every chat provider. They therefore do not
compile the library's research code or its chat-provider tests on their own.
These explicit local checks are added to phase checkpoints; none becomes a new
CI cell, because CI already runs the library with `all-features`
(`[package.metadata.ci.tests] all-features = true`), which will include
`research` once it exists.

| Check | Question it answers |
|---|---|
| `cargo nextest run -p messenger --features research` | Research module and corpus tests pass without chat providers |
| `cargo nextest run -p messenger --all-features` | Research coexists with all five chat providers and desktop (matches CI) |
| `cargo clippy -p messenger --all-features --all-targets -- -D warnings` | Lint coverage for code the local `lint` recipe skips |
| `cargo tree -p messenger -e normal` with `--no-default-features`, default features, and `--features desktop` contains no `darkmatter`, `biscuit-file`, or YAML crate | Ordinary send builds stay free of maintenance dependencies (Phase 3 checkpoint). Baseline on 2026-09-17: all three are clean; `--all-features` already reaches `biscuit-file` and two YAML crates through `sniff` → `schematic-*`, so it cannot serve as this check |
| `just test` / `just lint` in `claudine/` | Budget extension (Phase 5) |
