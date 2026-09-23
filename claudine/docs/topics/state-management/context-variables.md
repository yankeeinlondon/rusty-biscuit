# Context Variables

Context variables let a Markdown prompt use facts about the machine and project
where it runs: today's date, the repository name, the current package, or the
operating system. **Darkmatter gathers these facts; Claudine makes them available
when composing a document.**

Each variable starts with `ctx.`. Put it inside `{{ … }}` to insert its value
into the composed text:

```markdown
Today is {{ ctx.today }}.
Repository: {{ ctx.repo || "no repository detected" }}.
```

The braces are replaced with the expression's result. In the second line,
`||` supplies fallback text when the repository name is missing or empty.
See the [Expression Engine](expression-engine.md) guide for the expression
syntax and other ways to use values.

## Find a variable and inspect its value

Start with the list of available variables:

```sh
claudine context
```

The report has three columns: **Property**, **Type**, and **Description**. It
explains the available variables without gathering live machine or project
information.

To see their values in your current environment, run:

```sh
claudine context --values
```

This keeps the Property and Type columns and replaces Description with
**Value**. It gathers one snapshot and uses it throughout the report. A variable
with an unavailable value remains visible as `null`.

The reports group variables by purpose. Common examples include:

| Information | Examples |
|---|---|
| Invocation directory | `ctx.cwd` |
| Date and time | `ctx.today`, `ctx.now`, `ctx.timezone` |
| Repository and package scope | `ctx.repo`, `ctx.is_monorepo`, `ctx.current_package`, `ctx.area` |
| Languages and tools | `ctx.programming_language`, `ctx.package_manager` |
| Operating system | `ctx.os`, `ctx.os_distro`, `ctx.os_version` |
| Hardware | `ctx.cpu_cores`, `ctx.gpu` |

The command is the complete reference; this table is a starting point.

## Understand the types and missing values

The **Type** column describes the kind of value a variable provides:

| Type label | Meaning |
|---|---|
| `string` | Text, such as a repository name. |
| `boolean` | `true` or `false`. |
| `number(integer)` | A whole number, such as a CPU core count. |
| `date`, `datetime`, `time` | A date, a date and time, or a time value. |
| `string[]` | An array of text values, such as package names. |
| `object` or `object[]` | Structured properties, or an array of structured objects. |

A variable can be unavailable even though its Type column says `string` or
`number(integer)`. Whether a value is required is recorded separately in the
schema; optional values may be `null`. Some variables use an empty string or
empty array when there is nothing to report.

Provide a fallback when an absent value would make the prompt confusing:

```markdown
Package: {{ ctx.current_package || "no package selected" }}
GPU: {{ ctx.gpu || "no GPU detected" }}
```

Fallbacks apply to all falsy values, including zero and `false`, not just
`null`. See [Truthiness](expression-engine.md#truthiness) before using a fallback
for a number or boolean.

Arrays remain arrays inside expressions, even though the values report displays
their elements separated by commas. You can use indexing and collection
functions on them.

## Binding time: eager `ctx`, lazy `current`

`ctx` and `current` are the same data structure, and so are `env` and
`current_env`; they differ only in *when* each key is evaluated.

| Surface | Binding |
|---------|---------|
| `ctx.<key>` | Eager — captured once at the start of the run and shared by the whole run |
| `current.<key>` | Lazy — the same key as `ctx.<key>`, observed when the reference is reached |
| `env.<key>` | Eager — the frozen invocation snapshot |
| `current_env.<key>` | Lazy — the same key as `env.<key>`, reread from the live process environment when the reference is reached |

The spelling is a direct mirror: write `current.branch` for the live value of
`ctx.branch` and `current_env.HOME` for the live value of `env.HOME`. `current`
does not contain a `ctx` member and `current_env` does not contain an `env`
member; a nested path under either root names no member and fails as an
unknown path.

**Fixed versus refreshable.** Lazy does not mean everything can change.
Repository metadata and topology (`repo`, `repo_root`, `packages`, `area`, and
the rest of the repository keys) are fixed by the request's repository
observation, captured once when the request is created, so `current.repo`
always reads what `ctx.repo` does. Only mutable Git and filesystem facts
refresh at reference time: `branch`, recent history (`recent_commits`),
`dirty_files` and the other working-tree keys, and every `current_env.<key>`.
The invocation directory and the root document's identity (`cwd`, `self`,
`hash`, `id`, `sid`) are request-owned and never refresh either.

**Variable and function pairs.** Expression functions are evaluated lazily, at
call time. When a context variable and a function share a name they share one
definition and one output format: the variable is the eager snapshot and the
function is the lazy, parameterized form. `ctx.recent_commits` holds the last
10 commits captured at the start of the run; `recent_commits(count)` walks the
newest `count` commits when the call is reached. Both are projected from one
descriptor entry, so `claudine context` and `claudine context --expressions`
cannot drift.

Under Claudine, `current` and `current_env` are Darkmatter reserved roots
served by the invocation's refresh capability: a key that capability does not
hold renders `null` with a `PartialRuntimeCapture` diagnostic rather than
probing the host. Their primary use is inside lifecycle events, where state may
have changed since launch — see
[Lifecycle — Binding Time: Early vs Late](../lifecycle.md#binding-time-early-vs-late)
and
[Composition — Launch-Anchored Prepared Context](../composition.md#launch-anchored-prepared-context).

## Use project information in a prompt

The repository context comes from where the workflow was launched. A prompt
stored outside that repository can still read its name through `ctx.repo`.
Reading the repository name does not require also referencing a Git variable
such as `ctx.branch`.

`ctx.area` identifies the current package or package area within a monorepo—a
repository containing multiple packages. It is empty at the repository root
and outside a monorepo. For display, use:

```markdown
Scope: {{ ctx.area || "whole project" }}
```

Context variables also work in conditions. This block includes its instructions
only when the current repository is a monorepo:

```markdown
::block when="ctx.is_monorepo"
Check which package this task affects before editing.
::end-block
```

These values describe the captured environment; they are not a continuously
refreshing view. Composition can gather only the groups referenced by the
document, while `claudine context --values` gathers the report's full snapshot.

## If a variable name is misspelled

Claudine checks context references during composition preparation. A typo such
as `{{ ctx.toady }}` can produce a warning suggesting `ctx.today`. The diagnostic
is non-fatal and is suppressed by `--silent`; it does not change how missing
values resolve during expression evaluation.

## Implementation reference

The remaining sections are for contributors adding or maintaining variables.

### Where descriptions and values come from

The context catalog is derived from the `ctx` block in the embedded
[Darkmatter base schema](../../../../darkmatter/docs/schemas/darkmatter.yaml).
It supplies each variable's name, type, description, required/generated flags,
and any default. The
[catalog module](../../../../darkmatter/lib/src/markdown/compose/context/catalog.rs)
adds presentation categories through `CONTEXT_VARIABLE_GROUPING`.

`context_variable_descriptors()` exposes those records. The first access parses
the compiled-in schema and caches the result; it does not capture runtime
context or read a schema file from disk. Descriptors implement the shared
`Described` trait for exact lookup, suggestions, and error descriptions.

Actual values are collected by the
[capture modules](../../../../darkmatter/lib/src/markdown/compose/context/capture/)
and stored in `ComposeContext` as named JSON values. The
[CLI context command](../../../cli/src/commands/context/mod.rs) combines these
values with descriptors for the live report.

The compose cache deliberately excludes volatile `memory_used` and
`memory_avail` values from its context hash. See
[cache hashing](../../../../darkmatter/lib/src/markdown/compose/cache/hashing.rs).

### How to add a context variable

1. Declare it in the embedded base schema's `ctx` block with an accurate type,
   description, and applicable flags or default.
2. Add its category and subsection to `CONTEXT_VARIABLE_GROUPING`.
3. Populate its runtime value in the appropriate capture module, including
   handling unavailable data and capture-group selection.
4. Run the relevant catalog and capture tests.

The schema name and captured key must match exactly. The CLI reads the catalog,
so its report needs no separate variable entry.

### What the tests check

| Test | What it checks |
|---|---|
| `projected_descriptors_match_base_schema` | Descriptor metadata matches the schema. |
| `grouping_map_is_total` | Every schema variable has a presentation group, with no obsolete entries. |
| `every_descriptor_has_a_captured_runtime_key` | Catalog names and captured keys match in both directions. |
| `capture_shape_matches_projected_type` | Captured values have the expected general JSON shapes, allowing `null` for optional variables. |
| `values_report_captures_context_exactly_once` | The live report reuses a single capture. |

Context descriptors currently have no example records. These checks validate
metadata and captured shapes; they do not prove every description correct or
every possible machine configuration covered. See [Drift Control](drift.md)
for the broader guarantees and limitations.
