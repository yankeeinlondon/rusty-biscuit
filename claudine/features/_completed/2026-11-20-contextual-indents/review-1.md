---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): No-op Markdown saves rewrite authored text and resolve indentation"
    - "R1-F2 (high): In-memory document editing ignores explicit indentation"
    - "R1-F3 (high): Four schematic L1 tests retain failing package timings"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-20T22:43:32-08:00"
spec: 2026-11-20-contextual-indents/spec.md
implemented: true
description: "A **feature** review of `2026-11-20-contextual-indents/spec.md`"
feature: 2026-11-20-contextual-indents/review-1.md
log: /home/build/coding/rusty-biscuit/claudine/features/2026-11-20-contextual-indents/log.md
implemented_by: claude/opus
next: 2026-11-20-contextual-indents/review-2.md
---

# Review of Contextual Indents

The feature is not production ready. Three findings need repair. None needs
an author decision. This is the first implementation review; there are no
previous findings to carry forward.

## Findings

### R1-F1 (high): No-op Markdown saves rewrite authored text and resolve indentation

**Authority: spec**, “Existing no-op saves stay byte-identical” and “keep
discovery out of … no-op paths,” under Migrating Writers. The defect class
is missing unchanged-value detection before rendering and destination lookup.

In Darkmatter, [the frontmatter edit entry point](/home/build/coding/rusty-biscuit/darkmatter/lib/src/markdown/hash/path_edit.rs:91)
always applies the requested edits, even when the expected values equal the
original values. The plain-YAML editor gained an equality check, but the
Markdown editor did not. Its CLI caller also
[resolves indentation before editing](/home/build/coding/rusty-biscuit/darkmatter/cli/src/commands/frontmatter.rs:140).
The lifecycle [save function](/home/build/coding/rusty-biscuit/darkmatter/lib/src/effects/verbs.rs:46)
always serializes and writes, including when its caller changed no value.

Reproduction: create a document with this exact text, then run
`md set doc.md title same --save`:

```markdown
---
title: 'same' # keep
---
Body
```

The command exits successfully but removes the quotes. Setting an unchanged
mapping or sequence also removes comments inside that value. Through the
public lifecycle API, setting the same title, merging `{title: same}`, or
deleting a missing property removes the comment and the body's final newline.
These are ordinary repeated saves of user-authored files.

The bounded sweep covered the shared editor, its save callers, and the
changed lifecycle save callers. The input above was reused for each
applicable site; collection cases changed only the property's shape.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Markdown frontmatter editor, reached by `md set --save` | Equal quoted scalar | `'same'` becomes `same` | Original bytes |
| R1-F1.2 | Same editor, whole mapping replacement | Equal `meta: {name: same}`, authored as a block with a quoted value and comment | Quotes and inner comment removed | Original bytes |
| R1-F1.3 | Same editor, whole sequence replacement | Equal `tags: [same]`, authored as a block with a quoted item and comment | Quotes and item comment removed | Original bytes |
| R1-F1.4 | CLI destination lookup before the shared editor | Equal scalar in a fixture Git repository with `.editorconfig` | `strace -f -e trace=openat` records opening `.editorconfig` | No indentation lookup |
| R1-F1.5 | `EffectEngine::set_frontmatter` → save | Equal title | Comment, quotes, and body final newline lost | Original bytes, no lookup |
| R1-F1.6 | `EffectEngine::merge_frontmatter` → save | Merge the existing title | Same loss | Original bytes, no lookup |
| R1-F1.7 | `EffectEngine::delete_frontmatter` → save | Remove an absent property | Same loss | Original bytes, no lookup |
| R1-F1.8 | Plain-YAML editor | Equal authored scalar | Clean: existing equality short circuit returns source bytes | Original bytes |
| R1-F1.9 | `ConfigTransaction`, including Claudine saves | Equal values on a second save | Clean: bytes unchanged, zero resolver calls, `written: false` in the public outcome | Same |
| R1-F1.10 | Hash save | Unchanged stored baseline | Clean by the `new_stored == None` branch: no fallback call and no write | Same |
| R1-F1.11 | Claudine compatibility writer | No alias/name changes | Clean by its caller's `if changed` guard; writer is not called | Same |
| R1-F1.12 | Removal CLI; lifecycle increment/decrement and append/prepend | Successful actual mutation | Not an unchanged-value path: these operations necessarily change the value or return an error | No no-op branch required |

The transaction and plain-YAML control cases were checked by the targeted
public-API tests listed below. The hash and compatibility controls follow
explicit guards in the code, rather than an assumption that two writers agree.

**Recommended repair:** compare authored values before layout or writing in
the Markdown and lifecycle paths, and defer the CLI indentation lookup until
a changed value actually needs rendering. Preserve the existing validation
and refusal behavior. Extend the no-op tests with an authored quoted value,
an inner comment, and a final body newline; the current lifecycle test only
repeats a save after the first write has already normalized the file.

### R1-F2 (high): In-memory document editing ignores explicit indentation

**Authority: spec**, “New documents use the shared layout for both the initial
skeleton and the edits that complete it,” and destination-aware whole-document
output under Migrating Writers. The defect class is dropping a caller's
formatting argument on a serialization branch.

Darkmatter's new `edited_document_text_with_indentation` API accepts a
preference for the destination, but its
[source-less serialization branch](/home/build/coding/rusty-biscuit/darkmatter/lib/src/markdown/hash/path_edit.rs:159)
calls `edited.as_string()`, which always uses four spaces. A caller building
a new Markdown document in memory therefore gets the wrong layout even
though it supplied the resolved destination preference.

Public-API reproduction: create `Markdown::new("Body\n".to_string())`, insert
`meta = {child: {leaf: 1}}`, and call
`edited_document_text_with_indentation(&document, &[], two_spaces)`.
The result contains `meta:\n    child:\n        leaf: 1`, rather than
`meta:\n  child:\n    leaf: 1`. This was run against the built library, using
nested standard-library maps as the inserted value.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Source-less branch of `serialized_into_source` | New in-memory document; two-space argument | Four spaces | Two spaces |
| R1-F2.2 | Source-present serialization branch | Replace `meta` in flow-style frontmatter with the same nested value used above | Clean: public-API reproduction writes two-space levels through whole-frontmatter fallback | Caller preference |
| R1-F2.3 | Source-preserving node edit branch | Replace `meta: {}` with the same nested value, without a local example | Clean: public-API reproduction writes two-space levels through `Layout` | Caller preference |
| R1-F2.4 | `Markdown::as_string_with_indentation` | New in-memory document | Clean: directly forwards the argument to the writer; explicit-width unit test covers it | Caller preference |
| R1-F2.5 | Default wrappers | No explicit preference | Clean by contract: delegate with `Indentation::default()` | Four spaces |

**Recommended repair:** use `edited.as_string_with_indentation(indentation)`
in the source-less branch. Extend the existing in-memory-document test to
exercise a nondefault width through the new public API. The current test
calls the default wrapper, so it cannot detect the dropped argument.

### R1-F3 (high): Four schematic L1 tests retain failing package timings

**Authority: defect**, the generator tests exceed the review's required L1
execution budget on their ordinary shipped-API inputs: “An L1 test over 5 s
is a failing test,” including tests in the implementer's recorded runs. The defect class
is L1 verification exceeding its execution budget. These are existing
Schematic generator tests; the finding does not attribute their origin to
the indentation change.

The implementation records four slow tests in the Schematic package run on
a quiet 32-core host. Three also remain slow alone. The fourth's isolated
4.87 s result did not clear its quiet-package failure: the isolated-rerun
exemption applies only to a loaded original run. That mistaken conclusion
has been corrected in the implementation log with a dated verification note.

The sweep includes all four reported slow tests and the feature's targeted
test groups as clean controls. No whole-workspace timing audit was added.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | [Generated documentation test](/home/build/coding/rusty-biscuit/schematic/gen/tests/l1/generated_docs.rs:105) | Shipped API documentation generation/checks | Recorded slow in package; 5.10 s alone; review: **5.095 s** alone | At most 5 s alone and in a quiet package run |
| R1-F3.2 | [Generated Rust artifact test](/home/build/coding/rusty-biscuit/schematic/gen/tests/l1/artifact_drift.rs:401) | Generate and compare shipped Rust artifacts | Recorded slow in package; 5.16 s alone; review: **5.183 s** alone | Same |
| R1-F3.3 | [Generated REST source test](/home/build/coding/rusty-biscuit/schematic/gen/tests/l1/fresh_output_compile.rs:3352) | Generated REST ownership checks | Recorded slow in package; 5.34 s alone; review: **5.388 s** alone | Same |
| R1-F3.4 | [Definition-bundle test](/home/build/coding/rusty-biscuit/schematic/gen/tests/l1/definition_bundles.rs:23) | Every shipped bundle round trip and export | Recorded slow in quiet package; 4.87 s alone; review: **4.863 s** alone; no passing package remeasurement | Same |
| R1-F3.5 | Feature tests rerun during review | Resolver, CLI, writer, editor, transaction, Claudine persistence | Clean: no selected test reached 5 s | Same |

Review remeasurements used `just _test schematic-gen 'Schematic Generation'
<full-test-name>` for each row separately. `uptime` showed load averages
1.31, 1.63, and 2.98 when checked, consistent with a quiet host.

**Recommended repair:** reduce repeated generation/setup or divide the
all-API loops into bounded cases with a coverage guard, keeping every
assertion. Remeasure individually and in the package run. Do not rename
these into an excluded tier, increase timeouts, or drop coverage. These
repairs need no author ruling despite being outside the changed YAML path.

## Verification and coverage

I reviewed the spec and formatter reference, implementation log, changed
writer/editor/transaction code, resolver loaders and shared validators,
CLI projection, migration callers, serializer guard, tests, and documentation.
Two agents independently read the decision record and spec in parallel before
the implementation comparison.

Targeted canonical runs completed:

| Requirement | Strongest relevant verification | Review result |
| --- | --- | --- |
| YAML values, tags, order, multiline content, compact sequences, wide indentation | L1 writer corpus and explicit-layout assertions | `biscuit-file: just test yaml_writing::`: 21 selected tests passed; feature-minimal checks also passed |
| Formatter ranking/defaults, EditorConfig matching, invalid configuration diagnostics, destination discovery | L1 public library API with fixture worktrees | `sniff: just test indentation::`: 160 selected tests passed |
| Plain/JSON CLI results, aggregate payload, help, invalid arguments, loader wiring | L1 subprocess tests | `sniff: just test repo_indent::`: 22 passed |
| Authored local layout, indentless lists, multiline values, CRLF, no-op plain YAML | L1 public editor API | `darkmatter: just test yaml_edit::indentation_of_new_values`: 7 passed |
| Generated configuration, effective symlink destination, fallback diagnostics, lazy resolution, refused/raced creation | L1 public transaction API and filesystem results | `darkmatter: just test config_write_indentation::`: 8 passed |
| Claudine setup, destination preference, commented edits, unchanged saves, legacy migration/read-back/backup/retry | L1 public persistence/setup API | `claudine: just test config_indentation::`: 9 passed |
| No-op Markdown/lifecycle saves and new in-memory document layout | L1 direct public API and shipped CLI reproductions | Wrong results, detailed in R1-F1 and R1-F2 |
| L1 duration gate | Canonical nextest timing output | Failing, detailed in R1-F3 |

All selected feature tests were below five seconds. The Schematic timing
runs are **not passing gates**, even though nextest reports successful
assertions and the recipe prints a success banner. The feature's command
reports data and text; the spec expressly calls for L1 subprocess tests for
this command. It introduces no keyboard-input, terminal-encoder, or scrolling
requirement needing L2/L3 evidence. Cross-OS proof remains CI work and is not
a readiness finding.

The new test modules are declared by their consolidated test binaries. The
selected tests actually executed in L1; the implementation also records zero
stranded tests in touched areas. The serializer guard and hash/fingerprint
controls have passing implementation-run evidence; this review did not repeat
every unaffected package suite.

### Configuration input robustness

This applies to Sniff's formatter readers, not to YAML rendering or Markdown
text editing. Invalid individual settings produce a located `skipped`
diagnostic and preserve usable earlier/lower-priority values; malformed
documents are skipped and later candidates can apply. Absent settings take
formatter defaults. Empty declaring mappings are valid; empty/comment-only
YAML formatter files deliberately declare those defaults. Explicit null
settings remain invalid and are diagnosed rather than treated as absent.

The public-API tests exercise shared width, boolean, style, mapping, glob-list,
and override validators, plus per-field wiring controls. They cover absent,
null, wrong whole-field types, mixed/all-invalid list elements, and applicable
empty forms. Loader tests cover strict JSON, JSON5, TOML, YAML, and YAML
streams, including duplicate keys, trailing content, implicit YAML null,
second-document handling, and JSON5 comments/trailing commas. The code-smell
search and loader reads found no additional load-bearing permissive coercion
to report. CLI control/error wiring exists for each loader family. No new
input-robustness finding was identified.

## Implementation decisions (informational)

These are the implementer's recorded choices, assessed against the stop
rules. I recommend keeping them unless a finding above requires the narrow
correction stated there. None warrants an author interruption.

1. **YAML structure instead of source spans.** The writer walks the backend's
   reparsed value tree and obtains scalar spellings separately, rather than
   using the planned span parser. This is a literal implementation-method
   departure from the spec's source-span wording. The log documents valid
   YAML that the span parser rejects. The replacement protects scalar
   content structurally and checks semantic equivalence and mapping order;
   the corpus passes. Reintroducing the failing parser would buy no verified
   behavioral improvement. Keep the equivalent structure-based approach.
2. **Sequence scalar layout.** Literal content advances one requested level
   past the dash; tagged collections keep compact alignment. The spec does
   not fix a different scalar column. Keep it.
3. **Shared type conveniences.** Constructors and a default-width constant
   supplement the types; style has the required serde spelling, while the
   preference struct has no serde representation. Keep them.
4. **EditorConfig matcher.** The custom matcher supports nested alternatives
   and numeric ranges that the planned glob library cannot express. Tests
   exercise those cases. Keep it.
5. **Selected configurations without an opinion.** Unsupported shared
   Prettier presets and clang-format files for another language shadow later
   candidates; disabled Biome does so as the spec explicitly requires. This
   avoids reporting a configuration the formatter would not select.
6. **Make width.** Tab width 8 supplies the width the convention table leaves
   implicit. It does not impose a tab prefix on every Makefile line.
7. **Empty YAML configurations.** Empty/comment-only/marker-only files select
   formatter defaults, while explicit null is invalid. Keep the documented,
   tested distinction.
8. **Extension case.** File-type selection is case-normalized; EditorConfig
   matches the synthetic basename with caller case. This follows the spec's
   narrower “for file-type selection” wording.
9. **Resolver API details.** Extension lookup discovers from a base directory;
   normalization, root access, typed errors, and dotted diagnostic paths are
   exposed. Destination directories are canonicalized while the final
   filename is retained; writers must supply the effective destination they
   actually follow, as the spec requires.
10. **Plain source labels.** Output includes keys/defaults, tab widths,
    separate style/width attribution, adjustments, and skipped notes. These
    explain provenance without changing the result.
11. **Summary disagreement.** The additional “sources disagree” marker keeps
    summary disagreements visible.
12. **Aggregate discovery reuse.** The parent JSON command reuses its known
    worktree root. It shares resolution logic with the leaf command without
    repeating repository discovery.
13. **One width per rendered value.** A newly rendered value uses the nearest
    authored example of its top collection kind throughout its subtree.
    The spec does not require mixed widths inside a generated subtree.
14. **Scalar indentation examples.** Multiline scalar content can use the
    nearest authored collection of either kind. Scalar whitespace is never
    itself an indentation example.
15. **Plain-YAML equal-value short circuit.** Returning source bytes is the
    right implementation of the no-op contract; extend that protection to
    the missing Markdown paths in R1-F1.
16. **Resolver/warning APIs and eager callers.** The transaction's warning
    field, resolver seam, and Markdown helpers are reasonable. Eager
    lookup at a genuine write is acceptable; lookup on unchanged saves is
    covered by R1-F1 and must be removed there.
17. **Layout error classification.** Existing-text failures reuse
    `UnsupportedEdit`; generated-document failures use `Layout`. This
    preserves existing error matching while still refusing unsafe writes.
18. **DMLS insertion repair.** Fixing a discovered bug that glued a required
    key to the closing fence is appropriate; it needed no separate spec.
19. **Destinationless Claudine documents.** Inline composed documents retain
    four-space output because no saved destination is known.
20. **Schematic destination resolution.** The Sniff dependency and stderr
    fallback warning let OpenAPI YAML follow its actual destination.
21. **Additional whole-document migrations.** Generic YAML conversions,
    YAML-to-YAML CLI output, and Markdown frontmatter output belong to the
    spec's whole-document audit even where the plan did not list them
    individually. Conversion layout errors remain typed variants.
22. **Legacy migration validation.** Checking equivalence under the lock
    before publication is sound. Keeping the old post-publication check and
    allowing backup-conflict errors to be reported first are harmless.
23. **Serializer guard scope and cost.** Selective parsing, import/re-export
    resolution, the manifest coverage table, and excluding dev-only reach
    are reasonable. Macro-expanded calls from another crate remain an
    explicitly documented source-scanner limit.
24. **Execution choices.** Phase 2 ran without parallel agents to keep shared
    types consistent. EditorConfig duplicate assignments retain ordered
    merging and non-assignment lines are ignored. Neither needs a ruling.

### Deferred work and verification limits

- **Slow Schematic tests:** deferred because they do not reach the changed
  YAML path. That reason does not exempt the gate; R1-F3 inventories all four
  failures from the recorded quiet package run.
- **Schematic fresh-output tier:** the recorded offline build lacks cached
  `target-tuple`. The implementer declined the documented Cargo fetch because
  it changes the host. Fetching declared dependencies is routine authorized
  setup; no author approval is needed. This is an outstanding environment
  verification limit, not evidence of a YAML implementation defect.
- **Darkmatter lint:** the recorded final Zed check lacks `wasm32-wasip2`.
  Clippy passed; the full lint recipe was not green. This review did not
  install the target or verify the unchanged extension.
- **Other operating systems:** execution was left to CI because the log
  records no configured local rigs. This does not block this review.
- **Unrelated Sniff terminal tests:** the existing git-status/CI-style L2
  suite was not rerun; it does not establish any new indentation requirement.
- **Earlier consumer omissions:** earlier phases omitted unchanged consumer
  suites; the later Claudine suite and this review's persistence tests cover
  the final Claudine implementation. This is not a remaining scope deferral.

The 23 frontmatter decision entries were flagged for review, but none reserves
an unresolved choice for the author. There is no “Decided during
implementation” heading or additional outstanding author ruling. No
functional scope was otherwise explicitly deferred.
