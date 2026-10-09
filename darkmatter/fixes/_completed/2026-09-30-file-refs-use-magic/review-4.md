---
$schema: feature-review.yaml
ready: false
findings:
    - title: Schema detection discards the opened document's resolution context
      priority: high
    - title: Hash directory detection overrides earlier files and suppresses I/O failures
      priority: high
human_review: true
human_review_items:
    - |-
        Review the completeness of the reference-reader tests before restarting the automated cycle. The previous fixes now pass their original reproductions, but schema detection still loses the document's context after opening it, and hashing makes its own file-versus-directory decision before shared resolution. Compare the reader census and instance tables below with the permanent tests. Confirm that tests check the selected target and the meaning of references inside an opened document, including competing files and directories and terminal filesystem errors. This is a completeness check; neither finding needs a new product decision to implement.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-02T04:50:44-07:00
spec: 2026-09-30-file-refs-use-magic/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-30-file-refs-use-magic/log.md
description: "A **fix** review of `2026-09-30-file-refs-use-magic/spec.md`"
fix: 2026-09-30-file-refs-use-magic/review-4.md
previous: 2026-09-30-file-refs-use-magic/review-3.md
next: 2026-09-30-file-refs-use-magic/review-5.md
---

# Review 4

This fix is **not production ready**. The previous review's demonstrated completion, external-source magic, and CLI argument-opening failures are fixed. Two sibling decisions still disagree with the shared resolver: schema detection silently changes valid file values into strings, and hashing can select a later directory instead of an earlier file or filesystem error.

## Previous findings

Review #3 had two unblocked findings and no blocked findings. No blocked implementation item was subsequently unblocked. Its separate request for a human completeness check has no recorded completion in the implementation log; it remains a process item, not a reason to leave these implementation fixes blocked.

| Previous finding | Verification in this review | Status |
|---|---|---|
| External-source contexts lose repository or launch magic anchors | Two-repository completion tests for all three commands and schema forms; external-document composition, graph, validation, cleaning, and public context tests | Previous reproductions fixed. Detection is an additional affected reader, detailed below. |
| CLI file readers still bypass the reference grammar | Expanded 693-cell CLI matrix, configured-root route tests, directory and literal-content controls, and messenger research argument test | Previous malformed-input and unsupported-prefix reproductions fixed. Hashing's new directory pre-scan still bypasses the shared selection and probe rules. |

## Unblocked Findings

### High — Schema detection discards the opened document's resolution context

**Defect class:** a reader opens a source correctly but derives its authored references from an incompatible launch context, converting a context failure into a permissive result.

In darkmatter-cli, [run_detect](../../cli/src/commands/schema/detect.rs:28), which infers a schema from document frontmatter, captures the launch context and passes it to both detection branches. Its [document loader](../../cli/src/commands/schema/detect.rs:77) resolves the argument through `open_argument` but discards the opening reference and never obtains `OpenedArgument::document_context`. In the darkmatter library, [detect_from_document](../../lib/src/markdown/schemas/detect.rs:95) then uses ordinary `for_source` derivation. For an external document that derivation is invalid; even absolute and home-based values fail because the context itself is invalid. Detection deliberately turns resolution failures into `string`, so the command exits successfully with an incorrect schema.

This also happens inside the same repository when the document's absolute spelling uses a symlink alias different from the process CWD, such as macOS `/var` versus `/private/var`. The same file and values infer as `file` with the canonical spelling and as `string` with the alias.

**Reproduction:** use two temporary Git repositories, `launch` and `source`, following the existing cross-repository fixture. Put `source/docs/doc.md`, `source/docs/beside.md`, and `source/target.md` in the source repository, `launch/magic.md` in the launch repository, and `home/home.md` under a fixture home. Give the document these properties:

```yaml
relative: './beside.md'
bare: 'target.md'
repo: '&target.md'
scoped: '^target.md'
magic: '@magic.md'
home: '~/home.md'
absolute: '/fixture/source/target.md'
```

Run `md schema detect /fixture/source/docs/doc.md` from `launch`. All seven properties are reported as `string`; all seven should be `file`. Run from `source` with the canonical document path as the positive control: all seven are `file` when a source-side `magic.md` also exists. Repeat with two input documents, with and without `--merge`, and with an alias of the document's path. A repository-free document opened by either an absolute path or quoted `~/doc.md` from `launch` also misclassifies existing relative, home, magic, and absolute references.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| CLI detection, single input | Foreign repository; all seven forms above | All `string`, exit 0 | All `file` |
| CLI detection, multiple inputs without merging | Same document twice, foreign repository | Both emitted schemas contain all `string` | Each document uses its own context; all `file` |
| CLI detection, `--merge` | Same two inputs | All `string(required)` | All `file(required)` |
| All three CLI detection branches | Same-repository canonical spelling, then `/var` alias | Canonical: all `file`; alias: all `string` | Identical inference for the same file |
| CLI detection | Repository-free document opened by absolute and quoted home reference | All four existing references become `string` | Infer `file`, preserving the opening anchor |
| Library `detect_from_document`, `detect_schema`, `detect_schema_with_contexts`, and `DarkmatterSchemas::detect` | Same seven values; launch context versus a correctly prepared document context | Each reproduces all `string` with the incompatible launch context; each yields all `file` with the document context | CLI must supply the document context; libraries must remain passive and must not rediscover a repository |
| CLI schema validation and cleaning | Same external document, explicit schema declaring all seven properties as `file`; canonical and alias spelling | Validation succeeds; cleaning reports no diagnostics; clean | Keep source anchors and launch magic scope |
| CLI composition, graph, reference validation, and trigger inspection | Same external document and both spellings, with `::file &target.md` | All succeed; graph/composition select the source target; clean | Keep these results |
| Claudine effective-schema and authored-order completion readers | Permanent two-repository process cases, all three commands, bare/`&`/`^`/`./`/absolute/`@` schemas | Correct values and authored name order; clean | Keep these results |
| DMLS repository-context reader and reference projections | Repository-context and editor parity tests, including narrower workspace and invalidation | Pass; clean | Context intentionally belongs to the document's repository |

The detector census comprises the two CLI dispatch branches and all four public library detection surfaces; there are no DMLS or Claudine detection callers. The source-reader census additionally covers all six callers of `MdRequest::document_context`, both Claudine completion consumers, and DMLS's repository-owned context. Existing public library APIs already produce the correct result when supplied with a correctly prepared context; this finding does not ask them to perform discovery.

Retain one opening reference and derived context per loaded document, then use the existing `detect_schema_with_contexts` API for multiple documents. Handle equivalent source spellings consistently with the other readers and preserve home/environment opening provenance. Add process tests that assert inferred **types**, rather than only the property name identifying the opened route fixture. The current `MdRoute::SchemaDetect` observation confirms that the right document was opened but cannot detect this failure inside it.

### High — Hash directory detection overrides earlier files and suppresses I/O failures

**Defect class:** a reader independently probes a preferred filesystem type across the candidate list, bypassing earlier matches and terminal I/O failures from shared resolution.

In darkmatter-cli, [input_directory](../../cli/src/commands/hash.rs:224), which decides whether `md hash` hashes one document or a directory tree, searches the entire candidate plan for the first `Path::is_dir()` before ordinary file resolution runs. A file at an earlier candidate is skipped. The boolean probe also erases metadata errors, allowing a later directory to turn an earlier I/O failure into success.

**Reproduction:** copy the route fixture's Markdown document, with a distinguishing title and heading, into `source/docs/collision.md`. Create a **directory** `source/collision.md` containing another Markdown document. Launch from `source/docs`:

```text
md hash ./collision.md
md hash collision.md
md hash /fixture/source/collision.md
md hash --diff collision.md
md hash --save collision.md
```

The bare argument produces the directory's aggregate hash, equal to the third invocation and different from the first. The last two invocations reject directory mode instead of comparing or updating the nearer file. The same failure occurs with `@collision.md` when `source/docs` is a configured first magic root, and with `^collision.md` when the nearer file is at a recognized package root and the later directory is at the repository root.

For the error case, change only the first candidate into a self-referencing symlink `docs/loop.md`, with a later repository directory `loop.md`. `md hash loop.md` succeeds and hashes the directory; forced-file code-block input and every ordinary file reader return `failure: io` for the first candidate.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Hash single-document/directory dispatch | Bare reference; first candidate file, later directory | Hashes later directory | Hash the earlier file |
| Hash dispatch, magic and package-scoped forms | Configured first magic root or recognized package has file; repository has directory | Hashes later directory; forced-file code-block control reads earlier file | Preserve root order and earlier file identity |
| Hash `--diff` and `--save` | Bare collision case | Rejects unsupported directory options | Apply the option to the earlier document |
| Hash directory pre-scan | First candidate self-referencing symlink, later directory | Success and directory hash | Stop with `ResolutionFailure::Io` |
| Hash explicit-relative and absolute controls | Explicitly name the earlier file | Correct file hash; diff reports missing baseline; save writes initial baseline | Keep these results |
| Hash directory controls | Explicitly name the later directory; existing `&area` directory test | Correct aggregate hash | Keep directory support |
| Render, compose, clean, TOC, frontmatter get/set/rm, both delta slots, graph, edit, reference validation, schema validate/detect/triggers | Same bare collision and first-candidate symlink error | Select earlier file; error case fails with I/O provenance; clean | Keep shared ordering and error behavior |
| Code-block forced file and default branches | Same file/directory collision and symlink error | Both read earlier file; forced file reports I/O; default renders erroring input literally | Keep documented default literal behavior |
| Shared `open_argument`/`resolve_file_path`, and edit's separate creation branch | Same collision/error process observations | Earlier file wins; I/O stops before creation or editor launch; clean | Keep these results |

The sweep covers every one of the 18 `MdRoute` variants, both code-block branches, edit's separate resolution/creation decision, the hash directory decision, and all hash modes affected by it. Single-candidate `&`, home, and absolute references have no competing-root ordering to lose; their positive resolution controls are in the passing matrix. Recursive references skip the directory pre-scan and retain shared file resolution. Output paths and configured directory settings are not source-reference readers.

Choose file-versus-directory mode without scanning past an earlier file or swallowing a metadata failure. Use fallible probing in candidate order, preserving the shared failure classes. Add first-file/later-directory cases for bare, magic, and package-scoped roots, plus an earlier I/O failure. Review the new directory-selection documentation/comment when correcting this behavior: “first candidate that is a directory” currently describes the implementation but omits the precedence contract it violates.

## Blocked Findings

None. Both fixes can proceed without additional permissions, unavailable hardware, or a new design decision. The human completeness check is separate.

## Recurrence

The first finding continues **review-3.md — “External-source contexts lose repository or launch magic anchors.”** That fix swept `MdRequest::document_context` and its existing callers, but did not include detection, whose newly corrected argument reader discards the opened context afterward. The complete detection and source-reader census is above, including clean siblings.

The second finding continues the shared-resolution bypass class in **review-3.md — “CLI file readers still bypass the reference grammar”** and **review-2.md — “Claudine schema completion still resolves committed prompt paths by joining directories.”** Hashing now parses the grammar but makes a separate raw filesystem decision before resolution; the previous sweep should have checked the new directory branch's selection and probe semantics, not only whether its argument parsed. The complete CLI reader census and competing-type/error results are above.

Both also expose the recurring verification problem from **review-1.md — “The parity matrix omits required cases and changes a required failure”** and **review-2.md — “The parity matrix substitutes composition for completion and omits configured CLI roots.”** Actual completion and route-specific opening assertions are now present, which is an improvement. The remaining missing observations are inferred file types after opening, and root precedence when candidates have different filesystem types or an earlier probe fails. `recurrence: true` requests a human completeness check before another automated cycle.

## Verification and requirement coverage

Level 1 is appropriate for this fix's filesystem, process-completion, and language-server protocol behavior. No changed requirement concerns physical keyboard input, terminal geometry or styling, mouse input, or browser interaction; Level 2 and Level 3 are not required for these findings. No terminal or browser window was opened or focused. Edit probes used `/usr/bin/true` as both editor settings.

| Requirement | Strongest verification performed here | Result |
|---|---|---|
| Prepared context, preflight/pipeline agreement, package roots, directives, fallback chains, literal `#`, schema file values | Level 1 shared library/CLI/editor parity tests | Pass for the permanent cases |
| CLI grammar, malformed introducers, escapes, configured roots, route identity | Level 1 693-cell CLI argument matrix and magic-root process tests | Pass; added competing-type/error probes expose hash defect |
| External-source repository anchors and launch magic scope | Level 1 two-repository process and public API tests, plus detector sweep | Prior cases pass; detection fails |
| Actual Claudine completion, all three commands, emitted-token reuse and authored order | Level 1 completion and parity process tests | Pass |
| Builder validation, shared environment snapshot, tracing, nested request reuse | Level 1 builder/epoch and construction guards | Pass |
| DMLS repository reuse, watched/rescan/configuration invalidation, failure recovery, untitled buffers | Level 1 repository-context tests and editor parity tests | Pass |
| Messenger research document arguments | Level 1 normal CLI invocation | Pass |
| Test compilation and running tier selection | Declared L1 targets/modules, enabled metadata features, area tier-coverage checks | All three area checks pass; no stranded tiers |

Successful targeted runs: Darkmatter `just test entry_point_parity` (20), `just test magic_root` (10), `just test context_construction_guard` (4), `just test request_context` (22), and `just test repository_contexts` (11); Claudine `just test entry_point_parity` (3) and `just test completion_committed_prompt_schema` (6); Messenger `just test validate_resolves_document_arguments_through_the_reference_grammar` (1). `just check-tier-coverage` passes for darkmatter, claudine, and messenger. These are targeted results, not a full-suite or lint claim.

A temporary Level 1 public API probe verified every detector surface with the incompatible launch context and the correctly prepared document context. Its successful assertions document the observed bad result and the positive control; they do not establish production acceptance. The first probe compilation failed because my temporary test omitted imports; I corrected them, reran successfully, and restored the original test source byte-for-byte. A tier listing overlapping that failed compilation was rerun successfully after restoration.

The configuration-field robustness matrix is not applicable: this fix changes reference contexts and argument resolution, not a file format's field-presence, null, duplicate-key, or deserialization rules. The review tests reference forms, valid and invalid contexts, aliases, competing candidate types, and terminal filesystem errors through public results. Existing frontmatter parsing is not being silently certified by this review.

The two ambient-state sites explicitly assigned to the subsequent glob work remain outside this review. Messenger messaging attachments and DMLS server configuration paths are outside this specification's research/document-reference surface. The known unrelated link error snapshot, cross-OS receipts, and the external human review process do not determine readiness here.

Process fixtures and observations are retained under `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/file-refs-review4-_jq4_xg5`; the temporary detector probe is retained at `/tmp/file-refs-review4-detection-probe.rs`. Review output consists only of this review and the requested previous-review/spec metadata updates. No implementation changes or commits were made.
