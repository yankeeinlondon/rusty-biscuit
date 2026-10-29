---
kind: implementation-log
---

# Documentation audit: 2026-10-25-loops-and-flow-control

Updated **5 existing canonical documents**; created **0 documents**. The
completed specification now lists ten canonical documents using implicit
repository-relative paths. All ten declare
`kind: canonical-documentation`; the specification body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `claudine/docs/topics/flow-control/looping.md` | Uniform document/group/sequence loops, cap precedence, iteration order, typed results, nearest-loop break, atomic actions, retry pass restoration, Ctrl-C | Distinguished an action staging failure from a refused container runtime write. Existing iteration and break diagrams, examples, and checkpoint contract are current. |
| `claudine/docs/topics/flow-control/flow-control.md` | Choosing directives, returning helper calls, root-only break exit codes, skip versus stop, preparation limitations | Corrected the retry summary to distinguish non-looping containers from looping pass checkpoints. |
| `claudine/docs/topics/flow-control/flow-control-reference.md` | Exact directive placement, caller data, isolation, cycles, cancellation, and dry-run rules | Corrected the container retry summary: completed passes survive, current-pass mutations and outputs restore before the retry overlay, and the count does not advance. |
| `claudine/docs/topics/flow-control/sequences.md` | Task guards, captured group state, container loops/results/retries, caller values, single-pass dry-run rendering | Reviewed current; already canonical. |
| `claudine/docs/topics/flow-control/lifecycle.md` | Returning `prep`, loop gates, initialization, cancellation restrictions and typed action parameters | Reviewed current; already canonical. |
| `claudine/docs/topics/composition.md` | Command behavior, caller layers, interpolation and dry-run limitations | Reviewed current; already canonical. |
| `claudine/docs/cli/sequence.md` | CLI usage, dry-run and mutating `--dry-run-shell`, guard/loop previews | Reviewed current; already canonical. |
| `claudine/docs/topics/execution-flow.md` | Execution pipeline and container recovery | Added canonical kind, removed a generated conversational preamble and prompt-generation metadata, and corrected the looping retry summary. |
| `claudine/docs/getting-started/index.md` | Introductory loop and helper-call examples | Added canonical kind. Replaced invalid YAML/increment syntax, incorrect cap of 99, and retired `next` hook with a complete post-run loop example and current `prep`/`proxy` guidance. |
| `darkmatter/docs/topics/darkmatter-expressions.md` | Shared file-reference resolution, missing-target path construction, absolute anchors, HTTP(S) refusal and typed probe errors | Reviewed current; already canonical, with existing/missing-path and caller-anchor examples. |

The implementation log records that the review-loop prompt migration was
canceled by an author ruling; the prompt is absent. No runnable review-loop
workflow was invented in the documentation. Existing helper-call docs correctly
mark group-file targets and static target approval inventory as planned, and
explain that timeouts apply per provider child rather than through an
invocation-wide deadline. `defer` remains explicitly planned.

## Validation

Compared the pass-restoration wording with `PassCheckpoint::restore`, nested
sequence retry handling, and `finish_container_pass`; checked `prep` behavior
against the in-process CLI target runner. Existing loop iteration and break
propagation Mermaid diagrams cover the required state changes. The path guide
already documents the shared resolver and the changed missing-target contract.

Validated all ten references, the canonical kind on every referenced page,
unchanged specification body, existing relative Markdown targets in changed
pages, and absence of dated specification names in those pages.
`git diff --check` passed. Test execution belongs to the concurrent test agent;
this documentation audit ran no tests. No skill update was needed because the
Claudine skill already describes pass checkpoints, returning preparation, dry
run, guards, and the implemented limitations accurately.
