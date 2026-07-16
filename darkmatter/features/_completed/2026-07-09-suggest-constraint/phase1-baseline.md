# Phase 1 Baseline Map

This map records the repository and implementation baseline used by the
`suggest(...)` acceptance scaffolding. Paths are relative to the repository
root.

## Package and recipe authority

`sniff repo` reports `darkmatter` as the current package area and Cargo as the
workspace authority. `sniff repo package-count --json` reports 72 discovered
packages. `cargo metadata --no-deps --format-version 1` confirms that the
package area contains these workspace members:

| Package | Manifest |
|---|---|
| `darkmatter` | `darkmatter/lib/Cargo.toml` |
| `darkmatter-cli` | `darkmatter/cli/Cargo.toml` |
| `dmls` | `darkmatter/dmls/Cargo.toml` |

`sniff repo test-runner --json` identifies `cargo nextest run` for all three
packages. The package-area `justfile` is the recipe authority:

- `just test` runs the non-resource nextest suites for `darkmatter`,
  `darkmatter-cli`, and `dmls`, excluding the `level2_`, `level3_`, `browser_`,
  and `real_` name tiers.
- `just test-l2` runs the three packages through the shared `_test_l2` recipe.
- `just lint` runs the shared `_lint` recipe for all three packages.

No workspace membership or test command in this plan is inferred from a
directory name.

## Darkmatter schema flow

1. `simplified/mod.rs::parse_yaml_schema` classifies the YAML payload as one
   `SchemaShape` or a root union. Property scalars and property-union scalar
   arms route to `grammar.rs::parse_type_expr`.
2. `grammar.rs` lexes every token with an expression-relative byte range. Its
   argument parser already owns comma separation, single/double quotes, and
   escape decoding. `parse_constraint_list` and `parse_one_constraint` enforce
   type and array-level eligibility.
3. `types.rs` owns `Constraint`, `PropertyAtom`, `PropertyDef`, and the root
   schema shapes. `Clone` and `PartialEq` derive directly over constraint
   payloads.
4. `serialize.rs::serialize_property_atom` and its exhaustive
   `write_constraint` match produce the canonical authoring expression used by
   round-trip tests.
5. `convert.rs::to_json_schema` lowers shapes and unions. `atom_to_fragment`
   hoists universal metadata, then the exhaustive inline-object, array,
   string, number, file, enum, URL, and universal-only constraint matches
   decide keyword placement or rejection. Array item constraints are lowered
   by the scalar fragment before the array wrapper is built.
6. `resolve.rs::resolve_schema` handles inline mappings/sequences and file
   references. Referenced YAML is currently SimplifiedSchema only when its root
   has a mapping/sequence `$schema`; otherwise it is treated as raw JSON
   Schema. Named imports are expanded before conversion. `ResolvedSchema`
   retains the simplified projection plus import, example, and directly
   referenced dependency paths.
7. `validate.rs::ValidatorCache` compiles the generated Draft 2020-12 schema.
   Its SHA-256 cache key covers the serialized JSON Schema and document base
   directory, so a generated suggestion annotation becomes part of validator
   identity without a separate cache field.
8. `completion.rs` reads `EffectiveSchema.simplified`, selects property/root
   union arms, and currently exposes file, enum, and format-hint completions.
   It is the library seam for a typed suggestion query; raw JSON Schema has no
   simplified projection.
9. `about.rs` owns the public constraint catalog and contains a parity test
   that enumerates constraint variants and compares `Constraint::keyword`.
10. Compose calls `schema_validation::run` from
    `compose/pipeline/mod.rs` after first-pass interpolation and before shell
    expansion. It builds one `EffectiveSchema`, coerces pending-safe values,
    validates, and normalizes eager file references. Candidate lint must remain
    outside this failure path.

## Constraint matches and hash inputs

Adding `Constraint::Suggest` must update these exhaustive production matches:

- `simplified/types.rs::Constraint::keyword`
- `simplified/serialize.rs::write_constraint`
- `simplified/convert.rs::{inline_object_fragment, apply_array_constraints,
  string_fragment, number_fragment, file_fragment, enum_fragment,
  url_fragment, reject_unsupported}`

It must also be considered by the converter's universal-metadata hoist and
`apply_object_arity`, even though their wildcard arms make them compile without
an update. Read-only consumers with wildcard matches that require an explicit
semantic decision are `schemas/completion.rs`, `schemas/resolve.rs`,
`compose/context/catalog.rs`, and `dmls/providers/frontmatter.rs`. Detection
must not synthesize suggestions.

The affected cache identities are:

- `compose/cache/hashing.rs::options_hash`: converts a baseline
  `SimplifiedSchema`, canonicalizes the JSON, and xxHashes the options record.
  Suggestion metadata must convert successfully so distinct lists do not
  collapse into the existing conversion-error sentinel.
- `schemas/validate.rs::canonical_hash`: hashes generated JSON Schema bytes and
  the base directory, so `x-darkmatter-suggest` is included automatically.
- `dmls/overlay/mod.rs::schema_cache_key`: xxHashes the open document text and
  schema config; dependency files are separately byte-hashed by
  `collect_dep_hashes`. Standalone authoring buffers need classification in the
  cache/router path, not a filename-derived key.

## DMLS flow

1. `workspace/documents.rs::DocumentStore` retains full-sync open buffers and
   negotiated source maps. `router.rs` creates a `DocumentContext`, requests an
   overlay, invokes the provider registry, and publishes versioned diagnostics.
2. `overlay/frontmatter.rs::FrontmatterAst` calls Darkmatter's
   `extract_frontmatter_block`, parses the YAML slice losslessly, and projects
   every entry to document-relative byte spans. It currently activates only for
   Markdown frontmatter.
3. `overlay/schema.rs::assemble` applies the Darkmatter baseline, matching
   extension baselines, and the document `$schema`, then calls
   `DarkmatterSchemas::effective_for`. It retains extension shapes and all
   dependency paths needed by the overlay cache.
4. `diagnostics/frontmatter.rs` maps current YAML errors, schema preparation
   failures, validation problems, and style warnings to LSP diagnostics.
   `diagnostics/codes.rs` already owns source `darkmatter.schema`; the new stable
   code belongs beside the other schema codes.
5. `providers/frontmatter.rs` detects a scalar key/value context from the
   `FrontmatterAst`, resolves nested inline-object paths, and offers enum,
   boolish, or file completions. It does not yet detect block/flow sequence
   item contexts or read suggestion candidates.
6. `dmls/tests/lsp_session.rs::ClientFixture` drives bounded in-memory LSP
   sessions and captures push diagnostics. Phase 1 uses ignored red sessions so
   the normal gate stays green; later phases remove the ignores as behavior
   lands. Corpus generation is unrelated to semantic routing and needs no
   feature-specific branch.

## Phase 1 red-test policy

The scaffolds are executable but ignored by default with a phase-specific
reason. The narrow checkpoint runs them with `--run-ignored all` and must fail
at the first missing `suggest(...)` behavior. This preserves a passing
package-area `just test` while retaining executable acceptance assertions for
subsequent phases.
