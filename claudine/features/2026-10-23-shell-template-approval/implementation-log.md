---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-23-shell-template-approval/spec.md"
plan: "claudine/features/2026-10-23-shell-template-approval/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - claudine
    - claudine-cli
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/matching.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
message_to_agent: |-
    Phase 3 landed discovery provenance and structured approval transport (details in the "Phase 3" log section). What Phase 4 needs:

    1. Template execution is the only missing link. `prepare_directive` (darkmatter `shell_expansion/mod.rs`) admits only exact members of `ShellPreApprovals`, so a site approved only by its shape is refused with `NotPreApproved` at execution. Add the template path there: re-parse with `parser::authored_template` / `parse_authored`, re-type against the owner schema, require `contains_template(&shape)`, then fill. Never insert the resolved bytes.
    2. `ShellExpansionError::MissingValue` and `TemplateGrammar` exist, and both are fatal through `shell_input_violation()`. `TemplateValueRejected` and `TemplateTarget` are still to be added, with their first producer. The template-mode result-cache key (C7) is also still to be added.
    3. Sequence wiring (C8) is untouched. Sequence preflight still raises `SequenceShellLateBinding`. Pass `ComposeOptions::with_deferred_shell_roots(DeferredShellRoots::new(["outputs", "loop", "group"]))` (plus any task parameter names derived from late roots) to the task-document discovery requests. Approve their `TemplateRequired` candidates; `claudine::composition::preflight::approve_commands` already does this through `harness::shell::validate_and_approve_template_site`. Sequence task `shell:` and stack commands do not go through Darkmatter discovery, so build their candidates with `parse_authored` + `project_types` yourself.
    4. A frontmatter `$( … )` with a deferred hole still hits `DynamicCommandShape` in `scan_one_frontmatter` (`detect_dynamic_frontmatter_command_shape`) when the root is unbound. This fails closed. Relax it only together with frontmatter template execution.
    5. The `md` prompt (`darkmatter/cli/src/approval.rs`) and Claudine's CLI handler do not yet show `request.template` or offer the shape choice. The batch path (`compose_preflight_approvals`) already persists every row, including `AllowTemplatePersist`, and treats an unoffered choice as `Denied`. Prompts must read `request.offered`.
    6. Discovery links a site to its resolved bytes by document order (body, shell-block) or by key (frontmatter). On a count mismatch the site stays unlinked (`resolved: None`) and its bytes are approved as plain entries.
    7. Claudine's session cache is `HashMap<ApprovalKey, _>`. A policy write failure is `HarnessError::ShellPolicyWriteFailed` (`io.write_failed`) and caches nothing.
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/approval.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shell_template_discovery.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/cli/src/approval.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/audit.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/coordinator/tests.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/shell_missing_values.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/docs/topics/pre-flight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/cli-commands.md
---

# Implementation Log for 2026-10-23-shell-template-approval (6 phases)

## Phase 1

- 2026-10-14: started Phase 1 (contract review, no source changes planned).

### Dependency Gate — `2026-10-16-core-schema-types`

- The dependency is complete: all 17 plan checkboxes are checked and its log ends "implementation complete, ready for review". Its plan frontmatter still says `phase: 4`; that is bookkeeping drift in the dependency, not missing behavior.
- **The covers relation** is `darkmatter::markdown::schemas::is_subtype(a: &PropertyAtom, b: &PropertyAtom) -> bool`, defined in `darkmatter/lib/src/markdown/schemas/simplified/subtype.rs:40`. It answers "is `a` a subset of `b`?", so "the rule type covers the declared type" is `is_subtype(&declared, &rule)`.
    - Both atoms must already be resolved. An unresolved `TypeExpr::Imported` equals nothing.
    - It ignores ordinary constraints, presence markers, and descriptions.
- **Verified:** `cargo nextest run -p darkmatter -E 'test(/subtype/)'` ran 7 tests and all 7 passed. These are the 6 unit tests in `subtype.rs` and the L1 test `tests/l1/schemas_core_type_values.rs::subtype_relation_is_reachable_through_the_public_schema_api`.
- **Departure the contract must handle:** `is_subtype` returns true for any two identical types (`identical_types_are_subtypes_whatever_the_type`). Spec §7 instead says "unsupported pairs are refused even when identical", and that unions, containers, and `unknown` are never string-like.
    - Template matching therefore gates `is_subtype` behind its own check that the *rule* atom is a supported string-like, non-array, single atom.
    - The broad `string` rule is a separate check, not `is_subtype` (§7 calls it "a shell-template matching exception").
    - This adds no second subtype relation; it only restricts which inputs reach the dependency's relation.
- **Supporting API from the dependency and from Darkmatter**
    - Parse a type: `simplified::grammar::parse_type_expr(property, text) -> Result<PropertyAtom, SchemaError>`. It parses constraints after `;` and rejects options written after `;`, mixed options, unknown options, and repeated options.
    - Print a type: `serialize_property_atom(&PropertyAtom) -> String`. This is the canonical rule text once constraints and descriptions are stripped.
    - Read canonical options: `PropertyAtom::alphanumeric_options()` returns them, and `AlphanumericOptions::canonical()` drops redundant blocks.
    - Look up a declaration: `EffectiveSchema.simplified: Option<SimplifiedSchema>`, then `Single(shape).properties.get(name)`, which gives `PropertyDef::{Single, Union}`. Root unions and layered schemas go through `EffectiveSchema::property_shape` or `property_def_at_path`.
    - Validate one value against one atom: `schemas::completion::alternative_accepts(&PropertyAtom, &Value)`. It is documented as passive. For the original-value check, Phase 2/4 must confirm it enforces constraints (`min`, `pattern`) and does not coerce. If it does not, they must use the `EffectiveSchema` validator on a one-property document instead. **Open for Phase 2**, not a product question.
    - Convert a scalar to text: `compose::expression::scalar_string(&Value)`, the existing conversion engine. Null becomes `""`, and arrays/objects become JSON text. Template mode must therefore reject null, arrays, and objects *before* calling it.

### Sequence and group surfaces today (Dependency Gate, second half)

`2026-10-24-sequence-containers` is planned but unimplemented: its plan says `phase: 1`, it has no log, and it has no code. Today's names are therefore the ones this feature uses (plan rule 3).

| Spec name | Spelling today | Exists today? |
| --- | --- | --- |
| `seq.outputs` (late root) | `outputs` (`OUTPUTS_KEY`, `runtime_state.rs:43`; `ROOT_OVERLAY_KEYS`, `sequence/reserved.rs:31`) | yes |
| `seq.*` other fields (approval-time) | `state.*`, `previous`, `next`, `sequence_id` | yes |
| `loop.*` (late root) | `loop.{count,is_first,is_last,last_output,last_exit_code}` (`composition/reserved.rs:4`) | in documents yes; task/group `loop:` no |
| `group.*` (late root) | `group.<var>` (`GROUP_SCOPE_KEY`, `task/group.rs:83`); `legs`/`succeeded`/`failed`/`cancelled` do not exist | groups yes (serial or parallel, not nested, no group loop, no group lifecycle events) |
| nested sequence | rejected (`SequenceNestedSequence`, `sequence/preflight/mod.rs:779`) | **no** |
| container terminal event (`finalize`) | none | **no** |

**Deferred acceptance cases.** These depend on a surface that does not exist yet. They are recorded here rather than claimed. Phase 5 must list them as "not applicable until the container surfaces land".

- AC16 "when the first task is a nested sequence, only that sequence's own first task" gets resolved-value approval. Nested sequences are rejected today. Phase 3 should still build the position rule recursively, so that the containers feature only has to add a traversal.
- AC16 "a first group's `finalize` command is a template". Groups have no lifecycle events today. The rule "a container terminal event is always a template" is part of the contract, but there is no surface to test it on.
- AC2, AC9, and the §10 examples write `seq.outputs`. Tests use the current spelling `outputs` (for example `last(outputs)`). Whether the tests should be renamed is left to the containers feature.

These can all be tested today: first task as an action (with its stacks and its document), first task as a group (every leg), any `group.*` read, a skipped predecessor (position stays static), retry and resume, and a later group's legs.

### Boundary Inventory

The full inventory was gathered read-only, by tracing the code. Below is what each later phase owns, with file:line anchors.

#### Darkmatter (`darkmatter/lib/src/markdown/compose/`)

- **Discovery produces bytes only.** `ShellCommandEntry` (`shell_expansion/types.rs:277`) holds `{raw_command, executable, args, normalized, source_file, origin}`. It has no span, no owner schema, no authored template, and no record of which expressions it read.
    - `collect_recursive` (`preflight/collect.rs:304`) first composes the body with `Interpolation` enabled and then tokenizes the **already-interpolated** text. `scan_one_frontmatter` (`:852`) does the same.
    - Entries are de-duplicated graph-wide by `normalized` (`seen: HashSet<String>`, `collect.rs:183`), so the first origin wins. **Template mode needs per-site retention**: two sites with the same bytes can have different owner schemas and types.
- **Interpolation runs before tokenizing on every surface.** The authored shape is recovered only by masking:
    - frontmatter uses `mask_interpolations` (`frontmatter_shell_expansion.rs:1008`), which produces `__DM_INTERP_n__`;
    - the body and `::shell-block` use `DataRanges::masked` (`body_origin.rs:186/252`);
    - shape checks are `data_changed_shape` (`shell_expansion/parser.rs:183`), `reject_data_shaped_commands` (`shell_blocks/mod.rs:241`), `validate_pipeline_shape_preserved` (`frontmatter_shell_expansion.rs:1042`), and `validate_no_executable_interpolation` (`:1118`).

  These masks are the existing seam that Phase 2's template grammar replaces with real placeholder nodes.
- **The tokenizer** is `tokenize` (`shell_expansion/tokenize.rs:75`), which yields `ShellToken::{Word, And, Or, 4 curated redirects}`, followed by `parse_pipeline` (`:399`), which yields `ShellPipeline`. `{`/`}` are ordinary word characters.
- **Null handling today:** null becomes `""` through `scalar_string` (`expression/mod.rs:395`). After tokenizing, `ls {{ x }}` with `x = null` becomes `ls` with **no argument at all**. That is the §1 defect.
- **Byte-only approval keys**
    - `ComposeOptions.pre_approved_commands: Option<HashSet<String>>` (`context/options.rs:331`, setter `:1445`, identity hash `:2917`).
    - Where it is checked: `prepare_directive` (`shell_expansion/mod.rs:244-264`, error `NotPreApproved`) and `validate_pre_approved` (`preflight/mod.rs:295`).
    - `ComposePreflightApprovals.pre_approved_commands: HashSet<String>` (`preflight/lifecycle.rs:60`); `approval_set()` (`preflight/approval.rs:18`).
    - `SharedShellExpansionRuntime` (`types.rs:1118`): `allow_once` and `pending_allow_once` (`HashSet<String>`), plus `reserve_allow_once` / `complete_allow_once` (`:1261/:1320`) and `ReservationGuard` (`shell_expansion/mod.rs:67`).
- **The result cache** is separate and also uses a string key: `cache_key` (`shell_expansion/mod.rs:483`). The key is the per-action `normalize_command`, joined with the real `&&`/`||` and the redirection text, plus cwd, strip_ansi, timeout, and timeout behavior. It is read through `cached_outcome` (`types.rs:1394`).
    - **Gap for template mode:** `normalize_command` (`policy.rs:279`) escapes `\` and `"` inside quoted arguments. However, it does not quote an empty argument: `[""]` renders as a trailing space. Template mode makes empty arguments routine (spec §4). The template-mode result key is therefore built from explicit argument boundaries, never from a rendered string. (An inventory claim that `normalize_command` fails to escape `"` was checked against the source and is wrong.)
- **Display drift:** `normalized_exact` in the prompt request joins a chain with `" && "` even when the operator is `||` (`shell_expansion/mod.rs:380`). Template shapes must carry the real operators.
- **Executor** (`shell_expansion/executor.rs`):
    - It already dispatches directly: `run_actions` (`:283`) short-circuits `&&`/`||` in Rust, and `run_action` (`:414`) uses `Command::new(resolved).args(args)` with `which`/`which_in` and no `sh -c`.
    - `configure_streams` (`:614`) maps the curated redirects.
    - It does **not** refuse `.cmd`/`.bat` targets today. On Windows, Rust's `Command` routes `.bat`/`.cmd` through `cmd.exe` implicitly. Phase 4 adds the refusal for template mode only.
- **Suppression boundaries**
    - `handle_errors` (`shell_expansion/mod.rs:669`), which applies `ErrorHandling::resolve` to `ExecutionFailed` only.
    - Timeout `EmptyString` (`:646`).
    - Transclusion notices in `pipeline/phases.rs` (`fit_notice_to_span` `:515`, guarded by `transclusion_failure_is_fatal` `:~536`, which already treats `pre_approval_violation`, `unavailable_binding`, and `shell_span_failure` as fatal).
    - Nested `as_markdown` (`nested.rs:248` `is_structural`), where an ordinary shell failure can be absorbed by `||`.

  The new typed template errors must be added to `transclusion_failure_is_fatal` and `is_structural`, and must not be `ExecutionFailed`.
- **Policy store** (`shell_expansion/store.rs`): `load_ruleset` (`:85`) accepts `exact`/`prefix` and skips malformed lines with `tracing::warn!`. It reports **no line number**; the spec requires file and line. It de-duplicates by sorting. Invalid UTF-8 already becomes `PolicyIo`. All appends go through `append_rule` (`:225`).
- **Discovery seams:** `BindingView` (`expression/binding.rs:247`), `ComposeOptions::with_host_globals` (`context/options.rs:1949`), `RESERVED_ROOTS` (`expression/catalog/roots.rs:62`), and the existing refusals `DynamicCommandShape` / `UnevaluatedDependencyShape` (`collect.rs:1008/1375/1551`), pending-shell inheritance (`collect.rs:1051-1137`), and `detect_authored_dynamic_target` (`:1155`).
- **The `md` CLI prompt** is in `darkmatter/cli/src/approval.rs`: `CliShellApprovalHandler`, `approve_with_io` (`:29`), `write_prompt` (`:72`), and `can_prompt_interactively` (`:190`).
    - **Pre-existing defect:** `Markdown::compose_preflight_approvals` (`preflight/lifecycle.rs:166-171`), which is the only path `md compose` uses, treats `AllowExactPersist`/`AllowCommandPersist` like `AllowOnce` and **persists nothing**.
    - AC8 requires every choice to persist its §8 row in both prompts, so Phase 4 (Approval Prompts) must route this path through the store.

#### Claudine (`claudine/lib/src/`, `claudine/cli/src/`)

- **Byte-only sets**
    - `PreFlightResult.approved_commands: HashSet<String>` (`composition/preflight.rs:31`), built by `approve_discovered_commands` (`:198`) through `harness::shell::validate_and_approve_command_parts` (`harness/shell.rs:141`).
    - `PrepareOptions.pre_approved_commands` (`composition/prepare.rs:103`) and `CallerInputLayers.pre_approved_commands` (`types.rs:525`, with `add_approved_commands` `:632`).
    - `SharedApprovalCache = HashMap<String, CachedApprovalDecision>` (`types.rs:23`).
    - Handed to Darkmatter at `prepare.rs:427-428`; bootstrap is empty at `:938`.
    - Lifecycle `set` values: `ResolvedShellValue::commands()` goes into a fresh `ComposeOptions` (`lifecycle/executor.rs:357-364`).
- **CLI flows**
    - compose/wrap: `compose/prep.rs:844, 1524, 1535`; staged boot `wrap/composition/staged_boot.rs:85-88, 334-351`; harness lifecycle `wrap/composition/runner.rs:245`.
    - sequence: `wrap/sequence/mod.rs:65` (`approve_preflight_graph`), seeding `phase1c.rs` and then `SequenceRunContext.approved` (`mod.rs:715`). `jit.rs:167/212-236` extends it per step, and prompt tasks re-enter at `task_run.rs:611`.
- **Re-audit paths**
    - Loop: `compose/prep.rs:1761` (`audit_loop_iteration_template`) and `:1783` `frozen_loop_approvals`.
    - Retry/resume/proxy: `harness_orch/loop_control.rs:1386-1395, 1012, 1043, 1766` (freeze) and `prompt.rs:265, 286, 420-441`; thaw in `harness_orch/shell_options.rs:204-237`.
- **Sequence preflight** (`composition/sequence/preflight/`)
    - `Loader::walk_plan` (`mod.rs:300`) and `load_task` (`:389`), with groups at `load_group`/`build_group` (`:533/:592`) and external tasks at `load_external_task` (`:496`).
    - Shell bytes come from `resolve_shell_bytes` (`:886-923`). It raises `SequenceShellLateBinding` (`composition/error/mod.rs:1868`) for the roots declared `Unavailable(PREFLIGHT_UNAVAILABLE)` in `lifecycle/bindings.rs:181-199` (`outputs`, `err`, `timing`, `group`, `loop`); `current` is refused through `LATE_NAMESPACES` (`bindings.rs:79, 430-439`).
    - **Gap:** task-stack whole-value `set $( … )` is not collected by `collect_lifecycle_shell` (`:857`, `shape::collect_stack_shell_commands` `shape.rs:197`). Today it fails at run time ("pre-flight never resolved", `lifecycle/executor.rs:1556-1566`). AC16 requires `set` shell values in later tasks, so Phase 3 (Sequence Position) collects them.
    - **Static position is not tracked.** `PreflightStep.index` (`shape.rs:18`) and member indexes exist but are not threaded into approval. Phase 4 adds this.
- **Runners**
    - Sequence `TaskShellRunner` / `SystemTaskShell` (`sequence/task/shell.rs:222/279`) runs `sh -c` or `cmd /D /C`. It uses a Unix process group or a Windows kill-on-close Job Object, has a 30 s default deadline, honors interrupts, and has volume caps.
        - The structured template path **must reuse this process-tree machinery** (`:812-857, :918-941`) and change only how argv is built.
    - Lifecycle `ShellRunner` (`lifecycle/executor.rs:295`): `SystemShellRunner`/`DocumentShellRunner` go through `run_system_shell` (`:452`), which inherits stdio and has **no deadline, no process tree, and no interrupt check**.
        - "Retain its contracts" therefore means the structured lifecycle path keeps exactly that: inherited stdio, the same cwd and environment, no new deadline.
        - Adding deadlines to lifecycle shells is out of scope.
    - `DisabledShellRunner` (`:384`) must refuse the new structured method too.
- **Approval handler:** `validate_and_approve_command_parts` (`harness/shell.rs:141-351`) checks the built-in blacklist, then the user blacklist, the whitelist, the session cache, and finally the handler. It is installed in `cli/.../harness_orch/shell_options.rs:96-155` (`CliShellApprovalHandler` when there is a TTY, `YoloApprovalHandler` for `--yolo`, otherwise none). With no TTY the result is `ShellApprovalUnavailable{NoHandler|DryRun}` (`composition/preflight.rs:246-259`).
- **Comments that will drift when template mode lands.** Phases 3 and 4 must fix them in the same change that alters the behavior:
    - `sequence/preflight/mod.rs:18-21, 61-63, 850-852, 886-901, 913-917`
    - `sequence/preflight/shape.rs:99-102, 148, 261-267`
    - `sequence/task/shell.rs:3-4`
    - `sequence/task/mod.rs:703-704, 934-938, 954-959`
    - `sequence/preflight/reapproval.rs:1-11`
    - `composition/preflight.rs:124-126, 418-423`
    - `composition/prepare.rs:894-898`
    - `composition/error/mod.rs:1861-1867, 1920-1923`
    - `cli/src/commands/wrap/sequence/mod.rs:346-348` and `task_run.rs:281-285`
- **Existing tests to keep green** (AC12 corpus)
    - Darkmatter: `compose/tests/{shell,preflight}.rs`, `frontmatter_shell_expansion/tests/*`, `lib/tests/l1/{shell_*,transcluded_shell_failure,preflight_*,interpolation_literal_pipeline,error_snapshots/shell_expansion}.rs`, `cli/tests/l1/compose_shell.rs`.
    - Claudine: `composition/sequence/{preflight,task,task/shell}/tests.rs`, `composition/preflight/tests.rs`, `lifecycle/{bindings,executor}/tests*`, `cli/tests/l1/{sequence_cli,sequence_jit,sequence_groups,sequence_errors_cli,lifecycle_set_shell_values,wrap_compose_validation}.rs`.

### Contract Definition (Wave 2)

This is the shared design the later waves build against. The names are binding for the shared API. Private helpers may differ. Anything not stated here is left to the implementing wave, which records its choice in this log.

#### C1. Module placement and single owner

- All shared template types live in one new module, **`darkmatter/lib/src/markdown/compose/shell_expansion/template/`**, exported as `darkmatter::markdown::compose::shell_expansion::template`.
    - `mod.rs` holds the types and exports.
    - `grammar.rs` holds the authored parser.
    - `canonical.rs` holds the rule text encoder and decoder.
    - `typing.rs` holds the type projection.
    - `matching.rs` holds the whitelist and blacklist matching.
    - `fill.rs` holds the resolver, validator, and filled chain.
    - `target.rs` holds direct-dispatch executable resolution.
- **Owners**
    - Phase 2 "Template Grammar" owns `template/mod.rs` and the `shell_expansion/types.rs` additions: the `ShellRuleEntry::Template`, `ShellApprovalDecision::AllowTemplatePersist`, and `ShellExpansionError` variants.
    - Every later task consumes these and does not redefine them. A needed change to a shared type goes into the log first, then into `mod.rs`.

#### C2. Authored template (Phase 2)

```rust
pub struct CommandTemplate { pub authored: String, pub segments: Vec<TemplateSegment> }
pub struct TemplateSegment {
    pub operator: ChainOperator,          // existing type; None for the first
    pub program: String,                  // always literal; a hole here is a grammar error
    pub args: Vec<TemplateArg>,
    pub redirects: Vec<RedirectOp>,       // authored order retained
}
pub struct TemplateArg { pub parts: Vec<ArgPart> }  // `""` is one empty Literal part, never zero parts
pub enum ArgPart { Literal(String), Hole(Placeholder) }
pub struct Placeholder {
    pub expression: String,               // authored text inside `{{ }}`, trimmed
    pub span: std::ops::Range<usize>,     // byte span in `authored`, for diagnostics
    pub timing: HoleTiming,               // filled by discovery (C5)
    pub typing: HoleType,                 // filled by type projection (C3)
}
pub enum RedirectOp { StdoutNull, StderrNull, StderrToStdout, StdoutToStderr }
pub enum TemplateGrammarError { ProgramWordHole, RedirectHole, Pipe, Semicolon, Background,
    InputRedirect, ArbitraryRedirect, CommandSubstitution, Unterminated, ControlCharacter, Empty }
```

- `parse_authored(text) -> Result<CommandTemplate, TemplateGrammarError>` recognizes each complete `{{ … }}` span **before** it interprets quotes, whitespace, or operators.
    - It reuses the existing tokenizer's rules for quoting and for the four curated redirects.
    - `RedirectOp` lowers to the existing `RedirectionConfig` with the same dup semantics `parse_pipeline` uses. Execution is therefore unchanged.
- Resolved-byte mode keeps the existing tokenizer and its grammar exactly as they are.

#### C3. Typing (Phase 2 "Type Projection")

```rust
pub struct HoleType { pub approval: ApprovalType, pub validator: HoleValidator, pub provenance: TypeProvenance }
pub enum ApprovalType { BroadString, StringLike(PropertyAtom) } // atom: resolved, non-array, constraints+description stripped, canonical options
pub enum HoleValidator { FallbackScalar, Declared(PropertyDef) }  // Declared keeps constraints, unions, named types resolved
pub enum TypeProvenance { BareProperty { name: String }, NoProjection, Undeclared, NotBareProperty, ReservedRoot }
```

- **Supported string-like** means a single, non-array atom for which the dependency defines a string-like subset.
    - Every other declaration (numeric, boolean, union, container, `unknown`) gets `ApprovalType::BroadString` while keeping `HoleValidator::Declared`. That is the disclosure case.
    - The prompt then shows `{{ count: string (validated as number) }}`.
- **Covers:** `rule_covers(rule: &ApprovalType, declared: &ApprovalType) -> bool`.
    - A `BroadString` rule covers everything. This is the §7 exception, and execution still rejects null, arrays, and objects.
    - A `StringLike(r)` rule covers only a `StringLike(d)` declaration with `is_subtype(&d, &r)`.
    - A `StringLike` rule never covers a `BroadString` declaration.
    - The rule atom is checked to be a supported string-like type before `is_subtype` is called. This handles the departure recorded in the Dependency Gate.

#### C4. Canonical shape and rule text (Phase 2 "Policy Persistence")

```rust
pub struct TemplateShape { pub segments: Vec<ShapeSegment> }        // CommandTemplate with holes reduced to ApprovalType
impl TemplateShape {
    pub fn from_template(&CommandTemplate) -> Self;
    pub fn canonical_text(&self) -> String;                          // `template ` payload
    pub fn parse_rule(text: &str) -> Result<Self, TemplateRuleError>; // file+line added by the loader
}
pub enum ShellRuleEntry { Exact(String), Prefix(String), Template(TemplateShape) }
```

- **Reconciling the spec's text with AC11.**
    - §3 says canonical text "double-quotes literal argument segments". AC11 and every §3/§10 example persist bare words, such as `template just clean --out {{ string }}/build`.
    - Contract: the **writer** emits a literal run bare when it holds only characters that need no escaping. Otherwise it double-quotes the run using the §3 escapes (`\\`, `\"`, `\{`, `\}`, `\n`, `\r`, `\t`). Any run that contains a brace, whitespace, quote, backslash, operator character (`& | ; < > $` and backtick), or control character is quoted, and so is an empty literal argument (`""`).
    - The **reader** accepts both forms. De-duplication compares parsed `TemplateShape`, never text.
    - This satisfies AC11's literal line and §3's escaping rules together.
    - This is an interpretation, not a new ruling. It is recorded here per plan rule 2.
- **Rule placeholder text** is `{{ string }}` or `{{ <serialize_property_atom(atom)> }}`.
    - The loader rejects the following, each with file and line: constraints (with or without `;`), descriptions, unresolved or named types, arrays, unions, unsupported types, unknown options, and program-word holes. A rejected line is skipped; it does not invalidate the other lines.

#### C5. Discovery output, timing, and missing values (Phase 3 "Discovery Provenance")

```rust
pub enum HoleTiming { ApprovalTime, Deferred { roots: Vec<String> } }
pub struct ShellCandidate {
    pub site: CommandSite,                 // source file, ShellCommandOrigin, owner (document source key), stable site id
    pub template: Result<CommandTemplate, TemplateGrammarError>,
    pub resolved: Option<ResolvedCommand>, // today's normalized/executable/args/chain_executables; None when any hole is Deferred
    pub requirement: ModeRequirement,      // AnyMode | TemplateRequired (set by host position or Deferred holes)
}
```

- **Deferral is declared by the host** through a new `ComposeOptions::with_deferred_shell_roots(DeferredShellRoots)`. Claudine passes `outputs`, `loop`, and `group` for sequence approval.
    - This is separate from `BindingView`, and it does not make a root lexically available.
    - Darkmatter tracks deferral transitively through derived frontmatter and task parameters by dependency analysis, without evaluating anything.
    - A compound expression that touches a forbidden root (`err`, `timing`, `current.*`) keeps today's refusal.
- Discovery keeps **one candidate per site**, no longer de-duplicated by `normalized`. Approval prompting may still group identical keys so the user is asked once.
- **Missing values** raise one typed `ShellExpansionError::MissingValue { ctx, command, expression, origin }`. The command is the authored text; the value never appears.
    - It is raised at preflight for every approval-time hole that evaluates to null.
    - It is raised at the step barrier or at command entry for deferred holes.
    - An empty string, `0`, `false`, or a fallback that yields a present value are not missing.

#### C6. Approval transport and keys (Phase 3 "Approval Transport")

```rust
pub enum ApprovalKey {
    Exact { policy_root: PathBuf, normalized: String },
    Template { policy_root: PathBuf, shape: String /* canonical_text incl. ApprovalTypes */ },
}
pub struct ShellPreApprovals { exact: HashSet<String>, templates: HashSet<String /* canonical shape */> }
pub enum ShellApprovalDecision { AllowExactPersist, AllowCommandPersist, AllowTemplatePersist, AllowOnce, Deny, BlacklistPersist }
pub enum ApprovedMode { ResolvedBytes, Template }  // what AllowOnce means is fixed by the request's offered mode
```

- `ComposeOptions.pre_approved_commands` is **replaced** by `Option<ShellPreApprovals>`. Claudine's `PreFlightResult`, `PrepareOptions`, `CallerInputLayers`, `SharedApprovalCache`, `ResolvedShellValue`, and the sequence task records carry the same type. The old `HashSet<String>` field is removed, not kept alongside.
- **Membership at execution**
    1. If the site is `AnyMode` and its freshly resolved bytes are in `exact`, run the bytes.
    2. Otherwise, re-parse the **authored** text, project it against the owner schema, and require its canonical shape to be in `templates`. Then run in template mode.
    3. Otherwise, fail with `NotPreApproved`.
- A `TemplateRequired` site never consults `exact`.
- Freshly resolved bytes are never inserted into either set.
- Prefix and once decisions for a template candidate record its shape in `templates`.
- The `allow_once` and `pending_allow_once` sets in `SharedShellExpansionRuntime` become `HashSet<ApprovalKey>`. The existing reservation, condvar, and failure-release semantics stay unchanged.
- A failed policy write releases the reservation and returns `PolicyIo`. It caches no successful decision and is never downgraded to once.
- **The prompt request** (`ShellApprovalRequest`) gains:
    - `offered: OfferedChoices { exact, shape, prefix }`,
    - `template: Option<TemplateShape>` plus a display form (`{{ expr: type }}`, with disclosure), and
    - `resolved: Option<…>`.
  An unavailable value is `None`, never `""`.
- **Eligibility**
    - Exact is offered only when the site is `AnyMode` and resolved.
    - Shape is offered when there is at least one hole and the grammar is valid.
    - Prefix is offered when the program words are fixed. A template-required site must also pass the grammar check.
    - Once, deny, and blacklist are offered after the candidate validates.
    - A template-required site with a grammar error fails with a typed `TemplateGrammar` error before any prompt.

#### C7. Fill, validation, and dispatch (Phase 4)

```rust
pub struct FilledSegment { pub operator: ChainOperator, pub program: String, pub args: Vec<String>, pub redirects: Vec<RedirectOp> }
pub struct FilledChain { pub segments: Vec<FilledSegment> }
pub struct LateValues(/* site id + hole index -> validated, converted text */);
pub fn fill_template(t: &CommandTemplate, eval: &dyn HoleEvaluator, late: Option<&LateValues>) -> Result<FilledChain, ShellExpansionError>;
pub fn resolve_late_values(sites: &[&ShellCandidate], eval: &dyn HoleEvaluator) -> Result<LateValues, ShellExpansionError>;
pub fn check_filled_chain(chain: &FilledChain, user_blacklist: &ShellRuleSet) -> Result<(), ShellExpansionError>; // builtin then user, every segment
pub fn resolve_direct_target(program: &str, path: Option<&OsStr>, cwd: &Path) -> Result<PathBuf, ShellExpansionError>;
```

- **Order of operations**
    1. Evaluate each hole once.
    2. Reject null (`MissingValue`), arrays and objects, and embedded NUL.
    3. Validate the **original** value against `HoleValidator`.
    4. Convert it with `scalar_string`.
    5. Concatenate the parts into one argument.
    6. After every segment is filled, call `check_filled_chain`.
    7. Only then may a result-cache lookup or a spawn happen.
- **Errors** are new typed `ShellExpansionError` variants: `MissingValue`, `TemplateValueRejected { placeholder, declared_type, origin }` (never the value), `TemplateGrammar`, and `TemplateTarget`.
    - None of them is `ExecutionFailed`, so `handle_errors` / `when_error` cannot absorb them.
    - They are added to `transclusion_failure_is_fatal` and to the nested `is_structural`.
    - Claudine wraps them while keeping the source chain.
- **The step barrier** is owned by Claudine.
    - Before a step's first shell effect, Claudine calls `resolve_late_values` over every template site of that step whose holes are `Deferred`. That covers task `shell:`, the stacks, `set $( … )`, and the step document's commands, including commands on conditional branches.
    - Only deferred holes are kept, in `LateValues`, which is passed into composition (`ComposeOptions::with_late_values`) and into the runners.
    - Approval-time holes are still evaluated at command entry against current state. A lifecycle `set` uses its pre-write snapshot.
    - Mutable event-time values are therefore not frozen, and no placeholder is evaluated twice.
- **Result-cache key in template mode** is a length-prefixed encoding of the mode tag, then for each segment the operator, program, argument count, every argument with its length, and the ordered redirects, followed by the existing context (cwd, strip_ansi, timeout, behavior). No hashing is needed, so `biscuit-hash` is not involved. The key is kept separate from `ApprovalKey`.
- **Direct-target rule.** This is an interpretation of §4 "reject a target that needs an interpreter", recorded per plan rule 2.
    - On Windows, only resolved `.exe`/`.com` targets are accepted. `.cmd`, `.bat`, and any other `PATHEXT` hit are refused, because Rust would route them through `cmd.exe`, which re-parses arguments.
    - On Unix, a resolved file must start with a native executable magic number (ELF / Mach-O) or `#!`. A file with neither is refused, because `execvp` would fall back to `/bin/sh`.
    - `#!` scripts are allowed: the kernel passes argv through unchanged, so values are not re-interpreted. This keeps common tools such as `npm`/`pnpm` usable on Unix.
    - Resolution never falls back to a shell. Builtins, which are not on `PATH`, fail as `TemplateTarget`.
- **Runner adapters.** Claudine keeps operational ownership.
    - `TaskShellRunner::run_structured(&FilledChain, timeout, interrupt, live)` reuses `SystemTaskShell`'s process-group and Job Object spawn, its deadline, its interrupt handling, and its volume caps. Only argv construction changes. The `&&`/`||` short-circuit is evaluated in Rust after whole-chain validation.
    - `ShellRunner::run_structured(&FilledChain) -> Result<i32, ShellRunError>` keeps the lifecycle runner's current contract: inherited stdio, same cwd and environment, and no new deadline.
    - `DisabledShellRunner` refuses `run_structured`.
- **Native recording executable:** none exists today.
    - Phase 4 "Shared Validation" adds the recorder body as a module in `tools/test-toolkit`, with one thin `[[bin]]` per consuming package. Darkmatter and Claudine each get their own, such as `claudine/cli/tests/bin/argv_recorder/main.rs`, following the existing `claudine-fake-*` pattern.
    - Tests locate the binary at runtime through `CARGO_BIN_EXE_*` or the toolkit's lookup. The recorder writes its argv as length-prefixed records to a path taken from an environment variable.

#### C8. Sequence position (Phase 4 "Sequence Position")

- `TaskPosition::{First, Later}` is computed statically in `Loader::walk_plan` and `load_task` and stored on every prepared site.
    - Step 0's task is `First`.
    - Each leg of a group in step 0 is `First`.
    - A nested sequence would be `First` only through its own first child. This is written recursively, though there is no surface to exercise it today.
    - Everything else is `Later`.
- A `Later` site, any site reading `group.*`, and any container terminal-event site are `TemplateRequired`.
- Skips, retries, and resume never change position.
- The `SequenceShellLateBinding` refusal is removed only for `outputs`, `loop`, and `group`. It stays for `err`, `timing`, and `current.*`.

#### C9. Acceptance-to-task map

| AC | Primary task (phase) | Supporting |
| --- | --- | --- |
| 1 Missing, single doc | Discovery Provenance (P3) | Shared Validation (P4), Execution Corpus (P5) |
| 2 Missing, sequence | Sequence Position (P4) | Orchestration Corpus (P5) |
| 3 Store round trip | Policy Persistence (P2) | — |
| 4 Argument-list matching | Policy Matching (P2) | Template Grammar (P2) |
| 5 Covers table | Type Projection (P2) | — |
| 6 One argument per word | Shared Validation (P4) | Sequence / Lifecycle Dispatch (P4), Package Gates on 4 OSes (P5) |
| 7 Run-time check | Shared Validation (P4) | Execution Corpus (P5) |
| 8 Prompt | Approval Prompts (P4) | Approval Transport (P3); fixes the `md` persist defect |
| 9 Late approval end to end | Sequence Dispatch (P4) | Orchestration Corpus (P5) |
| 10 Loop re-audit | Lifecycle Dispatch (P4) | Approval Transport (P3) |
| 11 First vs later tasks | Sequence Position (P4) | Approval Prompts (P4) |
| 12 Unchanged paths | Package Gates (P5) | every task |
| 13 Runtime policy | Policy Matching (P2), Shared Validation (P4) | Execution Corpus (P5) |
| 14 Grammar and boundaries | Template Grammar (P2), Shared Validation (P4) | Package Gates on 4 OSes (P5) |
| 15 Approval vs results | Approval Transport (P3) | Execution Corpus (P5) |
| 16 State and discovery | Sequence Position (P4), Discovery Provenance (P3) | Orchestration Corpus (P5); nested-sequence and container-`finalize` cases deferred (see surfaces) |
| 17 Schema provenance | Type Projection (P2) | Execution Corpus (P5) |

#### C10. File ownership for the parallel waves

| Wave task | Owns |
| --- | --- |
| Template Grammar | `template/{mod,grammar,canonical}.rs`, new variants in `shell_expansion/types.rs` |
| Type Projection | `template/typing.rs` |
| Policy Persistence | `shell_expansion/store.rs` and its tests |
| Policy Matching | `template/matching.rs`, `shell_expansion/policy.rs` |
| Discovery Provenance | `compose/preflight/{collect,mod}.rs`, frontmatter / body / shell-block discovery masks |
| Approval Transport | `preflight/{approval,lifecycle}.rs`, `context/options.rs`, runtime `allow_once` in `types.rs` (after Grammar lands), Claudine `composition/{preflight,prepare,types}.rs`, `harness/shell.rs` decision mapping, CLI handoff and re-audit paths |
| Shared Validation | `template/{fill,target}.rs`, `shell_expansion/{mod,executor}.rs` wiring, `pipeline/phases.rs` + `nested.rs` fatal classification, recorder bins |
| Sequence Position | `composition/sequence/preflight/{mod,shape,reapproval}.rs`, prepared task records, `lifecycle/bindings.rs` |
| Sequence Dispatch | `composition/sequence/task/{shell,mod}.rs`, `cli/.../wrap/sequence/{task_run,jit}.rs` |
| Lifecycle Dispatch | `composition/lifecycle/{executor,preflight}.rs`, `cli/.../compose/prep.rs`, `harness_orch/*` re-audit |
| Approval Prompts | `darkmatter/cli/src/approval.rs`, Claudine handler install (`harness_orch/shell_options.rs`) and decision persistence |

**Checkpoint:**

- Dependency readiness is explicit: `is_subtype` was verified, along with its departure on identical pairs.
- Both execution modes (C6, C7) and both validation barriers (C7) have contracts.
- Every acceptance criterion maps to a task (C9).
- No spike was scheduled.

### Phase 1 verification and close

- Phase 1 is contract and inventory work only. No source, test, documentation, or skill file was changed. The only edits are this log and the plan's checkboxes and frontmatter.
- **Requirement-to-test mapping:** this phase changed no behavior, so it adds no tests. C9 maps every acceptance criterion to the task that will test it.
- **Gates run on macOS:**
    - `just lint` (claudine): clean.
    - `just test` (claudine): 9152 run, 9152 passed, 9 skipped, 0 failed.
    - Dependency check: `cargo nextest run -p darkmatter -E 'test(/subtype/)'` passed 7 of 7.
- **No cross-OS run:** no OS-sensitive code changed, so `just cross-check` was not needed.
- **Interpretations recorded (plan rule 2).** These are not new rulings.
    - C4: canonical rule text writes literal runs bare when safe and quotes them otherwise. This matches AC11's literal line.
    - C7: the direct-target rule allows Unix `#!` scripts and native binaries. It refuses files without a format magic number and Windows `.cmd`/`.bat`.
    - Bounded deviation: lifecycle structured dispatch keeps the lifecycle runner's current contract, which has no deadline and no process tree. It does not invent one.
- **Pre-existing defects found and assigned:**
    - The `md` batch preflight path does not persist persist-decisions. Approval Prompts (P4) fixes it.
    - Sequence task-stack `set $( … )` is never resolved at preflight. Sequence Position (P4) fixes it.
    - The policy loader's warnings carry no line number. Policy Persistence (P2) fixes it.
- Status: Phase 1 complete.

## Phase 2

- 2026-10-14: started Phase 2 (template grammar, type projection, policy persistence, policy matching). All source changes are in Darkmatter; Claudine compiles unchanged against them.

### Template Grammar (Wave 3)

- New module `darkmatter/lib/src/markdown/compose/shell_expansion/template/` (`mod.rs`, `grammar.rs`, `canonical.rs`, `typing.rs`, `matching.rs`, `tests.rs`), exported as `shell_expansion::template`.
- `parse_authored(text)` finds every `{{ … }}` span with `ExpressionFinder::scan_plain`, which is the same scanner interpolation uses, before it reads quotes, whitespace, or operators. A span's own `||`, quotes, and spaces never split or chain the command. A hole stays inside its containing argument, quoted or not. Quoting rules and the four curated redirects mirror `tokenize`.
- Arguments are normalized: adjacent literal runs merge and empty runs drop, so spelling variants have equal parts. `""` stays one empty literal argument; the resolved-byte tokenizer drops it, but template mode keeps it, as spec §4 requires.
- Redirects are kept in authored order (`RedirectOp`). `redirection_config(ops)` lowers them through `apply_redirection`, which was extracted from `tokenize::parse_pipeline`. The two modes therefore share one implementation of the dup semantics, and a test compares the two lowerings on five orderings.
- `TemplateGrammarError` variants: the contract's set plus `InterpolationLiteral` (`{{{ … }}}` and literal tokens are refused in template mode). An unquoted lone `&` is `Background`; the resolved-byte tokenizer still treats it as a literal character.
- **Departures from C1/C2, recorded per plan rule 2:**
    - `Placeholder` has `expression`, `span`, and `typing` but not yet `timing`. `HoleTiming` belongs to Discovery Provenance (Phase 3), which adds the field when it has a producer.
    - `ShellApprovalDecision::AllowTemplatePersist` and the `ShellExpansionError` variants `MissingValue`, `TemplateValueRejected`, `TemplateGrammar`, and `TemplateTarget` are **not** added yet. Each would be an unused variant that every exhaustive match (`prepare_directive`, `preflight/lifecycle.rs`, Claudine `harness/shell.rs`) would have to handle with invented behavior. Approval Transport (P3) and Shared Validation (P4) add them alongside their first producer. The phases are sequential, so the single-owner rule is not at risk.
    - `FilledChain`/`FilledSegment` (C7) are defined now because blacklist matching against filled argv is a Phase 2 task. `FilledSegment` carries the lowered `RedirectionConfig`, not `Vec<RedirectOp>`, so that `FilledChain::from_pipeline` can represent a resolved-byte pipeline (which has no authored order) in the same form.
    - `ApprovalType::StringLike` wraps a `StringLikeType`, not a bare `PropertyAtom`. `PropertyAtom` is only `PartialEq` (it holds `f64` constraints), but `ShellRuleEntry` needs `Eq + Hash`. `StringLikeType` stores the identity atom (boxed) and compares and hashes by its canonical text.

### Type Projection (Wave 4)

- `project_hole(expression, schema: Option<&SimplifiedSchema>, host_globals)` → `HoleType { approval, validator, provenance }`. `CommandTemplate::project_types` applies it to every hole. Only `Expr::Variable` naming a bare top-level property (`dir`, `fm.dir`) reads a declaration. Every other case uses the broad-string fallback with a `FallbackScalar` validator: dotted paths, calls, operators, Darkmatter reserved roots (`fm`, `ctx`, `env`, `current`), host globals (`loop`, `outputs`, …), undeclared names, and no projection. The provenance is recorded in each case.
- A root `SimplifiedSchema::Union` gives a declaration only when every arm that declares the property declares it identically. Otherwise the hole falls back as `NoProjection`. (Spec §2 names only "root union of mixed shapes"; this is the conservative reading for an all-Simplified union whose arms disagree.)
- The approval type is `StringLike` for a single, non-array, always-text atom other than `string`. Its identity keeps only canonical `alphanumeric` options, sorted and deduplicated `enum` members, and a `literal` value. A `string` declaration, numeric, boolean, union, container, and unresolved declarations get `BroadString` while keeping `HoleValidator::Declared` (boxed). `HoleType::display_type()` gives the prompt form, e.g. `string (validated as number(min(1)))`.
- A new `schemas::is_string_like(atom)` is exported from `simplified/subtype.rs`. It is `!is_array && is_always_text(atom)` and reuses the dependency's own list of always-text types instead of copying it. `rule_covers` gates `is_subtype` behind it, which handles the Phase 1 departure (`is_subtype` accepts any identical pair).
- **Open item 3 from Phase 1, resolved:** `schemas::completion::alternative_accepts` lowers the atom on its own with `required` added and validates it through the cached structural JSON-Schema validator. It **enforces** constraints (`min`, `pattern`, …) and **never coerces**: `12345` fails `alphanumeric`, and `"2"` fails `number`. Only `file` values are judged by syntax alone. `HoleValidator::accepts` therefore uses it over the flattened, non-array alternatives of the declaration, and no one-property `EffectiveSchema` document is needed. Tests: `values_are_validated_as_declared_without_conversion` and `numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator`.

### Policy Persistence (Wave 4)

- `ShellRuleEntry::Template(TemplateShape)` was added. `ShellRuleEntry` and `ChainOperator` now derive `Hash`.
- Canonical rule text (C4): `TemplateShape::canonical_text()` writes a literal run bare when every character is safe, and otherwise double-quotes it with the §3 escapes. AC11's line `template just clean --out {{ string }}/build` is produced exactly; a doc test checks it. `TemplateShape::parse_rule` accepts bare and quoted runs and decodes escapes before it recognizes holes. It finds the closing `}}` outside quoted strings, so a quoted `enum` member may contain braces.
- The rule reader rejects:
    - an unescaped brace inside quotes (`UnescapedBrace`), so a hand-written `"{{ string }}"` is reported rather than silently read as literal text;
    - a program-word hole;
    - a constraint, with or without `;`;
    - a description;
    - a named type (including `{{ 123 }}`, which the type grammar reads as a name);
    - array, union, and non-string-like types;
    - unknown options;
    - invalid escapes;
    - control characters;
    - unterminated quotes or holes.
- `store::load_ruleset_with_warnings(path)` returns `(ShellRuleSet, Vec<PolicyLineWarning { path, line, text, reason }>)`. `load_ruleset` keeps its signature and emits each warning through `tracing::warn!`, now with `file:line`. This resolves open item 4 from Phase 1.
- Deduplication now preserves order through a `HashSet<ShellRuleEntry>` instead of the old sort. The old sort could not order template entries, and nothing depended on the sorted order; the whole Darkmatter and Claudine suites pass.
- `append_whitelist_template` / `append_blacklist_template` append one line deduplicated by parsed shape (they read the file first). They refuse with `PolicyIo` (`InvalidData`) if the canonical text would not read back as the same shape. A failed write is always a typed `PolicyIo`. Only approval types are ever written, never a placeholder value.

### Policy Matching (Wave 5)

- Whitelist: `TemplateShape::covers(candidate)` requires the same segment count, operators, ordered redirects, program, argument count, byte-equal literals, and hole positions, plus `rule_covers` for each hole. `ShellRuleSet::template_covers` exposes it.
- Blacklist (`blocks_shape`, `blocks_filled`, `ShellRuleSet::template_blocks_shape`, `ShellRuleSet::blocks_filled`). **Interpretation recorded per plan rule 2**, following spec §8's "a deny list must err toward refusing":
    - holes match any hole regardless of type;
    - redirects are ignored;
    - a single-segment rule matches any segment of a chain (a multi-segment rule must match the whole chain, operators included);
    - against a filled command, each hole matches any run of its containing argument (`*` semantics, linear greedy match).
- `policy::check_template_blacklist(shape, user_blacklist)` is the preflight check. It runs the built-in blacklist on fixed program words and known literal arguments, and **keeps hole positions** (`KnownArg::{Literal, Partial}`): `git {{ x }} reset` is not read as `git reset`, while `git reset {{ x }}` and `psql -c DROP{{ x }}` are refused. It then applies user `prefix` rules, `exact` rules for hole-free segments, and `template` rules by shape. `check_builtin_blacklist` delegates to the same table walk, so its behavior is unchanged.
- `policy::check_filled_chain(chain, user_blacklist)` is the runtime check: built-in first, then user `exact`/`prefix`/`template` over **every** segment before any starts. Shared Validation (P4) calls it.
- **Behavior change now live:** `check_user_blacklist` also applies template blacklist rules to the command it is given, so a hand-written `template deploy {{ string }}` in `.darkmatter-shell-blacklist` already blocks a resolved-byte `deploy prod` in both Darkmatter's `prepare_directive` and Claudine's `validate_and_approve_command_parts`. Multi-segment template blacklist rules reach resolved-byte chains only when P4 wires `check_filled_chain` into execution.

### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| Grammar: holes before syntax, containing argument, empty args, chains, redirect order and lowering, every refusal | `template::tests::{a_placeholder_is_recognized_before_its_spaces_quotes_and_operators, literals_and_holes_stay_in_their_containing_argument, an_empty_quoted_argument_is_one_present_empty_literal, quoting_and_whitespace_spellings_have_one_shape, chains_keep_their_operators_and_redirections_their_order, redirections_lower_exactly_as_the_resolved_byte_tokenizer_does, unsupported_template_grammar_is_a_typed_error, values_are_never_parsed_only_authored_text_is}` |
| Canonical text: AC11 line, round trip with quotes, backslashes, braces, `{{` literal, newline/CR/tab, Unicode, `%`, `!`, empty args, adjacent holes, operators-as-literals; reader forms; rejections | `template::tests::{the_canonical_text_of_the_build_clean_example_is_bare, canonical_text_round_trips_every_literal_and_boundary, the_reader_accepts_bare_and_quoted_literal_runs_alike, malformed_rule_text_is_rejected, rule_types_keep_only_their_canonical_identity}` |
| AC3 store round trip; malformed lines with file and line; I/O failures | L1 `shell_template_policy::{exact_prefix_and_template_rules_survive_write_read_write_read, a_constraint_or_program_word_hole_line_is_skipped_with_its_file_and_line, unreadable_and_non_utf8_policy_files_are_policy_io_failures}` |
| Policy robustness matrix (whitelist and blacklist, 17 cells plus a blank document, control row, sentinels) | L1 `shell_template_policy::{the_whitelist_robustness_matrix, the_blacklist_robustness_matrix}`. The blacklist walk also asserts that preflight (`check_template_blacklist`) and runtime (`check_filled_chain`) agree on every cell |
| AC4 argument-list matching | L1 `whitelist_templates_match_on_the_argument_list` (whitespace/quoting variants match; a changed literal, an extra argument, a moved hole, a changed operator, a changed redirect order, dropped redirects, and an added hole do not) |
| AC5 covers table | L1 `the_covers_table_holds_for_a_declared_placeholder` (every §7 row, plus the broad `string` rule on every row), `a_broad_string_declaration_is_never_covered_by_a_narrower_rule`, `unsupported_rule_types_never_load_even_when_identical_to_the_declaration` |
| AC17 provenance (primitive portion) | L1 `a_bare_property_reads_its_resolved_named_type`, `only_a_bare_top_level_property_reads_a_declaration`, `a_local_child_schema_types_its_own_placeholders`, `numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator`, `values_are_validated_as_declared_without_conversion`; malformed constraints with and without `;` in `malformed_rule_text_is_rejected` and the store test |
| AC13 primitives | L1 `a_string_template_cannot_authorize_a_dangerous_filled_value`, `preflight_keeps_hole_positions_when_checking_known_words`, `user_blacklists_block_filled_commands_with_and_without_authored_holes`, `a_blocked_later_segment_blocks_the_whole_chain`, `a_blacklist_template_matches_on_shape_whatever_the_hole_type`; `policy::tests::a_hole_keeps_its_position_and_never_matches_a_literal_word` |

- **Matrix scope.** The plan asks that the matrix be walked through "public composition approval results". Template approvals reach composition only in P3/P4. This phase therefore observes the public library decisions instead: `load_ruleset_with_warnings`, `ShellRuleSet::template_covers`, `check_template_blacklist`, and `check_filled_chain`. The composition-level walk is listed under Phase 5's Execution Corpus ("the policy matrix").
- **Code-smell audit of touched reader paths:**
    - no `#[serde(default)]`;
    - no `Option` collapsing absent and null;
    - no `filter_map(.. as_str())` on loaded data (the only `filter_map`s select rule variants);
    - no `unwrap_or_default()` or `.ok()` on a parse.
  A missing file still defines an empty rule set, as the format specifies.
- **Tiers:** the unit tests are in the `darkmatter` lib target. The L1 tests are declared in `tests/l1/main.rs` (`mod shell_template_policy;`), and no tier marker is used. The new L1 tests take about 0.05 s in total.

### Gates (macOS)

- `just lint` (darkmatter): clean (library, CLI, DMLS, zed-dmls wasm). The first run hit `clippy::large_enum_variant` on `ApprovalType`/`ArgPart`; it was fixed by boxing the identity atom and the declared validator.
- `just test` (darkmatter): 9278 run, 9278 passed, 10 skipped.
- `just lint` (claudine): clean.
- `just test` (claudine): 8677 run, 8677 passed, 9 skipped. This is the final summary line; Phase 1 reported 9152 from its own invocation. No Claudine test or target was changed in this phase.
- Doc tests for the new public items (`parse_authored`, `TemplateShape::canonical_text`, `HoleType::display_type`, `is_string_like`): 4 passed (`cargo test --doc -p darkmatter -- template is_string_like`).
- **No cross-OS run.** This phase adds pure string parsing, typing, and matching, plus the existing line-based file read. There is no process spawn, no path comparison, and no `cfg(windows)` code. `content.lines()` already strips `\r\n`. Native argv evidence on all four OSes is Phase 5's Package Gates.
- No pre-existing failures were seen. Nothing was skipped.

### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md` gained a "File format" subsection: the three rule kinds, hole types, quoting and escapes, which template lines are skipped and why, and the blacklist template behavior that is live now. Whitelist authorization, the shape choice, and the run-time check are marked **planned**.
- `.claude/skills/darkmatter/compose.md` gained a "Typed shell templates" subsection for agents: module placement, authored parsing, the approval-type/validator split, the `rule_covers` gate, blacklist semantics, and the warnings loader.
- No Claudine skill change: no Claudine code or behavior changed in this phase.

- Status: Phase 2 complete.

## Phase 3

- 2026-10-14: started Phase 3 (Discovery Provenance, Approval Transport). One agent implemented both tasks, because both touch the shared types in `template/` and `shell_expansion/types.rs` (contract C1 single-owner rule).

### Discovery Provenance (Wave 6)

- **New shared types**, in `template/candidate.rs` and exported from `template`:
    - `ShellCandidate { site: CommandSite, template, resolved: Option<Vec<String>>, requirement: ModeRequirement }`
    - `CommandSite { source_file, origin, owner }`
    - `ModeRequirement::{AnyMode, TemplateRequired}`
    - `DeferredShellRoots`
- `Placeholder.timing: HoleTiming::{ApprovalTime, Deferred { roots }}` was added, completing the C2 departure recorded in Phase 2.
- `ComposePreflightReport.candidates` holds **one candidate per command site, never merged by bytes**.
    - `collect_effects` now returns a `CollectedEffects` struct.
    - Entries stay deduplicated by normalized bytes, as before.
- **Authored sites.** Each surface is read from the *authored* text, alongside the existing resolved discovery:
    - body `::shell`: `parser::authored_directive_commands` + `authored_template`, which drops `--when-error`/`--no-cache`/`::timeout:` exactly as `extract_options_from_tokens` does;
    - `::shell-block`: `group_logical_commands` over the authored block bodies;
    - frontmatter `$( … )`: `frontmatter_shell_expansion::authored_shell_template`. A conditional `$( a ? b : c )` is never a template, and gets the new `TemplateGrammarError::Conditional`.
- **Linking a site to its resolved bytes.** Each site is matched to the bytes discovery resolved for it:
    - body and shell-block sites by document order (interpolation inserts data, never a directive);
    - frontmatter sites by key.
    - If the counts differ, the site stays unlinked (`resolved: None`) rather than guessed. Its bytes are still approved as entries, so this fails closed.
    - **Decision (plan rule 6 asks for a recorded choice):** linking by order instead of re-filling the template. A resolved-byte command tokenizes interpolated text, so `a b` becomes two arguments there, while a template fill keeps one argument. The two forms cannot be compared byte for byte.
- **Typing.** `CandidateScope::build` types holes with `project_types(owner_schema, host_globals)`.
    - The owner schema is assembled lazily, only for a site that has a placeholder: `prepare_schemas(...).effective_for(...).simplified` for that document. A transcluded child uses its own schema.
    - Host globals come from `HostGlobals::roots()` (new, crate-private).
- **Deferral.**
    - A hole is `Deferred` when its expression reads a root from `ComposeOptions::with_deferred_shell_roots` (new). It is also `Deferred` when it reads a frontmatter key whose authored value reads such a root, directly or transitively (`derived_deferred_keys`).
    - Derived keys pass to children through `InheritedDiscovery.deferred`.
    - A deferred hole is never evaluated.
    - A site with a deferred hole is `TemplateRequired`. It contributes no resolved-byte entry, and fails with the new `ShellExpansionError::TemplateGrammar` when its authored text is not a valid template.
    - Declaring a root deferred is independent of `BindingView`/host-global availability (plan rule 4).
- **Missing values (AC1).**
    - Every approval-time hole is evaluated against the document's discovery state: `ResolvingLookup` over the prepared document state, with the discovery request's resolution context.
    - A `null` result raises the new `ShellExpansionError::MissingValue { ctx, command, expression, origin }`. The command is the authored text; the value is never included.
    - An evaluation error is left to the existing strict paths, so parse and unavailable-binding errors are unchanged.
    - When the grammar fails, holes are found with `ExpressionFinder::scan_plain`. A missing value therefore also refuses a command that runs only as resolved bytes.
    - Claudine's lifecycle `shell` and `set $( … )` resolution (`preflight::resolve_shell_command_expr`) applies the same rule. It composes the command and each placeholder as whole values in **one** `SubtreeCompose` call, because the `composition_seams` drift guard allows one call per site. It raises `LifecycleShellResolution`, whose source is Darkmatter's typed `MissingValue`.
- **Unsuppressible.** `MarkdownError::shell_input_violation()` (new) finds `MissingValue`/`TemplateGrammar` anywhere in the cause chain. It was added to `transclusion_failure_is_fatal` (`pipeline/phases.rs`) and to nested `is_structural` (`nested.rs`). Neither error is `ExecutionFailed`, so `when_error` cannot absorb them.
- **Not done here, by design (recorded for Phase 4).**
    - Claudine sequence preflight does not yet declare `outputs`/`loop`/`group` deferred, and it still raises `SequenceShellLateBinding` (C8, Sequence Position).
    - Mapping *task parameters* derived from late roots onto deferral is part of that same wiring. Darkmatter already follows derived frontmatter keys and inherited keys. A task parameter reaches the task document as data, so the host passes its name in `DeferredShellRoots` when that parameter reads a late root.
    - A frontmatter `$( … )` whose hole is deferred still meets the existing `DynamicCommandShape` refusal (`detect_dynamic_frontmatter_command_shape`) when the deferred root is not bound in discovery. This fails closed. Frontmatter template-mode execution is Phase 4 (Shared Validation).
    - Discovery's body interpolation still evaluates every body span in lenient mode. With Claudine's preflight `BindingView`, a late root is `Unavailable`, so nothing is observed. "Not evaluated" is therefore guaranteed for the candidate check, not for the pre-existing body interpolation pass.

### Approval Transport (Wave 6)

- **New shared types**, in `template/approval.rs`:
    - `ShellPreApprovals { exact: HashSet<String>, templates: HashSet<TemplateShape> }`, with `From<HashSet<String>>` (exact only) and `FromIterator<String>`;
    - `ApprovalKey::{Exact { policy_root, normalized }, Template { policy_root, shape }}`;
    - `OfferedChoices { exact, shape, prefix }`;
    - `TemplateApprovalView { shape, display }`;
    - `CommandTemplate::display_text()`, which gives `{{ expr: type }}` with the validator disclosure.
- **Departure from C6, recorded per plan rule 2:** `ApprovalKey::Template` holds the `TemplateShape` itself instead of its canonical text, so keys compare structurally. `policy_root` is an `Option`, because Claudine runs without a policy root in some paths.
- **Darkmatter**
    - `ComposeOptions.pre_approved_commands: Option<ShellPreApprovals>`.
        - `with_pre_approved_commands(impl Into<ShellPreApprovals>)`.
        - Its identity hash encodes templates only when present, so resolved-byte sets keep their fingerprint.
        - `with_deferred_shell_roots` is also hashed only when non-empty.
    - `ShellApprovalDecision::AllowTemplatePersist` was added.
    - `ShellApprovalRequest` gains `offered` and `template`. An unknown value is never encoded as bytes: a template-required request has an empty `normalized_exact`.
    - `SharedShellExpansionRuntime.allow_once`/`pending_allow_once` are now `HashSet<ApprovalKey>`; `ReservationGuard` holds keys. The reservation, condvar, and drop-release semantics are unchanged.
    - **Membership.**
        - `validate_pre_approved` checks it through `first_unapproved`. A template-required site must have its shape approved. A resolved-byte entry needs its bytes approved, unless every site resolving to those bytes is approved by its shape.
        - `prepare_directive` admits only exact members. **Template-mode execution is Phase 4**, so a site approved only by its shape is refused with `NotPreApproved` at execution. It is never run as interpolated bytes, and nothing inserts the resolved bytes.
        - A resolved-byte prompt that returns `AllowTemplatePersist` (not offered) is treated as `Deny`.
    - **`compose_preflight_approvals` (the `md` batch path) now decides per site.**
        - The order is: the blacklists for every entry; then per candidate an `exact` rule, a `template` rule (the shape is blacklist-checked by `check_template_blacklist`), a `prefix` rule, and finally the handler with the site's `offered` choices and template view.
        - Unlinked entries keep the old per-entry flow.
        - Every choice now **persists its row**: exact, prefix, template, and blacklist exact or template. This also fixes the pre-existing defect recorded in Phase 1, where this path persisted nothing. It was assigned to Approval Prompts (Phase 4), but `AllowTemplatePersist` could not be handled without it.
        - A failed write is `PolicyIo` and approves nothing.
        - A choice the request did not offer is `Denied`.
- **Claudine**
    - `PreFlightResult.approved_commands`, `PrepareOptions.pre_approved_commands`, `CallerInputLayers.pre_approved_commands`, and `add_approved_commands` now carry `ShellPreApprovals`.
    - So do every CLI handoff and re-audit set: `compose/prep.rs` loop iteration and staged stabilization, `staged_boot.rs`, sequence `mod.rs`/`phase1c.rs`/`jit.rs`/`iterate.rs`, and harness `prompt.rs`.
    - `preflight_bootstrap_shell` and `preflight_document_shell` return it too.
    - Lifecycle `set` values pass their bytes as an exact-only `ShellPreApprovals`.
    - `SharedApprovalCache` and `ShellApprovalOptions.approval_cache` are keyed by `ApprovalKey`.
    - `harness::shell` was restructured around a private `SessionPolicy`.
        - `validate_and_approve_command_parts` keeps its behavior, messages, and order. It now keys the cache by `ApprovalKey::exact(policy_root, …)`, and a failed policy write is the new `HarnessError::ShellPolicyWriteFailed { path, source }` (`io.write_failed`) **with nothing cached**. Before this change the write failure was silently ignored (`let _ =`) and the decision cached.
        - New `validate_and_approve_template_site(candidate, options) -> TemplateShape` applies, in order: the template blacklist; a template rule or prefix rules for every program; the template cache key; then the handler with the site's offered choices. An exact decision never covers it.
        - New `template_rule_covers` and `check_command_blacklists`.
    - `composition::preflight::approve_commands` now takes Darkmatter's `candidates`.
        - Template-required sites are approved by shape.
        - A resolved-byte site whose bytes no exact rule covers, and whose shape a template rule covers, is approved by shape.
        - Its bytes are still checked against both blacklists, but they are not prompted for or added to the exact set unless another site needs them as bytes.
        - Errors map through one `audit_failure` helper, keeping the existing `ShellCommandDenied`/`ShellApprovalUnavailable` families.
- **Result cache:** unchanged. The template-mode result key (C7) arrives with template execution in Phase 4.

### Comment drift fixed

- Darkmatter `preflight/mod.rs` and `preflight/approval.rs` module docs, `preflight/lifecycle.rs` steps, the `ComposeOptions.pre_approved_commands` field doc, and the `prepare_directive` fast-path comment.
- Claudine `resolve_shell_approvals` doc, `execute_set_shell_values` doc, `PreFlightResult`, and `CallerInputLayers`.
- The sequence comments listed in Phase 1 (`sequence/preflight/*`, `task/*`) still describe today's behavior, which is unchanged. They change with Sequence Position and Sequence Dispatch in Phase 4.

### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| AC1: missing value on every Darkmatter surface (null, absent, `fm.` namespace, shell-block, frontmatter, invalid grammar), typed error naming command, expression, and origin | L1 `darkmatter::l1 shell_template_discovery::a_null_or_absent_value_fails_preflight_on_every_shell_surface` |
| AC1: empty string, 0, false, fallback, and absent-with-fallback are present | `shell_template_discovery::empty_zero_false_and_fallback_values_are_present` |
| AC1: unsuppressible by `--when-error`; non-shell null still renders empty | `shell_template_discovery::a_missing_value_is_unsuppressible_and_a_non_shell_null_still_renders_empty` |
| AC1: nothing runs (frontmatter `git init` marker) | `shell_template_discovery::compose_fails_with_the_missing_value_before_any_command_runs` |
| AC1 end to end, `md compose` (body, absent, frontmatter, shell-block; marker proves nothing ran; empty string runs) | `darkmatter-cli::l1 compose_shell::test_compose_refuses_a_missing_shell_value_before_running_anything`, `test_compose_runs_a_shell_command_whose_value_is_an_empty_string` |
| AC1 end to end, `claudine compose` (body, frontmatter, lifecycle `shell`, lifecycle `set $( … )`; marker; empty string runs) | `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface`, `an_empty_string_value_is_present_and_runs` |
| AC16/AC17: one candidate per site, owner-schema typing for root vs transcluded child, owner key | `shell_template_discovery::each_site_keeps_its_authored_template_and_its_owners_types` |
| Directive options are not template arguments; a hole stays in its containing argument | `shell_template_discovery::directive_options_are_not_template_arguments_and_holes_stay_whole` |
| No shape for a hole-free site | `shell_template_discovery::a_site_without_placeholders_offers_no_shape` |
| Deferral: direct, derived through frontmatter, mixed with approval-time holes; no entry for a late site; offered choices | `shell_template_discovery::a_late_root_is_not_evaluated_and_requires_a_template` |
| A late read does not mask a missing approval-time value | `shell_template_discovery::a_late_read_with_a_missing_approval_time_value_still_fails` |
| A late site with invalid grammar fails as `TemplateGrammar` (unsuppressible) | `shell_template_discovery::a_late_command_that_is_not_a_valid_template_fails_preflight` |
| Without declared deferral, the same read is approval-time | `shell_template_discovery::without_declared_late_roots_the_same_read_is_an_approval_time_value` |
| AC15: exact and template approvals never authorize each other, including at the execution gate | `shell_template_discovery::exact_and_template_approvals_never_authorize_each_other` |
| Membership is proved and never manufactured; a changed declared type is not a member | `shell_template_discovery::the_gate_accepts_a_shape_approval_and_never_adds_its_bytes` |
| AC12: exact pre-approvals still run resolved bytes | `shell_template_discovery::exact_pre_approvals_still_run_resolved_bytes` (plus the whole unchanged corpus) |
| AC8 foundation, `md` path: offered choices, template display, each choice persists its row | `shell_template_discovery::a_resolved_site_prompt_offers_exact_and_shape_and_each_choice_persists_its_row`, `a_template_required_site_is_offered_no_exact_choice` |
| Precedence exact, then template, then prefix | `shell_template_discovery::rules_are_preferred_exact_then_template_then_prefix` (md), `claudine composition::preflight::tests::a_template_rule_approves_a_resolved_site_by_shape_and_an_exact_rule_wins` |
| A persisted template covers a late site; an exact rule does not | `shell_template_discovery::a_persisted_template_rule_covers_a_late_site_without_prompting` |
| Shape blacklist; blacklisting persists the shape | `shell_template_discovery::a_shape_blacklist_refuses_a_late_site_and_blacklisting_persists_the_shape`, `claudine … a_template_blacklist_refuses_the_site_before_any_prompt` |
| AC15: a failed write is a typed policy failure; nothing approved or cached | `shell_template_discovery::a_failed_template_write_is_a_policy_failure_and_approves_nothing`, `claudine … a_failed_policy_write_is_typed_and_caches_no_decision` (template and exact) |
| AC17: the owner schema flows into the persisted rule, with numeric disclosure | `shell_template_discovery::a_hole_type_follows_the_owner_schema_into_the_persisted_rule` |
| Mode- and policy-root-tagged runtime reservations; failure releases | `darkmatter … template::tests::session_reservations_are_keyed_by_mode_and_policy_root` |
| Claudine: a template site is offered its shape and persists it; the cache key is template | `claudine … a_template_site_is_offered_its_shape_and_persists_the_template_row` |
| Claudine: exact cache entries and rules never answer for a template site; an unoffered exact choice is denied and persists nothing | `claudine … an_exact_decision_never_answers_for_a_template_site` |
| Claudine: policy-root isolation of the session cache | `claudine … a_template_decision_is_scoped_to_its_policy_root` |
| Claudine: a shape-approved site's bytes still meet the blacklist | `claudine … a_shape_approved_site_still_checks_its_bytes_against_the_blacklist` |
| Claudine: a late document site with no approval fails pre-flight naming it | `claudine … a_late_document_site_without_an_approval_fails_pre_flight_naming_it` |

- **Tiers.**
    - Darkmatter L1 tests are declared in `tests/l1/main.rs` (`mod shell_template_discovery;`).
    - Claudine CLI L1 tests are declared in `cli/tests/l1/main.rs` (`mod shell_missing_values;`).
    - Claudine lib tests are in `composition/preflight/tests.rs`. The runtime unit test is in `template/tests.rs`.
    - No tier marker is used.
    - Timing: all new tests are fast. The two CLI process tests take about 1.7 s and 0.7 s; each lib test takes under 0.1 s.
- **Input robustness matrix:** not applicable. No configuration reader changed in this phase; the policy-file reader is the same one Phase 2 matrix-tested.

### Gates (macOS)

- `just test` (darkmatter): 10053 run, 10053 passed, 10 skipped.
- `just test` (claudine): 9162 run, 9162 passed, 9 skipped.
    - The first run failed the `composition_seams::subtree_compose_baseline_holds_the_line` drift guard: the lifecycle missing-value check had added a second `SubtreeCompose` call. It was fixed by folding both reads into the existing call.
- `just lint` (darkmatter): clean. Two fixes were needed: `#[allow(clippy::too_many_arguments)]` on `scan_one_frontmatter`, matching `collect_recursive`, and a needless borrow in a test.
- `just lint` (claudine): clean.
- **No cross-OS run.** This phase adds no `cfg(windows)` code, no new spawn path, and no path comparison except `canonicalize_simplified` for owner keys. The new process tests spawn `git` through `host_path()`, as `lifecycle_set_shell_values.rs` already does on every OS. Native argv evidence on all four OSes remains Phase 5.
- No pre-existing failures were seen. Nothing was skipped.

### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md` gained:
    - a "Missing values" section;
    - template-rule precedence;
    - the persisted `md` choices, and policy-write failure;
    - request `offered`/`template`;
    - per-site `candidates`, late values (`with_deferred_shell_roots`), and the structured membership proof.
    - **planned** markers for template-mode execution and the `md` prompt's shape choice.
- `claudine/docs/topics/pre-flight-checks.md` gained:
    - a "Missing Values" section;
    - template-rule precedence and the template-only approval of late sites;
    - `io.write_failed`;
    - the mode- and policy-root-keyed cache;
    - `ShellPreApprovals` membership.
    - **planned** markers for template execution and sequence late-root declaration.
- Skills: `.claude/skills/darkmatter/compose.md` (discovery provenance and transport) and `.claude/skills/claudine/cli-commands.md` (the shell pre-flight row).

- Status: Phase 3 complete.
