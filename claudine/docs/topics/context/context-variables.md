# Context Variables

Context variables are the runtime facts a composed Markdown document can read
through the `ctx.*` namespace: the date, the repository, the current package,
the operating system, the hardware, and more. They are the *inputs* to the
[expression engine](expression-engine.md) and to `{{ … }}` interpolation.

```markdown
Today is {{ ctx.today }} and you are working in {{ ctx.current_package }}.
```

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

## How to add a context variable

1. **Capture it.** Add a `values.insert("my_var", …)` in the appropriate
   `populate_*` function in `capture.rs`, choosing the right `Value` shape.
2. **Describe it.** Add a `ContextVariableDescriptor` to
   `CONTEXT_VARIABLE_DESCRIPTORS` in `catalog.rs` with its `name`, accurate
   `display_type`, `description`, `category`, `subsection`, and `order`.
3. The CLI report needs **no change** — it reads the catalog directly.

The `name` in step 2 must exactly match the key in step 1. That contract is not
optional: the test `descriptor_name_set_equals_captured_runtime_key_set` asserts
the descriptor name set equals the captured runtime key set, in both directions.
Add a variable to only one side and the build fails.

## Drift control for context variables

- **CLI ↔ catalog**: structural — the CLI imports `context_variable_descriptors()`.
- **Catalog names ↔ runtime keys**: enforced by
  `descriptor_name_set_equals_captured_runtime_key_set`.
- **Catalog `display_type` ↔ actual captured JSON type**: **not** yet enforced.
  The name-set test proves a variable *exists* but not that its declared type
  matches the `Value` shape `capture.rs` produces. (The `Nullable(String)` /
  `Nullable(Integer)` accuracy described above was corrected by hand, not by a
  failing test.) This is the one open gap for context variables — see
  [Drift Control](drift.md#next-steps) for a concrete way to close it.
