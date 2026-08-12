# Text Replacement

Text Replacement allows us to find/replace a string pairing in the document body. To enable this in Darkmatter we treat the `replace` frontmatter property with special meaning:

- to be considered appropriate for a Darkmatter text replacement, `replace` property is a dictionary type; for example:

    ```md
    ---
    replace:
        foo: bar
        one: two
    ---

    Some prose about foo.
    ```

    > **Note:** if `replace` is defined but is NOT a dictionary, it is simply ignored from a Text Replacement standpoint

- Calling `.compose()` on a Markdown struct kicks off Markdown composition features
    - this has nothing to do with exporting to a target output like HTML, terminal, AST, etc.
    - this is the trigger which will update the content based on the Darkmatter DSL found on the page
- When we transform content with the `replace` property a dictionary we will find all _keys_ in the dictionary and replace them with the _value_ for the given _key_.
- In our simple example above the body of the page would change from `Some prose about foo.` to `Some prose about bar.`

## Ordering

the **Text Replacement** feature is

- run as the first step of the Darkmatter, and
- immediately _before_ the [Frontmatter Interpolation](./interpolation.md)

Refer to the [Darkmatter Pipeline](./darkmatter-compose-pipeline.md) document for a full overview of sequencing.


## Technical Design Options

### Shared Resolution Rules (all options)

Regardless of implementation path, the replacement step should resolve inputs in a single predictable way:

- At compose pipeline entry, create an **effective state** by merging:
    - the optional incoming state map, and
    - the document's parsed frontmatter
- Make merge precedence explicit in implementation (recommended for runtime use cases: incoming state overrides on conflict, equivalent to `PreferExternal`)
- Read `replace` from the effective state (not just the original document frontmatter)
- If `replace` is missing or not a dictionary/map, skip this step with no error
- Treat replacement keys as literal, case-sensitive strings
- Ignore empty-string keys (to avoid non-terminating scans)
- For replacement values:
    - allow scalar values (`string`, `number`, `boolean`, `null`)
    - coerce scalars to string (`null` becomes empty string)
    - ignore non-scalars (`array`, `object`) in v1

This keeps Text Replacement deterministic and compatible with runtime-provided state.

### Option 1: Source-First Deterministic Rewriter (Recommended for v1)

Implement replacement as a body-to-body transform stage (stage input buffer -> stage output buffer) using a left-to-right scanner:

- Build replacement rules from the effective `replace` map
- Resolve overlaps by deterministic precedence:
    - longest key wins
    - ties break lexicographically (stable output across runs)
- Apply in a **single pass** (non-recursive): replacement output is not scanned again
- Mutate only the markdown body content (never frontmatter during this step)

Pros:

- preserves source formatting exactly (no markdown reserialization churn)
- easy to place as first transform stage before interpolation
- deterministic overlap behavior avoids accidental key-shadow bugs
- no dependency on parser event boundaries

Cons:

- markdown-agnostic (replaces inside code fences, inline code, links, etc.)
- naive implementations can be slower with very large rule sets unless optimized

### Option 2: pulldown-cmark Event-Scoped Replacement (Markdown-aware Hybrid)

Use `pulldown-cmark` with offsets to constrain where replacement is allowed:

- Parse with `Parser::new_ext(...).into_offset_iter()`
- Only apply replacements inside eligible events (typically `Event::Text`)
- Skip replacement in code spans/blocks by policy
- Patch original source by offsets (descending order) to avoid reserialization

Pros:

- markdown-aware and policy-controlled (for example, "never replace in code")
- leverages parser infrastructure already used in Darkmatter
- keeps original markdown text by patching source offsets

Cons:

- higher bookkeeping complexity than a pure source scanner
- event segmentation can make edge cases harder (tokens split across events)
- more implementation overhead for first milestone

### Option 3: markdown-rs MDAST Transform Pass (Pipeline-First Architecture)

Implement Text Replacement as an AST transform stage:

- Parse to MDAST using `markdown-rs`
- Visit text-bearing nodes and apply replacement rules
- Serialize transformed markdown for downstream stages

Pros:

- aligns with long-term multi-stage pipeline architecture
- explicit node-level semantics for future structural features
- easier to compose with future AST-level transforms

Cons:

- round-trip fidelity depends on markdown serialization behavior
- higher memory/runtime cost than source patching
- likely over-engineered for an initial literal replacement feature

### Recommendation

For first implementation, choose **Option 1** with explicit overlap rules and single-pass semantics.  
If users need markdown-aware guardrails early (for example, skipping code blocks), evolve to **Option 2** while preserving the same effective-state resolution and replacement semantics.  
Reserve **Option 3** for the point where multiple structural transforms justify a full AST-native pipeline.
