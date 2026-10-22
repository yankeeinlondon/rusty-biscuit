---
kind: canonical-documentation
---

# Typed type registration

Use `schematic_define::DefinitionContext` to collect the types for one generated
module. Registration returns a `TypeDesc`: the `TypeRef` an endpoint stores
(component names plus wrapper shape) and its Rust spelling. Every shipped REST
provider and imported module uses one typed module-group assembly, and every
API it builds carries the context's schema components in `RestApi::schemas`.

```rust
use schematic_define::{definition_type, DefinitionContext, TypeRef};
use schemars::JsonSchema;

definition_type! {
    #[derive(JsonSchema)]
    pub struct Model { pub id: String }
}

let mut context = DefinitionContext::types_in("::my_definitions::models");
let response = context.register::<Option<Vec<Model>>>()?;
// Stored reference: Optional(Array(Named("Model")))
assert_eq!(response.type_ref(), &TypeRef::Optional(Box::new(
    TypeRef::Array(Box::new(TypeRef::Named("Model".into()))))));
assert_eq!(response.rust_path(),
    "::std::option::Option<::std::vec::Vec<::my_definitions::models::Model>>");
context.finish()?;
```

`definition_type!` wraps a named, tuple, or unit struct or an enum declaration and emits its
`Registration` implementation beside it. Its Rust identifier comes from the
declaration; its component name comes from `JsonSchema`, including Serde or
Schemars renaming. There is no blanket implementation for every `JsonSchema`
type. Nominal types containing collections still produce nominal components.

The module path must name a public module accessible from the generated crate.
Compilation of the generated crate proves that visibility. `type_name` is used
only for diagnostics, never to derive emitted Rust paths.

For aliases and public re-exports, register the exceptional spelling first:

```rust
use schematic_define::{definition_type, DefinitionContext, rust_path};
use schemars::JsonSchema;
definition_type!(#[derive(JsonSchema)] pub struct CommitInfo { pub id: String });
pub type Commit = CommitInfo;
let mut context = DefinitionContext::types_in("::my_definitions::models");
let commit = context.register_as::<CommitInfo>(rust_path!(Commit)
    .schema_name("Commit"))?;
```

`rust_path!` requires a resolvable Rust type path and proves it names the type
passed to `register_as`. Concrete generic paths are supported. Authors must
choose paths accessible from the generated crate; a local import can compile
at registration time and still fail there. The hidden token constructor and
custom registration traits are authoring contracts, not protection against
deliberately dishonest trait implementations.

The first registration of a canonical Rust type fixes its component name, path,
and insertion position. Registering an alias later reuses those choices.
Distinct types with the same component name fail registration; `finish()`
returns every collected collision in encounter order. A collision never replaces
an earlier schema. Use `error.diagnostics(api_name)` for one contextual line per
collision. `register_as` accepts nominal leaves; it rejects containers,
primitives, and free-form JSON. Foreign types need an implementation owned by
`schematic-define`; currently `Duration` and `Range<u64>` are adapted.

`Vec`, `Option`, and `HashMap` recursively reuse leaf registrations. They do not
create components. Primitive types, strings, unit, and `serde_json::Value` are
inline. Options preserve Rust wrappers and wire nullability. Nullable schemas
use the same OpenAPI 3.0 transforms as nominal schema generation. Maps accept
String and all Serde-supported signed/unsigned integer widths. A string-like
newtype can implement `MapKey` with an explicit public Rust path when its Serde
representation is a string. Boolean and floating-point keys fail before the
value type is registered. Use `register_for::<T>(api, endpoint)` for contextual
registration errors.

## From registration to references

`DefinitionContext::components()` turns the registrations into the list an API
stores as `RestApi::schemas`. Each `SchemaComponent` has a `name` (the
`#/components/schemas/{name}` target), a `rust_path`, and a `schema` with its
nested definitions removed. Types that appear only nested inside a registered
type (`$defs`) become components of their own with `rust_path: None`; they are
exported to OpenAPI but cannot be referenced by an endpoint. Components follow
registration order, with a nested-only type placed after the first type that
contains it; registering that type later takes over its entry in place.
Explicit registrations survive even when no endpoint uses them.

One component name holds one wire schema. When a name arrives a second time,
as another nested copy or as an explicit registration, its schema must equal
the one already stored, or `components()` (and therefore API build and module
group assembly) fails with `DefinitionError::Collisions` listing one
`ComponentConflict { schema_name, first, second }` per name. Each source is a
registered Rust type or "a schema nested in" one:

```rust,ignore
mod left  { pub struct Status { pub value: String } pub struct Left  { pub status: Status } }
mod right { pub struct Status { pub value: bool }   pub struct Right { pub status: Status } }

ctx.register::<left::Left>()?;
ctx.register::<right::Right>()?;
ctx.components() // Err: schema component "Status" has different definitions
                 // from a schema nested in …::left::Left and a schema nested in …::right::Right
```

The same type reached twice is not a conflict. The comparison ignores the
top-level `title`, and it applies the OpenAPI 3.0 transforms (such as
`nullable`) to nested copies, because schemars applies them only to a root
schema. It also reads a recursive type's `"$ref": "#"` as a reference to the
registration it was found under.

Endpoints never store a Rust path. They store a `TypeRef`, and every reader
resolves its names through the API's `schemas` (Rust spellings abbreviated):

| Stored reference | Rust spelling (`RestApi::rust_type`) | OpenAPI wire schema |
| --- | --- | --- |
| `Named("Model")` | `::my_definitions::models::Model` | `$ref: '#/components/schemas/Model'` |
| `Array(Named("Model"))` | `::std::vec::Vec<…::Model>` | array of that `$ref` |
| `Optional(Named("Model"))` | `::std::option::Option<…::Model>` | nullable `$ref` |
| `Map(Named("Model"))` | `::std::collections::HashMap<::std::string::String, …::Model>` | object with `additionalProperties` |
| `KeyedMap { key: "u32", value: Named("Model") }` | `::std::collections::HashMap<u32, …::Model>` | same as `Map` |
| `Primitive(U32)`, `Primitive(Json)` | `u32`, `::serde_json::Value` | inline, no component |

`KeyedMap` holds the one Rust spelling a reference ever carries, because a map
key type has no component of its own. The primitive widths cover `i8`–`i128`,
`u8`–`u128`, `isize`, `usize`, `f32`, `f64`, `bool`, `char`, `String`, `()`,
bytes (`Vec<u8>`), and free-form `serde_json::Value` (`Json`).

```mermaid
flowchart LR
    D[Model declarations] --> C[One module-group context]
    C --> F[Collision check]
    C --> T["TypeDesc → TypeRef stored on endpoints"]
    F --> S["RestApi::schemas (components)"]
    T -->|names resolve through| S
    S --> R[Rust paths in the generated client]
    S --> O["OpenAPI components.schemas and $ref"]
```

`schematic_definitions::registry::assemble_module_group` calls an author-supplied
assembly function with one context and returns `Result<Vec<RestApi>,
DefinitionError>`. There is no separate API inventory. Because a module group's
APIs are built one at a time, an early member's build sees only the types
registered so far; assembly therefore gives every member the finished context's
components and re-runs `validate_references`, so all members carry identical
`schemas`. Collisions are checked first, even if the callback swallowed one.
Shared API variants use one context; isolated modules may reuse component names.

## Polymorphic bodies: `oneOf`, `anyOf`, `allOf`

A hand-written serde type can be bound to a structural composition of types
registered in the same context. The structure you state is what OpenAPI
receives, and the Rust generator references your type:

```rust,ignore
let cat = ctx.register::<Cat>()?;
let dog = ctx.register::<Dog>()?;
let pet = ctx.one_of::<Pet>([cat, dog])?; // also any_of, all_of
// api.schemas: Pet = {"oneOf":[{"$ref":"#/components/schemas/Cat"},
//                              {"$ref":"#/components/schemas/Dog"}]}
Endpoint::post("Adopt", "/pets").request_json::<Pet>(&mut ctx) // references Pet
```

- `oneOf` means exactly one member matches, `anyOf` at least one, `allOf` every
  member. Matching the Rust serde representation to the declared structure is
  the author's job: an untagged enum does not by itself prove `oneOf`
  exclusivity, so give the variants distinguishable shapes. Use a struct that
  satisfies every member for `allOf`.
- The composition replaces schemars' own schema for the bound type in
  `components()`. The returned `TypeDesc` names the bound type, and
  `TypeDesc::structure()` returns the composition; a structure can only be
  attached through these methods.
- Registration fails, and the error is also collected for `finish()`, for an
  empty member list, a duplicate member, a primitive or container member
  (`NonNominalMember`), or a member this context did not register
  (`ForeignMember`). A member must match this context's entry by name, Rust path,
  and structure; a descriptor carries no context identity, so an identical
  descriptor from another context is indistinguishable. A bound type that was
  registered earlier without this composition, or with a different one
  (including reordered members), is a `ComponentConflict`. Repeating the same
  composition, or a later plain `register::<Pet>()`, returns the composed
  descriptor.
- `DefinitionContext::models()` lists every composition as a
  `ModelDef::Alias` in registration order. The model catalog types serialize
  with serde; unknown members and explicit `null` for optional fields are
  rejected.
- Postman has no schema-composition declaration; exporting a composition there
  reports an unsupported-feature diagnostic instead of inventing an example
  (**planned**).

## References in transferred definitions

`TypeDesc` is an authoring handle only: it has no Serde derives and no public
constructor. A `RestApi` does serialize, and its references and components
travel with it. Treat a deserialized definition as untrusted and call
`RestApi::validate_references` before using it. It is the same check API build
runs:

- every name in every JSON body, response, and form-part reference resolves to
  a component with a Rust path, or the error is
  `DefinitionError::UnresolvedReference { api, endpoint, name }`
- component names are nonempty and unique, Rust paths are nonempty, and each
  schema is a JSON Schema object or boolean
- compositions, `Unknown`, and empty map keys are rejected

```json
{
  "name": "Fixture",
  "endpoints": [{ "id": "Get", "response": { "Json": { "Array": { "Named": "Model" } } } }],
  "schemas": [{ "name": "Model", "rust_path": "::models::Model", "schema": { "type": "object" } }]
}
```

Renaming the component to `Renamed` in this excerpt fails with
`Fixture endpoint Get: unresolved schema reference "Model"`; changing its
`rust_path` to `::other::Model` changes the generated Rust spelling and nothing
else. Components reject unknown fields, a missing `name` or `schema`, wrong
types, duplicate keys, and trailing content while reading JSON. An absent or
null `rust_path` reads as a nested-only component, which a reference cannot
name. The `builders` integration target covers this matrix; shipped definition
snapshots and artifact drift tests exercise the catalog through normal
generation.

[Definition builders](definition-builders.md) describe how endpoint builders
lower a registration to its reference and how API build stores the components.
WebSocket messages keep their separate `Schema { type_name, module_path }`
representation.

A nominal generic model needs a concrete Rust path for each instantiation it
registers. For example, Bitbucket registers `PaginatedResponse<Repository>` with
both the wrapper and `Repository` qualified through their public module. Its
registration implementations live beside `PaginatedResponse`; a context can
register one directly without first assembling the provider. Writing the
literal path `PaginatedResponse<T>` inside a generic implementation would leave
an unbound `T` in generated client code.

[The compiling authoring example](authoring-examples.md) combines ordinary nominal
registration, alias-first registration, nested containers, and complete API assembly.
