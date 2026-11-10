# Frontmatter

Frontmatter is the YAML block between the `---` fences at the top of a
document. Darkmatter parses it into ordered JSON-like values that composition,
schemas, expressions, and the `md get` / `md set` / `md rm` commands all
share.

```markdown
---
title: Release notes
owner:
  name: Kim
  email: kim@example.com
tags: [cli, docs]
---
# Release notes
```

This page covers how frontmatter is parsed, the key order you can rely on,
how nested edits treat the rest of the file, and the shallow merges the library
offers. How a parent's frontmatter reaches
a transcluded child is covered in
[Frontmatter and Recursion](./frontmatter-recursion.md).

## Parsing

- The block must be valid YAML whose root is a mapping.
- A key may appear only once in any mapping, at any depth. A duplicate key is
  a parse error, not "last one wins":

  ```yaml
  title: First
  title: Second   # error: duplicate entry with key "title"
  ```

- Values become strings, numbers, booleans, `null`, lists, and mappings. A
  [schema](./schemas/definition.md) can then check and convert them.

## Key Order

**Key order is the order you wrote**, at every depth, including mappings
inside lists. Darkmatter never sorts your keys. That order is what you see
when you:

- read a value with `md get` (`md get doc.md owner` prints `name` before
  `email` for the example above),
- render a mapping in a composed document, for example with `{{ fm }}` or
  `{{ owner }}`,
- edit with [`md set`](../cli/set.md) or [`md rm`](../cli/rm.md), and
- iterate a mapping with `keys()` / `values()` in an expression.

Edits follow these rules, at the top level and inside a nested mapping alike:

| Edit | Result |
| --- | --- |
| Replace an existing key | the key stays where it was |
| Add a new key | it is appended after the existing keys |
| Remove a key | the keys after it close the gap, in order |

```bash
md set doc.md title "Notes" --save   # title stays first
md set doc.md draft true --save      # draft becomes the last key
md rm doc.md owner                   # tags moves up to second place
md set doc.md owner.phone 555 --save # phone becomes owner's last key
```

Values produced by a program, such as an expression result or the external
state a host passes to composition, keep the order their producer built them
in.

Key order never changes what a document means. The semantic (non-strict)
hash from [`md hash`](../cli/hash.md#key-order-and-stored-baselines) ignores
it at every depth, while the strict hash follows it.

## Editing Nested Values

`md set` and `md rm` name a value with a [property path](../cli/index.md#property-paths)
and change that value's lines, leaving every other byte of the file alone. Use
this section to predict what happens to your comments and formatting.

```yaml
# before                         # after: md set doc.md owner.email k@x.io --save
owner:                           owner:
  # the on-call contact            # the on-call contact
  name: Kim  # primary             name: Kim  # primary
  email: kim@example.com           email: k@x.io
```

| Edit | What happens to the text |
| --- | --- |
| Replace a value | its lines are rewritten in place, in the standard YAML style; a comment on its key's line stays, and comments inside a replaced mapping or list go with it |
| Add a key | a new line at the end of its parent, at the parent's indentation; missing parents are created |
| Remove a value | its own lines go; a comment on the line above stays, because it is not part of the value |
| Remove the last child | the parent stays, as `{}` or `[]`, and a later `md set` can add to an empty `{}` in place |

Every edit is checked before anything is written. The edited text is parsed
again and every value compared with the intended result, and a failed edit
leaves the file exactly as it was.

```mermaid
flowchart TD
    A["edit the text in place"] --> B{"could the source<br/>form be edited?"}
    B -->|"no: indented root,<br/>inside a non-empty flow collection"| F["rewrite the whole block<br/>in the standard style"]
    B -->|yes| V{"parse the result:<br/>did only the named values change?"}
    F --> V
    V -->|yes| W["write, or print"]
    V -->|no| X["refuse with exit status 1;<br/>nothing is written"]
```

**The fallback.** Some forms cannot be edited safely line by line: an indented
root mapping, a path inside a non-empty flow collection (`tags: [a, b]`), or a
multi-line scalar whose end cannot be found. Darkmatter then rewrites the whole
frontmatter block in its standard style. The values are the same but the
block's comments and formatting are not, and the body is untouched. Naming a
flow collection whole (`md set doc.md tags '["x"]'`) still edits in place, and
`md clean --save` removes an indented root.

**The refusal.** A YAML alias reads a value from elsewhere, so an edit can reach
further than the path you named. Such an edit is refused, with no fallback:

```yaml
base: &b
  x: 1
ref: *b
```

```text
$ md set doc.md base.x 2
Error: Nothing was written: the edit would change the value of `ref.x`, which it does not name (an alias there likely reads an anchor in the edited node)
```

Replace the alias with the value you want first. Removing an anchor an alias
still needs is refused the same way.

The library's `Markdown::fm_get` and `Markdown::fm_insert` keep working on
literal top-level keys. A caller that wants paths uses
`simplified_schema::PropertyPath`, described in
[Property Paths](../../../simplified-schema/docs/topics/property-paths.md),
with `darkmatter::markdown::hash::edited_document_text` for the text.

## Shallow Merges (Library)

The library's `Frontmatter` type merges another mapping into itself with
`merge_with`, choosing what happens when both sides hold the same key:

| Strategy | Conflict on an existing key |
| --- | --- |
| `PreferDocument` | the existing value is kept |
| `PreferExternal` | the incoming value replaces it |
| `ErrorOnConflict` | the merge fails |

`set_defaults` adds the keys a document is missing and keeps every existing
value, like `PreferDocument`.

These merges are **shallow**: they compare top-level keys only. Whichever
strategy is chosen, key order follows the same rules:

- existing keys keep their positions, even when their value is replaced;
- keys only the incoming mapping holds are appended in the incoming order;
- a nested mapping that is replaced is replaced whole, keeping the incoming
  mapping's own key order. Its keys are not merged one by one.

```yaml
# document              # incoming             # PreferExternal result
z: 1                    a: 3                   z: 1
a: 2                    b: 4                   a: 3
owner:                  owner:                 owner:
  name: Kim               email: k@x.io          email: k@x.io
                                               b: 4
```

With `PreferDocument` the result has the same order, `z, a, owner, b`, but
keeps `a: 2` and the document's `owner`.

```mermaid
flowchart TD
    start([merge_with document, incoming]) --> next{next incoming key}
    next -->|none left| done([done])
    next --> exists{key already in document?}
    exists -->|no| append[append key and value at the end]
    exists -->|yes| strategy{strategy}
    strategy -->|PreferDocument| keep[keep existing value]
    strategy -->|PreferExternal| replace[replace value in place]
    strategy -->|ErrorOnConflict| fail([error])
    append --> next
    keep --> next
    replace --> next
```

An `ErrorOnConflict` failure does not roll back: keys merged before the
conflicting key stay merged.

Composition does not use these shallow merges for inherited state; it merges
nested mappings key by key, as described in
[Frontmatter and Recursion](./frontmatter-recursion.md).
