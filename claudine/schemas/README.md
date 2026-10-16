# Claudine schemas

The SimplifiedSchema files that type Claudine's Markdown frontmatter. This YAML
is the only definition of Claudine's schemas; the files are hand-maintained.
Darkmatter discovers the two trigger files through its schema locations, so a
document under `claudine/` is validated against them in the editor (DMLS) and
by `md schema validate`. The `claudine` binary embeds the same five files and
judges them in its launch verdict, so an invalid Claudine control fails
`claudine compose` and `claudine inline-compose` before any provider starts.

[Schemas](../docs/topics/schemas.md) explains what the schemas check, which
events allow which actions, and how a document outside `claudine/` reaches
them.

| File | Kind | Role |
| --- | --- | --- |
| `action.yaml` | library (`kind: schema`) | the four action types (`communicate`, `flow`, `shell`, `expression`) and the two dictionary-bundle types (`bundle`, `initialize-bundle`) |
| `lifecycle.yaml` | library (`kind: schema`) | the lifecycle stack types (`stack` and `initialize-stack`, typed three conditional levels deep; `cancelled-stack`, typed at all five levels the parser accepts), the `loop:` block type, and the sequence container types (`step`, `task`, `group`) |
| `linking.yaml` | library (`kind: schema`) | the eleven optional linking keys of a linked skill, command, or agent |
| `claudine.yaml` | trigger (`kind: trigger-schema`) | applies Claudine's frontmatter catalog to a document with a non-null lifecycle event, `loop`, `prompt`, `agent`, or `model` |
| `inline-compose.yaml` | trigger (`kind: trigger-schema`) | applies inline-compose's document metadata to a document with a non-null `prompt` and no non-null `sequence` |
| `review.yaml` | standalone schema (`$schema`) | the schema of a specification review document |

A **library** (`kind: schema`) holds named types only and reaches a document
only when a trigger or a document's `$schema` imports it
(`stack@./lifecycle.yaml`). A **trigger** (`kind: trigger-schema`) applies its
payload to every document its `match:` selects; it is never named by a
`$schema`. The five files above are used together: copy all of them, as
regular files, when placing the bundle elsewhere.

The lifecycle parser in `claudine/lib/src/composition/lifecycle/` remains the
authority on what can run. Tests in `claudine/lib/tests/l1/claudine_schemas.rs`
keep the action types aligned with its verb inventory and with Darkmatter's
side-effect and expression-function catalogs.

The frontmatter catalog includes `cancelled` on documents and containers. Its
stack accepts communication, expressions, shells, and `stop`, and rejects
every other flow directive at every conditional depth the parser accepts. An
empty `cancelled: []` opts a primitive into cancellation.

`lifecycle.yaml` also exports `step`, `task`, and `group` for editor completion
and authored parameter schemas, for example
`$schema: { nested: task@./lifecycle.yaml }`.
A task can contain an inline `sequence:` list or a referenced sequence;
a group owns `tasks:`, `variables:`, and `fail_fast:`. Named types are
inlined, so containers are typed two levels deep (`task`/`group` and their
direct children, `step-2`/`task-2`/`group-2`), each with a restricted
`cancelled`; their other events are checked for shape only. A container
nested deeper than that is accepted as any list, so its `cancelled` is
checked by the task parser, not the schema. Deeper lists,
those events' actions, executable exclusivity, unique leg names,
container-event placement, and removed-key diagnostics remain
parser-validated. The document catalog
does not declare `fail_fast` or the former injected sequence overlays.
