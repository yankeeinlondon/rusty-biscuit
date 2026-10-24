# Stable registration design proof

This isolated workspace settles the Phase 1 registration dispatch and naming risks. It is a throwaway prototype, not a production implementation or a second definition registry. Run it from the repository root:

```sh
python3 schematic/features/2026-10-14-schematic-definition-api/spike/prove.py
cargo clippy --manifest-path schematic/features/2026-10-14-schematic-definition-api/spike/Cargo.toml --lib --bins -- -D warnings
```

Both commands passed on macOS with stable rustc 1.98.1. The lockfile records the isolated dependency resolution. No terminal or browser is opened; neither command runs `cargo fmt` or `cargo test`.

## Requirement-to-test map

These are new authoring contracts rather than regressions in shipped behavior. The initial design mapped each behavior to the witnesses below; the runtime tests were authored alongside the isolated implementation. The prototype does not change a shipped parser or configuration format, so the production input-robustness matrix and shipped-artifact corpus belong to Phase 2's descriptor validation and Phase 1's separate baseline work.

| Contract and boundary | Concrete witness | Fidelity / tier |
| --- | --- | --- |
| Canonical registration, schema rename versus Rust spelling, alias/reexport, foreign and concrete generic paths; schemars boundary | `nominal_renamed_alias_reexport_foreign_and_generic_paths` | F2/L1 for schemars; F1/L1 for context logic |
| Vec/Option/HashMap nesting, nullable wire schema, alias reuse, insertion order, no wrapper components, repeated serialization round trip | `wrappers_inline_shape_nullability_and_order_reuse_leaf` | F1/L1; real serde F2/L1 |
| Primitive and Value inline schemas, String and integer map keys, rejected bool key with API/endpoint/type context | `primitives_value_and_integer_map_keys_create_no_components` | F1/L1 |
| Two distinct conflicting types produce both ordered collisions without replacing the first schema | `collision_aggregation_is_ordered_and_never_merges_distinct_types` | F1/L1 |
| Missing destination registration, matching destination, forged schema name/path, forged inline schema, reserved structural payload | `destination_checks_named_paths_wrappers_inline_and_reserved_structure` | F1/L1 through public `validate` |
| Real external crate resolves every actual emitted nominal/alias/reexport/nested/foreign/generic Rust path | `prove.py` creates a temporary consumer from `emitted_path_source` output and runs `cargo check` | F2/L1, Rust compiler boundary |
| `register_as::<Model>(rust_path!(models::Renamed))` fails type checking | `examples/mismatched_type.rs`, compiler diagnostic checked by `prove.py` | F2/L1 |
| `rust_path!(models::DoesNotExist)` fails resolution | `examples/misspelled_path.rs`, compiler diagnostic checked by `prove.py` | F2/L1 |
| External descriptor construction cannot access either private field, including `structure` | `examples/private_descriptor.rs`, E0451 checked by `prove.py` | F2/L1 |

All five unit tests are compiled by the standalone library target, have ordinary L1 names, and are explicitly selected with `cargo nextest run --lib`. The three invalid examples have `required-features = ["negative-cases"]`; the proof runner explicitly enables that feature and demands the expected nonzero exit and diagnostic. They are compiler witnesses, not unselected passing tests. Each runtime test took under 0.01 s in the measured proof. The external consumer builds once outside the unit-test suite.

## Chosen signatures and bounds

```rust
DefinitionContext::types_in(module_path: &str) -> DefinitionContext
DefinitionContext::register<T: Registration>(&mut self) -> Result<TypeDesc, DefinitionError>
DefinitionContext::register_as<T: Registration>(&mut self, path: RustPath<T>)
    -> Result<TypeDesc, DefinitionError>
DefinitionContext::validate(&self, descriptor: &TypeDesc) -> Result<(), DefinitionError>
DefinitionContext::finish(&self) -> Result<(), &[DefinitionError]>
RustPath<T>::schema_name(self, name: &'static str) -> RustPath<T>
```

`Registration: JsonSchema + 'static` has one implementation emitted alongside each nominal definition by a declarative helper or the import emitter. That implementation supplies `RUST_IDENTIFIER = stringify!(Identifier)`, independently of the schema name. The helper wraps the declaration; it introduces no second manually maintained registration list. Ordinary paths concatenate the context's single module declaration with this proven identifier. Production should use a hidden helper implementation macro supporting the actual struct/enum declarations; the prototype deliberately covers a struct declaration only.

There is no blanket `impl<T: JsonSchema> Registration for T`. Containers have disjoint generic implementations over `T: Registration`, and primitives/Value have concrete implementations. Nominal wrappers remain nominal because their declaration receives its own implementation, even if their schema resembles a collection. Schemars itself does not supply a stable public Rust identifier or a nominal-versus-container discriminator, so schema inspection cannot replace this dispatch contract.

`register_as` uses the same bound and checks `IS_LEAF`; Vec/Option/HashMap are rejected. `RustPath<T>` carries the exact type argument required by the `register_as::<T>` signature. The macro mentions the actual type in both its token type and typed witness; unresolved and mismatched paths fail at compile time. Concrete generic paths are supported, as proved by `::std::ops::Range<u64>`. Paths containing out-of-scope type parameters are not promised. `stringify!` supplies spelling; `type_name` is used exclusively in diagnostics. Public reexports and aliases retain their authored path even when Rust erases alias identity.

The macro uses a hidden public token constructor because exported declarative macros expand in their caller's crate. This constructor does not construct a `TypeDesc`. As with the nominal trait contract, advanced callers can deliberately provide misleading token text; the generated-crate compiler check remains the final witness that authored emitted paths resolve. The phase does not promise a security boundary against deliberately dishonest implementations of authoring traits.

### Foreign-type ownership

Rust's orphan rule prevents a provider crate from implementing `schematic-define::Registration` for an arbitrary foreign type. The foreign-leaf implementation must be supplied in the crate defining `Registration`. The prototype proves this using provider-neutral standard-library `Duration` and concrete `Range<u64>` adapters. Production should add an adapter there for each foreign leaf actually found in the inventory. This is a documented additional bound on foreign types, not blanket support for every `JsonSchema` foreign type. It needs no second per-provider registry and introduces no vendor preset. A foreign type without an adapter fails the trait bound rather than guessing its Rust spelling. Local aliases of an adapted foreign type reuse its identity and may explicitly select a public reexport with `register_as`.

## Descriptor and registry storage

`TypeDesc` derives serde and keeps both fields private. Its recursive private `Shape` records named leaves, native inline leaves, Vec, Option, and Map; it stores all wrapper information independently of `structure`. Production uses the specified `Option<TypeRef>` for the reserved carrier; the prototype substitutes `Option<Value>` only to avoid coupling its isolated workspace to unfinished production types. Authored descriptors always set it to None; validation rejects Some. This field is neither a schema source nor the container dispatcher.

An ordered vector stores each canonical `TypeId` once with its schema name, emitted Rust path, diagnostic type name, and OpenAPI-compatible schemars root schema. First registration determines the canonical spelling and optional historical schema-name override; later registration of the same canonical type returns that entry without moving it. Explicit exceptional registration should therefore precede container/ordinary discovery of that type. Distinct types claiming a name accumulate structured `Collision { schema_name, first, second }` errors; `finish` rejects assembly with all errors. Production renders these using the specification's API-context collision message. Production also retains existing nested-definition extraction in the definitions-crate adapter; this prototype proves the dispatch, not a replacement schema exporter.

Destination validation walks every wrapper and checks each named leaf's schema name and Rust path together. A descriptor from another context may pass only if the destination has an exact matching registration. Inline leaves must match a known native schema/path pair. The reserved structural payload cannot bypass validation. Production must extend this with the complete supported primitive set, every Serde-supported integer key, explicit string-newtype map-key contracts, and full malformed-descriptor matrix from Phase 2. The prototype covers String/i64/u64 positive map keys and bool rejection; a rejected key does not register its value as a side effect. The API/endpoint labels in the synthetic rejected-key test are the fixture's `Spike`/`Map`; production obtains those labels from authoring context.

## Recorded outcomes and limits

The complete proof passed: five nextest tests, a real isolated consumer compiling six emitted paths, and three required compiler failures. Clippy passed for the library and emitted-source binary with warnings denied. One initial expectation guessed `Range_uint64`; schemars actually supplied `Range_of_uint64`, and the assertion was corrected to preserve the real schema name. No production code or shipped definition was changed. No cross-OS behavior is introduced: the proof uses portable Rust, Python subprocesses, and temporary directories. The workspace remains isolated and is not added to monorepo CI or package-area tier recipes.
