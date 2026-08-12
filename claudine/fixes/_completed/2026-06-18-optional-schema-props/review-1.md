---
ready: false
agent: codex
model: ""
created: "2026-06-19T07:05:27"
---

# Review 1 — Optional Schema Properties

## Findings

### High — Required unions still accept the optional `file` empty-string sentinel

The spec explicitly says required property-level unions must receive neither the property-level `null` arm nor optional-file empty-string tolerance. The implementation suppresses the union-level `null` arm when any arm is required, but it still builds each arm with `atom_fragment_without_null_wrap` before union-level requiredness is known. That helper applies the `file` empty-string arm whenever the file atom itself is not marked `required`.

Relevant code:

- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:142` calls `atom_fragment_without_null_wrap` per union arm.
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:146` only learns `arm_required` after the file arm may already have been wrapped.
- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:291` applies the empty-string sentinel based only on the atom-local `required` flag.

I confirmed this with a black-box CLI check:

```yaml
---
$schema:
  asset:
    - file
    - "number(required)"
asset: ""
---
# test
```

`md schema validate --format json` reports `valid: true` with no problems. Per the spec, this is a required union and `asset: ""` should fail.

Suggested fix: determine union requiredness first, then build union arms with a mode that disables both nullable wrapping and file empty-string tolerance when the union property is required. Add a regression test for a required union containing an otherwise optional `file` arm.

Verification level: Level 1 is appropriate because this is pure schema conversion/validation with no terminal-emulator behavior. Current Level 1 coverage misses this required-union/file combination.

### Low — The historical schema spec example places `design` outside frontmatter

The new example in `darkmatter/features/_completed/2026-05-11-schemas/spec.md` closes frontmatter before the `design` property:

```yaml
$schema:
    design: string
---
# If design.md does not exist, `design` resolves to null and the document is valid.
design: "{{ file_exists('design.md') ? 'design.md' : null }}"
```

That example does not actually declare a frontmatter `design` value. Move `design:` above the closing `---` and put the explanatory comment either above it as a YAML comment or below the frontmatter as prose.

Verification level: documentation-only, no L2/L3 requirement.

## Test Coverage Review

The core user-facing requirement here is non-terminal behavior: schema conversion, frontmatter coercion, and Claudine prepare-time validation. Level 1 is the right verification level for all acceptance criteria in this spec. No Level 2 or Level 3 tests are required because there are no terminal rendering, keybinding, paste, IME, mouse, or OS keyboard-input requirements.

Observed Level 1 coverage is strong for:

- Optional primitives, objects, inline objects, arrays, property-level unions, and `file` accepting `null`.
- Required typed atoms rejecting `null`.
- Optional `file` retaining `""` support.
- Non-null coercion through nullable wrappers and null preservation.
- Claudine direct and inline composition when an optional string template resolves to `null`.

Remaining Level 1 gaps:

- Required property-level union containing a `file` arm must reject `""`.
- The converter tests do not directly exercise required `enum`, `boolish`, and `numberlike` rejecting `null`; this is lower risk because the required-wrapper path is shared, but those types are named in the acceptance matrix.

## Commands Run

- `cargo nextest run -p darkmatter --lib -E 'test(optional_file_accepts_null_and_empty_as_absent) or test(optional_union_property_is_single_level_nullable_any_of) or test(optional_property_level_union_runs_per_arm_coercion)'`
- `cargo run -q -p darkmatter-cli -- schema validate --format json <temp-required-union.md>`
- `cargo nextest run -p claudine --lib -E 'test(optional_string_resolved_to_null_passes_direct) or test(optional_string_resolved_to_null_passes_inline) or test(required_string_resolved_to_null_fails_schema_validation)'`

## Production Readiness

Not ready. The primary optional-null behavior works in the tested paths, but the required-union/file sentinel case violates the spec and permits an empty required value.
