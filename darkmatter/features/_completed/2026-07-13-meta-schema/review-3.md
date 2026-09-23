---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T20:55:51-07:00
spec: 2026-07-13-meta-schema/spec.md
implemented: true
next: 2026-07-13-meta-schema/review-4.md
implemented_by: claude/default
log: darkmatter/features/2026-07-13-meta-schema/log.md
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-3.md
previous: 2026-07-13-meta-schema/review-2.md
---

# Review 3 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 2's mixed-union,
standalone-error fallback, and trimmed-reference defects are fixed and covered
at the correct Level-1 boundary. The new tolerant cursor API also fixes the
specific incomplete-value cases named by Review 2.

Two high-severity gaps remain. Standalone authoring does not consistently use
the shared schema-declaration parser, so it accepts invalid reference arms and
rejects a valid scalar reference declaration. DMLS also still derives semantic
owners and ancestor paths from dotted strings, indentation, and raw colon
searches instead of the structural AST/sidecar, breaking valid quoted and
punctuated keys. The canonical Level-2 area gate remains red on unrelated CLI
rendering tests, although all meta-schema-relevant Level-2 slices pass.

## Findings

### High: Standalone schema documents bypass declaration-level reference validation

The specification makes `parse_schema_declaration` the passive authority for a
complete `schema` value, including syntax-checking every local reference and
rejecting HTTP(S) without performing I/O
([spec.md:351](spec.md#L351), [spec.md:723](spec.md#L723)). The authority does
that for scalar declarations and root-union reference arms
([simplified/mod.rs:73](../../lib/src/markdown/schemas/simplified/mod.rs#L73)).

Standalone parsing takes a different path. Its source-aware payload helper calls
`parse_yaml_schema`, not `parse_schema_declaration`
([source.rs:168](../../lib/src/markdown/schemas/simplified/source.rs#L168)).
`parse_yaml_schema` deliberately stores string arms as unchecked
`SchemaArm::FileRef` values
([simplified/mod.rs:101](../../lib/src/markdown/schemas/simplified/mod.rs#L101)).
Consequently, standalone pure documents such as these are accepted as valid
authoring models and publish no schema diagnostic:

```yaml
$schema: ["   "]
```

```yaml
$schema: [https://example.com/schema.yaml]
```

The inverse is also broken. A scalar local reference is a valid `schema`
declaration, but `parse_standalone_schema_document` rejects every pure scalar
payload before the declaration parser is reached
([standalone.rs:143](../../lib/src/markdown/schemas/simplified/standalone.rs#L143)).
Thus `$schema: ./other.yaml` becomes
`dm.schema.document_malformed` instead of an active, valid standalone schema
reference. This also contradicts the module documentation that says standalone
envelopes support whole-file references
([standalone.rs:15](../../lib/src/markdown/schemas/simplified/standalone.rs#L15)).

These behaviors violate acceptance criteria 3, 4, 7, and 9. They are not
optional follow-ups: the implementation log records both cases, but the
specification requires declaration-parser parity. Make the standalone model
carry the shared `SchemaDeclaration` product (or an equivalent projection of
it), route every pure payload through the source-aware declaration parser, and
retain mapping-only enforcement for tagged `types`. Add Level-1 public-parser
and in-memory LSP tests for local scalar references, whitespace-only and remote
reference arms, mixed valid/invalid root unions, and exact diagnostic ranges.

### High: DMLS still reconstructs semantic paths instead of consuming structural paths

Review 2 required completion and diagnostic ownership to come from the shared
AST/sidecar rather than line indentation and decoded-text searches. The new
cursor API correctly replaces the value-internal `rfind`/split heuristics, but
the owner path reaching that API remains text-derived:

- Frontmatter semantic activation splits `FmEntry::dotted` on `.`
  ([schema.rs:169](../../dmls/src/overlay/schema.rs#L169)). A valid property key
  such as `"build.target"` is therefore interpreted as two nested keys and never
  activates its declared `type-definition` or `schema` behavior.
- Completion finds the first raw `:` on the line and reconstructs ancestors by
  indentation plus `split(':')`
  ([frontmatter.rs:101](../../dmls/src/providers/frontmatter.rs#L101),
  [frontmatter.rs:1741](../../dmls/src/providers/frontmatter.rs#L1741)). Quoted
  keys containing `:` are misparsed, and an ancestor written as `"$schema"`
  retains its quotes and does not match the reserved `$schema` path.
- Standalone inner-definition diagnostics rebuild dotted lookup strings with
  `format!("{payload_key}.{key}")`
  ([frontmatter.rs:782](../../dmls/src/diagnostics/frontmatter.rs#L782)). A key
  containing `.` can therefore fall through from
  `dm.schema.invalid_type_definition` to a whole-document malformed result.

These are observable acceptance-criterion 7/9 failures, not merely internal
style concerns. `FrontmatterAst` already exposes RFC-6901 pointers, entry
parents, and `entry_at_offset`, while `SchemaSourceMap` exposes structural
`SchemaSourcePath`s. Use those products end-to-end; do not convert them through
dotted strings or rescan source lines. This also avoids the current reverse
line scan on each completion request. Add Level-1 LSP regressions for quoted
`$schema`, keys containing `.`, `:`, `/`, and `~`, nested mappings, CRLF, and a
malformed current buffer retaining last-good semantic ownership.

### Medium: The canonical Level-2 release gate is still red

Acceptance criterion 13 requires both `just test` and `just test-l2` to pass.
The complete Level-1 population passes, and the relevant Level-2 slices pass:
18/18 Darkmatter library tests, 3/3 `schema about` CLI tests, and 3/3 DMLS
tests. However, canonical `just test-l2` still aborts in the CLI tier on
`level2_code_block_clears_inherited_dim_before_theme_colors`; a no-fail-fast
diagnostic run also reproduced the other known code-block luminance failures
and exposed harness PATH failures where `md` was not found.

The failures are outside the meta-schema execution paths, so they do not weaken
the Level-1 semantic evidence. They do mean the feature cannot truthfully claim
the specification's area-level release gate is green. Either repair the CLI L2
gate or explicitly revise AC13 to define an approved scoped exception before
marking the feature ready.

## Prior Review Closure

- **Trimmed schema references — closed.** Classification, `FileReference`,
  resolution, and error text now share the canonical trimmed string; passive and
  resolver parity tests cover padded and whitespace-only values.
- **Mixed semantic unions — closed.** Specialized diagnostics are gated on
  whole-union rejection, ordinary sibling-arm completions are merged, and both
  arm orders have Level-1 LSP regressions.
- **Standalone outer-declaration fallback — closed for reached errors.** Empty
  unions, invalid scalar arms, invalid scalar references, malformed YAML, and
  inner-definition errors now have distinct codes and focused ranges. The first
  finding identifies declaration inputs that incorrectly never reach this path.
- **Shared parser/source authority — partially closed.** Value-internal tolerant
  cursor state and structural invalid-node ranging are now library-owned. Owner
  activation, ancestor selection, and several diagnostic lookups still use the
  prohibited dotted/line heuristics, as described in the second finding.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, semantic arrays | Level 1 | Level-1 library/CLI integration, proptest, and filesystem round-trip tests | Appropriate and passing for ordinary inline/frontmatter paths. Standalone reference parity remains broken under AC3/4. |
| AC7: one passive semantic/source-aware authority | Level 1 | Level-1 parser parity, span projection, and tolerant cursor tests | Library evidence is strong, but DMLS owner/path selection still bypasses the structural authority. |
| AC8: base schema and `$schema` hover identity | Level 1 | Level-1 resolver and in-memory LSP tests | Appropriate and passing for the exercised mapping form. |
| AC9: DMLS completion, hover, diagnostics, activation, last-good state | Level 1 | Level-1 in-memory LSP tests | Correct level, but quoted/punctuated keys and standalone scalar/reference-arm declarations are missing and broken. |
| AC10: no file/process/shell/network side effects | Level 1 | Level-1 sentinel integration tests | Appropriate and passing; the required fixes remain passive. |
| AC11–12: recursion bound and compatibility | Level 1 | Level-1 depth-boundary, baseline replay, and downstream tests | Appropriate and passing for covered inputs. |
| AC13: real terminal presentation/editor integration and area release gate | Level 2 for terminal/editor rendering; Level 1 for LSP protocol semantics | Feature-relevant L2 slices pass; canonical CLI L2 gate fails | Appropriate levels exist, but the declared area gate is not green. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. The two production
blockers are missing/incorrect Level-1 semantic behavior, not a need to promote
their tests to Level 2 or Level 3.

## Verification Performed

- `FileReference` resolved the specification and Review 2 to the canonical
  Darkmatter feature directory. The requested
  `@prompts/./_reviews/.../review-2.md` reference does not exist, so the canonical
  resolved Review 2 file is the previous-review target.
- `sniff` identified the `darkmatter`, `darkmatter-cli`, and `dmls` crates and
  their downstream dependency edges.
- GitNexus reports CRITICAL upstream impact for `classify_schema_reference`
  (40 impacted symbols across six modules), MEDIUM impact for the tolerant
  cursor and meta-schema completion paths, and LOW impact for the standalone
  diagnostic helper.
- Eight bounded `just test --partition hash:N/8` runs passed: Darkmatter
  **5,915/5,915**, Darkmatter CLI **561/561**, and DMLS **605/605**. Three
  Darkmatter tests passed on configured retry.
- `just build` and `just lint` passed for all three affected crates.
- Darkmatter Level 2: **18/18 passed**. Focused CLI `schema about` Level 2:
  **3/3 passed**. DMLS Level 2: **3/3 passed**.
- Canonical `just test-l2` failed in the CLI tier on the documented code-block
  luminance assertion. A bounded no-fail-fast diagnostic run was interrupted
  after 110 seconds once it had established additional unrelated luminance and
  missing-`md` harness failures; pane cleanup completed through the recipe trap.
- `md schema validate` accepts the specification. Review 3 cannot currently be
  schema-adjudicated because the existing `schemas/feature-review.yaml` tagged
  envelope contains unsupported `description` and `$schema` keys; this is the
  same schema-infrastructure drift recorded by Review 2.
- No Rust source was modified and no formatting command was run during this
  review.

## Production Readiness

**Not ready.** Unify standalone parsing with `parse_schema_declaration`, remove
the remaining dotted/line-based DMLS path reconstruction, add the Level-1
regressions above, and resolve or formally scope the canonical L2 gate before
setting `ready: true`.
