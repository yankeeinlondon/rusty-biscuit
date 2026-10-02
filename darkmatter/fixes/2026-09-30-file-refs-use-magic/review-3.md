---
$schema: feature-review.yaml
ready: false
findings:
    - title: External-source contexts lose repository or launch magic anchors
      priority: high
    - title: CLI file readers still bypass the reference grammar
      priority: high
human_review: true
human_review_items:
    - |-
        Review the completeness of the reference-reader sweep before restarting the automated fix cycle. Compare the CLI routes and source-context consumers listed below with the actual test invocations, including malformed references and documents in a second repository. Confirm that every reader uses the shared reference contract and that a passing matrix cannot omit a failing route. The grammar-bypass defect repeats review #2 in additional readers. No new design decision or permission is needed to implement either finding.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-02T02:10:09-07:00
spec: 2026-09-30-file-refs-use-magic/spec.md
implemented: true
implemented_by: claude/opus
log: darkmatter/fixes/2026-09-30-file-refs-use-magic/log.md
description: "A **fix** review of `2026-09-30-file-refs-use-magic/spec.md`"
fix: 2026-09-30-file-refs-use-magic/review-3.md
previous: 2026-09-30-file-refs-use-magic/review-2.md
next: 2026-09-30-file-refs-use-magic/review-4.md
---

# Review 3

This fix is **not production ready**. Actual completion and configured CLI roots now work for the previously tested cases. Two remaining defect classes break the promised agreement between reference readers: external-source contexts lose anchors, and several CLI readers still interpret references as ordinary filenames.

## Previous findings

Review #2 had two unblocked findings and no blocked findings. There was no blocked implementation item to unblock. Its human request was a completeness check, separate from implementing either finding; the implementation log does not record that check as completed.

| Previous finding | Verification in this review | Status |
|---|---|---|
| Claudine schema completion joins committed prompt paths instead of using the shared grammar | All six new process tests pass, including all three commands, names and values, emitted magic tokens, directory precedence, and referenced schema order | Previous reproductions fixed; the external-repository case below remains |
| Parity tests substitute composition for completion and exclude configured CLI roots | Claudine's matrix now invokes `__complete`, preserves supplied-value composition separately, and checks prompt execution. All three `md` variants include configured roots. Both area matrices and magic-root tests pass | Specific substitutions and excluded rows fixed; the sweep still omits failing CLI readers and source transitions |

## Unblocked Findings

### High — External-source contexts lose repository or launch magic anchors

**Defect class:** changing a reference context for an external source drops an anchor that must remain authoritative, either the source's repository or the request's launch magic scope.

In claudine-cli, [CommittedPrompt::resolve](../../../claudine/cli/src/completion/schema_completion/mod.rs:75), which locates the prompt supplying setter suggestions, derives an external prompt from the launch context without discovering the prompt's repository. The trusted derivation admits the file but has no catalog for its repository. Consequently its `$schema: '&schemas/order.yaml'`, `^schemas/order.yaml`, and bare `schemas/order.yaml` cannot resolve. Claudine composition discovers that repository in [InvocationContext::derive_source](../../../claudine/lib/src/invocation_context.rs:1154) and succeeds. The new same-repository spelling repair does not cover a genuinely different repository.

In darkmatter-cli, [MdRequest::document_context](../../cli/src/request.rs:142), which supplies document contexts to composition, validation, cleaning, graphs, and trigger inspection, discovers an external document's repository but also replaces the launch magic scope. It rebuilds at the source directory and never restores the launch context's scope. An authored `@magic.md` then reads repository B's file instead of repository A's file. The public library's trusted derivation and Claudine composition preserve A's scope. The source's `&` and `^` anchors should belong to B while `@` retains A's launch scope; these are independent anchors.

**Reproduction:** copy the new committed-prompt fixture's `zebra: enum(red, blue)` and `apple: enum(one, two)` schema into two temporary Git repositories, `launch` and `source`. Put `source/prompts/prompt.md` below the second repository and `source/schemas/order.yaml` at its root. Launch from the first repository and change only the prompt's `$schema` reference between the forms in the table. Invoke:

```text
claudine __complete --current 3 -- claudine compose /fixture/source/prompts/prompt.md zebra=
claudine __complete --current 3 -- claudine compose /fixture/source/prompts/prompt.md a
claudine compose --dry-run /fixture/source/prompts/prompt.md
md compose /fixture/source/prompts/prompt.md
```

Repeat completion with `inline-compose` and `sequence`. For the magic-scope half, put different `magic.md` markers in both repositories and use `::file @magic.md` in the external document. Also put different `@order.yaml` enum schemas at both roots and set `zebra: launch` in a document using that schema: the launch schema accepts it, the source schema rejects it.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `compose` setter-value completion | External prompt; `$schema` uses bare, `&`, or `^` root lookup | No suggestions | Offer `red` and `blue` from the source repository |
| `inline-compose` setter-value completion | Same three schema references | No suggestions | Same source schema suggestions |
| `sequence` setter-value completion | Same three schema references | No suggestions | Same source schema suggestions |
| Property-name completion in all three commands | Same external prompt; partial `a` | No names | Offer `zebra=` then `apple=` |
| Authored-order reader, sharing `CommittedPrompt` | External prompt; `./`, absolute, `~/`, and launch `@` schema references | `zebra=` precedes `apple=`; clean | Preserve order when the schema can be located |
| Effective-schema loader in all three commands | Same four reference forms, plus an inline-schema external prompt | Correct enum suggestions; clean | Keep these results |
| Completion controls | Launch inside the source repository; bare, `&`, `^`, and `./` references | Correct names and values; clean | Keep these results |
| Claudine prompt composition | External prompt with bare, `&`, `^`, and `./` schema references | Success; clean | Keep the source's repository |
| Claudine prompt composition | External document with `::file @magic.md` | `LAUNCH_MAGIC`; clean | Keep the launch magic scope |
| `md compose` | Same external document | `SOURCE_MAGIC` | `LAUNCH_MAGIC` |
| `md graph --json` | Same document and launch | Transclusion target is `source/magic.md` | Target is `launch/magic.md` |
| `md schema validate` and compose schema stage | External document; `$schema: '@order.yaml'`, `zebra: launch` | Reject against source enum | Accept against launch enum |
| `md clean --json` schema reader | Same enum document | Reports an invalid-enum repair diagnostic from source schema | No such diagnostic |
| `MdRequest::document_context`, also used by reference validation and schema-trigger inspection | External document B, launch A; public API observation | Repository B and launch scope B | Repository B and launch scope A |
| `md validate refs` | External magic directive with both target files present | Reports valid; does not expose selected file | Retain A's identity, not merely existence; its context comes from the failing API above |
| Darkmatter compose and preflight APIs | Same external magic directive, prepared request at A | Compose reads `LAUNCH_MAGIC`; preflight names A's target; clean | Keep these results |
| DMLS repository-context reader | Same external document | Public context API retains B and resolves its `&schemas/order.yaml`; clean | Its request intentionally belongs to B, unlike a CLI launched in A |

The source census includes both Claudine completion consumers (effective-schema loading and authored-order recovery), the shared referenced-schema reader, all six callers of `MdRequest::document_context` (`compose`, reference validation, `clean`, schema validation, graph, and schema triggers), Claudine's invocation-owned source builder and exported source-context helper, Darkmatter's ordinary/trusted source derivations and expression-context projection, and DMLS's per-repository reader. No command-specific completion reader remains to fix separately. Claudine's exported source helper explicitly preserves the launch scope; the normal composition process exercises its invocation-owned counterpart. Darkmatter's expression projection inherits the supplied context and cannot restore a scope already lost by the CLI.

Build completion's source context using the same source-repository policy as composition, retaining the opening reference and launch scope. Preserve the original launch scope when `md` rebuilds around a foreign repository. Add two-repository process cases and positive controls to the shared matrix. Recheck the comments claiming completion follows composition and the documentation describing launch-scoped magic lookup.

### High — CLI file readers still bypass the reference grammar

**Defect class:** a file reader substitutes raw path operations for shared reference resolution, either rejecting valid prefixed references or accepting invalid syntax and tree escapes.

Darkmatter-cli's reference validation joins its argument to the launch directory; schema validation canonicalizes the raw argument before loading; schema detection directly loads it; and schema-trigger inspection canonicalizes it without reference resolution. The code-block file branches similarly use raw joins or filesystem probes. Having a correctly built context afterward cannot repair opening the wrong filename first.

The otherwise shared [resolve_file_path](../../cli/src/io/mod.rs:77), which opens arguments for rendering, composition, cleaning, frontmatter commands, hashes, table of contents, delta, and graph, also turns every `FileReference::new` error into a plain path. Bare malformed introducers such as `@`, `&`, and `^`, and the removed `!legacy.md` syntax, therefore open literal files in many routes. [run_edit](../../cli/src/commands/frontmatter.rs:274) has a separate copy of this parse-error fallback. This contradicts the reserved-introducer contract and removes `InvalidReference` provenance. Spell a literal reserved filename as `./@` to identify it explicitly instead.

**Reproduction:** copy the route fixture into a temporary Git repository. Use a document with a `title: Fixture` frontmatter value and a `ROUTE_TARGET` heading, plus a package launch directory, a separate fixture home, a configured root, and an `outside.md` above the repository. For each route, supply bare, `./`, `&`, `^`, `@`, `~/`, absolute, and escaping relative arguments from both launch directories. Restore the fixture document before each mutating command. Then create literal files named `@`, `&`, `^`, and `!legacy.md` and repeat the malformed arguments. The unedited document is the positive control for every route.

```text
md --magic-root /fixture/home/.claudine/prompts schema validate '@doc.md'
md --magic-root /fixture/home/.claudine/prompts validate refs '@doc.md'
md schema detect '../outside.md'
md get '@' title
```

The first two cannot open the reference even with the root configured. Detection accepts the tree escape. `get` reads the literal malformed filename.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| [Reference validation](../../cli/src/commands/validate.rs:28) argument | `&`, `^`, `@`, `~/`; nested bare; tree escape; malformed introducers | Prefixes/nested bare fail as filesystem misses; escape and malformed files accepted | Resolve supported forms; refuse escape and invalid syntax with their reference classes |
| [Schema validation](../../cli/src/commands/schema/validate.rs:143) document arguments | Same forms and launches | Prefixes/nested bare exit 3 as parse/load failures; escape and malformed files accepted | Resolve before loading and retain opening provenance |
| [Schema detection](../../cli/src/commands/schema/detect.rs:62) document arguments | Same forms and launches | Same unsupported-prefix and escape behavior | Open through the request context |
| [Schema-trigger inspection](../../cli/src/commands/schema/triggers.rs:16) document argument | Same forms and launches | Prefixes/nested bare fail; malformed files accepted; outside document rejected later for lacking a repository | Resolve first; an escape must fail as `InvalidReference`, not later as missing repository |
| [Code-block forced file branch](../../cli/src/commands/code_block.rs:39) | Same forms and launches | Prefixes/nested bare fail; escape and malformed files accepted | Use shared resolution for file input, as the plan's document-argument migration requires |
| Code-block default filesystem branch | Existing prefixed input; malformed literal filename; escape | Prefixed input becomes literal code; malformed file and escape are read | Resolve file input through the context while retaining explicit content mode |
| Code-block `--content` control | All reference-looking strings | Renders the authored string; clean | Keep literal-content behavior |
| Render, `toc`, `get`, `set`, `rm`, `hash`, and both `delta` input slots | Valid forms/launches; tree escape; malformed introducers with matching files | Valid forms and escape rejection are correct; malformed filenames are opened | Keep valid behavior; return `InvalidReference` for parse failures |
| Compose, `clean`, and graph argument readers | Same shapes | Valid forms and escape rejection are correct; malformed input falls through to a relative raw path, then fails building a context with an empty request directory | Reject at reference parsing, with `InvalidReference` |
| `edit`'s separate reader | Valid forms and escape at root; malformed introducers | Valid resolution and escape rejection correct; malformed literal files opened | Reject invalid reference syntax before launching an editor |
| Claudine prompt execution control | Same four malformed introducers | `failure: invalid-reference`; clean | Preserve typed rejection |
| Claudine completion and authored-schema readers | Supported forms and nested `./` negative control | Shared parsing; permanent tests pass; clean for this class | Preserve these fixes; source-context defect is the first finding |

The `edit` probe set both editor variables to `/usr/bin/true`, so it ran a non-interactive no-op and opened no application. Output destinations, cache directories, and the new magic-root directory flags are directory/path settings, not source-file reference readers, and are not included in this claim.

Centralize opening all source-file arguments through `FileReference`, preserve the opening spelling when deriving a document context, and remove both parse-error-to-plain-path fallbacks. Extend `MdArgument` coverage to the actual route census above, rather than just composition. The new magic-root test's reference-validation row checks `@` *inside* `main.md`, while its schema-validation matrix checks authored values inside documents opened by ordinary paths. Neither exercises these failing argument readers. Its negative route control also searches only for the heading marker, even for routes that print a title or hash; assert each route's own observable result or failure.

## Blocked Findings

None. Both findings can be implemented without permissions, unavailable hardware, or a new design decision. The human completeness check is a separate review-loop requirement.

## Recurrence

The second finding repeats **review-2.md — “Claudine schema completion still resolves committed prompt paths by joining directories”**: additional readers bypass the same shared grammar, apply raw-path behavior, or discard parser failures. That fix swept completion readers but did not sweep CLI reference validation, schema document readers, the two code-block filesystem branches, the common argument reader's parse-failure branch, or `edit`'s independent fallback.

It also exposes the verification class reported in **review-1.md — “The parity matrix omits required cases and changes a required failure”** and **review-2.md — “The parity matrix substitutes composition for completion and omits configured CLI roots.”** The fixed matrix still treats `MdArgument` as composition's argument rather than enumerating the CLI argument readers, and never tries malformed reserved introducers. Its same-repository and repository-free home fixtures do not test a source in a second repository.

The first finding is a source-context transition defect beyond the earlier completion reproduction; its missing external-repository case belongs in that completeness sweep. This review enumerates the failing and clean sibling readers above. `recurrence: true` requests the human completeness check before another automated cycle, independently of implementation readiness.

## Verification and requirement coverage

Level 1 is appropriate for this fix's filesystem resolution, process completion, and language-server protocol contracts. It introduces no terminal-layout, physical-key, mouse, or browser behavior requiring Level 2 or Level 3. No terminal or browser window was opened or focused.

| User-facing requirement | Verification level and evidence | Result |
|---|---|---|
| Preflight repository-reference regression and package-scoped lookup | Level 1 shared library/CLI matrix | Existing cases pass |
| Required context, shared builder, binary-only process capture | Level 1 construction guards in Darkmatter, CLI, DMLS, Claudine library/CLI, generator | Pass |
| Builder rejection, captured environment, tracing, nested request reuse | Level 1 request-context builder and epoch tests | Pass |
| File, code, and table-of-contents directive resolution, fallback chains, literal `#` targets | Level 1 shared matrix and directive tests | Pass |
| Editor navigation, graph, schema diagnostics, narrow workspace handling | Level 1 protocol parity matrix | Pass |
| Editor repository reuse, watcher/rescan/configuration invalidation, failure recovery, untitled buffers | Level 1 repository-context tests | Pass |
| Actual completion, all commands, emitted-token round trip and authored order | Level 1 process tests plus two-repository probes | Existing forms pass; external-source schemas fail |
| Configured roots and consistent CLI argument semantics | Level 1 magic-root tests, matrix, complete route probes | Composition/root configuration passes; additional readers fail |
| Cross-source launch magic scope | Level 1 public context, composition/preflight, and process probes | Library and Claudine preserve it; `md` loses it |
| Test target declaration, enabled features, active tier selection | Cargo manifests, `l1/main.rs`, metadata, both area tier checks | Permanent new tests compile and run; no stranded tiers |

Successful local runs: Darkmatter `just test entry_point_parity` (12), `just test magic_root` (10), `just test context_construction_guard` (4), `just test repository_contexts` (11), and `just test request_context` (22); Claudine `just test entry_point_parity` (1), `just test completion_committed_prompt_schema` (6), and `just test context_construction_guard` (3). Both `just check-tier-coverage` area checks passed. These are targeted runs, not a full-suite or lint claim.

Temporary Level 1 observations directly exercised the public CLI context API, Darkmatter composition/preflight APIs, and DMLS repository-context API. Their passing assertions describe observed behavior; they do not turn the wrong `md` scope into acceptance evidence. An initial temporary probe had a borrow-type error, corrected before its successful run. Root-recipe invocations that treated a filter as a package name were rerun through the area recipe. Probe sources were removed and the original test files restored byte-for-byte.

The configuration-field robustness matrix is not applicable to the changed acceptance rules: this implementation changes reference lookup/context ownership and adds a repeated CLI directory flag, rather than changing deserialized configuration fields. The review exercised valid, missing, malformed, explicit-relative, and escaping reference inputs through public results. Existing schema parsing rules were not changed or expanded into a separate parser review.

The two ambient-state sites expressly assigned to the following glob work remain outside this review. The logged interrupt-notice hyperlink and lifecycle frontmatter-write target reader are unchanged presentation/lifecycle behavior, not the completion or CLI source-opening defects demonstrated here. DMLS's lack of a production setting for extra roots is distinct from its public snapshot-based resolution contract and was already disclosed; no new setting is required by this review. Cross-OS receipts and the existing unrelated link snapshot failure were not used to decide readiness.

Process fixtures and JSON observations are retained on this host under `/var/folders/l9/xdcp3xnn6s78_5l9w2_mnvtw0000gn/T/file-refs-review3-imy4_f9w`; temporary Rust probe sources are under `/tmp/file-refs-review3-*.rs`. Review output consists only of this file and the requested previous-review/spec metadata updates. No implementation changes or commits were made by this reviewer.
