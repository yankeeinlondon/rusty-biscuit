---
$schema: feature-log.yaml
feature: 2026-07-14-invalid-frontmatter
deferred_perf_measurement: true
implementation_1: "2026-07-20T23:38:36-07:00"
implementation_2: "2026-07-21T00:45:59-07:00"
implementation_3: "2026-07-21T08:30:12-07:00"
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

## Implementation of Review Findings #2

> **started at:** 2026-07-21T00:45:59-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-14-invalid-frontmatter/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review contains **4** findings:
        - **High** — file-level BOM and lone-CR documents bypass frontmatter analysis
        - **High** — `--json` ships a different wire contract and emits no JSON for unrepaired invalid YAML
        - **High** — the explicit no-regression performance acceptance is still unverified
        - **Medium** — public CLI and library documentation still describes the pre-feature behavior
- affected packages identified by the review and confirmed with `sniff`: `biscuit-file`, `darkmatter`, and `darkmatter-cli`
- affected package areas: `biscuit-file` and `darkmatter`
- findings are being dispatched serially so each subagent can own coherent updates to this log section

### Finding 1 — High: file-level BOM and lone-CR documents bypass frontmatter analysis

- starting the work on 'high-file-level-bom-and-lone-cr-frontmatter-analysis' at 00:48:46
        - loaded the required `darkmatter`, `rust`, and `rust-testing` skills, including Darkmatter's frontmatter model and the repository's Level-1 test conventions
        - began with the required Sniff package-scope discovery and GitNexus impact workflow before editing any production symbol
        - GitNexus rated `extract_frontmatter_block` CRITICAL: 15 direct and 99 total upstream symbols, 2 affected CLI processes, and 11 affected modules; `parse_frontmatter` was also CRITICAL with 21 direct and 205 total upstream symbols, 9 processes, and 20 modules
        - the critical blast radius was surfaced before editing; the change was kept in the shared frontmatter boundary and exercised the DMLS overlay/source-map consumers identified by the impact report
        - replaced LF-only boundary splitting with one byte-span-preserving scanner for LF, CRLF, and lone CR; both the internal parser and public extractor now accept exactly one leading UTF-8 BOM while retaining the existing trimmed, exact `---` delimiter rule
        - `md clean` now removes a recognized file BOM and reconstructs the frontmatter block with canonical LF delimiters and the YAML engine's normalized source before Markdown parsing; the body remains owned by the existing cleanup path
        - added Level-1 extractor/parser tests for BOM, lone CR, source spans, parser/extractor presence parity, and a BOM near-miss proving `---x` is still not accepted as a delimiter
        - added six spawned-process Level-1 cases covering BOM and lone-CR documents through file input, stdin, and `--save`; each proves reserved-indicator repair, BOM removal, and LF normalization
        - Sniff scope discovery and dependency mapping selected `darkmatter`, `darkmatter-cli`, and `dmls` in the Darkmatter package area; `biscuit-file` was not changed, so no biscuit-file gate was added
        - focused Darkmatter parsing verification passed: 28 of 28 frontmatter extractor/parser tests
        - focused CLI verification passed: all 28 tests in `darkmatter-cli --test clean_frontmatter`, including the six new boundary cases
        - affected DMLS verification passed: `cargo check -p dmls --all-targets` and all 21 frontmatter-overlay/source-map tests
        - warnings-denying lint passed for all affected crates: `cargo clippy -p darkmatter -p darkmatter-cli -p dmls --all-targets --color never -- -D warnings`
        - the first combined test invocation was interrupted at the non-interactive command ceiling after compilation produced no failure output; subsequent smaller incremental gates completed successfully, so the slow command was not retried
        - formatting diagnosis could not run because the pinned stable toolchain lacks the `rustfmt` component; no component was installed and no write-mode formatting command was run
        - GitNexus `detect_changes` reports CRITICAL aggregate worktree risk across 31 changed symbols and 18 affected processes; its report includes unrelated pre-existing shared-worktree files, while this finding's production changes are confined to the shared frontmatter parser/extractor and `md clean` repair boundary
- work completed for 'high-file-level-bom-and-lone-cr-frontmatter-analysis' at 00:57:27

### Finding 2 — High: `--json` wire contract and unrepaired invalid YAML envelope

- starting the work on 'high-json-v1-wire-contract-and-invalid-yaml-envelope' at 00:59:20
        - loaded the required `darkmatter`, `rust`, and `rust-testing` skills, plus the CLI-output testing guidance for machine-readable channel contracts
        - confirmed ratified decision D8 requires the exact v1 envelope families: `version`, structured `source`, structured `frontmatter`, document-position `diagnostics`, actually-applied repair audit, and document-level `changed`
        - GitNexus rated every Finding 2 production surface LOW risk: `run_clean` has 2 direct and 3 total upstream symbols across 2 processes; `repair_frontmatter` has 1 direct and 4 total upstream symbols across the same 2 processes; `CleanJsonReport` has 1 direct dependent and no affected process
        - replaced the conflicting four-field CLI report with the ratified version-1 envelope: `version`, structured file/stdin `source`, structured present/absent `frontmatter`, unified diagnostics, `applied`, and document-level `changed`
        - projected diagnostic spans into whole-document byte coordinates with 1-indexed line and byte-column fields; candidate and applied repair spans now use whole-document byte coordinates as well
        - retained the analyzer's edit audits across initial syntax analysis, syntax rescan, and schema repair; the audit also records the document-level UTF-8 BOM removal introduced by Finding 1
        - JSON mode now serializes and flushes a valid v1 envelope before preserving exit code 1 for unrepaired invalid YAML; the spawned-process proof confirms stderr stays empty and the file remains untouched
        - expanded the Level-1 golden contract from 13 to 16 cases, pinning every top-level field, every diagnostic and repair field, enum/code spellings, source/frontmatter variants, empty arrays, ordering, byte columns, BOM coordinates, body-only `changed`, save behavior, and failure-channel behavior
        - Sniff package discovery and dependency mapping selected `darkmatter-cli` in the Darkmatter package area for this CLI-local wire change; no shared biscuit-file or Darkmatter library shape was changed
        - focused Level-1 verification passed: `cargo nextest run -p darkmatter-cli --test clean_frontmatter --test clean_json --test clean_schema --color never` completed all 61 tests successfully, including all 16 JSON contract tests
        - `cargo check -p darkmatter-cli --color never` passed; `git diff --check` also passed
        - the warnings-denying all-target clippy command was terminated at the orchestration checkpoint after 10 seconds with no diagnostic output, rather than allowing a non-interactive gate to run toward the command ceiling; no slow retry was attempted
        - the orchestrator subsequently authorized one bounded incremental retry; `cargo clippy -p darkmatter-cli --all-targets --color never -- -D warnings` completed successfully in 1.32 seconds with no warnings
        - GitNexus `detect_changes` reports CRITICAL aggregate worktree risk across 63 changed symbols and 18 affected processes because it includes Finding 1's shared frontmatter-boundary edits and unrelated pre-existing worktree changes; this finding's own pre-edit impact remains LOW and its production edits are confined to `darkmatter-cli` JSON projection, audit capture, and error-channel handling
- work completed for 'high-json-v1-wire-contract-and-invalid-yaml-envelope' at 01:08:06

### Finding 3 — High: explicit no-regression performance acceptance is unverified

- starting the work on 'high-explicit-no-regression-performance-acceptance' at 01:10:32
        - loaded the required `darkmatter`, `rust`, `rust-testing`, and `sniff` skills, including the Criterion baseline-comparison guidance
        - began by auditing the retained benchmark/deferred-performance records, current worktree isolation, host CPU characteristics, power state, and scheduler load before attempting any timing run
        - Sniff identified an Apple M4 Max with 16 physical and 16 logical cores; the host was on AC power
        - the observed load averages were 81.59 / 53.66 / 39.87 with approximately 923.8% aggregate process CPU, so the host was not quiet enough for a trustworthy comparison
        - the shared feature worktree was dirty with concurrent review-finding edits; branch `darkmatter` was at `7278b1ccbed5` while local `main` was at `4bf0db8813de`, so neither revision could be isolated without including unrelated uncommitted state
        - no Criterion timing was run and no performance number is claimed; a single run under this load would not satisfy the required retained `main`/branch/`main` drift bracket
        - no completed Linux or Windows runtime result for both corrected full-pipeline cases was available in the inspected feature or CI records; workflow configuration and static portability were not treated as runtime evidence
        - `deferred_perf_measurement: true` remains set; full details, exact observed load, isolation blockers, and the required rerun procedure were appended to [`deferred-performance.md`](./deferred-performance.md), explicitly mapped to Review 2 Finding 3
        - bounded structural verification passed: the corrected benchmark compiled in 12.46 seconds, all 8 `clean_counters` tests passed, and bench-target Clippy passed with warnings denied in 28.77 seconds after waiting for the shared build lock
        - no production symbol or test was edited for this finding, so GitNexus symbol impact analysis was not required; edits are limited to the feature log and deferred-performance record
- work completed for 'high-explicit-no-regression-performance-acceptance' at 01:12:32

### Finding 4 — Medium: public CLI and library documentation describes pre-feature behavior

- starting the work on 'medium-public-cli-and-library-invalid-frontmatter-documentation' at 01:13:14
        - loaded the required `darkmatter`, `rust`, `rust-testing`, and `biscuit-file` skills before auditing the public documentation
        - selected `darkmatter/docs/cli/clean.md`, the cleanup section of `darkmatter/cli/README.md`, and the public `biscuit-file` crate guide as the user-facing surfaces named by the review
        - verified every documented claim against `decisions.md`, generated `md clean --help`, the current raw-source repair implementation, and the spawned-process golden tests rather than carrying assumptions forward from the specification
        - documented that deterministic frontmatter repairs are default-on, operate only on the frontmatter block, preserve body YAML fences, and mutate a file only when `--save` is present
        - documented all five relevant flags: `--json`, `--schema`, `--baseline-schema`, `--no-baseline-schema`, and `--no-trigger-schemas`; the guide also makes the absence of `md clean --strict` explicit
        - documented baseline → matching triggers → document `$schema` precedence, `--schema` replacement of the document layer, and stdin's trigger-discovery isolation while explicit schema flags remain active
        - documented the human channel and status contract: report-only findings render on stderr and exit 0, unrepaired invalid YAML exits 1, and JSON mode emits its envelope on stdout with empty stderr before preserving exit 1
        - added the exact version-1 JSON envelope fields and flagship repair example from the Finding 2 golden fixture, including stable classifications/codes, whole-document byte coordinates, one-based byte columns, applied-repair audit, and document-level `changed`
        - expanded `biscuit-file/lib/README.md` with `analyze_yaml`, retained-source `Yaml::analyze`, the duplicate-scan caveat for shorthand views, parse outcomes, all three certainty tiers, and `YamlAnalysis::apply` audit behavior; linked that guide from the package-area README for discovery
        - no Rust symbol or production execution flow was edited, so GitNexus symbol impact analysis was not required; the directly affected package areas remain `biscuit-file` and `darkmatter`
        - the exact Darkmatter invalid-frontmatter Level-1 gate passed: all 61 tests in `clean_frontmatter`, `clean_json`, and `clean_schema`
        - biscuit-file retained-analysis verification passed: all 6 `parse_count` tests and all 19 crate doctests; the doctest set includes the public `Yaml::analyze` example
        - generated CLI help confirmed all five documented flags and no `--strict`; the embedded JSON example parsed successfully, referenced local documentation targets exist, and `git diff --check` passed
        - no additional full-area test or lint gate was started: this finding changes Markdown only, biscuit-file's compiled public examples passed doctests, and warnings-denying `darkmatter-cli` Clippy had already passed during Finding 2 against the implementation documented here
- work completed for 'medium-public-cli-and-library-invalid-frontmatter-documentation' at 01:18:21

### Successful Completion

The implementation of review cycle 2 has completed successfully in 33 minutes and 29 seconds. During this implementation all 4 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 1 was deferred (see reasons below):

- **Explicit no-regression performance acceptance**
        - deferred because the 16-core host was under extreme load (81.59 / 53.66 / 39.87 load averages and approximately 923.8% aggregate process CPU), so Criterion results would not have been legitimate
        - the shared dirty worktree also prevented an isolated retained `main`/branch/`main` bracket
        - no completed Linux or Windows runtime evidence for both corrected full-pipeline cases was available; workflow configuration and static portability were not treated as runtime evidence
        - `deferred_perf_measurement: true` remains set and the full rerun procedure is recorded in [`deferred-performance.md`](./deferred-performance.md)
- final scoped verification passed:
        - Darkmatter frontmatter parser/extractor tests: 28 of 28
        - Darkmatter CLI clean integration tests: 61 of 61
        - DMLS affected frontmatter/source-map tests: 21 of 21
        - biscuit-file retained-analysis tests: 6 of 6
        - biscuit-file doctests: 19 of 19
        - Darkmatter, darkmatter-cli, DMLS, benchmark-target, biscuit-file, and biscuit-file-cli Clippy gates passed with warnings denied
        - the corrected benchmark compiled and all 8 load-independent clean-counter tests passed
- GitNexus `detect_changes` reports CRITICAL aggregate worktree risk across 104 changed indexed symbols and 18 affected processes; the report includes unrelated pre-existing changes, while this cycle's shared production change is the explicitly reviewed frontmatter boundary

The files changed by this implementation are `darkmatter/lib/src/markdown/frontmatter.rs`, `darkmatter/cli/src/commands/clean.rs`, `darkmatter/cli/src/commands/clean/frontmatter_repair.rs`, `darkmatter/cli/tests/clean_frontmatter.rs`, `darkmatter/cli/tests/clean_json.rs`, `darkmatter/docs/cli/clean.md`, `darkmatter/cli/README.md`, `biscuit-file/lib/README.md`, `biscuit-file/README.md`, `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md`, this log, and the Review 2 metadata.

## Implementation of Review Findings #3

> **started at:** 2026-07-21T08:30:12-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'darkmatter/features/2026-07-14-invalid-frontmatter/review-3.md'
- this is iteration 3 of the review-to-implement cycle

### Finding 1 — High: JSON spans stop indexing the authored document after an earlier repair

- starting the work on 'high-json-authored-coordinate-mapping-after-stacked-repairs' at 08:32:28
        - loaded the required `darkmatter`, `rust`, `rust-testing`, and `sniff` skills before tracing the repair coordinate spaces and selecting verification scope
        - Sniff identified the directly affected package areas as `biscuit-file` and `darkmatter`; this finding changes `darkmatter` and `darkmatter-cli`, with `dmls` retained as the downstream consumer to check for the shared span helper
        - GitNexus rated both edited production surfaces LOW risk: `repair_frontmatter` has 1 direct and 4 total upstream symbols across 2 CLI processes; `line_col_of_offset` has 2 direct consumers and no affected production process
        - reproduced the review defect: after quoting `@daily-report`, the later `1.20` schema diagnostic and repair were shifted two bytes past the authored scalar and could extend beyond authored frontmatter
        - added a pass-composing reverse coordinate map that projects every rescan/schema diagnostic, candidate repair, and applied repair through all earlier accepted edit sets before retaining it
        - spans landing wholly in synthetic replacement text conservatively project to the authored range that the replacement superseded; unchanged regions retain exact byte coordinates after cumulative length deltas
        - corrected the shared byte line/column helper to recognize LF, CRLF, and lone CR without splitting CRLF into two line breaks; its behavior documentation was updated with the expanded contract
        - added Level-1 golden coverage for a stacked length-changing syntax repair plus later syntax/schema artifacts, and for schema diagnostics and applied repairs over CRLF and lone-CR authored documents
        - focused verification passed: all 4 `line_col_of_offset` unit tests and all 19 `darkmatter-cli --test clean_json` tests
        - full `biscuit-file` package-area verification passed via `just test`: 624 library tests and 61 CLI tests passed; `just lint` also passed for both crates
        - the full Darkmatter `just test` gate exceeded the non-interactive command ceiling and was interrupted after 2,281 of 5,909 library tests had passed with no failures; it was not retried as another unbounded run
        - complete feature-focused verification passed: all 19 Darkmatter schema-quoting/span-compatibility tests and all 64 `clean_frontmatter`, `clean_json`, and `clean_schema` CLI tests
        - Darkmatter package-area `just lint` passed for `darkmatter`, `darkmatter-cli`, and the `dmls` downstream crate after Clippy identified and a bounded retry removed three redundant iterator clones
        - `git diff --check` passed; no write-mode formatting command was run
        - GitNexus `detect_changes --compare main` reports CRITICAL aggregate shared-worktree risk across 3,719 changed symbols, 682 files, and 77 affected processes; the result includes extensive pre-existing and concurrent work, while this finding's pre-edit symbol impacts were both LOW
- work completed for 'high-json-authored-coordinate-mapping-after-stacked-repairs' at 08:42:52

### Finding 2 — High: Frontmatter delimiters are rewritten outside the accepted edit set

- starting the work on 'high-frontmatter-delimiter-preservation-and-repair-audit' at 08:44:28
        - loaded the required `darkmatter`, `rust`, `rust-testing`, and `sniff` skills before inspecting delimiter reconstruction and selecting the affected verification scope
        - Sniff identified `biscuit-file` and `darkmatter` as the specification's package areas; the changed production path is confined to `darkmatter-cli`, so the exact package selector was used for the complete affected Level-1 suite while the package-area lint retained `darkmatter`, `darkmatter-cli`, and `dmls`
        - GitNexus rated `splice_frontmatter` HIGH risk: 1 direct caller, 4 total upstream symbols, 2 affected CLI processes, and 3 affected modules; the warning was surfaced before editing and the implementation was deliberately constrained to source-slice reconstruction
        - replaced canonical delimiter emission with byte-for-byte copies of the authored opening and closing delimiter slices, preserving surrounding whitespace plus LF, CRLF, or lone-CR delimiter terminators
        - retained YAML-interior normalization as an accepted repair: the line terminator immediately before the closing delimiter remains inside the extracted YAML span, is normalized to LF, and appears in the document-relative `applied` audit
        - updated the `assemble` contract and public `md clean` documentation so they state that delimiter whitespace and terminators are preserved and only accepted YAML edits or ordinary body cleanup may change other bytes
        - added Level-1 file, stdin, and `--save` matrices covering trimmed delimiters with LF, CRLF, and lone CR; each asserts the complete emitted or saved document bytes
        - added a Level-1 JSON matrix for the same three line-ending forms, proving clean LF input has `changed: false` and an empty `applied`, while CRLF/lone-CR YAML-interior normalization has `changed: true`, audits exactly that interior terminator, and never overlaps either delimiter terminator
        - the first focused run exposed three stale lone-CR expectations from Review 2 that still required canonical LF delimiter terminators; only those assertions were updated to the C-4 preservation contract
        - focused verification then passed all 51 tests in `clean_frontmatter` and `clean_json`
        - the complete affected `darkmatter-cli` Level-1 suite passed: 641 tests passed and 71 higher-tier tests were skipped by the canonical filter
        - Darkmatter package-area `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`; `git diff --check` also passed and no write-mode formatting command was run
        - GitNexus `detect_changes --compare main` reports CRITICAL aggregate shared-worktree risk across 3,722 changed symbols, 682 files, and 77 affected processes; this includes extensive pre-existing and concurrent work, while this finding's pre-edit `splice_frontmatter` impact was HIGH and its production edit is limited to delimiter reconstruction
- work completed for 'high-frontmatter-delimiter-preservation-and-repair-audit' at 08:50:26

### Finding 3 — High: Performance and cross-platform acceptance remain explicitly open

- starting the work on 'high-performance-and-cross-platform-acceptance-evidence' at 08:51:00
        - loaded the required `darkmatter`, `rust`, `rust-testing`, and `sniff` skills, including the Criterion performance guidance, before auditing the retained evidence
        - honored the review's explicit `DECISION: DEFERRED` and non-blocking disposition; no Rust symbol or test needed to change for this evidence-only finding
        - Sniff identified macOS 26.5.2 on an Apple M4 Max with 16 physical and 16 logical cores and confirmed the current linked worktree is `darkmatter`
        - host admission failed: the observed load averages were 26.27 / 19.37 / 17.79 with approximately 228.1% aggregate process CPU, which exceeds a quiet 16-core host; AC power and normal power mode were confirmed but do not remove scheduler contention
        - worktree admission also failed: Sniff reported the current feature worktree dirty with 15 changed paths, while `git status` independently showed 13 modified and 2 untracked paths; the current candidate is therefore not an isolated retained revision
        - the registered `/private/tmp/dmbench/{before,base,after}` paths now exist only as non-repository directories, so they cannot supply an auditable main/branch/main bracket; existing `target/criterion` samples likewise lack a retained Review 3 bracket and were not reinterpreted as current evidence
        - no Criterion timing was run and no performance value or no-regression verdict is claimed
        - the bounded, load-independent benchmark check passed: `cargo check -p darkmatter --bench clean_hot_paths --color never` completed successfully, and source inspection confirmed both required `no_frontmatter/full_pipeline` and `clean_frontmatter/full_pipeline` cases remain registered
        - macOS retains successful scoped runtime evidence from Findings 1 and 2, including the full biscuit-file Level-1 suite, feature-focused Darkmatter/CLI suites, and the complete `darkmatter-cli` Level-1 suite
        - no completed Linux or Windows run for the current candidate was found in feature records; the public GitHub Actions API reported zero `darkmatter-tests` runs for branch `darkmatter`, and the uncommitted Review 3 candidate has no remote commit check record
        - workflow configuration was treated only as structural evidence: Darkmatter declares full Level-1 Linux and Windows jobs plus a macOS all-target check, Windows remains soft-fail, and no equivalent three-OS biscuit-file area runtime workflow closes the feature row
        - the combined cross-platform acceptance row remains open because Windows and Linux runtime evidence is absent; no new platform/timing acceptance became green during this finding
        - `deferred_perf_measurement: true` remains set and [`deferred-performance.md`](./deferred-performance.md) now maps Review 3 Finding 3 to the observed blockers, retained evidence, and exact rerun procedure
- work completed for 'high-performance-and-cross-platform-acceptance-evidence' at 08:54:09

### Successful Completion

The implementation of review cycle 3 has completed successfully in 25 minutes and 59 seconds. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 1 was deferred (see reasons below):

- **Performance and cross-platform acceptance remain explicitly open**
        - deferred in accordance with Review 3's explicit non-blocking decision because the 16-core host was not quiet enough for a legitimate Criterion comparison (26.27 / 19.37 / 17.79 load averages and approximately 228.1% aggregate process CPU)
        - the dirty 15-path shared worktree and non-repository `/private/tmp/dmbench` directories prevented an isolated, retained `main`/branch/`main` bracket
        - no completed Linux or Windows runtime evidence exists for the current uncommitted candidate; workflow configuration and static portability were not treated as runtime evidence
        - `deferred_perf_measurement: true` remains set, and the Review 3 evidence plus exact rerun procedure are recorded in [`deferred-performance.md`](./deferred-performance.md)
- final scoped verification passed:
        - biscuit-file package-area Level-1 tests: 624 library and 61 CLI tests passed
        - Darkmatter feature-focused schema/span tests: 19 passed
        - Darkmatter CLI frontmatter-focused tests: 64 passed after Finding 1; the combined delimiter suite passed 51 of 51 after Finding 2
        - complete affected `darkmatter-cli` Level-1 suite: 641 passed, with 71 higher-tier tests skipped by the canonical filter
        - biscuit-file and Darkmatter package-area lint gates passed for `biscuit-file`, `biscuit-file-cli`, `darkmatter`, `darkmatter-cli`, and `dmls`
        - the clean hot-path benchmark target compiled successfully with both required full-pipeline cases registered
        - the full Darkmatter `just test` attempt was interrupted at the non-interactive ceiling after 2,281 of 5,909 library tests passed with zero failures; it was not retried unbounded
- final GitNexus `detect_changes --compare main` reports CRITICAL aggregate shared-worktree risk across 3,722 changed symbols, 682 files, and 77 affected processes; this includes extensive pre-existing changes, while the task-specific pre-edit impact was LOW for Finding 1 and HIGH for the narrowly constrained delimiter reconstruction in Finding 2

The files changed by this implementation are `darkmatter/cli/src/commands/clean.rs`, `darkmatter/cli/src/commands/clean/frontmatter_repair.rs`, `darkmatter/cli/tests/clean_frontmatter.rs`, `darkmatter/cli/tests/clean_json.rs`, `darkmatter/lib/src/markdown/span.rs`, `darkmatter/docs/cli/clean.md`, `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md`, this log, and the Review 3 metadata.
