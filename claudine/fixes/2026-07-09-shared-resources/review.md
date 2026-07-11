# Shared Resource Review

Date: 2026-07-09

## Scope and standard

This review covers the production paths behind:

1. Agent Skills
2. Agent/subagent definitions
3. Slash commands / prompts
4. MCP

The provider roster is the compiled ten-provider roster: Antigravity, Claude Code,
Codex, Gemini CLI, Goose, Kilo Code, Kimi Code, OpenCode, Pi, and Qwen Code.

The standard used here is stricter than "there is a generated field for it":

- provider support, paths, formats, portability, and activation decisions must come
  from `provider_info()` or another generated catalog surface;
- every compiled provider must receive an explicit supported, degraded, conversion,
  inventory-only, or unsupported disposition;
- list/analyze/apply must honor the configured canonical provider for each scope and
  resource;
- a provider-native format must either round-trip safely or be rejected without
  mutation; and
- adding a provider or changing research facts must fail a fleet invariant until the
  relevant resource behavior is explicitly implemented.

## Executive finding

The shared-resource functionality is **not fit for purpose yet** under that standard.
The provider metadata work supplied the right research and a generated catalog entry
point, but the production resource workflows still implement an older Claude-centric
linker. MCP has a cleaner provider behavior boundary, but its support decisions are
hand-written rather than research-generated, cover only four of ten providers, and are
not round-trip safe for all four.

The highest-risk issue is not merely missing provider coverage. The CLI displays the
configured canonical provider and then ignores it during list and apply operations.
That makes the current UI claim a provider-driven behavior that the mutation path does
not implement.

## Findings

### P0 - Skills, commands, and agents ignore the configured canonical provider

All three production workflows hard-code Claude as the source for both user and repo
scope:

- `lib/src/linking/skills/portable.rs:21-33` and
  `lib/src/linking/skills/native.rs:20-32`
- `lib/src/linking/commands.rs:149-166` and `lib/src/linking/commands.rs:273-287`
- `lib/src/linking/agents.rs:140-157` and `lib/src/linking/agents.rs:235-249`

Each apply loop then skips `Provider::Claude` explicitly and treats every other
provider as a target (`skills/native.rs:50-57`, `commands.rs:174-183`, and
`agents.rs:165-173`). In contrast, the CLI renders the configured user/repo canonical
provider in `cli/src/commands/link_display.rs:40-75` and init persists per-resource
canonical slots through `lib/src/linking/canonical.rs:117-150`.

Consequences:

- selecting Codex, Gemini, OpenCode, or another Markdown provider as canonical does
  not change what `claudine skills`, `claudine commands`, or `claudine agents` lists;
- `--apply` still reads and may normalize files under Claude's roots;
- resources that exist only in the configured canonical provider are reported as
  absent or are invisible; and
- a user can be told that one provider is canonical while mutations originate from a
  different provider.

The newer provider-neutral detector, canonical selection, and compatibility APIs are
not used by production call sites. `SkillsDetector`, `SlashCommandsDetector`, and
`AgentDefinitionsDetector` are only referenced by their own tests, and
`classify_canonical_candidate` / `classify_target_reference` are likewise unused
outside tests.

**Required correction:** replace the three parallel Claude-specific list/fix paths
with one resource engine that loads the per-scope canonical provider, detects all
provider roots, classifies canonical candidates, and plans target-specific actions
before mutation. Until then, apply should fail closed when the configured canonical
provider is not Claude rather than silently using Claude.

### P1 - The generated resource model cannot express the ratified portability contract

The metadata feature ratified five target-specific classes:
`Portable`, `PortableWithProviderMapping`, `LinkedButDegraded`, `RewriteRequired`, and
`NonPortable`, plus `artifact_kind`, conversion (`none`, `mechanical`, `semantic`),
precedence, activation gates, and inventory-only scopes
(`features/2026-07-02-provider-metadata/spec.md:300`).

The implemented generated type still exposes only:

- `SupportLevel::{Full, CustomFormat, Limited, None}`
  (`lib/src/linking/capabilities.rs:117-160`);
- one `ResourceFormat`, one user path, one repo path, and a flat
  `also_reads_from` list (`capabilities.rs:188-205`); and
- required/optional property names without mapping rules
  (`capabilities.rs:162-186`).

There is no source-provider x target-provider portability decision, no artifact kind,
no conversion recipe, no precedence model, no trust/activation disposition, and no
inventory-only source representation. Production behavior consequently reduces every
decision to "custom?" and "Markdown?" (`linking/paths.rs:113-134`,
`commands.rs:179-190`, `agents.rs:170-181`).

This is behaviorally wrong, not just incomplete typing:

- Gemini TOML commands and Goose recipe/config commands are counted as format
  incompatible instead of receiving the ratified mechanical conversion.
- Goose and Kimi YAML agent definitions are excluded rather than translated or
  classified.
- provider-specific fields are handled by a hard-coded Claude-property blacklist,
  not target mappings.
- same-format Markdown is assumed symlink-safe even where model, tool, permission,
  argument, or activation semantics are degraded.
- Antigravity's skill-derived commands cannot be expressed as `RewriteRequired`; the
  facts collapse them to `None` (`docs/providers/facts/antigravity.yaml:125-132`).

**Required correction:** make the five-class portability decision and conversion
metadata generated catalog types. The apply planner must consume the pairwise result;
only `Portable` may create a direct symlink. Mechanical conversions should create
owned generated artifacts with provenance, never symlinks.

### P1 - Central resource metadata is stale relative to its authoritative research

`resource_support` is still sourced from hand-authored facts
(`gen/src/registry.rs:416-424`), while only `supports_skills` is sourced directly from
research. This has already produced material contradictions:

- Codex subagents are TOML configuration layers (`docs/research/subagents/codex.md:78-102`),
  but central facts say Markdown (`docs/providers/facts/codex.yaml:106-114`). The
  current apply path can therefore symlink a Claude Markdown agent into
  `.codex/agents`, where Codex expects `*.toml`.
- Gemini research says `~/.agents/skills` and `.agents/skills` are supported and take
  precedence over the Gemini-branded roots, but Gemini's central
  `also_reads_from` is empty (`docs/providers/facts/gemini.yaml:159-170`).
- most `properties.source_doc` values point to the removed
  `claudine/docs/cross-referencing/*.md` tree instead of the live
  `docs/research/{skills,slash-commands,subagents}` documents.

The generated catalog therefore centralizes values, but does not make them current or
authoritative. Runtime code consuming those values can still make the wrong decision
perfectly consistently.

**Required correction:** graduate the shared resource schema from the three research
topics rather than maintaining a second facts model. Generation must reject missing
evidence paths and contradictions between `format`/`artifact_kind`, locations, and
support level.

### P1 - Resource inventory is canonical-only and omits provider-native assets

The public list commands scan only Claude's user and repo roots and construct every
reported item with `provider: Provider::Claude` (`skills/portable.rs:18-100`,
`commands.rs:273-345`, `agents.rs:235-307`). They do not inventory:

- isolated definitions in any other provider's native root;
- generic `.agents` roots unless they happen to be reached as a target exception;
- provider-compatible alternate roots (singular/plural paths and compatibility
  brands);
- bundled, managed, plugin, extension, URL, marketplace, or built-in sources; or
- non-Markdown native formats, even though the unused detector can recognize several.

`also_reads_from` is used only as a reason to suppress missing-link diagnostics. Those
directories are not actually scanned, so Claudine cannot detect collisions, shadowing,
or whether a compatible source is present. A string containment check such as
`contains(".claude/skills")` substitutes for the precedence and compatibility facts
the research collected (`skills/portable.rs:203-209`).

This means "list skills/commands/agents across providers" is not an accurate
description of current behavior. It lists the Claude canonical set and reports a
subset of expected target links.

**Required correction:** inventory all research-declared locations with provenance,
scope, origin kind, precedence tier, and compatibility origin. Keep inventory separate
from sync eligibility so managed/bundled/plugin resources remain visible but are never
silently copied.

### P1 - MCP support is hand-written and covers only four of ten providers

`ProviderInfo.mcp` points at a behavior trait object, but whether MCP is supported is
decided by hand-written `behavior.rs` overrides. Only Claude, Codex, Gemini, and
OpenCode override `supported()`; the other six inherit `false`
(`lib/src/provider/behavior.rs:65-85` and `lib/src/provider/*/behavior.rs`). No generated
MCP capability descriptor exists in `ProviderInfo`, even though the MCP research fleet
has an explicit `support`, config, schema, and runtime-injection contract.

Current compiled-provider coverage is:

| Provider    | Native posture in current research       | Claudine import/export | Runtime injection |
| ----------- | ---------------------------------------- | ---------------------: | ----------------: |
| Claude      | import/sync plus native one-run config   |                    yes |                no |
| Codex       | import/sync                              |                    yes |               yes |
| Gemini      | import/sync                              |                    yes |               yes |
| Goose       | import/sync plus argv one-run activation |                     no |                no |
| Kimi        | import/sync                              |                     no |                no |
| OpenCode    | runtime injection                        |                    yes |               yes |
| Qwen        | import/sync                              |                     no |                no |
| Kilo        | runtime injection                        |                     no |                no |
| Pi          | no native MCP                            | explicitly unsupported |                no |
| Antigravity | import/sync                              |                     no |                no |

The metadata spec had already identified Goose, Kimi, Qwen, and Claude runtime
injection as unintended last-mile gaps
(`features/2026-07-02-provider-metadata/spec.md:324-332`). Since then Kilo and
Antigravity joined the compiled provider roster without corresponding behavior. The
MCP summary is also stale: it still includes removed Roo and describes Pi/Kilo as not
compiled (`docs/research/summary/mcp.md`, provider matrix).

**Required correction:** generate a typed MCP capability record covering native
support, Claudine import/export/apply, runtime strategy, supported scopes/transports,
and an explicit implementation status. Dynamic parsers/writers/injectors should remain
traits, but availability and routing preconditions must be generated metadata checked
against trait conformance.

### P1 - MCP import/export is not round-trip safe for wired providers

The normalized catalog claims to be a provider superset
(`lib/src/mcp/types.rs:51-105`), but the current parsers and writers lose or misname
provider-native fields.

The clearest failure is Gemini:

- current research defines `url` and `httpUrl`, camelCase `includeTools` /
  `excludeTools`, headers, OAuth, and `type` (`docs/research/mcp/gemini.md:194-200`);
- the parser reads `transport`, `url`, and hyphenated `include-tools` /
  `exclude-tools`, and ignores `httpUrl`, headers, OAuth, and auth-provider fields
  (`lib/src/mcp/import.rs:579-638`);
- the writer and runtime injector emit `url` for every remote transport, omit the
  transport discriminator, and emit the same hyphenated tool-filter keys
  (`lib/src/mcp/export.rs:420-447`, `lib/src/mcp/inject.rs:305-330`).

An ordinary Streamable HTTP Gemini entry can therefore import as stdio with no
command, and export/injection can change transport or silently drop auth and filters.

Claude also loses operational fields: its parser initializes headers to an empty map
and its writer never emits headers (`lib/src/mcp/import.rs:379-408`,
`lib/src/mcp/export.rs:258-286`). OpenCode intentionally collapses HTTP and SSE into
one `remote` representation, while the catalog fingerprint excludes all
`provider_overrides` (`lib/src/mcp/types.rs:108-137`), allowing definitions with
different provider policy/auth overrides to deduplicate as if equivalent.

Because `mcp sync` imports automatically and `mcp export --apply` mutates native
configuration, lossy normalization is a data-integrity and credential-availability
risk.

**Required correction:** add provider fixture round-trip tests based on the current
research schema before enabling apply. Preserve unmodeled native fields in a lossless
provider payload or reject the entry as non-round-trippable. Fingerprints used for
merge decisions must include behaviorally relevant provider overrides.

### P1 - User path semantics are inconsistent and already generate an invalid path

`ResourceSupport.user_path` is documented as relative to home and path expansion simply
does `home.join(path)` for non-absolute values (`lib/src/linking/paths.rs:145-150`).
Antigravity facts store `~/.gemini/config/skills`
(`docs/providers/facts/antigravity.yaml:151-163`). `Path::is_absolute()` is false for
that value, so Claudine resolves it as `<home>/~/.gemini/config/skills`.

The same duplicated expansion logic exists in the canonical selector and detector
(`linking/canonical.rs:163-197`, `linking/detector.rs:282-315`), so fixing only one call
site would preserve inconsistent behavior.

**Required correction:** define one generated path-template type with explicit
`home`, `repo`, config-root/env-root, and platform semantics. Reject `~/` in fields
declared home-relative, or normalize it during generation. All discovery, canonical
selection, and apply paths must use the same resolver.

### P2 - Fleet tests verify shape, not behavioral coverage

Provider tests confirm that each provider has a `resource_support` object and that the
facade equals the generated catalog (`lib/src/provider/tests.rs:345-383` and
`:935-977`). They do not confirm that:

- the research artifact kind agrees with `ResourceFormat`;
- every evidence path exists;
- every compiled provider has an explicit behavior for each resource;
- a supported mechanical conversion has an implementation;
- canonical selection is honored by list/apply;
- MCP research support agrees with `McpBehavior::supported()` and injector/exporter
  availability; or
- provider-native fixtures survive import/export without semantic loss.

This is why newly compiled providers can appear in the central matrix while remaining
absent from the actual workflows.

**Required correction:** add fleet invariants at the generated metadata/behavior
boundary. A provider graduation should fail until each shared resource has an explicit
implementation disposition and fixture coverage.

## Provider readiness by resource

The following matrix describes Claudine behavior, not provider-native capability.

| Provider    | Skills                                        | Commands/prompts                               | Agents/subagents                                           | MCP                                       |
| ----------- | --------------------------------------------- | ---------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------- |
| Claude      | source-only implementation                    | source-only implementation                     | source-only implementation                                 | import/export; no native one-run injector |
| Codex       | path/link metadata only                       | user Markdown prompt links; deprecated surface | **incorrectly treated as Markdown; native format is TOML** | import/export/runtime                     |
| Gemini      | branded-root links only; generic root omitted | TOML conversion absent                         | Markdown link path, semantics not mapped                   | wired but schema handling is unsafe       |
| Goose       | relies on Claude compatibility scan           | native recipe/config conversion absent         | YAML conversion absent                                     | no Claudine behavior                      |
| Kimi        | relies on compatibility roots                 | correctly not linked as custom command         | YAML conversion absent                                     | no Claudine behavior                      |
| OpenCode    | relies on Claude compatibility scan           | direct Markdown links                          | direct Markdown links without semantic mapping             | import/export/runtime                     |
| Qwen        | direct Markdown links                         | direct Markdown links                          | direct Markdown links without asymmetric mapping           | no Claudine behavior                      |
| Kilo        | relies on Claude compatibility scan           | direct Markdown links                          | direct Markdown links without semantic mapping             | no Claudine behavior                      |
| Pi          | generic-root suppression only                 | direct Markdown prompt links                   | explicitly unsupported                                     | explicitly unsupported                    |
| Antigravity | invalid user path; repo direct link           | skill-derived command rewrite absent           | inventory/support absent                                   | no Claudine behavior                      |

No row is complete across all four surfaces. Skills have the broadest native
convergence, but the inventory/canonical/precedence layer is still incomplete. Commands
and agents need conversion rather than more symlink cases. MCP needs both provider
coverage and a lossless normalized contract.

## Recommended implementation order

1. **Fail closed on canonical mismatch and unsafe MCP apply.** Prevent mutations that
   use a different source than the displayed canonical provider; disable Gemini MCP
   apply/injection until its current schema round-trips.
2. **Graduate executable metadata.** Add generated pairwise portability/conversion
   metadata and a typed MCP capability record sourced from the live research topics.
3. **Unify inventory and planning.** Wire the existing detector/canonical concepts into
   one list/analyze/plan engine for skills, commands, and agents. Inventory all origins;
   mutate only eligible user/repo assets.
4. **Implement format adapters.** Start with deterministic conversions: Gemini
   commands, Codex TOML agents, and the provider pairs classified mechanical by the
   research. Store provenance and drift hashes for generated artifacts.
5. **Close MCP provider gaps.** Add Claude one-run injection, Goose argv activation,
   Kilo inline-config injection, and persistent import/export for Kimi, Qwen, and
   Antigravity where research supports it. Keep Pi explicitly unsupported.
6. **Add fleet gates.** Require every provider/resource disposition and native fixture
   round-trip before a provider can graduate into the compiled roster.

## Exit criteria for this fix

This review should not be considered resolved merely when all ten providers appear in
a table. It is complete when:

- list/analyze/apply honor per-scope, per-resource canonical providers;
- decisions are derived from generated research-backed metadata;
- all provider-native roots are inventoried with origin and precedence;
- every source-target pair has one of the five ratified portability outcomes;
- direct links are limited to truly portable pairs;
- mechanical conversions are deterministic, owned, and drift-detectable;
- MCP capability and implementation status agree for all ten providers;
- import/export/apply is lossless for supported native schemas; and
- fleet tests make provider graduation fail on any missing disposition or behavior.
