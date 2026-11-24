# Ergonomic Sequences

We have improved sequences but today we use the `prompt` directive to perform a "compose" or "inline-compose" operation
on a Markdown or YAML file and that might be ok but being more explicit would be better. In addition, today, a sequence
can't call another sequence!

In this specification we will add the following operations:

- `compose` - composes the referenced document and uses as a prompt to an agent; produces an error if the referenced document has a "prompt" or "sections"
  frontmatter property (indicating that it is intended to be executed with `inline-compose`)
- `inline-compose` - same as compose but with the intention of executing an inline-compose operation; error if the neither the `prompt` or `sections`
  frontmatter properties are set
- `sequence`

All three operations will have the format of:

- short form: `{operation}: <file-reference>`
- long form:

    ```yaml
    $schema:
        compose: fn
        inline-compose: fn
        sequence:
    ```

All three operation will accept the `with` parameter in the long form which allows for explicit setting of properties
of Frontmatter state prior to implementing. Note, however, that _by default_ the sequence's state is passed into each
child operation.

## Defined and default values

A run of `compose` or `inline-compose` on a single document must always end up with an `agent`, and often with a
`model`. A sequence needs neither itself, but most of its tasks do. Today a sequence author has no good way to say "use
this unless the task or its prompt says otherwise". This section adds that.

Every child a task runs (`compose`, `inline-compose`, or a child `sequence`) receives two layers of values.

**Defined values** override what the child document says:

- the sequence's own frontmatter properties, other than its controls (see below), including values the caller passes
  as `key=value`
- the task's `with:` values

**Default values** fill only what nothing else sets, including the child document itself:

- the task's `default-values:` (a task option, beside the executable)
- the sequence root's `default-values:`
- `--<provider>` (`--claude`, `--codex`, …, `--provider <name>`) and `--model`, which set the root's
  `default-values.agent` and `default-values.model`

Precedence, highest first:

1. the task's `with:`
2. the sequence's defined values (root properties and caller `key=value`)
3. the child document's own frontmatter
4. the task's `default-values:`
5. the sequence root's `default-values:`, including `--<provider>` and `--model`

```yaml
kind: sequence
default-values:
    agent: claude          # --codex on the command line replaces this
    model: sonnet
sequence:
    - name: find
      compose: ./_find.md                  # _find.md names no agent: claude, sonnet
    - name: review
      compose: ./_review.md                # _review.md says `agent: codex`: codex (the prompt wins over a default)
    - name: summary
      compose: ./_summary.md
      default-values: { model: opus }      # claude, opus, unless _summary.md names a model
    - name: notes
      compose: ./_notes.md
      with: { agent: gemini }              # gemini, whatever _notes.md says
```

A root `agent:` (outside `default-values`) is a **defined** value, so it overrides every task's prompt:

```yaml
kind: sequence
agent: claude              # every child runs claude unless its task's `with:` says otherwise
sequence:
    - name: find
      compose: ./_find.md
```

### What counts as a sequence control

The sequence's own controls are never passed to a child as values: `kind`, `description`, `sequence`, `template`,
`$schema`, `$state`, `default-values`, `loop`, and the lifecycle events (`initialize`, `start`, `blocked`, `success`,
`failure`, `cancelled`, `finalize`). Every other root property is a defined value. `template:` is unrelated to
`default-values`: it supplies defaults for each step's own state (`seq.*`), not for the documents the steps run.

### Changes from today's behavior

- **A provider flag no longer forces every task.** Today `--claude` decides every task, even one whose `with:` or prompt
  names another agent. With this change it is a default value, so a task's `with:`, a root `agent:`, or a prompt's own
  `agent` wins over it.
- **A root `agent:` overrides prompts.** Today it is only a fallback for a task whose `with:` and prompt name no agent,
  and a prompt's own `agent` beats it. With this change it is a defined value. Authors who relied on the fallback move
  it under `default-values:`.
- `model` follows the same rules as `agent` in both cases.

### Decisions (2026-11-22)

1. **Inherited defaults in a child sequence.** A child sequence receives the parent's defaults and has its own
   `default-values:`. The child's own `default-values:` win over the inherited ones, because they are nearer to the
   documents they serve. The parent's defined values still override both.
2. **Group legs.** A group leg runs a child like a task does, so a leg's `with:` and `default-values:` behave as a
   task's. A group gains no `default-values:` of its own until someone needs one.

### Diagnostics

`2026-11-22-better-errors-and-dry-run` reports a task with no provider by listing the selection chain link by link.
When this spec changes the chain, that finding lists the defined and default links above, in this precedence, and its
`selection_key_as_data` hint also offers `default-values:` as a place for `agent` and `model`.
