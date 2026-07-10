---
status: draft
created: 2026-07-05
depends_on:
  - ../../../darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md
schema: ../../../darkmatter/docs/schemas/claudine.yaml
---

# Claudine Base Frontmatter Schema

## Context

Claudine composes Markdown through Darkmatter and then interprets additional
frontmatter for provider selection, lifecycle stacks, loops, sequences,
timeouts, runaway guards, linking, and prompt reporting. Darkmatter
frontmatter is also available during Claudine compose operations, so Claudine's
effective base schema must include both Darkmatter-owned and Claudine-owned
properties.

Claudine is mid-refactor on provider architecture. This feature should land
after that refactor so the schema/code generation path can align with the new
provider update model rather than stabilizing around soon-to-change provider
internals.

## Goals

1. Make `darkmatter/docs/schemas/claudine.yaml` the authored source of truth
   for Claudine-owned baseline frontmatter properties.
2. Keep `SimplifiedSchema` as the schema language. Do not introduce a separate
   metadata format in v1.
3. Use existing `SimplifiedSchema` description syntax to document properties
   directly in the schema.
4. Use Darkmatter's proposed nested object schema syntax so Claudine's lifecycle,
   loop, guard, and provider-facing objects can be documented without large
   quoted object-literal strings.
5. Generate or expose Claudine's effective base schema as:

   ```text
   darkmatter base schema + claudine base schema
   ```

6. Use the effective schema during Claudine compose, inline-compose, and
   sequence operations.
7. Add documentation that transcludes the Claudine schema YAML and explains
   that the runtime baseline also includes Darkmatter's base schema.
8. Introduce a deterministic code generation path appropriate for Claudine's
   post-refactor provider architecture.

## Non-Goals

1. Do not land this feature before the provider refactor it depends on.
2. Do not make Claudine frontmatter validation reject unknown user properties.
3. Do not fully encode every lifecycle action, provider, or guard nested shape
   in v1.
4. Do not replace existing Claudine runtime parsers. The schema gives baseline
   validation and documentation; detailed parsers remain authoritative.
5. Do not introduce long-form `SimplifiedSchema` entries in v1.

## Source Of Truth

The Claudine-owned schema source is:

```text
darkmatter/docs/schemas/claudine.yaml
```

The effective runtime schema is the merge of:

```text
darkmatter/docs/schemas/darkmatter.yaml
darkmatter/docs/schemas/claudine.yaml
```

Darkmatter properties are inherited because Claudine composition uses
Darkmatter composition. Claudine properties are merged on top. If a property is
declared by both schemas, the conflict should be explicit during generation or
tests; accidental shadowing should fail fast.

## Nested Object Syntax Requirement

This feature depends on the Darkmatter parser enhancement specified in
`darkmatter/features/2026-07-04-darkmatter-base-schema/spec.md`: a YAML mapping
used as a property value in `SimplifiedSchema` is an object schema definition.

Claudine needs this because its frontmatter contains several structured
surfaces that are awkward and error-prone as quoted inline object literals:

- lifecycle event objects
- `loop` configuration
- `guard_settings`
- longhand `exit_expressions`
- provider/linking metadata where a stable nested shape exists

The old inline object syntax remains valid:

```yaml
$schema:
  obj: "{ foo: string, bar: number, baz: boolean }"
```

The preferred form for new base schemas is:

```yaml
$schema:
  obj:
    foo: string
    bar: number
    baz: boolean
```

This is not the future long-form metadata grammar. In v1, nested mappings are
object schemas, while descriptions and defaults continue to live in ordinary
type-expression strings.

## Generated Constraint

This feature also depends on Darkmatter's `generated` constraint. Claudine has
runtime-provided values in late-binding lifecycle scenarios, especially the
`current` global. Those values should be visible to expression tooling and
runtime validation without implying that prompt authors must define them in
static frontmatter.

The Claudine base schema should use the same rule as Darkmatter `ctx`:

- generated values are available in the effective runtime schema
- generated values are not required from the static authored document
- generated values may still be marked `required` to describe the non-null
  effective runtime type when Claudine guarantees them in that scope
- editor tooling may surface them in expression contexts where they exist

The initial Claudine schema can defer a full `current` schema until the
late-binding lifecycle scope is modeled in the generated schema output, but the
spec requires the underlying `generated` constraint before that model is made
authoritative.

## Baseline Property Scope

The Claudine schema should include Claudine-owned frontmatter properties from
composition, lifecycle, linking, prompt reporting, system prompt preparation,
timeouts, runaway guards, sequence overlays, and loop overlays.

Initial scope includes:

- `prompt`
- `agent`, `model`, `interactive`
- `fail_fast`, `sequence`, `loop`
- lifecycle event keys: `initialize`, `start`, `blocked`, `success`,
  `failure`, `finalize`
- lifecycle communication/action fields through broad event object validation
- `operation`, `output`, `yolo`
- `timeout`, `step_timeout`, `timeout_warn`, `step_timeout_warn`,
  `stall_timeout`
- `exit_expressions`, `guard_settings`
- linking and portability fields: `name`, `allowed-tools`, `tools`, `skills`,
  `license`, `compatibility`, `user-invocable`,
  `disable-model-invocation`, `argument-hint`, `max_turns`, `maxTurns`
- prompt reporting and system prompt fields: `verbosity`, `mode`
- sequence overlay fields: `state`, `previous_state`, `next_state`,
  `is_first`, `is_last`, `step`, `total_steps`
- loop overlay fields: `_loop_count`, `_loop_is_first`, `_loop_is_last`,
  `_loop_last_output`, `_loop_last_exit_code`

The implementation pass should compare this list against
`claudine/docs/topics/frontmatter-properties.md` and the current source before
closing the feature.

## Documentation Contract

Add a documentation file next to the schema:

```text
darkmatter/docs/schemas/claudine-schema.md
```

It should explain that Claudine adds its own baseline schema on top of
Darkmatter's base schema, then transclude the Claudine-authored schema:

```md
# Claudine Schema Defaults

Claudine composes Markdown through Darkmatter, so Claudine's effective baseline
schema is Darkmatter's base frontmatter schema plus Claudine's own schema
defaults.

::code ./claudine.yaml
```

The documentation should link to the Darkmatter schema defaults document and
make clear that the displayed Claudine YAML is the Claudine-owned fragment, not
the fully merged effective schema.

## Runtime Integration

Claudine compose preparation should inject the effective merged baseline schema
when composing prompts. This applies to:

- `compose`
- `inline-compose`
- `sequence`
- system-prompt discovery composition where no more specific baseline already
  applies

The existing system-prompt `mode` baseline should be reconciled with the new
base schema. Because `mode` can have context-specific meanings, the v1
Claudine base schema may keep `mode: string` while narrower code paths continue
to apply stricter validation where appropriate.

Document-level `$schema` declarations retain precedence over baseline
properties on conflict, following Darkmatter merge semantics.

## Code Generation

This feature introduces a significant code generation piece for Claudine, but
the generator should be deterministic and reviewable.

Recommended pipeline:

```text
darkmatter/docs/schemas/darkmatter.yaml
darkmatter/docs/schemas/claudine.yaml
  -> parse referenced SimplifiedSchema files
  -> validate each schema independently
  -> merge Darkmatter + Claudine fragments
  -> emit checked-in generated Rust artifact for Claudine runtime use
  -> optionally emit generated docs/completion data later
```

Generated files should be checked in. Ordinary builds should not silently
rewrite files. A `just` recipe should run the generator explicitly.

The generator should run after the provider refactor has stabilized enough that
provider-specific schema fragments, if any, have a clear ownership model.

## Provider Refactor Dependency

This feature lands after the provider refactor. The post-refactor architecture
should answer:

1. whether providers contribute schema fragments
2. where provider-specific frontmatter keys are registered
3. how generated provider docs are grouped
4. whether provider additions require schema regeneration

If providers do contribute fragments, the Claudine effective schema can become:

```text
darkmatter base + claudine base + provider fragments
```

That provider-fragment model is optional future scope unless the refactor makes
it cheap and obvious.

## Future: Long Form Schema Entries

If descriptions, examples, aliases, deprecations, or provider ownership
metadata outgrow the current short-form grammar, `SimplifiedSchema` may add a
long-form property entry later:

```yaml
$schema:
  agent:
    type:
      - string
      - string[]
    description: Preferred agentic CLI provider.
    examples:
      - claude
      - [claude, codex]
```

That is future work. V1 should use short-form `SimplifiedSchema` with
descriptions.

## Testing

Tests should verify:

1. `darkmatter/docs/schemas/claudine.yaml` parses as referenced
   `SimplifiedSchema`.
2. The Claudine schema converts to a baseline-compatible JSON Schema.
3. The generated effective schema includes Darkmatter and Claudine properties.
4. Known valid Claudine frontmatter examples pass validation.
5. Invalid known-property values fail validation.
6. Unknown user-defined frontmatter keys remain accepted.
7. Document `$schema` definitions override baseline properties on conflict.
8. Existing system-prompt `mode` behavior remains correct.
9. Sequence and loop overlay keys validate when injected by Claudine.
10. Nested mapping syntax validates lifecycle, loop, guard, and longhand
    exit-expression object shapes.
11. Generated late-binding values are visible to effective-schema tooling
    without being required in authored frontmatter.

## Open Questions

1. Should the merged effective schema be exposed as a public Claudine library
   API or remain internal to composition preparation?
2. Should provider-specific frontmatter keys be included in the v1 Claudine
   base schema or deferred until after provider-fragment support exists?
3. Should the generated effective schema be written as YAML, Rust, or both?
4. Should Claudine docs display the fully merged effective schema in addition
   to the Claudine-owned fragment?
