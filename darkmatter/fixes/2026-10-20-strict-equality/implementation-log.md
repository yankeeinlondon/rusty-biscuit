---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-20-strict-equality/spec.md"
plan: "darkmatter/fixes/2026-10-20-strict-equality/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - claudine
human_review: false
message_to_agent: |-
    Phase 3 rebuilt the guard analysis on `equality::values_equal`:
    `scope/shape.rs` now has `Shape::equal_to(value)` / `Shape::differs_from(value)`
    (the text_* helpers are gone), and `scope/facts.rs` feeds them literal operands,
    `Container(Family)` operands (array/object literals -> `of_family`), and finite
    other-side values including a lone `null` (no both-null routes remain). Literal
    numbers come from the evaluator's own `expression/mod.rs::number_literal`.
    darkmatter `just test` (10793) and claudine `just test` (10600) are fully green;
    both lints are clean.
    Behavior change for Phase 4 consumers: an UNDECLARED value compared with a
    scalar literal now narrows to that literal (`mode == 'release'` ->
    `literal(release)`, `x == ''` -> `literal('')`, `x == null` -> null only),
    where it used to be `unknown(present)`. That changes the shell TemplateShape
    discovered under such guards. Claudine's `assert_runs_only_when_selected`
    helper (lifecycle scope_site_conformance.rs) was rebuilding the approval from
    an unguarded scope; it now approves the discovered site's shape. Look for the
    same unguarded-approval pattern in any consumer test you extend.
    The shared rows in darkmatter/lib/tests/common/type_scope_cases.rs (read by the
    lib and DMLS conformance suites) already assert `x != ''` / `x == null` do NOT
    prove presence while `x != null`, `!(x == null)`, `x == ''` do. Phase 4 still owns
    the per-consumer proofs (shell preflight messages, `::file` targets, absence
    warnings, DMLS hover/diagnostics, Claudine lifecycle/sequence).
    The two claudine-cli tests Phase 2 called pre-existing failures passed in the
    full claudine `just test` this phase; treat them as load-sensitive.
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/expression/equality.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/tests/l1/expression_strict_equality.rs
    - darkmatter/lib/tests/l1/main.rs
    - claudine/lib/src/composition/looping/expression/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/common/type_scope_cases.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/scope_site_conformance.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
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

## Phase 2

- Started 2026-10-20. Scope: the shared typed comparator, the evaluator's
  comparison branch, the semantics catalog, and the acceptance-pair suite.
  The guard analysis (`scope/facts.rs`, `scope/shape.rs`) is Phase 3 and was
  not touched.

### What changed

- **New `lib/src/markdown/compose/expression/equality.rs`** with
  `pub(crate) fn values_equal(&Value, &Value) -> bool`. Same runtime type and
  value, recursive through arrays and objects; objects compare by equal size
  plus per-key lookup, so key order is irrelevant and a missing key never
  equals a `null` entry. Numbers: integer storage (`i64`/`u64`) is widened to
  `i128` and compared exactly; an integer/float pair is equal only when the
  float has no fraction, lies inside `(-2^127, 2^127)`, and converts to that
  exact integer; float/float uses IEEE `==` (so `0.0 == -0.0`). Nothing goes
  through `to_string`, serialization, or an intermediate collection. No
  `arbitrary_precision` feature is enabled for `serde_json` in this
  workspace, so `Number` storage is only `i64`/`u64`/`f64`.
- **`expression/mod.rs`**: `ComparisonOp::Equal` → `values_equal`,
  `NotEqual` → `!values_equal`; the `both_null` exception and its comment are
  gone. Operand evaluation order and `?` error propagation are unchanged;
  ordering arms still use `to_number_coerce`. `scalar_string` remains for
  rendering and for the functions that use it (`contains`, `as_json`, …),
  which are out of scope.
- **`expression/semantics.rs`**: the `Comparison` operator description now
  says equality is strict; `COMPARISON_OPERATOR_DESCRIPTORS` was rewritten to
  five rows, each with an executable example the catalog test verifies:
  `5 == "5"` → `false`, `5 == number("5")` → `true`, `"10" > 2` → `true`,
  `missing_a == missing_b` → `true`, `"" != missing` → `true`. (`null` is not
  a lexer keyword; it reads as a missing variable, so the catalog uses
  missing variables.) These render through `claudine context --expressions`;
  its CLI test asserts only the section heading.

### Tests

New L1 file `lib/tests/l1/expression_strict_equality.rs` (declared in
`tests/l1/main.rs`), all through public `parse` + `evaluate` with a typed
`EvaluationLookup`; every expected result is hand-written. `assert_pair`
checks `==` and `!=` in both operand orders.

| Requirement | Test |
| --- | --- |
| Spec's motivating table (`5 == "5"`, `null == ''`, `null == null`, `[] == '[]'`, `true == "true"`) | `the_motivating_table_now_compares_type_and_value` |
| Spec's decision table (all 13 rows) | `the_decision_table_holds` |
| Missing vs explicit `null`; null vs `0`/`false`/`[]`/`{}`/`''`/`'null'` | `a_missing_value_is_null_and_null_equals_only_null` |
| Booleans, strings (no trim/case-fold), numbers vs text, explicit `number(...)` | `scalars_compare_only_within_their_runtime_type` |
| Empty/nested containers, key order, missing key vs `null`, array vs JSON text | `containers_compare_recursively` |
| `2^53` vs `2^53+1` as typed values, int/float pairs, `i64::MIN`/`MAX`, `u64` above `i64::MAX`, negative zero | `numbers_compare_by_exact_value_across_storage` |
| Ordering unchanged: `5 <= "5"` and `5 >= "5"` hold while `5 == "5"` is false | `ordering_still_coerces_while_equality_does_not` |
| Operand failure stays an error (`1 / 0`, `5 % 0`, unknown function) | `an_operand_failure_stays_an_error_not_an_unequal_result` |
| Reflexivity, symmetry, `!=` complement, and the full expected-equal set over a 27-entry corpus | `equality_is_reflexive_symmetric_and_complemented_over_the_sample_corpus` |

Comparator unit tests in `equality.rs` cover the same numeric boundaries at
the module level. Tier: L1 (pure in-process evaluation, F1; no crate, CLI,
filesystem, or terminal boundary is crossed by this phase's change, and
Phase 4 owns the consumer surfaces).

Old-contract tests updated:

- `conditions.rs` `null_equal_null_is_false` → `null_equal_null_is_true`
  (asserts `true`); `null_not_equal_null_is_false` still holds unchanged.
- `expression/mod.rs` `parity::comparison_behavior_is_consistent`: two
  missing values now compare equal.
- `interpolation/evaluator.rs`
  `equality_comparison_unaffected_by_line_separated_output`: the array
  equals an array literal, and no longer equals its JSON text or its
  line-separated rendering.
- `claudine/lib/src/composition/looping/expression/tests.rs`: the
  `env.PATH != ""` expectation now treats an unset `PATH` (read as `null`) as
  unequal to `""`. On this host `PATH` is set, so the observed result is
  unchanged; the expectation was wrong only for the unset case.

### Gates

- `darkmatter/` `just test`: **10781/10789 passed, 8 failed** (run once
  fail-fast, then with `--no-fail-fast` to exercise the CLI and DMLS suites;
  both runs failed on the same 8 tests and nothing else). The 8 are all in
  `lib/tests/l1/expression_type_scope.rs`:
  `two_nulls_are_neither_equal_nor_unequal` and the
  `every_guard_side_admits_every_value_the_evaluator_sends_there::{booleans,
  literals, containers, optional_strings, undeclared, container_unions,
  scalar_unions}` differential rows. They compare the still-text-based guard
  analysis with the now-strict evaluator, which is exactly the
  disagreement the plan's Phase 2 checkpoint predicts ("shows only Phase 3
  analysis failures") and Phase 3 exists to remove; the plan states that
  landing either half alone leaves these shared tests red. They were not
  silenced, skipped, or edited.
- `darkmatter/` `just lint`: clean. `just check-tier-coverage darkmatter`:
  0 stranded.
- Claudine (downstream, >200 tests, so targeted): `cargo nextest run -p
  claudine -p claudine-cli -E 'test(/looping|condition|sequence|lifecycle|dispatch|context_command|expression|frozen|implement|commit|until|guard|when|agent_text_is_data|hook/)'`
  → 3680/3682 passed. The two failures,
  `document_environment_routes::successive_sequence_steps_each_read_the_map_they_captured`
  (steps read `env=launch` instead of their captured maps) and
  `lifecycle_message_drain::stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown`
  (11 s, drain-budget assertion), **also fail with the baseline evaluator
  restored** (`mod.rs` temporarily reset to `HEAD`, rerun alone, then
  restored), so they are pre-existing and not caused by this change.
  `claudine/` `just lint`: clean.
- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: ['claudine', 'darkmatter', 'test-toolkit']`,
  `change_class: package`; gates were therefore required and were run.
- No cross-OS check: the change is pure in-memory value comparison with no
  path, process, or platform-specific code.

### Notes for later phases

- `scope/facts.rs` (`comparison` doc comment, `compare_with`,
  `against_values`' `both_null_route`) and `scope/shape.rs` (`text_equal`,
  `text_differs`, `number_with_text`, `arm_text_*`) still describe and
  implement the old text contract; they are Phase 3's rewrite and should call
  `equality::values_equal` (a `pub(crate)` sibling module of `scope`).
- `facts.rs::number_operand` builds the literal with `Number::from_f64`
  while the evaluator produces `Number::from(n as i64)` for integral literals
  (plan task "Literal value modeling"). Under `values_equal` the two storage
  forms compare equal, but shape pinning by `literal(4)` may still differ.
- Docs and the darkmatter skill still teach the old rules
  (`.claude/skills/darkmatter/expressions.md:86,107-108`,
  `docs/topics/darkmatter-expressions.md:575,587-594`); those are Phase 6
  rows and were left alone here.

## Phase 3

- Started 2026-10-20. Scope: the guard analysis (`scope/facts.rs`,
  `scope/shape.rs`) rebuilt on the Phase 2 comparator, plus its tests. No
  consumer code, docs, or skills changed (Phases 4 and 6).

### What changed

- **`scope/shape.rs`**: `text_equal`, `text_differs`, `number_with_text`,
  `arm_text_equal`, and `arm_text_differs` are gone. Two strict primitives
  replace them, both deciding by `equality::values_equal`:
    - `Shape::equal_to(value)` (true side of `x == value`): against `null`,
      only `null`; otherwise `null` is removed, alternatives of another
      runtime family are dropped (a number never equals text, a container
      never equals its JSON text), literal arms stay only when strictly
      equal, an unconstrained `string`/`boolean`/`enum` pins to the literal,
      constrained arms and container arms keep their arm. An unknown shape
      pins the same way (`x == 'a'` → `literal(a)`, `x == ''` → `literal('')`,
      `x == null` → `null`), matching the spec's table "for an otherwise
      unconstrained `x`".
    - `Shape::differs_from(value)` (true side of `x != value`): against
      `null`, `without_null`; otherwise strictly equal literals, the matching
      `enum` member, and the matching `boolean` are removed, and `null` stays.
    - Numbers pin to `literal(n)` only when `alternative_accepts` admits every
      `i64`/`u64`/`f64` storage strictly equal to `n` (`pinned_number` /
      `equal_storages`), else the plain `number` arm is kept, as the Phase 1
      ruling requires. In practice `literal(4)` admits `4.0`, so `count == 4`
      still pins.
- **`scope/facts.rs`**: `comparison` folds operator and polarity into one
  "are the operands equal on this side" flag. Literal operands call
  `equal_to`/`differs_from`; array and object literals are a
  `Container(Family)` operand whose equal side is `of_family` (present and of
  that family; elements may be computed, so no value is pinned) and whose
  unequal side learns nothing. Subject-vs-subject comparison joins
  `equal_to` over every finite other-side value, `null` included; the
  unequal side narrows against a single value, now also a lone `null`. The
  `both_null_route` and the null-specific `compare_with` table are removed.
- **Literal value modeling**: the evaluator's integral-literal conversion
  moved into `expression/mod.rs::number_literal` (private, so the descendant
  `scope` module can call it), used by both `Expr::NumberLiteral` evaluation
  and `facts.rs::literal_operand`. Unary minus mirrors the evaluator: the
  literal's value, as `f64`, negated through the same `json_number`. So
  `9007199254740993` is analyzed as the `i64` `2^53` and
  `99999999999999999999` as the saturated `i64::MAX`, exactly what the
  evaluator compares. Evaluator behavior is unchanged (a pure extraction).

### Tests

All L1 (pure in-process analysis plus the real evaluator, F1). No crate,
CLI, filesystem, or terminal boundary changed in this phase; the consumer
surfaces that read these facts (shell preflight, `::file` targets, DMLS)
are exercised through the shared rows below and are Phase 4's to extend.

| Requirement | Test |
| --- | --- |
| Spec guard table: `x == null` / `x != null` / `x == ''` / `x != ''` / `x == 'a'` / `x != 'a'` both sides, reversed operands, negation | `expression_type_scope::equality_narrows_to_literals_of_every_scalar_family` (new rows: `either == 4`, `flag != true`, `optional != ''`, `optional == 'a'`, `optional != 'a'`, `null == optional`, `'' != optional`, `!(optional == null)`) |
| Unknown subject facts; container never equals JSON text; container-family refinement (`x == []` → `unknown[]`, `x == {}` → `object`, union narrowed to its array/object arm, reversed, `!=` and false side learn nothing) | `equality_pins_an_unknown_value_and_a_container_to_its_family` (renamed from `equality_against_unknown_or_a_container_proves_no_scalar_family`) |
| Constraints kept; numbers never equal text; `4.0`/`-0`/`-4.5` literals | `equality_keeps_constraints_and_numeric_families_it_cannot_pin` |
| Literal modeling mirrors the evaluator (`…993` → `2^53`, saturation to `i64::MAX`), with the evaluator result asserted alongside | `literal_numbers_narrow_to_the_value_the_evaluator_compares` (new) |
| Singleton `null` other side is usable | `equality_with_a_null_only_subject_narrows_the_other_side` (new) |
| `null == null` | `two_nulls_are_equal` (inverted from `two_nulls_are_neither_equal_nor_unequal`) |
| Every runtime branch admits what the evaluator sends (acceptance 2) | `every_guard_side_admits_every_value_the_evaluator_sends_there::*`, now 12 declarations (new `null_other_side`, `large_integers`) × 58 guards (new: `x == 4.0`, `x == 0`, `x == -0`, `x == 9007199254740992`, `x != 9007199254740992`, `x == 'null'`, `x == []`, `x != []`, `x == ['a']`, `x == {}`, `{'k': 'v'} == x`, `null != x`, `!(x == null)`, `y == null`, `y == / != 9007199254740992`, `x == y && y == null`) over 28 samples (new: `4.0`, `0.0`, `-0.0`, typed `u64` `2^53` and `2^53+1`, `["a", ["b"]]`, `{"k": null}`, `{"j": "v"}`, `"null"`) |
| Shared consumer rows (shell, shell-block, `::file` target, DMLS): `x != ''` and `x == null` no longer prove presence; `x != null`, `!(x == null)`, `x == ''` do | `tests/common/type_scope_cases.rs` `NotNull` rows, read by `scope_site_conformance.rs` and `dmls/tests/l1/expression_scopes.rs` |
| `::block when` target narrowing agrees with the evaluator | `directive_target_analysis::block_guards_narrow_targets_through_the_shared_type_scope` (`x != ''` / `x == null` now narrow nothing; `x == ''`, `!(x == null)` narrow `x`) |

The 8 tests left red by Phase 2 are green. The in-module `scope/tests.rs`
holds no equality rows and needed no change. Contradictory guards still keep
the incoming type (`pair == 'z'`, `items == '[]'`, `big == '4'`).

Expected results are hand-written; the differential cases decide branches
with the real evaluator, so a shared-comparator bug would surface in the
hand-written evaluator tests of `expression_strict_equality.rs` (Phase 2),
not be masked here.

Grep checkpoint: no `scalar_string` use remains under `scope/`; no
`both_null` route remains anywhere (the only `both_null` hits are two
unrelated shell-pipeline test names in `shell_expansion/tokenize.rs`).

### Downstream effect found in Claudine

Pinning an undeclared value to its literal (`mode == 'release'` →
`literal(release)`, previously `unknown(present)`) changes the
`TemplateShape` that shell pre-flight discovers for a site under that guard.
Two Claudine conformance cells failed with "pre-flight never approved as its
bytes or its shape":
`composition::lifecycle::executor::tests::scope_site_conformance::{lifecycle_inline_guard_equality, lifecycle_shell_equality}`.
The fault was in the test helper, not in production. `assert_runs_only_when_selected`
approved a shape rebuilt from an **unguarded** scope (`shape_of`), whereas
production approves what discovery reports for the guarded site. The helper
now approves `TemplateShape::from_template(site.template())` of the one
discovered site. Other `shape_of` callers use guards that do not pin a
value (truthiness, `!= null`, `file_exists`) and are unchanged. Approvals
stored before this change, for a guarded site whose guard compares an
undeclared value with a literal, no longer match and must be approved again.
There are no users yet, so no migration was added.

### Gates

- `darkmatter/` `just test`: **10793/10793 passed** (10 skipped), the 8
  Phase 2 reds included. `just lint`: clean. `just check-tier-coverage
  darkmatter`: 0 stranded.
- Claudine (downstream; edited directly in one test file):
  the Phase 2 targeted filter widened with `preflight|scope|narrow` ran
  3867 tests. It failed only the two cells above (since fixed) and the two
  tests Phase 2 called pre-existing (`document_environment_routes::successive_sequence_steps_each_read_the_map_they_captured`,
  `lifecycle_message_drain::stalled_deliveries_share_one_drain_budget_and_are_reported_as_unknown`).
  After the fix, `claudine/` `just test` passed **10600/10600** (9 skipped),
  **including both of those tests**, so in this run they behaved as
  load-sensitive rather than as deterministic failures. `claudine/`
  `just lint`: clean.
- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` →
  `packages: ['claudine', 'darkmatter', 'test-toolkit', 'dmls']`,
  `change_class: package`. The working tree still holds Phase 2's
  uncommitted work too. Gates were required, and they were run.
- No cross-OS check: the change is in-memory type analysis with no path,
  process, or platform code.
- Skill: no darkmatter skill edit in this phase. The stale equality prose in
  `.claude/skills/darkmatter/expressions.md:86,107-108` belongs to Phase 6
  (worksheet row), together with the docs that teach the old guard table.
