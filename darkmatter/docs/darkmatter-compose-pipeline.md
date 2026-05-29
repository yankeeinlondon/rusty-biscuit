# Darkmatter Compose Pipeline

## High Level Flow

```mermaid
block-beta
    columns 3


    block:pre
        columns 1
        preTitle["<b>1. Inline Pre</b> (<i>serial</i>)"]
        preFlightChecks("<a href='./inline/preflight-checks.md'>0. Pre-flight checks 🏁</a>")
        fmInterpolate("<a href='./inline/fm-interpolation.md'>1. Frontmatter Interpolation 🏁</a>")
        schemaValidation("<a href='./topics/schema-validation.md'>2. Frontmatter Shell Expansion 🏁</a>")
        shellExp("<a href='./inline/fm-shell-expansion.md'>2. Frontmatter Shell Expansion 🏁</a>")
        textReplacement("<a href='./inline/text-replacement.md'>3. Text Replacement 🏁</a>")
        pageBlocks("<a href='./inline/page-blocks.md'>4. Page Blocks 🏁</a>")
        interpolation("<a href='./inline/interpolation.md'>5. Interpolation 🏁</a>")
        shellExpansion("<a href='./inline/shell-expansion.md'>6. Shell Expansion 🏁</a>")
        shellBlocks("<a href='./inline/shell-blocks.md'>7. Shell Blocks 🏁</a>")
        linkResolve("<a href='./operations/link-resolve.md'>8. Link Resolve (abs) 🏁</a>")
    end

    block:transclusion
        columns 1
        transTitle["<b>2. Transclusion</b> (<i>parallel</i>)"]
        blockTransclusion("<a href='./transclusion/block-transclusion.md'>Block Transclusion 🏁</a>")
        fmTransclusion("<a href='./transclusion/fm-transclusion.md'>Frontmatter Transclusion 🏁</a>")
        codeBlockTransclusion("<a href='./transclusion/code-transclusion.md'>Code Block Transclusion 🏁</a>")
        tocLinking("<a href='./inline/toc-linking.md'>TOC Linking 🏁</a>")
        promptExpansion("<a href='./transclusion/prompt-expansion.md'>🧠 Prompt Expansion</a>")
        summarization("<a href='./transclusion/summarization.md'>🧠 Summarization</a>")
        consolidation("<a href='./transclusion/consolidation.md'>🧠 Consolidation</a>")

        ts1[" "]
        ts2[" "]
    end

    block:post
        columns 1
        postTitle["<b>3. Inline Post</b> (<dim><i>serial</i></dim>)"]
        cleaning("<a href='./inline/cleaning.md'>1. Cleaning 🏁</a>")
        structural("<a href='./inline/structural-normalization.md'>2. Structural Normalization 🏁</a>")

        s1[" "]
        s2[" "]
        s3[" "]
        s4[" "]
        s5[" "]
        s6[" "]
        s7[" "]
    end

    block:final
        columns 1
        finalTitle["<b>4. Finalization</b> (<dim><i>serial</i></dim>)"]
        links("<a href='./operations/link-normalization.md'>Link Normalization 🏁</a>")

        fs1[" "]
        fs2[" "]
        fs3[" "]
        fs4[" "]
        fs5[" "]
        fs6[" "]
        fs7[" "]
    end



    pre --> transclusion
    transclusion --> post
    post --> final

    style preTitle fill:transparent,stroke:transparent
    style transTitle fill:transparent,stroke:transparent
    style postTitle fill:transparent,stroke:transparent
    style finalTitle fill:transparent,stroke:transparent

    classDef spacer fill:transparent,stroke:transparent,color:transparent
    class s1,s2,s3,s4,s5,s6,s7,ts1,ts2,fs1,fs2,fs3,fs4,fs5,fs6,fs7 spacer

    classDef notReady font-weight:100,font-family:sans-serif,stroke:gray
    class promptExpansion,summarization,consolidation notReady
```

> **Note:** items marked with `🏁` are implemented

## Pipeline Stages

### 1. Inline Pre

The **Pre Transclusion** group are a set operations run in a serial process, one operation after another. These operations take place _before_ any transclusions take place and allow the document to become stable before the transclusion process is executed.

Being run serial is important so that one operation can _setup_ or _effect_ the next operation. Rather then be a side effect, this is intentional and often adds useful power to the pipeline. It does mean, however, that the ordering of these operations must be considered and organized in a way to provide maximum value.

> **Example:** if a conditional page block is evaluated to _false_ (aka, do not render this page), then identifying this first means any
shell expansion commands (or any other inline mutation) contained within the block will now be ignored because this part of the page has
been removed.
>
> **Note:** in this example, even though we will not execute the shell expansion, when we do the pre-flight checks we will require that ALL shell commands are whitelisted before starting the 

### 2. Transclusion

The **transclusion** stage is typified by recursive operations which have the potential to be time consuming (and more dependent on 
[caching](./topics/caching.md)) then those found in the inline steps. 

> **Note:** Not all operations are expensive -- for instance the most common transclusion directive is the `::file <ref>` directive which points to another local Markdown document. Assuming the document it references
doesn't have it's own transclusions this operation will be lightning fast and no slower than any of the the inline mutation operations.
However, even in this example, we don't know how expensive the operation is until the graph dependency has been traversed

### 3. Inline Post

- [Cleaning](./inline/cleaning.md) - makes the markdown as standard bearing and consistent as possible                    
- [Normalization](./inline/structural-normalization.md) - ensures that the heading structure is valid and fixes where it is not

### 4. Finalization

The **finalization** stage is _only_ performed in the root document of the compose operation and does any adjustments on the fully transposed document before passing it back to the caller.

- [Link Normalization](./operations/link-normalization.md) - converts absolute paths back to portable forms (relative, `~/`, or `${ENV}`)


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

> For more details on the **rendering pipeline** always refer to: [Rendering Pipeline](./darkmatter-rendering-pipeline.md)
