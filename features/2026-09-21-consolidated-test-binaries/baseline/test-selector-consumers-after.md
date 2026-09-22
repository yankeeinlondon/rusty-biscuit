---
kind: evidence
feature: 2026-09-21-consolidated-test-binaries
plan_phase: 7
generator: baseline/consumer-sweep.py --after
---

# `--test` consumer re-sweep (after all four migrations)

Every `--test <name>` selector in git-tracked text files whose `<name>` is an
old per-file test target of `claudine-cli`, `darkmatter`, `darkmatter-cli`, or
`biscuit-terminal`, as recorded in the four `*-migration.json` manifests.
A line that names another package with `-p`/`--package`
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
| active | 0 | 0 |
| in-flight-spec | 4 | 3 |
| self | 38 | 1 |
| historical | 547 | 226 |

## active (0)

| File:line | Target | Package | Why | Line |
|---|---|---|---|---|

## in-flight-spec (4)

| File:line | Target | Package | Why | Line |
|---|---|---|---|---|
| `claudine/features/2026-07-13-error-propogation/plan.md:571` | `error_guards` | claudine-cli | active dated spec/plan; its author owns the command at landing time | `*Now `just _test claudine-cli --test error_guards` (1.4s). The recipe's` |
| `claudine/features/2026-07-13-proxy-with/notes/acceptance-map.md:652` | `level2_lifecycle_control` | claudine-cli | active dated spec/plan; its author owns the command at landing time | `BISCUIT_L2_THREADS=8 just test-l2 level2_lifecycle_ --test level2_lifecycle_control --no-fail-fast` |
| `renderable/features/2026-06-30-style-everywhere/plan.md:275` | `layout_matrix` **ambiguous** | biscuit-terminal,darkmatter | active dated spec/plan; its author owns the command at landing time | `--test layout_matrix`). ✅ 2563 lib + 404 cli tests pass; `just lint` green.` |
| `renderable/features/2026-06-30-style-everywhere/plan.md:643` | `layout_matrix` | biscuit-terminal,darkmatter | active dated spec/plan; its author owns the command at landing time | `- `cargo nextest run -p biscuit-terminal --test layout_matrix` green across the full` |

## historical (by file; never rewritten)

| File | Hits | Record date |
|---|---:|---|
| `biscuit-file/features/2026-07-31-portable-strings/review-2.md` | 1 | 2026-07-31 |
| `biscuit-file/features/2026-07-31-portable-strings/review-3.md` | 1 | 2026-07-31 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/plan.md` | 7 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-2.md` | 1 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-3.md` | 1 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-5.md` | 1 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-1.md` | 1 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-2.md` | 2 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-3.md` | 3 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-apple-terminal/review-plan-4.md` | 3 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/plan.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-1.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-2.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-1.md` | 5 | 2026-05-02 |
| `biscuit-terminal/features/_completed/2026-05-02-l1-l2-tests/review-plan-2.md` | 9 | 2026-05-02 |
| `claudine/features/2026-07-11-sequence-plus/review-12.md` | 2 | 2026-07-11 |
| `claudine/features/2026-07-13-file-resolution/review-4.md` | 1 | 2026-07-13 |
| `claudine/features/_completed/2026-04-04-sequence/review-2.md` | 1 | 2026-04-04 |
| `claudine/features/_completed/2026-04-10-dry-providers/plan.md` | 1 | 2026-04-10 |
| `claudine/features/_completed/2026-04-17-cli-pre-processing/plan.md` | 5 | 2026-04-17 |
| `claudine/features/_completed/2026-04-17-file-completion/phase-0-lock.md` | 4 | 2026-04-17 |
| `claudine/features/_completed/2026-04-17-file-completion/plan.md` | 5 | 2026-04-17 |
| `claudine/features/_completed/2026-04-17-perf-flag/plan.md` | 4 | 2026-04-17 |
| `claudine/features/_completed/2026-04-17-perf-flag/review-plan-2.md` | 1 | 2026-04-17 |
| `claudine/features/_completed/2026-04-17-perf-flag/review-plan-3.md` | 1 | 2026-04-17 |
| `claudine/features/_completed/2026-04-18-file-completion-supplement/plan.md` | 4 | 2026-04-18 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-2.md` | 6 | 2026-04-24 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-3.md` | 5 | 2026-04-24 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-1.md` | 8 | 2026-04-24 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-2.md` | 20 | 2026-04-24 |
| `claudine/features/_completed/2026-04-24-improved-shell-completions/review-plan-3.md` | 17 | 2026-04-24 |
| `claudine/features/_completed/2026-05-15-schemas/review-1.md` | 1 | 2026-05-15 |
| `claudine/features/_completed/2026-05-15-schemas/review-2.md` | 3 | 2026-05-15 |
| `claudine/features/_completed/2026-05-15-schemas/review-4.md` | 2 | 2026-05-15 |
| `claudine/features/_completed/2026-05-15-schemas/review-5.md` | 2 | 2026-05-15 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-1.md` | 4 | 2026-05-25 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-2.md` | 2 | 2026-05-25 |
| `claudine/features/_completed/2026-05-25-prompt-reporting-encapsulation/review-3.md` | 2 | 2026-05-25 |
| `claudine/features/_completed/2026-06-03-always-harness/review-1.md` | 3 | 2026-06-03 |
| `claudine/features/_completed/2026-06-03-always-harness/review-2.md` | 2 | 2026-06-03 |
| `claudine/features/_completed/2026-06-03-always-harness/review-3.md` | 6 | 2026-06-03 |
| `claudine/features/_completed/2026-06-09-improved-descriptions/review-1.md` | 1 | 2026-06-09 |
| `claudine/features/_completed/2026-06-09-improved-descriptions/review-2.md` | 3 | 2026-06-09 |
| `claudine/features/_completed/2026-06-14-auto-complete/review-5.md` | 1 | 2026-06-14 |
| `claudine/features/_completed/2026-06-14-interactive/plan.md` | 1 | 2026-06-14 |
| `claudine/features/_completed/2026-06-14-interactive/review-2.md` | 1 | 2026-06-14 |
| `claudine/features/_completed/2026-06-14-interactive/review-3.md` | 2 | 2026-06-14 |
| `claudine/features/_completed/2026-06-18-composition-shell-error-diagnostics/review-1.md` | 1 | 2026-06-18 |
| `claudine/features/_completed/2026-06-18-composition-shell-error-diagnostics/review-2.md` | 2 | 2026-06-18 |
| `claudine/features/_completed/2026-06-18-state-sequencing/review-1.md` | 2 | 2026-06-18 |
| `claudine/features/_completed/2026-06-18-state-sequencing/review-2.md` | 2 | 2026-06-18 |
| `claudine/features/_completed/2026-06-19-repetitive/review-3.md` | 2 | 2026-06-19 |
| `claudine/features/_completed/2026-06-19-repetitive/review-4.md` | 1 | 2026-06-19 |
| `claudine/features/_completed/2026-06-21-remove-validations/review-2.md` | 1 | 2026-06-21 |
| `claudine/features/_completed/2026-06-21-remove-validations/review-3.md` | 2 | 2026-06-21 |
| `claudine/features/_completed/2026-06-28-real-errors/review-10.md` | 2 | 2026-06-28 |
| `claudine/features/_completed/2026-06-28-real-errors/review-11.md` | 2 | 2026-06-28 |
| `claudine/features/_completed/2026-07-02-provider-metadata/next-session-prompt.md` | 1 | 2026-07-02 |
| `claudine/features/_completed/2026-07-06-more-struture/cluster-0-kickoff.md` | 1 | 2026-07-06 |
| `claudine/features/_completed/2026-07-06-more-struture/kickoff-prompt.md` | 1 | 2026-07-06 |
| `claudine/features/_completed/2026-07-11-provider-errors-as-data/spec.md` | 1 | 2026-07-11 |
| `claudine/features/_completed/2026-07-11-real-error-messages/review-2.md` | 1 | 2026-07-11 |
| `claudine/features/_completed/2026-07-11-real-error-messages/review-3.md` | 2 | 2026-07-11 |
| `claudine/fixes/_completed/2026-04-08-different-configs/review.md` | 1 | 2026-04-08 |
| `claudine/fixes/_completed/2026-04-15-args-parsing/review-2.md` | 1 | 2026-04-15 |
| `claudine/fixes/_completed/2026-05-26-poor-error-report/review-2.md` | 2 | 2026-05-26 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-1.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-2.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-3.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-4.md` | 2 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-10-error-suggest-format/review-6.md` | 1 | 2026-06-10 |
| `claudine/fixes/_completed/2026-06-18-dirname/plan.md` | 1 | 2026-06-18 |
| `claudine/fixes/_completed/2026-06-18-dirname/review-1.md` | 1 | 2026-06-18 |
| `claudine/fixes/_completed/2026-06-18-expression-engine/plan.md` | 1 | 2026-06-18 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/phase4-test-map.md` | 4 | 2026-07-20 |
| `claudine/fixes/_completed/2026-07-20-claudine-mega-merge/plan.md` | 1 | 2026-07-20 |
| `claudine/fixes/_completed/2026-08-01-cli-slow-tests/log.md` | 6 | 2026-08-01 |
| `claudine/fixes/_completed/2026-08-12-ctx-launch-anchor/log.md` | 2 | 2026-08-12 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/inventory.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/log.md` | 3 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/plan.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/results.md` | 2 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-1.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-07-faster-claudine-tests/review-2.md` | 1 | 2026-09-07 |
| `claudine/fixes/_completed/2026-09-08-frontmatter-matters/review-1.md` | 2 | 2026-09-08 |
| `claudine/fixes/_completed/2026-09-08-frontmatter-matters/spec.md` | 1 | 2026-09-08 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/implementation-log.md` | 17 | 2026-09-12 |
| `claudine/fixes/_completed/2026-09-12-shadow-home/review-2.md` | 1 | 2026-09-12 |
| `claudine/fixes/_completed/2026-09-13-better-static-analysis/implementation-log.md` | 2 | 2026-09-13 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/evidence.md` | 4 | 2026-09-15 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/implementation-log.md` | 6 | 2026-09-15 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/review-1.md` | 1 | 2026-09-15 |
| `claudine/fixes/_completed/2026-09-15-initialize-after-proxy/review-2.md` | 2 | 2026-09-15 |
| `claudine/fixes/_completed/2026-09-16-better-spec-syntax/implementation-log.md` | 1 | 2026-09-16 |
| `darkmatter/features/2026-07-13-meta-schema/phase5-baseline-replay.md` | 3 | 2026-07-13 |
| `darkmatter/features/2026-07-13-more-is-more/log.md` | 1 | 2026-07-13 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md` | 1 | 2026-07-14 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/log.md` | 13 | 2026-07-14 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/review-2.md` | 5 | 2026-07-14 |
| `darkmatter/features/2026-07-14-invalid-frontmatter/review-3.md` | 5 | 2026-07-14 |
| `darkmatter/features/2026-07-15-performance-followup/results.md` | 1 | 2026-07-15 |
| `darkmatter/features/_completed/2026-04-18-hr/review-3.md` | 1 | 2026-04-18 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-1.md` | 1 | 2026-04-18 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-3.md` | 4 | 2026-04-18 |
| `darkmatter/features/_completed/2026-04-18-hr/review-plan-4.md` | 8 | 2026-04-18 |
| `darkmatter/features/_completed/2026-04-20-better-errors/review-1.md` | 1 | 2026-04-20 |
| `darkmatter/features/_completed/2026-04-20-better-errors/review-plan-1.md` | 8 | 2026-04-20 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-2.md` | 2 | 2026-05-05 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-3.md` | 1 | 2026-05-05 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-4.md` | 1 | 2026-05-05 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-plan-2.md` | 2 | 2026-05-05 |
| `darkmatter/features/_completed/2026-05-05-filepath-interpolation/review-plan-3.md` | 1 | 2026-05-05 |
| `darkmatter/features/_completed/2026-05-07-unary-ops-for-interpolation/review-3.md` | 1 | 2026-05-07 |
| `darkmatter/features/_completed/2026-05-07-unary-ops-for-interpolation/review-4.md` | 1 | 2026-05-07 |
| `darkmatter/features/_completed/2026-05-08-good-errors/review-3.md` | 1 | 2026-05-08 |
| `darkmatter/features/_completed/2026-05-08-good-errors/review-4.md` | 1 | 2026-05-08 |
| `darkmatter/features/_completed/2026-05-11-schemas/review-2.md` | 4 | 2026-05-11 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-11.md` | 2 | 2026-05-23 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-6.md` | 1 | 2026-05-23 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-7.md` | 1 | 2026-05-23 |
| `darkmatter/features/_completed/2026-05-23-compose-schema/review-9.md` | 1 | 2026-05-23 |
| `darkmatter/features/_completed/2026-06-01-more-context-variables/plan-local.md` | 2 | 2026-06-01 |
| `darkmatter/features/_completed/2026-06-07-file-links/plan.md` | 1 | 2026-06-07 |
| `darkmatter/features/_completed/2026-06-07-file-links/review-1.md` | 1 | 2026-06-07 |
| `darkmatter/features/_completed/2026-06-07-file-links/review-4.md` | 1 | 2026-06-07 |
| `darkmatter/features/_completed/2026-06-10-schema-improvement/review-2.md` | 2 | 2026-06-10 |
| `darkmatter/features/_completed/2026-06-10-schema-improvement/review-3.md` | 2 | 2026-06-10 |
| `darkmatter/features/_completed/2026-06-11-simplified-rendering/review-5.md` | 2 | 2026-06-11 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-1.md` | 1 | 2026-06-12 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-2.md` | 4 | 2026-06-12 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-3.md` | 1 | 2026-06-12 |
| `darkmatter/features/_completed/2026-06-12-disclosure/review-4.md` | 3 | 2026-06-12 |
| `darkmatter/features/_completed/2026-06-15-context-vars-additions/review-2.md` | 1 | 2026-06-15 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/plan.md` | 2 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-1.md` | 3 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-2.md` | 4 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-3.md` | 4 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/review-4.md` | 3 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-17-cli-atheist/spec.md` | 2 | 2026-06-17 |
| `darkmatter/features/_completed/2026-06-27-file-property-rewrite/review-1.md` | 1 | 2026-06-27 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-1.md` | 1 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-2.md` | 2 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-3.md` | 2 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-4.md` | 3 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-5.md` | 3 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-6.md` | 4 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-12-literal-expression/review-7.md` | 4 | 2026-07-12 |
| `darkmatter/features/_completed/2026-07-15-reference-graph/review-1.md` | 1 | 2026-07-15 |
| `darkmatter/fixes/_completed/2026-04-26-indent-shell-expansion/review-4.md` | 1 | 2026-04-26 |
| `darkmatter/fixes/_completed/2026-05-07-indent-toc-linking/plan.md` | 1 | 2026-05-07 |
| `darkmatter/fixes/_completed/2026-07-09-godless-beauty/phase-1-baseline.md` | 1 | 2026-07-09 |
| `darkmatter/fixes/_completed/2026-07-09-godless-beauty/plan.md` | 2 | 2026-07-09 |
| `darkmatter/fixes/_completed/2026-07-20-dm-mega-merge/resolution-record.md` | 1 | 2026-07-20 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/log.md` | 8 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-1.md` | 3 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-07-faster-darkmatter-tests/review-2.md` | 3 | 2026-09-07 |
| `darkmatter/fixes/_completed/2026-09-13-unify-array-rendering/results.md` | 1 | 2026-09-13 |
| `features/2026-09-21-consolidated-test-binaries/implementation-log.md` | 1 | 2026-09-21 |
| `fixes/_completed/2026-09-19-less-brittle/review-2.md` | 1 | 2026-09-19 |
| `messenger/features/2026-09-17-research-metadata-pipeline/implementation-log.md` | 2 | 2026-09-17 |
| `renderable/features/_completed/2026-04-17-layout-and-style/review-2.md` | 1 | 2026-04-17 |
| `renderable/features/_completed/2026-05-14-kickoff/plan-project-3-migrate-callers.md` | 4 | 2026-05-14 |
| `renderable/features/_completed/2026-05-14-kickoff/review-3.md` | 2 | 2026-05-14 |
| `renderable/features/_completed/2026-05-16-color-decisions/review-1.md` | 2 | 2026-05-16 |
| `renderable/features/_completed/2026-05-16-color-decisions/review-2.md` | 3 | 2026-05-16 |
| `renderable/features/_completed/2026-05-17-prose-cross-target/review-1.md` | 1 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-render-comparison-assertions/plan.md` | 7 | 2026-05-17 |
| `renderable/features/_completed/2026-05-17-render-comparison-assertions/spec.md` | 2 | 2026-05-17 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage1-and-2/StatusBlock-review-1.md` | 1 | 2026-05-19 |
| `renderable/features/_completed/2026-05-19-pushing-toward-ir/stage3-plan.md` | 18 | 2026-05-19 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/plan.md` | 2 | 2026-05-20 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/review-1.md` | 1 | 2026-05-20 |
| `renderable/features/_completed/2026-05-20-darkmatter-tree/review-2.md` | 1 | 2026-05-20 |
| `renderable/features/_completed/2026-05-23-style-property/completion-review-1.md` | 1 | 2026-05-23 |
| `renderable/features/_completed/2026-05-23-style-property/plan-1.md` | 1 | 2026-05-23 |
| `renderable/features/_completed/2026-05-23-style-property/review-3.md` | 1 | 2026-05-23 |
| `renderable/features/_completed/2026-05-23-style-property/review-5.md` | 1 | 2026-05-23 |
| `renderable/features/_completed/2026-05-26-block-extension/plan.md` | 4 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-block-extension/review-2.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-block-extension/review-3.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-block-extension/review-4.md` | 1 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-block-extension/review-5.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-4.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-5.md` | 1 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-6.md` | 1 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-graphics-policy/review-7.md` | 1 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-inline-span/phase-1-prototype-notes.md` | 1 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-inline-span/plan.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-05-26-inline-span/review-3.md` | 2 | 2026-05-26 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-3-notes.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-4-notes.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-non-structural/phase-5-notes.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-tree-cutover/implementation-notes.md` | 2 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-4.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-5.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-02-tree-cutover/review-6.md` | 1 | 2026-06-02 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/plan.md` | 2 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/review-2.md` | 2 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-darkmatter-cutover/review-4.md` | 2 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-renderer-folds/review-1.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-tree-attrs/plan.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-tree-attrs/review-2.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-04-tree-attrs/review-3.md` | 1 | 2026-06-04 |
| `renderable/features/_completed/2026-06-06-tree-closeout/performance-record.md` | 1 | 2026-06-06 |
| `renderable/features/_completed/2026-06-06-tree-closeout/plan.md` | 1 | 2026-06-06 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-10.md` | 1 | 2026-06-06 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-8.md` | 1 | 2026-06-06 |
| `renderable/features/_completed/2026-06-06-tree-closeout/review-9.md` | 1 | 2026-06-06 |
| `renderable/features/_completed/2026-06-06-tree-features/plan.md` | 3 | 2026-06-06 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-1.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-2.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-3.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-4.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-5.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-6.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-7.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-8.md` | 1 | 2026-06-08 |
| `renderable/features/_completed/2026-06-08-prose-cells/review-9.md` | 1 | 2026-06-08 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-1.md` | 1 | 2026-05-22 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-2.md` | 2 | 2026-05-22 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-3.md` | 1 | 2026-05-22 |
| `renderable/fixes/_completed/2026-05-22-darkmatter-failures/review-4.md` | 1 | 2026-05-22 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-1.md` | 4 | 2026-06-20 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-2.md` | 4 | 2026-06-20 |
| `sniff/features/_completed/2026-06-20-faster-package-list/review-3.md` | 4 | 2026-06-20 |
| `sniff/fixes/_completed/2026-09-07-faster-sniff-tests/log.md` | 1 | 2026-09-07 |
