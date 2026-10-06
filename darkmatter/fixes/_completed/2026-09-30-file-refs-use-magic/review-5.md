---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - |-
        Complete the reference-test completeness check requested in the earlier reviews. Compare the reader tables in this review with the permanent tests: each command must exercise its own reader, assert which target it selects, and check references inside documents opened from another repository. The automated checks now pass, including the additional collision and filesystem-error sweep across all 18 md readers. This preserves the earlier request, whose completion is not recorded in the implementation log; it asks for no new design decision and blocks no implementation finding.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-02T11:27:59-07:00
spec: 2026-09-30-file-refs-use-magic/spec.md
implemented: false
description: "A **fix** review of `2026-09-30-file-refs-use-magic/spec.md`"
fix: 2026-09-30-file-refs-use-magic/review-5.md
previous: 2026-09-30-file-refs-use-magic/review-4.md
---

# Review 5

This fix is **production ready within its specified scope**. Both unblocked findings from review-4.md are implemented, their original reproductions pass, and the additional reader sweep found no remaining instance. The earlier request for a human completeness check remains a separate process item; it does not change this implementation-readiness result.

## Previous findings

Review-4.md contained two unblocked findings and no blocked findings. No blocked implementation finding was subsequently unblocked. The implementation log records both fixes and their additional affected readers; it does not record completion of the separate human check.

| Previous finding | Implementation and verification | Status |
|---|---|---|
| Schema detection discards the opened document's resolution context | Each opened document retains its opening reference, canonical path, and derived context. All three detection modes use the per-document contexts. Process tests assert inferred types for all seven reference forms, external repositories, repository-free documents, home openings, and symlink aliases. | Fixed |
| Hash directory detection overrides earlier files and suppresses I/O failures | Hashing selects a file or directory from the shared detailed resolution result, then reuses that selection for ordinary hashing, comparison, and saving. Regression tests cover bare, magic, and package-scoped collisions and an earlier symlink error. The implementation also corrected directory lookup in `find_files` and the editor's later-buffer fallback. | Fixed |

## Unblocked Findings

None.

## Blocked Findings

None. The retained human completeness check is not an unresolved implementation finding.

## Reader sweeps

### Source context after opening a document

**Class checked:** a reader opens the right document but interprets references inside it using incompatible launch anchors, silently changing its result.

In darkmatter-cli, [run_detect](../../cli/src/commands/schema/detect.rs) infers frontmatter types. It now loads the canonical document and obtains its context through [MdRequest::document_context](../../cli/src/request.rs), which preserves the source repository and the launch magic scope. The single-input, separate-output, and merged-output paths all call darkmatter's [detect_schema_with_contexts](../../lib/src/markdown/schemas/detect.rs) with the corresponding contexts. This matters because opening the right document alone cannot establish that references inside it mean the right thing.

The shared external-source fixture supplies existing relative, bare, repository-root, package-scoped, magic, home, and absolute references. The process tests compare source-repository and foreign-repository launches, canonical and alias spellings, and all three detection modes. Repository-free controls also open the document by absolute and quoted home reference. The public-library test checks all four detection surfaces with compatible and incompatible contexts.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| CLI detection: single input | Seven forms; source and foreign launches; canonical and alias paths | Every existing reference infers `file` | Same inferred types regardless of equivalent source spelling |
| CLI detection: multiple inputs without merging | Same fixture and spellings, two inputs | Each output infers `file` | Use the context belonging to each input |
| CLI detection: merged inputs | Same fixture and spellings, two inputs | Existing shared references infer `file(required)` | Merge the independently detected shapes |
| CLI detection: repository-free document | Absolute and quoted home opening; relative, magic, home, absolute, and home-parent references | Existing references infer `file` | Preserve the opening anchor and launch magic scope |
| Library detection: `detect_from_document`, `detect_schema`, `detect_schema_with_contexts`, `DarkmatterSchemas::detect` | Same seven string values with a prepared document context and incompatible launch context | `file` with the document context; `string` with the incompatible context | Passive detection uses the supplied context; CLI supplies the compatible one |
| CLI composition, graph, and reference validation | External document with distinguishable source and launch magic targets | Launch magic target retained; source repository anchors retained | Preserve both independent anchors |
| CLI cleaning, schema validation, and trigger inspection | External document and referenced schemas | Permanent external-source tests pass; readers retain the opening reference and document context | Interpret authored references in the source context |
| Claudine completion: `compose`, `inline-compose`, `sequence`; effective-schema and authored-order readers | Foreign prompt, all supported schema-reference forms, name and value suggestions | Correct suggestions and authored order | Resolve the prompt's schema in its source context while retaining launch magic |
| Claudine composition | Foreign prompt and disjoint magic targets | Correct source schema and launch magic selection | Keep composition and completion consistent |
| DMLS document contexts and projections | Repository-owned context, narrower workspace, repository-free document | Protocol tests pass | Editor request intentionally belongs to the document's repository |
| Render assets; TOC; frontmatter get/set/remove; hash; both delta inputs; code-block; edit | Reader census and CLI argument matrix | Assets use the opened file's directory; other routes do not evaluate authored file references | No downstream launch-context interpretation to repair |

The census includes every detection caller, every CLI document-context consumer, both Claudine completion readers, and DMLS's repository-owned context. There is no additional command-specific detection reader in Claudine or DMLS. The permanent detection test uses two copies of a document for its multiple-input controls; independent source/context pairing is also directly visible in the loader and dispatch code, rather than inferred from that duplicate-input test alone.

### Candidate order and filesystem failures

**Class checked:** a reader independently favors a later filesystem type or buffer, overriding an earlier match or turning a terminal filesystem error into success.

In darkmatter, [resolve_entry_in_context](../../lib/src/markdown/fs.rs) supplies the file-or-directory selection used by darkmatter-cli hashing and expression directory lookup. It consumes the shared resolver's ordered probe results and fallibly checks earlier non-file entries for directories. Recursive references retain file-only resolution. In DMLS, [locate](../../dmls/src/graph/arena.rs) now consults a later indexed buffer only after a genuine `NoMatch`, preserving an earlier `Io` failure.

Beyond the permanent regression tests, a temporary Level-1 probe reused the existing CLI fixture and route-specific observations. For **each of all 18 routes**, it placed a nearer `beside.md` file in the package and a later `beside.md/` directory at the repository root. It then changed only the nearer entry to a self-referencing symlink. All 36 observations matched the expected selected file or typed error. A fresh fixture was used for every route and shape, including mutating routes, and edit used a no-op editor.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Render, compose, clean, TOC | Nearer file/later directory; nearer symlink error/later directory | Earlier file selected; `io` on failed probe | Shared ordering and terminal failure |
| Frontmatter get, set, remove; edit | Same two shapes, fresh fixture per invocation | Earlier file selected; `io` before mutation or editor launch | Never act on the later target after an earlier error |
| Hash | Same two shapes; permanent bare, magic, and package-scoped cases | Earlier file hash; `io` on failed probe | First existing file or directory decides |
| Hash comparison and save | Permanent nearer-file/later-directory fixture | Missing baseline exits 2; save updates only the nearer file; subsequent comparison exits 0 | Reuse the selected file in each mode |
| Both delta input positions; graph | Same two shapes | Earlier file selected; `io` on failed probe | Resolve both input positions consistently |
| Reference validation; schema validate, detect, triggers | Same two shapes | Earlier document selected; `io` on failed probe | No independent raw-path or directory preference |
| Code-block forced file | Same two shapes | Earlier file read; `io` on failed probe | Preserve forced-file failure |
| Code-block default | Same two shapes | Earlier file read; erroring argument rendered literally | Preserve documented default literal fallback |
| Hash explicit-relative, absolute, and repository-root directory controls | Explicit earlier file or later directory | Correct individual or aggregate hash | Keep directory support without changing ordered selection |
| Expression `find_files` directory lookup | Earlier file/later directory; earlier symlink error/later directory; missing earlier candidate | Empty result for the earlier file; typed `Io` for failed probe; later directory searched after absence | First existing entry decides whether there is a directory to search |
| DMLS graph later-buffer fallback and unresolved diagnostic | Earlier symlink error, later unsaved indexed buffer | Unresolved graph edge and diagnostic with `Io` | Later buffer cannot answer an earlier failed probe |
| Shared file argument opening; edit creation; expression path-shape resolution; link absolutization; Claudine effect paths and committed prompts; messenger research arguments | Source census plus applicable process/API tests | Shared file resolution occurs before any clean-miss fallback | Preserve typed failures and reference grammar |
| Schema lazy file binding; DMLS candidate planning | Source census | Deliberately unprobed candidate plans | Planning does not claim a filesystem match |
| Resolved-file checks in file existence, TOC targets, file trees, and transclusion source directories | Source census and passing Level-1 suite | Checks operate on an already selected path | No second candidate-selection pass |
| Magic-root settings; completion directory enumeration; skill roots; trigger discovery; schema suggestions; DMLS configuration/topic lookup; file-match absolute checks | Source census | Settings, listings, suggestions, or single-path checks | These do not select a competing source-reference candidate |

The candidate-search census also checked `file_links` directory discovery: its directory option is a single source-relative target, and its separate ambient boundary lookup remains explicitly assigned to the following glob work. The changed `find_files` directory precedence is documented in the expression topic page and recorded as an implementation departure in the log.

## Recurrence

No new finding repeats an earlier class; `recurrence: false`.

I compared this review with every earlier review in the directory. The first review's migration, editor-index, directive-chain, preflight-consumer, and parity omissions have passing applicable tests. The second review's actual completion and configured-root gaps are covered by real completion invocations and CLI root flags. The third review's external-source anchors and argument-grammar failures pass the expanded process matrix. The fourth review's detection-context and ordered-probing failures pass their regressions and the sibling checks above.

The human completeness request originated in recurring findings in the earlier reviews. It remains recorded because its completion is not documented, not because this review found another recurring defect.

## Verification and requirement coverage

Level 1 is appropriate for every changed user-facing requirement: filesystem selection, command output, completion's process protocol, and language-server responses. This fix adds no physical keyboard, terminal-layout, styling, scrolling, mouse, paste, or browser requirement needing Level 2 or Level 3. No terminal or browser window was opened or focused.

| Requirement | Appropriate verification and evidence | Result |
|---|---|---|
| Original preflight repository references and package catalog | Level 1 CLI and library parity, including file/code/TOC consumers | Pass |
| Required context; one builder; binary-only capture; explicit ambient-state allowances | Level 1 declared construction guards and source census | Darkmatter guards pass; relevant downstream tests compile |
| Context validation, snapshot environment, tracing, nested request reuse | Level 1 request-context builder and epoch tests | Pass in the full Darkmatter run |
| Every reference form, tree boundary, source-opening provenance, configured magic roots | Level 1 shared process/API matrix | Pass |
| Document detection in foreign repositories and equivalent path spellings | Level 1 type assertions through the CLI and public detection APIs | Pass |
| Hash selected target, comparison/save behavior, terminal probe errors | Level 1 process regressions and 18-route observation probe | Pass |
| Editor schema, links, graph, definition, and actions | Level 1 protocol/provider parity and graph tests | Pass |
| Editor per-repository reuse, watcher/rescan/configuration invalidation, failure recovery, untitled buffers | Level 1 repository-context protocol tests | All 11 pass in the full run |
| Claudine composition and actual completion; emitted-token reuse; schema order | Level 1 process parity and committed-prompt completion tests | 3 parity and 6 completion tests pass |
| Messenger research document arguments | Level 1 normal CLI invocation | Pass |
| Declared targets, enabled features, running tiers, shared-source declarations | Cargo metadata, `l1/main.rs` module lists, observed nextest selection, area tier checks | No stranded tiers in darkmatter, claudine, or messenger |

Runs performed for this review:

- Darkmatter `just test entry_point_parity`: 23 passed.
- Darkmatter `just test hash_entry_precedence`: 4 passed.
- Darkmatter `just test context_construction_guard`: 4 passed.
- Darkmatter `just lint`: passes, including the Zed extension compile check.
- Darkmatter `just test --no-fail-fast`: 8,842 run, 8,841 passed, one failed, 12 skipped. The failure is the previously recorded `error_snapshots::link::unrecognized_format_mentions_html_and_markdown`, an unchanged rendering snapshot outside this fix's reference behavior. This is **not** a claim that the full suite is green.
- Temporary CLI sibling probe through `just test review5_every_cli_reader`: one test, 36 process observations, all expected results. An initial fixture setup incorrectly assumed a root-level `beside.md` file existed and failed before invoking the CLI; I corrected the setup and reran. The original test source was restored byte-for-byte after both attempts.
- Claudine `just test entry_point_parity`: 3 passed; `just test completion_committed_prompt_schema`: 6 passed.
- Messenger `just test validate_resolves_document_arguments_through_the_reference_grammar`: one passed.
- Root `just check-tier-coverage` for darkmatter, claudine, and messenger: all pass, zero stranded tests.

An initial attempt to combine test names with `|` was interpreted by the recipe's shell. The filters were rerun individually; no result from that failed command is counted as evidence.

The input-field robustness matrix does not apply: this fix changes reference resolution and context ownership, not format fields, deserialization, duplicate-key rules, or configuration-field defaults. The review checks valid, malformed, missing, escaping, external-source, alias, competing-type, and filesystem-error inputs through public results. Existing frontmatter and manifest parsing are outside this certification.

The two ambient-state sites explicitly assigned to `2026-09-30-glob-reference` remain outside this fix's readiness decision. Its same-branch merge requirement remains in force. Cross-OS receipts, branch publication, and the separate human check are not used to decide implementation readiness here.

The temporary CLI probe source and successful output are retained at `/tmp/file-refs-review5-cli-probe.rs` and `/tmp/file-refs-review5-cli-probe.log`; downstream test logs are `/tmp/file-refs-review5-claudine-tests.log`, `/tmp/file-refs-review5-completion-tests.log`, and `/tmp/file-refs-review5-messenger-tests.log`. Review output is this file and the requested previous-review/spec metadata. No production code, permanent tests, commits, or lifecycle-directory moves were made.
