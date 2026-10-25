# Claudine schemas

The SimplifiedSchema files that type Claudine's Markdown frontmatter. The files
are hand-maintained. The shape of every lifecycle verb is Darkmatter's
(`darkmatter/schemas/partials/lifecycle.yaml`): these files name those shapes
with the reserved `darkmatter:` namespace (`say-short@darkmatter:lifecycle`)
instead of copying them. A `darkmatter:` reference resolves from the schemas
compiled into Darkmatter, wherever the referring file is.
Darkmatter discovers the three trigger files through its schema locations, so
a document under `claudine/` is validated against them in the editor (DMLS) and
by `md schema validate`. The `claudine` binary embeds the same seven files and
judges them in its launch verdict, so an invalid Claudine control fails
`claudine compose`, `claudine inline-compose`, and `claudine sequence` before
any provider starts.

[Schemas](../docs/topics/schemas.md) explains what the schemas check, which
events allow which actions, and how a document outside `claudine/` reaches
them.

| File | Kind | Role |
| --- | --- | --- |
| `action.yaml` | library (`kind: schema`) | the four action types (`communicate`, `flow`, `shell`, `expression`), the two dictionary-bundle types (`bundle`, `initialize-bundle`), and the `cancelled` selections, each verb's shape referenced from Darkmatter |
| `lifecycle.yaml` | library (`kind: schema`) | the three recursive lifecycle stack families (`stack`, `initialize-stack`, `cancelled-stack`), typed at every conditional depth, and the `loop:` block type |
| `sequence-types.yaml` | library (`kind: schema`) | the sequence step and container types (`steps`, `step`, `task`, `group`); `task` and `group` recurse, so containers are typed at every depth |
| `linking.yaml` | library (`kind: schema`) | the eleven optional linking keys of a linked skill, command, or agent |
| `claudine.yaml` | trigger (`kind: schema-trigger`) | applies Claudine's frontmatter catalog to a document with a non-null lifecycle event, `loop`, `prompt`, `agent`, or `model`; types Darkmatter's `composed` and `failed` as errors that name `start` and `blocked` |
| `inline-compose.yaml` | trigger (`kind: schema-trigger`) | applies inline-compose's document metadata to a document with a non-null `prompt` and no non-null `sequence` |
| `sequence.yaml` | trigger (`kind: schema-trigger`) | applies the sequence document's catalog (`sequence`, `template`, and the removed root `fail_fast`) to a document with a non-null `sequence` |
| `review.yaml` | standalone schema (`$schema`) | the schema of a specification review document |
| `globals.yaml` | host catalog (`kind: schema` + `$schema`) | the shapes of Claudine's host globals `seq`, `loop`, and `group` |
| `globals.lifecycle.json` | generated host lifecycle artifact | Claudine's event registry and lifecycle parse options, for DMLS; regenerate with `just regen-lifecycle-artifact` |

The **host catalog** (`globals.yaml`) declares the shape of each global
Claudine supplies to lifecycle and sequence expressions, in Darkmatter's
global-catalog format. The `claudine` binary embeds it separately from the
trigger bundle and registers it with its events; it is never a document
schema or a trigger, and which events may read each global is decided in
code. DMLS reads it from a schema location for hover and completion. Beside
it, the generated `globals.lifecycle.json` records Claudine's events, renames,
per-event globals, and accepted verbs, so DMLS types lifecycle stacks as a
launch does; copy it with `globals.yaml`
([Host Lifecycle Artifacts](../../darkmatter/docs/topics/lifecycle/host-artifact.md)).

A **library** (`kind: schema`) holds named types only and reaches a document
only when a trigger or a document's `$schema` imports it
(`stack@./lifecycle.yaml`). A **trigger** (`kind: schema-trigger`) applies its
payload to every document its `match:` selects; it is never named by a
`$schema`. The trigger and library files above are used together: copy all of
them (`*.yaml`), as regular files, when placing the bundle elsewhere.

The lifecycle parser in `claudine/lib/src/composition/lifecycle/` remains the
authority on what can run. Tests in `claudine/lib/tests/l1/claudine_schemas.rs`
keep the action types aligned with its verb inventory and with Darkmatter's
side-effect and expression-function catalogs; `claudine_schema_parity.rs`
checks the event names and per-event action types against Claudine's event
registry, and that the action types equal Darkmatter's neutral ones.

Darkmatter's own baseline types its event names (`initialize`, `composed`,
`failed`, `cancelled`, `finalize`) with the `md` host's verbs. On a document
this catalog applies to, its declarations of `initialize`, `cancelled`, and
`finalize` replace the baseline's, and `composed` or `failed` is an error
whose description names the Claudine event to write instead.

The frontmatter catalog includes `cancelled` on documents and containers. Its
stack accepts communication, expressions, shells, and `stop`, and rejects
every other flow directive at every conditional depth. An
empty `cancelled: []` opts a primitive into cancellation.

`sequence-types.yaml` exports `steps`, `step`, `task`, and `group` for
editor completion and authored parameter schemas, for example
`$schema: { nested: task@./sequence-types.yaml }`. A task can contain an
inline `sequence:` list or a referenced sequence; a group owns `tasks:`,
`variables:`, and `fail_fast:`. A task's inline `sequence:` items and a
group's `tasks:` legs reference `task` again, so every container, at every
depth, has its events typed like a document's (`initialize` shell-free,
`cancelled` restricted to `stop` among flow directives) and its `loop` typed
by the shared `loop` type. A group object is closed, so a removed key such as
`execution` is a schema error; a task object is open, so an undeclared key is
step data. Executable exclusivity, unique leg names, container-event
placement, and removed task keys remain parser-validated. The document
catalog (`claudine.yaml`) does not declare `sequence`; `sequence.yaml` is its
only declaration.
