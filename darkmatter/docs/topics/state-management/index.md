---
kind: index
---

# State Management in Darkmatter

## State

If you went to your corner Markdown store and ask a Markdown document, "hey i've been hearing a lot about **state** lately ... what the bloody christ is that?" Undoubtedly one of the less shy Markdown documents would take you aside and say -- _in hushed tones_ -- **state** is our Frontmatter properties. 

They wouldn't be wrong in a normal world but we all know this topsy turvy world we live in is messy so let's let Darkmatter tell us what state is:

### Eager State

Eagerly evaluated state variables get their values set precisely once as soon as Darkmatter composition process starts. They are available _everywhere_ (frontmatter, document body, lifecycle events).

- **Document State** (`doc`) is the Frontmatter values your Markdown friend told you about and is the **primary** form of state in Darkmatter
    - when you use variables that refer to Markdown's Frontmatter you are not required to add the `doc` namespace but you _can_ if you want to be explicit
    - the variable `{{ foo }}` is the same as the variable `{{ doc.foo }}`
    - Darkmatter just assumes a bare variable name is a reference to the `doc` global variable
    - you'll never _need_ to use the `doc.` namespacing prefix unless you have a frontmatter variable called "doc" or that has the same name as a Darkmatter [expression engine](../expression-engine.md)
    - A document -- _as you likely know_ -- can define a dictionary of key/values which we call Frontmatter but in Darkmatter we need to know a bit more:
        - A few of the key's like `$schema`, `title`, `style` are defined by Darkmatter's default schema; that means they are _typed_ and your values must meet the expectation of that type and in most cases that they have some sort of specific semantic meaning to Darkmatter.
        - The number of properties that are imposed by Darkmatter is limited but if you're using the [DMLS](../dmls.md) LSP then they'll all be highlighted to you right in the editor
- **Context Variables** (`ctx`) is a key/value object that Darkmatter provides to you that help you understand the environment you're in
    - often referred to as [_context variables_](./context-variables.md) in the documentation
    - it provides things like the current repo's name, the host's operating system, and lots more
    - one important aspect to know about context variables is that they are considered _eager_ which just means their values are evaluated at the start of the composition process
- **Environment Variables** (`env`) is a key/value objects where you can get the outside environment variables into your document
    - environment variables are all UPPER CASED variables which hang off of `env`
    - they are available on every surface of 

### Lazy State

Lazily evaluated state variables are inexorably tied to [lifecycle hooks](../lifecycle/index.md) and are evaluated at every configured lifecycle event so that a lifecycle hook can respond to changes in state.

- **Lazy Context** (`content`)
    - The `context` variable is precisely the same structure as `ctx` but it provides the most update measurements of these metrics
- **Error State** (`err`)
    - When an error occurs in the lifecycle system, the error's characteristics can be addressed and responded to
- **Timing / Tracking** (`tracking`)
    - You can various measurements on the performance of the running document

### Summary

In summary, **state** consists of various global variables that are provided by Darkmatter but if people are being informal about state they're likely to just mean the Frontmatter data.

## Related Topics

- [Expression Engine](../expression-engine.md) 
    - all state can be used in the a rich expression engine 
    - it includes not only operators like `+`, `-`, `&&`, etc. 
    - but also functions that are guaranteed safe to use because outside of the page's own Frontmatter, they are mutation free.
    - it can be used in frontmatter property definitions, in the body of your document, in lifecycle events, ... everywhere
- [Safe Side Effects](../safe-side-effects.md) 
    - while the Expression Engine offers utility functions that are mutation free to the outside world, **Safe Side Effects** do perform real _mutations_ but are intentionally limited to operations we deem as "safe"
    - safe side effects are only available as part of [lifecycle event cycle]()
- [Flow Control](../flow-control/index.md)
    - during lifecycle events we can choose to redirect the composition flow
    - flow change operations like `proxy`, `stop`, `error`, and others can be specified conditionally
- [Lifecycle Event Cycle](../lifecycle/index.md)
    - Darkmatter has a really powerful lifecycle model that allows you tap into different stages of the _composition_ pipeline
- [Darkmatter Composition Pipeline](^darkmatter/docs/darkmatter-compose-pipeline.md)
    - One of the core capabilities of Darkmatter is _composition_
    - Check out this document if you're interested in the details of what happens in this pipeline to produce the composition results you're hopefully already enjoying

## Reference

- Schemas
    - [Darkmatter Base Schema](^darkmatter/schemas/darkmatter.yaml)
    - [Context Variables](^darkmatter/schemas/context-variables.yaml)
    - [Other Global Variables](^darkmatter/schemas/global-variables.yaml)
