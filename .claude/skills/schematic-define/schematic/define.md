# Definition authoring reference

Use `schematic-define`'s typed builders for REST authoring. The current reference
is `schematic/docs/definition-builders.md`; type registration and ordered module
assembly are in `schematic/docs/typed-registration.md`. Read those pages before
changing definition factories, schema components, or imported module emitters.

- Create one `DefinitionContext` per module group, then explicitly register
  nominal models in component order. Shared API variants use the same context.
- Implement `Registration` beside nominal models with `definition_type!`, or
  the existing nominal registration macro used in the model module. There is
  no blanket `JsonSchema` registration. Register aliases before canonical uses
  with `register_as::<T>(rust_path!(PublicAlias).schema_name("Component"))`.
- Containers recursively register their leaves; primitives and free-form JSON
  remain inline. API build stores the context's components in
  `RestApi::schemas` and checks that every reference resolves there.
- Build endpoints with description and one success shape. JSON requests and
  successes take types and the mutable context. GET request bodies fail.
- Build APIs with description and one explicit authentication choice. Environment
  declarations must agree with explicit mappings regardless of setter order.
  Explicit `auth_policy` remains the effective multi-method policy.
- `Endpoint` and `RestApi` are non-exhaustive with no default. External callers
  cannot use literals. `EndpointParams::new()` is a valid empty parameter set.
- Query/header/cookie locations use `ParamDef`. Route identity is method/path;
  `?` in paths fails shared validation and build. Use optional query inputs to
  combine same-route variants. `rust_name` changes only Rust spelling.
- Pagination is API default plus endpoint `Collection`. Keep strategy helpers
  provider-local; validate parameter adoption and caller controls. The retired
  pagination enums, API exception tables, and parameter pagination fields are gone.
- REST JSON bodies store a `TypeRef` naming components plus wrapper shape, with
  no Rust path or inline schema copy. `RestApi::rust_type` spells it through
  `RestApi::schemas`; `RestApi::validate_references` rejects dangling names
  (`DefinitionError::UnresolvedReference`) and malformed or duplicate components,
  for built and deserialized APIs alike. `Schema` is WebSocket-only.
- The unused endpoint OAuth scopes field is removed. OAuth authentication policy
  and WebSocket definition constructors retain their established behavior.

`assemble_module_group` returns `Result<Vec<RestApi>, DefinitionError>` and gives
every member the finished context's components, so group members carry identical
`schemas`. Components preserve explicit registration order, with nested-only
types flattened in (`rust_path: None`, not referenceable). Keep unused explicit
components unless a documented container normalization removes them. Generator
and exporter entry points read `api.schemas`; there is no separate registry.

See `schematic/docs/definition-snapshots.md` for transfer contracts,
`schematic/docs/headers/README.md` for WebSocket header helpers, and
`schematic/docs/io/openapi-extensions.md` for OpenAPI wire extensions.
