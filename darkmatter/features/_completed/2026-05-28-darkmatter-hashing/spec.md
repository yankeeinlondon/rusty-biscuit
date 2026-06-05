# Darkmatter hashing feature

We have had Darkmatter hashing for a while but there's been a big whole in the functionality:

- you can run `md hash <file>` and it produces two hashes joined by the `-` character:
    - the first hash represents the frontmatter
    - the second hash represents the body's content
    - these hashes are "Markdown aware" so they know how to ignore whitespace differences which would make zero semantic difference
- so far this all looks good
- so what's missing?
    - the main thing is that if you were to save the hash to the markdown file (as you will often want to do) you need to have a policy which _ignores_ the frontmatter property which holds the key
    - as a secondary concern we may want to store different hash variants in documents and there is no way to do that currently

## Feature Definition

> **Note:** the functionality we'll describe here will be discussed via the CLI interface of `md hash ...` but the changes we
> expect to be made effect both the library and the CLI. Remember that the CLI should contain almost no business logic of it's
> own, it's job is simply to call into the library and report results.

> **Library/CLI boundary.** The library takes **explicit options** — the hash property name, the extra-ignored
> property list, the forced kind, and the save/diff mode — and the CLI is responsible for reading the environment
> variables (`HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`) and the flags (`--kind`, `--save`, `--diff`) and populating
> those options. The library reads no environment of its own, so it stays deterministic and testable.

- the _assumed_ Frontmatter property that will be set on a Frontmatter document is `hash`, however:
    - the CLI will change the assumed Frontmatter property if HASH_PROPERTY is set
    - the library will provide an easy way to switch from the default of 'hash'
- the hash Frontmatter property can take one of two forms:
    - shorthand form is a single string that takes the form of: `{frontmatter-hash}-{body-hash}`
    - the longhand form is defined by a "kind" and "value" property under "hash":

        ```yaml
        hash:
            kind: simple
            value: "asdfasdfasdfas-adsfasfasdfda"
        ```

    - the on-disk shape of `value` depends on the kind:
        - `simple`, `structured`, `fm`, `body` — `value` is a single string. For `simple` it is `{fm-hash}-{body-hash}`; for `structured` it is the four-part `{fm-hash}-{fm-keys-hash}-{body-hash}-{body-structure-hash}` string; for `fm` and `body` it is a single hash.
        - `detailed` — `value` is a **nested YAML object**, because the comparison rules read a stored detailed value back for downgrade/comparison, so its persisted shape must be defined:

            ```yaml
            hash:
              kind: detailed
              value:
                frontmatter: { fm: aaaa, keys: bbbb }
                preamble: cccc          # or null
                sections:
                  - [2, "Installation", eeee]
                  - [3, "Configuration", "0000"]
            ```

    - the "kind" property is a reference to a new `MdHashKind` enum (new) which enumerates the various kinds of hashes that Darkmatter supports
        - all hashes are based on the **xxHash** algorithm and use the `biscuit-hash` library for all hashing functionality
        - the different "kinds" simply describe structurally what is being hashed:
        - The variants should be:
            - `fm` - a hash of the frontmatter (keys and values)
            - `body` - a hash of the markdown's body
            - `simple` - the basic `{frontmatter-hash}-{body-hash}` which is used today and is the fallback default when a document has no existing hash (see **Kind selection** below)
            - `structured` - a four part hash of the form `{fm-hash}-{fm-keys-hash}-{body-hash}-{body-structure-hash}`
                - the structured hash has the same hashes found in simple but also adds two more which help to identify changes in "structure"
                - the `fm-keys-hash` hashes all the Frontmatter keys and ignores their values
                - the `body-structure-hash` hashes all headings in the document including the heading level, so:
                    - an H2 heading `## Section` is hashed as `## Section` not `Section` so as to preserve the name and the level
                    - all headings, in order of their appearance, concatenated together to form one long string slice and then that string slice is hashed to represent the state of the body's structure
            - `detailed` - produces an object response with the following keys:
                - **frontmatter**: the same pair of hashes carried by `structured` - a `fm-hash` over all keys and values and a `fm-keys-hash` over the keys alone - so frontmatter changes can be reported at key-vs-value resolution
                - **preamble**: `{hash}` - a hash of the content before the first heading is encountered; leading and trailing whitespace ignored ... if there is no text before the first heading then this `null`
                - **sections**: each section is organized as a tuple of `[ level-num, heading, content-hash ]`, where:
                    - the `level-num` is a number (1-6) representing the heading level of the section
                    - the `heading` is the **literal heading text** — NOT a hash — with the leading `#` characters and surrounding whitespace removed
                    - the `content-hash` is a hash of all content after the heading and _up to_ the next heading which is at the same level or a parent level
- **Kind selection** — which `MdHashKind` is generated when none is forced:
    - by default the kind is **matched to whatever the document already declares** in its `hash` property — a shorthand string is read as `simple`; a longhand object is read as its declared `kind`
    - if the document has no existing `hash` property, the default kind is **`simple`**
    - the CLI accepts a `--kind <k>` switch (`fm`, `body`, `simple`, `structured`, `detailed`) that **forces** the kind, overriding the matched/default kind; the library exposes the same choice programmatically
    - forcing a kind with `--kind` together with `--save` is the intended way to deliberately change a document's hash kind (an upgrade or downgrade in resolution — see the `--save` semantics below)
    - **Resolution** orders the kinds by how much they capture: `simple` < `structured` < `detailed`, each a strict superset of the one before. `fm` and `body` are *partial* kinds — each captures only one side of `simple` — so both are lower resolution than `simple` but are not comparable to one another. "Higher/lower resolution" in the `--save` rules below refers to this ordering.

- Just like today, the CLI will provide the response to STDOUT; it will **not** save the hash to the file
- However, we will add the `--save` switch which will:
    - evaluate the current hash property:
        - if the hash type is the same then we evaluate for changes
            - if there are changes then update the `hash` property with the new hash value AND we set `last_updated` frontmatter to today's date `YYYY-MM-DD`
            - if there are no changes then the file is not modified
        - if the hash type is different (only possible when `--kind` forces a new kind):
            - **higher resolution** (the forced kind is a strict superset, e.g. `simple` → `structured`): evaluate the document at the old, lower resolution; if nothing changed there we just upgrade the resolution in the `hash` property and leave `last_updated` alone; if there is a change at the lower resolution we upgrade the resolution and update `last_updated` too
            - **lower resolution** (the forced kind is a strict subset, e.g. `detailed` → `simple`): recompute at the new kind and compare it against the corresponding components of the stored higher-resolution value; rewrite `hash` at the lower kind, setting `last_updated` only if that lower-resolution view actually changed
            - **incomparable** (the two kinds share no overlapping component, e.g. `fm` ↔ `body`): we cannot evaluate change across the switch, so we simply rewrite `hash` at the forced kind and leave `last_updated` untouched — a kind switch alone is not a content change
    - **Write-back fidelity.** `--save` performs a **full re-serialization** of the document to establish a new on-disk
      baseline, using the same library primitive that `md clean --save` already uses (`Markdown::as_string`, in
      `darkmatter/lib/src/markdown/output/string.rs`). Frontmatter is re-emitted canonically from the parsed key/value
      map: key order is preserved, but YAML comments, quoting style, and block-scalar formatting are **not** preserved.
      This matches existing `clean` behavior; no per-byte fidelity guarantee is offered. (`--save` is **not** the first
      write-back path — `md clean --save` predates it.) The library mutates the frontmatter model and exposes the
      serialization; the CLI performs the `fs::write`.
    - **The body is written byte-for-byte unchanged.** `--save` only injects/updates the hash (and `last_updated`) in
      the frontmatter; it must **not** run `clean`'s body normalization. The body hash is computed over the body content
      directly, so normalizing the body after computing the hash would make the recorded body hash not match the bytes
      written. Body cleanup remains the job of `md clean`.
    - when the `--save` flag is used we do NOT return the hash information but instead an explanation of the changes
        - the explanation we're able to provide will of course vary based on the type of hash strategy we have (and it's a lowest common denominator in terms of the explanation level)
        - after saving, STDOUT explains the differences discovered, reusing the existing delta-reporting path
    - see the Explaining Differences section
    - **Comparison uses the stored ignore-set (like-for-like).** Whether the document changed is **always** evaluated
      by recomputing under the document's **stored** `hash.ignored` set (see Ignored Properties) — never under the
      current `HASH_IGNORE_PROPERTIES` env value. The env value matters only when **writing** a new baseline on
      `--save`. This keeps results reproducible across machines/CI; an implementer should be able to write a test where
      the stored ignore-set and the env ignore-set deliberately differ and assert the document still compares as
      unchanged. When the stored `ignored` set differs from the current env set, surface a **separate advisory line**
      (e.g. `Ignore policy changed: now also ignoring [foo]; previously [bar]`) that is **not** counted as a content
      change.
    - **An ignore-policy-only change does not bump `last_updated`.** On `--save`, if the only difference is the
      ignore-set (no document content change under the stored set), rewrite `hash.ignored` and recompute `value` under
      the new set, but leave `last_updated` **untouched** — `last_updated` tracks document content change, not operator
      policy/config change.
    - we will also include a `--diff` flag which not update anything in the underlying file but will will explain the differences detected instead of the hashes

### Exit codes and flag interactions

`--save` and `--diff` carry distinct exit-code contracts, and they are **mutually exclusive** — enforced at the CLI
arg layer via clap `conflicts_with`; passing both is a usage error. The bare `md hash` (neither flag) is unchanged: it
prints the hash to STDOUT and exits **0**.

- **Exit code 0** — no differences detected, or a successful `--save` (whether or not it actually wrote). This matches
  `md clean --save`, which exits 0 on a successful save regardless of whether the file needed changing.
- **Exit code 1** — operational error: file not found, not Markdown / no frontmatter block when one is required,
  malformed YAML, or an unreadable/unwritable file. This is the existing eyre error path in
  `darkmatter/cli/src/main.rs`.
- **Exit code 2** — `--diff` detected differences. This reuses the repo's existing "valid document, but the check found
  something" convention (`darkmatter/cli/src/commands.rs:583` and `:1882`).

**No existing stored hash** (the document has no `hash` property) is handled per flag:

- under `--diff`: there is nothing to compare against, so report `No stored hash to compare against` and exit **2**
  (treated as "differs").
- under `--save`: simply write the first baseline and exit **0**.

## Ignored Properties

We **always** ignore two managed keys: the `hash` frontmatter property (or its replacement via `HASH_PROPERTY`) and the
`last_updated` property. These are always-ignored regardless of configuration. `last_updated` is always-ignored because
`--save` writes it, so it must never feed back into the hash.

- the `HASH_IGNORE_PROPERTIES` environment variable is **additive**: it adds further properties to ignore **on top of**
  the always-ignored keys. It is a CSV value, so `foo,bar` adds `foo` and `bar` to the ignore-set. It does not and
  cannot un-ignore `last_updated` or `hash`.

Being able to ignore certain frontmatter properties ensures that all change detection properties are not included in
hashes designed to detect change but you can also remove other properties which are "noisy" and you don't care about
when they change.

- **Exact key matching.** Ignore-property names are matched against frontmatter keys **byte-exactly**, consistent with
  how all other frontmatter keys are handled — the frontmatter map applies no key normalization. There is **no**
  hyphen/underscore fuzzy matching. The canonical spelling is snake_case (`last_updated`).

### Recording the extra-ignore policy: the `ignored:` field

When extra ignore-properties (beyond the always-ignored `hash` + `last_updated`) are in effect, the saved `hash` object
records an `ignored:` list naming **only** those **extra** properties, so later comparisons are like-for-like against
the baseline.

- the list contains only the extras, **not** the always-implied managed keys
- the list is **sorted** for stable diffing
- an absent `ignored` is equivalent to an empty list — never write `ignored: []`

**Promotion invariant.** The `hash` property is stored in shorthand string form **if and only if** `kind` is `simple`
**and** there are zero extra ignored properties. The moment any extra ignored property is in effect, `hash` **must** be
longhand:

```yaml
hash:
  kind: simple
  value: "aaaa-bbbb"
  ignored: [project, tags]
```

Promotion never changes the kind. `ignored` is **orthogonal** to kind — for `structured` / `detailed` / `fm` / `body`
(already longhand) it is simply an additional optional field on the object.

## Explaining differences

The `Markdown` struct has a whole "delta" engine for a while now but it's explanation for what changed are poorly implemented.

At some point we'll likely replace it's descriptive features with what this feature produces but for now we'll just keep
them as separate implementations. As discussed before, the amount we can describe about the change depends on the "kind" of hash we have.

### **simple** Hashes

All we know is whether the frontmatter, the body or both changed. We use smart-whitespace rules to avoid non-semantic
changes from being detected as a change. The reporting would be one of the following:

- `No semantic changes detected`
- `Frontmatter has changed, body remains unchanged semantically`
- `Frontmatter remains unchanged, but body has changed`
- `Both the Frontmatter and body have changed`

### **fm** and **body** Hashes

These are the single-concern degenerate forms of the two-concern **simple** model: each carries only one side of
`simple`, so each reports only its own concern.

- `fm` reports only the Frontmatter concern — there is no Body line:
    - `Frontmatter has changed`
    - `Frontmatter remains unchanged`
- `body` reports only the Body concern — there is no Frontmatter line:
    - `Body has changed`
    - `Body remains unchanged`

**Empty/absent frontmatter is not an error.** An empty or absent frontmatter block is hashed as the empty-frontmatter
hash (`xx_hash("")`), so `md hash --kind fm` on a frontmatter-less document produces a stable hash. A `--diff` against
a stored `fm` hash for a document that has since lost its frontmatter simply reports `Frontmatter has changed`.

### **structured** Hashes

Now we can distinguish between structural changes from content changes to both Frontmatter and the body. Possible messages are:

- `No semantic changes detected`
- `- Frontmatter has changed but no changes to the keys, only to the values\n- No semantic changes to the body`
- `- Frontmatter has changed but no changes to the keys, only to the values\n- The body has the same structural layout as before, however, there are semantic changes to the contents within the content`
- `- Frontmatter has changed but no changes to the keys, only to the values\n- The body has changed both structurally as well as the semantic content`
- `- Frontmatter has not changed\n- The body has the same structural layout as before, however, there are semantic changes to the contents within the content`
- `- Frontmatter has not changed\n- The body has changed both structurally as well as the semantic content`

### **detailed** Hashes

Now we have a full understanding of precisely which sections in the body have changed — to the heading,
the heading's content, or both — plus the preamble and the document's section layout (additions, removals,
and moves).

#### What we can compare

A detailed hash captures:

- **frontmatter** — the same two hashes used by **structured** (a hash of all keys and values, and a hash of the
  keys alone) so frontmatter changes can be reported at key-vs-value resolution
- **preamble** — one hash (or `null`) for the content before the first heading
- **sections** — an ordered list of `[ level-num, heading, content-hash ]` tuples, in document order, where `heading`
  is the literal heading text (not a hash) and `content-hash` is a hash

Comparing the stored hash against the freshly computed one, we can therefore report, per section: whether its
heading changed, whether its content changed, whether its level changed (promotion/demotion), and whether it
moved relative to its siblings — and across the whole body, which sections were added or removed.

Just as with the other kinds, smart-whitespace rules apply so non-semantic whitespace differences never register
as a change.

#### Naming caveat

The explanation is generated while the live document is in hand, so any section that **still exists** can be
named using its current heading text. A **removed** section can also be named: the stored hash records the literal
heading text in each section tuple, so a section that exists only in the stored hash is still reported by name (along
with its level and approximate position).

The caveat applies solely to a **removed preamble**: the preamble's stored value is only ever a hash, never text, so a
preamble that no longer exists can be reported only as "the preamble was removed" — it has no name to surface.

#### Section identity / alignment

Because both a heading and its content can change at once, matching old sections to new ones is an alignment
problem rather than a lookup. The default:

1. Anchor on sections whose `heading` text is unchanged (a heading match), then compare their `content-hash`.
2. For the remaining unmatched sections, pair by `content-hash` where the content is identical — these are
   **renames** (heading changed, content unchanged).
3. Pair any sections still unmatched by their corresponding position at the same level — these are treated as
   **renamed-and-edited** (both heading and content changed but the section persisted). See the tie-break below.
4. Only after positional pairing, a surplus of unmatched sections on the new side is **added** and a surplus on
   the old side is **removed**.
5. A matched section whose `level-num` differs is **promoted** / **demoted**; one whose ordinal position among
   its siblings differs is **reordered** / **moved**.

This classifies every section as one of: **unchanged**, **content-changed**, **renamed**, **promoted**,
**demoted**, **reordered**, **moved** (to a different parent), **added**, or **removed**. These are the change
categories the hashing feature owns directly. The legacy `Markdown` delta engine's change *descriptions* have
always been unsatisfactory and are not authoritative here; the categories above are defined by this feature
independently. Where parts of the delta engine's underlying machinery (e.g. section alignment) prove sound, they
may be salvaged and reused — but the user-facing classification and messaging are owned by this feature.

**Tie-break — both heading and content changed.** A section whose `heading` text *and* `content-hash` both differ
from every old section is genuinely ambiguous: it could be one section that was renamed *and* rewritten
(rename-plus-edit), or an old section removed and an unrelated new one added in its place (remove-plus-add).
We default to **rename-plus-edit** — when an unmatched old section and an unmatched new section occupy the same
relative position at the same level, we treat them as the same section that changed in both respects and report
a single `heading renamed and content has changed` line. This preserves section continuity and keeps reports
compact; genuine adds/removes surface only as the leftover surplus once positional pairing is exhausted.

As in the **simple** and **structured** kinds, every report has the same two top-level concerns —
**Frontmatter** and **Body**. The difference is that the body now has internal structure, so the Body concern
expands into a **nested list** with one child item per changed section (in document order); when the preamble
changes it is the first child item. Sections with no change are omitted; a removed section is named from its stored
heading text (with level and position as added context), while — per the naming caveat — a removed preamble has no
name and is reported simply as removed. The Frontmatter concern keeps the same resolution as **structured**
(distinguishing key changes from value-only changes).

The message space is large, so the following are **representative, not exhaustive**. Each item is one complete
report, with `\n  - ` marking a nested body child:

- `No semantic changes detected`
- `- Frontmatter has changed but no changes to the keys, only to the values\n- No semantic changes to the body`
- `- Frontmatter has not changed\n- There were changes to the document body:\n  - "Installation" section: content has changed`
- `- Frontmatter has not changed\n- There were changes to the document body:\n  - "Configuration" section: heading renamed, content unchanged\n  - "Usage" section: content has changed`
- `- Frontmatter has not changed\n- There were changes to the document body:\n  - The preamble has changed\n  - "Quick Start" section: heading renamed and content has changed`
- `- Frontmatter keys and values have changed\n- There were changes to the document body:\n  - "Usage" section: promoted from H3 to H2\n  - "Appendix" section: demoted from H2 to H3, content unchanged`
- `- Frontmatter has not changed\n- There were changes to the document body:\n  - "Examples" section: reordered, now appears before "Usage"\n  - "Caveats" section: moved beneath "Advanced" (previously beneath "Usage")`
- `- Frontmatter has not changed\n- There were changes to the document body:\n  - A preamble was added before the first heading\n  - "Troubleshooting" section: added (H2)\n  - "Changelog" section was removed (previously between "Usage" and "Examples")`

To make the nesting concrete, the "Frontmatter keys and values have changed" example renders as:

```text
- Frontmatter keys and values have changed
- There were changes to the document body:
  - "Usage" section: promoted from H3 to H2
  - "Appendix" section: demoted from H2 to H3, content unchanged
```
