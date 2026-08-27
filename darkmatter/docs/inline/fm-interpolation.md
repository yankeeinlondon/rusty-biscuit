---
blast_radius:
  - darkmatter/features/2026-03-30-fm-interpolation/spec.md
  - darkmatter/features/2026-03-30-fm-interpolation/tech-design.md
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/compose/state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/lexer.rs
  - darkmatter/lib/src/markdown/compose/context/capture.rs
  - darkmatter/lib/src/markdown/compose/context/mod.rs
  - darkmatter/cli/src/commands.rs
---

# Frontmatter Interpolation

Frontmatter interpolation resolves `{{ ... }}` expressions inside frontmatter values before the final compose-time state is built. This allows computed frontmatter values to participate in later stages such as:

- body interpolation
- frontmatter transclusion targets like `prologue` and `epilogue`
- page-block conditions
- inherited state passed into child transclusions

## Why It Exists

Darkmatter has long supported interpolation in the markdown body:

```md
The spec is at {{ spec }}.
```

But frontmatter can also contain compose-time configuration:

```yaml
---
base: /tmp/project
spec: "{{base}}/spec.md"
prologue: "{{base}}/intro.md"
---
```

Without frontmatter interpolation, `spec` and `prologue` remain literal strings and downstream pipeline stages cannot use the resolved values.

## Pipeline Position

`FrontmatterInterpolation` is the first Inline Pre operation.

It runs:

1. After external-state defaults are applied
2. After `--set` overrides are applied
3. Before the final `EffectiveState` is built

That ordering matters because frontmatter interpolation shapes the state that later stages consume.

## Core Rule: Seed-Only Semantics

Frontmatter interpolation is a single-pass transform over top-level frontmatter keys.

Each top-level key is classified as one of:

- **Seed value**: the value tree contains no interpolation expressions anywhere
- **Templated value**: the value tree contains at least one interpolation expression somewhere

Only seed values participate in lookup for the frontmatter interpolation pass.

### What Counts As Templated

Classification happens at the top-level key, not at the individual leaf:

```yaml
---
base: /tmp/project
author: Alice
spec: "{{base}}/spec.md"
metadata:
  home: "{{base}}/docs"
  owner: Alice
---
```

In this example:

- `base` is a seed value
- `author` is a seed value
- `spec` is templated
- `metadata` is templated because one nested string contains `{{ ... }}`


### Two-Pass Interpolation

Frontmatter interpolation runs in **two passes** that bracket Frontmatter Shell
Expansion. Pass 1 runs _first_ — before Schema Validation and shell expansion —
and **defers** any templated key that references a whole-value `$(...)` shell
value, since that literal form must survive into shell expansion. Pass 2 runs
_after_ shell expansion and resolves the deferred keys against the now-concrete
values. A single pass therefore cannot always suffice.

1. Recursive Interpolation
    
    Here's an example:

    ```yaml
    area: "{{ctx.current_package_area}}"
    
    ```



## Available Variables

Frontmatter interpolation resolves against these sources:

| Prefix | Source | Example |
|---|---|---|
| *(none)* | Non-templated frontmatter seed values (plus already-resolved keys) | `{{ base }}` |
| `doc` / `doc.` | The current document's frontmatter object / a property of it | `{{ doc.base }}` |
| `ctx.` | Runtime context (demand-driven) | `{{ ctx.today }}` |
| `env.` | Environment variables | `{{ env.HOME }}` |

Bare `doc` is the whole frontmatter object; `doc.<path>` reads a property, and a
property literally named `doc` is reached as `doc.doc`. See
[Namespaces](../topics/darkmatter-expressions.md#namespaces).

### Read-Side Functions

The [read-side functions](../topics/darkmatter-expressions.md#read-side-functions)
(`file_exists`, `frontmatter`, `markdown_title`, `markdown_body_empty`,
`validate_schema`, `absolute`, `relative`) resolve in frontmatter interpolation
just as they do in body interpolation — both passes carry a resolution context
anchored on the source document's directory and its repository root (implicit
paths resolve from the document directory first, then the repository root). The motivating pattern relies on this:

```yaml
possible_spec: "{{dir}}/spec.md"
spec: "{{ file_exists(possible_spec) ? possible_spec : '' }}"
```

The frontmatter context is local-filesystem only — this holds for both the
pre-shell and post-shell interpolation passes, and for the `$()` shell ternary
condition/branch: a remote URL argument fails loudly here (use body
interpolation for remote reads).

Dotted access into nested seed values is supported:

```yaml
---
meta:
  owner:
    name: Alice
path: "{{meta.owner.name}}"
---
```

### Context Variable Groups

The `ctx.*` namespace provides 70+ runtime variables organized into demand-driven groups. Only groups whose variables are actually referenced in the document are captured, avoiding unnecessary work (e.g., git queries, subprocess calls).

| Group | Variables (examples) |
|---|---|
| DateTime | `today`, `yesterday`, `tomorrow`, `now`, `now_utc`, `year`, `month`, `month_name`, `day`, `day_abbr`, `time`, `timezone`, `season`, `timestamp` |
| Repo | `repo`, `repo_root`, `is_monorepo`, `packages`, `current_package`, `current_package_area` |
| FileChanges | `dirty_files`, `staged_files`, `untracked_files`, `dirty_packages`, `staged_packages` |
| Languages | `programming_language`, `programming_languages_in_repo`, `package_manager` |
| Documents | `docs_readme`, `docs_blast_radius`, `docs_drift`, `docs_skill` |
| Os | `os`, `os_distro`, `os_version`, `os_package_manager` |
| Hardware | `memory_total`, `memory_used`, `memory_avail`, `cpu_cores`, `cpu_arch` |
| Gpu | `gpu` |

List-valued variables (e.g. `packages`, `dirty_files`) are captured as real
arrays; render them with the list-formatting functions (`as_csv`,
`as_unordered_list`, `as_ordered_list`, …) or rely on the default line-separated
rendering of a bare `{{ ctx.foo }}`. DateTime variables have `_utc` counterparts.

## Supported Value Shapes

Frontmatter interpolation walks the full JSON/YAML value tree for each templated top-level key:

- `String`: rewritten if it contains interpolation expressions
- `Array`: each element is visited recursively
- `Object`: each field value is visited recursively
- `Number`, `Bool`, `Null`: preserved unchanged

Only string leaves are rewritten. The surrounding structure is preserved.

## Example

```yaml
---
base: /path/to/something
spec: "{{base}}/spec.md"
plan: "{{base}}/plan.md"
---
# My Document

The spec is located at: {{spec}}
The plan is located at: {{plan}}
```

After frontmatter interpolation, the frontmatter becomes:

```yaml
---
base: /path/to/something
spec: /path/to/something/spec.md
plan: /path/to/something/plan.md
---
```

Later, the normal body interpolation stage sees those resolved values and produces:

```md
# My Document

The spec is located at: /path/to/something/spec.md
The plan is located at: /path/to/something/plan.md
```

## Interaction With `--state` And `--set`

Because frontmatter interpolation runs after frontmatter has been initialized, both of these can influence the result:

- `--state` can fill missing or null frontmatter values before interpolation runs
- `--set` can override frontmatter values before interpolation runs

That means patterns like this are supported:

```yaml
---
spec: "{{base}}/spec.md"
---
```

```bash
md compose doc.md --set '{base: "/tmp/project"}'
```

The resulting `spec` value becomes `/tmp/project/spec.md`.

## Missing Variables And Errors

Frontmatter interpolation uses the same expression grammar and evaluator as body interpolation.

That means:

- missing variables resolve to the empty string
- fallbacks work: `{{ color || "unknown" }}`
- ternaries work: `{{ enabled ? "yes" : "no" }}`
- helper functions work: `{{ length(items) }}`

When `fail_fast` is enabled, parse or evaluation failures stop the compose run.
When `fail_fast` is disabled, the original string is preserved and a warning is recorded.

### Whole-Value Exception (Strict)

There is one exception to the lenient `fail_fast`-off behavior above. When a
frontmatter value's trimmed content is **exactly one** `{{ ... }}` span (only
whitespace before and after it), the value is treated as executable state, not
text, and is held to a strict parse-and-evaluate contract:

- The expression is parsed and evaluated directly, and the typed
  `serde_json::Value` result is preserved (so `{{ false }}` stays the boolean
  `false`, a numeric expression stays a number, and an array/object result keeps
  its type).
- A parse failure or an evaluation failure is **fatal regardless of
  `fail_fast`**, so malformed expansion syntax (e.g. a mismatched paren) can
  never leak downstream as a raw `{{ … }}` string.
- Undefined variables stay lenient: a whole-value `{{ missing }}` resolves to
  `null`, not an error.

This is scoped to whole-value spans only. Mixed text (`"a {{ x }}"`), strings
holding more than one expression, and body interpolation fall through to the
lenient string path described above — they are **not** newly fatal when
`fail_fast` is off.

## Important Limitation

Chained references between templated top-level keys are intentionally not supported in v1.

```yaml
---
base: /root
spec: "{{base}}/spec.md"
plan: "{{spec}}.plan.md"
---
```

Here:

- `spec` is templated, so it is excluded from the seed state
- `plan` cannot use the resolved value of `spec` during the same pass
- `{{spec}}` therefore resolves as missing and becomes an empty string

Result:

```yaml
plan: ".plan.md"
```

This constraint keeps the behavior deterministic and avoids source-order-dependent chaining rules.

## Downstream Effects

Once frontmatter interpolation has run, later compose stages see the rewritten frontmatter values.

That means it directly affects:

- body `{{ ... }}` interpolation
- `prologue` and `epilogue` references
- page-block `when="..."`
- child-document inherited state during transclusion

## Compose Reporting

The compose report tracks frontmatter interpolation separately from body interpolation.

- `ComposeOperation` variant: `FrontmatterInterpolation`
- Phase: `InlinePre`
- Report field: `frontmatter_interpolations_applied`
- Perf metric name: `frontmatter interpolation`

## Drift Detection

This document may need review when any of these files change:

- `darkmatter/features/2026-03-30-fm-interpolation/spec.md`
- `darkmatter/features/2026-03-30-fm-interpolation/tech-design.md`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/lib/src/markdown/compose/types.rs`
- `darkmatter/lib/src/markdown/compose/state.rs`
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/lexer.rs`
- `darkmatter/lib/src/markdown/compose/context/capture.rs`
- `darkmatter/lib/src/markdown/compose/context/mod.rs`
- `darkmatter/cli/src/commands.rs`
