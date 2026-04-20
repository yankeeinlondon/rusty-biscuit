# Setting Frontmatter Properties in Transclusion

The `::file` directive in Darkmatter provides a [block transclusion](darkmatter/docs/transclusion/block-transclusion.md) feature that brings in another local Markdown file. This feature provides the ability to SET state on the referenced file in a similar fashion to have the Darkmatter CLI's `--set JSON5` switch does.

## Syntax

There are two forms of the **set** syntax:

- JSON5 Object based
- Property based

Both forms are in scope for v1.

### The `set` property

Using the syntax `set=VALUE` syntax a user is able to assign a key/value dictionary to the transcluded file's Frontmatter:

```md
::file foo.md set='{ name: "Bob", age: 42 }'
```

This also allows a caller to pass in one of it's own Frontmatter properties too:

```md
---
dictionary: 
    name: "Bob"
    age: 42
---
::file foo.md set={{dictionary}}
```

Of course for this to work the property passed in MUST be shaped as a dictionary. If it is not then this will result in a `InvalidFrontmatterAssignment` error.

- the CLI should include a `--allow-invalid-frontmatter-assignment` which will remove this error condition and replace it with a well formed warning message to STDERR. Only the offending clause is skipped: sibling setters on the same directive line that are independently valid STILL apply. For example, `::file foo.md set=42 set.name="Bob"` under this flag emits a warning about the `set=42` clause, drops that clause, and the effective child frontmatter still receives `name: "Bob"`. Note: this feature should be made available in the Darkmatter library's configuration settings.

### The Property Setter 

It is often much more convenient to set just a single property on the referenced file and we will use dot notation for this:

```md
::file foo.md set.name="Bob" set.age=42
```

We will allow for a dictionary set to be done _in addition to_ the property set operations but in this case the property set operations take precedence if there is any overlap in keys.

#### Property Errors

If a block transclusion sets the same property twice:

```md
::file set.name="Bob" set.name="Mary"
```

then this will cause a `InvalidReassignedFrontmatterProperty` error.

The CLI should include a `--allow-reassigned-frontmatter-property` CLI switch. When used it will no longer create an error but it will still provide a useful warning message to STDERR and the last (right-most) assignment will be used.

A duplicate object-form assignment such as `::file foo.md set='{...}' set='{...}'` is likewise an error. This is treated as symmetric with `InvalidReassignedFrontmatterProperty`; the same error variant is extended to cover the object form (or a parallel `InvalidReassignedFrontmatterObject` variant may be defined — implementer's call).

## Grammar

The `::file` directive options are whitespace-separated on the directive line. Whitespace is a required separator between options; values that themselves contain whitespace MUST be quoted (already enforced by JSON5 string rules).

### `set=<value>` (object form)

- The RHS is parsed as a **JSON5 value** that MUST evaluate to an object (dictionary).
- Valid: `set='{ name: "Bob", age: 42 }'`, `set="{count: 3}"`.
- Invalid (not an object): `set=42`, `set="hello"`, `set=[1,2]` → these raise `InvalidFrontmatterAssignment`.
- Duplicate `set=` on the same directive line is an error (see above).

### `set.NAME=<value>` (property form)

- `NAME` is a **single identifier segment**. Nested dotted keys are NOT supported in v1.
    - `set.author="Bob"` — valid
    - `set.author.name="Bob"` — **parse error** in v1
- The RHS is parsed as a **JSON5 value**: scalar (`42`, `true`, `false`, `null`, `"Bob"`), array (`[1, 2, 3]`), or object (`{ name: "Bob" }`).
- Unquoted bare words are NOT accepted as strings. `set.name=Bob` is a parse error; users MUST write `set.name="Bob"`.
- An empty RHS (`set.foo=`) is a parse error. It is not interpreted as empty string or null.
- Bare `set` without `=` is invalid.
- A repeated property assignment (`set.name="Bob" set.name="Mary"`) raises `InvalidReassignedFrontmatterProperty` as described above.

### Null Semantics

A `null` RHS — whether appearing as `set.x=null` or inside a `set=<dict>` payload such as `set.author='{name: null}'` — sets the key to the **literal null value**. It does NOT delete the key or act as a tombstone. Under deep-merge, `set.author='{name: null}'` sets `author.name` to null; the child's original `author.name` is overridden with null, but the surrounding `author` dict is still deep-merged per the normal rules. Tombstone/delete semantics are explicitly out of scope for v1 and deferred to a future spec if a concrete use-case emerges.

### Interpolation on the RHS

The parent document's pipeline performs interpolation (stage 5) BEFORE transclusion is processed — see `darkmatter/docs/darkmatter-compose-pipeline.md`. As a consequence, expressions like:

```md
::file foo.md set={{dictionary}}
::file foo.md set.x="{{ env.X }}"
```

are resolved by the parent's stage-5 interpolation into literal JSON5 values before the `::file` directive is parsed for transclusion. "Interpolation on the set RHS" is therefore not a distinct feature of this spec — it is automatic parent-side behavior that the grammar simply inherits.

## Merge Semantics

When a `::file` directive carries any `set=` and/or `set.NAME=` options, the resulting frontmatter the child pipeline sees is produced by a **three-layer precedence** merge:

1. **Base layer** — the child file's own frontmatter (as authored in its `---` block).
2. **Middle layer** — the object-form `set=<dict>` payload (if present), merged on top of the base.
3. **Top layer** — each `set.NAME=<value>` property, merged on top of the middle layer.

At every layer, the rule is:

- **Leaf values**: hard override. The higher layer wins on any leaf-value conflict.
- **Dict values**: deep-merge (recursive union of keys). Overlapping leaves inside the dict follow the same hard-override rule, with the higher layer winning. This preserves the repo's existing deep-merge convention documented in `darkmatter/docs/composition/frontmatter-in-pipelining.md`, while inverting precedence so the caller's `set` wins over the child's authored frontmatter.

### Worked Example

Child file `foo.md`:

```md
---
name: "Alice"
author:
  name: "Alice"
  handle: "@alice"
tags: ["red", "green"]
---
```

Parent directive:

```md
::file foo.md set='{ author: { handle: "@bob" }, tags: ["blue"] }' set.name="Bob"
```

Effective frontmatter seen by the child's pipeline:

```yaml
name: "Bob"              # set.name leaf override
author:
  name: "Alice"          # preserved from child (deep-merge union)
  handle: "@bob"         # set= dict leaf override
tags: ["blue"]            # arrays are leaves; set= replaces entirely
```

Key points illustrated:

- Dict-valued `author` is deep-merged: the child's `author.name` survives because `set=` only supplied `author.handle`.
- The array-valued `tags` is a leaf from the merge engine's perspective: it is replaced, not concatenated.
- `set.name` (top layer) beats any `name` that might have come from a `set=` object (middle layer), and both beat the child's authored `name`.

## Pipeline Integration

The `set` values act as **caller-supplied parameter overrides** on the transcluded file. For that mental model to hold, the child's own conditional logic and interpolation must be able to observe them. Accordingly, the override is **overlay-first**: `set` values are installed on the child's frontmatter BEFORE any of the child's pre-op stages (1-5) run.

In the child's pipeline, the following stages therefore observe the overridden values:

1. Frontmatter interpolation
2. Frontmatter shell expansion
3. Text replacement (`replace:` rules)
4. Page blocks (`::block when="…"` conditions evaluate against overridden values)
5. Interpolation (`{{ fm.* }}` sees overridden values)
6. Shell expansion (`::shell` directives run in an environment where overridden values are present)

### Parent-side ordering context

Per `darkmatter/docs/darkmatter-compose-pipeline.md`, the parent's stage-5 interpolation runs **before** the parent's transclusion stage. This means a construct like `set={{dictionary}}` or `set.x="{{ env.X }}"` on the parent side is resolved to a literal value before the `::file` directive is even dispatched to transclusion. This is why the spec treats "RHS interpolation" as out of scope for the transclusion feature itself — it is already handled by the parent's standard pipeline stages.

## Warning Messages

Warning messages should be rendered using Darkmatter's existing warning-presentation mechanism (the `Status` struct from biscuit-terminal in WARN state, wrapped in a `BlockQuote` with an orange vertical bar). If that mechanism is available in the codebase it is to be used directly. Otherwise, the format is:

- `Status` struct from biscuit-terminal in WARN state: `<b>{error-name}</b>`
- The warning message will be wrapped in a `BlockQuote` struct from biscuit-terminal and will have an orange vertical bar.
- The message inside the `BlockQuote` will be:
    - `{general description of error}:`
    - code block showing the line causing the error with the line before and line after for context
    - blank line
    - `- this occurred in the <blue><a href={abs-path}>{relative-path}</a></blue> file`
    - `- because of possible transclusion the line number may not be reliable but before transclusion it was on line <yellow>{#}</yellow>`

The important thing is that the user is given enough information to act on the problem.

## Acceptance Criteria

Testable outcomes that define done for this feature:

### Grammar

- `::file foo.md set.name="Bob"` parses successfully; effective child frontmatter has `name: "Bob"`.
- `::file foo.md set.name=Bob` (unquoted bare word) is a **parse error**.
- `::file foo.md set.name=` (empty RHS) is a **parse error**.
- `::file foo.md set` (bare, no `=`) is a **parse error**.
- `::file foo.md set.author.name="Bob"` (nested dotted key) is a **parse error** in v1.
- `::file foo.md set=42` (non-object JSON5) raises `InvalidFrontmatterAssignment` (or warns under `--allow-invalid-frontmatter-assignment`).
- `::file foo.md set='{a:1}' set='{b:2}'` is an error (duplicate object-form assignment).
- `::file foo.md set.name="Bob" set.name="Mary"` raises `InvalidReassignedFrontmatterProperty` (or warns under `--allow-reassigned-frontmatter-property` and uses `"Mary"`).
- `set.age=42`, `set.tags=[1,2,3]`, `set.meta={x:1}`, `set.x=null`, `set.ok=true` all parse as JSON5 values of the respective types.

### Null Semantics

- Given child frontmatter `{ x: 5 }` and directive `set.x=null`, the effective frontmatter is `{ x: null }` — the key remains present with a null value (not removed).
- Given child frontmatter `{ author: { name: "Alice", handle: "@alice" } }` and directive `set.author='{name: null}'`, the effective `author` is `{ name: null, handle: "@alice" }` — `author.name` is set to the literal null, and the surrounding dict is deep-merged.

### Flag Behavior

- Under `--allow-invalid-frontmatter-assignment`, given directive `::file foo.md set=42 set.name="Bob"`: the `set=` clause is dropped with a STDERR warning; the effective child frontmatter includes `name: "Bob"`. Sibling valid setters on the same directive line are unaffected.

### Merge Semantics

- **Dict deep-merge**: given child frontmatter `{ a: { x: 1 } }` and directive `set.a='{y:2}'`, the effective `a` is `{ x: 1, y: 2 }`.
- **Leaf override**: given child `{ name: "Alice" }` and directive `set.name="Bob"`, the effective `name` is `"Bob"`.
- **Three-layer precedence**: given child `{ name: "Alice" }` and directive `set='{name: "Carol"}' set.name="Bob"`, the effective `name` is `"Bob"` (property-form top layer wins).
- **Arrays as leaves**: given child `{ tags: ["a","b"] }` and directive `set.tags=["c"]`, the effective `tags` is `["c"]` (no concatenation).

### Pipeline Integration (overlay-first)

- A child block guarded by `::block when="role == 'admin'"` renders when the parent calls `::file child.md set.role="admin"`, even if the child's own frontmatter sets `role: "guest"`.
- A child line containing `{{ fm.name }}` renders as `Bob` when the parent calls `::file child.md set.name="Bob"`, even if the child's own frontmatter sets `name: "Alice"`.
- A child `::shell` directive sees the overridden values in its evaluation environment.
- A child `replace:` rule sees overridden frontmatter values.

### Parent-side interpolation passthrough

- Given parent frontmatter `dictionary: { name: "Bob" }`, the directive `::file foo.md set={{dictionary}}` behaves identically to `::file foo.md set='{ name: "Bob" }'`.

## Open Questions / Follow-ups

- **Deep-merge of arrays**: the spec treats arrays as leaves (replaced wholesale). Confirm this matches the behavior of `darkmatter/docs/composition/frontmatter-in-pipelining.md`; if that document permits array concatenation in some context, the two need to be reconciled or the divergence explicitly noted.
- **Future extension for nested dotted keys**: `set.author.name="Bob"` is deferred to post-v1. A follow-up spec should decide the grammar (dot-chain vs bracket notation) and how it interacts with the object-form layer's deep-merge.
- **Precedence tie-break among multiple `set.NAME=`**: distinct names never overlap, but the spec should confirm whether option order on the directive line is semantically significant for anything other than duplicate-detection (current stance: order is not significant except for the duplicate-error rule).
- **CLI-surface consistency**: verify the `--set JSON5` switch on the Darkmatter CLI uses the same merge semantics as `::file ... set=` so that transclusion and CLI behavior are aligned.
- **Tombstone / delete semantics**: v1 treats `null` as a literal value (see `## Grammar` → Null Semantics). A future spec could introduce an explicit delete/tombstone syntax (e.g., a sentinel or a dedicated directive option) if a concrete use-case emerges for removing a key from the effective frontmatter rather than setting it to null.
