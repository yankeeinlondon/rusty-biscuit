---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
created: 2026-09-21
plan_phase: 1
generator: baseline/consumer-sweep.py
---

# Active `--test` consumer sweep (before any migration)

Every `--test <name>` selector in git-tracked text files whose `<name>` is a
current test target of `claudine-cli`, `darkmatter`, `darkmatter-cli`, or
`biscuit-terminal`. A line that names another package with `-p`/`--package`
is dropped. A bare name that another workspace package also uses is kept
and marked **ambiguous** for the package phase to confirm.

Classification rules, first match wins:

- `(^|/)(features|fixes)/_complete(d)?/` → **historical** (completed spec record); never rewritten
- `(^|/)reviews?/` → **historical** (review record); never rewritten
- `(^|/)(implementation-log|review-log|CHANGELOG)[^/]*\.md$` → **historical** (dated log); never rewritten
- `(^|/)(features|fixes)/[^/]+/(review-\d+|log|results|deferred-[^/]*|phase\d+-[^/]*)\.md$` → **historical** (spec-directory record (log, review round, results)); never rewritten
- `(^|/)(features|fixes)/_unscheduled/` → **historical** (unscheduled spec (not active guidance; revisit when scheduled)); never rewritten
- this feature's own directory → **self**
- other dated `features/`/`fixes/` directories → **in-flight-spec**; reviewed individually in Phase 7, and rewritten only if the spec is still unimplemented guidance
- everything else (recipes, skills, docs, prompts, source comments) → **active**; the Phase 3–7 worklist

| Class | Hits | Files |
|---|---:|---:|
| active | 30 | 23 |
| in-flight-spec | 8 | 6 |
| self | 0 | 0 |
| historical | 546 | 225 |

## active (30)

| File:line | Target | Package | Why | Line |
|---|---|---|---|---|
| `.claude/skills/claudine/completions/shell-completions.md:779` | `completion_perf` | claudine-cli | agent skill | `cargo test -p claudine-cli --test completion_perf --release \` |
| `.claude/skills/darkmatter/errors.md:114` | `error_snapshots` | darkmatter | agent skill | `INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots` |
| `.claude/skills/darkmatter/errors.md:131` | `error_snapshots` | darkmatter | agent skill | `5. Run `INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots`` |
| `.claude/skills/rust-testing/SKILL.md:565` | `context_command` | claudine-cli | agent skill | `recipe's `*args` (`just test-cli --test context_command`): it leaves the` |
| `biscuit-terminal/lib/tests/inline_content_matrix.rs:4` | `inline_content_matrix` | biscuit-terminal | source comment or doc in a test/source file | `//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test inline_content_matrix`` |
| `biscuit-terminal/lib/tests/layout_matrix.rs:4` | `layout_matrix` | biscuit-terminal,darkmatter | source comment or doc in a test/source file | `//! `INSTA_UPDATE=always cargo test -p biscuit-terminal --test layout_matrix`` |
| `biscuit-terminal/lib/tests/render_comparison.rs:13` | `render_comparison` | biscuit-terminal,darkmatter | source comment or doc in a test/source file | `//! `RECORD_DRIFT=1 cargo test -p biscuit-terminal --test render_comparison -- --nocapture`` |
| `claudine/cli/tests/completion_perf.rs:22` | `completion_perf` | claudine-cli | source comment or doc in a test/source file | `//! cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture` |
| `claudine/cli/tests/compose_ttff_perf.rs:28` | `compose_ttff_perf` | claudine-cli | source comment or doc in a test/source file | `//! `cargo test -p claudine-cli --test compose_ttff_perf -- --ignored`.` |
| `claudine/cli/tests/dispatch_inventory.rs:32` | `dispatch_inventory` | claudine-cli | source comment or doc in a test/source file | `//! CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory` |
| `claudine/cli/tests/dispatch_inventory.rs:94` | `dispatch_inventory` | claudine-cli | **string literal in test code** (R9) | `"CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory";` |
| `claudine/cli/tests/real_opencode_yolo_subagent.rs:37` | `real_opencode_yolo_subagent` | claudine-cli | source comment or doc in a test/source file | `//!   cargo test -p claudine-cli --test real_opencode_yolo_subagent -- --nocapture` |
| `claudine/cli/tests/shipped_prompt_route_drift.rs:50` | `shipped_prompt_route_drift` | claudine-cli | source comment or doc in a test/source file | `//! CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1 cargo nextest run -p claudine-cli --test shipped_prompt_route_drift` |
| `claudine/cli/tests/shipped_prompt_route_drift.rs:150` | `shipped_prompt_route_drift` | claudine-cli | **string literal in test code** (R9) | `--test shipped_prompt_route_drift"` |
| `claudine/docs/topics/completions/shell-completions.md:762` | `completion_perf` | claudine-cli | documentation | `cargo test -p claudine-cli --test completion_perf --release \` |
| `claudine/docs/topics/performance-testing.md:47` | `completion_perf` | claudine-cli | documentation | `cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture` |
| `claudine/docs/topics/performance-testing.md:63` | `completion_perf` | claudine-cli | documentation | `cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture` |
| `claudine/docs/topics/performance-testing.md:71` | `sequence_perf` | claudine-cli | documentation | `cargo test -p claudine-cli --test sequence_perf` |
| `claudine/docs/topics/signal-handling.md:368` | `level3_wrap_ctrl_c` | claudine-cli | documentation | `> (`cargo check --target x86_64-pc-windows-gnu -p claudine-cli --test level3_wrap_ctrl_c`).` |
| `claudine/docs/topics/signal-handling.md:433` | `level3_wrap_ctrl_c` | claudine-cli | documentation | `(`cargo check --target x86_64-pc-windows-gnu -p claudine-cli --test level3_wrap_ctrl_c`)` |
| `claudine/justfile:208` | `error_guards` | claudine-cli | recipe | `@just _test claudine-cli --test error_guards` |
| `claudine/prompts/create-new-provider.md:141` | `dispatch_inventory` | claudine-cli | documentation | `(`CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`).` |
| `darkmatter/docs/errors/README.md:85` | `error_snapshots` | darkmatter | documentation | `INSTA_UPDATE=always cargo test -p darkmatter --test error_snapshots` |
| `darkmatter/lib/tests/benchmark_fixtures.rs:14` | `benchmark_fixtures` | darkmatter | source comment or doc in a test/source file | `//! `DM_BENCH_EMIT=1 cargo nextest run -p darkmatter --test benchmark_fixtures`` |
| `darkmatter/lib/tests/benchmark_fixtures.rs:136` | `benchmark_fixtures` | darkmatter | **string literal in test code** (R9) | `# --test benchmark_fixtures`. Verified by benchmark_fixtures.rs.\n";` |
| `darkmatter/lib/tests/layout_matrix.rs:4` | `layout_matrix` | biscuit-terminal,darkmatter | source comment or doc in a test/source file | `//! `INSTA_UPDATE=always cargo test -p darkmatter --test layout_matrix`` |
| `darkmatter/lib/tests/render_comparison.rs:11` | `render_comparison` | biscuit-terminal,darkmatter | source comment or doc in a test/source file | `//! `RECORD_DRIFT=1 cargo test -p darkmatter --test render_comparison -- --nocapture`` |
| `darkmatter/lib/tests/render_invariants.rs:40` | `render_invariants` | darkmatter | source comment or doc in a test/source file | `//! `RECORD_INVARIANTS=1 cargo test -p darkmatter --test render_invariants -- --nocapture`` |
| `prompts/_prompt.md:200` | `shipped_prompt_contract` | claudine-cli | documentation | `- **`cargo nextest run -p claudine-cli --test shipped_prompt_contract`** checks every file under `prompts/`: schemas parse, expressions p…` |
| `renderable/justfile:161` | `render_comparison` **ambiguous** | biscuit-terminal,darkmatter | recipe | `out=$(RECORD_DRIFT=1 cargo test -q -p "$crate" --test render_comparison -- --nocapture)` |

## in-flight-spec (8)

| File:line | Target | Package | Why | Line |
|---|---|---|---|---|
| `claudine/features/2026-07-13-error-propogation/plan.md:571` | `error_guards` | claudine-cli | active dated spec/plan; its author owns the command at landing time | `*Now `just _test claudine-cli --test error_guards` (1.4s). The recipe's` |
| `claudine/features/2026-07-13-proxy-with/notes/acceptance-map.md:652` | `level2_lifecycle_control` | claudine-cli | active dated spec/plan; its author owns the command at landing time | `BISCUIT_L2_THREADS=8 just test-l2 level2_lifecycle_ --test level2_lifecycle_control --no-fail-fast` |
| `claudine/features/2026-09-08-steering/verification/README.md:41` | `real_pi_steering` | claudine-cli | active dated spec/plan; its author owns the command at landing time | `--test real_pi_steering --run-ignored all --no-fail-fast` |
| `darkmatter/features/2026-07-15-performance-followup/benchmarks/README.md:51` | `benchmark_fixtures` | darkmatter | active dated spec/plan; its author owns the command at landing time | `3. `DM_BENCH_EMIT=1 cargo nextest run -p darkmatter --test benchmark_fixtures` —` |
| `darkmatter/features/2026-07-15-performance-followup/benchmarks/README.md:53` | `benchmark_fixtures` | darkmatter | active dated spec/plan; its author owns the command at landing time | `4. `cargo nextest run -p darkmatter --test benchmark_fixtures` — verifies.` |
| `darkmatter/features/2026-07-15-performance-followup/benchmarks/manifest.yaml:4` | `benchmark_fixtures` | darkmatter | active dated spec/plan; its author owns the command at landing time | `# --test benchmark_fixtures`. Verified by benchmark_fixtures.rs.` |
| `renderable/features/2026-06-30-style-everywhere/plan.md:275` | `layout_matrix` **ambiguous** | biscuit-terminal,darkmatter | active dated spec/plan; its author owns the command at landing time | `--test layout_matrix`). ✅ 2563 lib + 404 cli tests pass; `just lint` green.` |
| `renderable/features/2026-06-30-style-everywhere/plan.md:643` | `layout_matrix` | biscuit-terminal,darkmatter | active dated spec/plan; its author owns the command at landing time | `- `cargo nextest run -p biscuit-terminal --test layout_matrix` green across the full` |

## historical (by file; never rewritten)

| File | Hits |
|---|---:|
| `biscuit-file/features/2026-07-31-portable-strings/review-2.md` | 1 |
| `biscuit-file/features/2026-07-31-portable-strings/review-3.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/plan.md` | 7 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-2.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-3.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-5.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-1.md` | 1 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-2.md` | 2 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-3.md` | 3 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-4.md` | 3 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/plan.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-1.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-2.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-1.md` | 5 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-2.md` | 9 |
| `claudine/features/2026-07-11-sequence-plus/review-12.md` | 2 |
| `claudine/features/2026-07-13-file-resolution/review-4.md` | 1 |
| `claudine/features/_completed/2026-04-04-sequence/review-2.md` | 1 |
| `claudine/features/_completed/2026-04-10-dry-providers/plan.md` | 1 |
| `claudine/features/_completed/2026-04-17-cli-pre-processing/plan.md` | 5 |
| `claudine/features/_completed/2026-04-17-file-completion/phase-0-lock.md` | 4 |
| `claudine/features/_completed/2026-04-17-file-completion/plan.md` | 5 |
| `claudine/features/_completed/2026-04-17-perf-flag/plan.md` | 4 |
| `claudine/features/_completed/2026-04-17-perf-flag/review-plan-2.md` | 1 |
| `claudine/features/_completed/2026-04-17-perf-flag/review-plan-3.md` | 1 |
| `claudine/features/_completed/2026-04-18-file-completion-supplement/plan.md` | 4 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-2.md` | 6 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-3.md` | 5 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-1.md` | 8 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-2.md` | 20 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-3.md` | 17 |
| `claudine/features/_completed/2026-05-15-schemas/review-1.md` | 1 |
| `claudine/features/_completed/2026-05-15-schemas/review-2.md` | 3 |
| `claudine/features/_completed/2026-05-15-schemas/review-4.md` | 2 |
| `claudine/features/_completed/2026-05-15-schemas/review-5.md` | 2 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-1.md` | 4 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-2.md` | 2 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-3.md` | 2 |
| `claudine/features/_completed/2026-06-03-always-harness/review-1.md` | 3 |
| `claudine/features/_completed/2026-06-03-always-harness/review-2.md` | 2 |
| `claudine/features/_completed/2026-06-03-always-harness/review-3.md` | 6 |
| `claudine/features/_completed/2026-06-09-improved-descriptions/review-1.md` | 1 |
| `claudine/features/_completed/2026-06-09-improved-descriptions/review-2.md` | 3 |
| `claudine/features/_completed/2026-06-14-auto-complete/review-5.md` | 1 |
| `claudine/features/_completed/2026-06-14-interactive/plan.md` | 1 |
| `claudine/features/_completed/2026-06-14-interactive/review-2.md` | 1 |
| `claudine/features/_completed/2026-06-14-interactive/review-3.md` | 2 |
| `claudine/features/_completed/2026-06-18-composition-shell-error-diagnostics/review-1.md` | 1 |
| `claudine/features/_completed/2026-06-18-composition-shell-error-diagnostics/review-2.md` | 2 |
| `claudine/features/_completed/2026-06-18-state-sequencing/review-1.md` | 2 |
| `claudine/features/_completed/2026-06-18-state-sequencing/review-2.md` | 2 |
| `claudine/features/_completed/2026-06-19-repetitive/review-3.md` | 2 |
| `claudine/features/_completed/2026-06-19-repetitive/review-4.md` | 1 |
| `claudine/features/_completed/2026-06-21-remove-validations/review-2.md` | 1 |
| `claudine/features/_completed/2026-06-21-remove-validations/review-3.md` | 2 |
| `claudine/features/_completed/2026-06-28-real-errors/review-10.md` | 2 |
| `claudine/features/_completed/2026-06-28-real-errors/review-11.md` | 2 |
| `claudine/features/_completed/2026-07-02-provider-metadata/next-session-prompt.md` | 1 |
| `claudine/features/_completed/2026-07-06-more-struture/cluster-0-kickoff.md` | 1 |
| `claudine/features/_completed/2026-07-06-more-struture/kickoff-prompt.md` | 1 |
| `claudine/features/_completed/2026-07-11-provider-errors-as-data/spec.md` | 1 |
| `claudine/features/_completed/2026-07-11-real-error-messages/review-2.md` | 1 |
| `claudine/features/_completed/2026-07-11-real-error-messages/review-3.md` | 2 |
| `claudine/fixes/_completed/2026-04-08-different-configs/review.md` | 1 |
| `claudine/fixes/_completed/2026-04-15-args-parsing/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-05-26-poor-error-report/review-2.md` | 2 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-3.md` | 1 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-4.md` | 2 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-6.md` | 1 |
| `claudine/fixes/_completed/2026-06-18-dirname/plan.md` | 1 |
| `claudine/fixes/_completed/2026-06-18-dirname/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-06-18-expression-engine/plan.md` | 1 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/phase4-test-map.md` | 4 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/plan.md` | 1 |
| `claudine/fixes/_completed/2026-08-01-cli-slow-tests/log.md` | 6 |
| `claudine/fixes/_completed/2026-08-12-ctx-launch-anchor/log.md` | 2 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/inventory.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/log.md` | 3 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/plan.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/results.md` | 2 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-09-08-frontmatter-matters/review-1.md` | 2 |
| `claudine/fixes/_completed/2026-09-08-frontmatter-matters/spec.md` | 1 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/implementation-log.md` | 17 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/review-2.md` | 1 |
| `claudine/fixes/_completed/2026-09-13-better-static-analysis/implementation-log.md` | 2 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/evidence.md` | 4 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/implementation-log.md` | 6 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/review-1.md` | 1 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/review-2.md` | 2 |
| `claudine/fixes/_completed/2026-09-16-better-spec-syntax/implementation-log.md` | 1 |
| `darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md` | 3 |
| `darkmatter/features/2026-07-13-more-is-more/log.md` | 1 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md` | 1 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/log.md` | 13 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/review-2.md` | 5 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/review-3.md` | 5 |
| `darkmatter/features/2026-07-15-performance-followup/results.md` | 1 |
| `darkmatter/features/_completed/2026-04-18-hr/review-3.md` | 1 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-1.md` | 1 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-3.md` | 4 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-4.md` | 8 |
| `darkmatter/features/_completed/2026-04-20-better-errors/review-1.md` | 1 |
| `darkmatter/features/_completed/2026-04-20-better-errors/review-plan-1.md` | 8 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-2.md` | 2 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-3.md` | 1 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-4.md` | 1 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-plan-2.md` | 2 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-plan-3.md` | 1 |
| `darkmatter/features/_completed/2026-05-07-unary-ops-for-interpolation/review-3.md` | 1 |
| `darkmatter/features/_completed/2026-05-07-unary-ops-for-interpolation/review-4.md` | 1 |
| `darkmatter/features/_completed/2026-05-08-good-errors/review-3.md` | 1 |
| `darkmatter/features/_completed/2026-05-08-good-errors/review-4.md` | 1 |
| `darkmatter/features/_completed/2026-05-11-schemas/review-2.md` | 4 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-11.md` | 2 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-6.md` | 1 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-7.md` | 1 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-9.md` | 1 |
| `darkmatter/features/_completed/2026-06-01-more-context-variables/plan-local.md` | 2 |
| `darkmatter/features/_completed/2026-06-07-file-links/plan.md` | 1 |
| `darkmatter/features/_completed/2026-06-07-file-links/review-1.md` | 1 |
| `darkmatter/features/_completed/2026-06-07-file-links/review-4.md` | 1 |
| `darkmatter/features/_completed/2026-06-10-schema-improvement/review-2.md` | 2 |
| `darkmatter/features/_completed/2026-06-10-schema-improvement/review-3.md` | 2 |
| `darkmatter/features/_completed/2026-06-11-simplified-rendering/review-5.md` | 2 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-1.md` | 1 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-2.md` | 4 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-3.md` | 1 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-4.md` | 3 |
| `darkmatter/features/_completed/2026-06-15-context-vars-additions/review-2.md` | 1 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/plan.md` | 2 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-1.md` | 3 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-2.md` | 4 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-3.md` | 4 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-4.md` | 3 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/spec.md` | 2 |
| `darkmatter/features/_completed/2026-06-27-file-property-rewrite/review-1.md` | 1 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-1.md` | 1 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-2.md` | 2 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-3.md` | 2 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-4.md` | 3 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-5.md` | 3 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-6.md` | 4 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-7.md` | 4 |
| `darkmatter/features/_completed/2026-07-15-reference-graph/review-1.md` | 1 |
| `darkmatter/fixes/_completed/2026-04-26-indent-shell-expansion/review-4.md` | 1 |
| `darkmatter/fixes/_completed/2026-05-07-indent-toc-linking/plan.md` | 1 |
| `darkmatter/fixes/_completed/2026-07-09-godless-beauty/phase-1-baseline.md` | 1 |
| `darkmatter/fixes/_completed/2026-07-09-godless-beauty/plan.md` | 2 |
| `darkmatter/fixes/_completed/2026-07-20-dm-mega-merge/resolution-record.md` | 1 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/log.md` | 8 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-1.md` | 3 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-2.md` | 3 |
| `darkmatter/fixes/_completed/2026-09-13-unify-array-rendering/results.md` | 1 |
| `fixes/_completed/2026-09-19-less-brittle/review-2.md` | 1 |
| `messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md` | 2 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-2.md` | 1 |
| `renderable/features/_completed/2026-05-14-kickoff/plan-project-3-migrate-callers.md` | 4 |
| `renderable/features/_completed/2026-05-14-kickoff/review-3.md` | 2 |
| `renderable/features/_completed/2026-05-16-color-decisions/review-1.md` | 2 |
| `renderable/features/_completed/2026-05-16-color-decisions/review-2.md` | 3 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-1.md` | 1 |
| `renderable/features/_completed/2026-05-17-render-comparison-assertions/plan.md` | 7 |
| `renderable/features/_completed/2026-05-17-render-comparison-assertions/spec.md` | 2 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage1-and-2/StatusBlock-review-1.md` | 1 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage3-plan.md` | 18 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/plan.md` | 2 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/review-1.md` | 1 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/review-2.md` | 1 |
| `renderable/features/_completed/2026-05-23-style-property/completion-review-1.md` | 1 |
| `renderable/features/_completed/2026-05-23-style-property/plan-1.md` | 1 |
| `renderable/features/_completed/2026-05-23-style-property/review-3.md` | 1 |
| `renderable/features/_completed/2026-05-23-style-property/review-5.md` | 1 |
| `renderable/features/_completed/2026-05-26-block-extension/plan.md` | 4 |
| `renderable/features/_completed/2026-05-26-block-extension/review-2.md` | 2 |
| `renderable/features/_completed/2026-05-26-block-extension/review-3.md` | 2 |
| `renderable/features/_completed/2026-05-26-block-extension/review-4.md` | 1 |
| `renderable/features/_completed/2026-05-26-block-extension/review-5.md` | 2 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-4.md` | 2 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-5.md` | 1 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-6.md` | 1 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-7.md` | 1 |
| `renderable/features/_completed/2026-05-26-inline-span/phase-1-prototype-notes.md` | 1 |
| `renderable/features/_completed/2026-05-26-inline-span/plan.md` | 2 |
| `renderable/features/_completed/2026-05-26-inline-span/review-3.md` | 2 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-3-notes.md` | 1 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-4-notes.md` | 1 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-5-notes.md` | 1 |
| `renderable/features/_completed/2026-06-02-tree-cutover/implementation-notes.md` | 2 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-4.md` | 1 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-5.md` | 1 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-6.md` | 1 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/plan.md` | 2 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/review-2.md` | 2 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/review-4.md` | 2 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-1.md` | 1 |
| `renderable/features/_completed/2026-06-04-tree-attrs/plan.md` | 1 |
| `renderable/features/_completed/2026-06-04-tree-attrs/review-2.md` | 1 |
| `renderable/features/_completed/2026-06-04-tree-attrs/review-3.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-closeout/performance-record.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-closeout/plan.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-10.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-8.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-9.md` | 1 |
| `renderable/features/_completed/2026-06-06-tree-features/plan.md` | 3 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-1.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-2.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-3.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-4.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-5.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-6.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-7.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-8.md` | 1 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-9.md` | 1 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-1.md` | 1 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-2.md` | 2 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-3.md` | 1 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-4.md` | 1 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-1.md` | 4 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-2.md` | 4 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-3.md` | 4 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/log.md` | 1 |
