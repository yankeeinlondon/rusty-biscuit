# Darkmatter Compose Pipeline

## High Level Flow

| Inline Pre (serial)                                       | Transclusion (concurrent)                                        | Inline Post (serial)                                            | 
| -------------                                             | -------------                                                    | ---------------                                                 | 
| [1. Frontmatter Interpolation 🏁](./inline/fm-interpolation.md)  | [Block Transclusion 🏁](./transclusion/block-transclusion.md) | [1. Cleaning 🏁](./inline/cleaning.md)                             |
| [2. Frontmatter Shell Expansion 🏁](./inline/fm-shell-expansion.md)| [Frontmatter Transclusion 🏁](./transclusion/fm-transclusion.md)  | [2. Normalization 🏁](./inline/normalization-and-releveling.md)    |
| [3. Text Replacement 🏁](./inline/text-replacement.md)          | [Code Block Transclusion 🏁](./transclusion/code-transclusion.md) |                                                                 |
| [4. Page Blocks 🏁](./inline/page-blocks.md)                    | [TOC Linking 🏁](./inline/toc-linking.md)                         |                                                                 |
| [5. Interpolation 🏁](./inline/interpolation.md)                | [AI Prompt Expansion](./transclusion/prompt-expansion.md)         |                                                                 |
| [6. Shell Expansion 🏁](./inline/shell-expansion.md)       | [AI Summarization](./transclusion/summarization.md)               |                                                                 |
|                                                                  | [AI Consolidation](./transclusion/consolidation.md)               |                                                                 |

> **Note:** items marked with `🏁` are implemented


## Pipeline Stages

### Inline Mutation

What defines the _inline mutation_ group is that the updates on a document are isolated to the document at hand. In this group, however,
we have two subset groups:

1. Pre Transclusion
2. Post Transclusion

For the **Pre Transclusion** group, these operations are run in a serial process, one operation after another. During these early steps we
have the potential for one operation to _setup_ or _effect_ the next operation. Rather then be a side effect, this is intentional and often adds useful power to the
pipeline.

> As a example, if a conditional page block is evaluated to _false_ (aka, do not render this page), the identifying this first means any
shell expansion commands (or any other inline mutation) contained within the block will now be ignored because this part of the page has
been removed.

In contrast the **Post Transclusion** group of operations -- which are also run serially -- are focused on finalizing output and structure
into the most valid form we can deterministically reach.

#### Pre Ops

- [Frontmatter Interpolation](./inline/fm-interpolation.md) - resolves `{{ variable }}` expressions inside frontmatter values using non-templated (seed) values, `ctx.*`, and `env.*` as inputs. Runs before the effective state is built so downstream stages see resolved values.
- [Frontmatter Shell Expansion](./inline/fm-shell-expansion.md) - executes `$(cmd)` expressions in top-level frontmatter string values, replacing them with trimmed stdout. Runs after interpolation and before the effective state is built, so later stages see the resolved values. Shares approval flow with body `::shell` directives.
- [Text Replacement](./inline/text-replacement.md) - when `replace` property in frontmatter is a key/value dictionary we will replace all instances of the _keys_ with the _values_ in the body of the document
- [Page Blocks](./inline/page-blocks.md) - allow for blocks in the page to be defined, often with _conditional_ logic to determine whether the block should be rendered or removed
- [Interpolation](./inline/interpolation.md) - looks for handlebars template markers in the page's body and replaces the template markers with data from frontmatter, ENV variables, or [context variables](./topics/context-variables.md).
- [Shell Expansion](./inline/shell-expansion.md) - allows _approved_ commands to be run and have the STDOUT replace the directive
- Link Validation is deferred and not part of the shipped compose pipeline yet.

#### Post Ops

- [Cleaning](./inline/cleaning.md) - makes the markdown as standard bearing and consistent as possible                    
- [Normalization](./inline/normalization-and-releveling.md) - ensures that the heading structure is valid and fixes where it is not

### Transclusion

The **transclusion** stage is typified by recursive operations which have the potential to be time consuming (and more dependent on 
[caching](./topics/caching.md)) then those found in the inline steps. 

> **Note:** Not all operations are expensive -- for instance the most common transclusion directive is the `::file <ref>` directive which points to another local Markdown document. Assuming the document it references
doesn't have it's own transclusions this operation will be lightning fast and no slower than any of the the inline mutation operations.
However, even in this example, we don't know how expensive the operation is until the graph dependency has been traversed



### Rendering

- mutates the document for one of the [supported output formats](./topics/output-formats.md)


## Ordering and Concurrency

The macro flow for execution is as follows:

```mermaid title="Pipeline Flow"
flowchart LR

  InlinePre["Inline (pre)"]
  Transclusion[Transclusion]
  InlinePost["Inline (post)"]
  InlinePre --> Transclusion --> InlinePost
```

- Both the `Inline` stages of the workflow process content serially but that is **NOT** true for transclusion:

    - transclusion starts by serially preparing the work items in `ComposeOperation::default_order()`
    - block/file transclusion, frontmatter transclusion, code transclusion, and TOC-linking are then resolved concurrently
    - `::toc-linking` reads headings from the referenced file's raw markdown source rather than recursively composed output
    - ancestry repetition is treated as a cycle; shared DAG dependencies across sibling branches are allowed
    - the base document's transclusion is done when all prepared work items are complete

- Only after these concurrent transclusions complete do we move to the final `Inline (post)` process.
- After we conclude the Inline post processing we optionally will move into rendering
    - the default output for the compose pipeline is plain text output so no rendering support is needed

## Rendering

The Compose pipeline does not do any "rendering" per se but it's not uncommon programmatically to chain 
[rendering](./darkmatter-rendering-pipeline.md) immediately _after_ a page being _composed_. In addition the 
Darkmatter CLI reinforces this pattern by providing CLI switches to provide this same chaining. 
This back-to-back pipelining transform is visualized below:

```mermaid title="Chaining Compose and Render Pipelines"
flowchart LR

DM@{label: "Darkmatter", shape: "doc"}
MdInput@{label: "Markdown", shape: "doc"}
Md@{label: "Markdown", shape: "doc"}
Compose("Composition
Pipeline")
Render("Render
Pipeline
")
Output(Output)

DM -->|provide to| Compose
MdInput -->|provide to| Compose
Compose -->|transform| Md
Md --> Render
Render -->|transform| Output
```

The key things to remember are:

- the **Compose Pipeline** expects either Markdown or Darkmatter content as input
    - in the case of receiving a Markdown document _without_ any Darkmatter directives, only very mild formatting changes from operations like 
- the [Rendering Pipeline](./darkmatter-render-pipeline.md) expects to receive Markdown content not Darkmatter and it returns one of the supported [output formats](./topics/output-formats.md).

##### For more details on the **rendering pipeline** always refer to: [Rendering Pipeline](./darkmatter-rendering-pipeline.md)
