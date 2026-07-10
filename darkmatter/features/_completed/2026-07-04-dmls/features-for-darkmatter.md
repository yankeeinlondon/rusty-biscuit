# LSP Features Applied

This document intends to takes the broader focused scope of the [Markdown LSP Features](./markdown-features.md) document -- which just lays out the common features you'd expect to find in a Markdown LSP -- and starts to drive into specifics for Darkmatter.

## Guiding Principles

- provide all of the functionality of the VSCode Darkmatter LSP (which is likely the most feature rich base set of functionality for Darkmatter)
    - we want to offer a "loose nothing" alternative to choosing to use Darkmatter over everything else
- Darkmatter's primary -- _but not only audience_ -- is for humans who are writing prompts for AI Agents and features which help that audience is our number one priority
- Darkmatter also wants to -- where it can at low cost -- provide a seamless integration into knowledge platforms like Obsidian which use wiki-style links in addition the traditional (one-)


## Baseline Features

> These are features which I believe we can largely "get for free" based on the IWES/IWE implementation starting point

- full CommonMark feature support
- GFM Markdown Features
- Wiki Style links?
    - i'm pretty sure IWES/IWE is meant to work with Obsidian well but we do need to analyze if there any gaps in it's coverage. In particular I'm interested in whether non fully-qualifed file paths would really work:
        - My understanding of the semi-official specs from Wikipedia is that the format is `[[filepath | alias]]` where the `| alias` is purely optional
        - Obsidian allows the "filepath" to be a fully qualified path from the **Vault'** root (which of course right now Darkmatter has no concept of)
        - But Obsidian also allows a basename to be used for filepath and I am not 100% clear on how it resolves this to a fully qualified file path
        - When the file being linked to is in the same directory then that is clearly the highest priority match; not sure after that
        - We should probably have a struct sort of like `FileReference` (in 'biscuit-file' called `ObsidianLink` which codifies these rules)

> Just for context, Obsidian is definitely a target we are interested in having first class support for at some point but it's not currently incorporated and it's not urgent.

## Schema Support

Darkmatter introduces the `SimplifiedSchema` enum (darkmatter/lib/src/markdown/schemas/types.rs) which allows Markdown authors to define schemas for Frontmatter. 

Markdown authors simply define a `$schema` frontmatter property using a grammar that is both ergonomic (especially in contrast to something like JSON Schema) and yet remains surprising powerful in what it can represent. 

We now have a `SimplifiedSchema` for Darkmatter and Claudine:

- Darkmatter: @claudine/docs/schemas/

`SimplifiedSchema` provides a core set of native types while using a concept of "constraints" (which mimics the utility of JSON Schema constraints) to allow a base type to be further constrained or ornamented with metadata.

In most (all?) Markdown LSP's the focus is on the Markdown document's body. DMLS focuses on _both_ the prose content in the body _and_ the YAML frontmatter. That means we'll likely get no "out of the box" support for the Frontmatter portion but because Darkmatter embues special meaning to certain Frontmatter (in Darkmatter but even more in the Claudine library who uses Darkmatter).


## Interpolation

- Interpolation directives in a document should highlight the template tagging by default and when hovered over it should show the current value of the interpolated result (shell commands should not be executed but instead the hover dialog will just show the command which will be )

## Transclusion

- `file` directive
- `code` directive
- conditional `block`s
- 


## Render Pipeline

- YouTube Previews
- Disclosures
-
