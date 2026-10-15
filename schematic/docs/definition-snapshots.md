---
kind: canonical-documentation
---

# Serialize and compare REST definitions

REST definitions can be copied through Serde without changing their endpoint order, request shapes, authentication policy, or pagination controls. `RestApi`, `Endpoint`, `EndpointParams`, `ParamDef`, `QueryParamType`, and `ParamStyle` implement `Serialize` and `Deserialize`, as do their nested policy and pagination values. This is a data-transfer format; deserialization does not replace the generator's semantic validation and does not bundle referenced Rust types or schemas.

```rust
let json = serde_json::to_string(&api)?;
let restored: schematic_define::RestApi = serde_json::from_str(&json)?;
assert_eq!(api, restored);
```

Required fields keep their native JSON types. Empty collections remain empty; missing collections are errors. Optional fields intentionally define both omission and explicit `null` as `None`. For example, `"params": null` means no parameter declaration, whereas `"params": {"query": [], "header": [], "cookie": []}` means a present, empty declaration. Booleans must be native (`true`), not quoted (`"true"`). Duplicate typed fields and trailing garbage are errors. Plain derives retain Serde's usual handling of unknown fields; this format is not a strict configuration schema.

## Migration checks

From `schematic/`, run `just migration-check`. It compares each shipped API to its historical JSON fixture, resolves every endpoint's pagination and caller controls, and regenerates Rust, OpenAPI, and Postman into temporary directories. It never updates fixtures or committed output. Rust comparisons cover the complete source file set and generated manifest; all artifact bytes must match. Description changes, removed files, reordered endpoints, changed schema content, and registry ordering changes fail the check.

Historical fixtures live under `gen/tests/fixtures/definition-baseline/`. Object keys are recursively sorted, while endpoint arrays, pagination arrays, and registry entries retain insertion order. Readable definitions and resolved pagination accompany BLAKE3 fingerprints of API metadata (all API fields except endpoints), each endpoint, and each named registry schema. Fingerprints use `biscuit-hash` on compact canonical JSON and include complete schema content. Historical JSON is compared as values before any deserialization into the current model, so later model changes cannot silently supply defaults or drop old fields.

The raw corpus measured 8,449,975 bytes. Schema fingerprints reduce the stored corpus to 1,270,391 bytes. Each registry entry keeps its name and position so a failure can identify the affected registration. The source-maintained `EXPLAINED` list requires an exact changed leaf and a nonempty cause; wildcard and stale entries fail. Paths include API keys, array positions, endpoint IDs and registry names. There are no accepted differences initially; a migration must explain any intentional difference for its exact API, endpoint and field, or registry entry. Artifact comparisons always use the current shipped outputs with no exemptions.

The explicit capture utility is `cargo run -p schematic-gen --example capture_definition_baseline -- <new-directory>` from the repository root. The destination must be a new directory with an existing parent; existing destinations are refused. Use a new temporary directory to inspect a candidate capture. Replacing historical fixtures is not part of ordinary migration checks.

The retired API exception table, endpoint OAuth scope field, and parameter-level
pagination fields are absent from current snapshots. Endpoint `pagination` is
an optional collection: omission and null mean no assignment; malformed shapes,
duplicate fields, and trailing content reject.

## JSON field outcomes

These tables define the data-transfer contract for the newly serializable structs. `N` means an optional field restores as `None`; `E` means an error. `V` means a native value is retained, including an empty string, array or object only where its declared type accepts that shape. Object-valued enum payloads still require their declared fields. A duplicate field or trailing garbage always fails typed deserialization. The tests edit shipped GitHub definitions one field at a time, with an unchanged positive control; every shipped API also undergoes two full serialization round trips with equal downstream pagination snapshots.

### RestApi

| Shape | `name` | `description` | `base_url` | `docs_url` | `auth` |
| --- | --- | --- | --- | --- | --- |
| native control | V | V | V | V | V |
| absent | E | E | E | N | E |
| null | E | E | E | N | E |
| wrong whole type | E | E | E | E | E |
| one wrong element | — | — | — | — | — |
| every element wrong | — | — | — | — | — |
| empty native | V | V | V | V | E |
| duplicate key | E | E | E | E | E |
| trailing content | E | E | E | E | E |

### RestApi (continued)

| Shape | `auth_policy` | `env_auth` | `env_username` | `headers` | `endpoints` |
| --- | --- | --- | --- | --- | --- |
| native control | V | V | V | V | V |
| absent | N | E | N | E | E |
| null | N | E | N | E | E |
| wrong whole type | E | E | E | E | E |
| one wrong element | — | E | — | E | E |
| every element wrong | — | E | — | E | E |
| empty native | V | V | V | V | V |
| duplicate key | E | E | E | E | E |
| trailing content | E | E | E | E | E |

### RestApi (continued)

| Shape | `module_path` | `request_suffix` | `version` | `env_mapping` | `cross_origin` |
| --- | --- | --- | --- | --- | --- |
| native control | V | V | V | V | V |
| absent | N | N | N | N | E |
| null | N | N | N | N | E |
| wrong whole type | E | E | E | E | E |
| one wrong element | — | — | — | — | — |
| every element wrong | — | — | — | — | — |
| empty native | V | V | V | V | E |
| duplicate key | E | E | E | E | E |
| trailing content | E | E | E | E | E |

### RestApi (continued)

| Shape | `request_id_header` | `secret_names` | `default_pagination` |
| --- | --- | --- | --- |
| native control | V | V | V |
| absent | N | E | N |
| null | N | E | N |
| wrong whole type | E | E | E |
| one wrong element | — | E | — |
| every element wrong | — | E | — |
| empty native | V | V | E |
| duplicate key | E | E | E |
| trailing content | E | E | E |

### Endpoint

| Shape | `id` | `method` | `path` | `description` | `request` |
| --- | --- | --- | --- | --- | --- |
| native control | V | V | V | V | V |
| absent | E | E | E | E | N |
| null | E | E | E | E | N |
| wrong whole type | E | E | E | E | E |
| one wrong element | — | — | — | — | — |
| every element wrong | — | — | — | — | — |
| empty native | V | E | V | V | E |
| duplicate key | E | E | E | E | E |
| trailing content | E | E | E | E | E |

### Endpoint (continued)

| Shape | `response` | `headers` | `params` |
| --- | --- | --- | --- |
| native control | V | V | V |
| absent | E | E | N |
| null | E | E | N |
| wrong whole type | E | E | E |
| one wrong element | — | E | — |
| every element wrong | — | E | — |
| empty native | E | V | E |
| duplicate key | E | E | E |
| trailing content | E | E | E |

### EndpointParams

| Shape | `query` | `header` | `cookie` |
| --- | --- | --- | --- |
| native control | V | V | V |
| absent | E | E | E |
| null | E | E | E |
| wrong whole type | E | E | E |
| one wrong element | E | E | E |
| every element wrong | E | E | E |
| empty native | V | V | V |
| duplicate key | E | E | E |
| trailing content | E | E | E |

### ParamDef

| Shape | `name` | `required` | `description` | `param_type` | `explode` |
| --- | --- | --- | --- | --- | --- |
| native control | V | V | V | V | V |
| absent | E | E | N | E | E |
| null | E | E | N | E | E |
| wrong whole type | E | E | E | E | E |
| one wrong element | — | — | — | — | — |
| every element wrong | — | — | — | — | — |
| empty native | V | — | V | E | — |
| duplicate key | E | E | E | E | E |
| trailing content | E | E | E | E | E |

### ParamDef (continued)

| Shape | `style` |
| --- | --- |
| native control | V |
| absent | E |
| null | E |
| wrong whole type | E |
| one wrong element | — |
| every element wrong | — |
| empty native | E |
| duplicate key | E |
| trailing content | E |
