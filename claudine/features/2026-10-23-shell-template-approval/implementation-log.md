---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-23-shell-template-approval/spec.md"
plan: "claudine/features/2026-10-23-shell-template-approval/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
    Phase 1 changed no source. Its output is the "Contract Definition (Wave 2)" section of this log (C1–C10). Read it before Phase 2; it names the shared module (`darkmatter/.../shell_expansion/template/`), the types, the owner of each file, and the acceptance-to-task map.

    Phase 2 notes:
    1. The covers relation is `darkmatter::markdown::schemas::is_subtype(&declared, &rule)`. It returns true for any two identical types, including unions and containers. Check that the rule atom is a supported string-like, non-array atom before calling it (C3). Handle the broad `string` rule separately.
    2. Canonical rule text (C4): write literal runs bare when safe, otherwise double-quote them with the §3 escapes. The reader accepts both forms. AC11 requires the exact line `template just clean --out {{ string }}/build`.
    3. Confirm whether `schemas::completion::alternative_accepts` enforces constraints such as `min` and `pattern` without coercing. If it does not, validate through `EffectiveSchema` on a one-property document. Record the choice.
    4. `store.rs::load_ruleset` warns with `tracing::warn!` and no line number. The spec needs file and line in the warning, and the matrix test must be able to capture it.
    5. Nested-sequence and container-`finalize` acceptance cases are deferred: those surfaces do not exist until 2026-10-24-sequence-containers lands. Use today's spelling `outputs`, not `seq.outputs`.
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
