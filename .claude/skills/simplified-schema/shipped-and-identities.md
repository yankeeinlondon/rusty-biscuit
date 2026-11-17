# Shipped artifacts and validator identities

Loaded from the `simplified-schema` skill. Read before editing shipped
schema YAML, preparation code, or anything that changes a validator's or
phase projection's identity.

## Shipped artifacts

`schemas/*.yaml` are served from the checked-in `lib/generated/shipped.json`
(`shipped::registry`; a refused artifact silently falls back to the text).
`simplified_schema::shipped` is the mechanism Darkmatter and Claudine share
(`accept`, `generate`, `check_fresh`, `PreparationMode::Uncached`, the
`*_with_mode` entry points, `ShippedSources::with_prepared`). `ShippedSources::with_aliases(&[(stem, path)])`
lets a stem name a shipped document outside the partials folder
(Darkmatter's `config` → `darkmatter-config.yaml`); aliases are part of
equality, memo keys, and the owner's artifact `configuration`. After editing
shipped YAML or its preparation code, run the root `just shipped-schemas`
(it regenerates all three artifacts, all or nothing; `tests/l1/shipped_corpus.rs`
parses every shipped schema artifact). A code-only change
bumps `PREPARATION_REVISION` of every owner whose payload it changes (a
lowering change can alter Darkmatter's and Claudine's payloads without
touching this crate's); see
[shipped-artifacts.md](../../../simplified-schema/docs/topics/shipped-artifacts.md).
The encoder writes the header pretty and each payload member compact on its
own line; a large payload loads with `accept_deferred` (`Sections::take` →
`Section::get`), which checks only layout, header, and member list at load
and decodes each member on first read, refusing that member alone. Tests that
hand-edit an artifact must re-encode it in that layout (or edit the text).

## Validator identities

`ValidatorCache` keys a validator by `(SchemaIdentity, anchor hash)`.
`SchemaIdentity::Canonical` is the canonical-JSON hash
(`SchemaDigest::canonical`, the only path that serializes; counted as
`Work::CanonicalSchemaSerialization`); `SchemaIdentity::Layered` comes from a
`LayeredIdentity` a host builds over ordered `(LayerRole, SchemaDigest)`
layers and phase projections, with fixed little-endian encoding (pinned in
`tests/l1/schema_identities.rs`). The variants never share a slot. Callers
with a known digest use `validator_for_shared_with_digest`,
`CompiledValidators::build_with_digest`, `structural_validator_identified`,
and `ConversionPlan::for_identity`; the canonical APIs are unchanged and
`shared_schema_hash` stays canonical. A shipped record stores a canonical
digest as `shipped::CanonicalDigestRecord` (refused under another
`CANONICAL_IDENTITY_REVISION`). `judges_files` is the one classification
rule; a layered digest ORs it. Changing what `merge_baseline` or a phase
projection produces bumps `resolve::MERGE_ALGORITHM_REVISION` or
`phase::PROJECTION_REVISION`, which changes the pinned identities in
`schema_identities.rs`.
