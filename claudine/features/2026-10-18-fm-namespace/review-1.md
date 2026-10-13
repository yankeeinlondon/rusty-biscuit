---
$schema: feature-review.yaml
ready: false
findings:
    - title: Namespace rename, editor support, and artifact migration are not implemented
      priority: high
    - title: Per-document environment capture and its acceptance tests are not implemented
      priority: high
observations:
    - Hook templates already reject direct reads through the old payload namespace
human_review: true
human_review_items:
    - |-
        Decide how shell approval should work when later sequence tasks capture different environment values. Today a command is approved as exact text before the sequence starts. A later task's environment could produce different text. The spec assumes approval of command templates, but that capability is scheduled after this feature and is not implemented.

        Choose one approach before implementing the affected environment work:

        - Keep exact-text approval temporarily and refuse a task when its environment changes the approved command text. This is the smallest option; record the temporary departure in the implementation log.
        - Implement command-template approval first, so this feature can follow the spec exactly.
        - Separate per-document environment capture from the namespace rename and schedule it after command-template approval.

        Recommended: the first option. It preserves existing approval enforcement without adding a second approval system. The namespace work can proceed independently.
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-12T21:12:00-07:00
spec: 2026-10-18-fm-namespace/spec.md
implemented: true
next: 2026-10-18-fm-namespace/review-2.md
description: A **feature** review of `2026-10-18-fm-namespace/spec.md`
feature: 2026-10-18-fm-namespace/review-1.md
log: claudine/features/2026-10-18-fm-namespace/log.md
implemented_by: claude/opus
---

# Review of the `fm` namespace

The feature is **not production ready**. The implementation completes the discovery and feasibility phase, rather than the specification's production changes. The [implementation plan](plan.md) checks off only its first two tasks; namespace implementation, document entry routing, migration, and final verification remain unchecked. The spec's `implemented: true` does not describe completion of those requirements.

This review treats the requested scope as the whole specification. It does not interpret completion of the first phase as permission to omit later acceptance criteria. There are two high findings. Only the shell-approval portion of the environment finding needs an author decision; the remaining work can proceed without human review.

## Findings

### High — Namespace rename, editor support, and artifact migration are not implemented

**Authority: spec.** Acceptance criterion 1 requires “Rename complete” and says “No production Rust code special-cases the literal `doc` root.” Criteria 2–4 require working `fm` and hook `hook` lookups, ordinary `doc` properties, and reserved-root precedence. Criteria 6–7 require editor support and migration of current schemas, prompts, documentation, and skills. These are acceptance requirements, rather than regressions attributed to the feasibility test.

**Defect class:** the coordinated namespace migration has not reached the runtime readers, passive classifiers, or authored artifacts that depend on it.

For a realistic document with `title: X`, `doc: {title: userland}`, and `fm: literal`, the public composition API renders:

```md
[{{ fm.title }}] [{{ fm['title'] }}] [{{ fm.fm }}] [{{ doc.title }}]
```

as `[] [] [] [X]`, rather than `[X] [X] [literal] [userland]`. Changing only the namespace in a Bash-event matcher from `doc.tool_name == 'Bash'` to `hook.tool_name == 'Bash'` changes a match into a non-match. An author using the specified spelling cannot obtain the specified result.

The sweep used a temporary Rust example calling the shipped public APIs with a temporary document directory and supplied environment maps. It reused the shape of the existing document-context fixture and hook-event fixture. The example was removed after execution. Runtime results below are distinguished from source inspections; private helper paths were inspected through their callers and were not independently executed.

| Site and purpose | Shape checked | Observed result | Required result |
| --- | --- | --- | --- |
| Darkmatter [`reserved_root_descriptors`](../../../darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs:63), the shared root catalog | Public catalog enumeration | `doc, ctx, env, current, current_env` | Exactly `fm, ctx, env, current` |
| Darkmatter [`EffectiveState`](../../../darkmatter/lib/src/markdown/compose/context/effective_state.rs:227), the effective document lookup | Bare and dotted reads of the fixture above | `fm` reads `literal`; `fm.title` and `fm.fm` are absent; `doc.title` reads `X` | `fm` snapshots the effective map; qualified reads select its members; `doc.title` reads `userland` |
| Claudine [`LoopExpressionLookup`](../../lib/src/composition/looping/expression.rs:128), loop-condition lookup | Same bare and dotted reads | Same incorrect namespace selection | Same frontmatter namespace contract |
| Claudine [`SourceExpressionLookup`](../../lib/src/composition/sequence/expr.rs:113), sequence-source lookup | Same bare and dotted reads | Same incorrect namespace selection | Same frontmatter namespace contract |
| Darkmatter [`evaluate_condition_against`](../../../darkmatter/lib/src/markdown/compose/conditions.rs:278), condition evaluation | `fm.title == 'X'`, with old spelling as control | New spelling is false; old spelling is true | New spelling is true |
| Darkmatter [`FrontmatterSeedState`](../../../darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:133), interpolation input lookup | `a: '{{ fm.b }}'`, `b: X`, composed through `ComposeRequest` | `a` renders empty and reports undeclared `fm` | `a` resolves to `X` |
| Darkmatter frontmatter interpolation, including indexed access | `a: '{{ fm["release.channel"] }}'`, `release.channel: X` | Empty with undeclared `fm` advisory | Literal bracket dependency resolves the single key and yields `X` |
| Darkmatter body interpolation through [`Markdown::compose_with`](../../../darkmatter/lib/src/markdown/compose/mod.rs) | The four-expression document above | `[] [] [] [X]` | `[X] [X] [literal] [userland]` |
| Claudine [`EventMetaExpressionLookup`](../../lib/src/dispatch/expression.rs:87), normalized hook payload lookup | Bare `hook`, `hook.tool_name`, `hook.extra.doc.title`, and old `doc.tool_name` | New reads are absent; old read returns `Bash` | New reads return payload, `Bash`, and nested data; old read gets no payload namespace treatment |
| Claudine [`EventMetaConditionLookup`](../../lib/src/dispatch/expression.rs:178), hook `when` lookup | Evaluate the same equality condition under each spelling | `hook` condition is false; `doc` condition is true | `hook` condition is true |
| Claudine [`matches`](../../lib/src/dispatch/matcher.rs:149), event matcher execution | Compile and execute the same equality condition | `hook` matcher is false; `doc` matcher is true | `hook` matcher is true |
| Claudine [`interpolate`](../../lib/src/dispatch/template.rs:442), dispatch and report-message templates | `{{ hook.tool_name }}` and bare `{{ hook }}` | Tokens remain literal | Render field and payload values |
| Document lookup controls across Darkmatter, Claudine loops, and Claudine sequences | Bare `title`; missing `fm.missing` | `title` returns `X`; missing path remains absent | These cases are clean, although missing `fm` alone does not prove reserved-root handling |
| Hook lookup controls | Bare `tool_name`, `extra.doc.title`, missing `hook.missing`, excluded `hook.env.HOME` | Existing bare fields resolve; missing and excluded paths are absent | Clean controls; preserve them during migration |

The following sibling decisions were also inspected. All remain on the old contract; none represents an additional isolated finding to repair in a later review.

| Sibling sites | Shape inspected | Observed result | Required result |
| --- | --- | --- | --- |
| Darkmatter [`doc_namespace.rs`](../../../darkmatter/lib/src/markdown/compose/expression/doc_namespace.rs:22) and its callers in conditions, seed lookup, and effective-state lookup | Bare root and prefix recognition | Recognizes `doc` / `doc.` | Recognize `fm` / `fm.` |
| Darkmatter [`classify_executed_body`](../../../darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:817) | Single-token namespace value | Uses the old namespace helper | `fm.build` bypasses executable lookup |
| Darkmatter dependency collectors in [`frontmatter_interpolation.rs`](../../../darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:1526), [`errors/blocks.rs`](../../../darkmatter/lib/src/markdown/errors/blocks.rs:525), and [`preflight/collect.rs`](../../../darkmatter/lib/src/markdown/compose/preflight/collect.rs:1494) | Bare, dotted, and literal-index namespace dependencies | Old-root special cases; the log's identified bracket gap is not repaired | Apply `fm` dependency ordering, including dotted literal keys and cycles |
| Darkmatter [`static_truthiness`](../../../darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:1454), [`presentation_path_value`](../../../darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:369), and directive path analysis in [`directive_targets.rs`](../../../darkmatter/lib/src/markdown/compose/directive_targets.rs:238) | Prefix normalization and presentation decisions | Strip or recognize `doc.` | Apply the corresponding `fm` rules |
| Darkmatter binding classification, missing-property advisories, prepared validation, and subtree lookup | Their shared catalog and remaining subtree root lists | Catalog still reserves `doc`; `fm` is classified as a document property | Classify renamed and released roots consistently without evaluation |
| Claudine [`EXPRESSION_RESERVED_ROOTS`](../../lib/src/composition/reserved.rs:13) and its loop-variable extraction caller | Reserved name list | Still names `doc` | Use the specified frontmatter spelling |
| Claudine lifecycle sessions and approval scopes | Document lookup beneath declared `err`, `timing`, and `group` globals | Use unchanged Darkmatter lookup; tests still use `doc.err` / `doc.group` | `fm.err`, `fm.timing`, and `fm.group` read frontmatter; globals retain precedence |
| DMLS [`document_root`](../../../darkmatter/dmls/src/overlay/expressions.rs:588), descriptor-based validation/hover, and bracket quick-fix generation | Catalog selection and replacement construction | Frontmatter descriptor and generated bracket spelling remain `doc` | Select and describe `fm`; preserve ordinary-property treatment for released names |
| DMLS expression completion/hover in [`providers/frontmatter.rs`](../../../darkmatter/dmls/src/providers/frontmatter.rs) | Namespace member branches | No implemented `fm` root/member support | Complete `fm` and `fm.` members and expose its catalog description |
| [`darkmatter.yaml`](../../../darkmatter/schemas/darkmatter.yaml:4), its `partials/doc.yaml`, and schema-corpus path assertions | Authored root/type/import spelling | Still `doc` and `partials/doc.yaml` | `fm`, `partials/fm.yaml`, and updated path assertions |
| Claudine mutation and side-effect schema partials | Description of the mutation destination | Still describes the `doc` namespace | Describe `fm` |
| Shipped [`clarify.md`](../../../prompts/clarify.md:38) and its frozen fixture | Authored expressions accessing the specification property | Both still use `doc.doc` | Migrate to `doc` or `fm.doc` and exercise the frozen copy through normal composition |
| Current Darkmatter/Claudine topic pages, inline docs, DMLS diagnostics docs, and named skill pages | Namespace tables, examples, root diagram, and environment explanations | Still document old roots; no completed migration audit | Update the complete current-document inventory and leave historical records intact |

**Test gap and remedy.** The current root-catalog test explicitly asserts the old five-root list. The existing namespace tests exercise `doc`, not the new contract. Acceptance criterion 2's bare snapshots, caller overrides, lifecycle writes, bracket dependencies, cycle behavior, and shell-token behavior still need tests of `fm`; hook templates, conditions, matchers, and report messages need tests of `hook`. Criteria 3–4 and 6 also need ordinary-property/schema behavior, shadowing, and editor coverage using the new catalog. These are Level 1 requirements: no OS keyboard injection is necessary for expression or language-server behavior. Implement and migrate the whole inventoried class before repeating the review, rather than fixing only the first missing lookup.

### High — Per-document environment capture and its acceptance tests are not implemented

**Authority: spec.** Decided section 4 says “Each new document captures its own: each sequence task, each group leg, and each `proxy` target” and requires all environment consumers of a document to read the same captured map. Acceptance criterion 5 requires removal of `current_env`, stability across lifecycle events, loops, retries/resumes, and supplied-map tests at each document-entry boundary. Criterion 8 requires updating and running the existing Level 2 lazy-root test for `current` only.

**Defect class:** environment ownership remains attached to the invocation or live process instead of being captured and consistently distributed at document entry.

The supplied-map feasibility test passes, but its caller explicitly replaces the environment on a cloned file-resolution context. The production sequence/group/proxy routes do not perform that operation. It therefore proves the existing building blocks can support the feature, not that those routes implement it.

The runtime probe also composed frontmatter `current_env: {HOME: userland}` with body `[{{ current_env.HOME }}]` and an empty supplied environment. It returned the ambient process home instead of `[userland]`. This directly demonstrates that the released name still has live-root behavior. No review probe changed the real process environment.

| Site and purpose | Shape checked | Observed result | Required result |
| --- | --- | --- | --- |
| Darkmatter [`ComposeRequest::with_context`](../../../darkmatter/lib/tests/l1/request_context_builder.rs:255), explicit document preparation | Existing fixture: map A for document/transclusion, manually supplied map B for second document | Test passes; expressions, file-reference expansion, and context environment agree; first request remains unchanged | Clean API building block; production callers must perform the replacement at document entry |
| Darkmatter catalog, [`CurrentScope`](../../../darkmatter/lib/src/markdown/compose/context/current.rs:509), and refresh capabilities | Runtime ordinary `current_env.HOME` fixture; source inspection of scanner and refresh path | Runtime reads ambient home; scanner and refresh method still exist | Ordinary frontmatter lookup; no environment refresh capability |
| Darkmatter preflight lazy-root detection and expression errors | `current_env` classification and nesting-error branches | Still classifies the root as lazy and names it in reserved-root errors | No special preflight or approval behavior for the ordinary property |
| Claudine [`LATE_NAMESPACES`](../../lib/src/composition/lifecycle/bindings.rs:48) and approval diagnostics | Source inspection of late-root declaration | Still `["current", "current_env"]` | Exactly `["current"]` |
| Claudine [`request::initialize`](../../cli/src/request.rs:31) and [`InvocationContext`](../../lib/src/invocation_context.rs:1003) | Source inspection of capture/storage | One launch snapshot is stored in `OnceLock`; invocation copies its environment | Keep launch-owned facts, with a separate environment map captured for each entered document |
| Claudine serial sequence tasks and serial group legs through [`prepare_task_context`](../../cli/src/commands/wrap/sequence/task_run.rs:209) | Source inspection of derivation and overrides | Derives from the same invocation; writes overrides only to the composed context environment | Enter each document with its own map, shared by expression/context/file/schema/approval consumers |
| Claudine parallel group legs through `open_group_run` and task preparation | Source inspection of group capture and leg preparation | Group capture is shared; no per-leg environment replacement | Each leg captures its own map without changing unrelated repository evidence |
| Claudine proxy commit and target resolution in [`coordinator/commit.rs`](../../lib/src/composition/coordinator/commit.rs:86) and [`lifecycle/control.rs`](../../lib/src/composition/lifecycle/control.rs:291) | Source inspection of target handoff | Reuses invocation/request context; no new target environment input | Target lifecycle receives its own captured map |
| Claudine provider/model selection in [`select.rs`](../../lib/src/composition/select.rs:161) | Source inspection of six process-environment reads | Selectors still query `std::env::var` | Preparation consumes the document map |
| Claudine loop configuration in [`looping/config.rs`](../../lib/src/composition/looping/config.rs:19) | Source inspection of fail-fast, iteration-limit, and pause-margin reads | Still queries live process environment | Consume the document's stable supplied inputs |
| Claudine lazy context evidence in [`refresh_evidence`](../../lib/src/invocation_context.rs:1663) | Source inspection of agent/model evidence refresh | Uses `std::env::vars()` | Environment-derived context facts agree with the document map while permitted non-environment facts can refresh |
| Claudine sequence/proxy snapshot tests | Source inspection of existing environment-mutation fixtures | Tests continue to assert request-wide reuse, rather than document capture with supplied maps | Supplied-map tests prove the new boundaries without mutating process environment |
| Claudine [`level2_ac28_lifecycle_lazy_roots.rs`](../../cli/tests/level2/level2_ac28_lifecycle_lazy_roots.rs:386) | Test declarations and generated lifecycle expressions | Still tests live `current_env` behavior | Update to `current` only and execute through `just test-l2` |

**Test gap and remedy.** The strongest new relevant evidence is a Level 1 manual context-replacement test. There is no implemented acceptance coverage proving capture on sequence tasks, serial/parallel group legs, or proxy targets, nor the required whole-lifecycle reuse across approvals, preparation, loop iterations, retries/resumes, handlers, and finalize. These environment semantics need Level 1 supplied-map integration tests through the actual document-entry routes. Preserve the specified Level 2 check of the remaining lazy `current` behavior. A passing feasibility test cannot substitute for either set.

**Partially blocked by the author decision.** The plan expressly requires an author ruling before implementing environment-dependent sequence approval. Commands are currently approved as exact text; the later template-approval feature is not built. Choose the interim approval contract described in the frontmatter, or implement the prerequisite first. This decision does not block the namespace migration, removal of the live root, or the rest of the implementation preparation. It also does not establish permission to execute changed command text under an old approval.

## Verification and scope

Reviewed the full specification, implementation plan/log, production root readers and their callers, catalog consumers, document-entry routes, changed feasibility test, test declarations, and the current-artifact inventory. No manifest/configuration deserializer was added or changed by this implementation. The requested input robustness matrix is therefore inapplicable; expression grammars and renderers are expressly excluded.

Executed on the current macOS checkout:

- `cargo nextest run -p darkmatter --test l1 an_environment_only_replacement --color never`: one test passed. Its target is declared despite `autotests = false`, its module is included by `tests/l1/main.rs`, and its name selects Level 1.
- Targeted Darkmatter library tests for the root catalog, seed/effective-state namespace lookup, and shortcut conditions: four tests passed. They assert the old contract and do not establish feature readiness.
- Temporary public-API sweep covering Darkmatter composition/conditions/effective lookup, Claudine loop/sequence lookup, hook adapters, templates, and matcher execution: results recorded above. No terminal/browser window, provider, or shell action was launched.

The implementation log records passing Claudine L1/lint and Darkmatter lint during the feasibility phase, but explicitly says the full Darkmatter L1 gate was not run. The final Darkmatter/Claudine `just test` and `just lint`, updated Claudine `just test-l2`, and prompt-corpus checks required by acceptance criterion 8 are still pending. This review did not rerun those broad gates against an implementation whose production phases are absent; their absence is not claimed as an independent runtime failure. Complete them after the findings are addressed. Cross-OS evidence is left to CI and does not affect this readiness decision.

## Observations

### Hook templates already reject direct reads through the old payload namespace

Claudine's [`interpolate`](../../lib/src/dispatch/template.rs:471), which fills dispatch/report messages, checks a simple variable against `is_known_path` before evaluation. That list excludes bare `doc` and `doc.*`, so the public probe preserves `{{ doc.tool_name }}` and `{{ doc }}` literally even though the old hook adapter resolves them. This predates this feature and is an observation, not a regression or a separate repair demand. When implementing the required `hook` support, include the template guard in the migration so adapter-only tests cannot leave direct `{{ hook.tool_name }}` messages broken.
