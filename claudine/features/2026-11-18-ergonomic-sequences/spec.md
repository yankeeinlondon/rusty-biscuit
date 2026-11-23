# Ergonomic Sequences

We have improved sequences but today we use the `prompt` directive to perform a "compose" or "inline-compose" (and it was supposed to also allow child sequences) operation on a Markdown or YAML file and that might be ok but being more explicit would be better. In addition, today, a sequence can't call another child sequence!

In this specification we will add the following explicit operations:

- `compose` - composes the referenced document and uses as a prompt to an agent; produces an error if the referenced document has a "prompt" or "sections"
  frontmatter property (indicating that it is intended to be executed with `inline-compose`)
- `inline-compose` - same as compose but with the intention of executing an inline-compose operation; error if the neither the `prompt` or `sections`
  frontmatter properties are set
- `sequence` - an explicit execution of a child sequence

It will **keep** the `prompt` operation but this operation will be a generic call for executing a compose, inline-compose, or a sequence operation.

The simplified schema for tasks will be changed to a union type that allows the following properties (as a union):

- compose (explicit composition)
- inline-compose (explicit composition)
- sequence (explicit child composition)
- prompt (generic composition; pre-existing)
- shell-command: (execute shell command; pre-existing)

In **Simplified Schema** it would look something like:

```yaml
$schema:
    # use `compose`, `inline-compose`, `sequence`, `prompt`, or `shell-command` but
    # only ONE of them
    - compose: compose
      $extends: common-task-props
    - inline-compose: inline-compose
      $extends: common-task-props
    - sequence: sequence
      $extends: common-task-props
    - prompt: prompt
      $extends: common-task-props
    - shell-command: shell-command
      $extends: common-task-props
types:
    common-task-props: 
        name: "string(required) -> the name of the task and made available as `seq.name` during execution"
        timeout: "number -> the wall clock time that this operation/task is allowed to take before it will be _timed out_."
        # ...
    compose:
        - compose-short-form
        - compose-long-form
    compose-short-form: 
        - file(required;match(**/*.md)) -> a file reference to a Markdown document that will be composed before being passed to the agent
        - string(required) -> an inline prompt which will be _composed_ before being passed to the agent.
    compose-long-form:
        file: file(required;match(**/*.md)) -> a file reference to a Markdown document that will be composed before being passed to the agent
        with: object -> an _optional_ key/value object of properties which will be set as the initial state of the composed document
        default-values: object -> an _optional_ key/value object of properties which will have a specified _default value_ for the state of the composed document
        # ...
```

## An Unspoken Linguistic Complication

We need to clear up one complication that may have gotten lost in the shuffle:

- Markdown files with no `sequence` property are a single document that will be executed _either_ via a "compose" or "inline-compose" operation. This is determined by whether the document's frontmatter has either a `prompt` or `sections` property. 
    - if it _does_ then it will be executed as an inline-compose operation
    - if it _does not_ then it will be executed with the document's body/prose as the prompt during a compose operation.
- So what about a Markdown file WITH a `sequence` property? 
    - this is the area of confusion which at least currently we don't have a great linguistic solution for
    - the answer is .. it is a **sequence** operation but there are two variants of a sequence:
        - inline sequences
            - if the document has a `prompt` or `sections` property then it is a _inline sequence_
            - this means that each step in the sequence is intended to operate on the prose/body of this document as it's primary mutation
        - compose sequences
            - if the document _does not_ have a `prompt` or `sections` property then it is considered a _compose sequence_
            - which means that the prompt for each step in the sequence will use the document's prose/body as the prompt 

In most ways a sequence defined by a YAML document is the clearest variant of a sequence:

- a YAML defined sequence is neither a _inline sequence_ or a _compose sequence_ it is just a pure sequence that makes no assumptions about where the "prompt" that will be composed is coming from. It is instead just an ordered set of steps/tasks.
- Each step in the YAML sequence chooses an operation specifically (`compose`, `inline-compose`, `sequence`, `shell-command`) or more generally (`prompt`).
    - When using the `compose` or `inline-compose` operation, if you point to Markdown file with the `sequence` property then that is an ERROR
    - When using the `sequence` operation, you can point to either a YAML or Markdown file:
        - if it's a Markdown file then it must have a `sequence` property or it will result in an ERROR
        - if it's a YAML file then it must be one of the following:
            - an **explicit** sequence (e.g., `kind: sequence`)
            - an **informal** sequence (e.g., an array of string or an array of objects which must have the `name` property defined)
        - Note: the informal syntax does allow _offsetting_ so if the base property of the YAML is not what you want to use as the sequence you can offset it with a `{filename}::{offset}` syntax that might look like `./foobar.yaml::data`

This linguistic complication is useful context for this spec but we must also make sure that it is fully documentation (e.g., documentation under Claudine's "docs" folder). 


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
