# Definition authoring reference

Use `schematic-define`'s typed builders for REST authoring. Current behavior:
`schematic/docs/definition-builders.md` (accepted forms, the `Default`
allow-list, optional vocabulary); type registration and ordered module
assembly: `schematic/docs/typed-registration.md`. Responses, errors, streams,
and GraphQL: [`responses.md`](./responses.md). Pagination:
[`pagination.md`](./pagination.md).

## Required-fact builders

- `Endpoint::{get,post,put,patch,delete,head,options}(id, path)` or
  `Endpoint::new(id, method, path)`, then `describes` and at least one
  success declaration before fallible `build`. JSON methods take
  `&mut DefinitionContext`; build endpoints before borrowing that context with
  `RestApi::builder(&ctx, name, base_url)`.
- An API needs a description and exactly one explicit auth choice
  (`no_auth`, `bearer_env([..])`, `basic_env(user, [..])`, or
  `authentication(strategy, env)`). Repeated auth choices fail; explicit env
  mappings must agree regardless of setter order. Explicit `auth_policy`
  remains the effective multi-method policy.
- `Endpoint` and `RestApi` are `#[non_exhaustive]` with no `Default`;
  external literals do not compile. `EndpointParams::new()` is a valid empty
  set. GET request bodies fail. Paths cannot contain `?` (endpoint/API build,
  shared validators, and import reject it with API, endpoint, and path
  context); route identity is method + path, so query/header/cookie/body
  inputs never distinguish two endpoints.
- New defaulted (`skip_serializing_if`) members must join `OMITTED_DEFAULTS`
  in `define/src/bundle.rs`.
- Value constraints (`define/src/constraints/`): the fields are
  `Option<Box<Constraints>>` (an inline one overflowed the debug stack of the
  imported OpenAI builders); `Constraints` is
  `#[non_exhaustive]` with `Default` (allow-listed); every `Option` member uses
  `serde_presence::non_null` so `null` is an error, not absent, except
  `allowed_values`/`constant` data, which the bundle's `reject_nulls` and
  `omit_nulls` exempt (`is_constraint_data`). Adding a `ParamDef`,
  `FormField`, or `Endpoint` field breaks every struct literal in define,
  gen, and the generated `definitions/src/openai/mod.rs` (emitted by
  `gen/src/definition_gen.rs` `param_def_tokens`); `import_overlay` pins
  two deliberate re-import differences, so patch that file to the
  generator's form rather than adopting the whole re-import.

## Typed registration

- One `DefinitionContext` per module group; explicitly register nominal
  models in component order. Shared API variants use the same context.
- Implement `Registration` beside nominal models with `definition_type!` (or
  the model module's nominal registration macro); there is no blanket
  `JsonSchema` dispatcher. Register aliases first with
  `register_as::<T>(rust_path!(PublicAlias).schema_name("Component"))`; the
  first canonical registration fixes name/path/order.
- Containers recurse into leaves; primitives and free-form JSON stay inline.
  Nominal generic registrations keep concrete public type arguments (never
  stringify a generic `T` into a Rust path).
- REST JSON bodies and shapes store a `TypeRef` (`Named`, `Array`,
  `Optional`, `Map`, `KeyedMap { key, value }`, `Primitive(..)` incl.
  `Primitive(Json)`), never a Rust path. API build stores the context's
  components in `RestApi::schemas` (`SchemaComponent { name, rust_path,
  schema }`, nested `$defs` flattened with `rust_path: None`) and runs
  `validate_references` (`DefinitionError::UnresolvedReference`) and
  `validate_schema_closure` (`DefinitionError::UnresolvedSchemaReferences`:
  every component `$ref` that names no `#/components/schemas/{name}`, with
  parent, JSON pointer, and missing name; external refs are never fetched;
  OpenAPI import skips it). A name
  reached with a different wire schema fails as `ComponentConflict`.
  `api.rust_type(&reference)` spells the Rust type. `Schema { type_name,
  module_path }` is WebSocket-only.
- Polymorphic bodies: `ctx.one_of::<T>([..])` / `any_of` / `all_of`.
- `schematic_definitions::registry::assemble_module_group` returns
  `Result<Vec<RestApi>, DefinitionError>`; every member carries identical
  `schemas`. `just doctest` runs the registration compile-fail examples.

## Auth and environment

`AuthStrategy` (`#[non_exhaustive]`): `BearerToken { header }`, `ApiKey {
header }`, `ApiKeyParam { location: ApiKeyLocation::{Query, Cookie}, name }`,
`Basic { username_env, password_env }`, `OAuth2(OAuth2Config)` (one variant
per grant: `AuthorizationCodePkce`, `ClientCredentials`, `DeviceCode`; API
build runs `OAuth2Config::validate`), `None`.

- `EnvMapping` fields: `bearer_token`, `basic_user`, `basic_pass`, `api_key`.
  No legacy `basic_auth` object.
- Basic reads every name in `username_env`/`password_env` in order (or the
  mapping's `basic_user`/`basic_pass`); `env_auth` is never a Basic source.
  `env_username(name)` fills or must equal Basic's `username_env`.
- Generated `definition()` translates the policy into runtime `AuthConfig`.
  The runtime resolves one credential per send: call `.credential(..)` →
  client (`api_key`, `basic_auth`, `credentials`) → env (`env_order([..])`
  replaces the order) → anonymous. `api_key(secret)` on a Basic-only or
  no-auth API is `BuildError::IncompatibleCredential`. A credential never
  overrides a same-named request query/cookie/header value.
- Endpoint overrides (at most one): `.public()`, `.auth(policy)`,
  `.auth_with_env_mapping(policy, mapping)`; resolve with
  `RestApi::endpoint_auth`.

## API-level policy and optional vocabulary

- `cross_origin: CrossOrigin` (exhaustive on purpose): `same_origin()`
  (default), `allow_origins([..])`, `allow_without_credentials()`,
  `danger_allow_any()`, `deny()`; callers may only tighten it.
- `request_id_header(name)` and `secret_header(name)`;
  `definitions/tests/api_policy.rs` pins all three policies for every
  shipped API, so a new definition adds its row there.
- `accept_unchecked_compositions()` sets `RestApi::unchecked_compositions`
  (`#[serde(default)]` plain `bool`, omitted when false, listed in
  `OMITTED_DEFAULTS`; `definition_gen` emits the call when set). It turns a
  request rule whose only blockers are generic compositions into a
  `type-constraint` warning instead of an error; positional items, other
  untraversed keywords, unresolved refs, and `constraint_problems` stay
  errors. `unenforceable_rules` sets `UnenforceableRule::composition` only
  when compositions are the sole blocker. OpenAI sets it in
  `definitions/src/openai_pagination.rs::apply`, never in generated code.
- `.path_param(ParamDef::path(name, type))`: once any is declared, names
  must equal the placeholders (`{+x}` counts as `x`).
- `.deprecated()` (endpoint and `ParamDef`), `.rate_limit(..)`,
  `.retry_guidance(..)` (endpoint value replaces the API's whole; read with
  `effective_rate_limit`/`effective_retry_guidance`). Descriptive only: the
  runtime never reads rate-limit/retry metadata and no `#[deprecated]` is
  generated.
- Query fidelity: a required non-adopted query parameter is a `T` field and a
  `new()`/endpoint-method argument after the path parameters; enums become
  `{Request}{FieldPascal}`; arrays follow `style`/`explode`. `ParamDef::new`
  is form with `explode: false`. An unsendable query shape is a
  `query-parameter` warning (optional) or error (required).

## Snapshots and bundles

- REST definitions derive plain Serde. Call `validate_references` and
  `validate_schema_closure` on a deserialized API before use. `just -f schematic/justfile migration-check`
  compares historical JSON; see `schematic/docs/definition-snapshots.md`.
- `DefinitionBundle::new(api)` (optionally `.with_models(catalog)?`): `to_json()`
  writes canonical JSON, `from_json` loads strictly (duplicate keys, trailing
  content, stray `null`, dropped members rejected) and reruns builder
  validation and reference checks, including error-entry, stream-item, and
  operation references. CLI: `schematic-gen bundle --api NAME --out FILE`,
  `schematic-gen export-bundle --input FILE --openapi-out DIR --postman-out
  DIR`. See `schematic/docs/io/definition-bundles.md`.

## Authoring guidance maintenance

`schematic/docs/authoring-examples.md` holds the compiling example included in
schematic-define rustdoc; the `create-schematic-api` command teaches the same
assembly. The L1 `authoring_guidance` corpus rejects obsolete REST
constructor examples.
