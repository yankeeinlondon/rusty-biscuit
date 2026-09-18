# Lifecycle boundary investigation

Read-only source investigation for the technical design checkpoint, 2026-09-17.
This is evidence and **unconfirmed recommendations**, not an agreed design or implementation plan.

## Evidence status

Read the complete specification, repository AGENTS.md, Claudine and Darkmatter skills, and their lifecycle/composition guidance. No nested AGENTS.md was found beneath these package areas.

GitNexus calls were explicitly bound to `better-static-analysis`. While the orchestrator awaited the existing index refresh, provisional `query` and `context` calls were used first for navigation. The index reported 46 commits behind; several lifecycle symbols were absent and some query rows had malformed IDs or mismatched names and paths. **These results provide no reliable current impact counts or risk assessment.** Current-source findings below were verified directly after those empty/unusable graph results. Current impacts still require a healthy refreshed index.

## Verified current architecture

| Surface | Source evidence | Consequence |
| --- | --- | --- |
| Event-time subtree interpolation | `claudine/lib/src/composition/lifecycle/executor.rs:911` (`resolve_string_value`) | Builds effective state and globals, invokes strict subtree compose, then rejects surviving spans. |
| Lifecycle shell preflight | `claudine/lib/src/composition/preflight.rs:381` (`resolve_shell_command_expr`) | A bespoke late-binding AST scan precedes subtree compose; successful bytes replace the authored command expression. Darkmatter failures already retain a boxed typed source. |
| Sequence shell preflight | `claudine/lib/src/composition/sequence/preflight/mod.rs:941` | Uses strict subtree compose with source-specific file-resolution context and launch context; wrapper retains the boxed Darkmatter source. |
| Sequence task values | `claudine/lib/src/composition/sequence/task/mod.rs:863` (`resolve_value`) | Uses strict subtree compose, preserving task/field context and boxed `MarkdownError`. |
| Direct lifecycle evaluation | `claudine/lib/src/composition/lifecycle/executor.rs:888` (`eval_expr`) | Builds a separate `LayeredLookup` and calls Darkmatter `evaluate` directly. A subtree-only binding fix would miss this path. |
| Guards and operands | Executor lines 1132, 1284, 1843 | `when_matches`, `render_message`, and `resolve_typed_value` each run closed-world root checks. The latter two evaluate first and then potentially interpolate the resulting string again. Typed values also undergo recursive remaining-span rejection. |
| Shared source-relative context | `claudine/lib/src/composition/mod.rs:189` | `document_expression_resolution_context` already centralizes source-relative file resolution while retaining invocation snapshots. It constructs ComposeOptions and asks Darkmatter for its local resolution context. |
| Preparation availability | `claudine/lib/src/composition/lifecycle/validate.rs:707` | `validate_no_err_in_no_error_events` traverses communication spans and stack ASTs itself. |
| Root catalog | `claudine/lib/src/composition/reserved.rs:19` | `LATE_BINDING_ROOTS` currently contains only `err`, `timing`, and `current`. |

### Capture timing and live environment discrepancy

`LifecycleCurrent` (`lifecycle/context.rs:480`) contains two values: `ctx` and `env`. `capture_at_event` (line 535) captures Darkmatter context and process environment before evaluation. `to_value` exposes **`current.ctx.*` and `current.env.*`**. `lifecycle_injected_globals` (line 566) registers `current` as a lazy closure over an already-captured clone; laziness delays JSON materialization, **not observation of environment or host state**. `err` and `timing` are eager. Missing inputs omit globals entirely.

`current_env` is not independently injected. `group` is added separately by `StackExecutionContext::injected_globals` (`executor.rs:813`) when a group map exists.

Event capture callers include library `composition/looping/engine.rs:1016` and CLI `commands/wrap/composition/pipeline.rs:1723`, `preflight.rs:146`, `staged_boot.rs:242`, and `harness_orch/loop_control/lifecycle_events.rs:425`.

The lifecycle skill describes direct `current.<key>` and `current_env.<key>` mirrors as ratified in the separate more-context specification, explicitly noting implementation is pending. That is not current implementation evidence. Selecting direct mirrors or reference-time recapture here changes observable shape or timing and needs an explicit human ruling. A conservative recommendation is to preserve event observation timing while separately deciding the public projections; do not silently infer a timing change from the word “lazy.”

Darkmatter's `InjectedGlobal::Lazy` (`darkmatter/lib/src/markdown/compose/subtree.rs:73`) accepts `Fn() -> Value`; `LayeredLookup` memoizes per lookup instance (lines 169, 198). Lifecycle currently constructs a fresh lookup for each `eval_expr` or subtree operation. A prepared object must therefore separate reusable parsed/binding metadata from a fresh runtime session/cache. Sharing a cache across actions or events would change behavior for general lazy providers even though the existing lifecycle closure only serializes a fixed snapshot.

### State and atomicity

`StackExecutionContext::early_binding_context` (`executor.rs:844`) reuses the preparation snapshot when supplied, otherwise performs demand-driven capture. `build_state` (line 862) uses the current working frontmatter, builds Darkmatter effective state, and has a permissive-context fallback. This is distinct from Claudine's orchestration of authored/external/runtime/overlay precedence; the latter should remain Claudine-owned.

`resolve_proxy_with` (`executor.rs:1739`) captures one fallback early-binding context for the complete overlay when no prepared context exists. It evaluates every leaf before returning the overlay, retaining exact nested error paths. Consolidation must preserve that snapshot boundary and all-or-nothing mutation. A shared Darkmatter prepared subtree can perform traversal without taking ownership of proxy handoff policy.

### Typed-error loss

`LifecycleExprError` (`executor.rs:216`) initially retains boxed `ExpressionError` or `MarkdownError`. `LifecycleErrorInfo::from_error_or_action` (`context.rs:172`) takes a borrowed error and projects its diagnostic snapshot and text; the original cause is discarded. `CompositionError::lifecycle_evaluation` (`error/mod.rs:3079`) later rebuilds `LifecycleEvaluationError` from the snapshot, retaining source path/event/property/reason but no original typed cause (`error/mod.rs:1470`).

The proxy overlay route loses the cause even earlier: `walk_proxy_with` (`executor.rs:1769`) creates `LifecycleProxyWithEvaluationFailed` using only `error.to_string()`. Fixing only the outer lifecycle variant would not repair this path.

**Unconfirmed recommendation:** carry an owned expression failure separately from the cloneable `err` diagnostic projection, using a typed cause enum whose heavy variants are boxed or shared where recovery needs cloning. Keep event/property/action/source context in Claudine and parsing/binding/schema causes in Darkmatter. Avoid making `LifecycleErrorInfo` itself the sole error transport; it is also the authored `err.*` value projection. Public library callers should inspect cause variants without rendering or parsing prose. Exact ownership/clone strategy remains a design decision.

### Other direct evaluation consumers

Direct Darkmatter evaluation also occurs in `composition/sequence/expr.rs:131`, `composition/looping/expression.rs:218`, `composition/looping/actions.rs:221`, and dispatch `template.rs:493`, `matcher.rs:160`, `runner/mod.rs:115`. They must consume the richer default resolver without accidentally inheriting lifecycle globals.

`SourceExpressionLookup` (`sequence/expr.rs:73`) tries per-item fields, live `env.*`, document fields, then `CtxLookup`. Its reserved-namespace behavior needs explicit migration review: per-item/document shadowing cannot replace the specification's reserved namespaces. `render_interpolated` (line 146) is a separate sequence interpolation implementation; consolidation must establish current fixtures before assuming its single-pass behavior matches ordinary mixed-string interpolation. Loop and sequence expression wrappers already preserve typed parse/evaluation causes.

## DRY seam dispositions proposed for discussion

| Responsibility | Proposed owner and boundary | Timing/error constraints |
| --- | --- | --- |
| Authored/external/runtime/proxy layering | Retain Claudine orchestration; Darkmatter receives effective document values. | Do not collapse preparation snapshots and evolving action state. |
| Effective-state construction | Reuse Darkmatter construction; consider a small Claudine adapter for its existing context policy. | Preserve explicit permissive-context policy; do not silently hide a failed build. |
| Context/file-resolution observation | Claudine supplies captured invocation/source evidence; Darkmatter resolves it. | Keep source anchoring distinct from launch-facing `ctx`; never recapture ambient context merely to simplify the API. |
| Global catalog and scope | One Claudine-owned declarative catalog, consumed by runtime and editor through Darkmatter data contracts. | `current_env` shape is unresolved; availability must include unavailable entries, not omissions. |
| Parsing, reference traversal, binding and interpolation | Darkmatter prepared expression/subtree API plus shared runtime resolver. | Separate immutable preparation metadata from runtime caches; preserve whole-value vs mixed-string source provenance. |
| Context requirement traversal | Darkmatter should expose requirements from prepared expression metadata. | Removes Claudine `ctx_scan_hint`/`collect_variable_paths` AST walk (`executor.rs:1963`) without recapturing unreferenced groups. |
| Event/shell scope validation | Claudine selects policy; Darkmatter traverses all active authored expression references. | No lazy invocation during preparation; inactive expression branches still checked for definite violations. |
| Deferred feature policy | Darkmatter retains and enforces location restrictions. | Runtime shell backstop remains Claudine policy; generated/moved value semantics need their own ruling. |
| Error projection | Darkmatter typed causes; Claudine contextual wrapper and cloneable diagnostic projection; CLI rendering. | Preserve the original cause through recovery and proxy/set paths, without changing catch routing. |
| Approval bytes | Retain Claudine orchestration. | Evaluate shell command once at preflight, execute the approved stored bytes without reinterpreting output braces. |

The prepared representation should distinguish authored templates from already-evaluated values. Removing only span rejection leaves the current `render_message`/`resolve_typed_value` result re-evaluation in place, which can violate the specification's single whole-value evaluation and literal-output contract.

## Remaining investigation

- Obtain trustworthy refreshed graph impact evidence; current graph results are not a clean check.
- Agree binding API before fixing prepared-object and typed-cause shapes.
- Resolve `current`/`current_env` public projection and capture timing explicitly.
- Complete the lookup migration inventory and shared conformance fixture audit.
- Specify the lifecycle descriptor payload and schema-location linkage with the schema/editor investigation.

No implementation, prototype, plan, specification edit, commit, provider launch, or audio activity was performed.
