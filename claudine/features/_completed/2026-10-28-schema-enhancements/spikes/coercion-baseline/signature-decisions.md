# Signature Decisions Surfaced by the Coercion-Baseline Spike

Throwaway spike output, 2026-09-26. Every item below is a question the one-pass
migration must answer before the ledger can say "preserved" or "intentionally
changed". Numbers come from `baseline.json` / `stats.json` in this directory.
"Draft" = `claudine/docs/schemas/partials/functions.yaml` as of this spike.

## 1. Null policy (the dominant decision)

The draft declares `| null` on **no** parameter. Under the spec's rule (a
parameter without `| null` never receives `null`; decision 20) today's null
behavior changes for **71 of 110** functions.

| What a function does with `null` today | Functions |
|---|---:|
| Returns `null` without running (null propagation) | 60 |
| Treats it as a value and returns a real result (`0`, `false`, `true`) | 36 (24 of them are `any` inspectors and are unaffected) |
| Treats it as "argument omitted" (`ipv4`, `ipv6`) | 2 |
| Rejects it with an error | 16 |

- Functions that would need `| null` somewhere to keep today's result: **71**.
- Of the 71 functions whose currently-working calls would start failing, **59
  fail only because of `null`**. Only 12 have any non-null regression.
- Null propagation is not even uniform within a function:
  `frontmatter(null)` returns `null`, `frontmatter("doc.md", null)` is an
  error; `ping("1.2.3.4", null)` returns `null`, `ping(null)` is an error.

Why it matters: a missing optional frontmatter property evaluates to `null`.
Today `{{ upper(title) }}` with no `title` renders empty; with the draft it
becomes a type error and the document fails to compose. Call sites
of affected functions are common (`frontmatter(` 423, `parent_dir(` 109,
`file_exists(` 101, `dirname(` 62 in this repo's Markdown; rough count, see
`callsites.json`).

**Recommendation — yes, a single default null policy is warranted.** Writing
`| null` on the 93 affected parameter positions (and `| null` on 71 returns) is noisy and easy to
get wrong in one pass. Two options that keep the spec's principles:

1. **Function-level flag (preferred):** a `propagates-null` flag on
   `function(...)`, like `fallible`. If any non-`| null` argument is `null`, the
   engine returns `null` without calling the function; the declared return is
   implicitly `T | null`. The function still never sees `null` (decision 20
   holds), laziness-style behavior stays in registration (decision 5), and the
   60 propagating functions keep today's behavior with one word each.
2. **Engine-wide default:** every non-`any` parameter propagates `null`. Simpler,
   but silently changes the 12 "consumed" functions and contradicts the spec's
   Null example (`min(low, high)` must be a type error).

Either way, the 12 functions that *consume* `null` as a value need one ruling
each (section 3).

## 2. Functions whose purpose is conversion

| Function | Today | Under draft | Ruling needed |
|---|---|---|---|
| `number(x: any, def_val?: number)` | Own `to_number` grammar: `" 4 "` → `0`, `"1_000"` → `0`, `true` → `1`, lists/objects → `0`, `"9007199254740993"` → `9007199254740992`. Default accepts anything; a non-number default collapses to `0` | `x` unchanged (`any`); `def_val` now rejects `true`, `"pear"`, lists, `null` (13 changed probes) | (a) Does `number()` adopt the engine's text grammar? Today `number(" 4 ")` is `0` while `min(" 4 ", 9)` would be `4`. (b) Is `number(true)` still `1` when the engine forbids boolean → number? (c) Failure result (spec item 1). (d) One name: `def_val` vs `fallback` |
| `round(x: number, default?: number)` | Same grammar as `number`, fallback `0`: `round("pear")` → `0`, `round(true)` → `1`, `round(null)` → `0`, `round([])` → `0` | `x: number` makes `"pear"`, `true`, `[]`, `null` type errors; `round(" 4 ")` changes `0` → `4` | Either `round` is arithmetic (`x: number`, drop `default`, no fallback) or a converter like `number()` (`x: any`). The draft's `x: number` plus `default` is contradictory: the fallback is unreachable |
| `is_positive` / `is_negative(val: any)` | Convert internally: `true` → positive, `"4"` → positive, `" 4 "`/`"1_000"` → error | `any` passes everything; handler still converts with a grammar different from the engine's (57 "wider than code" probes) | Make it `val: number` (then `is_positive(true)` becomes a type error; today `true`) or keep `any` and document the conversion as domain behavior. Drop `fallible` if `number` |

## 3. Functions that consume `null` / odd shapes today (non-null regressions)

| Function | Today | Draft outcome | Suggested ruling |
|---|---|---|---|
| `file_exists` | `null` → `false`; lists/objects → `false` | type error (draft `file: file`) | Draft is wrong: the spec already says `file_exists(value: string \| file \| null)`. Fix draft |
| `has_command`, `has_binary`, `can_execute` | `true`/`false` → `false`; `null`, lists, objects → `false` | `true` becomes `"true"` which **finds `/usr/bin/true`** → `true`; containers/`null` → type error | Accept as intentional, or declare `name: string(not-empty)` and treat booleans as out of scope. A real golden-rule edge case: the text is exact, but the caller almost certainly did not mean a binary named `true` |
| `has_alias`, `has_builtin_function`, `has_user_function` | `null`/containers → `false` | type error | Missing from draft; choose `name: string \| null` or accept |
| `length` | `null` → `0`; `true`/`false` → `0`; objects → key count | Draft `string \| number \| any[]` drops `object` (so `length({a:1})`, today `1`, becomes a type error) and turns `true` into `"true"` (length `4`) | Use the spec's `string \| any[] \| object`; decide whether numbers stay (today `length(2.5)` = `3`, preserved by number → text); rule on booleans |
| `has_key(obj: object, key: string)` | any non-object `obj` → `false` (44 probes) | type error | Keep `object` (intentional change) or `object \| null` |
| `validate_schema(file, obj: object)` | second argument accepted whatever its type; returns `true` for `"pear"` | type error for every non-object | Confirm what the second argument means; the handler appears to ignore its type |

## 4. Draft signatures that disagree with the code

- **Missing constraints** (handler rejects after binding): `pr_list`/`cicd_list`
  `count: number(integer)` needs `min(1); max(100)`; `recent_commits` needs
  `count: number(integer; min(1))`; `predict_conflicts` rejects `""`;
  `ping_under` attempts must be an integer 1–100; `ping` timeout has a range.
  Without these, `pr_list(0)` is not category 5 at design time as the spec's
  worked example requires.
- **Refined text shown as `string`**: `date`, `date_delta`, `older_than`,
  `newer_than` (dates and a duration), `has_agentic_cli` (enum), `ping`
  (`ip-address`). With `string`, `date(20260925, "long")` changes from a type
  error to a converted value that then fails inside the function (93 probes,
  9 functions). Refined parameter types keep these as type errors and visible
  at design time.
- **Accommodation-only `any`** (spec item 10): `ensure_leading`/`ensure_trailing`
  (today: text or number, keeps the number type → `string | number`);
  `pr(id)` (today: positive integer, digit text, or canonical URL → suggest
  `number(integer; min(1)) | url`); `cicd(id)` (suggest
  `number(integer; min(1)) | string(not-empty)`).
- **Old grammar**: `round`'s `default: number(optional)`, `remote_vendor`'s
  `remote: string(optional)` → `?:`; `branch_exists_on_remote`'s
  `parameters()` → `parameters(void)`. The stand-in binder handled all three.
- **Renamed parameters**: `terminal` `string` → `content`; `number` `default` →
  `def_val`.

## 5. The 14 functions missing from the draft

Predicted from old-catalog types. Proposed signatures:

| Function | Proposed | Note |
|---|---|---|
| `has_binary`, `can_execute`, `has_alias`, `has_builtin_function`, `has_user_function` | `name: string` | see section 3 on `null`/containers/`true` |
| `has_agentic_cli` | `agent: enum(claude, codex, ...)` | otherwise unknown names fail after binding |
| `package`, `package_area` | `where: file` | numbers now bind as text (benign) |
| `recent_commits` | `count: number(integer; min(1))` | `"3"` now binds (benign) |
| `ipv4`, `ipv6` | `filter?: string \| null` | today `null` means "omitted"; the spec says omitted ≠ `null` |
| `ping` | `address: ip-address, timeout?: number(...) \| null` | today a `null` timeout propagates `null` |
| `ping_under` | `address: ip-address, timeout: number(...), attempts?: number(integer; min(1); max(100)) \| null` | same |
| `as_markdown` | `content: string` | numbers now bind as text (benign) |

## 6. Spec-level rulings the spike suggests

1. **Default null policy** (section 1) — the single largest risk to the one-pass
   migration.
2. **Exact whole-number text does not survive the handlers.** The engine binds
   `"9007199254740993"` exactly, but `abs` (and `number`, `round`,
   `ping_under`) compute through `f64` and return `9007199254740992`.
   Criterion 21 passes only for a test-registered `echo`. Either require
   handlers to keep integers exact or scope the guarantee to binding.
3. **Boolean → text is not always harmless for lookup-style functions**
   (`has_command(true)` flips `false` → `true`). Decide whether that is
   acceptable or whether lookup parameters should reject booleans.
4. **Prefer refined types over `string` wherever a handler validates content**,
   so failures stay type errors (step 1) rather than domain errors (step 2) and
   the functions do not need `fallible` merely for bad input.
5. **The union rules barely touch today's catalog.** The draft uses unions only
   in `length`; there is no `numberlike`, `boolish`, `json`, or `yaml` parameter.
   Union and tie-breaker logic is exercised mainly by test-registered functions
   and frontmatter, not by the migration.
