---
status: ready for planning and implementation
reviewed: true
review_iterations: 6
inputs:
  - ../../lib/src/markdown/schemas/resolve.rs
  - ../../lib/src/markdown/schemas/mod.rs
  - ../../cli/src/commands/schema/validate.rs
  - ../../docs/schemas/darkmatter.yaml
related:
  - ../_completed/2026-07-08-schema-plus/spec.md
  - ../_completed/2026-07-08-single-sourcing-schema/spec.md
---

# Schema Triggers

In this feature we're formalizing how certain schemas get activated and merged on top of the base schema for Darkmatter (@darkmatter/docs/schemas/darkmatter.yaml).

The idea behind schema triggers is that the base Darkmatter schema should cover frontmatter definitions that are broadly reusable. However, there are often subsets of schemas that should be layered on top when the right conditions are met. The motivating case is Claudine: documents composed *through* Claudine (prompts, inline-compose documents, sequences) should get the Claudine schema layered on top of the Darkmatter base, while plain Darkmatter users are never touched by it.

Location is **not** a reliable activation signal for these documents — prompts tend to live in `prompts/` or `.claudine/prompts/` but do not need to, and inline-compose documents have essentially no locational identity. Frontmatter shape is the determinant, so activation is **content-triggered**.

## Surfaces

| Surface | Behavior |
|---------|----------|
| Compose through a host tool (e.g. Claudine) | Trigger discovery is off unless the host supplies a source path and enables it. Claudine continues to inject its baseline programmatically via `ComposeOptions::with_baseline_schema(...)`; it must not receive the same schema a second time through implicit discovery. |
| DMLS | Evaluates registered trigger schemas against each document's frontmatter and layers matching payloads into the effective schema. |
| `md` CLI (compose / schema validate) | Same discovery, matching, and merge implementation as DMLS; only the discovery boundary is sourced differently (sniff-discovered repo root vs. LSP workspace folder — see [Schema Directories](#schema-directories)). For the same settled file and the same configured boundary, the CLI and LSP produce identical effective schemas (pinned by acceptance criterion 4). |

Trigger evaluation is implemented **once**, in the library's effective-schema assembly — DMLS, `md schema validate`, and the compose validation stage all consume the same discovery, parsing, matching, and merge APIs. `schemas::resolve` remains the authority for resolving an individual schema value; trigger discovery is a sibling module rather than adding filesystem policy to the value resolver.

The library API must make implicit filesystem discovery explicit in configuration. `DarkmatterSchemas::new()` remains deterministic and does not search the host filesystem. A caller enables triggers with a document path plus a discovery boundary (or supplies a prebuilt trigger registry). This preserves in-memory/embedded callers and prevents validation behavior from depending silently on the process CWD.

- **`md schema validate` honors triggers by default.** This does not weaken its explicit-baseline contract (`--schema` / `BASELINE_SCHEMA`, no default Darkmatter baseline): triggers are repo/document-*declared* schemas, not tool defaults — placing the file in `schemas/` is the repo author's opt-in, exactly as writing `$schema` is the document author's, and validate honors document `$schema` today without any flag. The tool-default/explicit distinction continues to apply to baselines only.
- **`--no-trigger-schemas` is the explicit raw-mode escape hatch** for both `md compose` and `md schema validate`. It disables discovery and bare-name schema-root lookup together; an explicitly path-qualified document `$schema` continues to resolve normally. There is no environment-variable alias in v1.
- **`md schema detect`** is unaffected (value inference; no schema resolution).
- **`md schema about`** must document the trigger grammar (envelope, `$path`, combinators, arms, vacuous lint) via the typed descriptor catalog, with the existing parity tests keeping the catalog in lock-step with the implementation.

> **Reader note — review correction:** the first draft described triggers as part of `schemas::resolve` and always active for host composition. That would make a library call unexpectedly scan disk and could apply Claudine's programmatic baseline twice. The revised design keeps schema-value resolution pure, exposes trigger discovery as explicit configuration, and lets the file-oriented `md` and DMLS adapters enable it by default.

## Schema Directories

Trigger schemas **must** live in a `schemas/` directory discovered by an **ancestor walk**: starting at the document's directory, walk through the configured discovery boundary, inclusive; every `schemas/` directory on that path is a schema root, nearest first. There are no user-home schema roots — resolution is project-contained, so a checked-out project behaves identically on every machine.

The boundary is the repository root discovered through `sniff` for the `md` CLI. DMLS uses the containing LSP workspace folder, but may narrow it to a `sniff`-discovered repository root when one exists below that folder. A document outside every DMLS workspace folder has trigger discovery disabled. A stdin/virtual document has no discoverable roots unless its caller provides a synthetic source path and boundary. The walk never continues to the filesystem root or home directory merely because repository discovery failed.

- The walk anchor is the **document being validated** for `$schema` references and trigger activation, and the **referencing schema file** for `Name@file` imports — the same anchors existing relative-path resolution uses.
- Nearest root wins **by filename** with no merging: `{package}/schemas/claudine.trigger.yaml` fully shadows `{repo-root}/schemas/claudine.trigger.yaml`.
- The directory name is exactly `schemas` (no `.schemas` variant). The walk-up model subsumes monorepo levels (package → package area → repo root) without Darkmatter needing to define what a "package" is.
- Discovery considers regular `.yaml` and `.yml` files only and does not follow directory or file symlinks. Within a root, ordering is by the UTF-8 filename bytes, not locale or host filesystem collation. Two names that collide after case-folding are a load error so a repository cannot activate differently on case-sensitive and case-insensitive filesystems.
- `$path` receives a lexical, boundary-relative path with `/` separators on macOS, Windows, and Linux. Matching is case-sensitive on every platform; `..` never appears because documents outside the boundary are ineligible.

### Ruling log

> **Ratified (2026-07-10):** ancestor walk replaces a fixed level list; home-directory roots removed entirely; single directory name `schemas` (the `.schemas` variant was considered for user preference and dropped to avoid a permanent same-level ambiguity rule).
>
> **Rejected:** `.claudine/schemas` as a well-known path — Darkmatter and DMLS must stay Claudine-neutral (Claudine depends on Darkmatter, never the reverse; DMLS Phase 7 ships zero Claudine-specific code). Claudine scaffolds its schema files into the neutral `schemas/` roots.
>
> **Ratified (2026-07-10, match grammar):** `$path` glob predicate added (dollar-reserved spelling); `match:` sequence form = OR'd arms (supersedes bare-list-means-`all`).
>
> **Rejected (match grammar):** body-content matching (breaks the pure frontmatter+path evaluation model — per-keystroke body scanning is a different cost class in DMLS, and content heuristics are the fragile matching the typed grammar exists to avoid); environment/repo predicates such as `exists: .claudine/` (redundant — with home roots removed, a trigger's presence in the repo's `schemas/` directory *is* the repo-level opt-in); weighted/scored matching (fuzzy activation stops being explainable and muddies the DMLS hysteresis story — `min-match` is the ceiling for graded confidence).

## Bare-Name Schema References

A schema reference with **no path component** (e.g. `$schema: claudine.yaml`, `Name@claudine.yaml`) resolves against the schema roots, nearest first. References with a path separator, `./` prefix, or magic-path prefix are untouched and resolve exactly as today. Bare-name resolution is one more rung on the existing `FileReference`/schema-resolution ladder — `$schema:`, `Name@file` imports, and `example(...)` references all share it; no parallel resolver. The schema-root list is resolution context supplied by effective-schema assembly; `FileReference` remains the authority for turning the selected reference into a filesystem path.

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

Trigger envelopes and payloads are separate files. The example above therefore lives at a name such as `schemas/claudine.trigger.yaml`, while its payload lives at `schemas/claudine.yaml`. A trigger payload must not resolve to the trigger envelope itself, directly or through an import cycle; that is a hard load error. Shadowing keys are the trigger filenames (`claudine.trigger.yaml`), not payload filenames.

Although resolution accepts the document `$schema` value forms, a trigger payload must resolve to a merge-compatible object schema: a SimplifiedSchema single object, or a raw JSON Schema satisfying the existing simple-object baseline contract. Root unions and non-object schemas are rejected as trigger payloads because the established property-keyed merge has no sound "later wins" meaning for them. Document `$schema` retains all existing forms.

> **Reader note — review correction:** the first draft named both the trigger envelope and its payload `claudine.yaml`, which makes the bare payload reference resolve back to the envelope. The `.trigger.yaml` convention separates activation metadata from the reusable schema and makes direct payload references unambiguous.

### Match Grammar

There is no new predicate language for frontmatter. Each property condition is `property: <type-expr>` where the type expression reuses the existing SimplifiedSchema grammar, evaluated against the document's parsed frontmatter instead of compiled to a validator:

- **Guard** — a bare type expression (`sections: object`): the key may be absent; if present it must conform. A type contradiction (`prompt: 42` against `prompt: string`) defeats the match even when the key is not required — false-positive activation on look-alike documents is designed away.
- **Gate** — `required` in the constraint list (`prompt: string(required)`): the key must be present *and* conform.
- **Value matching** — `enum(...)` members and `pattern(...)` express literal-value and string-shape predicates with the existing grammar, so an explicit dialect marker (`kind: prompt`) and structural inference (lifecycle keys present) are the same mechanism.
- **Nested shape matching** falls out of the grammar for free: inline object literals work as guards (`sections: "{ intro: string }"` matches documents whose `sections.intro`, if present, is a string). No dotted-path syntax is needed.

One non-property predicate exists, and its spelling is **dollar-reserved** (mirroring `$schema` / `$constraints`) so it can never collide with a frontmatter property name:

- **`$path:`** — a glob (or list of globs, any-of, with `!` negations — the same grammar as the `file` type's `match()` constraint) evaluated against the document's discovery-boundary-relative path. A bare basename glob matches in any directory (`SKILL.md` ≡ `**/SKILL.md`, gitignore-style). Reuses the globset machinery already in DMLS workspace discovery. A `$path` condition is inherently a gate — there is no "absent" case for a document's path. Any future non-property predicate must also be dollar-prefixed.

Combinators form a small boolean tree, mirroring JSON Schema's `allOf`/`anyOf`/`not`:

- `all:` — every child condition holds.
- `any:` — at least one child holds.
- `none:` — no child holds.
- `min-match:` — N-of-M (`{ count: 2, of: [...] }`); expressible via nested `any`/`all` but first-class to avoid combinatorial explosion. Echoes the existing `min-keys(n)` vocabulary.

Combinators nest freely. Match evaluation is a **pure function** of parsed frontmatter plus the normalized document path plus the trigger expression — no I/O, no validator compilation — so it is viable per-keystroke in DMLS and composes with the content-hash effective-schema cache.

"Reuses the existing grammar" means the type-expression parser and value semantics are shared, not that every schema-authoring constraint is legal in a match. Match loading rejects constraints that resolve files, import types, load examples, provide defaults, mark generated values, or otherwise transform/consult state outside the candidate value. `file` may be tested only as a string-shaped value; eager existence checks are forbidden. Structural types and pure constraints such as `required`, `enum`, `pattern`, length/range, item-count, and key-count constraints are allowed. The typed descriptor catalog must identify the match-safe subset so `md schema about` cannot imply broader support.

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
- Directly referencing a trigger-schema file as a document schema (`$schema: ./schemas/claudine.trigger.yaml`) is an **error** — trigger schemas activate by placement and match, never by reference. The error should name the payload (`./schemas/claudine.yaml`) as the thing to reference instead.
- Discovery is transactional per boundary: if any unshadowed opted-in trigger is invalid, no trigger registry from that scan is installed. DMLS retains its last-good registry, publishes a schema-load diagnostic on the trigger file, and does not flap documents to a partially loaded schema set. The CLI reports a schema-load error and exits through its existing schema-error path.

## Precedence and Merging

Effective schema assembly order:

1. The caller-configured baseline, if any. For `md compose` this is the Darkmatter base unless disabled; for `md schema validate` it remains only `--schema` / `BASELINE_SCHEMA`; for a host it is the programmatically supplied baseline.
2. Matching trigger-schema payloads — nearest schema root first, filename-lexicographic within a root (boring and deterministic).
3. Document `$schema` — always wins on conflict (existing merge rule).

Across roots, nearest-wins-by-filename shadowing applies before matching (a shadowed trigger file is never evaluated). There is no cross-file schema merging beyond the standard baseline-merge semantics applied per layer.

Later layers win per top-level property using the existing merge contract. A trigger payload may intentionally replace an earlier trigger's complete property definition; nested fragments from two definitions of the same property are not combined. Effective-schema origins and dependencies must retain every contributing trigger envelope, payload, payload import, and example so diagnostics and cache invalidation identify the actual source set.

Trigger matching uses the parsed frontmatter snapshot presented to schema validation. For `md compose`, this is after `--set` / `--state` and non-executing frontmatter interpolation pass 1, but before frontmatter shell execution, matching the established pre-operation validation boundary. A value still pending `$(...)` cannot satisfy a value/type gate; key presence alone may satisfy only a presence test. After shell expansion and interpolation pass 2, effective-schema assembly runs again before downstream re-validation, so a newly concrete value may activate or deactivate a trigger.

`md schema validate` assignments require a two-pass flow: resolve the pre-assignment effective schema for assignment coercion, apply assignments, then discard that result and resolve triggers again for validation. This corrects the current assumption in `validate_one` that effective schema is unaffected by assignments. DMLS evaluates the last-good parsed source and performs no interpolation or execution; for the same settled on-disk source without compose-time overrides or shell values, its activation result is byte-for-byte the CLI result.

## DMLS Integration

- **Hysteresis via last-good frontmatter.** Trigger evaluation rides the per-document last-good frontmatter tree (`OverlayState`) so mid-keystroke YAML breakage does not flap activation: activate as soon as a settled parse matches; deactivate only when a *clean* parse affirmatively does not match.
- **Watched directories.** Schema roots join the `didChangeWatchedFiles` registration (with the save-driven rescan fallback for watcher-less clients); trigger files become dependency edges like `uses_schema`, so editing `claudine.yaml` re-validates every document it is active on. New edge source, existing invalidation machinery.
- Changes to an envelope can alter the match set, so an envelope is a boundary-level dependency even for documents it does not currently match. Adding, deleting, renaming, or editing any candidate trigger file rebuilds that boundary's registry and re-evaluates all indexed documents under the boundary. Payload/import/example changes may use the narrower reverse-dependency fan-out.
- Post-activation `missing required` diagnostics are a feature (authoring help for a document that is recognizably a Claudine prompt), which is exactly why activation must be high-precision.

## Dialect Family

The expected shape is not one Claudine trigger but a small family — `claudine.trigger.yaml`, `inline-compose.trigger.yaml`, `sequence.trigger.yaml` — each pointing to a separately named payload and carrying gates for its own dialect plus `none:` carve-outs keeping them disjoint. (`prompt: string(required)` gates an inline-compose document but not a compose or sequence document.) The grammar and directory convention accommodate this with no additions.

## Acceptance Criteria

- [x] The library matcher is side-effect-free and has table-driven tests for every combinator, outer OR arms, nesting, path normalization, forbidden stateful constraints, and vacuous-arm rejection.
- [x] Discovery tests cover nested roots, inclusive boundaries, filename shadowing, `.yaml`/`.yml`, symlink exclusion, case-fold collisions, malformed-trigger atomicity, and no discovery for pathless/out-of-boundary documents on macOS, Windows-compatible paths, and Linux-compatible paths.
- [x] A `.trigger.yaml` envelope and separate payload resolve without self-reference; direct and indirect envelope/payload cycles fail with source paths in the error.
- [x] `md compose`, `md schema validate`, and DMLS produce identical trigger sets for the same settled file and configured boundary. Tests also pin assignment re-resolution, pending shell values, host opt-in, and `--no-trigger-schemas` behavior.
- [x] Merge tests pin configured-baseline → ordered triggers → document-schema precedence, shadowing before matching, rejection of non-mergeable payloads, origin attribution, and complete dependency collection.
- [x] Trigger create/change/delete invalidates every document under its boundary; payload/import/example changes invalidate all and only effective schemas that depend on them. DMLS retains a last-good registry after a malformed edit.
- [x] Corpus migration confirms path-less schema references are either intentionally schema-root-resolved or changed to `./`; diagnostics suggest `./name` for a sibling-only legacy reference.
- [x] `md schema about` is generated from the typed catalog and documents the envelope, match-safe constraint subset, path semantics, combinators, arms, and vacuous lint.

Phase 8 acceptance sweep found no deviations from these criteria.

## Open Questions

1. **Per-document override.** A frontmatter marker that forces a named trigger schema on or off for one document (escape hatch for false positives/negatives without editing workspace state). Proposed, not ratified.

   - `$triggers: { enable: [...], disable: [...] }` is explicit and travels with the document, but reserves user frontmatter and must itself be excluded from ordinary unknown-key diagnostics.
   - A DMLS/CLI configuration override avoids changing documents, but creates machine/workspace-specific behavior and cannot make a document portable on its own.
   - A sentinel match property keeps the core grammar unchanged, but forces every trigger author to anticipate and standardize escape hatches.

   **Recommendation:** reserve `$triggers` for a later version if real false positives appear. It is the only option that is document-local and tool-neutral, but v1 should not add control-plane frontmatter without demonstrated need.
2. **Near-miss diagnostic.** An info-level "document nearly matches trigger-schema X; `prompt` has the wrong type" DMLS diagnostic when a guard defeats an otherwise-matching document. v2 nicety; the typed grammar preserves the information needed.
3. **Activation inspection UI beyond the CLI.** The v1 CLI includes `md schema triggers <file>` because implicit activation otherwise lacks a first-class "why (not)?" answer. It lists roots, shadowed envelopes, matching triggers and arms, and the first defeating condition for each non-matching arm. Whether DMLS exposes the same trace as hover, code lens, or a custom request remains open.

   - Hover is discoverable at the relevant frontmatter but has no obvious cursor target when a trigger does not match.
   - Code lens is visible and document-scoped but adds persistent editor noise.
   - A custom request is complete and UI-neutral but requires editor integration.

   **Recommendation:** ship the shared structured trace and CLI command in v1, then add a DMLS custom request when an editor client consumes it; do not choose a visual editor surface speculatively.
