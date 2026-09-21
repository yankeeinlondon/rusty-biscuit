# Context, Expressions, and Side Effects

A Markdown prompt can adapt to the project it is running in. It can include the
current date, calculate a value, or show instructions only when a condition is
met. A workflow can also save changes, such as marking a task complete.

Claudine uses **Darkmatter**, its Markdown-processing library, to provide three
parts of this behavior:

| Part | Purpose | Example |
|---|---|---|
| **Context variables** | Supply facts about the current machine and project. | `ctx.today` provides today's date. |
| **Expressions** | Read values and calculate results. | `upper("hello")` produces `HELLO`. |
| **Side effects** | Change runtime state, files, or a remote service. | `set_frontmatter` saves a property in a Markdown file. |

Darkmatter implements these capabilities. Claudine connects them to commands
and workflows, including deciding when to run side effects. A side-effect
operation does not run simply because it appears in a document's text.

## Start with the reports

The `claudine context` command lets you explore what is available without
writing a prompt first:

| Command | What it shows |
|---|---|
| `claudine context` | Available context variables, with their types and descriptions. |
| `claudine context --values` | The same variables and types, with current values instead of descriptions. |
| `claudine context --expressions` | Expression syntax, operators, functions, and examples. |
| `claudine context --side-effects` | Operations that change state and the restrictions that apply to them. |

Use one report flag at a time; the flags cannot be combined.

**Only `--values` gathers live context**, such as repository, operating-system,
and hardware information. It takes one snapshot for the report and shows `null`
where a value is unavailable. The other reports display descriptions without
capturing that context or executing the capabilities they describe.

The reports adapt to terminal width, up to 140 columns. Their minimum supported
width is 53 columns; expression and side-effect examples are hidden below 70
columns.

## Use values in a document

During **composition**, Darkmatter processes the instructions embedded in a
Markdown document to produce its output. For example:

```markdown
Today is {{ ctx.today }}.
```

The `{{ … }}` syntax inserts the result of an expression. Here the expression
reads a context variable, so the output contains the date instead of the
braces. Expressions can also transform values or decide whether a section
should be included.

Saving a change is a separate operation. For example, Claudine can call a
side-effect function after a workflow succeeds to update a file's `status`
property. These calls belong to the workflow's lifecycle actions; the
`claudine context --side-effects` command only documents them.

## Choose the guide you need

- [Context Variables](context-variables.md): find the machine and project facts
  available through `ctx.*`, and understand how they are captured.
- [Expression Engine](expression-engine.md): insert calculated values, write
  conditions, and handle missing values.
- [Side Effects](side-effects.md): update runtime state, edit frontmatter,
  create files, or send a request within the configured limits.
- [Composition](../composition.md): understand how Claudine's `compose`,
  `inline-compose`, and `sequence` commands use these capabilities.

## How the reports stay connected to the implementation

This section is for contributors maintaining the reports.

Darkmatter publishes a **descriptor catalog** for each subsystem: structured
records describing the available variables, functions, or operations. Claudine
builds its report tables from those records instead of maintaining a separate
list. Adding a descriptor therefore makes it available to the report on the
next build.

The records also support programmatic lookup through the `Described` trait:
`describe` finds an exact match, `suggest` finds similar names, and
`describe_for_error` formats a description for an error message.

Tests compare catalogs with runtime behavior and evaluate documented examples.
These checks reduce drift, but they do not prove every sentence correct or
find every possible omission. In particular, the side-effect tests cannot find
a public method omitted from both the catalog and its test table. See
[Drift Control](drift.md) for the checks and their limits, and
[the documentation-only guarantee](side-effects.md#documentation-only-guarantee)
for how tests ensure the reports do not construct an effects engine or invoke
its network operation.

### Source map

| Concern | Location |
|---|---|
| Context descriptions and capture | [Darkmatter context module](../../../../darkmatter/lib/src/markdown/compose/context/) |
| Expression language and functions | [Darkmatter expression module](../../../../darkmatter/lib/src/markdown/compose/expression/) |
| Side-effect operations and descriptions | [Darkmatter effects module](../../../../darkmatter/lib/src/effects/) |
| CLI reports | [Claudine context command](../../../cli/src/commands/context/) |
| Shared table layout and width limits | [Claudine context renderer](../../../cli/src/commands/context_render.rs) |
