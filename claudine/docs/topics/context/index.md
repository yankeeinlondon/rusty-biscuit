# Context, Expressions, and Side Effects

Claudine's composition features (`compose`, `inline-compose`, `sequence`) are
thin wrappers around three Darkmatter subsystems. The `claudine context`
command is the window onto all three — it documents what a composed Markdown
document can *read*, *compute*, and (under an orchestrator) *change*.

| Subsystem | What it provides | CLI report |
|-----------|------------------|------------|
| **Context variables** | Runtime facts about the host, repo, and scope, exposed as `ctx.*` values | `claudine context` / `claudine context --values` |
| **Expression engine** | A small read-only language for interpolation `{{ … }}` and conditions `when="…"` | `claudine context --expressions` |
| **Side effects** | A catalog of *mutating* operations (frontmatter, files, HTTP) driven by an orchestrator | `claudine context --side-effects` |

Each descriptor catalog also implements the shared `Described` trait from
`darkmatter::catalog`, so every item is runtime-accessible: exact lookup via
`describe`, fuzzy nearest-match via `suggest`, and plain-text enrichment via
`describe_for_error`. These powers drive the CLI reports *and* the enriched
error surfaces described in the subsystem docs.

Read each in detail:

- **[Context Variables](context-variables.md)** — what they are, how they are
  captured, how to add one, and how the default / `--values` reports are built.
- **[Expression Engine](expression-engine.md)** — the language, the function
  catalog, the runtime registry, and how the `--expressions` report is built.
- **[Side Effects](side-effects.md)** — the effect engine, its safety model,
  how the lifecycle drives it, and how the `--side-effects` report is built.
- **[Drift Control](drift.md)** — the cross-cutting story: how the CLI reports
  stay in lockstep with the Darkmatter implementation, where drift can still
  creep in, and concrete next steps to close those gaps.

## The one idea behind all three reports

Every `claudine context` report renders from a **typed, compile-time descriptor
catalog** that Darkmatter exports as part of its public API. A descriptor
catalog is a plain `const` slice of structs — constructing or reading it does
**no I/O, no host probing, and no context capture**.

```
Darkmatter (library)                         Claudine CLI (claudine context)
────────────────────                         ───────────────────────────────
context::CONTEXT_VARIABLE_DESCRIPTORS  ─────▶ default report  (Property/Type/Description)
expression_function_descriptors() ──────────▶ --expressions   (Function/Description)
effects::EFFECT_DESCRIPTORS            ─────▶ --side-effects   (Capability/Description/Safety)
```

The CLI **imports the catalog and folds it into tables** — it never keeps its
own parallel list. That single decision is what keeps the reports honest:

1. **CLI ↔ catalog drift is structurally impossible** — the report *is* the
   catalog, grouped and styled. Add a descriptor in Darkmatter and it appears
   in the CLI on the next build, with no CLI edit.
2. **Catalog ↔ runtime drift is caught by a test** — each subsystem ships an
   in-crate parity test that fails if the catalog and the real runtime surface
   disagree (see [Drift Control](drift.md)).

What this design does *not* yet guarantee — purely descriptive prose that is
not anchored by a verified example — is the subject of the
[Drift Control](drift.md) doc.

## The CLI command at a glance

```sh
claudine context                 # every ctx.* variable, grouped, with its Type
claudine context --values        # same, but Type → live captured Value
claudine context --expressions   # the expression language + full function catalog
claudine context --side-effects  # the side-effect capability catalog
```

- The flags are mutually exclusive (clap `multiple = false`).
- **Only `--values` captures runtime context** (host/repo/git/OS/hardware
  discovery). The default, `--expressions`, and `--side-effects` reports are
  pure documentation — they touch no disk and construct no effect engine, a
  guarantee that is itself enforced by a test (see [Side Effects](side-effects.md#documentation-only-guarantee)).
- All four reports share a width contract: total rendered width never exceeds
  `min(terminal width, 140)` columns, and stays whole down to a 53-column floor.
  Source: `claudine/cli/src/commands/context_render.rs`
  (`MAX_REPORT_WIDTH`, `MIN_SUPPORTED_REPORT_WIDTH`).

## Source map

| Concern | Location |
|---------|----------|
| Context catalog + capture | `darkmatter/lib/src/markdown/compose/context/` |
| Expression engine | `darkmatter/lib/src/markdown/compose/expression/` |
| Effects engine + catalog | `darkmatter/lib/src/effects/` |
| CLI report rendering | `claudine/cli/src/commands/context.rs`, `context_render.rs` |

See also the sibling topic doc [Composition](../composition.md) for how these
subsystems are wired into `compose` / `inline-compose` / `sequence`.
