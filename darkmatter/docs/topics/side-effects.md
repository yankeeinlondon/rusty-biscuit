# Side Effects

> **Status: planned (not yet implemented).** This page summarizes the agreed
> design. The authoritative, full specification lives in
> [`more-context-variables`](../../features/_unscheduled/more-context-variables/spec.md#side-effects).
> Until the feature lands, treat the linked spec as the source of truth and do
> not rely on any API described here existing yet.

Side Effects are the counterpart to the read-only [Expression Engine](./darkmatter-expressions.md).
Where expression functions only *report* on state, side effects **mutate**
external state — but only through a small, deliberately safe catalog governed
by hard boundaries.

## Relationship to the Expression Engine

| | Expression functions | Side effects |
|---|---|---|
| Purpose | read / report | mutate |
| Runs during `md compose`? | yes | **never** |
| Invoked by | the compose pipeline | an external orchestrator only |
| Module | `compose::expression` | `effects` (separate) |

Composing a document is **pure** — it never triggers a side effect. The
side-effect engine is a library surface that only a host invokes at
well-defined moments — for example, Claudine's
[lifecycle events](../../../claudine/features/2026-05-12-lifecycle/spec.md).
Darkmatter owns the engine; the host drives it.

## The Read/Write Seam

A side-effect call such as `set_frontmatter(@spec.md, "status", upper(env.AGENT))`
reuses the **same lexer/parser** as expression functions. The split is enforced
at evaluation, not syntax:

- side-effect **arguments** are evaluated by the read-only expression evaluator
  (so `upper(...)`, `frontmatter(...)`, and `{{ }}` interpolation work inside
  arguments)
- only the outer **dispatch** mutates

## Safety Boundaries

Both enforced inside Darkmatter, both configured by the host:

1. **Mutation root** — filesystem writes outside the configured root are
   hard-refused (`EffectError::OutsideMutationRoot`).
2. **Host allowlist** — network effects are refused unless the target host is
   allowed; the allowlist defaults to **deny-all** (`EffectError::HostNotAllowed`).

## v1 Catalog (summary)

- **Frontmatter:** `set_frontmatter`, `merge_frontmatter`, `delete_frontmatter`,
  `increment_frontmatter` / `decrement_frontmatter`,
  `append_frontmatter` / `prepend_frontmatter`
- **File & directory:** `ensure_file`, `ensure_dir`, `append_line`, `append_jsonl`
- **Network:** `http_post` (host-allowlist gated)

Markdown frontmatter writers re-hash a `hash:` property automatically by
default. Full-file overwrite and deletion are expressly out of scope for v1.

See the [spec](../../features/_unscheduled/more-context-variables/spec.md#side-effects)
for full signatures, return values, and the host integration surface.

## See Also

- [Darkmatter Expressions](./darkmatter-expressions.md)
- [Context Variables](./context-variables.md)
</content>
</invoke>
