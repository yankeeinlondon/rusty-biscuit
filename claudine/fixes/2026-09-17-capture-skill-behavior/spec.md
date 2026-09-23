---
created: 2026-09-17
status: proposed
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-17
implemented: false
area: claudine
packages:
    - claudine
    - claudine-cli
    - claudine-gen
---

# Capture Agent Skills Discovery, Scope, and Frontmatter Behavior

## Status

Reviewed specification. Implementation and fleet research remain future work.

## Problem

Claudine's Agent Skills research must explain what happens when a provider can
discover multiple versions of a skill. Listing user and repository directories
does not establish whether skills override one another, coexist, merge, become
namespaced, or produce an error. Discovery order, configuration precedence,
selector visibility, and the skill ultimately activated are different facts.

There are two independent dimensions:

- **Scope:** user, repository/project, nested project/workspace, managed/system,
  plugin, or another provider-defined scope.
- **Discovery root family:** the provider's own directories, Claude-compatible
  directories, shared directories such as `.agents/skills`, legacy directories,
  and explicitly configured directories.

A non-Claude provider may read both `~/.provider/skills` and `~/.claude/skills`
at user scope and their repository counterparts. Knowing that repository skills
override user skills, if verified, still does not answer which root wins within
a scope or whether scope takes priority over root family. These paths are
research candidates, not a claim that every provider supports them.

The current fleet at `claudine/docs/research/skills/_fleet.md` asks generally
about scope and precedence. Its `_schema.yaml` captures paths and a free-text
`discovery.precedence`, but does not require structured collision outcomes or
evidence for each rule. Its examples also favor a repository-over-user model.
The existing `codex.md` illustrates the ambiguity: it lists priorities while
also saying same-name skills coexist, and makes a categorical claim about
Claude-directory discovery that requires fresh, version-specific verification.
This spec does not ratify or reverse those provider claims.

Skill format compatibility also depends on the frontmatter properties each CLI
requires or gives special meaning. A shared directory or accepted `SKILL.md`
does not imply shared semantics. In particular, a skill's directory may supply
its name without a frontmatter `name`, and `model` selects a model from the
consuming CLI's available models, which makes that value inherently sensitive
to provider and configuration. Tool identifiers in `allowed-tools` have a
similar portability problem.

## Outcome and Scope

For every research-enabled provider in `claudine/docs/providers.yaml`, a reader
must be able to determine which roots are searched, how overlapping skills are
identified, and what happens at discovery and activation when they collide.
The report must also identify required and optional recognized frontmatter
properties, their runtime effects, and behavior when their values cannot be
used by the consuming CLI.
Unresolved behavior must be explicitly recorded with the evidence gap.

Extend the existing fleet prompt, schema, per-provider reports, and Agent Skills
summary workflow. Reuse the existing research lifecycle and publishing path.
Do not introduce a separate research runner or change Claudine's runtime linker
or provider precedence rules as part of this fix. Record justified follow-up
work through `requires_claudine_update` and `reason`.

> **Reader's note:** this fix intentionally changes the skills topic from an
> unversioned, primarily descriptive schema into a revisioned relational
> research contract. It follows the established steering-topic split:
> Darkmatter validates document shape, while `claudine-gen` validates stable
> IDs, references, required case coverage, and evidence policy. This change does
> not make the research findings runtime linker policy. Any linker behavior
> implied by refreshed evidence remains separately reviewable follow-up work.

For this specification, an **active research provider** is a roster entry in
`claudine/docs/providers.yaml` whose `skip_research` value is not `true`. A
**concrete support finding** is `supported` or `unsupported`;
`not_applicable` is a resolved applicability finding, while `unknown` records
incomplete knowledge rather than a negative result.

## Requirements

### R1. Inventory scope and root family separately

For each provider, investigate native, Claude-compatible, shared, legacy,
configured, and plugin skill roots where relevant. Cover user and repository
scope independently; support at one scope is not evidence of support at another.
Record additional scopes using the provider's terminology and explain their
mapping to the research vocabulary.

Every provider must explicitly answer whether it natively discovers
`~/.agents/skills` and `<repo>/.agents/skills`, independently of its own prefix
and `.claude/skills`. These two investigations are mandatory, including explicit
unsupported or unknown findings. Record whether support is automatic, opt-in,
or available only through a configured path or symlink; the last two do not
establish native discovery of the shared convention.

Each root record must identify its scope, root family, exact template path,
operating system, execution environment, discovery support, enabling conditions,
and evidence. Preserve separate macOS, Linux, and native Windows path records.
Use `os: linux` with `environment: wsl2_linux` for a Linux CLI in WSL2, and
`os: windows` with `environment: windows_from_wsl2` for a Windows executable
launched from WSL2. Neither is evidence for `environment: native` on the other
OS. A path record may be repeated when the same template has different support
or conditions across environments.

Capture CWD/ancestor traversal, repository boundaries, nested-project discovery,
extra working directories, environment/configuration root overrides, trust
gates, startup versus live reload, and flags that enable or disable discovery.
Use explicit supported, unsupported, unknown, and not-applicable states where
appropriate. Absence from documentation or an empty local directory is not
proof that a root is unsupported.

### R2. Establish skill identity and collision behavior

Research which key identifies a skill: frontmatter name, directory name,
qualified name, canonical path, content, or another provider-defined identity.
Distinguish these cases:

| Case | Question |
| --- | --- |
| Same name, same root family, user versus repo | Does one scope win, or do both survive? |
| Same name, different root families, same scope | Does native, Claude-compatible, shared, or legacy placement matter? |
| Same name, different root families and scopes | Does scope or root family decide, or is there no single winner? |
| Same name, parent versus nested repository directory | How do traversal and current working directory affect the result? |
| Same name, plugin versus filesystem skill | Does namespacing prevent a collision or change selection? |
| Different directories with the same metadata name | Is the collision key the name or the directory? |
| Identical copies at different paths | Are copies retained or deduplicated, and by what identity? |
| Multiple symlinks to one target, including a direct path | Is one physical skill exposed once or multiple times? |

Cover relevant root pairs rather than assuming transitivity. In particular,
compare user-native with repo-Claude and user-Claude with repo-native when both
families and scopes are supported. Unsupported combinations are not applicable;
untested combinations remain unknown.

Record discovery/catalog visibility, explicit activation resolution, and
implicit activation separately. Multiple selector entries do not prove both
bodies are loaded or that implicit selection is deterministic. For each case,
state override, coexistence, merge, namespace separation, rejection, or unknown;
identify the winner or surviving entries when known. For a merge, explain what
is merged. Record ambiguity diagnostics and ways to select a particular copy.
Invocation syntax remains owned by the slash-commands topic; document only the
selection semantics needed here and cross-reference syntax research.

Also investigate whether duplicate entries consume description/context budget,
whether truncation or omission affects availability, and whether disabling one
path disables aliases or all same-name skills. Do not infer these answers from
a startup warning alone.

### R3. Require versioned, attributable evidence

Each substantive root, collision, or frontmatter claim must reference official documentation,
version-pinned source, release notes, or a reproducible observation. Record the
research date, provider version or source revision, execution mode, OS, relevant
configuration, and source URLs or local probe artifacts. Separate documented,
source-derived, observed, and inferred conclusions; explain disagreements.

Use one top-level `evidence` registry with stable IDs. Reuse the established
evidence methods `official_docs`, `source_code`, `local_inspection`,
`disposable_test`, and `inference`; release notes are `official_docs` with the
release URL and version in the record. Each evidence record contains `id`,
`method`, `location`, `version`, `observed_on`, `os`, `environment`,
`execution_mode`, `configuration`, `claim`, and `limitations`. `os` accepts the
three OS values plus `not_applicable` and `unknown`; `environment` accepts the
R1 values plus `not_applicable` and `unknown`. Documentation or source evidence
uses `not_applicable` when it is genuinely platform- or mode-independent rather
than pretending it was observed on every platform.
Research records refer to it through `evidence_ids`; they do not embed a second
free-form evidence format. Concrete `supported` or `unsupported` findings need
at least one non-`inference` evidence record. `inference` may explain a gap but
cannot turn missing evidence into a negative finding.

Use distinct, harmless markers in collision fixtures and isolate the tested
roots so the evidence identifies which artifacts were discovered or read.
Prefer deterministic catalog output or source inspection to model self-report.
Seeing a skill listed is evidence of discovery, not evidence of activation.
Repeat a selection probe when needed to distinguish a fixed rule from a model's
choice, without declaring an uncertain result deterministic.

Native scanning must be distinguished from visibility introduced by symlinks,
Claudine linking, wrappers, plugins, or configuration. A skill originating under
`.claude/skills` can be visible through a native-root symlink without the provider
scanning Claude directories directly.

Research historical changes when relevant. State the old and new behavior and
the first verified version/date when evidence permits. Otherwise say the change
boundary is unknown. Broad configuration-precedence announcements must not be
used as proof of skill-name deduplication or override behavior.

### R4. Make the research contract enforce these questions

Extend `skills/_schema.yaml` using the existing SimplifiedSchema conventions.
The first versioned contract is `schema_revision: 2`; the legacy unversioned
shape is treated as revision 1 and is never current. Declare the new property as
`literal(2; required)` so a document cannot claim an arbitrary newer revision.
Also require `provider` (the roster slug), `created`, `last_updated`, `agent`,
`model`, `contract_validated`, `changes`, `requires_claudine_update`, and
`reason`. Preserve `created` on refresh; `reason` is always non-empty so a
`false` update verdict is an explicit conclusion rather than an unexplained
default. `contract_validated` is a fleet-managed boolean, not a researcher
attestation.

Keep `locations` as the canonical root inventory, with these required fields:

| Field | Contract |
| --- | --- |
| `id` | Stable within the provider report and unique across locations. |
| `os` | `macos`, `linux`, or `windows`. |
| `environment` | `native`, `wsl2_linux`, or `windows_from_wsl2`. |
| `scope` | `user`, `repo`, `nested_project`, `workspace`, `managed_system`, `plugin`, or `other`. |
| `root_family` | `native`, `claude_compatible`, `shared_agents`, `legacy`, `configured`, `plugin`, `managed_system`, or `other`. |
| `path` | Exact template path; use a provider-independent placeholder only when the provider itself defines one. |
| `support` | `automatic`, `opt_in`, `configured_only`, `symlink_only`, `unsupported`, `unknown`, or `not_applicable`. |
| `conditions` | Non-empty explanation of flags, trust, traversal, version, startup, or reload conditions; say `none` when verified unconditional. |
| `evidence_ids` | References into the evidence registry. |
| `reason` | Required explanation for `unknown` and `not_applicable`. |
| `gap_id` | Required for `unknown`; omitted for resolved and not-applicable findings. |

`configured_only` and `symlink_only` explicitly do not count as native discovery.
Do not create synthetic location rows for paths that have no meaning for the
provider; represent the mandatory investigation through `root_coverage` instead.

Add `root_coverage` with exactly one record for each required inquiry:
`native_user`, `native_repo`, `agents_user`, `agents_repo`, `claude_user`,
`claude_repo`, `configured_roots`, `plugin_roots`, and `nested_traversal`. Each
record contains `case`, `status` (`supported`, `unsupported`, `unknown`, or
`not_applicable`), `location_ids`, `evidence_ids`, `reason`, and optional
`gap_id`. `location_ids` must resolve; `supported` must name at least one
location; `unknown` must name a gap; `not_applicable` must explain why the
inquiry has no provider meaning.

Add `collisions` referring to location IDs and capturing `case`, optional
`left_location_id` and `right_location_id`, `identity_key`, `stage`,
`activation_mode`,
`outcome`, `winner_location_id`, `survivor_location_ids`, `selection`,
`conditions`, `evidence_ids`, `reason`, and optional `gap_id`. Use `stage:
discovery | activation | execution | unknown | not_applicable` and `outcome:
override | coexistence | merge | namespace_separation | rejection | unknown |
not_applicable`. `winner_location_id` is present only for `override`;
`survivor_location_ids` is non-empty for `coexistence` and
`namespace_separation`; `merge` explains the merged unit in `selection`.
Applicable cases require both location references except `metadata_name`,
`identical_copy`, and `symlink_alias`, whose compared paths may be represented by
one canonical location plus details in `conditions`. Unknown cases reference
every known side; not-applicable cases may omit both.

`activation_mode` is `explicit`, `implicit`, `not_applicable`, or `unknown`.
Discovery records use `not_applicable`. Each applicable collision case has a
discovery record plus distinct explicit- and implicit-activation records;
unknown is an acceptable researched result for either activation mode. Add an
execution-stage record only where execution can diverge after activation.

The required collision cases are the eight rows in R2's table, with stable case
IDs `scope_same_family`, `family_same_scope`, `family_cross_scope`,
`parent_nested`, `plugin_filesystem`, `metadata_name`, `identical_copy`, and
`symlink_alias`, plus `description_budget` and `disable_aliases` for the two
follow-on questions at the end of R2. A provider may add narrower records for
multiple applicable root pairs, but it must provide the required stage/mode
records for every case ID. A broad precedence statement cannot satisfy this
coverage.

Add top-level `gaps` records containing unique `id`, `area`, `question`,
`reason`, and `next_check`. Every `unknown` location, root-coverage, collision,
or frontmatter behavior record references one live gap. `not_applicable` needs a
reason but not a gap because it is a resolved applicability finding. Orphan gaps
fail validation so stale uncertainty does not survive after a finding is
resolved.

Darkmatter's `md schema validate` remains the shape gate. Add a skills-specific
relational checker beside `steering_check` in `claudine-gen`, exposed through
`claudine providers skills check [<slug>] [--json]`. It validates revision and
provider slug, uniqueness, reference integrity, required root/collision/property
case coverage, outcome-dependent fields, gap ownership, and the concrete-evidence
rule. Both the per-provider and fleet forms load reports through the existing
validated-frontmatter input path and use the active roster rather than a copied
provider list. Do not add these domain rules to Darkmatter.

Update the prompt's questions, deliverables, frontmatter instructions, examples,
body structure, and exit criteria together. Examples must demonstrate both
override and coexistence without presenting either as a universal default.
Require root and collision tables in the prose, with matching structured facts.
The property inventory and typed behavior records in R6–R9 are part of the same
required contract. Remove the existing `format.required_fields` and
`optional_fields` arrays from the revision-2 schema
and fleet prompt/examples instead of maintaining a derived duplicate; generate any
human-facing required/optional lists from the typed property inventory. Retain
`format.file_names`, `format.frontmatter`, `format.body_format`, and format notes.
Require a frontmatter behavior table in each report.

### R5. Refresh the fleet and preserve results downstream

The current initialization gate skips reports updated within fourteen days.
Replace its date-only definition of current with one predicate: the file exists,
its body is non-empty, `schema_revision == 2`, `last_updated` is within fourteen
days, `contract_validated == true`, and `validate_schema(file)` passes. A
missing/older revision, false/missing validation marker, or shape failure routes
the provider into research even when its date is recent. Before launching that
research, initialization sets `contract_validated: false` on an existing report;
a newly created report starts with false. The success gate runs `md schema
validate` and `claudine providers skills check <slug>`, requires `last_updated ==
ctx.today`, and only then sets `contract_validated: true` as its final action.
Every failure path leaves or restores false. Preserve `created`, document
changes, and retain honest gaps. The marker mutation uses the existing
`set_frontmatter` effect, including its atomic-write and auto-rehash behavior.

The relational checker is deliberately not duplicated as a lifecycle expression
function. The current lifecycle engine cannot branch on a shell action's exit
status, so pretending to run it as the initialization predicate would either
abort refresh of a stale report or require a new general orchestration feature
outside this fix. The managed marker records that the report passed the
relational success gate; repository corpus tests and the fleet-form checker catch
later manual drift.

Refresh all research-enabled roster entries through the normal fleet route.
Use Claude and Codex as initial pilots, plus a non-Claude provider for which
native and Claude-compatible discovery can be verified. Complete the remaining
roster after the pilots establish that the contract captures real differences.
Do not hard-code today's provider count or revive `skip_research` entries.

Land the schema, checker, fixtures, and fleet prompt before rewriting provider
reports, but treat that intermediate state as migration-in-progress rather than
a green fleet. Pilot reports may be revision 2 while other active reports remain
legacy; the fleet-level checker must report those legacy documents as failures.
After the three pilots validate the vocabulary, refresh every remaining active
provider in the same change. Do not weaken required fields or add compatibility
defaults merely to make legacy documents pass.

Update `claudine/docs/research/summary/agent-skills.md`'s generation instructions
and regenerate its comparison from the refreshed reports. Include separate
scope, root-family, collision, frontmatter, and evidence-gap comparisons. Show
`.agents/skills` support separately for user and repo scope, whether `name` is
required, and recognition and invalid-value behavior for `model` and
`allowed-tools`. Explain when linking creates redundant catalog entries,
changes effective selection, or carries unusable model/tool identifiers.
Publish the refreshed skill summary through the existing
`just publish-summary-research` workflow. Update the Claudine skill's research
guidance if the contract/workflow changes. Maintain Markdown hash properties
with Darkmatter's hashing functionality.

### R6. Inventory required and optional recognized properties

Research every frontmatter property recognized by the provider, including
standard properties and vendor extensions. Record exact spelling, accepted
types/value grammar, requiredness, defaults, semantic effect, and relevant
scope/mode/version conditions. Distinguish a property parsed and acted upon by
the CLI from arbitrary metadata merely exposed to the model as text. Separate
sidecar configuration from `SKILL.md` frontmatter; a similarly named sidecar
setting is not proof that a frontmatter key is recognized.

Use typed records, with these minimum fields and vocabularies:

| Field | Contract |
| --- | --- |
| `id` | Stable unique ID referenced by behavior records. |
| `property` | Exact frontmatter key; unique within a condition set. |
| `recognition` | `recognized`, `unrecognized`, or `unknown`. |
| `requirement` | `required`, `optional`, `conditional`, `not_applicable`, or `unknown`; conditional requires an explanation. |
| `value_types` | Explicit supported YAML types: `string`, `sequence`, `mapping`, `boolean`, `integer`, `number`, or `null`; the array may be empty only when a gap records that accepted types remain unknown. |
| `effect` | Explanation of the CLI's interpretation, default, and precedence against other settings. |
| `conditions` | Relevant version, mode, root/scope, configuration, or account restrictions. |
| `evidence_ids` | References to the versioned evidence required by R3. |
| `gap_id` | Required when recognition, requirement, or accepted types remain unknown. |

Require explicit records for `name`, `description`, `model`, and `allowed-tools`
even when unrecognized or unknown. Other properties form the researched inventory,
not a fixed list restricted to these four. Report the handling of unknown keys
and wrong YAML types as well as recognized valid values.

Behavior cases must also be structured under `frontmatter_behaviors`:
`property_id`, `case`, `handling`, `stage`, `diagnostic`, `effective_values`
(a string array), `conditions`, `evidence_ids`, `reason`, and optional `gap_id`. Use
`handling: accepted | defaulted | ignored | partially_applied | skill_rejected |
activation_blocked | session_failed | unknown | not_applicable` and
`diagnostic: none | warning | error | unknown | not_applicable`. Record the stage
as `discovery`, `activation`, `execution`, `unknown`, or `not_applicable`.
Multiple stage records may describe one case. A warning can accompany a fallback;
do not collapse these independent facts into one outcome. Unknown and
not-applicable results require reasons; unknown results also require a live gap.
The checker rejects duplicate `(property_id, case, stage, conditions)` records.
For a not-applicable case, handling, stage, and diagnostic are all
`not_applicable`; an unknown case uses the corresponding `unknown` values rather
than borrowing a concrete stage or diagnostic.
The relational checker enforces one or more records for every required case in
R7–R9 and rejects references to a property other than the matching mandatory
property. Use explanatory detail for provider nuance rather than pretending
these enums alone establish behavior.

Add `frontmatter_global_behaviors` with the same behavioral fields except
`property_id`. It must contain the `unknown_key` case so arbitrary metadata
handling cannot disappear from prose or be falsely inferred from a known-key
test. Providers may add global malformed-frontmatter cases; syntax parsing in
general is not expanded into a second YAML-parser research topic.

### R7. Determine whether frontmatter name is required

Research the distinction between specification requirements and the CLI's
actual acceptance rules. A skill directory already has a name, so do not assume
that a missing frontmatter `name` invalidates the skill.

Require `name_behavior` with `fallback: directory_name | other | none | unknown`,
`directory_match: required | not_required | conditional | unknown`,
`evidence_ids`, `reason`, and optional `gap_id`. Explain any `other` fallback or
conditional rule. Unknown values require a live gap. Link this result to R2's
collision identity rather than independently guessing how duplicates are keyed.

Required cases are a valid matching name, omitted name, empty name, null name,
wrong type, invalid syntax, and a valid name differing from the directory.
Use the stable case IDs `valid_matching`, `omitted`, `empty`, `null`,
`wrong_type`, `invalid_syntax`, and `directory_mismatch`.
For accepted cases, record the effective catalog and invocation names and any
normalization. For rejected cases, record whether the skill is skipped, a warning
is emitted, or the wider session fails. An omitted value is not interchangeable
with an explicit empty or null value.

### R8. Determine model selection and unusable-model behavior

Investigate whether the skill's `model` property is used to choose the model.
Record accepted identifiers and aliases, the default when omitted, which
execution receives the selection, when it takes effect, and precedence against
the invocation's model, CLI flags, and configuration. Describe any restoration
after skill execution where applicable. A provider-specific departure from
model-selection semantics must be evidenced, not assumed.

Require `model_behavior` to record `identifier_grammar`, `model_sources`,
`validation_stage`, `precedence`, `restoration`, `evidence_ids`, `reason`, and
optional `gap_id`. `model_sources` is an array drawn from `built_in_catalog`,
`configured_providers`, `account_availability`, `combination`,
`not_applicable`, and `unknown`; `validation_stage` is `discovery`, `activation`,
`execution`, `backend`, `not_applicable`, or `unknown`. Cross-reference the existing `agent-models` and
`model-config` research rather than embedding a second model catalog. Record
whether validation happens locally or only when a model request reaches a
backend. Unknown behavior requires a live gap.

Required cases are omission, a known usable model, a syntactically valid but
unknown identifier, a known identifier unavailable to the current account or
configuration, and a wrong YAML type. Where aliases are supported, include one.
Use the stable case IDs `omitted`, `known_usable`, `unknown_identifier`,
`known_unavailable`, `wrong_type`, and `alias` (the last may be
`not_applicable`).
State whether an unusable model causes an error, warning, ignored setting,
fallback, skipped skill, or failed activation/session, and identify the effective
fallback model when observable. Distinguish an unrecognized property from a
recognized property containing an unrecognized model. If a valid skill is
listed but later fails on model selection, preserve both stage results.

Treat model identifiers as a portability constraint even when the property has
the same meaning across providers. Do not infer that the same identifier is
available or equivalent everywhere, and do not prescribe silent model rewrites.

### R9. Determine allowed-tools semantics and unknown-tool behavior

Investigate whether `allowed-tools` is recognized, its accepted encoding
(including strings versus YAML sequences), identifier matching, namespaces,
argument patterns, wildcards, and treatment of dynamically provided/MCP tools.
Do not assume that a tool name used by one CLI exists in another.

Require `allowed_tools_behavior` with a `semantics` collection drawn from
`permission_preapproval`, `availability_filter`, `advisory`, `other`,
`not_applicable`, and `unknown`, plus `encoding`, `identifier_grammar`,
`evidence_ids`, reason, and optional `gap_id`. Explain combinations and `other`;
unknown behavior requires a live gap. State whether
the property restricts tools, bypasses ordinary permission prompts for named
tools, or has another effect; these are materially different contracts. Record
how it interacts with session permissions, deny rules, and tool availability.

Required cases are omission, a valid existing tool, only a nonexistent tool,
a mixture of existing and nonexistent tools, an empty value, and a wrong YAML
type. Include malformed patterns and an unavailable external/MCP tool where
those mechanisms are supported. Capture whether unknown entries are ignored
individually, invalidate the whole list or skill, warn, or fail only when invoked.
Use the stable case IDs `omitted`, `valid_existing`, `nonexistent_only`,
`mixed_existing_nonexistent`, `empty`, `wrong_type`, `malformed_pattern`, and
`unavailable_external`; the final two may be `not_applicable` only with a
provider-specific reason.
Record the effective treatment of valid entries in mixed lists. Separate
nonexistent tools from existing tools denied by policy and, where relevant,
tools registered later in the session.

Use harmless fixture tools and isolated permissions for observations. Visibility
of a tool or successful skill loading alone does not prove permission behavior.
Report uncertain enforcement explicitly. Link these findings to portability
notes without changing Claudine's runtime permission policy in this fix.

## Implementation Boundaries

The implementation has four owned layers:

1. `docs/research/skills/_schema.yaml` owns the revision-2 shape and enum
   vocabulary; `_fleet.md` owns research instructions, freshness routing, and
   per-provider success gates.
2. `claudine-gen` owns relational validation in a dedicated `skills_check`
   module, following `steering_check` without sharing topic-specific rules.
3. `claudine providers skills check` is a thin CLI projection of that library
   checker. Its human output uses the existing terminal rendering conventions;
   `--json` remains stable, pipeable data on stdout and diagnostics/status stay
   on stderr.
4. Provider reports and the Agent Skills summary own researched facts. The
   published Claudine skill is a downstream copy, not a second source of truth.

Do not modify the runtime shared-resource linker, provider metadata generator,
or provider precedence behavior in this fix. Findings that require those
changes set `requires_claudine_update: true`, explain the affected behavior in
`reason`, and enter separately scoped follow-up work.

The checker must be deterministic and offline. It reads only the roster,
sidecar-validated provider reports, and referenced repository-local fixtures; it
does not invoke provider CLIs, inspect user homes, browse, or follow external
URLs. Live research remains an explicit fleet operation.

## Validation and Acceptance Criteria

1. Every active provider report validates against the updated schema and carries
   the new contract revision. Every required scenario has a supported answer or
   a reasoned unknown/not-applicable record; empty arrays cannot bypass coverage.
2. Reports distinguish scope from root family and include evidence for native,
   Claude-compatible, and shared-root discovery investigations. Unsupported
   roots remain visible as researched negative findings, not omitted rows.
3. The collision model can represent override and coexistence, cross-scope and
   cross-family conflicts, plugin namespacing, and physical-target deduplication
   without contradictory blanket precedence claims.
4. Fixtures prove that missing collision coverage, dangling root/evidence
   references, and unsupported outcome values fail validation. Valid unknown
   findings with reasons and evidence gaps pass.
5. A recently dated old-contract report is selected for refresh; a recently dated,
   validated current-contract report retains the normal freshness skip. A
   schema/checker failure leaves `contract_validated: false`, and the next run
   does not skip that report. Failed validation cannot report fleet success.
   Cover this through the normal
   composition/sequence invocation path using isolated fake providers, alongside
   passive validation of the shipped fleet prompt and report corpus.
6. Prose, structured facts, generated comparison, and published skill summary
   agree. Pilot evidence explicitly distinguishes direct discovery from linked
   visibility and catalog coexistence from activation behavior.
7. Record validation commands/results and unresolved research gaps. Do not claim
   cross-platform observation from documentation alone. Live probes use disposable
   homes/repos/configuration, never modify real user skills, and never focus
   terminal or browser windows. Use existing fixture, silent-audio, and nextest
   conventions for automated regression coverage.
8. Every report explicitly answers native `.agents/skills` discovery at user
   and repo scope. Configured-path and symlink access cannot be reported as
   native support without independent evidence.
9. Each report contains required and optional recognized properties with types,
   effects, and evidence. The four mandatory property records and all R7–R9
   cases are present, with reasoned unknown/not-applicable findings where needed.
   Missing cases, invalid enums, contradictory requiredness, and dangling
   evidence references fail the contract checks.
10. Valid corpus fixtures represent directory-name fallback versus required
    frontmatter names, recognized versus ignored `model`, model errors versus
    fallback, and unknown-tool rejection versus partial acceptance of a mixed
    list. Diagnostics and handling remain separately expressible. These fixtures
    test the research representation, not invented claims about real providers.
11. Provider research includes attributable observations or source/documentation
    findings for unknown model and tool values, identifies the failure stage,
    and explains resulting portability limits. Reports and summaries never
    equate property recognition with universal model/tool availability.
12. Checker tests cover duplicate IDs, dangling references, missing required
    case IDs, invalid outcome-dependent fields, inference-only concrete claims,
    missing/orphan gaps, provider-slug mismatch, legacy revisions, and
    `skip_research` roster handling. Each negative fixture fails for the intended
    reason; a complete override example and a complete coexistence example pass.
13. The package area's `just test`, `just lint`, and applicable passive
    schema/corpus checks pass. The full active skills fleet passes both
    `md schema validate` and `claudine providers skills check`; the summary is
    regenerated and `just publish-summary-research` leaves no stale published
    copy. Live probes remain non-interactive and do not focus terminal or browser
    windows.

Implementation is complete when the refreshed research and validation evidence
are ready for review. The author closes the review cycle and moves the fix into
`_completed`.
