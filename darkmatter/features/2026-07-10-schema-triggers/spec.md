# Schema Triggers

In this feature we're formalizing how certain schemas get activated and merged on top of the base schema for Darkmatter (@darkmatter/docs/schemas/darkmatter.yaml).

The idea behind schema triggers is that the base Darkmatter schema should cover frontmatter definitions that are broadly reusable. However, there are often subsets of schematics that should be layered on top when the right conditions are met. The motivating case is Claudine: documents composed *through* Claudine (prompts, inline-compose documents, sequences) should get the Claudine schema layered on top of the Darkmatter base, while plain Darkmatter users are never touched by it.

Location is **not** a reliable activation signal for these documents — prompts tend to live in `prompts/` or `.claudine/prompts/` but do not need to, and inline-compose documents have essentially no locational identity. Frontmatter shape is the determinant, so activation is **content-triggered**.

## Surfaces

| Surface | Behavior |
|---------|----------|
| Compose through a host tool (e.g. Claudine) | Unaffected. The host already injects its baseline programmatically via `ComposeOptions::with_baseline_schema(...)`; the host is the trigger. |
| DMLS | Evaluates registered trigger schemas against each document's frontmatter and layers matching payloads into the effective schema. |
| `md` CLI (compose / schema validate) | Same discovery and evaluation as DMLS, so the CLI and LSP always agree on the effective schema. |

Trigger evaluation is implemented **once**, in the library's effective-schema assembly (`schemas::resolve`) — DMLS, `md schema validate`, and the compose validation stage all inherit it from that single path, exactly as they already share `DarkmatterSchemas::validate`.

- **`md schema validate` honors triggers by default.** This does not weaken its explicit-baseline contract (`--schema` / `BASELINE_SCHEMA`, no default Darkmatter baseline): triggers are repo/document-*declared* schemas, not tool defaults — placing the file in `schemas/` is the repo author's opt-in, exactly as writing `$schema` is the document author's, and validate honors document `$schema` today without any flag. The tool-default/explicit distinction continues to apply to baselines only.
- **`md schema detect`** is unaffected (value inference; no schema resolution).
- **`md schema about`** must document the trigger grammar (envelope, `$path`, combinators, arms, vacuous lint) via the typed descriptor catalog, with the existing parity tests keeping the catalog in lock-step with the implementation.

## Schema Directories

Trigger schemas **must** live in a `schemas/` directory discovered by an **ancestor walk**: starting at the document's directory, walk toward the repo root; every `schemas/` directory on that path is a schema root, nearest first. There are no user-home schema roots — resolution is repo-contained, so a checked-out repo behaves identically on every machine.

- The walk anchor is the **document being validated** for `$schema` references and trigger activation, and the **referencing schema file** for `Name@file` imports — the same anchors existing relative-path resolution uses.
- Nearest root wins **by filename** with no merging: `{package}/schemas/claudine.yaml` fully shadows `{repo-root}/schemas/claudine.yaml`.
- The directory name is exactly `schemas` (no `.schemas` variant). The walk-up model subsumes monorepo levels (package → package area → repo root) without Darkmatter needing to define what a "package" is.

### Ruling log

> **Ratified (2026-07-10):** ancestor walk replaces a fixed level list; home-directory roots removed entirely; single directory name `schemas` (the `.schemas` variant was considered for user preference and dropped to avoid a permanent same-level ambiguity rule).
>
> **Rejected:** `.claudine/schemas` as a well-known path — Darkmatter and DMLS must stay Claudine-neutral (Claudine depends on Darkmatter, never the reverse; DMLS Phase 7 ships zero Claudine-specific code). Claudine scaffolds its schema files into the neutral `schemas/` roots.
>
> **Ratified (2026-07-10, match grammar):** `$path` glob predicate added (dollar-reserved spelling); `match:` sequence form = OR'd arms (supersedes bare-list-means-`all`).
>
> **Rejected (match grammar):** body-content matching (breaks the pure frontmatter+path evaluation model — per-keystroke body scanning is a different cost class in DMLS, and content heuristics are the fragile matching the typed grammar exists to avoid); environment/repo predicates such as `exists: .claudine/` (redundant — with home roots removed, a trigger's presence in the repo's `schemas/` directory *is* the repo-level opt-in); weighted/scored matching (fuzzy activation stops being explainable and muddies the DMLS hysteresis story — `min-match` is the ceiling for graded confidence).

## Bare-Name Schema References

A schema reference with **no path component** (e.g. `$schema: claudine.yaml`, `Name@claudine.yaml`) resolves against the schema roots, nearest first. References with a path separator, `./` prefix, or magic-path prefix are untouched and resolve exactly as today. Bare-name resolution is one more rung on the existing `FileReference`/schema-resolution ladder — `$schema:`, `Name@file` imports, and `example(...)` references all share it; no parallel resolver.

This is a **behavior change**: today a bare name resolves relative to the document's parent directory. Ruling: acceptable, because existing documents overwhelmingly use `./`-prefixed relative paths. Soften the break with a pointed error: when a bare name is not found in any schema root but a document-sibling file of that name exists, the error must suggest `./‹name›`. Before landing, grep the corpus for path-less schema references.

## Defining a Trigger Schema

A trigger schema is a standalone YAML file claiming the `kind: trigger-schema` envelope (the same envelope-claiming pattern as `kind: schema`):

```yaml
kind: trigger-schema
match:
    all:
        - prompt: string(required)            # gate: must be present and a string
        - sections: object                    # guard: absent OR object
        - any:                                # at least one lifecycle key present
            - initialize: object(required)
            - start: object(required)
            - success: object(required)
            - failure: object(required)
        - none:                               # carve-out: never fire on these
            - kind: enum(schema; required)
$schema: claudine.yaml
```

The `$schema` value is the **payload** — the schema layered onto matching documents. It accepts the same value forms as a document `$schema`: an inline SimplifiedSchema mapping, or a reference (bare name or relative path).

### Match Grammar

There is no new predicate language for frontmatter. Each property condition is `property: <type-expr>` where the type expression reuses the existing SimplifiedSchema grammar, evaluated against the document's parsed frontmatter instead of compiled to a validator:

- **Guard** — a bare type expression (`sections: object`): the key may be absent; if present it must conform. A type contradiction (`prompt: 42` against `prompt: string`) defeats the match even when the key is not required — false-positive activation on look-alike documents is designed away.
- **Gate** — `required` in the constraint list (`prompt: string(required)`): the key must be present *and* conform.
- **Value matching** — `enum(...)` members and `pattern(...)` express literal-value and string-shape predicates with the existing grammar, so an explicit dialect marker (`kind: prompt`) and structural inference (lifecycle keys present) are the same mechanism.
- **Nested shape matching** falls out of the grammar for free: inline object literals work as guards (`sections: "{ intro: string }"` matches documents whose `sections.intro`, if present, is a string). No dotted-path syntax is needed.

One non-property predicate exists, and its spelling is **dollar-reserved** (mirroring `$schema` / `$constraints`) so it can never collide with a frontmatter property name:

- **`$path:`** — a glob (or list of globs, any-of, with `!` negations — the same grammar as the `file` type's `match()` constraint) evaluated against the document's repo-root-relative path. A bare basename glob matches in any directory (`SKILL.md` ≡ `**/SKILL.md`, gitignore-style). Reuses the globset machinery already in DMLS workspace discovery. A `$path` condition is inherently a gate — there is no "absent" case for a document's path. Any future non-property predicate must also be dollar-prefixed.

Combinators form a small boolean tree, mirroring JSON Schema's `allOf`/`anyOf`/`not`:

- `all:` — every child condition holds.
- `any:` — at least one child holds.
- `none:` — no child holds.
- `min-match:` — N-of-M (`{ count: 2, of: [...] }`); expressible via nested `any`/`all` but first-class to avoid combinatorial explosion. Echoes the existing `min-keys(n)` vocabulary.

Combinators nest freely. Match evaluation is a **pure function** of parsed frontmatter plus the document path plus the trigger expression — no I/O, no validator compilation — so it is viable per-keystroke in DMLS and composes with the content-hash effective-schema cache.

### Match Arms (Outer OR)

The `match:` value takes two forms, mirroring the root-level `$schema` union convention (mapping = one schema, sequence = `anyOf` of arms):

- **Mapping** — a single match expression. A mapping whose keys are all combinators (or `$`-predicates) is a combinator node (multiple sibling keys are AND'd). A mapping with **no** combinator keys is an implicit `all` of its property conditions — `match: { prompt: string(required), sections: object }` is the terse single-arm form.
- **Sequence** — a list of arm mappings, logically **OR'd**: the trigger matches when any arm matches. This supersedes the earlier bare-list-means-`all` rule from the first draft.

```yaml
match:
    - kind: enum(prompt; required)          # arm 1: explicit marker
    - all:                                  # arm 2: structural inference
        - prompt: string(required)
        - any:
            - initialize: object(required)
            - success: object(required)
    - $path: "prompts/**/*.md"              # arm 3: locational
```

Rules:

- Mixing combinator keys and property-condition keys in one mapping is a **load error**, never a guess.
- **Known v1 limitation:** a frontmatter property literally named `all`, `any`, `none`, or `min-match` cannot be condition-matched (the combinator interpretation wins). Documented, not escaped; future non-property predicates stay dollar-prefixed so this reserved set never grows.

### Vacuous-Trigger Lint

If no satisfiable path through an arm's boolean tree contains a presence-requiring condition (a `required` gate or a `$path` predicate inside `all`/`any`, not under `none`), that arm matches every document — and because arms OR together, one vacuous arm makes the whole trigger vacuous. The lint is applied **per arm**, is statically checkable, and is a **load-time error**, consistent with the fail-loud posture of `example(...)` validation.

## Discovery and Errors

- During discovery, a file in a schema root that does not claim `kind: trigger-schema` is **silently ignored** (it is a normal schema or unrelated YAML). Generic `schemas/` directories in existing repos therefore never cause noise.
- A file that **does** claim `kind: trigger-schema` but is malformed (bad envelope, bad match grammar, unresolvable payload, vacuous trigger) is a **hard load error**. Ruling: loud errors, no silent skip, for files that opted in.
- Directly referencing a trigger-schema file as a document schema (`$schema: ./schemas/claudine.yaml` where that file is a trigger envelope) is an **error** — trigger schemas activate by placement and match, never by reference. The error should name the payload as the thing to reference instead.

## Precedence and Merging

Effective schema assembly order:

1. Darkmatter base schema.
2. Matching trigger-schema payloads — nearest schema root first, filename-lexicographic within a root (boring and deterministic).
3. Document `$schema` — always wins on conflict (existing merge rule).

Across roots, nearest-wins-by-filename shadowing applies before matching (a shadowed trigger file is never evaluated). There is no cross-file schema merging beyond the standard baseline-merge semantics applied per layer.

## DMLS Integration

- **Hysteresis via last-good frontmatter.** Trigger evaluation rides the per-document last-good frontmatter tree (`OverlayState`) so mid-keystroke YAML breakage does not flap activation: activate as soon as a settled parse matches; deactivate only when a *clean* parse affirmatively does not match.
- **Watched directories.** Schema roots join the `didChangeWatchedFiles` registration (with the save-driven rescan fallback for watcher-less clients); trigger files become dependency edges like `uses_schema`, so editing `claudine.yaml` re-validates every document it is active on. New edge source, existing invalidation machinery.
- Post-activation `missing required` diagnostics are a feature (authoring help for a document that is recognizably a Claudine prompt), which is exactly why activation must be high-precision.

## Dialect Family

The expected shape is not one Claudine trigger but a small family — `claudine.yaml`, `inline-compose.yaml`, `sequence.yaml` — each with gates for its own dialect and `none:` carve-outs keeping them disjoint. (`prompt: string(required)` gates an inline-compose document but not a compose or sequence document.) The grammar and directory convention accommodate this with no additions.

## Open Questions

1. **Per-document override.** A frontmatter marker that forces a named trigger schema on or off for one document (escape hatch for false positives/negatives without editing workspace state). Proposed, not ratified.
2. **Near-miss diagnostic.** An info-level "document nearly matches trigger-schema X; `prompt` has the wrong type" DMLS diagnostic when a guard defeats an otherwise-matching document. v2 nicety; the typed grammar preserves the information needed.
3. **CLI opt-out.** Whether `md` needs a `--no-trigger-schemas` flag (parallel to `--no-baseline-schema`) for raw behavior. Default-on trigger participation in `md schema validate` strengthens the case; the flag should apply to both `validate` and `compose`.
4. **`min-match` spelling.** `min-match: { count: N, of: [...] }` vs. a postfix-style form; pick during implementation planning.
5. **Activation inspection surface.** Implicit activation needs a first-class "why (not)?" answer — e.g. `md schema triggers <file>` listing discovered trigger schemas and, per trigger, the matching arm or the condition that defeated each non-matching arm. Pure match evaluation makes this cheap; DMLS could later surface the same data (hover/code-lens). Rated near-essential for debuggability, pending ratification.
