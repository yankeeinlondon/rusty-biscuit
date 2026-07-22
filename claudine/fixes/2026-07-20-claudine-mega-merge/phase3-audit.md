# Phase 3 Proxy-With Integration Audit

- Proxy-with revision: `e348486c810969abe87a6b7209979034f5454b07`
- Foundation checkpoint: `766b31a2a`
- Candidate tree before semantic resolution:
  `4403f57bea49fc3b8e31e1767359d717295325fa`
- Integration method: the exact candidate paths were materialized and resolved
  in the worktree without changing the index because this execution request
  prohibits staging and committing. The ancestry-preserving merge remains a
  separately authorized history operation.

## Conflict resolution

The merge base, foundation version, and proxy-with version were inspected for
all 35 text-conflict paths. The result retains one owner for each behavior:

| Cluster | Final responsibility | Resolution and evidence |
|---|---|---|
| Shared errors/types | Diagnostic registry and lifecycle protocol | Typed snapshots, structured file-reference detail, transition errors, and exactly-once rendering are retained. See the diagnostic matrix in `phase3-test-map.md`. |
| Darkmatter | Explicit compose context and deferred schema stage | Context options accept the caller snapshot and the authoritative schema verdict remains after initialize plus stabilized reread. |
| Preparation | `composition::prepare_document` | Direct, proxy, retry, resume, and loop refresh use the same stage-aware service and carry `FileResolutionContext`. |
| Transitions | Library commit plus CLI coordinator adoption | Producers return `ProxyHandoffRequest`; the coordinator resolves, prepares, validates, and commits before the CLI adopts one committed transition. |
| Launch/retry | Complete `LaunchPlan` | One rebuilt plan supplies compatibility comparison, provider argv/environment, MCP, prompt delivery, and spawn inputs; retry budgets remain command-owned. |
| Sequence Plus | Sequence invocation plus active document | Sequence retains step/task/output ownership while a proxied document owns its lifecycle, loop, launch, and closure inside the current step. |
| Terminal | Diagnostic renderer and task stream | Component rendering and task framing retain typed identity and exactly-once closure without a private error print path. |
| Guards/docs/CI | Owning generator, skill, or test-tier contract | Generated inventories, source guards, fail-closed L2 selection, and architecture documentation describe the merged surfaces. |

The per-path owner, rationale, and evidence pointer are recorded in
`conflict-checklist.md`. No broad ours/theirs resolution was used; conflicting
files were resolved as the smallest coherent ownership units.

## One-sided semantic review

- Target context, environment sanitation, attempt classification, wrapper
  entry, system-prompt lifetime, and session reporting consume the same
  invocation snapshot and rebuilt launch plan.
- Darkmatter transclusion, reference graph, expression, and schema paths retain
  the caller-provided context; no ambient recapture or eager final verdict was
  introduced.
- Coordinator, preparation-stage, restored-diagnostic, launch-plan, Sequence
  task/group, and task-stream modules have no competing adapter. The merged
  boundary guard requires typed handoff producers and a coordinator-owned
  commit point.
- Completion remains a best-effort presentation of the shared
  `biscuit-file::FileReference` grammar. Source scans found no private proxy
  path rewriter.
- Nextest and CI keep L2 fail-closed through
  `BISCUIT_TEST_LEVEL_REQUIRED=2`; backend selection is not silently converted
  into a pass.
- The Windows Rendezvous named-pipe `PipeConnection` still implements tonic's
  `Connected` contract behind the Windows transport, with the existing
  target-gated dependencies. It was reviewed but not modified in Phase 3.

## Architectural responsibility audit

| Invariant | Sole merged owner | Evidence |
|---|---|---|
| I1 typed diagnostic transport | `DiagnosticSnapshot` and diagnostic registry | Diagnostic route/serialization matrix |
| I2 file-reference grammar | `biscuit_file::FileReference` | Boundary/source guards and native/quoted reference tests |
| I3 request context | `FileResolutionContext` in caller layers/request | CWD mutation and cross-repository tests |
| I4 preparation ordering | `composition::prepare_document` | Entry-stage matrix and direct/proxy equivalence |
| I5 transition state machine | coordinator commit/adoption path | Typed handoff guard and failure matrices |
| I6 sequence containment | sequence invocation plus adopted active document | Two macOS L2 containment tests |
| I7 launch rebuild | complete `LaunchPlan` | Facet, compatibility, retry, and resume tests |
| I8 immutable overlay | `ProxyHandoffRequest::with` plus caller layers | Type, precedence, lifetime, and no-write tests |
| I9 schema verdict | Darkmatter deferred/authoritative stages | Coercion, reread, and invalid-input tests |
| I10 terminal emission | diagnostic renderer and task stream | Exactly-once and route-equivalence tests |
| I11 trunk survival | merged repository tree | Source/guard audit and Claudine L1 |
| I12 platform contract | platform-gated adapters and CI matrix | macOS gate plus retained Linux/Windows jobs; no native claim for unrun hosts |

Detailed requirement-to-test mapping is in `phase3-test-map.md`; acceptance
status and platform limitations are recorded in `acceptance-ledger.md`.

## History boundary

The worktree contains the resolved candidate, but the index remains untouched.
Therefore the ancestry merge, proxy merge SHA, staged marker review, ledger-only
checkpoint, and clean-worktree claims remain open in the plan. This is not a
product-code or test blocker; it is the direct consequence of the explicit
no-stage/no-commit instruction.
