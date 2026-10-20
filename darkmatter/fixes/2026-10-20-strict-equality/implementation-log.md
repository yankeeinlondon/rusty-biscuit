---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-20-strict-equality/spec.md"
plan: "darkmatter/fixes/2026-10-20-strict-equality/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 changed no code. The migration worksheet is in this log under
    "## Phase 1 > Migration audit worksheet"; Phase 5 executes it, and its
    "Rust tests that assert the old contract" list is the starting point for
    Phases 2-4 (conditions.rs null_equal_null_is_false / null_not_equal_null_is_false,
    interpolation/evaluator.rs `items == '["a","b"]'`, expression_type_scope.rs,
    type_scope_cases.rs, directive_target_analysis.rs, semantics.rs operator
    description, claudine dispatch/runner/tests.rs `ctx.today != ''`, claudine
    looping/expression/tests.rs `env.PATH != ""`).
    Baseline: darkmatter and claudine `just test`/`just lint` are green. Two dmls
    timing tests (searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back,
    registry_observability::a_cancelled_request_reports_its_parked_time) are
    load-sensitive and pre-existing: they fail under load and pass alone; do not
    attribute them to the equality change.
    Behavior to watch: `prompts/code-comment-quality.md:15`
    (`ctx.area == ctx.package || null`) flips when both are null.
---

# Implementation Log for 2026-10-20-strict-equality (6 phases)

## Phase 1

- Started 2026-10-20. Phase 1 is groundwork only: baseline gates and the
  migration audit worksheet. No behavior changes.

### Baseline (before any behavior change)

- `sniff repo package-areas` lists both `darkmatter` and `claudine` (plus
  nested `darkmatter/dmls`, `claudine/rendezvous`).
- `claudine/`: `just test` 10600/10600 passed (9 skipped); `just lint` clean.
- `darkmatter/`: `just lint` clean. `just test` failed once on each of two
  runs with a different dmls timing test each time
  (`searched_candidates::a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back`,
  then `registry_observability::a_cancelled_request_reports_its_parked_time`).
  Both pass alone, `cargo nextest run -p dmls` passes 1275/1275 in isolation,
  and a third full `just test` passed 10777/10777. Recorded as
  **load-sensitive, pre-existing** (the first ran concurrently with the
  read-only sweep subagents); no source had changed.
- No source, doc, or skill file changed in this phase; only this log and the
  plan's checkboxes/frontmatter.

### Gate-skip check

`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
→ `packages: ['darkmatter']`, `change_class: package`. The working tree also
holds earlier uncommitted work, but even the phase's own two files alone give
`['darkmatter']`, so the gates were run (results above).

### Phase 1 rulings

The plan's "Necessary Rules" were adopted as written; none proved wrong during
the inventory. In particular: one shared comparator in a new
`expression/equality.rs` (Phase 2); a numeric literal guard keeps the broader
number arm where a `literal(n)` shape would reject equivalent float storage;
no performance spike; no input-robustness matrix (expression grammar, not a
file-format reader); prompt corpus checks stay opt-in; historical specs
(`features/`, `fixes/`, `docs/research/`) are out of the audit.

### Migration audit worksheet

Classes: **keep** = exact typed comparison is the intent and behaves the same
under strict equality; **rewrite** = depends on the old text/both-null
contract (Phase 5 for authored expressions, Phase 6 for prose); **verify** =
behavior depends on a runtime type the text does not show (Phase 5 confirms by
reading the producing value, then keeps or rewrites).

Sweep commands (repository root): `grep -rnE '(==|!=)'` over `prompts/`,
`.claude/commands/` (0 hits), `darkmatter/prompts/` (0 hits),
`claudine/prompts/` (2 hits, both prose about Rust code — excluded),
`.claude/skills/`, `darkmatter/docs/`, `claudine/docs/`, `docs/`, all
`README.md` under `darkmatter/` and `claudine/`, schema YAML/JSON
(`darkmatter/schemas`, `darkmatter/docs/schemas`, `claudine/schemas`,
`claudine/docs/schemas`, `schemas/` — no expression hits), and every
`fixtures/` directory under `darkmatter/` and `claudine/`. Rust, JS/TS,
shell, Python, GitHub Actions, and math/field prose were excluded.

#### Root `prompts/`

| Site | Expression | Class | Reason / proposed rewrite |
| --- | --- | --- | --- |
| `fix.md:45` | `diagnosis == 'fix'` | keep | `diagnosis: string` vs string |
| `fix.md:48` | `diagnosis == 'ask'` | keep | same |
| `_interactive-prompting.md:47` | `maturity == 'greenfield'` | keep | string default `"na"` |
| `pr.md:35` | `remote_vendor() != 'github'` | keep | string-or-null vs string; same result |
| `feature.md:36-43` (8) | `stage == '<stage>'` | keep | string vs string |
| `_reviews/feature-review.md:53,59,69,79` (5 comparisons) | `frontmatter(review,'ready'\|'human_review') == true` / `!= true` | verify | needs YAML boolean; a quoted `"true"` now fails (intended). `!= true` now also true for null/missing |
| `_reviews/feature-review.md:186,422,438` | `iteration == 1` | keep | `iteration: number`, value from `(… \|\| 0) + 1` arithmetic |
| `_os.md:8,10` | `ctx.os == 'Windows'` etc. | keep | string vs string |
| `_prompt.md:25,29` | `length(defects_matches) == 1` / `== 0` | keep | integer vs integer |
| `_prompt.md:30` | `defects_status == ''` | keep | guarded by `!is_string(…) \|\|`; exact empty-string intent |
| `_prompt.md:34` | `length(defects_fixed) != defects_known_ids` | keep | integer vs arithmetic integer |
| `_prompt.md:36` | `defects_status == 'completed'` | keep | string |
| `_prompt.md:103` | `frontmatter(report,'status') == 'passed'` | keep | string |
| `commit.md:19,26` | `length(…) == 1` / `== 0` | keep | integer |
| `review.md:27` | `frontmatter(spec,'reviewed'\|'implemented') == true` | verify | YAML boolean |
| `_pr/fix.md:24,33,48` | `frontmatter(report,'status') == 'fixed'\|'error'` | keep | string |
| `_implement/implement-review.md:23,26,77` | `target == review`, `target != review`, `initial_review != target` | keep | file strings, `review` required (never both null) |
| `_reviews/cross-platform.md:17,21` | `frontmatter(review,'ready') == true` / `!= true` | verify | YAML boolean |
| `_implement/_fm.md:22` | `kind == 'initial-spec-implementation'` | keep | string |
| `code-comment-quality.md:15` | `ctx.area == ctx.package \|\| null` | **verify (likely rewrite)** | outside any package area both are null: old `false`, strict `true`, so the package list would render. Candidate: `ctx.area != null && ctx.area == ctx.package` |
| `_pr/open.md:14` | `frontmatter(report,'status') == 'pr_opened'` | keep | string |
| `_pr/push.md:23` | `number(commits_ahead, 0) == 0` | keep | explicit conversion already |
| `_pr/push.md:32,48,64` | `frontmatter(report,'status') == '…'` | keep | string |
| `_pr/push.md:87` | `err.category == 'timeout'` | keep | string |
| `_pr/push.md:124,127` | `ctx.branch != base`, `ctx.branch == base` | keep | strings; detached HEAD (null) vs `base` gives the same result |
| `_pr/_facts.md:14` | `length(ctx.dirty_files) == 0` | keep | integer |
| `_pr/diagnose.md:24,29` | `frontmatter(report,'status') != 'diagnosed'` | keep | string |
| `_pr/diagnose.md:40,45` | `on_failure == 'stop'\|'fix'` | keep | enum |
| `_pr/diagnose.md:60` | `err.category == 'timeout'` | keep | string |
| `_pr/dirty.md:17` | `uncommitted == 'abort'` | keep | enum |
| `_pr/triage.md:15` | `frontmatter(report,'fix_requested') == true` | verify | YAML boolean |
| `_implement/implement-plan.md:92` | `frontmatter(log,'human_review') == true` | verify | YAML boolean (`phase >= total_phases` is ordering, unchanged) |
| `_implement/implement-plan.md:272,297` | `phase == total_phases` | verify | both `number`-declared; confirm whole-value `{{ }}` defaults keep number type after schema coercion, else `number(phase) == number(total_phases)` |
| `_implement/implement-suggestions.md:22,37,40,46,49,54` | `ready == true`, `frontmatter(review,'implemented'\|'human_review') ==/!= true` | verify | `ready` is `frontmatter(review,'ready')` or null; YAML boolean |
| `_implement/implement-suggestions.md:154` | `followups == 0` | keep | both arms of `followups` produce integers |
| `_implement/implement-suggestions.md:270` | `frontmatter(review,'recurrence') == true` | verify | YAML boolean |
| `_implement/implement-review-findings-plan.md:113,132` | `phase == total_phases` | verify | as implement-plan |
| `merge-conflicts.md:50` | `=======` | excluded | prose, not an expression |

Commit-message guard `ctx.dirty_files && commit_message` (`prompts/_pr/…`,
`prompts/commit.md` route): truthiness, **intentionally unchanged**; Phase 4
keeps it as the acceptance-5 regression case.

#### `.claude/skills/`

| Site | Expression | Class | Reason / proposed rewrite |
| --- | --- | --- | --- |
| `darkmatter/expressions.md:86` | prose: `==`/`!=` by `scalar_string` with the both-null rule | rewrite (Phase 6) | describe strict typed equality |
| `darkmatter/expressions.md:107-108` | prose: missing compares as `''`, `x == null` proves nothing, `x == ''` false side proves presence | rewrite (Phase 6) | new guard table |
| `darkmatter/compose.md:574` | `x == 1 ? …` | keep | illustrates "direct" use, not equality semantics |
| `darkmatter/compose.md:730-731` | operator list | keep | no semantics stated |
| `claudine/hook-actions.md:291` | `doc.tool_name == 'Bash'` | keep | string |
| `claudine/hook-actions.md:302` | `git.branch == 'main'` | keep | string |
| `claudine/hook-actions.md:310` | `ctx.today != ''` | **rewrite** | presence check; strict null is `!= ''`. Use `ctx.today != null` or `!is_empty(ctx.today)` |
| `claudine/research-contracts.md:121` | `env.AGENT == agent` | keep | string vs string |
| `claudine/research-contracts.md:122` | `err.category == 'cap'` | keep | string |
| `claudine/validations-and-handlers.md:56` | `err.category == 'cap'` | keep | string |

Other skill hits (`rust-devops` GitHub Actions `!= '[]'`, Rust/JS/TS/Python
code, field prose such as `DocumentRecord::parsed == false`) are excluded.

#### Documentation (prose updates land in Phase 6; authored examples in Phase 5)

Rewrite rows:

| Site | Content | Proposed change |
| --- | --- | --- |
| `claudine/docs/topics/configuring-actions.md:289` | `ctx.today != ''` example | `ctx.today != null` / `!is_empty(ctx.today)` |
| `claudine/docs/topics/flow-control/lifecycle.md:327-328` | guard list: `x != ''` proves presence; `x == null` does not | `x != null` proves presence; `x == null` false side proves presence |
| `darkmatter/dmls/docs/diagnostics.md:315` | guard list includes `x != ''` | drop `x != ''`; keep `x != null` |
| `darkmatter/docs/inline/interpolation.md:90-91` | "numeric comparison… `"6"` auto-converts" under a list including `==` | coercion is ordering-only; `"6" == 6` is false; use `number(...)` |
| `darkmatter/docs/lsp/features.md:45` | "Numeric strings auto-coerce." for all comparison operators | ordering coerces; `==`/`!=` strict |
| `darkmatter/docs/topics/darkmatter-expressions.md:180` | `log != ''` listed as a guard | remove |
| `darkmatter/docs/topics/darkmatter-expressions.md:287-288` | guard-table rows for `x != ''`, `x != null`, `x == ''` | split per the spec's guard table |
| `darkmatter/docs/topics/darkmatter-expressions.md:293-296` | both-missing / missing-as-empty / `x == null` proves nothing | strict: `null == null`, `x == null` ≡ `is_null(x)` |
| `darkmatter/docs/topics/darkmatter-expressions.md:575` | "compare scalar string representations" | strict typed, recursive, `5 == 5.0` |
| `darkmatter/docs/topics/darkmatter-expressions.md:587-594` | both-missing false, missing == `''`, `x == null` for `""`, `[] == '[]'` | new decision table + migration table |
| `darkmatter/docs/topics/schemas/definition.md:2596` | "even when `==` would compare them equal" (`4` vs `'4'`) | drop/reword: strict `==` also treats them unequal |
| `darkmatter/docs/transclusion/transclusion-design.md:374` | "Equality/inequality compare normalized scalar strings" | strict type and value |
| `darkmatter/docs/topics/expression-engine.md` | type-aware narrowing narrative | align with the new guard facts (Phase 6, plan task) |

Verify rows:

| Site | Expression | Note |
| --- | --- | --- |
| `claudine/docs/topics/flow-control/flow-control.md:291` | `frontmatter(review,'ready') == true` | YAML boolean required; document that |
| `claudine/docs/topics/flow-control/lifecycle.md:1168` | `err.detail.response_text != null` | strict also true for `""`; if "non-empty" is meant use `!is_empty(...)` |
| `claudine/docs/topics/flow-control/looping.md:324` | `until: "done == true"` | boolean required; note it |
| `darkmatter/docs/inline/interpolation.md:93` | `color \|\| blue == "blue" ? …` | example looks malformed regardless; fix while there |

Keep rows (86 sites, string/enum/integer comparisons that behave the same):
`claudine/cli/README.md:95,102,104,105,113,114`;
`claudine/docs/getting-started/index.md:505,510,517,772`;
`claudine/docs/topics/configuring-actions.md:38,279,301,319`;
`claudine/docs/topics/flow-control/flow-control.md:51,90,117,206,231,270,344,347`;
`claudine/docs/topics/flow-control/lifecycle.md:173,212,216,763,766,1103,1118,1151,1158,1311,1371,1378,1431,1436`
(`:1103` `err.detail.reset_at == null` becomes *correct* under strict);
`claudine/docs/topics/flow-control/looping.md:125,137,365,378,382`;
`claudine/docs/topics/flow-control/sequences.md:233,294`;
`claudine/docs/topics/state-management/expression-engine.md:125,126,146`;
`claudine/docs/topics/unified-events.md:933,942`;
`darkmatter/dmls/docs/autocomplete.md:149`; `darkmatter/dmls/docs/diagnostics.md:315,317` (`x != null`, `x == 'main.md'`);
`darkmatter/docs/cli/graph.md:114,115`; `darkmatter/docs/inline/interpolation.md:76,77,167`;
`darkmatter/docs/inline/page-blocks.md:18,23,84,88,102`;
`darkmatter/docs/inline/preflight-checks.md:515,537,569`;
`darkmatter/docs/topics/darkmatter-expressions.md:180 (log != null),194,229,286,289,305,339,350,571,578,582,1329,1555,1557,1573,1574,1581,1589`;
`darkmatter/docs/topics/schemas/authoring-schemas.md:283`, `definition.md:614`, `schema-targeting.md:50`, `parsing/lexing.md:139`;
`darkmatter/docs/transclusion/block-transclusion.md:208,231,232`.

#### Fixtures (authored Markdown/YAML under `fixtures/`)

No fixture site depends on the old text semantics (0 rewrite). 47 keep,
27 verify lines (29 comparisons).

- **keep**: `darkmatter/dmls/tests/fixtures/mapping_only_corpus/` (`_interactive-prompting.md:47`, `_implement___fm.md:22`, `feature.md:36-43`); `darkmatter/lib/tests/fixtures/clean/prompts-commit-464e2d90e.md:18,26`; `claudine/cli/tests/fixtures/lifecycle_traces/catch-*.md` (`frontmatter('nope-*.md','status') == 'x'`, 7 sites) and `loop-break.md:12`; `claudine/cli/tests/fixtures/nested_span_regression/commit.md:17,23,28`; `claudine/lib/tests/fixtures/claudine-schemas/*` (schema-validated only, never evaluated); `claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md:46,49,52,56` (`as_json([...]) != '[true]'` is string-vs-string; the boolean comparisons are guarded by it).
- **verify** (boolean frontmatter): `darkmatter/lib/tests/fixtures/frozen_prompts/prompts/_reviews/feature-review.md:30,37,43,53,63`; `claudine/cli/tests/fixtures/frozen_prompts/prompts/review.md:27`; `claudine/lib/tests/fixtures/frozen_prompts/prompts/review.md:27`, `_reviews/cross-platform.md:17,21`; `claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md:17`, `example-loops-task-guard.md:6`; both frozen `implement-plan.md` copies `:40`; `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md:78`.
- **verify** (number typing of `{{ }}`-derived values): `darkmatter/dmls/tests/fixtures/sequence_descent/implement-plan.md:36,195,213` (`phase != 1`, `phase == total_phases`); `darkmatter/lib/tests/fixtures/frozen_prompts/prompts/_reviews/feature-review.md:275` (`iteration != 1`); frozen `implement-plan.md` copies `:45` and `:214/:258` (prompts copy) / `:207/:251` (shipped_implement_route copy); `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md:258,283`; `claudine/lib/tests/fixtures/sequence-schema/example-loops-review-loop.md:40` (`… != expected_iteration`, where `expected_iteration` comes from a `set:` arithmetic value).

**Frozen copies.** Neither frozen `implement-plan.md`
(`claudine/cli/tests/fixtures/frozen_prompts/prompts/_implement/` and
`…/frozen_prompts/shipped_implement_route/_implement/`) mirrors the current
`prompts/_implement/implement-plan.md`: both are older snapshots with four
equality sites (`frontmatter(spec,'implemented') == true`, `phase != 1`,
2× `phase == total_phases`), while the original has three
(`until: … frontmatter(log,'human_review') == true`, 2× `phase == total_phases`).
They are frozen on purpose; Phase 5 migrates whatever equality sites each copy
holds (none currently needs a textual rewrite) rather than re-syncing them to
the live original.

#### Rust tests that assert the old contract (Phase 2–4 updates)

Inventory for the evaluator/analysis phases, not authored-artifact migration:

- `darkmatter/lib/src/markdown/compose/conditions.rs` — `null_equal_null_is_false` (~:518) and `null_not_equal_null_is_false` (~:524); `:920 absolute('spec.md') != ''` proves nothing about presence.
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:~1021` — `items == '["a","b"]'` asserted true (array vs JSON text).
- `darkmatter/lib/tests/l1/expression_type_scope.rs` — `either == '4'`, `optional == ''`/`!= null`/`== null`, `big == '4.0'`, `x == ''`, `items == '[]'`, `two_nulls_are_neither_equal_nor_unequal`, `GUARDS`/`samples()`.
- `darkmatter/lib/tests/common/type_scope_cases.rs:~108-109` — `x == null` treated as presence proof.
- `darkmatter/lib/tests/l1/directive_target_analysis.rs:~221-222` — `x != ''` claimed to prove presence.
- `darkmatter/lib/src/markdown/compose/expression/semantics.rs:~94` — operator description "Equality compares scalar string forms".
- `darkmatter/lib/src/markdown/compose/expression/absence.rs` — re-check whether absence analysis relies on equality facts.
- `claudine/lib/src/dispatch/runner/tests.rs:~609,632,839` — `ctx.today != ''` / `ctx.year != ''` used as "resolved" evidence.
- `claudine/lib/src/composition/looping/expression/tests.rs:~176` — `env.PATH != ""` expected false when unset; strict makes it true.
- `claudine/cli/tests/l1/agent_text_is_data.rs:~743` — `total_phases != ''` (still passes, proves little).

#### Worksheet totals

| Area | keep | rewrite | verify |
| --- | --- | --- | --- |
| root `prompts/` | 41 sites | 0 | 26 comparisons (incl. `code-comment-quality.md:15`, likely rewrite) |
| `.claude/skills/` | 7 | 3 | 0 |
| docs / READMEs / schemas | 86 | 13 rows (16 sites) | 4 |
| fixtures | 47 | 0 | 27 lines |

The two open `verify` questions for Phase 5 are the same everywhere:
(1) do whole-value `{{ }}` frontmatter defaults, `set:` values, and CLI args
for `number`-declared properties reach the comparison as numbers after schema
coercion (`phase`, `total_phases`, `iteration`, `expected_iteration`); and
(2) do review/spec/log files write `ready`, `implemented`, `reviewed`,
`human_review`, `recurrence`, `fix_requested` as YAML booleans. Where (1)
holds the site is keep; where (2) holds strict failure on a quoted `"true"`
is the intended behavior.

### Phase 1 close

- Requirement-to-test mapping: none. Phase 1 changes no behavior, so it adds
  no tests; the worksheet's "Rust tests that assert the old contract" list
  names the tests Phases 2–4 must change.
- Gates: `darkmatter` `just test` 10777/10777 and `just lint` clean;
  `claudine` `just test` 10600/10600 and `just lint` clean. No skipped gates.
  Pre-existing load-sensitive dmls tests are noted above.
- No cross-OS check: no code changed.
- The darkmatter skill needs no update for this phase; its stale equality
  prose (`.claude/skills/darkmatter/expressions.md:86,107-108`) is a Phase 6
  worksheet row.
