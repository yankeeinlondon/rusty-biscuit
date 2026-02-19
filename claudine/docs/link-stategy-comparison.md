# Link Strategy Redesign vs Current Implementation

## Scope
This document compares the redesign in `claudine/docs/link-strategy.md` against what is currently implemented in source:

- `claudine/lib/src/linking/mod.rs`
- `claudine/lib/src/linking/paths.rs`
- `claudine/lib/src/linking/discovery.rs`
- `claudine/lib/src/linking/conflict.rs`
- `claudine/lib/src/linking/symlink.rs`
- `claudine/lib/src/linking/capabilities.rs`
- `claudine/cli/src/commands/link.rs`
- `claudine/lib/src/events/config.rs`
- `claudine/lib/src/dispatch/loader.rs`

## Resolved Design Decisions
The following design decisions have now been confirmed:

1. Canonical provider selection is global per `(scope, resource type)`.
2. Non-Markdown formats are excluded from canonical selection, but still participate in sync through derived representations when a converter exists (`DerivedLink`, `DerivedStale`, `DerivedMissing`).
3. Required/optional field contracts per provider/resource type should be built into capabilities metadata, seeded from `claudine/docs/cross-referencing`.
4. Canonical frontmatter upgrades are allowed to mutate canonical source files in place.
5. Multiple non-symlink candidates with different content and no valid canonical assets are treated as `ResourceReference::Isolated`.
6. Repo scope must resolve repo root from cwd using Sniff.
7. Migration from `.hooker` / `.hook-config` to `.claudine` is immediate, with no backward compatibility.
8. Category-level symlinks (resource-root-to-resource-root links) should be flagged and users should be encouraged to move to granular resource links; Claudine must never remove user-created links without explicit permission.

## What Already Matches the Redesign
The current linker already implements several core rules from the redesign:

- Scope-aware symlink target behavior:
  - User scope creates absolute links.
  - Repo scope creates relative links.
  - Implemented in `claudine/lib/src/linking/symlink.rs`.
- Linking is resource-granular (individual skill directory / command file), not directory-wide.
- Non-destructive behavior:
  - Existing real directories are never overwritten.
  - Mismatched existing symlinks are skipped.
- Hash-based conflict detection exists.

## Key Gaps vs Redesign
Major differences between the redesign and current implementation:

1. **Resource coverage**
   - Redesign: skills, slash commands, agents/subagents, shared scripts.
   - Implemented: skills + markdown commands only.

2. **Provider coverage**
   - Redesign assumes all supported providers participate in strategy decisions.
   - Implemented linker hardcodes 4 providers (`claude`, `gemini`, `codex`, `opencode`) in `paths.rs`.
   - Capability metadata exists for 8 providers in `capabilities.rs`, but linker path/discovery logic does not consume it.

3. **Canonical provider model**
   - Redesign introduces canonical/base provider selection, persisted config, preference ordering.
   - Implemented system has no canonical provider state or preference-based selection.

4. **Compatibility and frontmatter normalization**
   - Redesign defines required/optional field checks and upgrade/incomplete states.
   - Implemented system does not parse or validate frontmatter; it hashes raw content.

5. **State model richness**
   - Redesign defines `Source`, `PartialSource`, `Isolated`, `LinkMissing`, `IncompleteLink`, `DerivedStale`, etc.
   - Implemented state model is simpler: `LinkCandidate`, `InSync`, `Conflict`, `AlreadyLinked`.

6. **Format conversion / derived resources**
   - Redesign supports conversion and derived representations.
   - Implemented linker only creates symlinks; no conversion or derived file management.

7. **Config model**
   - Redesign proposes `.claudine` user/repo config with canonical provider metadata.
   - Implemented config is still `.hooker` / `.hook-config`, with no canonical provider fields.

8. **CLI execution mode**
   - Redesign implies active linking service for user and repo scopes.
   - Current `claudine link` runs linker in dry-run mode (`link_skills(..., true)`) and uses user scope only.

## Pros and Cons: New Design

### Pros
- Comprehensive domain model for linking and drift handling across multiple resource types.
- Explicit canonical-source strategy reduces long-term ambiguity if implemented well.
- Stronger interoperability goals via compatibility checks and metadata normalization.
- First-class support for derived artifacts can cover incompatible formats cleanly.
- Clear intent for scope separation and operational safety.

### Cons
- Significant implementation complexity and state explosion.
- Some operational details remain underspecified (exact detection/prompt UX for category-level symlinks, conversion lifecycle and failure policy).
- Requires broad schema and conversion infrastructure before delivering user-visible value.
- Higher risk of partial rollout producing confusing mixed behavior.
- Additional config migration and persistence complexity.

## Pros and Cons: Current Implemented Solution

### Pros
- Simple, understandable 4-phase pipeline (discover, hash, analyze, link).
- Strong safety defaults (skip real directories, skip mismatched symlinks, conflict reporting).
- Good test coverage for core linking mechanics.
- Hash-based comparison is robust and format-agnostic at the binary content level.
- Existing behavior is conservative (read-only CLI path), reducing accidental writes.

### Cons
- Functionally narrow versus stated goals (limited resources/providers).
- Divergence between capability metadata and executable linker logic.
- No canonical provider semantics, preference model, or persistent strategy state.
- No frontmatter-aware compatibility checks or repair guidance.
- No conversion/derivation path for custom formats.
- CLI currently cannot apply links in normal usage.

## Recommendations
Proceed incrementally, preserving the current safety characteristics while adopting the redesign in phases.

1. **Unify source of truth first**
   - Make linker path/discovery generation consume `linking::capabilities` instead of hardcoded provider lists.
   - Remove mismatches (for example command support/path differences between `capabilities.rs` and `paths.rs`).

2. **Ship practical linker upgrades before full redesign**
   - Add explicit apply mode (`--apply`) and scope selection (`--scope user|repo`) to CLI.
   - Keep dry-run as default for safety.
   - Expand active linking to all providers/resources that are already same-format and low risk.

3. **Implement canonical provider model directly**
   - Persist canonical provider decisions and user preferences in config.
   - Enforce canonical provider selection per `(scope, resource type)` in linker behavior.
   - Keep non-Markdown providers out of canonical selection, but include them in derived sync when converter support exists.

4. **Introduce frontmatter contract checks next**
   - Add required/optional contract metadata to capabilities (backed by cross-referencing research).
   - Parse frontmatter and classify `Source` vs `PartialSource`.
   - Apply canonical upgrade in place where mappings are deterministic (`name` from filename, key alias duplication, etc.).

5. **Defer conversion engine until contracts stabilize**
   - Implement conversion as separate module with explicit adapter tests.
   - Start with one high-value path (for example markdown command to TOML/YAML) before generalizing.

6. **Config migration plan**
   - Switch immediately to `.claudine` for both user and repo configs.
   - Do not implement backward compatibility reads for `.hooker`/`.hook-config`.

7. **Testing strategy**
   - Add integration fixtures for all providers and both scopes.
   - Add regression tests for canonical selection by `(scope, resource type)`, partially compatible frontmatter, and derived stale detection.
   - Add tests for repo-root detection using Sniff from nested working directories.

### Bottom Line
Use the current linker as the operational baseline, then layer in redesign capabilities in bounded slices. This preserves reliability while moving toward the richer canonical/compatibility-driven model.
