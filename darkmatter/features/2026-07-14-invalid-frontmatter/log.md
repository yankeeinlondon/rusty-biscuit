---
$schema: feature-log.yaml
feature: 2026-07-14-invalid-frontmatter
deferred_perf_measurement: true
implementation_1: "2026-07-20T23:38:36-07:00"
---

# Log — Invalid Frontmatter

## Implementation of Review Findings #1

> **started at:** 2026-07-18T18:33:32-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-14-invalid-frontmatter/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review contains **5** findings:
        - **Critical** — `md clean` never invokes the invalid-frontmatter pipeline
        - **High** — the focused schema-aware Level-1 gate has six failures
        - **High** — every new user-facing CLI requirement lacks its required Level-1 proof
        - **High** — safety, corpus, performance, and platform acceptance evidence is incomplete
        - **Medium** — the convenience API can accidentally analyze the same source twice
- affected package areas (per the review's sniff analysis): `biscuit-file` and `darkmatter`
- orchestrator survey before dispatch established the actual starting state:
        - the `biscuit-file` analyzer engine is substantially built (`yaml/analyze/` with `engine`, `scan`, `locate`, `recover`, `edit_set`, `report`, `diagnostic`, `analysis` plus 15 unit-test modules)
        - two suites the review reported as absent do in fact exist — `biscuit-file/lib/tests/parse_count.rs` and `biscuit-file/lib/tests/yaml_safety.rs` (they landed in commit `aab72bfb9`, after the review's snapshot)
        - still absent: `yaml_corpus.rs`, `yaml_mutation.rs`, `darkmatter/lib/tests/schema_quoting_safety.rs`, `darkmatter/lib/tests/clean_counters.rs`, `darkmatter/cli/tests/clean_json.rs`, `darkmatter/cli/tests/clean_schema.rs`
        - `darkmatter/cli/src/commands/clean.rs` confirmed to call `load_markdown` first and expose none of the new schema/JSON flags — the Critical finding reproduces
- dispatch order was chosen so each finding builds on the prior one: finding 2 (green the gate) → finding 5 (single-analysis API) → finding 1 (CLI wiring) → finding 3 (CLI Level-1 proofs) → finding 4 (acceptance evidence)

### Finding 2 — focused schema-aware Level-1 gate has six failures

- starting the work on 'high-focused-schema-gate-six-failures' at 18:35:12-07:00
- work completed for 'high-focused-schema-gate-six-failures' at 18:55:09-07:00
        - reproduced first: `cargo test -p darkmatter --lib clean_quoting` → 31 passed, 6 failed (4 setup panics on `unknown type 'integer'`, 2 S1 assertion failures)
        - root cause A — `integer` is not a DMLS type keyword at all; `SimplifiedType::from_keyword` (`simplified/types.rs:314-336`) admits `number`/`numberlike`, and integrality is expressed as `Constraint::Integer` (parsed `simplified/grammar.rs:1310`, lowered `simplified/convert.rs:856`)
        - fixed the four fixtures to the real DMLS spelling `number(integer)` rather than adding an `integer` type keyword — a new type keyword would be a public grammar/meta-schema change made to repair what is a test-authoring typo
        - `number(integer)` preserves each fixture's intent exactly: `count: abc` remains a genuine non-string mismatch (the second problem that trips the "exactly one problem" gate) and `count: "42"` still coerces cleanly
        - root cause B — `schema_result_set_identical` compared only the coercing `validate()` report, which collapses an authored `42` and an authored `"42"` onto the same problem set; the helper was therefore structurally unable to see a type-changing edit and returned `true` for both the regression case and the S2 quoting transition
        - fixed the helper to require identity on **both** the raw (`validate_raw`, non-coercing) and the coerced (`validate`) problem sets
                - raw alone is insufficient: `"42"` and `"abc"` against `number` are raw-identical but coercion-divergent
                - requiring both is the strictly more conservative predicate, which is the correct direction for an auto-apply safety gate
        - no test was weakened — all four S1 assertions now pass on their original text, per the review's instruction to reconcile the helper to the tests and not the reverse
        - removed the unused `std::path::Path` import from the test target
        - doc drift pass per CLAUDE.md authoring discipline: the `///` claimed "uses the shared coercing semantics", which the behavior change invalidated — rewrote it to state both halves and why each is load-bearing, and corrected the S2 sentence to say the transition shrinks the *raw* problem set
        - blast radius: `schema_result_set_identical` is `pub` and re-exported from `schemas/mod.rs:88` but has zero non-test callers workspace-wide
                - **caveat:** the GitNexus `impact` MCP tool was denied in the subagent's session, so this is grep evidence rather than call-graph evidence
        - gates: focused `clean_quoting` **37/37 green**; `just test` (L1) green — darkmatter 5836/5836, darkmatter-cli 561/561, dmls 567/567; `just lint` clean with zero warnings
        - not run: `test-l2`, `test-browser`, `doctest` — this finding is L1-scoped and touches no rendering or terminal surface

### Finding 5 — convenience API can accidentally analyze the same source twice

- starting the work on 'medium-double-analyze-convenience-api' at 18:55:30-07:00
- work completed for 'medium-double-analyze-convenience-api' at 18:59:17-07:00
        - added `Yaml::analyze() -> Option<YamlAnalysis>` as the documented primary diagnostics entry point — one scan, with the caller retaining the analysis and deriving every view from it
        - added `YamlAnalysis::repairs()`, a borrowing iterator over candidate repairs in stable source order
                - this was the actual missing piece: `YamlAnalysis` already exposed `diagnostics()` and `apply()` but had no repairs view, which is *why* `Yaml::repair_candidates()` had to re-scan
                - returning a borrowing iterator means the second view costs no allocation, not merely no re-scan
        - `diagnose()` and `repair_candidates()` retained with unchanged signatures, reimplemented over `analyze()`, and re-documented as single-view shorthand that scans once each
        - design call worth flagging — `analyze()` returns `Option` rather than synthesizing an empty analysis for a `from_value` document
                - `YamlAnalysis::apply()` yields a *patched source* derived from `source()`, so a synthetic analysis over `""` would make `apply().source` empty
                - the exact downstream consumer this is built for (`md clean`, which writes the patched source back to disk) would then truncate the file
                - `Option` makes that case unrepresentable rather than merely unlikely; the cost is one extra line at the call site (`if let Some(analysis) = yaml.analyze()`)
        - steered callers in the `yaml/mod.rs` module rustdoc with a new "Diagnostics and Repairs" section plus a worked `analyze()` example
        - tests: single-analysis views proven byte-equal to both shorthands; `parse_count.rs` extended to assert the shorthand pair costs exactly 2× the retained-analysis path, expressed as a relationship (`analyze_parse_count() == retained * 2`) rather than a magic number so it survives internal parse-count shifts
        - gates: `just test` 605 lib (4 skipped) + 61 CLI passed; `just lint` clean; `just doctest` 19 passed, up from 17 — doctests were run beyond the assigned gates because nextest does not execute them and two new doc examples would otherwise have gone unverified
        - the darkmatter compile check was not required: both additions are new API, the two existing methods kept exact signatures, and a repo-wide grep found no consumers outside biscuit-file's own tests

### Finding 1 — Critical: `md clean` never invokes the invalid-frontmatter pipeline

- starting the work on 'critical-md-clean-not-wired-to-pipeline' at 18:59:40-07:00
- work completed for 'critical-md-clean-not-wired-to-pipeline' at 19:32:47-07:00
        - `run_clean` now reads raw source and repairs frontmatter *before* constructing `Markdown`; `load_markdown` is gone from the clean path entirely
        - ordering landed as: resolve path → reject stdin-under-`--save` before any read → `fs::read_to_string` → `repair_frontmatter` → `Markdown::try_from_content` → `apply_cleanup` on the body → assemble
        - output assembled as verbatim repaired frontmatter block + cleaned body via `block_span`; the assembled shape matches `Markdown::as_string`, so documents needing no repair are unaffected
        - the unrepairable-YAML contract is enforced structurally: parsing fails at `Markdown::try_from_content`, which sits *before* any write, so exit 1 with the file untouched falls out of the ordering rather than needing a guard
- new module `darkmatter/cli/src/commands/clean/frontmatter_repair.rs`
        - retains ONE `YamlAnalysis` and derives both diagnostics and repairs from it, consuming the API added for finding 5
        - only `yaml_span` is ever analyzed or written, so body ` ```yaml ` fences are structurally out of reach rather than merely filtered — this is the spec's hard "Target surface" requirement
        - zero-cost bail on absent or empty frontmatter: no YAML analysis, no schema resolution, no trigger-schema git-root walk
- new flags on `Clean`: `--json`, `--schema`, `--baseline-schema`, `--no-baseline-schema`, `--no-trigger-schemas`
        - schema state reuses the existing `CleanSchemaConfig`/`CleanSchemaContext` surface rather than reimplementing compose's resolution
        - `compose::env_disables_baseline_schema` promoted to `pub(crate)` so `DARKMATTER_NO_BASELINE_SCHEMA` behaves identically to compose
        - resolution is called at most once per run and only behind a non-empty frontmatter block, so the returned context *is* the per-invocation cache the spec asks for
        - `--baseline-schema` conflicts with `--no-baseline-schema`, matching compose
- JSON envelope: `path`, `frontmatter_offset`, `repaired`, `diagnostics[]`
        - each diagnostic reuses biscuit-file's own `Serialize` (`code`/`span`/`classification`/`message`/`repairs[]`, spans as `{start,end}`), keeping the wire format pinned to the shared diagnostic vocabulary instead of a CLI-local copy
        - added a `stage` field (`syntax` | `schema`) because the two tiers analyze *different text* — `syntax` spans index the authored YAML while `schema` spans index the syntax-repaired YAML, so a single offset for both would have been silently wrong
- an idempotency defect was surfaced and fixed by the new test, not by inspection
        - parse-equivalence-gated repairs (whitespace, normalization) cannot be proven while the block is still unparseable — there is no original `Value` to compare against — so run 1 applied only the reserved-indicator fix and run 2 applied the whitespace fixes, which is not a fixed point
        - fixed with a single rescan **gated on the first pass having actually made an edit**, so an already-clean block is still analyzed exactly once and the spec's "reparse only candidates" requirement still holds
- `run_clean` signature changed to `(input, &CleanOptions)` — 12 positional arguments would have tripped `clippy::too_many_arguments`
- added `serde` (derive) to `darkmatter-cli` and recorded it in `darkmatter/docs/dependencies.md` per the drift rule
- new Level-1 target `darkmatter/cli/tests/clean_frontmatter.rs` — 12 tests, all passing
        - flagship proven end to end: `title: @daily-report` → `title: "@daily-report"`, exit 0, across file, stdin, and `--save`
        - also covers idempotency, comment and key-order preservation, no-frontmatter, empty frontmatter, unrepairable-exit-1-untouched, the fenced-body sentinel, the JSON envelope, and schema-flag acceptance plus conflict
- gates: `darkmatter just test` PASS (5836 + 573 + 567), `darkmatter just lint` PASS with zero warnings, `biscuit-file just test` PASS (605 + CLI)
- orchestrator verification: a stale rust-analyzer `E0061` diagnostic appeared after the subagent reported green, so I independently re-ran `cargo build -p darkmatter-cli` and `cargo test -p darkmatter-cli --no-run` — both clean, confirming the diagnostic was mid-edit state and not a real break
- **two items surfaced for Ken's decision (neither blocks the cycle):**
        - `--schema PATH` was mapped to `with_schema_override` (the document-`$schema` layer) rather than baseline semantics; compose has no `--schema`, and the nearest CLI analogue (`md schema validate --schema`) *is* a baseline, so this was a judgment call against an ambiguous spec — a one-line change if baseline was intended
        - `md clean` no longer reserializes frontmatter, so comments, key order, and quote style now survive; this is what the spec's byte-for-byte preservation requires and no existing test depended on the old behavior, but it is a visible change for **every** document with frontmatter, not only broken ones
- limitation: the GitNexus `impact` MCP tool was denied in the subagent's session, so blast radius is grep evidence — `run_clean` has 2 call sites (`commands/mod.rs:77`, `main.rs:150`), both updated, and `Command::Clean` is destructured with `..` so the new fields are additive

### Finding 3 — every new user-facing CLI requirement lacks its Level-1 proof

- starting the work on 'high-cli-requirements-lack-level1-proof' at 19:33:10-07:00
- work completed for 'high-cli-requirements-lack-level1-proof' at 19:55:22-07:00
- all 11 rows of the review's gap matrix (D-1 … D-11) now have Level-1 coverage
        - added `darkmatter/cli/tests/clean_json.rs` — 13 tests closing D-5; pins the envelope field set, the diagnostic and repair shapes, enum spellings, `stage` discrimination, `frontmatter_offset` span projection, deterministic ordering, stdout-as-sole-payload, `--save --json`, stdin null path, and that the failure path emits no envelope
        - added `darkmatter/cli/tests/clean_schema.rs` — 16 tests closing D-10 and D-8; exercises all five schema flags, default-baseline liveness, document `$schema` file-ref/inline/root-union, matching and non-matching triggers, stdin, and `--save`
        - extended `darkmatter/cli/tests/clean_frontmatter.rs` from 12 to 22 tests, closing D-1 … D-4, D-6, D-7, D-9, and D-11, and wiring in the four canonical `baselines/` fixtures
        - Level 1 only, per the review's explicit ruling that no requirement depends on terminal rendering, input encoders, or OS keyboard events
- D-8 was proven by an **observable-work differential** rather than by output inspection
        - `--baseline-schema <missing-file>` makes schema resolution fail loudly, so it acts as a probe for whether that work ran at all
        - the with-frontmatter case is a positive control proving the probe is live (exit 1); absent and empty frontmatter then succeed *only* because the bypass fired before any resolution, including the trigger-schema ancestor walk
        - `clean_schema.rs` is built on the same differential technique throughout
- `--save` + stdin exit-1 was found to be genuinely already proven by the pre-existing `clean.rs::test_clean_save_rejects_stdin`, so it was deliberately not duplicated
- **three genuine implementation bugs were surfaced by writing these tests and were fixed** (none was worked around by weakening a test)
        - report-only diagnostics were emitted **twice, in two different coordinate spaces** — the syntax tier's second pass re-reported every pass-1 finding with a post-repair shifted span while still tagging it `stage: syntax`, whose contract says spans index the YAML exactly as authored; users saw each suggestion printed twice and `--json` consumers had no way to tell which of the two `syntax` spans was authored-relative. Fixed with a restatement filter; the `CleanStage` doc was corrected to match
        - the STDERR suggestions header rendered glued to the first bullet (`frontmatter suggestions (not applied)- yaml.parse…`) because `Prose`/`UnorderedList` emit no trailing newline; `eprint!` → `eprintln!`
        - `--save` over originally-invalid frontmatter printed `✓ No changes (0.0% changed)` **while rewriting the file** — `original` is built from the already-repaired source, so the delta is structurally blind to the frontmatter rewrite. This is exactly the baseline-`Markdown` pretense D-4 forbids and it actively misreported to the user; fixed with a `Frontmatter: repairs applied` notice gated on `repair.repaired`
        - the subagent deliberately did **not** write a test asserting the old "No changes" output, since that would have encoded the bug as contract
- gates (darkmatter area only): `just test` PASS exit 0 — **7015 passed, 0 failed** (darkmatter 5836, darkmatter-cli 612, dmls 567; 214 skipped, 2 flaky-passed-on-retry); `just lint` PASS on all three crates with `clippy --all-targets -- -D warnings`, so the new test files are themselves linted
- testing-hygiene catch worth recording: the subagent's first gate run reported exit 0 but had **actually failed with exit 100** — the 0 was `tail`'s exit code because the output had been piped. It caught this and re-ran unpiped. The library `transclusion` timeout did not reproduce in isolation and was concurrent-load noise in code the task never touched
- orchestrator verification: confirmed the subagent's temporary probe file was removed and `git status` shows only intended files changed
- open item surfaced for a ruling (observed, not fixed): a document combining a reserved-indicator scalar *and* duplicate keys exits 1, because the repair candidate still fails the safety gate's parse proof — while duplicate keys **alone** exit 0. Defensible as the safety gate working as specified, but the asymmetry deserves an explicit decision

### Finding 4 — safety, corpus, performance, and platform acceptance evidence is incomplete

- starting the work on 'high-acceptance-evidence-incomplete' at 19:55:40-07:00
- work completed for 'high-acceptance-evidence-incomplete' at 20:40:16-07:00
- created `biscuit-file/lib/tests/corpus/yaml_corpus.json` — the pinned, fully offline corpus: 20 preserved + 13 repaired cases, each provenance-tagged `monorepo` / `yaml-spec` / `regression`
        - 7 cases are verbatim frontmatter lifted from real `.md` files in this monorepo
        - stored as JSON rather than raw `.yaml` on purpose: this feature is *about* BOM, CRLF, and trailing-space bytes, and no raw `.yaml` file survives a checkout or an editor save with those bytes intact
        - **stated shortfall:** YAML Test Suite cases were authored *to the same failure classes* rather than vendored verbatim, and no suite case IDs are claimed — the IDs cannot be verified offline and inventing them would be false provenance
- created `biscuit-file/lib/tests/yaml_corpus.rs` (11 tests — per-class expectations plus universal no-panic, determinism, idempotency, and untouched-byte reconstruction invariants)
- created `biscuit-file/lib/tests/yaml_mutation.rs` (6 proptest properties, 256 cases each) — mutates valid frontmatter the way agents actually break it and asserts never-corrupt, never-mutate-outside-span, idempotency, and that the gate admits no `Value`-changing edit; added `proptest` to biscuit-file dev-deps
- created `darkmatter/lib/tests/schema_quoting_safety.rs` (15 tests) — proves the raw **and** coerced halves of the gate are *each* load-bearing, with a test that fails if either is deleted
- created `darkmatter/lib/tests/clean_counters.rs` (8 tests) — parse-once, cost-does-not-scale-with-candidate-count, per-run validator caching, zero parses with no frontmatter
- assessed `yaml_safety.rs` and `parse_count.rs` rather than recreating them: both already discharge their acceptance obligations properly and the review listed them as absent only because it predated commit `aab72bfb9`; their real gaps are filled by the new files and **neither existing file was edited**
- **a second genuine bug was found and fixed — a UTF-8 BOM silently disabled all frontmatter repair on multi-key documents**
        - `serde_yaml_ng` reads a BOM as a document boundary, so `﻿` plus *more than one* top-level key is rejected as a multi-document stream; a single-key block happens to parse, which is exactly why unit fixtures never caught it
        - `analyze_yaml` only generates S1 candidates on the parsed branch, so an unparseable BOM document produced **zero** repairs: the mark was not removed, the accompanying CRLF endings were not normalized either, and `md clean` then failed the document outright
        - this is the shape of essentially every Windows-authored file, and it is precisely the cross-platform normalization the spec puts in scope
        - fixed with an additive `bom_recovery` branch in `yaml/analyze/engine.rs` (+36/-1), proved the same way S3 proves reserved-indicator recovery: the stripped source parses and the original did not, so no `Value` comparison is possible or needed
        - strictly additive — it fires only where the code previously produced nothing; regression tests live in `yaml_corpus.rs`
        - **warrants reviewer confirmation:** GitNexus `impact` was denied, so this leans on the review's own recorded blast radius plus the additive nature of the change
- **a further finding was reported but deliberately not fixed:** `schema_result_set_identical` has zero production callers
        - `frontmatter_repair.rs` applies S1 edits *before* schema resolution, so the schema half of the safety gate is currently unreachable even though module docs describe it as active
        - it is provably redundant (`Value` equality implies schema equality), so this is dead code plus doc drift rather than a correctness hole
        - wiring it in would be production change beyond this finding's scope, and the new `schema_quoting_safety.rs` suite pins the contract it encodes either way
- the subagent corrected three of its *own* wrong expectations rather than the implementation — real frontmatter legitimately raises report-only `ambiguous-scalar`; `null` against a non-required `string` is not a type error; `007` is already a string but `0x1F` is a number — with proptest regression seeds for the latter two checked in
- gates: `biscuit-file` test 623 (+4 skipped) and 61 pass, lint pass; `darkmatter` test 5859 + 612 + 567 pass, lint pass
        - the 2 flaky results were `LKFAIL` leak-timeouts on CLI-spawning binaries — the known spurious class in this repo — and passed on retry

### Successful Completion

The implementation of review cycle 1 has completed successfully in 2 hours and 9 minutes. During this implementation all 5 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 5 were fixed, 0 were deferred — however **three pieces of acceptance evidence within finding 4 were deferred**, and are detailed below:

- **Phase 7 benchmark comparison** (finding 4 — "High — Safety, corpus, performance, and platform acceptance evidence is incomplete")
        - deferred because the host CPU load did not permit a legitimate measurement
        - load averages recorded across the task: **87.07 / 128.63 / 117.97** at start, **168.92** at peak, **13.59 / 52.84 / 72.71** at finish, with concurrent agents running throughout
        - a cross-run comparison under that variance would have been noise, so **no number was reported rather than an untrusted one**
        - a valid measurement needs a main-vs-branch drift bracket on a quiet host
        - the durable, load-independent substitute that *did* land is `darkmatter/lib/tests/clean_counters.rs`, which expresses the spec's performance requirements as counter invariants that hold identically on a loaded box
        - full detail recorded in `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md`
- **Linux gate evidence**
        - deferred because `docker run` required an approval that is unavailable in a non-interactive session
        - Docker itself is running and usable, so this is blocked by permission rather than by capability and is achievable in a next session
- **Windows gate evidence**
        - deferred because no Windows host is available and cross-compilation has previously been blocked in this repo
        - static platform-correctness review was performed as a partial substitute and found no Unix-only assumptions, no hardcoded `/tmp`, no `cfg(unix)`, and no path-separator literals in either the feature code or the new tests; the corpus loads via `Path::join` from `CARGO_MANIFEST_DIR`, and JSON escaping makes the fixtures immune to `core.autocrlf`
        - this static evidence is **explicitly not a substitute** for a real Windows run

Two implementation defects were found and fixed *by* the new test suites rather than by inspection — the BOM repair failure and the doubled report-only diagnostics — along with two further user-visible reporting bugs in `md clean`. Three items are recorded above as needing Ken's ruling: the `--schema` flag's layer semantics, the reserved-indicator-plus-duplicate-keys exit-code asymmetry, and the now-unreachable `schema_result_set_identical` helper.

The files changed across this cycle span the `biscuit-file` and `darkmatter` package areas: the YAML analyzer engine and its convenience API in `biscuit-file/lib/src/yaml/`, the `md clean` command and its new `frontmatter_repair` module plus schema flags in `darkmatter/cli/src/`, the schema-aware clean layer in `darkmatter/lib/src/markdown/schemas/`, and eight new or extended test targets across both areas.

## Implementation of Review Findings #1

> **started at:** 2026-07-20T23:38:36-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-14-invalid-frontmatter/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review contains **5** findings:
        - **Critical** — `md clean` never invokes the invalid-frontmatter pipeline
        - **High** — the focused schema-aware Level-1 gate has six failures
        - **High** — every new user-facing CLI requirement lacks its required Level-1 proof
        - **High** — safety, corpus, performance, and platform acceptance evidence is incomplete
        - **Medium** — the convenience API can accidentally analyze the same source twice
- affected package areas established from the specification, review, and `sniff`: `biscuit-file` and `darkmatter`
- the worktree already contains unrelated user changes; each subagent was instructed to preserve them and limit edits to its assigned finding

### Finding 1 — Critical: `md clean` never invokes the invalid-frontmatter pipeline

- starting the work on 'critical-md-clean-invalid-frontmatter-pipeline' at 23:41:22
        - the review finding is stale against the current branch: `run_clean` reads raw source, runs `repair_frontmatter`, constructs `Markdown` only after repair, cleans the body, and reassembles repaired frontmatter with the cleaned body
        - `CleanOptions` and `CleanSchemaFlags` already expose `--json`, `--schema`, `--baseline-schema`, `--no-baseline-schema`, and `--no-trigger-schemas`
        - the raw-source pipeline structurally limits analysis to `yaml_span`, bypasses absent or empty frontmatter before YAML/schema work, retains one `YamlAnalysis`, and runs schema resolution only after syntax repair restores parseability
        - existing Level-1 targets `clean_frontmatter.rs`, `clean_json.rs`, and `clean_schema.rs` cover the flagship repair, file/stdin/save paths, idempotency, fenced-body isolation, diagnostics, JSON, schema precedence, and lazy bypass contracts
        - GitNexus reports LOW risk for `run_clean`: 2 direct callers, 3 total upstream symbols, 2 affected process groups, and no package-area fan-out beyond Darkmatter
        - the full Darkmatter `just test` gate exceeded the non-interactive command ceiling and was stopped after 2,530 of 5,905 library tests passed with no failures; the interruption is a bounded gate limitation, not a test failure
        - focused Level-1 verification passed: `cargo nextest run -p darkmatter-cli --test clean_frontmatter --test clean_json --test clean_schema` completed 51 of 51 tests successfully
        - the full Darkmatter `just lint` gate likewise exceeded the command ceiling; the directly affected `darkmatter-cli` target passed `cargo clippy -p darkmatter-cli --all-targets -- -D warnings`
        - GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup edits; this finding changed no Rust symbol and only adds this log entry
- work completed for 'critical-md-clean-invalid-frontmatter-pipeline' at 23:46:27

### Finding 2 — High: focused schema-aware Level-1 gate has six failures

- starting the work on 'high-focused-schema-aware-level-1-gate' at 23:48:44
        - the review finding is stale against the current branch: all four fixtures already use the supported SimplifiedSchema spelling `number(integer)`, the unused `std::path::Path` import is absent, and the S1 helper compares both raw and coercing validation result sets
        - the existing 37-test Level-1 target fully covers multiple-problem suppression, coercion, root unions, determinism, type-changing S1 rejection, the intentional S2 transition, and unparseable candidates; no Rust or test edit was necessary
        - GitNexus reports LOW risk for `schema_result_set_identical`: 0 direct callers, 0 affected processes, and 0 affected modules
        - the exact focused gate passed: `cargo nextest run -p darkmatter -E 'test(/clean_quoting/)' --color never` completed 37 of 37 tests successfully
        - the full Darkmatter `just test` gate exceeded the non-interactive command ceiling and was stopped after 1,917 of 5,905 library tests passed with no failures; the interruption is a bounded gate limitation, not a test failure
        - the Darkmatter `just lint` gate passed for `darkmatter`, `darkmatter-cli`, and `dmls` with no warnings
        - GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup edits; this finding changed no Rust symbol and only adds this log entry
- work completed for 'high-focused-schema-aware-level-1-gate' at 23:51:26

### Finding 3 — High: user-facing CLI requirements lack Level-1 proof

- starting the work on 'high-user-facing-cli-level-1-proof' at 23:52:59
        - the review finding is mostly stale against the current branch: dedicated Level-1 targets `clean_frontmatter.rs`, `clean_json.rs`, and `clean_schema.rs` now exercise the spawned `md` process across D-1 through D-11
        - the audit found one narrow D-2 omission: stdin repair and explicit schema flags were covered, but no process test placed stdin below a Git root with a matching trigger schema to prove repository trigger discovery remains inert without a document path
        - added `test_stdin_does_not_discover_repository_trigger_schemas`, with a file-input positive control proving the trigger fixture is active and a stdin assertion proving the same source remains unquoted even when the process working directory is inside that repository
        - GitNexus reports LOW risk for the exercised `run_clean` path: 2 direct callers, 3 total upstream symbols, 2 affected process groups, and 2 affected modules
        - Sniff confirms the directly affected package is `darkmatter-cli` in the `darkmatter` package area; no downstream production package is affected by the test-only addition
        - focused Level-1 verification passed: `cargo nextest run -p darkmatter-cli --test clean_frontmatter --test clean_json --test clean_schema --color never` completed 52 of 52 tests successfully
        - the full Darkmatter `just test` gate exceeded the non-interactive command ceiling and was interrupted after 2,586 of 5,905 tests passed and 140 were skipped, with no failures before the interruption; the interruption is a bounded gate limitation, not a test failure
        - package-specific lint passed: `cargo clippy -p darkmatter-cli --all-targets --color never -- -D warnings`
        - the full Darkmatter `just lint` gate passed for `darkmatter`, `darkmatter-cli`, and `dmls` with no warnings
        - GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup edits; this finding adds only one Level-1 `darkmatter-cli` test and these log entries, with no production-symbol or execution-flow change
- work completed for 'high-user-facing-cli-level-1-proof' at 23:56:28

### Finding 4 — High: safety, corpus, performance, and platform acceptance evidence is incomplete

- starting the work on 'high-safety-corpus-performance-platform-evidence' at 23:58:04
        - the six targets named by the review now exist: `yaml_corpus.rs`, `yaml_mutation.rs`, `yaml_safety.rs`, `parse_count.rs`, `schema_quoting_safety.rs`, and `clean_counters.rs`; the review predates their addition
        - the safety, mutation, schema, parse-count, and clean-counter suites already cover no-panic behavior, deterministic and idempotent repair, untouched-byte reconstruction, S1/S2/S3 safety proofs, parse-count bounds, zero no-frontmatter work, and per-run schema caching
        - the corpus still had a genuine acceptance gap: its `yaml-spec` cases were locally authored analogues with no upstream IDs or commit, despite the acceptance matrix requiring a vendored, SHA-pinned YAML Test Suite subset
        - added seven exact inputs from the official YAML Test Suite `data-2022-01-17` release at commit `6e6c296ae9c9d2d5c4134b4b64d01b29ac19ff6f`, covering valid, expected-failure, duplicate-key, anchor/alias, flow, scalar, and multi-document categories
        - added `YAML-TEST-SUITE-NOTICE.md` with release paths and the upstream MIT license; the corpus test pins repository, release, commit, license, IDs, paths, categories, parse/diagnostic expectations, byte preservation, and all existing universal invariants
        - the released upstream suite contains no BOM case; BOM coverage remains explicit in the corpus's separately attributed regression and real-monorepo cases, including BOM plus CRLF and multi-key frontmatter
        - upstream case `2JQS` is syntactically accepted by the event-oriented YAML Test Suite, while `serde_yaml_ng::Value` rejects its duplicate empty key and the analyzer reports `yaml.parse`; the corpus records that real loader boundary without relabeling the upstream duplicate-key category
        - GitNexus reports LOW risk for the benchmark-only `clean_pipeline` helper: 1 direct caller, 0 affected processes, and 1 test module; the corpus test struct has 0 upstream dependents
        - corrected `clean_hot_paths/full_pipeline` so the candidate benchmark exercises the shipped raw-frontmatter extraction, source-first YAML analysis, default schema analysis, Markdown cleanup, and raw-preserving assembly path; previously it measured only the legacy Markdown cleanup sequence and could not observe feature overhead
        - no Phase 7 timing was reported: a valid result still requires a quiet-host `main`/branch/`main` drift bracket, while this shared dirty worktree had concurrent agent activity and cannot provide an isolated or legitimate comparison; `deferred_perf_measurement: true` remains set and full follow-up detail was appended to `deferred-performance.md`
        - platform evidence is partial and explicitly labeled: these focused gates ran successfully on the macOS host, and the changed corpus/benchmark code uses `Path::join`, escaped fixture bytes, and no platform-specific APIs; no Linux or Windows execution was available in this session
        - existing CI configuration gives Darkmatter full Level-1 Linux/Windows jobs and a macOS all-targets check, but no run result was inspected and Windows is currently soft-fail; biscuit-file has no equivalent three-OS area workflow, so neither configuration nor static review is claimed as Linux/Windows acceptance evidence
        - Sniff confirmed the affected package-area scope remains `biscuit-file` and `darkmatter`; no L2/L3 behavior is involved
        - focused biscuit-file Level-1 verification passed: `cargo nextest run -p biscuit-file --test yaml_corpus --test yaml_mutation --test yaml_safety --test parse_count --color never` completed 30 of 30 tests successfully
        - focused Darkmatter Level-1 verification passed: `cargo nextest run -p darkmatter --test schema_quoting_safety --test clean_counters --color never` completed 23 of 23 tests successfully
        - the corrected benchmark harness passed `cargo check -p darkmatter --bench clean_hot_paths --color never`
        - focused warnings-denying lint passed for all changed targets: biscuit-file's four named tests and Darkmatter's two named tests plus `clean_hot_paths`
        - GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup production edits; this finding changes only acceptance tests, corpus data/notice, the benchmark-only helper, and feature records, and adds no affected production execution process
- work completed for 'high-safety-corpus-performance-platform-evidence' at 00:07:55

### Finding 5 — Medium: convenience API can analyze the same source twice

- starting the work on 'medium-single-analysis-convenience-api' at 00:09:48
        - the review finding is stale against the current branch: `Yaml::analyze()` already returns one retained `YamlAnalysis`, and its rustdoc identifies that method as the preferred entry point whenever diagnostics, candidate repairs, or patched source are needed together
        - module-level docs and `YamlAnalysis::repairs` rustdoc reinforce the single-analysis contract; `Yaml::diagnose()` and `Yaml::repair_candidates()` remain convenient single-view shorthands and explicitly document that pairing them scans the same source twice
        - the production `md clean` integration already follows the intended path: it retains one `YamlAnalysis`, derives syntax diagnostics and the repaired source from it, and only rescans when a changed, newly parseable source unlocks additional repairs rather than reanalyzing the same source
        - existing Level-1 parse-count coverage proves the retained analysis serves diagnostics, repairs, and apply from one scan, while the shorthand pair performs twice the analyzer parse work; the unit suite also proves the retained and shorthand views are result-equivalent
        - no Rust or test edit was necessary because commit `24949be46` already implemented the documented single-analysis API and its focused coverage; this finding adds only these log entries
        - GitNexus reports MEDIUM risk for `Yaml::analyze` with 5 direct and 8 total upstream symbols, 0 affected execution processes, and 1 affected module; `Yaml::diagnose` and `Yaml::repair_candidates` are LOW risk with 3 and 2 direct upstream symbols respectively
        - Sniff confirmed `biscuit-file` as a package area in the repository catalog; its downstream dependency command could not complete because Sniff attempted to open the stale non-repository path `/private/tmp/dmbench/after`, so the zero-process GitNexus result was used to keep verification scoped to the unchanged biscuit-file API
        - focused Level-1 parse-count verification passed: `cargo nextest run -p biscuit-file --test parse_count --color never` completed 6 of 6 tests successfully
        - focused Level-1 convenience API verification passed: `cargo nextest run -p biscuit-file -E 'test(/yaml::tests::diagnose/)' --color never` completed 7 of 7 tests successfully
        - the full biscuit-file area `just test` gate passed: 624 biscuit-file tests and 61 biscuit-file-cli tests succeeded, with 4 library tests skipped by their existing gates
        - the full biscuit-file area `just lint` gate passed for both `biscuit-file` and `biscuit-file-cli` with no warnings
        - GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup and earlier review-finding edits; this finding changed no Rust symbol or execution flow and adds only this log entry
- work completed for 'medium-single-analysis-convenience-api' at 00:12:34

### Successful Completion

The implementation of review cycle 1 has completed successfully in 36 minutes and 30 seconds. During this implementation all 5 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 5 were fixed, 0 findings were deferred. Three acceptance-evidence items within finding 4 remain deferred (see reasons below):

- **Phase 7 benchmark comparison**
        - deferred because a trustworthy result requires a quiet-host `main`/branch/`main` drift bracket, while this session used a shared dirty worktree with concurrent agent activity
        - the benchmark vehicle was corrected to exercise the shipped raw-frontmatter analysis, schema analysis, Markdown cleanup, and raw-preserving assembly path
        - `deferred_perf_measurement: true` remains set, and full follow-up detail is recorded in [`deferred-performance.md`](./deferred-performance.md)
- **Linux runtime evidence**
        - deferred because no Linux host or completed Linux CI result was available in this session
        - static portability review and CI configuration were treated only as partial evidence, not as a runtime substitute
- **Windows runtime evidence**
        - deferred because no Windows host or completed Windows CI result was available in this session
        - the existing soft-fail Windows CI configuration was not treated as acceptance evidence
- final scoped Level-1 verification passed:
        - biscuit-file safety, corpus, mutation, and parse-count targets: 30 of 30 tests
        - Darkmatter schema-quoting and clean-counter targets: 23 of 23 tests
        - Darkmatter focused schema-cleaning target: 37 of 37 tests
        - `darkmatter-cli` invalid-frontmatter, JSON, and schema targets: 52 of 52 tests
        - biscuit-file `just test`: 624 library and 61 CLI tests passed, with 4 existing gated skips
        - biscuit-file `just lint` passed for both crates; Darkmatter `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`
        - the full Darkmatter `just test` recipe could not finish inside the non-interactive per-command ceiling; repeated bounded runs observed up to 2,586 of 5,905 tests passing with no failure before interruption
- GitNexus `detect_changes` reports HIGH aggregate worktree risk from unrelated fixed-width cleanup production edits already present in the shared worktree; this review implementation changes no production execution process

The files changed by this implementation are the pinned corpus and its tests/notice under `biscuit-file/lib/tests/`, the stdin trigger-schema Level-1 proof in `darkmatter/cli/tests/clean_schema.rs`, the feature-path benchmark in `darkmatter/lib/benches/clean_hot_paths.rs`, this log and its deferred-performance record, and the review metadata.
