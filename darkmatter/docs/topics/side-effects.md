# Side Effects

> **Status: v1 catalog implemented.** The frontmatter, file/directory, and
> host-policy-gated network verbs ship as `darkmatter::effects::EffectEngine`.

Side Effects are the counterpart to the read-only [Expression Engine](./darkmatter-expressions.md).
Where expression functions only *report* on state, side effects **mutate**
external state — but only through a small, deliberately safe catalog governed
by hard boundaries.

## Relationship to the Expression Engine

| | Expression functions | Side effects |
|---|---|---|
| Purpose | read / report | mutate |
| Runs during `md compose`? | yes | only from a [lifecycle event](./lifecycle/index.md) action |
| Invoked by | the compose pipeline | a lifecycle host (`md compose`, Claudine) |
| Module | `compose::expression` | `effects` (separate) |

Composing a document's body is **pure**: the body and its frontmatter values
never trigger a side effect. Side effects run only as actions of a
[lifecycle event](./lifecycle/index.md), which a host fires at well-defined
moments. `md compose` is such a host, so an `ensure_file` in `initialize` can
create the file the body then includes:

```yaml
initialize:
    - ensure_file: ["notes.md", "# Notes"]   # creates notes.md only if it is missing
```

Every other surface (`md render`, `md schema validate`, the library compose
API without a host run, DMLS) stays passive and runs no side effect.
Darkmatter owns the engine; the host drives it.

## The Read/Write Seam

A side-effect call such as `set_frontmatter(@spec.md, "status", upper(env.AGENT))`
reuses the **same lexer/parser** as expression functions. The split is enforced
at evaluation, not syntax:

- side-effect **arguments** are evaluated by the read-only expression evaluator
  (so `upper(...)`, `frontmatter(...)`, and `{{ }}` interpolation work inside
  arguments)
- the evaluated arguments then **bind** to the verb's typed signature, by the
  same rule as an expression-function call, before anything is written. A
  value that fits no signature stops the action with the expression layer's
  call-binding error, naming the argument; the engine is never called, and
  `no_error` does not hide it. The engine receives the bound values, so a
  number given for a text parameter arrives as its text, and the selected
  signature picks the `ensure_file` form:

  ```yaml
  composed:
      - ensure_file: ["count.txt", 3]                # writes the text 3
      - merge_frontmatter: ["doc.md", "{{ 5 }}"]     # fails before writing: obj must be an object
  ```

- only the outer **dispatch** mutates

## Safety Boundaries

Both enforced inside Darkmatter, both configured by the host:

1. **Mutation root** — filesystem writes outside the configured root are
   hard-refused (`EffectError::OutsideMutationRoot`).
2. **Host allowlist** — network effects are refused unless the target host is
   allowed; the allowlist defaults to **deny-all** (`EffectError::HostNotAllowed`).

## v1 Catalog (summary)

- **Frontmatter (shipped):** `set_frontmatter`, `merge_frontmatter`,
  `delete_frontmatter`, `increment_frontmatter` / `decrement_frontmatter`,
  `append_frontmatter` / `prepend_frontmatter`
- **File & directory (shipped):** `ensure_file` (with an
  `ensure_file_with_content` two-arg form), `ensure_dir`, `append_line`,
  `append_jsonl`. Whole-file writes replace the file atomically; the two
  append verbs write through an append-mode handle instead, so concurrent
  appenders (parallel sequence members, separate processes) never lose a line.
- **Network:** `http_post(url, body)` posts a body and returns an object with
  `status` and `body`. It is host-allowlist gated and deny-all by default.

Markdown frontmatter writers re-hash a `hash:` property automatically by
default. Full-file overwrite and deletion are expressly out of scope for v1.

`http_post` uses the same `biscuit-file` fetch policy as compose remote reads,
so denied hosts are rejected before any network request is attempted.

## See Also

- [Darkmatter Expressions](./darkmatter-expressions.md)
- [Context Variables](state-management/context-variables.md)
- [Lifecycle Events](./lifecycle/index.md)
