# Side Effects

A **side effect** is an operation that changes something: a value held in memory, a Markdown file, a log, or a remote service. For example, a workflow might mark a document as `done` after an agent finishes its task.

Darkmatter, the library that processes Markdown for Claudine, provides these operations. Claudine decides when to call them as part of a workflow. This page explains the available operations, their limits, and how to inspect them.

## View the available operations

Run:

```sh
claudine context --side-effects
```

This command **only displays documentation**. It does not run the operations, change files, or send requests. The report groups operations by purpose and shows their names, descriptions, and safety classifications. It also shows examples when the terminal is at least 70 columns wide.

An operation appearing in the report does not mean a particular workflow is allowed to use it. The caller's configuration and permissions still apply.

## When side effects run

The [expression engine](expression-engine.md) reads values and calculates results. The side-effect engine changes state. Reading a document's `status` is an expression; saving a new `status` to the file is a side effect.

Claudine can invoke side effects at **lifecycle events**: points in a workflow such as its start or successful completion. For example, this configuration updates `state.md` when the workflow succeeds:

```yaml
success:
  stack:
    - action: { set_frontmatter: ["state.md", "status", "done"] }
```

Here, `stack` is a list of actions to run. `set_frontmatter` changes a property in the YAML metadata block at the top of the named Markdown file. If its frontmatter contains `status: in-progress`, the action replaces that value with `status: done`.

See [Lifecycle: Side-Effect Actions](../lifecycle.md#side-effect-actions) for how to configure these actions.

The responsibilities are separate:

- **Darkmatter** implements the operations and enforces the engine's limits.
- **Claudine**, or another application using Darkmatter, chooses when to call them and supplies their configuration.
- **The context report** describes them without executing them.

Darkmatter's document composition pipeline does not invoke this effects engine. That is a specific boundary, not a promise that all composition is free of side effects: composition also supports shell execution. See [Composition](../composition.md) for that broader workflow.

## What you can change

### Values held in memory

`set(key, value)` changes a top-level value in the caller's runtime state and returns the previous value, or `null` if the key was absent. It does not write the value to disk.

This differs from `set_frontmatter`, which saves a change to a Markdown file. The catalog uses function-style names; Claudine lifecycle YAML expresses an in-memory update as `set: {ready: true}`. See [Lifecycle](../lifecycle.md) for the action syntax.

### Markdown frontmatter

Frontmatter is the YAML metadata between `---` lines at the start of a Markdown file. These operations change that metadata:

| Operation               | What it does                                                                                        |
|-------------------------|-----------------------------------------------------------------------------------------------------|
| `set_frontmatter`       | Sets one property and returns its previous value, or `null`.                                        |
| `merge_frontmatter`     | Merges an object's properties into frontmatter and returns the merged object. The merge is shallow. |
| `delete_frontmatter`    | Removes a property and returns its previous value, or `null`.                                       |
| `increment_frontmatter` | Adds one to a number and returns the new number. A missing property becomes `1`.                    |
| `decrement_frontmatter` | Subtracts one from a number and returns the new number. A missing property becomes `-1`.            |
| `append_frontmatter`    | Adds a value to the end of an array and returns the updated array.                                  |
| `prepend_frontmatter`   | Adds a value to the beginning of an array and returns the updated array.                            |

### Files and directories

These operations return the resulting absolute path.

| Operation                    | What it does                                                                        |
|------------------------------|-------------------------------------------------------------------------------------|
| `ensure_file(file)`          | Creates an empty file if it is missing.                                             |
| `ensure_file(file, content)` | Creates a file with initial content if it is missing. Existing files are unchanged. |
| `ensure_dir(dir)`            | Creates a directory and any missing parent directories.                             |
| `append_line(file, text)`    | Adds text followed by a newline to a file.                                          |
| `append_jsonl(file, obj)`    | Adds one JSON record followed by a newline, useful for logs.                        |

### Remote services

`http_post(url, body)` sends an HTTP POST request, for example to notify a webhook that a task finished. It returns an object containing the response `status` and `body`. Network access must be explicitly allowed as described below.

## Limits on changes

The application that creates the engine configures its limits:

- **Filesystem location (`mutation_root`).** File and directory operations are restricted to a configured directory. The engine rejects paths that escape that root through its lexical path-containment check.
- **Network destinations (`allowed_hosts`).** No hosts are allowed by default. `http_post` rejects a host that is not allowed before making a network request.
- **Markdown hashes (`auto_rehash`).** Enabled by default. When a frontmatter operation changes a document with a `hash:` field, the engine recomputes that hash so it reflects the updated document.

The report's **Safety** column identifies which kind of change an operation makes:

| Classification     | Meaning                                                           |
|--------------------|-------------------------------------------------------------------|
| `InMemoryState`    | Changes runtime state without filesystem or network I/O.          |
| `MarkdownMutation` | Changes Markdown within the mutation root and honors auto-rehash. |
| `FilesystemWrite`  | Writes within the mutation root.                                  |
| `Network`          | Uses the configured host allowlist.                               |

## Implementation reference

The remaining sections are for contributors extending the engine or its report.

### Creating an engine in Rust

`EffectEngine` is defined in [`effects/mod.rs`](../../../../darkmatter/lib/src/effects/mod.rs). Its builder configures the limits described above:

```rust
use darkmatter::effects::EffectEngine;

let engine = EffectEngine::builder()
    .mutation_root("/path/to/repo")
    .allowed_hosts(["example.com"])
    .auto_rehash(true)
    .build();
```

Building an engine configures it; calling a mutating method performs the change. The methods live in [`verbs.rs`](../../../../darkmatter/lib/src/effects/verbs.rs). File-content writes use `atomic_write_guarded`, which writes a temporary file and renames it into place.

### How the report gets its descriptions

The catalog in [`effects/catalog.rs`](../../../../darkmatter/lib/src/effects/catalog.rs) contains an `EffectDescriptor` for each documented operation: its signature, description, category, safety classification, display order, and example. `effect_descriptors()` exposes the `EFFECT_DESCRIPTORS` collection.

Claudine's [`render_side_effects_report`](../../../cli/src/commands/context/effects.rs) reads that collection directly. Adding a catalog entry therefore updates the report without changing its renderer. Descriptors also implement the shared `Described` trait for exact lookup, suggestions, and error descriptions.

The engine exposes typed Rust methods, rather than a dispatcher that executes an arbitrary operation name. The catalog describes those capabilities; callers such as Claudine provide their own action syntax and dispatch.

### Documentation-only guarantee

The CLI test `metadata_reports_construct_no_engine_and_attempt_no_network` in [`context/tests.rs`](../../../cli/src/commands/context/tests.rs) exercises the three documentation reports and checks that none constructs an engine or calls its network operation.

It uses Darkmatter's optional `effects-instrumentation` feature, which counts engine construction and entry into `http_post` (including refused calls). Neither counter may increase while rendering the reports. Production builds do not enable this instrumentation.

### Adding an operation and keeping the catalog accurate

1. Add an `EffectEngine` method in `verbs.rs`, honoring the applicable filesystem, network, and hash rules.
2. Add an `EffectDescriptor` to `EFFECT_DESCRIPTORS`, including an example.
3. Add an entry to the test-only `EFFECT_VERBS` table that calls the real method.

Two tests in `effects/catalog.rs` check the result:

- `verb_signature_set_equals_descriptor_signature_set` checks that the test table and descriptor catalog contain exactly the same signatures, and that every descriptor has an example.
- `every_verb_maps_to_a_reachable_method` exercises each test-table entry in a sandbox. The HTTP entry checks the default host refusal without sending a request.

These tests catch disagreement between the two tables, including missing entries for alternate signatures such as `ensure_file(file, content)`. They **cannot detect a new public method omitted from both tables**. Reviewing new methods for catalog coverage remains necessary.

Public mutating methods deliberately excluded from the catalog belong in `INTENTIONALLY_UNCATALOGUED`, with a rationale in the code-review discussion. That list is currently empty. See [Drift Control](drift.md#next-steps) for the remaining coverage limitation.
