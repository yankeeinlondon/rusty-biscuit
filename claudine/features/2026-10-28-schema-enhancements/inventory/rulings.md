# Phase 1 Rulings Needed

Each item below is a contract that `spec.md` and `plan.md` leave open, or that existing code contradicts. Each one needs an **author ruling** before the dependent task starts. The recommendations are the inventory agent's; they are not approvals.

**Status: all rulings are pending author review.** Evidence paths are listed in `ownership.md` and `reader-matrix.md`.

## R1. Criterion 10 fixtures for void functions and preservation-only parameters (plan rule 11)

**Question.** Criterion 10 (spec.md:1177) says every registered function must have three fixtures that run through normal dispatch:

- accepted,
- rejected,
- converted-input.

Some functions have no argument that can be converted, so no converted-input fixture can exist for them. How should those functions satisfy the criterion?

**Evidence.**

- **Void functions.** The draft catalog has one: `branch_exists_on_remote` uses `parameters(void)` (`claudine/docs/schemas/partials/functions.yaml:668-671`). Spec item 10 (spec.md:264-270) makes `void` the only spelling, so any function normalized from `parameters()` joins this group.
- **Functions that take only `any` today.** The current catalog has 113 functions; 44 of them take only `any`:
  - `is_*` and the date predicates
  - `first`, `last`, `contains`, `length`, `number`
  - `as_*`
  - `ensure_*`
  - `file_exists`
  - `and`, `or`
  - `pr`, `cicd`
- **What becomes of them.** Spec decision 10 and catalog item 10 (spec.md:422) narrow accommodation-only `any`. The predicates keep `unknown | null` (plan rule 3), so they still convert nothing.
- **What can be tested.** A rejected fixture can exist for void functions (arity) and for `unknown` parameters (a wrong argument count). Spec decision 82 also keeps `file_exists(42)` returning `false` without conversion.

**Options.**

1. **Registry-wide fixture row that records the impossible cell** (the plan's recommendation).
   - The converted-input cell is marked `impossible: void` or `impossible: preservation`, with a reason.
   - The test still sends candidate conversion inputs (for example `"4"` and `4`) through normal dispatch, and asserts that each is delivered unchanged, with the counter at 1.
   - Conversion itself is covered by test-registered probes.
   - *Pro:* the criterion's intent (every function binds through the engine, with no handler-local conversion) is proven, and no signature is weakened.
   - *Con:* the criterion's literal wording is not met for about 40 functions; the author has to accept that.
2. **Manufacture conversions** by narrowing parameters (for example `unknown` to `string`).
   - *Con:* this changes behavior, contradicts criterion 18 and decision 82, and breaks the predicates. Reject this option.
3. **Exclude these functions from criterion 10.**
   - *Con:* this weakens coverage and is explicitly forbidden by rule 11.

**Recommendation.** Option 1.

- The ledger records each function's applicability class: `convertible`, `void`, or `preservation-only`.
- Void functions get accepted and rejected (arity) cells only.
- Preservation-only functions get an "unchanged-delivery" cell in place of the converted-input cell.
- Until the author signs off, criterion 10 stays **blocked**.

## R2. Duplicate and empty names in `categories:`

**Question.** Is a `categories:` list valid when it contains two entries with the same name? When an entry's name is empty (for example `- ""`, `- " -> desc"`)? When the list itself is empty (`categories: []`)?

**Evidence.**

- The spec (spec.md:370-405) says:
  - a name is the trimmed raw text and is case-sensitive;
  - an undeclared `category(X)` is a load error when a list exists;
  - documentation shows categories in list order.
- Nothing addresses duplicate or empty names. I checked:
  - `spec.md`;
  - `drafts/schema-definition.yaml`, which has no `categories` key;
  - `claudine/docs/schemas/schema-definition.yaml`;
  - `darkmatter/docs/topics/schemas/definition.md`, whose only "categories" mention (:2047) is about lifecycle verbs.
- Related precedent:
  - The old function catalog rejects an empty `category` (`EmptyValue`) and duplicate return literals.
  - Spec grammar rejects duplicate singleton constraints and duplicate parameter names (spec.md:254).
  - The draft catalog's 22 names are unique and non-empty.

**Options.**

1. Reject both duplicates and empty names, and keep list order.
2. Allow duplicates by merging them (first occurrence decides the position).
   - *Con:* there is no sensible rule when two copies carry different descriptions.
3. Silently drop empty names.
   - *Con:* this is exactly the "filtering" the matrix forbids.

**Recommendation.** Option 1.

- An empty name (after trimming) is a schema-load error.
- A duplicate is a schema-load error at the second occurrence. Two names are compared after trimming and case-sensitively, consistent with the name rule. "Math" and "math" are therefore different names. (Alternative for the author: compare case-insensitively to catch near-duplicates.)
- A duplicate description that differs counts as a duplicate too.
- The matrix's "Empty" cell is unaffected: `categories: []` declares an empty list, and any `category(X)` that uses it is an error.

## R3. Duplicate keys in JSON readers (plan rule 9)

**Question.** Should each JSON reader this feature changes reject duplicate keys?

**Evidence, per reader:**

| Reader | Today | Changed by | Depends on last-wins? |
|---|---|---|---|
| Raw schema `.json` (`resolve.rs:1150`) | last-wins (probe) | Wave 4 Schema adapter | No consumer or test found |
| `json` content format (`format.rs:260`) | accepted | Wave 4 custom formats | No |
| DMLS editor settings (lsp-server transport) | last-wins **pinned** by `dmls/tests/l1/workspace_formatting_settings.rs:1001` (`duplicate_json_member_names_are_resolved_before_the_loader`) | Wave 8 (field only; transport unchanged) | **Yes.** It is an established protocol contract, and duplicates cannot be detected after decoding |
| `md set` value, `--state`/`--set`, shorthand, `::file set=`/`replace=` | last-wins | Only if the Wave 2 numeric seam reroutes them | No test depends on it. The nearest precedent goes the other way: a repeated `set.NAME=` on one directive is an error by default (`context/options.rs:1319`), and `schema_triggers.rs:589` asserts duplicate key means load error |

Both YAML readers (frontmatter and schema) already reject duplicates. JSON (RFC 8259) says names SHOULD be unique; it does not require it.

**Options.**

1. Reject duplicates in every changed JSON reader except the DMLS transport.
   - This needs a duplicate-detecting deserializer, a small custom `Visitor` over `serde_json`'s map access.
   - Spans are preserved by parsing the raw text, never by inspecting a `Value`.
2. Keep last-wins everywhere. This is inconsistent with YAML and silently changes meaning.
3. Reject duplicates only in the raw schema reader.

**Recommendation.** Option 1, applied as follows:

- **Reject** duplicates in the raw schema `.json` reader and in the `json` content format. Record the `json` format change in the behavior ledger, because it is user-visible for frontmatter text values.
- **Leave the DMLS transport alone.** Last-wins is the established reader policy there, it is pinned by test, and the cell is transport-owned.
- **Leave the CLI and transclusion Json5/JSON surfaces alone** unless the numeric seam is routed through them. If it is, reject duplicates there too, and get a separate author ruling on whether malformed JSON in `md set` and the shorthand still falls back to a literal string.
- **Fix the error message.** Duplicate keys and trailing content in the raw reader currently report `AmbiguousReferenced` ("ambiguous schema reference"). They should report a parse error with location and reason.

## R4. "workspace/configuration" in the spec versus the transport DMLS actually uses

**Question.** Criterion 40 and decisions 58 and 62 (spec.md:1002, 1129, 1133, 1224) say the setting is overridable through LSP `workspace/configuration`. DMLS never sends that request, by documented design (`darkmatter/docs/lsp/configuration.md:119`). It reads only `didChangeConfiguration` (`router.rs:2581`). Which transport should the work use?

**Options.**

1. Read the spec as meaning "the existing LSP configuration channel" (decision 62's own wording), which is `didChangeConfiguration` under `dmls`. Test exactly that.
2. Implement a `workspace/configuration` pull.
   - *Con:* new protocol behavior, capability gating, and timing.
   - *Con:* outside the feature's scope (plan rule 9: do not add formats or transports to fill a matrix).

**Recommendation.** Option 1. Record the interpretation in the implementation log, and phrase criterion 40's evidence as "`.dmls.toml` and the editor settings (`didChangeConfiguration`)".

## R5. YAML envelope cells that contradict documented behavior

**Question.** The plan's YAML table asks for E in three places where the code currently does something else. Where does E apply?

**Evidence, per cell:**

- **`kind` absent, null, or wrong.** The plan says "catalog load E". Today the classifier *declines*, the file is read as a raw JSON Schema, and it loads. This is documented and pinned (`lib/tests/l1/schema_yaml_spellings.rs` `TAGGED_CELLS`).
- **Named type explicit null.** The plan says E. Today `types.x: null` (or `~`, or an empty value) means the **`null` type** (`darkmatter/docs/topics/schemas/definition.md:666`, `simplified/mod.rs:321-324`).
- **Root `$schema` absent.** The plan says E. Today the file declines and is read as a raw schema (documented).

**Recommendation.** Keep all three current behaviors for general schema files, because they are documented contracts. Apply E only through the **explicit function-catalog loader** (Phase 4 Catalog replacement), which knows it requires `kind: schema` and a registered canonical. There:

- `kind` absent, null, or wrong is E;
- `types.<canonical>: null` is E ("not a function signature");
- a missing canonical is E.

Matrix tests assert those cells through catalog loading, not through the classifier's `Option`, as the plan requires.

## R6. Unknown keys in `[expressions]`

**Question.** Should an unknown key inside `[expressions]` be an error? A misspelling such as `strcit = true` is currently ignored silently, so strict stays false.

**Evidence.** Outside `[formatting]`, an unknown key is "Ignored, silently" (`configuration.md:73`), and no `deny_unknown_fields` exists on any section.

**Options.**

1. Follow the existing policy and ignore it silently. Consistent with every other section.
2. Use `deny_unknown_fields` on `ExpressionsConfig`. The whole file would then fail and fall back to defaults, which is harsher than the typo deserves.
3. Report a warning diagnostic and keep the rest of the file.

**Recommendation.** Option 1 for this feature, matching the documented policy and Rule 3. Record it in the matrix as an explicit cell, so it is not left implicit. If the author wants unknown keys reported, option 3 should be a repo-wide change rather than one specific to `expressions`.

## R7. Raw JSON Schema "Empty" cells that the Draft 2020-12 meta-schema decides the other way

**Question.** Two cells in the plan's tables disagree with the Draft 2020-12 meta-schema. Should the tables follow the meta-schema?

**Evidence (probes):**

- `prefixItems: []` fails when the validator is built. The meta-schema's `schemaArray` has `minItems: 1`. The plan says "empty list valid".
- `enum: []` loads, because the 2020-12 meta-schema has no `minItems` on `enum`. Every instance then fails validation. The plan says "[] E".

**Recommendation.** Follow the validator, per plan rule 7 (raw schemas are authoritative and fully validated):

- `prefixItems: []` becomes **E**.
- `enum: []` becomes **V**: the schema loads, and every instance fails.

Do not add Darkmatter-specific checks on top of the meta-schema. Update the two cells in the expanded matrix.

## R8. Other unresolved contracts found (for the author to note)

1. **Ownership gaps.** Neither `schemas/resolve.rs` (the raw loader) nor `schemas/mod.rs` (the `EffectiveSchema` wiring) is assigned to any task. The proposal:
   - Schema adapter (Wave 4) owns `resolve.rs`.
   - The integrator owns `schemas/mod.rs`.

   Moving the 27 context handler bodies out of `functions/mod.rs` before Wave 5 is a move-only integrator step (see ownership.md section 3).
2. **Envelope keys of a schema file.** Spec §Function Catalog Migration item 7 says a schema file holds only `kind`, `categories:`, and `types:`. The global catalog (`catalog.rs`) uses `kind`, `description`, and `$schema`, so that sentence must be read as covering function-catalog and type files only. The global catalog keeps its own envelope.
3. **"Current `md frontmatter` output".** No `md frontmatter` subcommand exists. The plan's fixture source should be read as `md get --yaml` and `md set`.
4. **Invalid editor overlay throws away every section.** An invalid `expressions.strict` in the editor overlay discards *all* non-formatting overlay sections (`config/mod.rs:583-596`). That is the existing behavior; keep it unless the author wants per-section isolation, which would be a new contract.
5. **Silent numeric losses.** These are recorded for the numeric-ingress spike, not ruled here:
   - YAML `.inf` becomes null;
   - `1e-400` becomes 0;
   - an integer above u64 fails the whole frontmatter;
   - `coerce.rs:597` rounds silently, and its comment is wrong;
   - type-expression numbers (`grammar.rs:279,1948`) are parsed with a bare `parse::<f64>()`.
