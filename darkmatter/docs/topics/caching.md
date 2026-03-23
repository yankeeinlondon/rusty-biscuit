# Caching

## Hashers

We rely on the `biscuit-hash` library not only for the base **xxHash** hashing algorithm but also for "context aware" hashing functions which avoid reporting change on meaningless changes. For instance, if two markdown documents are distinguished _only_ by one have a trailing new line character the document hashes should be the same! The features of the `biscuit-hash` library help us provide us these smart hashes:

- Markdown Prose
    - for a document's body we can be pretty aggressive in removing _semantically meaningless_ whitespace; to do this we'll use the following variants from `biscuit-hash`:
    - `HashVariant::BlockTrimming`
    - `HashVariant::LeadingWhitespace`
    - `HashVariant::TrailingWhitespace`
    - `HashVariant::InteriorWhitespace`
- Frontmatter
    - the frontmatter data is not as lenient regarding whitespace removal so we will use a standard xxHash for this instead of the context aware hashes we're allowed in the content's body

## Document Caching

To detect changes in **documents** (and cache invalidation) we need to understand the granularity of the caching we will be doing. The simplest approach would be to just treat a page at it's most macro level and hash the frontmatter and document body but because of the cost of some _transclusion_ operations we have decided to take a more modular approach:

The document content that is cached will be:

- Document Body, no Frontmatter
- All transclusion references replaced by a simple indexed referencing scheme
    - the index reference would look something like `::{transclusion-operation}-{#}` for all block transclusion directives in body, `{{transclusion-operation}}::{{#}}` for any inline transclusion directives in body
    - this allows the body's hash to only have a sensitivity to the _kind_ and _quantity_ of transclusions the document has but no sensitivity to the specific transclusions because the parameters which create the largest amount of variance are removed from the body
    - this lowered sensitivity allows us a much leaner document cache but means that we will want a performant and ergonomic way to swap "full transclusion directives" with "transclusion references" efficiently (bi-directionally)


## Transclusion Caching

Document caching is one thing but _transclusion_ caching is not the same thing and both are going to be needed. Ultimately, the transclusion cache will be the more important of the two from a performance perspective.

### Why and What is Transclusion Caching

All transclusions operations are described in the [darkmatter pipeline](../darkmatter-compose-pipeline.md) document (with links out to details on each operation) but the commonality for all of them is that they replace a "transclusion directive" with content derived from an external source.

- this external source might be a local Markdown document in the filesystem
- it could be a remote Markdown file pulled via http/https
- it could be a remote web page, sent to an LLM for summarization
- etc.

While the transclusion a from local Markdown file is a relatively straightforward and performant operation, other transclusions are very expensive and caching is the only way to make them practical.

> **Note:** even our "low cost" example of a local Markdown file is only low cost if the referred to document doesn't itself have expensive transclusions in it. This recursive nature of transclusions adds to the complexity and cost of this operation (although it also adds to it's power and desirability)

### Transclusion Cache Strategy

A transclusion's hash is composed of the _operation_ and _parameters_ which a document uses in their transclusion reference. However, we need to be able to break the parameters of each transclusion operation into two categories if we want to maximize cache hits:

1. Transclusion Param
2. Post Transclusion Param

This separation will allow us to base the hash **only** on the parameters which will cause variations in how the transclusion operation will render; not on any post processing steps required in the host document.

> **Note: having this concept of two different types of transclusion parameters is new and it will likely be worth formalizing the implementations of transclusion operations with a `Transclusion` trait. This trait could be made responsible for imposing a pre and post step to the operation where:
>
> - the "pre" step is the cacheable part (e.g., expensive, async, etc.)
> - and the "post" step should only be quick, synchronous based operations. 
> 
> This formality would forcing each operation to think through which aspects of their operation are expensive and should be made cacheable, versus those aspects which can be quickly layered on top afterward. Because each operation is formally broken into these two parts it also becomes much clearer which _parameters_ should be considered a transclusion param versus a post-transclusion param. 
>
> Finally the same **trait** could be used to force the operation to provide a `cacheable_params()` function which would allow the operation to define how to segment which parameters are considered "transclusion params". This allows the caching layer to remain oblivious to each operations internals and can simply ask the operation to provide the segmentation in a consistent fashion. 

Let's do a small exploration of what a "transclusion param" versus a "post transclusion param" is via an example:

- the [`file::` directive](../transclusion/block-transclusion.md#block-transclusion) allows a parent document to reference another local file for transclusion 
- the _file reference_ is clearly a critical "transclusion parameters" 
- we will now look at two parameters which need a little more consideration:
    - `exclude`, and
    - `replace`
- **exclude:**
    - the `exclude` parameter allows us to _exclude_ certain sections of the child document before injecting it into the parent document
    - the search expression the parent document is allowed to use is 

### Cache Lifetimes and Fallbacks


