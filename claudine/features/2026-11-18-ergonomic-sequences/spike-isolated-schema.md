# Spike 1: Cross-Stack `isolated` Constraint, Sticky-Union Plumbing & Schema Bundles

- **Feature:** `2026-11-18-ergonomic-sequences`
- **Status:** Complete (design-grounding verification spike — no production code or shipped schemas modified)
- **Scope:** `simplified-schema/lib`, `darkmatter/lib`, `darkmatter/schemas`, `claudine/lib`, `claudine/registration`, `scripts/shipped_schemas.py`

---

## Executive Summary & Critical Findings

This spike verified every layer of the `isolated` schema constraint (`"x-darkmatter-isolated": true`), Darkmatter's `EffectiveSchema` sticky-union assembly, all four Darkmatter child-inheritance call sites and compose-cache fingerprinting, and the full key-by-key rollout across `fm.yaml` and `claudine/registration/*.yaml`.

Six non-obvious architectural traps were uncovered and resolved below before implementation begins:

1. **Host-trigger merge conflict between `composition.yaml` and `sequence-document.yaml` (`darkmatter/lib/src/config/registration/merge.rs:256-306`).** Every Markdown sequence document selects **both** `composition.yaml` (which `$extends: common-attributes@./shared.yaml`) and `sequence-document.yaml`. `shared.yaml` already declares `title`, `description`, and `metadata`. Under `merge.rs::rules_differ`, if `sequence-document.yaml` redeclares those three keys with `isolated` while `shared.yaml` omits `isolated`, registered-host validation aborts with `dm.host.trigger_conflict` (`"their constraints differ"` / `"their descriptions differ"`). **Resolution:** mark `title`, `description`, and `metadata` as `isolated` directly in `shared.yaml` (and `metadata` in `linking.yaml`), and have `sequence-document.yaml` add only the three `fm.yaml` keys absent from `shared.yaml` (`replace`, `tags`, `draft`) plus sequence controls (`kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`). (An alternative design that keeps `title`/`description`/`metadata` non-isolated on non-sequence Claudine prompts by teaching `merge.rs` sticky narrowing is also documented in §4.2.)
2. **YAML sequence roots bypass `fm.yaml` and `composition.yaml` (`darkmatter/lib/src/markdown/schemas/mod.rs:2260`).** `EffectiveSchema::for_yaml_document` validates a `kind: sequence` YAML file against `formal-sequence-file.yaml` alone — with no `fm.yaml` baseline and no `composition.yaml` trigger. Today `formal-sequence-file.yaml` declares only 6 keys (`kind`, `description`, `template`, `$schema`, `$state`, `sequence`), even though YAML sequences author root lifecycle/run controls (`initialize`, `start`, `finalize`, `loop`, `timeout`, `step_timeout`, etc.). Without extending `common-attributes@./shared.yaml` (or unioning baseline + host controls during sequence propagation), those root YAML sequence controls would leak into direct prompt tasks.
3. **`ComposeOptions::new()` disables the Darkmatter baseline by default (`darkmatter/lib/src/markdown/compose/context/options.rs:1084-1086`).** `ComposeOptions::new()` sets `baseline_schema: None` and `baseline_is_darkmatter_default: false`. Roughly 400 library callers and tests (including `test_stage2_prologue_epilogue_do_not_propagate_to_children`) never call `.with_darkmatter_baseline_schema()`, causing `prepare_schemas` to return `PreparedSchemas { schemas: None }`. Darkmatter's transclusion isolation helper (`PreparedSchemas::isolated_keys_for`) must therefore always union with the precompiled `darkmatter_baseline_isolated_keys()` (`OnceLock` derived from the shipped `fm.yaml` artifact), while `EffectiveSchema::isolated_keys()` itself remains a pure sticky union of its own configured layers.
4. **Three separate trigger-discard traps in `darkmatter/lib/src/markdown/schemas/mod.rs`.** Reading `EffectiveSchema.json_schema` alone breaks sticky-union semantics because: (a) `merge_baseline` and `merge_around_union` perform whole-property replacement (`mod.rs:3245, 3323`); (b) `resolve_trigger_layers` calls `pending.without_properties(&discarded)` on template trigger payloads *before* lowering (`mod.rs:1499, 1509`); and (c) `registered_trigger_layers` uses `checked_layer`'s pruned `part.json_schema` (`mod.rs:1576-1593`). Each layer's isolated keys must be extracted *before* pruning/merging.
5. **Four hidden `Constraint` consumers beyond the obvious parser/converter files.**
   - `simplified-schema/lib/src/simplified/grammar.rs:2174-2180` (`validate_semantic_constraints`) must allow `Constraint::Isolated` on `schema` / `type-definition` or `"$schema": "schema(isolated)"` and `"$state": "schema(isolated)"` fail to parse.
   - `darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs:247, 720, 1297` switches explicitly on `Constraint` and rejects unrecognized variants on inline objects and literals (`guard_settings`, `kind: literal(sequence; required; isolated)`).
   - `darkmatter/lib/src/markdown/schemas/about.rs` enforces **four** closed L1 parity tests (`type_accepted_constraints_match_lowering`, `block_constraint_keys_match_the_lowering`, `constraint_set_matches_descriptor_set`, `match_safe_constraint_descriptor_set_matches_implementation`).
   - `simplified-schema/lib/src/simplified/narrowing.rs:173, 198` (`has_ordinary_constraints`, `type_identity`) must classify `Constraint::Isolated` as a non-type-restricting metadata marker.
6. **Zero cache-key changes needed on `ComposeOptions`.** `inherited_child_options` writes the post-isolation map directly into `child_request.external_state`, and `ComposeOptions::classify_options` (`options.rs:4012-4019`) already encodes `external_state` into `options_hash(&child_request)`. Two parents whose `$schema` differs in `isolated` keys already produce distinct transclusion cache keys without adding any field to `ComposeOptions`.

---

## 1. All 5 Simplified Schema Property Forms & Exact Lowering Rules (`simplified-schema/lib`)

### 1.1 Exact Touch-Point Inventory

Every file in `simplified-schema/lib` and downstream crates that must handle `Constraint::Isolated`:

| File | Symbol / Line | Required Change |
| --- | --- | --- |
| `simplified-schema/lib/src/simplified/types.rs` | `Constraint` (`:1213`), `Constraint::keyword` (`:1532`), `PropertyAtom::is_isolated` (near `:992`), `PropertyDef::is_isolated` (near `:310`) | Add `Constraint::Isolated` → `"isolated"`. Add `PropertyAtom::is_isolated(&self) -> bool` (checks `property_constraints()` and `union_arms()`, mirroring `is_generated`) and `PropertyDef::is_isolated(&self) -> bool` (checks single atom or any union arm). Unlike `Generated`, `Isolated` **never** suppresses static `required`. |
| `simplified-schema/lib/src/simplified/grammar.rs` | `is_constraint_keyword` (`:655`), `parse_constraint` (`:2909`), `parse_tuple` (`:1185`), `validate_semantic_constraints` (`:2174`) | Accept `"isolated"` (0-arity flag) in constraint lists, tuples, and semantic types (`schema`, `type-definition` — required for `$schema: "schema(isolated)"` and `$state: "schema(isolated)"`). |
| `simplified-schema/lib/src/simplified/mod.rs` | `parse_dollar_constraint` (`:605`), `parse_mapping_atom` (`:874-878`) | Parse `"isolated"` flag (`isolated: true`) in `$constraints`. Partition `Constraint::Isolated` alongside `Required | Generated` off `shape.constraints` onto the enclosing `PropertyAtom.constraints` (so root `$constraints: { isolated: true }` is automatically rejected by `reject_block_constraints` with `` `$constraints` key `isolated` is not valid on the root object ``). |
| `simplified-schema/lib/src/simplified/convert.rs` | `DARKMATTER_ISOLATED_KEYWORD` (new `pub const = "x-darkmatter-isolated"`, re-exported from `simplified/mod.rs` & `lib.rs`), `is_reference_annotation` (`:325`), `union_property_to_schema` (`:878, 905`), `union_arm_fragments` (`:937`), `atom_to_schema` (`:1015, 1021, 1055`), `atom_fragment_without_null_wrap` (`:1107, 1148, 1269`), `suggestion_target_schema` (`:1368`), `inline_object_fragment` (`:1398`), `apply_array_constraints` (`:1932`) | Filter `Constraint::Isolated` out of `value_constraints` at `:1148` (beside `Category`, `Operands`, `Deferred`) so every primitive, inline-object, and `$ref` fragment builder accepts it without duplicating match arms; accept it in `is_reference_annotation`, `inline_object_fragment`, and `apply_array_constraints`; emit `"x-darkmatter-isolated": true` at the top level of the property schema; hoist it across optional `null` wrappers in `atom_to_schema` and across property unions in `union_property_to_schema`; strip any arm-level `"x-darkmatter-isolated"` in `union_arm_fragments` after hoisting so single-arm and all-arm annotations produce identical JSON; strip it in `suggestion_target_schema`. |
| `simplified-schema/lib/src/simplified/serialize.rs` | `write_constraint` (`:496`) | `Constraint::Isolated => out.push_str("isolated")`. |
| `simplified-schema/lib/src/schema/export.rs` | `write_object_constraints` (`:302`), `write_atom` (`:351`) | Emit `("isolated", true.into())` in `$constraints`; allow `Constraint::Isolated` in `write_atom`'s block-form round-trip predicate (`.all(\|c\| matches!(c, Constraint::Required \| Constraint::Generated \| Constraint::Isolated))`). |
| `simplified-schema/lib/src/resolve.rs` | `is_union_marker` (`:4560`), `is_flag` (`:4707`), error text (`:4205`) | Add `Constraint::Isolated` to `is_union_marker` (allows `steps(required; isolated)@./sequence.yaml`, `loop(isolated)@./lifecycle-events.yaml`, `policy[](min(1); isolated)@...`) and to `is_flag` (deduplicates repeated `isolated` when written on both a named type and its use site). Update error text at `:4205`. |
| `simplified-schema/lib/src/resolve/recursive.rs` | `is_use_site_marker` (`:78`), `is_arm_property_marker` (`:94`), error text (`:982`) | Add `Constraint::Isolated` to `is_use_site_marker` (hoisted into `UseSummary.markers` and stripped from shared `$defs` bodies by `strip_use_site_markers` at `:1060`) and `is_arm_property_marker` (hoisted from union arms of stored named unions). Automatically keeps cyclic/shared `$ref` sites representable via `use_is_representable` (`:747-768`) and forces inlining via `arm_differs_from_reference` (`:615`) when a shared isolated type is used as an arm of a larger property union. |
| `simplified-schema/lib/src/validate.rs` | `NON_ASSERTING_KEYWORDS` (`:1886-1905`) | Add `"x-darkmatter-isolated"`. Prevents `arm_with_assertions_beside` (`:1911`) from misreading `"x-darkmatter-isolated": true` beside `$ref` or `anyOf` as an asserting schema and wrapping narrowed union arms in a spurious `allOf`. |
| `simplified-schema/lib/src/simplified/narrowing.rs` | `has_ordinary_constraints` (`:173`), `type_identity` (`:198`) | Add `Constraint::Isolated` alongside `Constraint::Generated` so narrowing ignores `isolated` when comparing type identities. |
| `simplified-schema/lib/src/triggers/grammar.rs` | `is_match_safe_constraint` (`:607`) | **Do NOT add** `Constraint::Isolated` (intentional — mirrors `Generated`, `Deferred`, `Default`: isolation is propagation metadata, not a trigger-match predicate). |
| `darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs` | `is_annotation` (`:247`), `inline_object` (`:720`), `literal` (`:1297`) | Add `Constraint::Isolated` alongside `Constraint::Generated` so expression call-binding shape translation does not reject isolated properties (especially `guard_settings` and `kind: literal(sequence; required; isolated)`). |
| `darkmatter/lib/src/markdown/schemas/about.rs` | `SCHEMA_TYPE_DESCRIPTORS` (`:25-194`), `SCHEMA_CONSTRAINT_DESCRIPTORS` (`:262`), `BLOCK_CONSTRAINT_DESCRIPTORS` (`:494`), `SCHEMA_SHAPE_DESCRIPTORS` (`:577-587`), and tests at `:1325, :1366, :1539` | Add `"isolated"` to every type's `accepted_constraints`, add the `isolated` descriptor to `SCHEMA_CONSTRAINT_DESCRIPTORS` and `BLOCK_CONSTRAINT_DESCRIPTORS` (`on_root: false`), update the union-shape descriptions, and add `("isolated", "true")` / `Constraint::Isolated` to the four closed parity tests. |

> **Important `convert.rs` invariant (`atom_to_schema` & `union_property_to_schema`):** `"x-darkmatter-isolated": true` MUST always land on the **top-level property schema object** (never buried inside `anyOf[1]` or a union arm), regardless of whether the property is optional, required, nullable (`null`/`any`/`unknown`), a `$ref`, an array, or a union. That guarantees any JSON Schema reader — including `EffectiveSchema::isolated_keys()` and raw JSON Schema authors — checks `prop_schema.get("x-darkmatter-isolated") == Some(&Value::Bool(true))` uniformly.

---

### 1.2 Verification of All 5 Property Forms (Syntax & Lowered JSON Schema)

#### Form 1 — Primitives (including semantic `schema`, `null`, `any`/`unknown`, `literal`, `enum`)

```yaml
description: "string(isolated) -> Human-readable summary"
"$schema": "schema(isolated) -> Document schema"
change: "any(isolated) -> Change metadata"
composed: "null(isolated) -> Claudine names this event `start`"
kind: "literal(sequence; required; isolated) -> the document kind"
```

Lowered JSON Schema:

```json
{
  "description": {
    "anyOf": [
      { "type": "null" },
      { "type": "string" }
    ],
    "description": "Human-readable summary",
    "x-darkmatter-isolated": true
  },
  "$schema": {
    "anyOf": [
      { "type": "null" },
      { "type": ["string", "object", "array"], "x-darkmatter-schema": true }
    ],
    "description": "Document schema",
    "x-darkmatter-isolated": true
  },
  "change": {
    "description": "Change metadata",
    "x-darkmatter-isolated": true
  },
  "composed": {
    "type": "null",
    "description": "Claudine names this event `start`",
    "x-darkmatter-isolated": true
  },
  "kind": {
    "const": "sequence",
    "description": "the document kind",
    "x-darkmatter-isolated": true
  }
}
```

- **Why `change` and `composed` work:** `atom.admits_null()` returns `true` (`convert.rs:989`), so `atom_to_schema` returns `fragment` directly without wrapping in `anyOf`, and `atom_fragment_without_null_wrap` (`:1269`) already placed `"x-darkmatter-isolated": true` on `fragment`.
- **Why `kind` stays in `required`:** `atom_to_schema` returns `(fragment, !generated)` (`:1007`). Because `Isolated` does not set `generated`, `"kind"` remains in the parent's `required` array and gets no optional `null` wrapper.
- **Optional scalar `file(isolated)`:** `atom_to_schema` (`:1029-1043, 1048-1064`) flattens `[{"type": "null"}, {"const": ""}, {"type": "string", "format": "darkmatter-file-reference"}]` and re-inserts `"x-darkmatter-isolated": true` onto the outer object.

#### Form 2 — Arrays

```yaml
tags: "string[](isolated) -> Freeform document tags."
argv: "string[](isolated) -> arguments passed through to the agent"
sequence: "step[](required; isolated)@./sequence.yaml -> the sequence's steps, in order"
content_policy: "policy[](min(1); isolated)@../../../content-policy/schemas/content-policy.yaml"
```

Lowered JSON Schema (optional `tags`, required `sequence` referencing cyclic/shared `step`):

```json
{
  "tags": {
    "anyOf": [
      { "type": "null" },
      { "type": "array", "items": { "type": "string" } }
    ],
    "description": "Freeform document tags.",
    "x-darkmatter-isolated": true
  },
  "sequence": {
    "type": "array",
    "items": { "$ref": "#/$defs/step" },
    "description": "the sequence's steps, in order",
    "x-darkmatter-isolated": true
  }
}
```

- **Placement rule:** `PropertyAtom::is_isolated()` inspects `property_constraints()` (`constraints.iter().chain(&array_constraints)`), so both `string[](isolated)` and `string(isolated)[]` lower identically on built-in primitives. **However**, on imported non-union named types (`resolve.rs:4200`, `resolve/recursive.rs:977`), constraints before `[]` are rejected. **Convention:** always place `isolated` **after `[]`** (`T[](isolated)`, `T[](required; isolated)`, `T[](min(1); isolated)@file`), which is valid uniformly for built-ins, non-union imports, and union imports.

#### Form 3 — Inline & Block Objects (`{ ... }(isolated)` and `$constraints: { isolated: true }`)

```yaml
# Inline form (e.g., guard_settings in claudine/registration/shared.yaml)
guard_settings: "{ repetition: { enabled: boolean }, volume: { enabled: boolean } }(isolated) -> thresholds for the runaway-output guards"

# Block form (equivalent AST after parse_mapping_atom)
custom_control:
    enabled: boolean
    $constraints:
        isolated: true
```

Lowered JSON Schema (both forms produce the exact same outer structure):

```json
{
  "custom_control": {
    "anyOf": [
      { "type": "null" },
      {
        "type": "object",
        "additionalProperties": false,
        "properties": {
          "enabled": { "anyOf": [{ "type": "null" }, { "type": "boolean" }] }
        }
      }
    ],
    "x-darkmatter-isolated": true
  }
}
```

- In `parse_mapping_atom` (`simplified/mod.rs:874-881`), `Constraint::Isolated` moves from `shape.constraints` to `atom.constraints`, matching `Required` and `Generated`.
- In `schema/export.rs:344-359`, `write_atom` moves `Constraint::Isolated` back to `$constraints: { isolated: true }` on YAML export, preserving round-trip fidelity even when the block object also has pattern keys or `min-keys`/`max-keys`.
- On a root schema mapping (`ObjectScope::Root`), `parse_mapping_atom` does not run, so `reject_block_constraints` (`convert.rs:1439`) rejects `$constraints: { isolated: true }` cleanly.

#### Form 4 — Constrained Imported & Union References (Acyclic, Shared, and Cyclic `$ref`)

```yaml
# Non-union object imports (shared via $defs or inlined)
style: "style(isolated)@./style.yaml"
ctx: "context(isolated)@./context-variables.yaml"

# Non-union scalar import
prompt: "prompt(isolated)@./document.yaml"
prompt_req: "prompt(required; isolated)@./document.yaml"

# Union imports (inlined or cyclic $ref)
sequence: "steps(required; isolated)@./sequence.yaml -> the sequence's steps"
loop: "loop(isolated)@./lifecycle-events.yaml -> run the prompt repeatedly"
initialize: "initialize-stack(isolated)@./lifecycle-events.yaml -> actions run before preflight"
```

Lowered JSON Schema:

1. **Optional `$ref` (e.g., `initialize: "initialize-stack(isolated)@./lifecycle-events.yaml"`):**
   - `recursive.rs::is_use_site_marker` keeps `Constraint::Isolated` on the `TypeExpr::Ref` atom, not in `#/$defs/initialize-stack`.
   - `convert.rs::reference_fragment` (`:295`) skips `Constraint::Isolated` via `is_reference_annotation` and emits `{"$ref": "#/$defs/initialize-stack"}`.
   - `atom_fragment_without_null_wrap` (`:1269`) attaches `"x-darkmatter-isolated": true`.
   - `atom_to_schema` (`:1021, 1055`) hoists `"x-darkmatter-isolated": true` out of `fragment` onto the outer optional `anyOf` wrapper:
     ```json
     {
       "anyOf": [
         { "type": "null" },
         { "$ref": "#/$defs/initialize-stack" }
       ],
       "description": "actions run before preflight",
       "x-darkmatter-isolated": true
     }
     ```
2. **Required `$ref` (e.g., `sequence: "steps(required; isolated)@./sequence.yaml"` if `steps` is stored as `$ref`):**
   - `atom_to_schema` (`:1001`) returns `fragment` directly without wrapping, keeping `"x-darkmatter-isolated": true` as a Draft 2020-12 sibling beside `"$ref"`:
     ```json
     {
       "$ref": "#/$defs/steps",
       "description": "the sequence's steps",
       "x-darkmatter-isolated": true
     }
     ```
   - Because `"x-darkmatter-isolated"` is in `NON_ASSERTING_KEYWORDS` (`validate.rs:1886`), Draft 2020-12 narrowing and validation treat it strictly as an annotation.
3. **Inlined postfixed union reference (e.g., `loop: "loop(isolated)@./lifecycle-events.yaml"` or `steps(required; isolated)` when inlined):**
   - `resolve.rs::apply_union_postfix` (`:4452-4490`) checks `is_union_marker` (accepts `Constraint::Isolated`) and produces `PropertyDef::Single(PropertyAtom { ty: TypeExpr::Union(arms), is_array: false, constraints: [Constraint::Isolated], .. })`.
   - `property_def_to_schema` (`:845`) calls `union_property_to_schema(name, arms, Some(atom), defs)`.
   - `any_isolated` reads `atom.is_isolated()` → `true`, and emits `"x-darkmatter-isolated": true` on the outer `{"anyOf": [...], "x-darkmatter-isolated": true}` object!

#### Form 5 — Multi-Arm YAML-List Property Unions

Examples in real shipped schemas: `prologue`, `epilogue`, `hash` (`fm.yaml`); `exit_expressions` (`shared.yaml`); `hash` (`document.yaml`); `allowed-tools`, `tools`, `skills`, `compatibility`, `argument-hint` (`linking.yaml`).

```yaml
# Inline YAML-list union (e.g., fm.yaml)
prologue:
    - "string(isolated; default(null)) -> File reference, inline text, or bare filename..."
    - "string[] -> Ordered prologue references..."

# Inline YAML-list union with inline-object array arm (shared.yaml)
exit_expressions:
    - "{ pattern: string, patterns: string[], kind: enum(literal, regex), ignore_case: boolean, scope: string }[](isolated) -> output patterns..."
    - "{ mode: enum(merge, override), rules: { ... }[] } -> output patterns..."

# Or on a named union imported by shared.yaml (two equivalent authoring choices):
# Choice A (use-site — preferred when keeping library types reusable):
allowed-tools: "allowed-tools(isolated)@./linking.yaml"
# Choice B (library definition arm 0 — every importer inherits `isolated`):
# in linking.yaml:
allowed-tools:
    - "string(isolated) -> tools the resource may use, as one comma- or space-separated string"
    - "string[] -> tools the resource may use"
```

Exact hoisting behavior in `convert.rs`:

1. `union_property_to_schema` (`:878-908`) computes:
   ```rust
   let any_isolated = container.map_or_else(
       || arms.iter().any(PropertyAtom::is_isolated),
       PropertyAtom::is_isolated,
   );
   ```
   Placing `isolated` on **any** arm (convention: the first arm for inline YAML lists) hoists `"x-darkmatter-isolated": true` onto the outer property union object.
2. In `union_arm_fragments` (`:937`), `map.shift_remove(DARKMATTER_ISOLATED_KEYWORD)` strips the per-arm annotation after `atom_fragment_without_null_wrap` returns. **Result:** putting `isolated` on arm 0 vs. every arm produces **byte-for-byte identical** lowered JSON Schema:
   ```json
   {
     "prologue": {
       "anyOf": [
         { "type": "null" },
         { "type": "string", "description": "File reference, inline text, or bare filename..." },
         { "type": "array", "items": { "type": "string" }, "description": "Ordered prologue references..." }
       ],
       "default": null,
       "x-darkmatter-isolated": true
     }
   }
   ```
3. **Important note for `claudine/registration/{document,linking}.yaml`:** Putting `isolated` at the **library definition** (`document.yaml` / `linking.yaml`) works cleanly AND propagates through `shared.yaml`, `composition.yaml`, and `inline-composition.yaml` via `UseSummary` (if shared) or `UnionTarget` (if inlined). Putting `isolated` at the **use site** in `shared.yaml` (`hash(isolated)@./document.yaml`, `allowed-tools(isolated)@./linking.yaml`) ALSO works cleanly and keeps `document.yaml`/`linking.yaml` clean if ever reused elsewhere. Because `resolve.rs::is_flag` deduplicates `Constraint::Isolated`, even marking both definition and use site is idempotent. **Recommendation:** mark `isolated` on the **library types** in `document.yaml` and `linking.yaml` (so every Claudine trigger that imports `prompt`, `sections`, `hash`, `last_updated`, or any linking key gets `isolated` automatically — including both arms of `inline-composition.yaml`!) and keep explicit `isolated` on inline properties in `shared.yaml`, `sequence-document.yaml`, and `formal-sequence-file.yaml`.

---

## 2. Exact Sticky-Union Design in `darkmatter/lib/src/markdown/schemas/mod.rs` (`EffectiveSchema`)

### 2.1 The Three Trigger-Discard Traps

Today, `DarkmatterSchemas::site_schema_with_override` (`darkmatter/lib/src/markdown/schemas/mod.rs:1143-1325`) discards lower-layer property declarations in three places:

1. **Whole-property replacement during JSON merge (`mod.rs:3245-3249`, `3321-3324`):**
   `merge_baseline` replaces `baseline[prop]` or `trigger[prop]` wholesale with `document[prop]`, and `merge_around_union` calls `remove_properties(&mut top, &redeclared)`. If a user document's `$schema` writes `hash: string` (omitting `isolated`), `merged_json["properties"]["hash"]` no longer carries `"x-darkmatter-isolated": true`.
2. **Template trigger pruning before slot evaluation (`mod.rs:1211-1212`, `1498-1513`):**
   ```rust
   let document_declared = resolved.as_ref().map(|r| declared_by_json(&r.json_schema)).unwrap_or_default();
   let trigger_layers = self.resolve_trigger_layers(source, &document_declared)?;
   ```
   Inside `resolve_trigger_layers`, for any matched root trigger with `payload.pending = Some(pending)`:
   ```rust
   concrete = self.resolve_pending_payload(
       trigger,
       &pending.without_properties(&discarded),
       PayloadForm::Object,
   )?;
   ```
   `pending.without_properties(&discarded)` strips properties that `document_declared` (or a higher trigger) redeclares *before* lowering to `concrete.json_schema`. Reading `concrete.json_schema` alone would miss any `isolated` key that a template trigger declared and the document overrode!
3. **Registered-host checked-layer pruning (`mod.rs:1576-1595`):**
   `registered_trigger_layers` calls `profile.checked_layer(matched)` (or `merge::check`), which strips losing properties from `part.json_schema` into a pruned `CheckedPayload`. Reading only `part.json_schema` instead of the unpruned `resolved[position]` would miss any property stripped during host-layer deduplication (though identical declarations preserve it on the winner, reading the unpruned payload before pruning guarantees sticky union by construction).

### 2.2 Clean Extraction Helpers

Add two tiny pure helpers in `darkmatter/lib/src/markdown/schemas/mod.rs` (zero allocations beyond the result set):

1. **`isolated_keys_in_json(schema: &Value) -> BTreeSet<String>`** — handles both lowered Simplified Schema AND raw JSON Schema:
   - Walks top-level object schemas: the root itself, plus every arm under root `anyOf` / `oneOf` / `allOf` (depth-bounded, e.g. max 16, to guard hostile raw JSON Schemas).
   - For each object's `"properties"` map, inserts `key` whenever:
     - `prop_schema.get(DARKMATTER_ISOLATED_KEYWORD) == Some(&Value::Bool(true))`, **OR**
     - (raw JSON Schema `$ref` support) `prop_schema` carries `"$ref": "#/$defs/<name>"` (or `"#/definitions/<name>"`) and following local `$ref` hops (cycle-guarded) reaches `"x-darkmatter-isolated": true`.
   - For a root union (such as `inline-composition.yaml` with `prompt` in arm 0 and `sections` in arm 1), taking the union across all arms yields `{prompt, sections, ...common-attributes}` regardless of which arm is active.
2. **`isolated_keys_in_root(root: &SchemaRoot) -> BTreeSet<String>`** — handles unresolved `PendingSchema` templates (when `{{ … }}` slots have not been evaluated yet):
   - Walks `SchemaRoot::Single(shape)` or every `SchemaArm::Inline(shape)` of `SchemaRoot::Union(arms)`.
   - Inserts every `key` where `def.is_isolated()` is `true`.

### 2.3 Per-Layer Storage & `Arc`-Shared Fast Path

1. **Precompiled Darkmatter baseline isolated keys (`darkmatter/lib/src/markdown/schemas/mod.rs` + `shipped.rs`):**
   ```rust
   /// Precompiled isolated frontmatter keys from the shipped Darkmatter baseline (`fm.yaml`).
   pub fn darkmatter_baseline_isolated_keys() -> &'static Arc<BTreeSet<String>> {
       static KEYS: OnceLock<Arc<BTreeSet<String>>> = OnceLock::new();
       KEYS.get_or_init(|| Arc::new(isolated_keys_in_json(darkmatter_base_json_schema_ref())))
   }
   ```
   - Computed **once per process** from the already-loaded shipped `fm.yaml` JSON Schema — zero hardcoded Rust key lists, zero per-document scan.
2. **`BaselineSchema` (`mod.rs:391`):**
   Add field `isolated_keys: Arc<BTreeSet<String>>`:
   - In `with_darkmatter_baseline_in` (`:965`): `Arc::clone(darkmatter_baseline_isolated_keys())` (O(1) refcount bump!).
   - In `set_baseline_json` (`:982`): `Arc::new(isolated_keys_in_json(&value))` (computed once when custom baseline is configured).
3. **`TriggerLayer` (`mod.rs:453`):**
   Add field `isolated_keys: BTreeSet<String>`:
   - In `registered_trigger_layers` (`:1591`): computed from the **unpruned** `payload.json_schema` (`resolved[position]`), NOT the pruned `part.json_schema`!
   - In `resolve_trigger_layers` (`:1501-1528`):
     - If `payload.pending` is `Some(pending)`: union `pending.template().map(|(root, _)| isolated_keys_in_root(root))` (extracted **before** `pending.without_properties(&discarded)`) with `isolated_keys_in_json(&concrete.json_schema)`.
     - If `payload.pending` is `None`: `isolated_keys_in_json(&payload.json_schema)` (never pruned in `resolve_trigger_layers`).
4. **`EffectiveSchema` (`mod.rs:1842`):**
   Add field `isolated_keys: Arc<BTreeSet<String>>` and public accessor:
   ```rust
   /// Top-level frontmatter keys that must not implicitly propagate to child
   /// documents, computed as the sticky union across baseline, all matched
   /// triggers (before document-`$schema` pruning), and the document `$schema`.
   #[must_use]
   pub fn isolated_keys(&self) -> &BTreeSet<String> {
       &self.isolated_keys
   }

   pub(crate) fn isolated_keys_arc(&self) -> Arc<BTreeSet<String>> {
       Arc::clone(&self.isolated_keys)
   }
   ```
   - **Baseline-only fast path (`mod.rs:1249`):** when `trigger_layers.is_empty()` and `resolved.is_none()`, `EffectiveSchema.isolated_keys = Arc::clone(&base.isolated_keys)` — **zero allocation, zero JSON scan!**
   - **General path (`site_schema_with_override`):**
     ```rust
     let isolated_keys = if trigger_layers.is_empty() && resolved.is_none() {
         self.baseline
             .as_ref()
             .map(|base| Arc::clone(&base.isolated_keys))
             .unwrap_or_default()
     } else {
         let mut keys = self
             .baseline
             .as_ref()
             .map(|base| base.isolated_keys.as_ref().clone())
             .unwrap_or_default();
         for layer in &trigger_layers {
             keys.extend(layer.isolated_keys.iter().cloned());
         }
         if let Some(doc) = &resolved {
             keys.extend(isolated_keys_in_json(&doc.json_schema));
         }
         Arc::new(keys)
     };
     ```
   - **`EffectiveSchema::for_yaml_document` (`mod.rs:2260`):** `isolated_keys: Arc::new(isolated_keys_in_json(&json_schema))`.

### 2.4 Edge-Case Matrix (`DarkmatterSchemas::isolated_keys_for`)

Add a fallible-free helper on `DarkmatterSchemas` (and `PreparedSchemas`) for compose/preflight/graph consumers:

```rust
impl DarkmatterSchemas {
    /// Best-effort sticky union of isolated keys for `source`, remaining safe
    /// when `$schema` is absent, deferred (unresolved slots), or broken.
    pub fn isolated_keys_for(&self, source: &Markdown) -> Arc<BTreeSet<String>> { ... }
}
```

| Scenario | Behavior of `DarkmatterSchemas::isolated_keys_for(source)` | Behavior of `PreparedSchemas::isolated_keys_for(source)` (Darkmatter Compose) |
| --- | --- | --- |
| **(a) No `$schema`, no matched triggers, Darkmatter baseline active** | Fast path: `Arc::clone(&base.isolated_keys)` (O(1), `fm.yaml`'s 16 isolated keys) | Same O(1) `Arc::clone` |
| **(a') No `$schema`, no triggers, `ComposeOptions::new()` (`baseline: None`)** | `DarkmatterSchemas::effective_for` returns `Ok(None)`; `isolated_keys_for` returns empty set (pure to its config) | `PreparedSchemas::isolated_keys_for` returns `Arc::clone(darkmatter_baseline_isolated_keys())` so `fm.yaml` controls (`prologue`, `epilogue`, `ctx`, `$schema`, etc.) **always** stay isolated in Darkmatter transclusion! |
| **(b) Deferred `$schema` or deferred trigger (`SiteSchema::Template { template, .. }`)** | Unions `baseline.isolated_keys` ∪ matched triggers' unpruned isolated keys (concrete JSON or `pending.template()`) ∪ `template.as_ref().map(\|t\| isolated_keys_in_root(&t.root))` | Same, unioned with `darkmatter_baseline_isolated_keys()` |
| **(c) Broken `$schema` (`site_schema_for` returns `Err(_)`)** | Falls back to `baseline.isolated_keys` (plus any matched triggers whose match is pure frontmatter/path and succeeds without the broken document `$schema`, or baseline alone) | Falls back to `baseline.isolated_keys` ∪ `darkmatter_baseline_isolated_keys()` |

---

## 3. All 4 Darkmatter Child-Inheritance Call Sites & Cache Fingerprinting

### 3.1 Audit of Every Inheritance Site

A repository-wide search across `darkmatter/lib/src/` for `inherited_child_options`, `markdown_child_options`, `remote_child_options`, `inclusion_child_options`, and hardcoded `"prologue" | "epilogue" | "ctx"` confirmed there are **no other stripping sites** anywhere in `darkmatter/lib`:

- `transclusion/engine.rs:1952-1971` (`inherited_child_options`) is the **single funnel** through which all 4 sites construct a local child's `external_state`:
  - `remote_child_options` (`engine.rs:1980`) inherits **no** parent state at all (`external_state` is not populated from `parent_data`), so it needs no change.
  - `cache/hashing.rs:94` filters `"ctx"` out of `effective_state_hash` because `ctx` is hashed separately by `context_hash` — unrelated to child inheritance, stays untouched.

### 3.2 Exact Plumbing Design Across All 4 Sites

Change the shared helper signatures in `darkmatter/lib/src/markdown/compose/transclusion/engine.rs`:

```rust
pub(crate) fn inherited_child_options(
    parent: &ComposeRequest,
    parent_data: &SharedFrontmatter,
    isolated_keys: &BTreeSet<String>,
    directive_options: &transclusion::BlockOptions,
) -> ComposeRequest {
    let inherited: Map<String, Value> = parent_data
        .iter()
        .filter(|(key, _)| !isolated_keys.contains(key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    ...
}

pub(crate) fn markdown_child_options(
    parent: &ComposeRequest,
    parent_data: &SharedFrontmatter,
    isolated_keys: &BTreeSet<String>,
    directive_options: &transclusion::BlockOptions,
    path: &Path,
    opening: Option<SourceOpening>,
) -> ComposeRequest {
    inherited_child_options(parent, parent_data, isolated_keys, directive_options)
        .with_accepted_source_file(path.to_path_buf(), opening)
}
```

How each of the 4 sites obtains `isolated_keys: Arc<BTreeSet<String>>` **without repeating schema resolution or trigger discovery**:

| # | Call Site | Exact Lines | How `isolated_keys` Flows In |
| --- | --- | --- | --- |
| **1** | `darkmatter/lib/src/markdown/compose/transclusion/engine.rs` (`render_markdown_transclusion`) | `:1629` (called from `resolve_prepared_transclusion` ← `pipeline/phases.rs:550`) | `pipeline/phases.rs` already holds `schemas: &PreparedSchemas` (line `:428`). Compute `let isolated_keys = schemas.isolated_keys_for(self);` **once per transclusion phase** beside `PhaseStateIdentity::capture(state)` (`:540`) and pass `&isolated_keys` into `resolve_prepared_transclusion` → `render_markdown_transclusion` → `markdown_child_options`. |
| **2** | `darkmatter/lib/src/markdown/compose/nested.rs` (`as_markdown()`) | `:259` (`prepare_child`), `:180` (`align_state`), `:121` (`Scope`) | Store `(SharedFrontmatter, Arc<BTreeSet<String>>)` in `Scope::caller_state` (initialized to `None`). In `pipeline/mod.rs:771`, pass `nested.align_state(effective_state.data(), prepared_schemas.isolated_keys_for(self))`. In `prepare_child`, pass `&isolated_keys` to `inherited_child_options`. (Calls made before `align_state` during frontmatter interpolation still take the `None` branch at `:264`, cloning the parent's already-filtered request.) |
| **3** | `darkmatter/lib/src/markdown/compose/preflight/collect.rs` (3 sub-sites: body `::file`, `prologue`/`epilogue`, `as_markdown()`) | `:991`, `:1136`, `:1202` | `collect_document` already runs an inline-pre compose via `prepared.run_root_pipeline_collecting(inline_options, None)` (`:543`), which runs `prepare_document` and prepares `prepared_schemas`. Add `pub(crate) isolated_keys: Option<Arc<BTreeSet<String>>>` to `ComposeReport` (populated **only** on inline-pre-only passes where `BlockTransclusion` is disabled, matching `body_data`/`body_origin`/`body_sites` at `pipeline/mod.rs:355` so normal compose reports keep `None` and `ComposeReport::PartialEq` tests stay green; never merged from child reports). `collect_document` reads `let isolated_keys = prepared_report.isolated_keys.take().unwrap_or_else(|| Arc::clone(darkmatter_baseline_isolated_keys()));` and threads `&isolated_keys` into all 3 calls. The ad-hoc `child_state_data.remove("$schema")` at `:635` is now redundant with `fm.yaml` isolating `$schema` (can be simplified or kept as belt-and-suspenders). |
| **4** | `darkmatter/lib/src/reference/graph.rs` (`darkmatter/lib/src/markdown/reference/graph.rs`: `inclusion_child_options` + 3 sub-sites) | `:491` (`inclusion_child_options`), `:712` (body `::file`), `:1022` (`prologue`), `:1114` (`epilogue`) | `prepare_content` (`:1292`) also calls `result.run_root_pipeline_collecting(inline_pre_options, None)`. Forward `isolated_keys: report.isolated_keys.unwrap_or_else(|| Arc::clone(darkmatter_baseline_isolated_keys()))` on `PreparedReferenceContent` (`:1233`) and thread `&isolated_keys` into `inclusion_child_options` at all 3 sites. |

> **Unit test call sites in `engine.rs:2135-2142`:** Those 3 direct unit tests of `inherited_child_options` pass `darkmatter_baseline_isolated_keys()` (or a custom `BTreeSet` when testing custom `$schema` isolation).

### 3.3 Compose Cache Key & Fingerprinting Verification

Inspected `transclusion/engine.rs:1628-1676`, `compose/cache/hashing.rs:90-177`, and `compose/context/options.rs:3850-4127`:

- `render_markdown_transclusion` builds `child_request = markdown_child_options(...)` **before** computing `cache_key` (`engine.rs:1628-1676`).
- `child_request.external_state` holds the **already-filtered** `inherited` map produced by `inherited_child_options`.
- `cache_key` includes `options_hash(&child_request)`, which calls `ComposeOptions::classify_options` (`options.rs:4012-4019`), which hashes `external_state` via `ordered_canonical(v)`.
- **Conclusion:**
  1. **No new field on `ComposeOptions` is required**, and `ReferenceGraphOptionsIdentity` / `compose_cache_fingerprint` need **zero** changes.
  2. Two parents with identical frontmatter values whose schemas mark different custom keys as `isolated` produce different `child_request.external_state` values → different `options_hash(&child_request)` → **distinct cache entries automatically** (no false cache hits are possible).
  3. `state_identity.state_hash` (`engine.rs:1672`) hashes `state.data()` (all keys except `ctx`, pre-isolation), which is strictly a superset of `external_state`. That is conservative (never causes a stale cache hit; at worst two different parents differing only in an isolated key won't share a single-flight entry for the same child within one run). Keeping `PhaseStateIdentity` unchanged is surgical and 100% sound.

---

## 4. Repo-Wide Audit of Schemas, Partials, Tests & `just shipped-schemas`

### 4.1 Key-by-Key Isolation Table: `darkmatter/schemas/partials/fm.yaml`

All 22 keys declared in `darkmatter/schemas/partials/fm.yaml`:

| Key | Current Type | Isolated? | Target Syntax in `fm.yaml` |
| --- | --- | :---: | --- |
| `"$schema"` | `schema` | **Yes** | `"schema(isolated) -> ..."` |
| `title` | `string` | No | unchanged |
| `description` | `string` | No | unchanged |
| `tags` | `string[]` | No | unchanged |
| `draft` | `boolish` | No | unchanged |
| `last_updated` | `date` | **Yes** | `"date(isolated) -> ..."` |
| `content_policy` | `policy[](min(1))@../../../content-policy/schemas/content-policy.yaml` | **Yes** | `"policy[](min(1); isolated)@../../../content-policy/schemas/content-policy.yaml"` |
| `metadata` | `object` | No | unchanged |
| `hash` | YAML list (`string`, block object) | **Yes** | arm 0: `"string(isolated) -> ..."` |
| `style` | `style@./style.yaml` | **Yes** | `"style(isolated)@./style.yaml"` |
| `change` | `any` | **Yes** | `"any(isolated) -> ..."` |
| `replace` | `object` | No | unchanged |
| `ctx` | `context@./context-variables.yaml` | **Yes** | `"context(isolated)@./context-variables.yaml"` |
| `prologue` | YAML list (`string(default(null))`, `string[]`) | **Yes** | arm 0: `"string(isolated; default(null)) -> ..."` |
| `epilogue` | YAML list (`string(default(null))`, `string[]`) | **Yes** | arm 0: `"string(isolated; default(null)) -> ..."` |
| `ignore_invalid` | `boolish` | **Yes** | `"boolish(isolated) -> ..."` |
| `interpolate_code_blocks` | `boolish` | **Yes** | `"boolish(isolated) -> ..."` |
| `initialize` | `md-initialize-stack@darkmatter:md-lifecycle` | **Yes** | `"md-initialize-stack(isolated)@darkmatter:md-lifecycle -> ..."` |
| `composed` | `md-stack@darkmatter:md-lifecycle` | **Yes** | `"md-stack(isolated)@darkmatter:md-lifecycle -> ..."` |
| `failed` | `md-stack@darkmatter:md-lifecycle` | **Yes** | `"md-stack(isolated)@darkmatter:md-lifecycle -> ..."` |
| `cancelled` | `md-stack@darkmatter:md-lifecycle` | **Yes** | `"md-stack(isolated)@darkmatter:md-lifecycle -> ..."` |
| `finalize` | `md-stack@darkmatter:md-lifecycle` | **Yes** | `"md-stack(isolated)@darkmatter:md-lifecycle -> ..."` |

**Totals:** 16 `isolated`, 6 non-isolated (`title`, `description`, `tags`, `draft`, `metadata`, `replace`).

---

### 4.2 Key-by-Key Isolation Table: `claudine/registration/*.yaml`

#### Critical Finding A: Host-Trigger Merge Interaction (`composition.yaml` + `sequence-document.yaml`)

Every Markdown sequence document selects **both** `composition.yaml` (which `$extends: common-attributes@./shared.yaml`) and `sequence-document.yaml` (verified in `claudine/lib/tests/l1/registration_inventory.rs:282`, `claudine_trigger_selection_parity.rs:159`, `sequence-document.yaml:15-17`). Because Claudine registers its bundle as a registered host (`claudine/lib/src/composition/schema/bundle.rs:123`), `darkmatter/lib/src/config/registration/merge.rs::check()` compares every property declared by both triggers:

- `common-attributes@./shared.yaml` declares **`title`** (`:34`), **`description`** (`:35`), and **`metadata`** (`:63` → `metadata@./linking.yaml`).
- `common-attributes@./shared.yaml` does **not** declare `replace`, `tags`, or `draft`.
- In `merge.rs:288-306`, `rules_differ` checks `same_set(&descriptions(a), &descriptions(b))` and `same_set(&rules_a, &rules_b)` where `rules()` includes every `Constraint` except `Members`, `LiteralValue`, `Options`, and `Required`. If `sequence-document.yaml` redeclares `title`, `description`, or `metadata` with `isolated` while `shared.yaml` omits `isolated`, `merge::check` fails with **`dm.host.trigger_conflict`**!

Two clean options exist — **Option 1 is strongly recommended**:

- **Option 1 (Recommended — zero `merge.rs` churn, matches `spec.md` Decision 3.3 row 3):**
  - Mark `title`, `description`, and `metadata` as `isolated` in **`shared.yaml`** (and `metadata` in **`linking.yaml`**) — consistent with `spec.md` line 281 ("All Claudine, inline-composition, linking, and harness control keys" are `isolated` except `agent` and `model`).
  - Do **not** redeclare `title`, `description`, or `metadata` in `sequence-document.yaml` (avoiding duplication and `dm.host.trigger_conflict`, matching `sequence-document.yaml:15-17`).
  - In `sequence-document.yaml`, declare `replace`, `tags`, `draft` (the 3 `fm.yaml` keys absent from `shared.yaml`) plus sequence controls (`kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`), all marked `isolated`.
  - Repo audit confirmed **zero** Claudine prompts or tests rely on a Claudine prompt's `title`, `description`, or `metadata` propagating implicitly into a `::file` partial. Plain Darkmatter documents (without Claudine controls) still use `fm.yaml` alone, where `title`, `description`, `tags`, `draft`, `metadata`, `replace` stay non-isolated.
- **Option 2 (Alternative — only if non-sequence Claudine prompts MUST keep `title`/`description`/`metadata` non-isolated for `::file` partials):**
  - Keep `title`, `description`, `metadata` non-isolated in `shared.yaml`/`linking.yaml`.
  - Redeclare them with identical descriptions and `isolated` in `sequence-document.yaml`.
  - Update `darkmatter/lib/src/config/registration/merge.rs` (`relation`, `rules_differ`, `rules`) so adding `Constraint::Isolated` on an otherwise-equivalent declaration is treated as strictly narrower (`FirstNarrower`/`SecondNarrower`) rather than a conflict.

#### Critical Finding B: YAML Sequence Roots (`formal-sequence-file.yaml`)

`EffectiveSchema::for_yaml_document` (`darkmatter/lib/src/markdown/schemas/mod.rs:2260`) validates a `kind: sequence` YAML file against `formal-sequence-file.yaml` alone — with **no** `fm.yaml` baseline and **no** `composition.yaml` trigger. However, root YAML sequences author lifecycle events (`initialize`, `start`, `blocked`, `success`, `failure`, `cancelled`, `finalize`), `loop`, `timeout`, `step_timeout`, etc. (see `claudine/lib/src/composition/sequence/preflight/tests/prepared_shell_values.rs:253`).

To prevent root YAML sequence controls from leaking into direct prompt tasks (Decision 2.2):
1. `formal-sequence-file.yaml` should either `$extends: common-attributes@./shared.yaml` (making every Claudine control typed and isolated on YAML sequences too — verify `claudine/lib/tests/l1/sequence_schema.rs` & `claudine_schemas.rs` expectations for `composed`/`failed`/`agent`/`model`) **OR** declare the sequence-valid subset of `common-attributes`; **AND**
2. Claudine's sequence value-propagation layer should union `effective.isolated_keys()` with `darkmatter_baseline_isolated_keys()` (so Darkmatter controls like `ctx`, `prologue`, `epilogue`, `style`, `ignore_invalid`, `interpolate_code_blocks`, `change` written on a YAML sequence never leak into direct child Markdown prompts either).

#### Complete File-by-File Key Inventory (`claudine/registration/`)

1. **`claudine/registration/document.yaml` (4 types — library definitions):**
   - `prompt`: `"string(isolated) -> ..."`
   - `sections`: `"object(isolated) -> ..."`
   - `hash`: arm 0 `"string(isolated) -> ..."`
   - `last_updated`: `"date(isolated) -> ..."`

2. **`claudine/registration/linking.yaml` (11 types — library definitions):**
   - `name`: `"string(isolated) -> ..."` *(Note: explicit `with: { name: ... }` still works; repo audit confirmed no Claudine prompt or sequence test relies on implicit parent-to-child `fm.name` inheritance, though `spec.md` should note `name` is isolated as a linking control.)*
   - `allowed-tools`: arm 0 `"string(isolated) -> ..."`
   - `tools`: arm 0 `"string(isolated) -> ..."`
   - `skills`: arm 0 `"string(isolated) -> ..."`
   - `license`: `"string(isolated) -> ..."`
   - `compatibility`: arm 0 `"string(isolated) -> ..."`
   - `metadata`: `"object(isolated) -> ..."` *(under Option 1 above)*
   - `user-invocable`: `"boolean(isolated) -> ..."`
   - `disable-model-invocation`: `"boolean(isolated) -> ..."`
   - `argument-hint`: arm 0 `"string(isolated) -> ..."`
   - `max_turns`: `"number(integer; isolated) -> ..."`

3. **`claudine/registration/shared.yaml` (`common-attributes`, 39 keys):**
   - **Non-isolated (2 keys — enforced by L1 parity test in `claudine-lib`):**
     - `agent` (unchanged)
     - `model` (unchanged)
   - **Marked `isolated` (37 keys):**
     - Primitives/enums/null/schema: `interactive: "boolean(isolated) -> ..."`, `"$schema": "schema(isolated) -> ..."`, `title: "string(isolated) -> ..."` *(Option 1)*, `description: "string(isolated) -> ..."` *(Option 1)*, `composed: "null(isolated) -> ..."`, `failed: "null(isolated) -> ..."`, `timeout: "string(not-empty; isolated) -> ..."`, `step_timeout: "string(not-empty; isolated) -> ..."`, `timeout_warn: "string(not-empty; isolated) -> ..."`, `step_timeout_warn: "string(not-empty; isolated) -> ..."`, `operation: "string(isolated) -> ..."`, `yolo: "boolean(isolated) -> ..."`, `verbosity: "string(suggest(silent, quiet, verbose); isolated) -> ..."`, `mode: "enum(append, replace; isolated) -> ..."`
     - Arrays: `argv: "string[](isolated) -> ..."`, `content_policy: "policy[](min(1); isolated)@./content-policy.yaml"`
     - Inline object: `guard_settings: "{ ... }(isolated) -> ..."`
     - YAML-list union: `exit_expressions` arm 0 `"{ ... }[](isolated) -> ..."`
     - Imported from `./lifecycle-events.yaml`: `loop: "loop(isolated)@./lifecycle-events.yaml -> ..."`, `initialize: "initialize-stack(isolated)@./lifecycle-events.yaml -> ..."`, `start: "stack(isolated)@./lifecycle-events.yaml -> ..."`, `blocked: "stack(isolated)@./lifecycle-events.yaml -> ..."`, `success: "stack(isolated)@./lifecycle-events.yaml -> ..."`, `failure: "stack(isolated)@./lifecycle-events.yaml -> ..."`, `cancelled: "cancelled-stack(isolated)@./lifecycle-events.yaml -> ..."`, `finalize: "stack(isolated)@./lifecycle-events.yaml -> ..."`
     - Imported from `./document.yaml`: `hash: hash@./document.yaml`, `last_updated: last_updated@./document.yaml` *(inherit `isolated` from `document.yaml`; writing `(isolated)` at the use site is also valid and idempotent)*
     - Imported from `./linking.yaml`: `name`, `allowed-tools`, `tools`, `skills`, `license`, `compatibility`, `metadata`, `user-invocable`, `disable-model-invocation`, `argument-hint`, `max_turns` *(inherit `isolated` from `linking.yaml`)*

4. **`claudine/registration/composition.yaml`:**
   - `$extends: common-attributes@./shared.yaml` (inherits 37 isolated + 2 non-isolated `agent`, `model`)
   - `prompt: prompt@./document.yaml` (inherits `isolated` from `document.yaml`)

5. **`claudine/registration/inline-composition.yaml` (2-arm root union):**
   - Arm 0: `$extends: common-attributes@./shared.yaml`, `prompt: "prompt(required)@./document.yaml"` (inherits `isolated` from `document.yaml`), `sections: "null(isolated) -> ..."`
   - Arm 1: `$extends: common-attributes@./shared.yaml`, `prompt: "null(isolated) -> ..."`, `sections: "sections(required)@./document.yaml"` (inherits `isolated` from `document.yaml`)

6. **`claudine/registration/sequence-document.yaml` (trigger layered alongside `composition.yaml`):**
   - `kind: "literal(sequence; isolated) -> the document kind"` *(optional on Markdown sequences)*
   - `sequence: "steps(required; isolated)@./sequence.yaml -> ..."`
   - `template: "object(isolated) -> ..."`
   - `$state: "schema(isolated) -> ..."`
   - `default-values: "object(isolated) -> fallback values applied to tasks and child documents when nothing higher sets the key"`
   - `fail_fast: "null(isolated) -> removed: ..."`
   - `replace: "object(isolated) -> ..."` *(non-isolated in `fm.yaml`; isolated on sequences)*
   - `tags: "string[](isolated) -> ..."` *(non-isolated in `fm.yaml`; isolated on sequences)*
   - `draft: "boolish(isolated) -> ..."` *(non-isolated in `fm.yaml`; isolated on sequences)*
   - *(Under Option 1, `title`, `description`, `metadata` are already isolated via `composition.yaml` / `shared.yaml` and are NOT redeclared here.)*

7. **`claudine/registration/formal-sequence-file.yaml` (`kind: yaml-schema`):**
   - Recommended: `$extends: common-attributes@./shared.yaml` (or explicit control declarations — see Finding B), plus:
   - `kind: "literal(sequence; required; isolated) -> ..."`
   - `title: "string(isolated) -> ..."` *(inherited if `$extends: common-attributes`)*
   - `description: "string(isolated) -> ..."` *(inherited if `$extends: common-attributes`)*
   - `tags: "string[](isolated) -> ..."`
   - `draft: "boolish(isolated) -> ..."`
   - `replace: "object(isolated) -> ..."`
   - `metadata: "object(isolated) -> ..."` *(inherited if `$extends: common-attributes`)*
   - `template: "object(isolated) -> ..."`
   - `$schema: "schema(isolated) -> ..."` *(inherited if `$extends: common-attributes`)*
   - `$state: "schema(isolated) -> ..."`
   - `default-values: "object(isolated) -> ..."`
   - `fail_fast: "null(isolated) -> ..."`
   - `sequence: "step[](required; isolated)@./sequence.yaml -> ..."`

8. **`claudine/registration/sequence.yaml` (task/group step types):**
   - `task` and `group` describe **step objects inside `sequence:`**, not top-level document frontmatter keys, so their internal fields (`name`, `when`, `with`, `default-values`, `compose`, `inline-compose`, `prompt`, `sequence`, etc.) do **not** take `isolated`. (Decision 1 adds `compose`, `inline-compose`, and `default-values` here separately.)

---

### 4.3 Existing Test & Partial Compatibility Audit

Audited `darkmatter/lib/src/markdown/compose/tests/{transclusion.rs,file_interpolate.rs,rendering.rs,frontmatter.rs}`, `darkmatter/lib/src/markdown/schemas/tests/baseline_grammar.rs`, `darkmatter/lib/tests/l1/*`, `claudine/prompts/*`, and `claudine/lib/tests/l1/*`:

- **No existing Darkmatter transclusion test breaks** when `fm.yaml` expands isolation from `[prologue, epilogue, ctx]` to the full 16-key processing-control set:
  - `transclusion.rs:493-570`: tests `author` (user key) and `replace` (explicitly kept non-isolated in `fm.yaml`) — all stay green.
  - `transclusion.rs:573`: tests `epilogue` non-propagation — stays green (now driven by `fm.yaml` via `darkmatter_baseline_isolated_keys()`).
  - `transclusion.rs:225, 2268`: tests `ignore_invalid` via `ComposeOptions::with_ignore_invalid_references(Some(true))` (a `ComposeOptions` field, not inherited frontmatter) — stays green.
  - `file_interpolate.rs:114-131`: tests `interpolate_code_blocks: true` authored on `child.md` itself and via `ComposeOptions::with_interpolate_code_blocks(true)` — never via parent frontmatter inheritance — stays green.
  - Zero tests in `darkmatter` or `claudine` expect parent frontmatter `$schema`, `hash`, `last_updated`, `change`, `style`, `content_policy`, `ignore_invalid`, `interpolate_code_blocks`, or `md` lifecycle events (`initialize`, `composed`, `failed`, `cancelled`, `finalize`) to inherit into a `::file` child.
- **Tests that WILL need updating as part of implementation:**
  1. `darkmatter/lib/src/markdown/schemas/about.rs` parity tests (`:1250`, `:1317`, `:1356`, `:1533`) — updated alongside the `about.rs` descriptor catalog additions.
  2. `darkmatter/lib/src/markdown/compose/transclusion/engine.rs:2135-2142` — direct unit calls to `inherited_child_options` take the new `isolated_keys: &BTreeSet<String>` parameter.
  3. `claudine/lib/tests/l1/claudine_schemas_embedded.rs:536` — check any assertions on exact property origins of `sequence-document.yaml` vs `composition.yaml` (under Option 1, `sequence`, `template`, `fail_fast`, `$state`, `default-values`, `kind`, `replace`, `tags`, `draft` all belong to `sequence-document.yaml`, keeping `:536` green).
  4. `darkmatter/lib/tests/l1/schema_json_key_order.rs` — verify `"x-darkmatter-isolated"` sits deterministically after `"x-darkmatter-generated"` in `atom_to_schema` and `atom_fragment_without_null_wrap` without disturbing existing key-order assertions (none of the existing assertions use `isolated`).
  5. `darkmatter/lib/tests/fixtures/recursive_schema_golden/` — uses its own frozen input copy of `fm.yaml`, so golden snapshots are unaffected.

### 4.4 `just shipped-schemas` & `PREPARATION_REVISION` Checklist

Verified `scripts/shipped_schemas.py`:
- Regenerates all 4 checked-in artifacts atomically in dependency order:
  1. `simplified-schema/lib/generated/shipped.json` (`PREPARATION_REVISION = 4` in `simplified-schema/lib/src/shipped/registry.rs:33`)
  2. `darkmatter/lib/generated/shipped.json` (`PREPARATION_REVISION = 4` in `darkmatter/lib/src/markdown/schemas/shipped.rs:64`)
  3. `claudine/lib/generated/shipped.json` (`PREPARATION_REVISION = 7` in `claudine/lib/src/composition/schema/shipped.rs:57`)
  4. `claudine/lib/generated/globals.lifecycle.json`
- Because `Constraint::Isolated` changes `simplified-schema`'s AST/lowering vocabulary, bump `PREPARATION_REVISION` in `simplified-schema/lib/src/shipped/registry.rs` (`4 → 5`) so any stale downstream artifact is explicitly rejected, then run `just shipped-schemas` from the repository root after updating `fm.yaml` and `claudine/registration/*.yaml`.

---

## 5. Summary of Spec Adjustments Recommended for `spec.md`

1. **Clarify `shared.yaml` vs. `sequence-document.yaml` ownership of `title`, `description`, `metadata` (Decision 3.3):** State explicitly that `title`, `description`, and `metadata` are marked `isolated` in `claudine/registration/shared.yaml` (and `linking.yaml`), while `sequence-document.yaml` isolates the three remaining `fm.yaml` keys (`replace`, `tags`, `draft`) plus sequence controls (`kind`, `sequence`, `template`, `$state`, `default-values`, `fail_fast`) — avoiding a `dm.host.trigger_conflict` in `darkmatter/lib/src/config/registration/merge.rs`.
2. **Document YAML sequence root isolation (`formal-sequence-file.yaml`, Decision 3.3):** Note that `formal-sequence-file.yaml` is validated via `EffectiveSchema::for_yaml_document` (without `fm.yaml` or `composition.yaml`), so `formal-sequence-file.yaml` must cover Claudine root controls (via `$extends: common-attributes@./shared.yaml` or explicit isolated declarations) and Claudine sequence propagation must union with `darkmatter_baseline_isolated_keys()`.
3. **Note `name` in `linking.yaml` / `shared.yaml`:** Explicitly confirm `name` (linked resource name) is `isolated` (blocking implicit inheritance while allowing explicit `with: { name: ... }`).
4. **Confirm `ComposeOptions::new()` baseline-free behavior:** Record that Darkmatter's transclusion isolation always unions `EffectiveSchema::isolated_keys()` with the precompiled `darkmatter_baseline_isolated_keys()` from `fm.yaml`, preserving `prologue`/`epilogue`/`ctx`/`$schema`/processing-control isolation even for library callers that do not call `.with_darkmatter_baseline_schema()`.
