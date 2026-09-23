# Context Variables

Context variables are the runtime facts a composed Markdown document can read
through the `ctx.*` namespace: the date, the repository, the current package,
the operating system, the hardware, and more. They are the *inputs* to the
[expression engine](expression-engine.md) and to `{{ … }}` interpolation.

```markdown
Today is {{ ctx.today }} and you are working in {{ ctx.current_package }}.
```

In composed prompts, `ctx.repo` is the repository name from the launch context,
including when the prompt lives outside that repository. It does not require a
reference to `ctx.branch` or another Git variable. `ctx.area` names the current
package or package area in a monorepo; it is empty at the repository root and
outside a monorepo. Use a conditional fallback when displaying that scope.

## What they are used for

During composition Darkmatter captures a snapshot of the host and project into
a `ComposeContext`, a flat map of `name → serde_json::Value`. Every entry is
addressable as `ctx.NAME` inside interpolation and `when="…"` conditions.
Variables fall into stable categories, each rendered as its own section in the
report:

- **Date and Time** — `now`, `today`, `yesterday`, `tomorrow`, and their `_utc`
  variants; `timezone`.
- **Repository** — repo root/name, monorepo flag, and the **Packages** and
  **Scope** subsections (`packages`, `current_package`, `area`, `depends_on`, …).
- **Languages** — `programming_language`, `package_manager`, and the repo-wide list.
- **Documents** — README / blast-radius / drift discovery, best-match skill.
- **Operating System** — `os`, `os_distro`, `os_version`, `os_package_manager`.
- **Hardware** — memory, CPU cores/arch, GPU.

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

## The two reports

### `claudine context` — the default report

Three columns: **Property** (`ctx.NAME`), **Type**, and **Description**. Every
variable in the catalog is shown, grouped by category and subsection. This
report is **pure** — it reads the descriptor catalog and renders it; nothing is
captured, so it is safe and instant.

### `claudine context --values` — live values

The same Property column, but the third column becomes the **live captured
Value**. This is the *only* context report that performs a real capture: it
invokes `ComposeContext::capture()` exactly once and reuses that single
snapshot for every row (enforced by the `values_report_captures_context_exactly_once`
test). Null values are shown as a dim `null`, never dropped, so an unavailable
variable is visible rather than missing.

## How the type system works

Each descriptor carries a `display_type: ContextValueType`. The enum lives in
`darkmatter/lib/src/markdown/compose/context/catalog.rs`:

```rust
pub enum ContextValueType {
    Date, DateTime, Time, Timezone,
    Integer, Number, Boolean, String,
    Csv, MarkdownList, NestedMarkdownList, Object,
    /// Value of the inner type that may be `null` when unavailable.
    Nullable(&'static ContextValueType),
}
```

`Nullable` is **parameterized by its inner type** — a `&'static` reference, so
the enum stays `Copy` and can be built in a `const`. A variable that may be
absent is described as `Nullable(String)`, `Nullable(Integer)`, etc., and the
report renders it that way:

```
ctx.os            Nullable(String)    Operating system name (Windows, macOS, Linux).
ctx.cpu_cores     Nullable(Integer)   Number of logical CPU cores.
ctx.os_distro     String              OS distribution name.
```

The CLI colors each type by category and wraps a nullable type in a grey
`Nullable( … )` around the inner type's own color
(`context_value_type_markup` in `claudine/cli/src/commands/context.rs`). A bare
`Nullable` with no inner type is no longer possible — the type must name what it
wraps.

## How values are captured

Capture happens in `darkmatter/lib/src/markdown/compose/context/capture.rs`.
A `ContextCapture` gathers host/repo/OS/hardware facts, then a family of
`populate_*` functions writes each `ctx.NAME` into the value map as a
`serde_json::Value`:

```rust
values.insert(
    "cpu_cores".into(),
    hw.map_or(Value::Null, |h| Value::Number(h.cpu.logical_cores.into())),
);
```

A few variables (`memory_used`, `memory_avail`) are deliberately treated as
volatile and excluded from compose-cache hashing — see
`compose/cache/hashing.rs`.

## Runtime-accessible descriptions

Every context descriptor implements the shared `Described` trait from
`darkmatter::catalog`. This means the catalog is queryable at runtime:

- `describe(CONTEXT_VARIABLE_DESCRIPTORS, "today")` returns the matching
  descriptor.
- `suggest(CONTEXT_VARIABLE_DESCRIPTORS, "toady", 1)` returns the nearest match
  (`today`) using fuzzy distance plus stable `order` tie-breaking.
- `describe_for_error(descriptor)` emits plain text suitable for error messages.

Claudine uses these helpers during composition preparation. A typo such as
`{{ ctx.toady }}` produces a non-fatal parser-aware diagnostic that suggests the
nearest real context variable, rather than silently rendering an empty string.
The diagnostic is suppressed by `--silent` and does not alter null-propagation
semantics.

## How to add a context variable

1. **Capture it.** Add a `values.insert("my_var", …)` in the appropriate
   `populate_*` function in `capture.rs`, choosing the right `Value` shape.
2. **Describe it.** Add a `ContextVariableDescriptor` to
   `CONTEXT_VARIABLE_DESCRIPTORS` in `catalog.rs` with its `name`, accurate
   `display_type`, `description`, `category`, `subsection`, `order`, and an
   optional verified `example`.
3. The CLI report needs **no change** — it reads the catalog directly.

The `name` in step 2 must exactly match the key in step 1. That contract is not
optional: the test `descriptor_name_set_equals_captured_runtime_key_set` asserts
the descriptor name set equals the captured runtime key set, in both directions.
Add a variable to only one side and the build fails.

## Drift control for context variables

- **CLI ↔ catalog**: structural — the CLI imports `context_variable_descriptors()`.
- **Catalog names ↔ runtime keys**: enforced by
  `descriptor_name_set_equals_captured_runtime_key_set`.
- **Catalog `display_type` ↔ actual captured JSON type**: enforced by
  `capture_value_shape_matches_display_type` and
  `context_example_results_are_type_consistent`, which capture a `ComposeContext`
  and assert every descriptor's value shape matches its declared type.
- **Examples are verified**: any `Example` on a context descriptor is asserted to
  match the `display_type` shape rules.
