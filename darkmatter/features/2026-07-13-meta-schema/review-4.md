---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T23:07:31-07:00
spec: 2026-07-13-meta-schema/spec.md
implemented: true
next: 2026-07-13-meta-schema/review-5.md
implemented_by: claude/default
log: darkmatter/features/2026-07-13-meta-schema/log.md
description: "A **feature** review of `2026-07-13-meta-schema/spec.md`"
feature: 2026-07-13-meta-schema/review-4.md
previous: 2026-07-13-meta-schema/review-3.md
---

# Review 4 — Meta Schema

## Verdict

This feature is **not ready for production**. Review 3's standalone-parser
parity defect is fixed, and its structural-path changes repair the quoted
`$schema`, dotted-key, hover, and diagnostic cases that the new tests exercise.
All Level-1 tests pass, as do the feature-relevant Level-2 slices.

Two high-severity implementation gaps remain. DMLS completion still reduces a
structural path to one decoded owner name and compares that name to an encoded
RFC 6901 pointer, which drops nested semantic values and top-level keys
containing `/` or `~`. Schema-file reference delegation also has no cycle/depth
guard and replaces rather than accumulates transitive dependency paths. The
canonical Level-2 gate remains red, and the specification still labels its
proposed three-test exception as awaiting ratification.

## Findings

### High: Completion loses nested semantic owners and RFC 6901-escaped keys

`frontmatter_authoring` correctly records each semantic value at its complete
RFC 6901 pointer, including nested entries
([schema.rs:169](../../dmls/src/overlay/schema.rs#L169)). Completion then throws
that structure away. `meta_schema_kinds_for_line` chooses only the first
ancestor (or the current key) as `owner`, and searches for a value whose
pointer, after removing its first `/`, equals that decoded owner
([frontmatter.rs:203](../../dmls/src/providers/frontmatter.rs#L203)).

That lookup cannot find either of these valid activations:

```yaml
$schema:
  parameter:
    type: type-definition
  "a/b": type-definition
  "c~d": type-definition

parameter:
  type: str
"a/b": str
"c~d": str
```

The nested value is recorded as `/parameter/type`, but completion searches for
`/parameter`. The escaped top-level values are recorded as `/a~1b` and `/c~0d`,
but completion compares those spellings with decoded `a/b` and `c~d`. In every
case `meta_schema_completion` returns no semantic candidates, so the user loses
the required type/constraint/scaffold completion. The nested example is the
authoring shape used by Feature A itself, and the escaped-key cases are among
Review 3's explicitly requested regressions.

The new structural-path LSP test verifies hover for `/` and `~`, while its
completion test covers only quoted `$schema` and a dotted top-level key
([lsp_session.rs:4284](../../dmls/tests/lsp_session.rs#L4284),
[lsp_session.rs:4341](../../dmls/tests/lsp_session.rs#L4341)). Neither test
exercises completion for a nested semantic property or an RFC 6901-escaped
owner. This violates acceptance criteria 7 and 9.

Match the complete structural key path (or exact encoded pointer) of the value
being authored. Do not compare one decoded segment with a serialized pointer.
Add Level-1 in-memory LSP tests for nested `type-definition` and `schema`
properties, `/` and `~` owner keys, their `[]` forms, LF/CRLF, and malformed
current-buffer retention.

### High: Referenced schema-file graphs can recurse without bound and lose transitive dependencies

The standalone fix makes a pure scalar schema document delegate to another
schema file by calling `resolve_reference` recursively
([resolve.rs:381](../../lib/src/markdown/schemas/resolve.rs#L381)). Root-union
file arms use the same recursion. There is no canonical-path stack, visited
set, or reference-depth limit on that path. A file that references itself, or
two files that reference each other, therefore recurses until the process
overflows its stack instead of returning a structured `SchemaError`.

The successful case also loses graph information. After a nested resolution
returns, each outer `resolve_reference` overwrites `origin` and replaces
`referenced_files` with only its own path
([resolve.rs:344](../../lib/src/markdown/schemas/resolve.rs#L344)); the
schema-root branch does the same at
[resolve.rs:305](../../lib/src/markdown/schemas/resolve.rs#L305)). For a document
referencing `a.yaml`, where `a.yaml` delegates to `b.yaml`, the final dependency
list contains `a.yaml` but not `b.yaml`. That contradicts
`EffectiveSchema::dependencies`' invalidation contract
([mod.rs:658](../../lib/src/markdown/schemas/mod.rs#L658)): editing `b.yaml` can
leave DMLS validating against a stale cached schema.

The new tests stop at passive parsing/classification of a scalar standalone
reference
([meta_schema_phase6.rs:70](../../lib/tests/meta_schema_phase6.rs#L70)); no
Level-1 test resolves a delegation, checks a transitive dependency, or supplies
a direct/transitive cycle. This affects acceptance criteria 8, 9, and the
production-readiness requirement in 13.

Thread one reference-resolution context through scalar and root-union
delegation, keyed by canonical file paths, with a bounded depth and structured
cycle error. Accumulate and deduplicate every visited schema dependency instead
of overwriting the nested result. Add Level-1 resolver tests for a valid
multi-hop chain, self-cycle, two-file cycle, and mixed root-union cycle, plus a
DMLS cache-invalidation test that edits the terminal file in a chain.

### Medium: The canonical Level-2 gate remains red and its exception is not approved

Acceptance criterion 13 still requires `just test-l2` to pass. This review
reproduced the existing failure in
`level2_code_block_clears_inherited_dim_before_theme_colors`: the Darkmatter
library tier passed 18/18, then the CLI tier stopped after 2 passes and that
failure, leaving 66 tests and the DMLS tier unrun.

The specification now documents a tightly scoped exception for three
code-block tests and gives persuasive evidence that the staging defect is
unrelated to meta-schema behavior. However, the specification also states that
the exception is **proposed and awaiting ratification**. Reviewers cannot treat
it as approved. The focused `schema about` Level-2 tests pass 3/3, and the DMLS
Neovim/tmux tests pass 3/3, so the meta-schema presentation evidence is sound;
the declared area-level release gate is not.

## Prior Review Closure

- **Standalone declaration-parser parity — closed.** Pure scalar, sequence, and
  mapping payloads now route through the shared declaration authority. Valid
  local scalar references activate, while whitespace-only and remote reference
  arms fail passively. The new Level-1 parser and LSP regressions pass.
- **Structural DMLS paths — partially closed.** Activation, hover lookup,
  quoted-key handling, and standalone diagnostic lookup now use structural AST
  helpers. Completion's owner lookup still collapses the path and compares
  decoded text with an encoded pointer, as described in the first finding.
- **Canonical Level-2 release gate — open.** The same unrelated code-block
  failure reproduces, and the proposed exception is not yet ratified.

## Requirement Verification Levels

| Requirement | Appropriate level | Strongest verification present | Assessment |
| --- | --- | --- | --- |
| AC1–6: grammar, lowering, validation, serialization, semantic arrays | Level 1 | Level-1 library/CLI integration, property tests, and filesystem round trips | Appropriate and passing. |
| AC7: one passive semantic/source-aware authority | Level 1 | Level-1 parser parity, source projection, and DMLS structural-path tests | Parser parity passes, but completion does not preserve the complete structural owner path. |
| AC8: base schema and referenced declaration preparation | Level 1 | Level-1 base-schema and direct-reference tests | Direct cases pass; delegated reference graphs have no cycle/dependency coverage and are broken. |
| AC9: DMLS completion, hover, diagnostics, activation, last-good state | Level 1 | Level-1 in-memory LSP tests | Correct tier, but nested and RFC 6901-escaped completion owners have no matching test and fail in the provider. |
| AC10: no file/process/shell/network side effects during semantic analysis | Level 1 | Level-1 sentinel integration tests | Appropriate and passing for passive analysis. |
| AC11–12: grammar recursion bounds and compatibility | Level 1 | Level-1 grammar-depth, baseline replay, and downstream tests | Appropriate and passing. The separate file-reference graph issue is outside AC11's semantic-parser depth contract. |
| AC13: real-terminal/editor presentation and area release gate | Level 2 for terminal/editor rendering; Level 1 for protocol semantics | 18 library, 3 `schema about`, and 3 DMLS Level-2 tests pass; canonical CLI tier fails | Feature slices have the right levels, but the required full gate is red and its exception is unratified. |

No requirement concerns OS keyboard encoding, modifier visibility, hotkeys,
paste, IME, or mouse delivery, so Level 3 is not applicable. The two behavioral
gaps require Level-1 verification; promoting them to Level 2 or Level 3 would
not test a more relevant boundary.

## Verification Performed

- The requested `@prompts/./_reviews/.../review-3.md` path does not exist. The
  canonical previous review is the colocated
  `darkmatter/features/2026-07-13-meta-schema/review-3.md`, which was used for
  the review chain update.
- `sniff` identified the affected `darkmatter`, `darkmatter-cli`, and `dmls`
  package area and the library's downstream consumers.
- GitNexus reports CRITICAL upstream impact for both passive parser authorities:
  `parse_property_definition` reaches 78 symbols across eight modules, and
  `parse_schema_declaration` reaches 92 symbols across eight modules.
  `resolve_reference` is HIGH risk with 49 impacted symbols across schema,
  trigger, and test modules.
- Eight bounded Level-1 partitions passed: Darkmatter **5,920/5,920**,
  Darkmatter CLI **561/561**, and DMLS **611/611**. The newly added standalone
  parser and structural-path regressions also passed in focused runs.
- `just build` passed for all three crates. The three scoped Clippy invocations
  completed without warnings or errors. No formatting command was run.
- Feature-relevant Level 2 passed: Darkmatter library **18/18**, CLI
  `schema about` **3/3**, and DMLS Neovim/tmux **3/3**. Canonical
  `just test-l2` failed in the CLI tier on the known inherited-dim code-block
  test after the library's 18/18 pass.
- `md schema validate` accepts the specification. Review 4 cannot be validated
  against `feature-review.yaml` because that existing tagged schema envelope
  contains unsupported `description` and `$schema` keys, the same schema
  infrastructure drift recorded by the prior reviews.
- No Rust source was modified during this review.

## Production Readiness

**Not ready.** Preserve full structural owner paths through DMLS completion,
make schema-file delegation cycle-safe and dependency-complete, add the missing
Level-1 regressions, and either restore the canonical Level-2 gate or obtain the
documented scope-exception ratification before setting `ready: true`.
