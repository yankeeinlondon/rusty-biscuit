# Functionality Matrix

Current implementation as of September 21, 2026. **✓** marks the package that defines and implements the core functionality. Numbered footnotes identify packages that supply supporting behavior, data, or host-specific extensions. Calling a feature or controlling when it runs does not by itself establish ownership or a contribution. **—** means no separate contribution identified within the row's scope; the package may still consume the feature. Supporting packages are named in **Other**; this is not an inventory of all transitive dependencies.

| Functionality          | Claudine | Darkmatter | Other |
| ---------------------- | -------- | ---------- | ----- |
| Context Variables      | see [^1]  | ✓          | Sniff [^2] |
| Safe Side Effects      | see [^3]     | ✓          | biscuit-file [^4] |
| Lifecycle Events       | ✓        | see [^5]       | biscuit-speaks, Playa, Messenger [^6] |
| Expression Parser/Lexer | —        | ✓          | — |
| Preflight Checking     | see [^7] | ✓          | — |

Ownership boundaries:

- **Context Variables:** Darkmatter owns the shared context catalog, capture, and lookup machinery in [`context`](../../../darkmatter/lib/src/markdown/compose/context/mod.rs). Sniff supplies observed host/repository facts; Claudine supplies invocation facts and event/loop bindings. These contributions extend the shared context system without owning its catalog or capture machinery.
- **Safe Side Effects:** Darkmatter owns [`EffectEngine`](../../../darkmatter/lib/src/effects/mod.rs), its mutation verbs, guarded filesystem writes, and HTTP host restrictions. This row concerns that explicit effect API; bespoke shell actions and communication channels belong to Claudine's lifecycle orchestration. “Safe” describes enforced boundaries, not an absence of external mutations.
- **Lifecycle Events:** Claudine owns the document lifecycle's event definitions, stack grammar, validation, execution, and control flow in [`composition/lifecycle`](../../lib/src/composition/lifecycle/mod.rs). Its normalized provider-hook events are also Claudine-owned, in [`events`](../../lib/src/events/mod.rs), but are a distinct event model.
- **Expression Parser/Lexer:** Darkmatter owns the [`lexer, parser, AST, and evaluator`](../../../darkmatter/lib/src/markdown/compose/expression/mod.rs). Claudine consumes this grammar without implementing or extending its lexer or parser. Its event/loop lookup adapters contribute variable bindings, covered under Context Variables [^1].
- **Preflight Checking:** Darkmatter owns the shared [`document preflight`](../../../darkmatter/lib/src/markdown/compose/preflight/mod.rs): document traversal, shell discovery, approval-set construction, and the shell-policy machinery used to authorize commands. Its composition runtime checks commands against the supplied approval set. Claudine extends collection to lifecycle and sequence actions, adds host-specific restrictions, and connects approval results to provider launch [^7]. Owning that launch decision does not make Claudine the owner of the underlying checking functionality.

These assignments describe shipped source, not the proposed ownership transfer in **2026-09-22-lifecycle-events**. That proposal would move reusable document lifecycle language and execution into Darkmatter. The ergonomic changes in **2026-09-21-lifecycle-ergonomics** are likewise not treated as implemented here.

[^1]: **Claudine → context variables.** [`InvocationContext`](../../lib/src/invocation_context.rs) supplies captured launch/repository facts, while [`CompositionPrepContext`](../../cli/src/commands/wrap/composition/prep_context.rs) carries provider-selection state. At lifecycle execution time, [`lifecycle_injected_globals`](../../lib/src/composition/lifecycle/context.rs) adds `err`, `timing`, and lazy `current` bindings through Darkmatter's injected-global interface. Its [`event lookup adapters`](../../lib/src/dispatch/expression.rs) and [`LoopExpressionLookup`](../../lib/src/composition/looping/expression.rs) also expose hook payloads and loop state to the evaluator. For example, a failure action can inspect `err`, a hook condition can read `tool_input`, and a loop condition can read its current counter. These are contributions to the available bindings, not to expression syntax.

[^2]: **Sniff → context variables.** Darkmatter's [`context capture snapshot`](../../../darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs) uses Sniff for OS, hardware, Git, repository/package, language, and document observations. Sniff supplies the facts; Darkmatter determines their exposed context names and capture behavior.

[^3]: **Claudine → safe side effects.** The [`lifecycle executor`](../../lib/src/composition/lifecycle/executor.rs) evaluates action arguments, resolves authored file references, invokes Darkmatter's effect verbs, and carries runtime `set` mutations into subsequent actions. Claudine adds host policy such as reserved state keys and lifecycle-phase restrictions; it also enforces the shell-free initialization/preflight boundary for its separate shell actions.

[^4]: **biscuit-file → safe side effects.** Darkmatter's [`http_post` implementation](../../../darkmatter/lib/src/effects/verbs.rs) delegates network enforcement to biscuit-file's `FetchPolicy`, `HostPattern`, `PolicyClient`, and `post_blocking`. For example, an HTTP effect must pass the configured host policy before sending. The guarded filesystem-write implementation itself remains in Darkmatter's [`fs_write.rs`](../../../darkmatter/lib/src/effects/fs_write.rs).

[^5]: **Darkmatter → lifecycle events.** Claudine stores Darkmatter `Expr` values for guards in its [`action model`](../../lib/src/composition/lifecycle/actions.rs) and uses Darkmatter's [`subtree composition and injected globals`](../../../darkmatter/lib/src/markdown/compose/subtree.rs) for deferred event-time evaluation. Darkmatter also supplies the effect verbs executed by the stack. For example, a failure event can interpolate `err` into a message and then update a document; Claudine decides when the event fires and how control proceeds.

[^6]: **biscuit-speaks, Playa, and Messenger → lifecycle events.** Claudine's [`lifecycle audio`](../../lib/src/composition/lifecycle/audio.rs) uses biscuit-speaks for speech and Playa for sound-effect playback/queued audio. Its [`messaging adapter`](../../lib/src/messaging/send.rs) uses Messenger for outbound messages and desktop notifications. These packages implement delivery for lifecycle communication actions; they do not define event ordering or lifecycle control flow.

[^7]: **Claudine → preflight checking.** [`composition/preflight.rs`](../../lib/src/composition/preflight.rs) combines Darkmatter's discovered document commands with Claudine lifecycle commands and manages invocation-scoped approval results. Its [`shell adapter`](../../lib/src/harness/shell.rs) reuses Darkmatter's tokenization and blacklist/whitelist checks. Claudine adds the shell-free initialization boundary, failure routing, and the [`sequence preflight`](../../lib/src/composition/sequence/preflight/mod.rs) extension that validates task/group structure and gathers task/setup/teardown commands. For example, a prompt's body shell directive is discovered by Darkmatter, while its `success` shell action is contributed by Claudine; both must pass approval before the relevant execution proceeds.

## Pre-flight Checking

- **Collect, validate, and authorize shell commands before execution.** Identify commands the document could run and establish which are permitted. A discovery error, policy violation, or denied approval blocks provider launch and is reported with available source details. For example, a prompt containing a shell directive cannot launch the agent until that command passes the approval process.
    - **Find commands throughout the document.** Inspect frontmatter shell expansions, body directives and blocks, lifecycle actions, and included documents. For example, a prompt's body shell directive and its `success` shell action both contribute commands for approval.
    - **Follow document includes.** Resolve relative paths from the document containing each reference and explicit repository-relative paths against that document's repository, so commands in included content are inspected too. For example, `./includes/context.md` in `prompts/review.md` leads preflight to inspect `prompts/includes/context.md`.
    - **Include commands behind conditions.** Inspect conditional sections even when their conditions currently evaluate to `false`, because those conditions may change before execution. For example, a shell action guarded by `release_ready` enters the approval process even when `release_ready` is currently `false`.
    - **Resolve the command text.** Substitute values available during preparation so approval covers the command that will execute. For example, a lifecycle shell action containing `echo {{ version }}` is approved as `echo 1.2.0` when `version` is `1.2.0`.
    - **Check policy and obtain permission.** Deduplicate commands, enforce blacklists and whitelists, reuse cached decisions, and request any remaining approvals. For example, the same command appearing in both a document and an included file shares an approval decision, while a blacklisted command blocks the run.
    - **Enforce the initialization boundary.** Audit lifecycle commands after initialization and the document reread; initialization and early failure/finalization handling cannot execute shells before preflight completes. For example, a shell action in `initialize` is rejected even if that command is whitelisted.
    - **Supply execution with the approval results.** Pass the approved command set to execution-time checks and retain resolved document references for reuse during composition. For example, a body shell directive can execute only if its resolved command appears in the supplied approval set.
    
- **For sequences, validate and preflight the entire sequence before its first step.** Check the sequence's structure and extend shell approval across all reachable work. A preflight failure blocks the whole sequence regardless of `fail_fast`; dry runs use the same static checks. For example, a missing task file in the final step prevents the first step from launching, even with `fail_fast: false`.

    - **Discover reachable tasks and prompts.** Follow task and group references, including work behind conditional steps. For example, a referenced group containing three tasks contributes all three tasks and their prompts to preflight.
    - **Validate the task structure.** Reject unsupported constructs, missing references, cycles, and naming collisions. For example, a task reference pointing to a nonexistent file causes preflight to fail before the sequence starts.
    - **Extend command collection to sequence actions.** Include task commands and setup/teardown shell actions alongside commands from their prompts in the shell approval process described above. For example, a setup command requires approval even when the task's prompt contains no shell directives.
    - **Reject shell commands that depend on future results.** Resolve command expressions before approval and reject dependencies on values only available during execution. For example, a shell command that reads a previous task's `outputs` is rejected because those results do not exist during preflight.



### Claudine's Use of Pre-flight Checking

- **Lifecycle shell discovery.** Add commands from Claudine's lifecycle stacks to the commands discovered in document content. For example, a prompt's `success` shell action must be checked alongside its body shell directives.

    - **Claudine supplies lifecycle knowledge.** Its lifecycle parser and collectors understand event stacks, conditional actions, and shell error handlers. They collect potentially executable commands regardless of whether an action's condition currently passes.
    - **Darkmatter supplies document discovery.** `Markdown::compose_preflight` receives the document and `ComposeOptions` and returns normalized commands with source locations, including commands in transcluded documents. It does not parse Claudine's lifecycle action grammar.
    - **The seam is the collected command set.** Claudine's [`resolve_shell_approvals`](../../lib/src/composition/preflight.rs) combines the two sources before deduplication and approval. Its shell adapter reuses Darkmatter's policy primitives; collecting a command does not itself authorize it.

- **Lifecycle execution rules.** Restrict shell execution according to Claudine's lifecycle phase. For example, a whitelisted command is still forbidden in `initialize` or in a failure handler reached before preflight completes.

    - **Claudine owns phase eligibility.** It rejects initialization shell actions during parsing and maintains a runtime backstop for early failure/finalization paths. It audits later lifecycle commands after initialization and the stabilized document reread.
    - **Darkmatter owns shared command-policy checks.** Its tokenizer and blacklist/whitelist machinery answer whether a command passes shell policy. Those checks do not determine whether Claudine has reached a phase where shells may run.
    - **The seam separates eligibility from authorization.** Claudine's [`lifecycle executor`](../../lib/src/composition/lifecycle/executor.rs) enforces the phase restriction, while its [`shell adapter`](../../lib/src/harness/shell.rs) applies the shared policy checks. Approval cannot override a lifecycle prohibition.

- **Sequence-wide checking.** Extend preflight across all reachable sequence work before the first step runs. For example, a missing task file in the final step blocks the sequence before the first agent launches.

    - **Claudine owns the task structure.** Its [`sequence preflight`](../../lib/src/composition/sequence/preflight/mod.rs) resolves task/group references, checks structure, retains referenced prompts, and gathers task/setup/teardown shell commands. It rejects shell expressions that depend on future results such as `outputs` and retains resolved command text for execution.
    - **Darkmatter checks each referenced document.** Claudine calls document preflight with the appropriate compose options for each prompt, letting Darkmatter discover its shell directives and included documents. Darkmatter does not need to understand task ordering or group semantics.
    - **The seam joins task commands with document commands.** [`resolve_graph_shell_approvals`](../../lib/src/composition/preflight.rs) combines Claudine's already-resolved task commands with Darkmatter's document discoveries for approval. Claudine's task graph and Darkmatter's document-inclusion graph describe different relationships and remain separate structures.

- **Agent launch and recovery integration.** Connect preflight results to provider launch, diagnostics, and lifecycle failure handling. For example, denied shell approval prevents the prompt from reaching the agent and can trigger Claudine's `blocked` and `finalize` handling.

    - **Claudine owns the invocation.** It supplies the approval handler and invocation-scoped decision cache, decides whether launch may proceed, and carries preflight failures into its diagnostic and recovery model.
    - **Darkmatter consumes approval results during composition.** The approved command set is supplied through `ComposeOptions::pre_approved_commands`; document shell execution checks membership in that set. Darkmatter reports composition errors without deciding how an agent session should recover.
    - **The seam carries results and typed errors.** Claudine's [`preflight adapter`](../../lib/src/composition/preflight.rs) wraps discovery and approval failures in `CompositionError`; its [`preflight failure routing`](../../cli/src/commands/wrap/composition/preflight.rs) supplies lifecycle error context and runs the appropriate handlers. Lifecycle shells still obey Claudine's runtime phase and shell-policy checks.
