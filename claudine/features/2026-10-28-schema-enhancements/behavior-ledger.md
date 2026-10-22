# Behavior Ledger: Expression Function Migration

> **Reviewed by the author on 2026-10-22 (criterion 13).** Every row is
> approved as written, including the proposed behavior of each row that was
> marked "ruling requested", the criterion 44 signature (`string | null`) for
> `has_command` and its analogues, and type errors (not `false`) when a yes/no
> check gets the wrong kind of value. `keys(null)` and `values(null)` return
> `[]`. Shipped documents, examples, and tests that rely on a changed result
> are updated in the same change.

This ledger records, for every registered Darkmatter expression function, what
happens when the one-pass migration moves argument binding into the coercion
engine: the target signature, what the function does with `null`, whether
today's behavior is preserved or intentionally changed and why, and which
dispatch-level fixtures (criterion 10) each function needs.

It starts from `current-state-coercion.md`, the coercion-baseline spike
(`spikes/coercion-baseline/baseline.json`, `ledger-draft.md`,
`signature-decisions.md`), and the spec. The function list is the registry
enumeration in [`inventory/function-registry.md`](inventory/function-registry.md):
**113 canonical functions**, one row each. Aliases are checked separately in
[Aliases](#aliases).

## How to read a row

- **Target signature.** Taken from the spec where it decides one, otherwise
  from the draft catalog (`claudine/docs/schemas/partials/functions.yaml`) with
  the null arm the rule below requires, otherwise "TBD Phase 4". Draft
  signatures are inputs for Phase 4, not decisions.
- **Null outcome.** Assigned by the spec's rule ("Which Functions Accept
  Null") and Necessary Rule 3: only an explicit `| null` arm admits null, even
  on an `unknown` parameter, so a predicate that inspects null declares
  `unknown | null`. The families are:
  - **text transform**: declares `| null`, returns `null` (`upper(null)` → `null`)
  - **predicate**: declares `| null`, returns `false` (`has_key(null, "a")` →
    `false`); `is_null(null)` is `true` because inspecting null is its purpose
  - **arithmetic and comparison**: no null arm, `null` is a type error
    (`min(null, 5)`)
  - anything else names the nearest analogy and the reason
- **Preserved or changed.** "Preserved" means no recorded outcome changes.
  "Intentionally changed (widening only)" means only calls that fail today
  start to succeed, typically number or boolean → text. "Intentionally
  changed" means at least one call that works today fails or returns
  something else; the reason column says which.
- **Fixtures.** Each function reserves an accepted, a rejected, and a
  converted-input fixture through normal dispatch. A cell reads "reserved" or
  "impossible: reason". Impossible cells are covered as decided in Necessary
  Rule 11 (decided 2026-10-22). Functions that reach Git, a provider, the network, or the shell
  need a stubbed context or a deny-all authority for their accepted fixture.
- Today's behavior comes from the baseline recorded on 2026-09-26 for 110
  functions. `code_link`, `find_files`, and `try_frontmatter` were added later
  and have no baseline outcomes; their rows come from reading the handlers.

## Ledger

| # | Function | Target signature | Null outcome; family or analogy | Preserved or changed | Reason for change | Fixture: accepted | Fixture: rejected | Fixture: converted input | Status |
|---:|---|---|---|---|---|---|---|---|---|
| 1 | `is_null` | `is_null(x: unknown \| null): boolean` | `true`; predicate whose purpose is inspecting null | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 2 | `is_empty` | `is_empty(x: unknown \| null): boolean` | `true`; predicate whose purpose includes null (absence predicate, like `is_null`) — decided 2026-10-22 | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 3 | `is_string` | `is_string(x: unknown \| null): boolean` | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 4 | `is_number` | `is_number(x: unknown \| null): boolean` | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 5 | `is_array` | `is_array(x: unknown \| null): boolean` | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 6 | `is_object` | `is_object(x: unknown \| null): boolean` | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 7 | `is_positive` | `is_positive(val: number \| null): boolean` (draft `val: number`; null arm added by the rule) | `false`; predicate — decided 2026-10-22 (today an error) | intentionally changed | Null: error → `false` (predicate family). The engine replaces the handler's own conversion: `true` becomes a type error (today `true`/`false` by sign of 1/0); `" 4 "` and `"1_000"` now convert (today errors). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 8 | `is_negative` | `is_negative(val: number \| null): boolean` (draft `val: number`; null arm added by the rule) | `false`; predicate — decided 2026-10-22 (today an error) | intentionally changed | Null: error → `false` (predicate family). The engine replaces the handler's own conversion: `true` becomes a type error (today `true`/`false` by sign of 1/0); `" 4 "` and `"1_000"` now convert (today errors). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 9 | `is_integer` | `is_integer(val: unknown \| null): boolean` | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 10 | `min` | `min(a: number, b: number): number` | type error; arithmetic | intentionally changed | Null: propagates `null` today → type error (spec null example). Numeric text now converts (`min("4", 5)` → `4`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 11 | `max` | `max(a: number, b: number): number` | type error; arithmetic | intentionally changed | Null: propagates `null` today → type error (spec null example). Numeric text now converts (`min("4", 5)` → `4`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 12 | `abs` | `abs(x: number): number` | type error; arithmetic | intentionally changed | Null: propagates today → type error. Numeric text now converts. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 13 | `round` | `round(x: numberlike, fallback?: number): number` (spec, charter M14) | `x` fails to bind → supplied `fallback` (rounded), else type error; charter recovery | intentionally changed | Charter M14: `round("pear")` and `round(null)` error instead of `0`; booleans no longer read as 1/0; fallback renamed from `default` and validated on its own. | reserved | reserved | reserved (via `fallback`: `round("pear", "2.7")` → `3`; `x` is `numberlike` and delivered as sent) | reviewed 2026-10-22 |
| 14 | `first` | `first(x: any[] \| null): any` (draft `x: any[]`) | `null`; nearest analogy text transform (selection passes absence through) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 15 | `last` | `last(x: any[] \| null): any` (draft `x: any[]`) | `null`; nearest analogy text transform (selection passes absence through) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 16 | `has_key` | `has_key(obj: object \| null, key: string \| null): boolean` (draft without null arms) | `false`; predicate (criterion 43) | intentionally changed | Null preserved (`false`). A non-object `obj` other than null (`"x"`, `1`, `[]`) becomes a type error (today `false`, 44 probes); a list/object `key` becomes a type error (today stringified). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 17 | `contains` | `contains(haystack: string \| any[] \| object \| null, needle: unknown \| null): boolean` (spec M10; draft without null arms) | haystack `null` → `false` (predicate); needle `null` admitted and rendered as empty text (M10), so `contains("abc", null)` stays `true` | preserved | — | reserved | reserved | reserved | reviewed 2026-10-22 |
| 18 | `length` | `length(val: string \| any[] \| object): number` (spec M11); null arm per ruling | proposed: type error; nearest analogy arithmetic (a count) — decided 2026-10-22 (alternatives: `\| null` → `null`, or keep `0`) | intentionally changed | M11: booleans become a type error (today `0`); objects keep counting keys; proposed null change `0` → type error. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 19 | `number` | `number(x: numberlike, fallback?: number): number` (spec, charter M12–M13) | `x` fails to bind → supplied `fallback`, else type error; charter recovery | intentionally changed | Charter M12–M13 and Per-function item 1: unconvertible input errors instead of `0`; `" 4 "` → `4` (today `0`); `true` no longer `1`; `def_val`/`default` renamed `fallback` and validated on its own. | reserved | reserved | reserved (via `fallback`: `number("pear", "7")` → `7`; `x` is `numberlike` and delivered as sent) | reviewed 2026-10-22 |
| 20 | `as_line_separated` | `as_line_separated(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 21 | `as_csv` | `as_csv(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 22 | `as_tsv` | `as_tsv(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 23 | `as_space_separated` | `as_space_separated(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 24 | `as_unordered_list` | `as_unordered_list(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 25 | `as_ordered_list` | `as_ordered_list(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 26 | `as_json` | `as_json(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 27 | `as_json5` | `as_json5(list: any[] \| null): string` (draft `list: any[]`) | `null`; text transform (renderer) | preserved | — | reserved | reserved | impossible: `any[]` never wraps a scalar or parses text, and its unconstrained elements are kept, so no input is converted | reviewed 2026-10-22 |
| 28 | `starts_with` | `starts_with(x: string \| null, find: string \| null): boolean` (draft without null arms) | `false`; predicate (String Predicates) — decided 2026-10-22 (today `null`) | intentionally changed | Null: `null` → `false` (predicate family). Numbers/booleans now convert to text (today type errors). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 29 | `ends_with` | `ends_with(x: string \| null, find: string \| null): boolean` (draft without null arms) | `false`; predicate (String Predicates) — decided 2026-10-22 (today `null`) | intentionally changed | Null: `null` → `false` (predicate family). Numbers/booleans now convert to text (today type errors). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 30 | `lower` | `lower(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 31 | `upper` | `upper(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 32 | `capitalize` | `capitalize(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 33 | `kebab_case` | `kebab_case(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 34 | `snake_case` | `snake_case(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 35 | `camel_case` | `camel_case(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 36 | `pascal_case` | `pascal_case(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 37 | `title_case` | `title_case(x: string \| null): string \| null` (draft `x: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text (`upper(2026)` → `"2026"`, criterion 15); today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 38 | `without_date` | `without_date(string: string \| null): string \| null` (draft `string: string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text; today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 39 | `ensure_leading` | `ensure_leading(var: string \| number \| null, prefix: string \| number \| null): string \| number \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Booleans select the `string` option and become `"true"`/`"false"` (today rejected); lists/objects stay errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 40 | `ensure_trailing` | `ensure_trailing(var: string \| number \| null, postfix: string \| number \| null): string \| number \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Booleans select the `string` option and become `"true"`/`"false"` (today rejected); lists/objects stay errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 41 | `replace` | `replace(x: string \| null, find: string \| null, replacement: string \| null): string \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text; today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 42 | `replace_first` | `replace_first(x: string \| null, find: string \| null, replacement: string \| null): string \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text; today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 43 | `replace_last` | `replace_last(x: string \| null, find: string \| null, replacement: string \| null): string \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text; today type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 44 | `terminal` | `terminal(content: string \| null): string \| null` (draft; parameter renamed from `string`) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers and booleans now convert to text; parameter renamed `content`. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 45 | `date` | `date(iso: string \| null, fmt: string \| null): string \| null` (draft); refined `date` for `iso` is TBD Phase 4 | `null`; text transform (Date Formatting) | intentionally changed (widening only; null preserved) | Numbers convert to text and then fail date parsing as a domain error unless `iso` is refined to `date` (signature-decisions §4). | reserved | reserved | reserved (via `fmt`); becomes impossible-success if Phase 4 refines both `iso` and `fmt` | reviewed 2026-10-22 |
| 46 | `is_date` | `is_date(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 47 | `is_date_utc` | `is_date_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 48 | `is_date_time` | `is_date_time(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 49 | `is_date_time_utc` | `is_date_time_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 50 | `is_today` | `is_today(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 51 | `is_today_utc` | `is_today_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 52 | `is_yesterday` | `is_yesterday(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 53 | `is_yesterday_utc` | `is_yesterday_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 54 | `is_tomorrow` | `is_tomorrow(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 55 | `is_tomorrow_utc` | `is_tomorrow_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 56 | `is_this_month` | `is_this_month(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 57 | `is_this_month_utc` | `is_this_month_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 58 | `is_this_year` | `is_this_year(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 59 | `is_this_year_utc` | `is_this_year_utc(x: unknown \| null): boolean` (draft `x: any`) | `false`; predicate | preserved | — | reserved | reserved (arity only: every value binds) | impossible: the only parameter is preservation-only `unknown \| null`, so the engine keeps every input | reviewed 2026-10-22 |
| 60 | `date_delta` | `date_delta(date1: string, date2: string, diff: string): boolean` (draft); refined `date`/duration types TBD Phase 4 | type error; comparison (date arithmetic) — decided 2026-10-22 (predicate analogy would give `false`) | intentionally changed | Null: propagates today → type error. Numbers now convert to text and fail as domain errors unless parameters are refined. | reserved | reserved | reserved; success is unverified: no number/boolean text is an ISO date, so success depends on `diff` accepting a bare number (`"14"`); otherwise impossible-success | reviewed 2026-10-22 |
| 61 | `older_than` | `older_than(date1: string, date2: string, diff: string): boolean` (draft); refined `date`/duration types TBD Phase 4 | type error; comparison (date arithmetic) — decided 2026-10-22 (predicate analogy would give `false`) | intentionally changed | Null: propagates today → type error. Numbers now convert to text and fail as domain errors unless parameters are refined. | reserved | reserved | reserved; success is unverified: no number/boolean text is an ISO date, so success depends on `diff` accepting a bare number (`"14"`); otherwise impossible-success | reviewed 2026-10-22 |
| 62 | `newer_than` | `newer_than(date1: string, date2: string, diff: string): boolean` (draft); refined `date`/duration types TBD Phase 4 | type error; comparison (date arithmetic) — decided 2026-10-22 (predicate analogy would give `false`) | intentionally changed | Null: propagates today → type error. Numbers now convert to text and fail as domain errors unless parameters are refined. | reserved | reserved | reserved; success is unverified: no number/boolean text is an ISO date, so success depends on `diff` accepting a bare number (`"14"`); otherwise impossible-success | reviewed 2026-10-22 |
| 63 | `predict_conflicts` | `predict_conflicts(branch: string \| null): string[] \| null` (draft without null arm) | `null`; nearest analogy text transform (Git accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text (branch `123`). | reserved (needs a Git fixture) | reserved | reserved | reviewed 2026-10-22 |
| 64 | `branch_exists_on_remote` | `branch_exists_on_remote(void)` / `(branch: string)` / `(branch: string, remote: string)`, all `: boolean` (draft; overloads) | type error; omission is expressed by the shorter overloads, so no null arm | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text (today type errors). | reserved (needs an allowlisted remote fixture) | reserved | reserved (via `branch`/`remote`; the void overload has no argument) | reviewed 2026-10-22 |
| 65 | `remote_vendor` | `remote_vendor(remote?: string): enum("", github, …)` (draft) | type error; optional means omission, not null | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text. | reserved (needs a repository fixture) | reserved | reserved | reviewed 2026-10-22 |
| 66 | `recent_commits` | `recent_commits(count: number(integer; min(1))): string[]` (spec item 11; draft adds `max(9007199254740991)`) | type error; arithmetic (a count) | intentionally changed (widening only; null preserved) | Numeric text now converts (`"3"` → `3`); `pair` replaced by a shared named type. | reserved (needs a repository fixture) | reserved | reserved | reviewed 2026-10-22 |
| 67 | `package_area` | `package_area(where: file): string` (draft) | type error; nearest analogy arithmetic (no repository answer for an absent location) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved (needs a repository fixture) | reserved | reserved | reviewed 2026-10-22 |
| 68 | `package` | `package(where: file): string` (draft) | type error; nearest analogy arithmetic (no repository answer for an absent location) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved (needs a repository fixture) | reserved | reserved | reviewed 2026-10-22 |
| 69 | `ipv4` | `ipv4(filter?: string): string[]` (draft) | type error; optional means omission, not null — decided 2026-10-22 | intentionally changed | Today `null` is read as "filter omitted"; the spec separates omission from `null`. | reserved (needs captured Network context) | reserved | reserved | reviewed 2026-10-22 |
| 70 | `ipv6` | `ipv6(filter?: string): string[]` (draft) | type error; optional means omission, not null — decided 2026-10-22 | intentionally changed | Today `null` is read as "filter omitted"; the spec separates omission from `null`. | reserved (needs captured Network context) | reserved | reserved | reviewed 2026-10-22 |
| 71 | `ping` | `ping(address: ip-address, timeout?: number): boolean \| null` (draft) | type error for both; optional means omission | intentionally changed | `timeout` `null` propagated `null` today → type error. `address` `null` stays a type error. | reserved (deny-all ICMP authority) | reserved | reserved (via `timeout`: `"500"` → `500`) | reviewed 2026-10-22 |
| 72 | `ping_under` | `ping_under(address: ip-address, timeout: number, attempts?: number(integer; min(1); max(100))): boolean \| enum("unstable") \| null` (spec item 8) | type error for all; optional means omission | intentionally changed | `attempts` `null` propagated `null` today → type error; `literals`/`nullable` become a union return. | reserved (deny-all ICMP authority) | reserved | reserved (via `timeout`/`attempts`) | reviewed 2026-10-22 |
| 73 | `pr` | `pr(id: number(integer; min(1)) \| string(min-length(1))): string` (draft; signature-decisions suggests `url` for `pr`) | type error; nearest analogy arithmetic (an identifier) | preserved | — (null was a domain error today and becomes a binding type error; callers see an error either way) | reserved (needs an allowlisted provider fixture) | reserved | reserved | reviewed 2026-10-22 |
| 74 | `pr_list` | `pr_list(query: object): string[]` / `pr_list(count: number(integer; min(1); max(100))): string[]` (spec item 11; overloads) | type error; arithmetic (a count) or object query | intentionally changed (widening only; null preserved) | `"3"` now converts to `3`; `0` is a type error at binding (criterion 23). | reserved (needs an allowlisted provider fixture) | reserved | reserved | reviewed 2026-10-22 |
| 75 | `cicd` | `cicd(id: number(integer; min(1)) \| string(min-length(1))): string` (draft; signature-decisions suggests `url` for `pr`) | type error; nearest analogy arithmetic (an identifier) | preserved | — (null was a domain error today and becomes a binding type error; callers see an error either way) | reserved (needs an allowlisted provider fixture) | reserved | reserved | reviewed 2026-10-22 |
| 76 | `cicd_list` | `cicd_list(query: object): string[]` / `cicd_list(count: number(integer; min(1); max(100))): string[]` (spec item 11; overloads) | type error; arithmetic (a count) or object query | intentionally changed (widening only; null preserved) | `"3"` now converts to `3`; `0` is a type error at binding (criterion 23). | reserved (needs an allowlisted provider fixture) | reserved | reserved | reviewed 2026-10-22 |
| 77 | `absolute` | `absolute(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 78 | `relative` | `relative(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 79 | `file_exists` | `file_exists(value: unknown \| null): boolean` (spec "`file_exists` Has No Exception"; the explicit null arm is required by Necessary Rule 3) | `false`; predicate (spec example) | preserved | Spec decision 82: the argument is inspected, not converted (`file_exists(42)` → `false`). | reserved | reserved (arity only: every value binds) | impossible: preservation-only `unknown \| null`; criterion 42 requires `file_exists(42)` to stay `false` without conversion | reviewed 2026-10-22 |
| 80 | `find_files` | TBD Phase 4 (not in the draft; catalog today `find_files(pattern: string): file`) | proposed: `pattern: string \| null`, handler keeps `[]`; nearest analogy text transform — decided 2026-10-22 (`[]` vs `null`) | preserved (proposed) | No baseline outcome; today `find_files(null)` → `[]` (`functions/mod.rs:2580`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 81 | `has_command` | TBD Phase 4; criterion 44 implies `has_command(cmd: string \| null): boolean` (draft `cmd: any` contradicts it) | `false`; predicate | intentionally changed | Criterion 44: `has_command(true)` looks for a command named `true` (today `false`); lists/objects become type errors (today `false`). | reserved | reserved | reserved (`has_command(true)`, criterion 44) | reviewed 2026-10-22 |
| 82 | `has_binary` | TBD Phase 4; same shape as `has_command` (draft `name_or_path: any`) | `false`; predicate | intentionally changed | Shares `has_command_fn`: booleans now probe `"true"`/`"false"`; lists/objects become type errors. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 83 | `is_indexed_file` | `is_indexed_file(file: file \| null): boolean` (draft without null arm) | `false`; predicate | intentionally changed | Null: `null` today → `false` (predicate family). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 84 | `file_index` | `file_index(file: file \| null): number \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 85 | `increment_file_index` | `increment_file_index(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 86 | `decrement_file_index` | `decrement_file_index(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 87 | `find_first_index` | `find_first_index(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 88 | `find_last_index` | `find_last_index(file: file \| null): file \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 89 | `basename` | `basename(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 90 | `basename_without_index` | `basename_without_index(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 91 | `dirname` | `dirname(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 92 | `ext` | `ext(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 93 | `parent_dir` | `parent_dir(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 94 | `file_trailing` | `file_trailing(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 95 | `dir_leading` | `dir_leading(file: file \| null): string \| null` (draft without null arm) | `null`; text transform (path accessor) | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 96 | `join` | `join(left: string \| null, right: string \| null): string \| null` (draft without null arms) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 97 | `link` | `link(file: file \| null): string \| null` / `link(target: file \| null, desc: string \| null): string \| null` (draft without null arms; overloads) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 98 | `code_link` | TBD Phase 4 (not in the draft); proposed to mirror `link` | `null`; text transform (shares `resolve_link_parts` with `link`) | preserved | No baseline outcome; today `null` → `null` (`functions/mod.rs:2378`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 99 | `has_alias` | TBD Phase 4; criterion 44 analogy implies `has_alias(name: string \| null): boolean` (draft `name: any`) | `false`; predicate | intentionally changed | Booleans now probe `"true"`/`"false"` (today `false`); lists/objects become type errors (today `false`). | reserved (deny-all shell authority) | reserved | reserved | reviewed 2026-10-22 |
| 100 | `has_builtin_function` | TBD Phase 4; criterion 44 analogy implies `has_builtin_function(name: string \| null): boolean` (draft `name: any`) | `false`; predicate | intentionally changed | Booleans now probe `"true"`/`"false"` (today `false`); lists/objects become type errors (today `false`). | reserved (deny-all shell authority) | reserved | reserved | reviewed 2026-10-22 |
| 101 | `has_user_function` | TBD Phase 4; criterion 44 analogy implies `has_user_function(name: string \| null): boolean` (draft `name: any`) | `false`; predicate | intentionally changed | Booleans now probe `"true"`/`"false"` (today `false`); lists/objects become type errors (today `false`). | reserved (deny-all shell authority) | reserved | reserved | reviewed 2026-10-22 |
| 102 | `can_execute` | TBD Phase 4; criterion 44 analogy implies `can_execute(name: string \| null): boolean` (draft `name: any`) | `false`; predicate | intentionally changed | Booleans now probe `"true"`/`"false"` (today `false`); lists/objects become type errors (today `false`). | reserved (deny-all shell authority) | reserved | reserved | reviewed 2026-10-22 |
| 103 | `has_agentic_cli` | `has_agentic_cli(agent: agentic-cli): boolean` (draft; enum of roster names) + null arm | `false`; predicate — decided 2026-10-22 (today an argument error) | intentionally changed | Null: argument error → `false`. | reserved | reserved | impossible (success): no number or boolean text form is a roster name, so every converted input then fails the enum check; a fixture can still assert conversion followed by rejection | reviewed 2026-10-22 |
| 104 | `has_skill` | `has_skill(name: string \| null): boolean` (draft without null arm) | `false`; predicate | intentionally changed | Null: `null` today → `false` (predicate family). Numbers/booleans now convert to text. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 105 | `has_local_skill` | `has_local_skill(name: string \| null): boolean` (draft without null arm) | `false`; predicate | intentionally changed | Null: `null` today → `false` (predicate family). Numbers/booleans now convert to text. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 106 | `frontmatter` | `frontmatter(file: file \| null): object \| null` / `(file: file \| null, prop: string): any` (draft; null arm on `file` only) | `file` `null` → `null`; text transform (document reader). `prop` `null` → type error | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. Highest-traffic function (about 423 call sites); keeping `file` null-propagating avoids breaking optional-path documents. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 107 | `try_frontmatter` | TBD Phase 4 (not in the draft; catalog today `try_frontmatter(file: file): object?`) | proposed: `null`; text transform (document reader) | preserved (proposed) | No baseline outcome; today `null` → `null` (`functions/mod.rs:2537`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 108 | `markdown_body_empty` | `markdown_body_empty(file: file \| null): boolean` (draft without null arm) | proposed: `false`; predicate analogy — decided 2026-10-22 (today `null`) | intentionally changed | Null: `null` → `false` (predicate analogy). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 109 | `markdown_title` | `markdown_title(file: file \| null): string \| null` (draft without null arm) | `null`; text transform | intentionally changed (widening only; null preserved) | Numbers/booleans now convert to text paths. | reserved | reserved | reserved | reviewed 2026-10-22 |
| 110 | `validate_schema` | `validate_schema(file: file \| null): boolean \| null` / `validate_schema(file: file \| null, obj: object): boolean \| null` (spec M9; draft; overloads) | proposed: `file` `null` → `null` (document-reader analogy) — decided 2026-10-22 (predicate analogy would give `false`); `obj` `null` → type error | intentionally changed | M9: `obj` is merged and validated (today ignored); a non-object or `null` `obj` becomes a type error (today accepted, returned `true`). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 111 | `as_markdown` | `as_markdown(content: string \| null): string \| null` (draft without null arm) | proposed: `null`; text transform — decided 2026-10-22 (today an argument error) | intentionally changed | Null: argument error → `null` (text-transform family). | reserved | reserved | reserved | reviewed 2026-10-22 |
| 112 | `and` | `and(...values: unknown \| null): boolean` (draft `any`; lazy in registration) | `null` admitted as a falsy value; domain truthiness | preserved | — (lazy short-circuiting kept, criterion 53) | reserved | impossible: every value binds and every arity, including zero, is eligible (`functions/mod.rs:6031`) | impossible: preservation-only `unknown \| null` variadic | reviewed 2026-10-22 |
| 113 | `or` | `or(...values: unknown \| null): boolean` (draft `any`; lazy in registration) | `null` admitted as a falsy value; domain truthiness | preserved | — (lazy short-circuiting kept, criterion 53) | reserved | impossible: every value binds and every arity, including zero, is eligible (`functions/mod.rs:6031`) | impossible: preservation-only `unknown \| null` variadic | reviewed 2026-10-22 |

## Planned functions (not yet registered)

These join the ledger as full rows when they are registered (spec "Map
Functions", criterion 45).

| Function | Target signature | Null outcome; family or analogy | Status |
|---|---|---|---|
| `keys` | `keys(obj: object \| null): string[]` | `[]`; author decision 2026-10-22: a missing map lists no keys (nearest analogy `find_files(null)` → `[]`) | reviewed 2026-10-22 |
| `values` | `values(obj: object \| null): any[]` | `[]`; same decision as `keys` | reviewed 2026-10-22 |

## Fixture applicability summary

Criterion 10 asks for accepted, rejected, and converted-input fixtures for
every function. These cells cannot hold a successful fixture as the criterion
is written:

| Cell | Functions | Why |
|---|---|---|
| Converted input impossible: preservation-only parameter (24) | `is_null`, `is_empty`, `is_string`, `is_number`, `is_array`, `is_object`, `is_integer`, the 14 date predicates (`is_date` … `is_this_year_utc`), `file_exists`, `and`, `or` | Every parameter is `unknown \| null`; the engine keeps the input unchanged by design (criterion 18, criterion 42) |
| Converted input impossible: `any[]` parameter (10) | `first`, `last`, `as_line_separated`, `as_csv`, `as_tsv`, `as_space_separated`, `as_unordered_list`, `as_ordered_list`, `as_json`, `as_json5` | A list target never wraps a scalar or parses text, and unconstrained elements are kept. This holds under the draft signatures; narrowing the elements in Phase 4 would make conversion possible |
| Converted-input success impossible (1) | `has_agentic_cli` | Conversion runs, but no number or boolean text is a roster name, so the enum check always rejects it |
| Converted-input success unverified (4) | `date_delta`, `older_than`, `newer_than`; `date` if both parameters are refined | Depends on whether Phase 4 refines the date and duration parameters |
| Rejected impossible (2) | `and`, `or` | Every value binds and zero or more arguments are eligible, so no call is rejected |
| Rejected by arity only (22) | the 7 type predicates, the 14 date predicates, `file_exists` | Every value binds; only a wrong argument count is rejected |

No registered function is wholly void. `branch_exists_on_remote` has a void
overload, and `remote_vendor`, `ipv4`, and `ipv6` can be called with no
arguments; each still has a parameter that converts.

Recommendation, as in Necessary Rule 11: keep one registry-wide fixture row
per function that marks the impossible success explicitly, still sends the
candidate conversion inputs (`4`, `true`, `"4"`) through normal dispatch where
arguments exist and asserts they are kept unchanged, and cover actual
conversion with registered probes.

## Rulings requested

All decided by the author on 2026-10-22: each as recommended below, and
`keys`/`values` return `[]` for `null`.

1. **Fixture applicability** for the cells above (Necessary Rule 11).
2. **Null outcomes where the family is ambiguous:** `is_empty` (`true`, kept
   as an absence predicate), `is_positive`/`is_negative` (`false`, today an
   error), `starts_with`/`ends_with` (`false`, today `null`),
   `date_delta`/`older_than`/`newer_than` (type error as comparisons, today
   `null`), `length` (type error, `null`, or keep `0`), `find_files` (`[]` or
   `null`), `markdown_body_empty` (`false`, today `null`), `validate_schema`
   (`null` or `false`), `as_markdown` (`null`, today an error),
   `has_agentic_cli` (`false`, today an error), `ipv4`/`ipv6` (type error,
   today read as "omitted"), and `keys`/`values`.
3. **`trim` is not registered,** but the null table and criterion 43 use
   `trim(null)` → `null`. Either add `trim` in this feature or change the
   criterion to a registered text transform.
4. **Example count.** The spec carries over "all 117" examples; the catalog
   now holds 121 (`find_files` and `try_frontmatter` added one executable
   example each, and `code_link` two display-only). Recommended: carry over
   all 121.
5. **`has_command` and the shell probes.** The draft declares `any`, which
   would preserve the input and contradict criterion 44 (`has_command(true)`
   looks for a command named `true`). The rows assume `string | null`.

## Aliases

Aliases resolve case-insensitively to their canonical function and share its
ledger row. Each alias reserves one alias check (dispatch through the alias
gives the same result as the canonical name); the check is not a separate
ledger row. `has_binary` is a separately registered canonical name, not an
alias, and has its own row above.

| Alias | Canonical | Alias check (reserved) |
|---|---|---|
| `isnull` | `is_null` | reserved |
| `isempty` | `is_empty` | reserved |
| `isstring` | `is_string` | reserved |
| `isnumber` | `is_number` | reserved |
| `isarray` | `is_array` | reserved |
| `isobject` | `is_object` | reserved |
| `ispositive` | `is_positive` | reserved |
| `isnegative` | `is_negative` | reserved |
| `isinteger` | `is_integer` | reserved |
| `haskey` | `has_key` | reserved |
| `aslineseparated` | `as_line_separated` | reserved |
| `ascsv` | `as_csv` | reserved |
| `astsv` | `as_tsv` | reserved |
| `asspaceseparated` | `as_space_separated` | reserved |
| `asunorderedlist` | `as_unordered_list` | reserved |
| `asorderedlist` | `as_ordered_list` | reserved |
| `asjson` | `as_json` | reserved |
| `asjson5` | `as_json5` | reserved |
| `startswith` | `starts_with` | reserved |
| `endswith` | `ends_with` | reserved |
| `kebabcase` | `kebab_case` | reserved |
| `snakecase` | `snake_case` | reserved |
| `camelcase` | `camel_case` | reserved |
| `pascalcase` | `pascal_case` | reserved |
| `titlecase` | `title_case` | reserved |
| `withoutdate` | `without_date` | reserved |
| `ensureleading` | `ensure_leading` | reserved |
| `ensuretrailing` | `ensure_trailing` | reserved |
| `replacefirst` | `replace_first` | reserved |
| `replacelast` | `replace_last` | reserved |
| `isdate` | `is_date` | reserved |
| `isdateutc` | `is_date_utc` | reserved |
| `isdatetime` | `is_date_time` | reserved |
| `is_datetime` | `is_date_time` | reserved |
| `isdatetimeutc` | `is_date_time_utc` | reserved |
| `is_datetime_utc` | `is_date_time_utc` | reserved |
| `istoday` | `is_today` | reserved |
| `istodayutc` | `is_today_utc` | reserved |
| `isyesterday` | `is_yesterday` | reserved |
| `isyesterdayutc` | `is_yesterday_utc` | reserved |
| `istomorrow` | `is_tomorrow` | reserved |
| `istomorrowutc` | `is_tomorrow_utc` | reserved |
| `isthismonth` | `is_this_month` | reserved |
| `isthismonthutc` | `is_this_month_utc` | reserved |
| `isthisyear` | `is_this_year` | reserved |
| `isthisyearutc` | `is_this_year_utc` | reserved |
| `datedelta` | `date_delta` | reserved |
| `olderthan` | `older_than` | reserved |
| `newerthan` | `newer_than` | reserved |
| `predictconflicts` | `predict_conflicts` | reserved |
| `branchexistsonremote` | `branch_exists_on_remote` | reserved |
| `remotevendor` | `remote_vendor` | reserved |
| `prlist` | `pr_list` | reserved |
| `cicdlist` | `cicd_list` | reserved |
| `fileexists` | `file_exists` | reserved |
| `findfiles` | `find_files` | reserved |
| `hascommand` | `has_command` | reserved |
| `hasbinary` | `has_binary` | reserved |
| `isindexedfile` | `is_indexed_file` | reserved |
| `fileindex` | `file_index` | reserved |
| `incrementfileindex` | `increment_file_index` | reserved |
| `decrementfileindex` | `decrement_file_index` | reserved |
| `findfirstindex` | `find_first_index` | reserved |
| `findlastindex` | `find_last_index` | reserved |
| `basenamewithoutindex` | `basename_without_index` | reserved |
| `parentdir` | `parent_dir` | reserved |
| `filetrailing` | `file_trailing` | reserved |
| `dirleading` | `dir_leading` | reserved |
| `codelink` | `code_link` | reserved |
| `hasskill` | `has_skill` | reserved |
| `haslocalskill` | `has_local_skill` | reserved |
| `tryfrontmatter` | `try_frontmatter` | reserved |
| `markdownbodyempty` | `markdown_body_empty` | reserved |
| `markdowntitle` | `markdown_title` | reserved |
| `validateschema` | `validate_schema` | reserved |
