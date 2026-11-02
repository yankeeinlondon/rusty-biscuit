# Change Detection and Blast Radius

It is very powerful to be able to structurally and semantically be able to express WHAT has changed. Once we have a reliable way of detecting and expressing CHANGE we can then look the "blast radius" which moves out from the first order change and identifies what areas are impacted as a second (or further downstream) effects. Blast radius could express the documentation that needs to change when a symbol is changed.

## Change Detection

Detecting change "in general" might be able to be done in a generalized way but in this document we'll be exploring specific _types_ of change so that we can get a more detailed and meaningful way of articulating change. The types we will focus on to start out with will be focused on helping with devops for this monorepo.

## Blast Radius

When you have a rich set of metadata to describe a file or symbols changes you can do a much better job of determining what external files or symbols might be impacted by this change. Determining the "blast radius" starts by leveraging some generalized and deterministic techniques:

- we can detect source code change down to the Symbol level quite easily (static analysis)
- we can also map Symbol relationships to understand how one Symbol's change can spread out to other Symbols downstream (static analysis)
- we can tag files into packages and package areas reliably across different monorepo standards
- etc.

But we can also benefit from a repo's standardization if we can define a grammar for this standardization to be expressed.

- for instance, in this repo we've started to use the `kind` Frontmatter tag in Markdown docs to categorize documents. 
    - One kind category is `canonical-documentation` which means that the document is the canonical documentation resource for certain concepts or symbols. 
    - We haven't done this yet but we could go further by having another Frontmatter property articulate which symbols or concept's it talks to. At that point we have a clear, fast, and deterministic way to better understand how a change to a symbol or concept has on the documentation.
    - we do in fact have a quite poorly followed practice of expressing a document's dependencies via the `blast_radius` frontmatter property
- today this is very haphazard and not systematically enforced but were we to have a descriptive grammar we could do a lot more

## Change Types

1. Configuration File Changes
    - when a `.toml`, `.json`, `.yaml` file changes we need to be able to assess:
        - what changed?
        - what _kind_ of configuration is this?
            - known programming language config file
            - repo specific config files
        - how can we understand how to describe the change and blast radius in both a generalized way but also allowing repo's to provide metadata on their own configuration files so we can do a better job on unknown config files
            - known config:
                - there are files like `package.json` in a JS/TS project or `cargo.toml` in a Rust project which we can and should "know about" and be able to leverage this rules to produce high quality outputs
            - local metadata:
            - frontmatter standards: 
2. Source Code Changes
    - where does this file reside: package area? package? module?
    - we need to be able to distinguish a file as being:
        - source code
        - test file
        - hybrid (test and source)
    - does this 
    - what symbols does this file define, and for each symbol:
        - what state is the symbol in:
            - unchanged
            - modified
            - new
            - removed
        - has a symbol had it's public API changed?
        - what tests test with this Symbol? is this symbol the thing being tested or being used to setup a test of something else?
        - 

3. Dependencies

    - we need to be able to clearly identify when a package's dependencies change and be able to express the impact of that change intelligently 
    - dependencies usage strategy needs to be categorized as:
        - dep
        - dev-dep
        - peer-dep
    - but also we need to distinguish a cross package monorepo dependency from an external dependency
    - in a monorepo we also need to know:
        - 
        - what other packages _share_ the same 

4. Test Coverage Changes





## Static Analysis

This monorepo's `tree-hugger` library has been improved recently and should represent a good foundation for our static analysis needs. In it's current state it won't meet our needs but below you'll find functionality that we would want to have available to us to address the "change detection" and "blast radius" goals this document discusses.

> Note: this document is not intended to discuss technical details nor is it to assess how much change will be required in `tree-hugger` but instead it is to


## Sniff Improvements

<!--TODO-->


## High Level Implementation Approach

We create a `change-detection` library and CLI:

- the library's primary job is to wrap tree-hugger and sniff based functionality and expose it in a natural way for callers who want to evaluate change and downstream impacts (aka, blast radius)
- the library may end up leveraging a few additional third party crates to round out its functionality and it's also allowed to do more direct work outright if that's appropriate
- the CLI like all CLI's in this monorepo is primarily a way of exposing the library's API surface to a terminal user

### Draft API Surface 

> Draft content; only being used to trigger debate, discussion, etc.

```sh
# if inside a git repo it provides a summary of changes 
# found in the repo; if NOT in a git repo then it shows the help system
change
```

> Note: `change` with no sub-command is an alias for `change repo` when in a repo

The example output in Prose might look something like:

```prose
The <b><blue>{repo}</blue></b> repo has <yellow>{#}</yellow> files in the <i>{branch}</i> branch that have been changed since the {last-commit-hash} _commited_ {timeframe} (<i><dim>last commit on this branch</dim></i>).

::block when="merge-conflicts"

::end-block

::block when="config-changes"
## Configuration Changes

There are <yellow>{#}</yellow> configuration files which changed:

::loop config-changes

::end-loop

::end-block


```
