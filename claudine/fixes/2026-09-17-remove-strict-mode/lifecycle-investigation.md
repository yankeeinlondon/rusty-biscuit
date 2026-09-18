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

## 2026-09-18 follow-up: current lookup inventory and graph recheck

Read the confirmed D1 and D2 decisions in `design.md`: an additive richer
resolver with an ordinary default, and immutable declarations separated from
runtime sessions with fresh per-evaluation caches. The dispositions below apply
those decisions; they do not select the still-open result/error types.

The initial source revision was `488b8b2e9cf4eebc02625ba33bc01afcf392fa1a`.
External commits advanced HEAD during inspection (including
`b3f69d2ba74fbd05ddd4c3ef14d89480132453a1`). A comparison from `488b8b2` to
that HEAD found no changes under the inspected Darkmatter compose/test or
Claudine composition/dispatch paths. The index metadata identified this exact
worktree and last indexed commit `488b8b2`, indexed at
`2026-09-18T07:40:37.164Z`, but later also recorded incremental work in progress.

The CLI no longer recognized the `better-static-analysis` alias; subsequent
calls explicitly used `/Volumes/coding/wt/rusty-biscuit/feat-better-static-analysis`.
`context EvaluationLookup` returned 17 implementation references, all matching
source. However, upstream `impact` remained inconsistent: its first response
reported **CRITICAL**, 12 affected symbols, one direct implementation and six
processes; a subsequent absolute-path call reported **CRITICAL**, 7,786 symbols
and `partial: true`; a bounded depth-one retry reported **CRITICAL**, one direct
implementation and 31 processes. The walks included unrelated imports/processes
and disagreed with the context operation's 17 direct implementations. The
CRITICAL warning must not be waived using `riskSharedAxes`, but these counts and
process lists are **not trustworthy impact evidence**. A current metadata stamp
did not resolve graph consistency. No clean graph-impact claim is made.

### Complete source implementation inventory

The source inventory contains **21 actual implementation blocks**, plus two
rustdoc `SimpleLookup` examples. The historical 17 is also the context query's
count, not the complete source count. The four additional source entries are
the two Darkmatter integration-test `Lookup` types and Claudine lifecycle's
`MapLookup`/`EmptyLookup` fixtures.

Paths below are repository-relative. “Ordinary default” means the confirmed D1
resolver default, not preservation of forbidden bare-context fallback or an
additional lifecycle-global catalog.

| Implementation | Location | Migration disposition |
| --- | --- | --- |
| `EffectiveState` | `darkmatter/lib/src/markdown/compose/context/effective_state.rs:352` | Ordinary default over corrected document-only bare lookup; remove underlying context fallback, preserve reserved namespaces and custom string coercion. |
| `ResolvingLookup` | Same file, line 411 | Forward richer resolution to wrapped state, retaining borrowed/owned file-resolution context and string coercion; never flatten typed outcomes into `Option`. |
| `FrontmatterSeedState` | `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:96` | Ordinary default; preserve seed-state namespace routing, invocation environment snapshot, and named-object display coercion. |
| `ShortcutLookup` | `darkmatter/lib/src/markdown/compose/conditions.rs:345` | Ordinary default after removing explicit bare-name retry as `ctx.{path}`; keep explicit lazy `ctx` capture and environment behavior. |
| `CtxLookup` | `darkmatter/lib/src/markdown/compose/expression/ctx.rs:82` | Ordinary default; remains an explicit `ctx`-only provider, returning missing for other paths without lifecycle globals. |
| `LayeredLookup` | `darkmatter/lib/src/markdown/compose/subtree.rs:213` | Rich resolver implementation: checked declarations/runtime association, namespace precedence, explicit unavailable outcomes and per-session memoization under D2. Remove root-membership hook. |
| `LoopExpressionLookup` | `claudine/lib/src/composition/looping/expression.rs:139` | Preserve loop ambient values as explicit non-lifecycle globals, document state and existing context/environment timing; use shared classification rather than infer globals from failed document lookup. |
| `SizedLookup` | `claudine/lib/src/composition/looping/actions.rs:248` | Must forward richer resolution through its `dyn EvaluationLookup`; its existing `get`/`get_string`-only forwarding would otherwise erase typed binding failures. Preserve unrelated loop action JSON reparsing. |
| `SourceExpressionLookup` | `claudine/lib/src/composition/sequence/expr.rs:75` | Ordinary sequence namespace; correct reserved namespace precedence and explicit `doc` routing while preserving ordinary item-field-over-document precedence. No lifecycle-global injection. |
| `EventMetaExpressionLookup` | `claudine/lib/src/dispatch/expression.rs:85` | Ordinary default over event-as-document projection; retain deliberate absence of `ctx` on template/matcher surfaces and existing event/environment aliases. No lifecycle globals. |
| `EventMetaConditionLookup` | Same file, line 169 | Route explicit `ctx` through its provider and forward other richer resolutions to inner event lookup; preserve hook-working-directory read-side context. |
| `TestLookup` | `darkmatter/lib/src/markdown/compose/expression/mod.rs:833` | Ordinary default; preserve exact-key test fixtures and missing/null behavior. |
| `FsLookup` | `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs:554` | Ordinary default; preserve filesystem function context, no globals. |
| `FixtureLookup` | Same file, line 885 | Ordinary default; preserve exact-key fixture data and function resolution context. |
| `MapLookup` | Same file, line 1078 | Ordinary default; preserve catalog-example values and rendering checks. |
| `FixtureLookup` | `darkmatter/lib/src/markdown/compose/expression/semantics.rs:810` | Ordinary default; retain dotted traversal and existing null propagation. |
| `MapLookup` | `claudine/lib/src/composition/looping/actions/tests.rs:12` | Ordinary default; retain loop action fixtures without lifecycle policy. |
| `MapLookup` | `claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs:1410` | Ordinary default; retain literal/action-shape data fixture. Availability tests should instead construct real declarations/session fixtures. |
| `EmptyLookup` | Same file, line 1418 | Ordinary default; missing stays null. |
| `Lookup` | `darkmatter/lib/tests/more_is_more_literals_and_indexes.rs:11` | Ordinary default; preserve explicit `ctx.limit` fixture and read-side context. |
| `Lookup` | `darkmatter/lib/tests/predict_conflicts.rs:147` | Ordinary default; preserve empty data and borrowed resolution context. |

The two rustdoc examples at `expression/mod.rs:176` and `:379` can retain their
small `get` implementations and inherit the ordinary default. Update their
explanatory contract if the surrounding documentation changes; they do not need
to reproduce classification or lifecycle policy.

The wrapper audit must include runtime evaluation and interpolation consumers
that call `get_string`: retaining coercion is necessary, but no display helper
may bypass an unavailable-global error by falling back to the old `get` path.
That is a contract concern for the selected additive design, not a reason to
introduce lifecycle semantics into ordinary fixtures.

### Consumer scope reverified

The four production `SubtreeCompose::new` sites remain lifecycle executor
`:917`, lifecycle preflight `:412`, sequence preflight `:941`, and sequence task
`:874`. Direct expression evaluation remains lifecycle executor `:893`, sequence
expression `:131`, loop expression `:218`, loop actions `:221`, dispatch template
`:493`, matcher `:160`, and runner `:115`. These paths establish the coordinated
Darkmatter/Claudine migration even though current transitive graph counts are
unreliable. Their domain policies remain separate; the shared richer resolver
must reach all of them.

## D4 investigation: prepared source and ordinary interpolation

**Recommendation, not a confirmed decision:** use a small immutable Darkmatter
prepared representation shared by passive validation and execution. It retains
source form, parsed/spanned expressions, reference/context-requirement metadata,
and schema-location/policy identity. It contains no resolved values, providers,
environment observations, context snapshots, or lazy cache. Runtime execution
still takes a fresh session at the existing lookup-operation boundary under D2.

Graph-first `context interpolate_value`, explicitly bound to the absolute
worktree path, identified `interpolation/rewrite.rs` and its calls to
`whole_value_span`, `eval_json`, and `interpolate_text`; direct source confirmed
these relationships. No index rebuild or impact claim was needed for this
read-only investigation.

### Existing primitives to reuse

- `expression/lexer.rs:41–73`: `ExpressionLocation`, `InterpolationLiteral`,
  and `ExpressionScanResult` already distinguish executable spans from triple
  brace literals. `ExpressionFinder::scan_plain` supplies the frontmatter scan.
- `expression/ast.rs:256,356`: `SpannedExpr` and its `erase` projection already
  provide a source-aware AST and the runtime AST shape. The parser supports
  separate interpolation and condition modes; retain that distinction.
- `expression/lint.rs:100`: `whole_value_span` classifies an exactly-one-span
  trimmed string independently of whether its expression parses successfully.
  Preparation must not turn a malformed whole-value expression into literal data.
- `interpolation/rewrite.rs:99–281`: existing string and value interpolation
  are the execution authority. No general reusable prepared source/session
  artifact was found in these expression/interpolation modules.
- `lifecycle/action_shape.rs:347`: Claudine currently lowers authored operands
  to `Expr`. Whole-value spans become expression ASTs while ordinary/mixed text
  becomes a string literal. This loses the source-form distinction once a
  whole-value expression itself produces a string literal or template-looking
  result; execution then compensates by inspecting result braces.

### Semantics a prepared representation must preserve

`interpolate_value` routes a whole-value expression to **one typed evaluation**
and returns its value without rescanning. An object/array result is data, not a
new authored subtree. Mixed strings route through `interpolate_text`, which
rescans its rewritten output using the existing bounded loop (currently ten
passes), preserves multiline replacement indentation, and converts triple-brace
literals **after the final interpolation pass**. Generated mixed-string spans
can therefore require runtime parsing; preparation cannot promise all future
parsing is eliminated. Preserve this existing pass count and depth-limit policy
rather than inventing a new stopping/error rule.

Triple braces are excluded from executable references and finally reduce to
literal double braces. `\{{ ... }}` and `\{\{ ... }}` are not executable spans;
composition preserves their backslashes. They need not produce byte-identical
output to triple braces to satisfy ordinary semantics. Scanner tests at
`expression/lexer.rs:1233` cover odd/even backslash parity and escaped openers;
`expression/lint.rs:680` covers whole-value classification, including malformed
spans and exclusions. Existing `composition/interpolation_conformance.rs:97–181`
covers scalar/container whole values, mixed strings, two spans, explicit doc,
functions, and quoted escapes; it needs the specified three literal forms added
as shared consumer fixtures. Its loop JSON-reparse divergence remains outside
this change's scope.

For passive all-reference checking, traverse every executable AST branch,
including inactive branches. Do **not** recursively scan quoted string literals
inside a whole-value/direct expression as if they were a second expression
program: their braces are successful literal output. For mixed strings,
runtime-introduced spans follow only the existing rescan mechanism and inherit
the applicable restriction context. Runtime checks cover references unavailable
to passive source analysis; preparation must neither evaluate branches nor
invoke providers to discover them.

`Evaluator::eval` (`interpolation/evaluator.rs:247`) has a variable fast path
that directly calls `get` and sometimes `get_string`, bypassing `evaluate`.
D1 integration must route this path through richer resolution too. The prepared
artifact does not by itself repair that bypass.

### Material alternatives

| Choice | Benefits | Costs and limitations |
| --- | --- | --- |
| Shared classifier/validator; callers keep parsing independently | Smallest public addition; can reuse current scanner/parser immediately. | Repeated parsing and loss of authored form remain easy; callers must consistently preserve spans, parse mode, context requirements and policy. Does not adequately remove existing source/result confusion without an additional tagged representation. |
| Immutable prepared source plus fresh runtime session **(recommended)** | One source classification and parsed reference model serves validation/execution; retains spans and policy identity; prevents whole-value output re-evaluation by construction. | Requires migrating lifecycle operands from bare AST-only storage; mixed runtime rescans still need parsing; schema generation changes require revalidation. |

Simplicity favors the second option because it consolidates already duplicated
mechanics without introducing an invocation-wide cache or a second evaluator.
There is no useful third alternative unless the human wants a different API
boundary; a combined prepared/evaluated snapshot would contradict D2 and timing
preservation.

An illustrative minimal shape (names not proposed as settled API):

```rust
enum PreparedInput {
    Expression { mode: ParseMode, expression: SpannedExpr },
    Value(PreparedValue), // literal, whole-value span, mixed template, array, map
}

// Pure preparation/validation; declarations contain no runtime providers.
fn prepare(input: SourceInput, location: SchemaLocation) -> Result<PreparedInput, PrepareError>;
fn validate(input: &PreparedInput, declarations: &BindingDeclarations,
            policy: &EffectiveSchemaPolicy) -> Result<(), ValidationError>;
fn evaluate(input: &PreparedInput, session: &EvaluationSession,
            policy: &EffectiveSchemaPolicy) -> Result<Value, EvaluationError>;
```

The artifact may carry policy identity/provenance, but a mutable editor registry
index alone is insufficient: it must be tied to the effective schema generation
or an immutable policy snapshot, so a refresh cannot silently reinterpret an old
artifact. Exact restriction propagation through moved/generated values is a
separate outstanding decision, not settled by this illustration.

Context requirement metadata records what may be needed; Claudine still decides
when and from which invocation/source to capture it. Do not precompute runtime
availability, property existence, branches, or values. Do not widen one existing
leaf evaluation's cache to an entire event simply because a prepared subtree is
reusable. Schemas and source may be prepared once, then evaluated against changing
document state with appropriate fresh sessions.
