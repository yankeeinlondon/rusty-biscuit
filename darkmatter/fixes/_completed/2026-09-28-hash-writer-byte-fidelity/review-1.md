---
$schema: feature-review.yaml
ready: false
findings:
    - title: Hash replacement can silently change unrelated alias values
      priority: high
    - title: Date writing mistakes indented comments for collection values
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-09-29T12:00:31-07:00
spec: 2026-09-28-hash-writer-byte-fidelity/spec.md
log: darkmatter/fixes/2026-09-28-hash-writer-byte-fidelity/log.md
implemented: true
next: 2026-09-28-hash-writer-byte-fidelity/review-2.md
implemented_by: claude/opus
description: "A **fix** review of `2026-09-28-hash-writer-byte-fidelity/spec.md`"
fix: 2026-09-28-hash-writer-byte-fidelity/review-1.md
---

# Review of Hash Writer Byte Fidelity

The fix is **not production ready**. The six concrete examples work, but a hash replacement can silently change an unrelated frontmatter value while passing the new validation. Valid empty dates followed by indented comments also remain unwritable. Neither finding requires human input to resolve.

The review covered the specification, plan, implementation log, changes since `dadebc029`, both production callers, the shared leaf locator, and the current documentation. The plan deliberately preserves leading spacing for spelled `null` and `~`, and adds refusal for flow collections; these are documented implementation choices rather than new findings.

## Findings

### High — Hash replacement can silently change unrelated alias values

**Defect class:** validating only that edited YAML parses does not establish that values outside the managed properties stayed unchanged when an edit removes an anchor declaration.

In the darkmatter package, [apply_hash_save_text](../../lib/src/markdown/hash/write.rs:270) replaces the complete hash node, and [validated](../../lib/src/markdown/hash/write.rs:339) only parses the result. YAML permits reuse of an anchor name: an alias resolves to the most recent preceding declaration. Removing the declaration inside the hash can expose an earlier declaration instead of creating an unresolved alias. The output is valid YAML with different data.

For example, run `md hash --save doc.md` on:

```yaml
---
earlier: &h before
hash: &h aaaa111111111111-bbbb222222222222
mirror: *h
last_updated: 2026-01-01
---
Changed body
```

Before the save, `mirror` means `aaaa111111111111-bbbb222222222222`. After the save it means `before`, although its source bytes did not change. The command exits successfully. An immediate `md hash --diff doc.md` exits `2`, because the stored hash was computed before this unrelated value changed.

This violates acceptance criterion 3: the parsed record may differ only at the managed hash and bumped date. The plan's decision to let output validation handle hash anchors covers unresolved aliases, but misses aliases that resolve to a different surviving declaration. The anchor-removal behavior predates this fix; the newly added safety validation and its acceptance coverage remain incomplete.

The sibling sweep used the same earlier declaration, managed declaration, and later alias, changing only the managed shape or caller:

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter text writer, existing simple hash | Anchor on hash scalar; earlier same-name declaration; date bump on and off | Returns text; `mirror` changes to `before` | Refuse, or preserve every unmanaged value |
| darkmatter text writer, existing collection hash | Anchor inside the removed hash mapping; bump on and off | Returns text; `mirror` changes from `after` to `before` | Same protection throughout the entire replaced node |
| darkmatter-cli save | Simple hash, structured hash with anchor on `value`, and quoted custom `fingerprint` key | All save successfully, change `mirror`, and fail immediate `--diff` with exit `2` | Refuse without writing if unmanaged values would change |
| claudine closure write-back | Anchored simple hash with the same earlier declaration and alias | Writes changed `mirror`; reports an empty frontmatter change list | Refuse before the atomic write |
| darkmatter text writer, control | A later declaration between hash and alias | Alias remains `after` | Clean: preserve the original alias value |
| darkmatter text writer, unresolved-alias control | Hash is the only declaration used by the alias | Existing test refuses with `FrontmatterTextEdit` | Clean: refuse without output |
| darkmatter date writer | Anchor, alias, or tag on bumped `last_updated` | Refuses; CLI and closure retain the file bytes | Clean: refuse |
| darkmatter leaf locator | Anchored hash scalar / collection root | Refuses with `NodeProperties` / `NotAScalar` | Clean: no editable unsafe leaf |
| darkmatter map-based writer, comparison only | Same reused-anchor fixtures | Serialized parsed map retains the original `mirror` value | Clean semantically; this serializer is outside the byte-preserving fix |

**Required correction:** compare parsed input and output values outside the managed property and, only when bumped, `last_updated`, returning `FrontmatterTextEdit` if they differ. This uses the preservation contract directly and protects anchors at every depth without adding a separate YAML reference parser. Alternatively, conservatively refuse removal of potentially referenced declarations anywhere in the replaced node. Preserve the early `None` return for decisions that require no write.

Add persistent Level-1 tests through the public writer for scalar and nested anchor reuse, with and without a date bump, plus CLI and closure tests proving refusal leaves the file untouched. Keep the later-declaration control so the intended safety boundary is explicit. Update the current docs' unresolved-alias example to explain that a still-valid alias can also change meaning.

### Medium — Date writing mistakes indented comments for collection values

**Defect class:** a node's physical line count is used as a substitute for its YAML value type, so harmless comment lines make an editable scalar or null look like a collection.

In the darkmatter package, [rewrite_date_scalar](../../lib/src/markdown/hash/write.rs:922) rejects any node extending past its first line. Both top-level node scanners include following indented comment lines in that range. Consequently, this valid empty date still cannot be stamped:

```yaml
---
hash: aaaa111111111111-bbbb222222222222
last_updated:
  # set this when the body changes
author: A
---
Changed body
```

`md hash --save` exits with “`last_updated` must be a scalar value” and leaves the file unchanged. Removing just the two spaces before `#` makes the save work. The same rejection occurs with `last_updated: 2026-01-01` before the indented comment. This guard predates the fix, but the specification's promise that null dates remain writable and comments survive is still incomplete.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| darkmatter date writer | Plain scalar followed by an indented comment | Refuses as nonscalar | Edit the scalar and retain the comment bytes |
| darkmatter date writer | Empty value followed by an indented comment | Refuses as nonscalar | Insert the date and retain the comment bytes |
| darkmatter-cli and claudine closure | Both shapes above, in LF, CRLF, and lone-CR files | Both refuse; files remain unchanged | Successful date write preserving each terminator and comment |
| darkmatter-cli, no-bump control | Empty date and indented comment, first hash baseline | Saves without editing the date | Clean: no date inspection required |
| darkmatter date writer, comment control | Empty date followed by an unindented comment | Writes date and preserves comment | Clean |
| darkmatter leaf locator | Plain scalar with indented comment | Locates only the scalar bytes | Clean: demonstrates that the comment does not prevent locating the value |
| darkmatter leaf locator | Empty date with indented comment | Refuses as `NotAScalar` | Clean for this string-leaf API; null handling belongs to the date writer |
| darkmatter property restoration | Identical snapshot/current with either comment shape | Retains all source bytes | Clean |
| darkmatter date writer, broader layout sweep | Plain or quoted scalar placed on the following line; `|-` scalar | Refuses as nonscalar; the leaf locator accepts all three | These are also scalars; support them or explicitly document a narrower layout contract |

**Required correction:** distinguish comments from actual value continuation before refusing the node, and replace only the date's value bytes while retaining the remaining authored text. Add the two comment cases to the existing matrix and exercise both callers with exact file-byte assertions. Resolve the broader scalar-layout limitation in the same pass; at minimum, documentation and diagnostics must not describe ordinary scalars as collections.

## Input robustness sweep

YAML frontmatter is the only format changed. The load-bearing fields are the configurable hash property and `last_updated`. The public writer was exercised with a canonical hash/date control, changing one field per case. This table records public write outcomes; the CLI's stored-hash parser has separate validation before the writer and is unchanged by this fix.

| Input shape | Managed hash property | `last_updated`, bump requested |
| --- | --- | --- |
| Control | Replaced | Date replaced |
| Absent | Inserted | Inserted |
| Explicit `null` | Replaced | Replaced |
| Empty YAML value | Replaced | Date inserted; indented-comment failure above |
| Number / boolean | Replaced as a managed node | Replaced as a scalar |
| Collection with one non-string element (`[a, 123]`) | Whole node replaced | Refused as collection |
| Collection with all non-string elements (`[123]`) | Whole node replaced | Refused as collection |
| Empty sequence / mapping (`[]`, `{}`) | Whole node replaced | Refused as collection |
| Duplicate semantic key, including quoted spelling | Refused | Refused |
| Invalid collection syntax | Refused | Refused |
| Valid property followed by invalid YAML inside frontmatter | Refused | Refused |
| Anchor, alias, tag | Managed replacement; anchor-reuse failure above | Refused before date edit |

Absence and explicit null intentionally lead to different editing operations, even though both end with a new value. No element is silently filtered. Body text after the closing delimiter is opaque Markdown, not trailing YAML. The committed matrix covers many of these rows but lacks the two defect classes above and some numeric, boolean, mixed-element, and empty-collection controls exercised during review.

## Requirement verification and execution evidence

All requirements in this fix concern text, parsing, or filesystem persistence. Level 1 is appropriate; there is no terminal rendering or keyboard-input promise requiring Level 2 or Level 3.

| Requirement | Strongest committed verification | Assessment |
| --- | --- | --- |
| Six reported cases, plus CRLF empty date | Level-1 writer tests with exact bytes or typed refusal | Pass |
| Alias/tag refusal and no-bump behavior | Level-1 writer tests; CLI anchored-date no-write test | Pass for covered forms; closure refusal has no committed dedicated test |
| Mixed terminators, lone CR, inserted date, existing/new BOM, unterminated body | Level-1 writer tests | Pass |
| Unmanaged byte and parsed-value preservation | Level-1 writer comparisons and CLI save/diff matrix | Incomplete: reused anchors and indented comments are missing |
| Repeated save stability | Level-1 writer, CLI, and closure tests | Pass for covered fixtures |
| Current documentation | CLI guide, darkmatter skill, Claudine composition topic | Serializer description corrected; findings require follow-up |

Executed successfully:

- `cd darkmatter && just test 'hash::write::'`: 50 tests.
- `cd darkmatter && just test hash_kind_save_diff::`: 23 tests.
- `cd claudine && just test-library 'composition::closure::tests::'`: 24 existing tests plus one temporary date-shape probe.
- Temporary Level-1 CLI probes for comment layouts and alias reuse, plus a temporary closure alias-reuse probe: all reproduced the stated behavior.
- A temporary example exercised the public writer, leaf locator, property restorer, and map serializer, including the robustness table and both date-bump states.
- `just check-tier-coverage darkmatter`: no stranded tests. The CLI test module is declared by `tests/l1/main.rs`; writer and closure tests compile as library unit tests. The changed tests have no tier prefix or feature gate that excludes them from L1.

Temporary probes were removed after verification; this review leaves production code and existing tests unchanged. The full area suites and lint were not rerun for this document-only review. Claudine's test link emitted a large unwind-table warning, but its tests passed. Cross-OS evidence is left to CI and does not affect this readiness decision.
