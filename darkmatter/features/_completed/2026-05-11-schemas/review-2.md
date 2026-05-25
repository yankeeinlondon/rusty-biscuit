---
ready: true
agent: codex
model: ""
---

# Review: Schemas in Darkmatter (Iteration #2)

## Summary

Most of the iteration-1 blockers have been addressed: root-union failures now compute a closest arm, JSON output carries `arm_index`, validator construction uses `PatternOptions::regex()`, and the validation fixture matrix is much broader. I verified the focused schema test targets:

- `cargo test --color=never -p darkmatter --test schemas_validate_table`
- `cargo test --color=never -p darkmatter --test schemas_detect_table`
- `cargo test --color=never -p darkmatter-cli --test schema_validate`
- `cargo test --color=never -p darkmatter-cli --test schema_detect`

All passed. The feature is still not ready for production because the remaining gaps are user-facing CLI/spec mismatches.

## Findings

### F1. Per-document schema resolution/build errors exit `1`, not the specified `2` [HIGH]

**File**: `darkmatter/cli/src/commands/schema/validate.rs:57-80`

The spec assigns exit code `2` to "Schema or baseline could not be loaded." The CLI handles baseline-load failures with `exit(2)` during `load_api`, but schema errors found while validating individual documents are folded into `any_failure` and then exit `1`.

I confirmed this with a document containing `$schema: ./missing.yaml`: the CLI printed a schema error block and exited `1`. That makes schema-load errors indistinguishable from ordinary validation failures for scripts and CI.

Expected fix: track `any_schema_error` separately and prefer exit `2` over validation failure `1` after all outcomes have been emitted. Add a CLI integration test for an unresolved document-level `$schema`.

### F2. Root-union pretty output does not prefix problems with the arm index [HIGH]

**File**: `darkmatter/cli/src/commands/schema/validate.rs:174-182`

The spec requires root-union failures to report the closest-matching arm and prefix problems with the arm index, e.g. `arm[1]: title is required`. The library now sets `ValidationProblem::arm_index`, and JSON output tests cover that, but pretty output ignores `problem.arm_index`.

Confirmed output for an invalid two-arm root union:

```text
root_union.md  ✗ 1 problem
  • <root> "title" is a required property
```

There is no `arm[0]` / `arm[1]` cue, so the default human-readable format still misses the user-facing requirement. This needs a pretty-format assertion, not only JSON coverage.

### F3. Pretty output treats the synthetic `<root>` path as Prose markup [MEDIUM]

**File**: `darkmatter/cli/src/commands/schema/validate.rs:176-181`

For root-level validation problems, `path_label` is the literal string `<root>`, but only `problem.message` is escaped before passing through `Prose`. The path label is not escaped, so the rendered output is parsed as markup and can emit malformed-looking text with a closing `</root>`.

Expected fix: escape `path_label` before interpolation, or use a non-markup label such as `root`.

### F4. Reported line/column positions are based on re-serialized frontmatter, not source frontmatter [HIGH]

**File**: `darkmatter/lib/src/markdown/schemas/mod.rs:332-356`

The spec says pretty and JSON problems include line/column "from the frontmatter." The implementation reconstructs a YAML document from the parsed map and scans that canonical rendering. That loses comments, blank lines, original quoting, and some formatting, so reported coordinates can differ from the actual source file the user is editing.

This is better than iteration 1's `None` positions, but it is not the source map promised by the spec. Tests currently check only that "at line" appears, not that the line/column match the original frontmatter.

Expected fix: preserve or compute positions from the original frontmatter slice during parsing. If that is out of scope, the spec and output text should explicitly say the coordinates refer to canonicalized frontmatter, but that is a weaker UX.

### F5. Raw JSON Schema validation includes the reserved `$schema` control key [MEDIUM]

**File**: `darkmatter/lib/src/markdown/schemas/mod.rs:394-400`

`frontmatter_as_json` validates the entire frontmatter map, including Darkmatter's reserved `$schema` key. SimplifiedSchema-generated schemas have `additionalProperties: true`, so this is mostly hidden there, but a referenced raw JSON Schema with `additionalProperties: false` will reject every document unless the schema explicitly allows Darkmatter's control key.

Because the spec reserves `$schema` for Darkmatter and also supports raw JSON Schema references, consumers should not need to model `$schema` as document data. Strip `$schema` from the instance before validation, or document and test the current behavior explicitly.

### F6. JSON Schema baseline restriction still permits `additionalProperties` [MEDIUM]

**File**: `darkmatter/lib/src/markdown/schemas/resolve.rs:405-413`

The spec says JSON Schema baselines are restricted to top-level `properties` and `required`, and explicitly lists `additionalProperties: false` among unsupported constructs. The implementation allows `additionalProperties` as a baseline key and later copies it into merged document schemas.

Allowing `additionalProperties: true` for generated SimplifiedSchema baselines may be harmless, but raw JSON baselines should reject `additionalProperties: false` per spec. Add a focused unit test for this restriction.

## Test Rigor Assessment

| Requirement | Strongest verification | Adequate? |
|---|---:|---|
| SimplifiedSchema grammar and conversion | Level 1 | Yes |
| Schema resolution and validation fixtures | Level 1 | Yes |
| CLI validate/detect behavior | Level 1 | Partial: missing per-document schema-error exit-code test and pretty arm-prefix test |
| Root-union closest-arm reporting | Level 1 | Partial: JSON only; pretty output still misses the arm prefix |
| Source line/column reporting | Level 1 | Partial: presence tested, source-coordinate accuracy not tested |
| Terminal rendering / keyboard / mouse behavior | N/A | No Level 2/3 needed; this feature is non-interactive and has no terminal-emulator input requirements |

## Verdict

Not ready for production. The remaining blockers are small in implementation size but visible to users and automation: schema errors return the wrong exit code, root-union pretty output omits the required arm index, and line/column reporting is not based on source coordinates.
