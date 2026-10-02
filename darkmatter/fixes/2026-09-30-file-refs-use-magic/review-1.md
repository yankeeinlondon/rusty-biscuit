---
$schema: feature-review.yaml
ready: false
findings:
    - title: Claudine still uses the removed context APIs and cannot compile
      priority: critical
    - title: Editor Markdown links reject existing targets outside the workspace index
      priority: high
    - title: Editor directive resolution treats fallback chains as one filename
      priority: high
    - title: Preflight does not resolve two required directive consumers
      priority: high
    - title: The parity matrix omits required cases and changes a required failure
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-01T20:49:56-07:00
spec: 2026-09-30-file-refs-use-magic/spec.md
implemented: false
description: "A **fix** review of `2026-09-30-file-refs-use-magic/spec.md`"
fix: 2026-09-30-file-refs-use-magic/review-1.md
---

# Review 1

This fix is **not production ready**. The shared request design fixes the original preflight regression, but Claudine cannot compile, several editor features still disagree about valid references, and the required parity contract is not fully enforced. These are implementation and testing issues within the agreed scope; none requires a new human design decision to start fixing it.

## Findings

### Critical — Claudine still uses the removed context APIs and cannot compile

**Defect class:** a required downstream consumer was not migrated when its dependency removed the old context APIs.

`cargo check --color=never -p claudine -p claudine-cli` fails with 22 library errors. The CLI cannot reach its own compilation because it depends on that library. For example, Darkmatter's [ComposeRequest](../../lib/src/markdown/compose/context/request.rs) now carries the required resolution context, but Claudine still passes `ComposeOptions` into composition and preflight. This prevents shipping the entire branch, not just Claudine's magic references. Acceptance criteria 2–5 and 10 remain incomplete for Claudine.

The sweep covered every production construction and old API caller in both Claudine packages, rather than just the first compiler error:

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Claudine library: [composition preparation](../../../claudine/lib/src/composition/prepare.rs), [preflight](../../../claudine/lib/src/composition/preflight.rs), [composition expression setup](../../../claudine/lib/src/composition/mod.rs), [lifecycle execution](../../../claudine/lib/src/composition/lifecycle/executor.rs) | Compile against the new required request API | Removed setters and incompatible arguments; compilation fails | Pass one prepared request through each caller |
| Claudine library: [schema loading](../../../claudine/lib/src/composition/schema/mod.rs), [supplied schemas](../../../claudine/lib/src/composition/schema/supplied.rs), [sequence schema checks](../../../claudine/lib/src/composition/sequence/formal.rs) | Same compile | Context-free schema constructors fail | Use the invocation's built or derived context |
| Claudine library: [loop expressions](../../../claudine/lib/src/composition/looping/expression.rs), [sequence expressions](../../../claudine/lib/src/composition/sequence/expr.rs), [dispatch expressions](../../../claudine/lib/src/dispatch/expression.rs) | Same compile | Directory-only resolution contexts and missing environment arguments fail | Expressions and references use the same snapshot |
| Claudine library: [system prompt preparation](../../../claudine/lib/src/system_prompt/prepare.rs), [shell harness](../../../claudine/lib/src/harness/shell.rs) | Same compile | Old request setter/composition call and missing policy context fail | Pass the required context |
| Claudine library: [invocation context](../../../claudine/lib/src/invocation_context.rs), [composition resolution](../../../claudine/lib/src/composition/resolve.rs), [sequence sources](../../../claudine/lib/src/composition/sequence/source.rs), [harness resolution](../../../claudine/lib/src/harness/resolve.rs), [system prompt resolution](../../../claudine/lib/src/system_prompt/resolve.rs) | Constructor census accompanying the failed compile | Still construct contexts directly with `new` or `from_snapshot` | Build through Darkmatter's shared builder; retain approved derivations |
| Claudine CLI: [completion scopes](../../../claudine/cli/src/completion/scopes.rs), [schema completion](../../../claudine/cli/src/completion/schema_completion/mod.rs), [signal schemas](../../../claudine/cli/src/commands/signals.rs), [wrap overlay](../../../claudine/cli/src/commands/wrap/overlay.rs), [wrap sequences](../../../claudine/cli/src/commands/wrap/sequence/mod.rs) | Same package build and source census | Library failure prevents execution; old construction/caller forms remain | Migrate these callers too |
| Claudine context holders in preparation, types, preflight, looping, lifecycle, sequence, and CLI wrap/compose orchestration | Optional-context census | Optional contexts remain; neither Claudine package has the required context guard or parity runner | Required context throughout; live guard tests and both Claudine parity entry points |
| Darkmatter library/CLI, DMLS, claudine-gen, messenger library/CLI | Their declared context guards | Passing guards; clean for the migration class | Keep passing |

Complete Claudine's already specified migration, including its process snapshot, extra prompt roots, guards, parity runner, and documentation. The existing spec frontmatter asks the author to schedule this work, but the implementation contract already requires it; this review does not treat scheduling as a blocked finding.

### High — Editor Markdown links reject existing targets outside the workspace index

**Defect class:** reference existence is decided by membership in an editor index instead of the shared file resolver's answer.

In DMLS, [ReferenceResolver](../../dmls/src/graph/arena.rs) selects only candidates present in `by_path`. Its sibling [diagnose_unresolved](../../dmls/src/graph/arena.rs) likewise reports `NoMatch` when an existing target is not indexed. DMLS's [definition and document-link providers](../../dmls/src/providers/definition.rs) then inherit that result. The shared context is present, but these features still disagree with composition and the filesystem-based editor providers. This violates the promised editor parity in acceptance criteria 9 and 10.

**Reproduction:** copy the existing repository-context fixture, put `target.md` at the repository root and `doc.md` in `docs/`, then initialize the editor with only `repo/docs/` as its workspace folder. Put the same reference into a `file(eager)` property, each of the three single-file directives, and a Markdown link. Repeat with `&target.md`, `^target.md`, `@target.md`, `~/target.md`, bare `target.md`, and `../target.md`; supply the home and extra magic root through the snapshot. All six forms reproduced the same split. Opening the whole repository is the positive control.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| DMLS Markdown-link graph and broken-link diagnostic | Each of the six forms; target exists above the workspace folder | Unresolved reference edge; `dm.links.broken_path` with `NoMatch` | Recognize the existing resolved file |
| DMLS Markdown-link document links | Same forms and fixture | No target link | Link to `repo/target.md` |
| DMLS Markdown-link go-to-definition | Same forms and fixture | Empty locations | Navigate to `repo/target.md` |
| DMLS directive links and definition: `::file`, `::code`, `::toc-linking` | Same forms and fixture | Correct target, no broken-target diagnostic; clean | Same target |
| DMLS schema file validation, links, and definition | Same forms and fixture | Correct target, no file-validation problem; clean | Same target |
| DMLS create-file code action | Same forms, using the erroneous broken-link diagnostic | No create action because the filesystem resolver sees the file; clean | Never offer to overwrite the existing target |
| Darkmatter composition | Same forms and fixture | Resolves the target successfully; clean | Same target |
| All tested editor navigation surfaces | Whole-repository workspace control | Correct target; clean | Same target |

Use the shared resolution result for file navigation and existence diagnostics even when the destination is outside the indexed documents. Preserve the index for heading lookup and relationships, but do not turn an unindexed existing file into a missing file. Add the narrower-workspace fixture to the parity tests: the current DMLS runner indexes the entire fixture root, including its home and external targets, so it cannot detect this defect.

### High — Editor directive resolution treats fallback chains as one filename

**Defect class:** editor projections bypass the directive's target grammar and resolve its compound target as a single file reference.

Darkmatter's [table-of-contents target parser](../../lib/src/markdown/compose/toc_linking/parser.rs) supports quoted alternatives separated by `|` and a terminal `false` that suppresses output when nothing matches. DMLS's [DSL provider](../../dmls/src/providers/dsl.rs) instead feeds the entire target to file resolution in hover, definition, document links, and diagnostics. The spec explicitly includes each alternative in the directive-target contract. Valid documents therefore receive false warnings and lose navigation.

**Reproduction:** in a temporary repository containing `target.md`, open a document with each quoted target below. The same public composition call supplies the expected behavior. Single-target `::file` and `::code` are sibling controls.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| DMLS directive diagnostics | `::toc-linking "&missing.md \| &target.md"`; reversed order | `NoMatch` for the whole chain | No broken-target warning; choose the existing alternative |
| DMLS directive links and definition | Both alternative orders | Empty links and locations | Navigate to `target.md` |
| DMLS directive hover | Both alternative orders | Says the compound target was not found | Show the chosen existing target |
| DMLS directive diagnostics and hover | `::toc-linking "&missing.md \| false"` | Broken-target warning and missing-target hover | Recognize intentional suppression; no broken-target warning |
| DMLS directive links, definition, and code actions | Suppressed chain | Empty; clean | No file target or create-file action |
| DMLS code actions | Both successful alternative orders | Empty; clean | No create-file action |
| Darkmatter composition | Both alternative orders; suppressed chain | Produces the target's table of contents; suppressed chain produces no output or warnings; clean | Same behavior |
| DMLS diagnostics, links, definition, hover, and code actions | Single quoted `&target.md` for `::file`, `::code`, and `::toc-linking` | Correct navigation/hover, no warning or create action; clean | Same behavior |

Share the passive target-chain parsing and selection rules across these editor projections. Preserve alternative spans for navigation and do not parse `false` as a filename. Add successful fallback, first-alternative success, and suppressed-chain cases to the shared tests.

### High — Preflight does not resolve two required directive consumers

**Defect class:** a public entry point omits required reference consumers and returns success without checking their targets.

Darkmatter's [compose_preflight](../../lib/src/markdown/compose/preflight/mod.rs) is required by the spec's Table 1 to resolve `::file`, `::code`, and `::toc-linking` targets. Its [collector](../../lib/src/markdown/compose/preflight/collect.rs) does not resolve the latter two. The [matrix rows](../../lib/tests/common/entry_point_parity/mod.rs) omit those consumers, hiding the difference rather than verifying the specified contract.

**Reproduction:** use the existing parity repository and the same prepared request, switching only the directive keyword and target between `&root-only.md` and `&missing.md`.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Darkmatter preflight, `::file` | Existing target; missing target | Resolved file edge; typed `NoMatch`; clean | Same results |
| Darkmatter preflight, `::code` | Existing target; missing target | Success with no target edges for both | Resolve the existing file; report `NoMatch` for the missing file |
| Darkmatter preflight, `::toc-linking` | Existing target; missing target | Success with no target edges for both | Resolve the existing file; report `NoMatch` for the missing file |
| Compose pipeline, schema validation, and CLI composition | Applicable existing matrix consumers | Existing parity tests pass; clean for those tested cells | Keep their required results |

Preflight need not recursively inspect code or table-of-contents output for shell commands to verify a target. Add passive target resolution without treating those outputs as Markdown children, and restore their required matrix rows.

### Medium — The parity matrix omits required cases and changes a required failure

**Defect class:** verification substitutes narrower coverage or more permissive expected results for the agreed reference contract.

The [shared matrix](../../lib/tests/common/entry_point_parity/mod.rs) and its [CLI runner](../../cli/tests/l1/entry_point_parity.rs) can pass while required cells never run or assert a different result. This is separate from missing cross-OS evidence: several cells are deliberately excluded even when the Windows suite runs.

| Site | Shape tested or inspected | Observed result | Expected result |
|---|---|---|---|
| CLI argument runner, repository-root and nested-package launches | Caller `../` references escaping the repository; both executed by the passing matrix | Resolves `outside.md`; the expected-value table asserts success | Acceptance criterion 10 explicitly requires `InvalidReference` for the tree-escape row at every entry point |
| Compose, preflight, schema, CLI, and DMLS document rows | Document-authored tree escapes | Existing rows assert `InvalidReference` and pass; clean | Retain that failure class |
| CLI runner on Windows | `@`, `~`, and home-opened documents | Explicitly drops 38 cells | Exercise the CLI process path for the supported forms, using safe temporary fixtures beneath the actual home when a process HOME override is unavailable |
| Shared magic-reference fixture across all runners | `@magic-doc.md` | Target is supplied by a home tier, not an extra configured magic root | Also exercise a snapshot-configured root across the required entry points |
| Darkmatter builder test and DMLS repository-context test | Extra configured magic root | Positive public-result tests exist and pass; clean within their own surfaces | Retain them and extend coverage to the remaining surfaces |
| DMLS shared matrix | Link graph and code-action consumer rows | Graph covers Markdown links only; code actions cover `::file` and Markdown links only | Account explicitly for every consumer required by the spec; mark unsupported actions as assertions of no action rather than silently omitting them |
| Claudine runners | Composition and completion variants | Rows exist in the shared enum, but no executable runner exists | Covered by the Claudine migration finding |

The implementation log acknowledges the caller-supplied escape departure; recording it does not satisfy the unchanged acceptance criterion. Restore the specified expectation and behavior. If the author later chooses a different contract, that must be an explicit contract change rather than a passing test that quietly asserts the opposite result. Expand the matrix to cover configured roots and remove platform-wide row omissions.

## Verification and requirement coverage

All behavior in this fix concerns filesystem resolution, composition results, command output, and language-server protocol results. **Level 1 is appropriate**: no requirement adds terminal input encoding, interactive keybindings, or terminal layout behavior requiring Level 2 or Level 3. The editor probes ran through real provider dispatch over the existing in-memory protocol fixture, not through a terminal or GUI, and stole no focus.

| Requirement | Strongest relevant verification inspected/run | Result |
|---|---|---|
| Original repository-reference regression and package-root lookup | Level 1 CLI process and parity fixtures | Existing matrix passes |
| Required context, shared builder, and permitted process-state reads | Level 1 source guards | Darkmatter, CLI, DMLS, generator, and messenger guards pass; Claudine guards absent |
| Invalid context rejection, environment consistency, and extra magic roots | Level 1 public builder/composition tests | Request-context tests pass |
| Shared context across nested sources and preflight | Level 1 request-epoch and parity tests | Existing tests pass; missing directive consumers reproduced above |
| Editor per-repository reuse, manifest/configuration invalidation, failed-context recovery, and untitled buffers | Level 1 protocol tests | All 11 existing repository-context tests pass |
| Editor navigation parity | Level 1 protocol matrix plus review probes | Existing matrix passes; narrower-workspace and target-chain defects reproduced |
| Every reference form and caller launch directory | Level 1 shared matrix | Incomplete as described above |
| Test compilation and active tier selection | Declared `l1` binaries, their `mod` lists, feature metadata, and `just check-tier-coverage darkmatter` | No stranded Darkmatter tier tests; Claudine's required new tests do not exist |

Commands/results from this review:

- `cargo check --color=never -p claudine -p claudine-cli`: failed with 22 Claudine library errors.
- `just test entry_point_parity` in Darkmatter: 4 passed.
- `just test repository_contexts` in Darkmatter: 11 passed.
- `just test request_context` in Darkmatter: 22 passed.
- `just test context_construction_guard` in Darkmatter: 4 passed.
- `just test production_source_builds_contexts_only_through_the_builder` in messenger: 2 passed.
- The root generator test invocation ran its complete Level 1 suite: 195 passed, including its guard.
- `just check-tier-coverage darkmatter`: no stranded tests.
- Three temporary Level 1 observation probes exercised the omitted shapes through public APIs and protocol requests. Their observations, rather than their pass counts, establish the findings. Probe sources and output were retained under `/tmp/file-refs-review-*` and `/tmp/*-file-refs-review-probes.rs` on the reviewing host.

The input robustness matrix does not apply to this change: it changes request-context ownership and reference resolution, not the fields or acceptance rules of a file-format/configuration parser. The sniff manifest changes centralize filename recognition; they do not change manifest deserialization. Existing permissive parser code was not treated as a new defect of this fix.

The two ambient-state sites expressly handed to the following glob-reference implementation are not new findings here. Cross-OS receipts and human review were not used to decide readiness. The previously documented bare-name `dirname` behavior was not introduced by this fix and is not a readiness blocker for it.

## Workspace note

The branch advanced during review from `875429e43` through `cdf1cbda3` to `b436c6b7d` as another process committed changes. Those commits briefly captured temporary observation probes while they were running. This reviewer made no commits and removed only those probes afterward; the concurrent process subsequently incorporated that cleanup and the iteration update. No temporary probe changes remain in the working tree. The review and `review_iterations: 1` update are the intended review artifacts. Existing implementation changes were preserved.
