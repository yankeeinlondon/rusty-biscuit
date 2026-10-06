---
area: darkmatter
status: draft-spec
created: 2026-10-04
owner: Ken Snyder <ken@ken.net>
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-05
review_iterations: 0
human_review: true
human_review_items:
    - |-
        The prerequisite this feature depends on is not built, so implementation
        cannot continue past Phase 1. This needs your decision before any further
        phase runs.

        **Why now:** Phase 1 of the plan checked whether
        `2026-09-28-recursive-schema-types` (the Darkmatter work that lets a
        named union type be used as a list item with a minimum-size rule) is
        implemented. It is not: the feature is still a draft spec with no plan,
        and a live test attempting the exact schema shape this feature needs
        (`content_policy: "policy[](min(1))@./content-policy.yaml"`) fails with
        Darkmatter's error "cannot apply `[]`/constraints to the union-typed
        named type". Without that capability, adding Content Policy's schema to
        Darkmatter's baseline cannot work — the baseline would fail to load.

        **Your options:**

        1. **Schedule and implement `2026-09-28-recursive-schema-types` first,
           then re-run this plan** (recommended).
           - Pros: this is the dependency the plan already names; its own spec
             estimates 13–20 engineer-days and it separately fixes a real
             performance problem (schema size doubling per recursion level);
             this plan then proceeds unchanged.
           - Cons: the benefit arrives later; the recursive-types work is
             substantial and touches many resolver and editor surfaces.
        2. **Reshape this feature to avoid union list items** — e.g. type
           `content_policy` as a loose `string[]`/`any[]` with no per-entry
           checking.
           - Pros: could land quickly without the prerequisite.
           - Cons: throws away nearly all validation value (unknown rules and
             actions would pass), contradicts this spec's acceptance criteria,
             and would need a new spec/plan cycle anyway.
        3. **Put this feature on hold** until the recursive-types work happens
           organically.
           - Pros: no wasted planning; the Phase 1 survey and rulings are
             already written to the implementation log and stay valid.
           - Cons: the CLI default-validation improvement stays unavailable,
             and the surveyed code map may drift in the meantime.

        I recommend option 1: the Phase 1 survey, rulings, and accessor maps in
        `implementation-log.md` were written so the next run can begin at Phase
        2 immediately after the prerequisite lands.
depends-on:
    - 2026-09-28-recursive-schema-types
related:
    - 2026-09-28-content-policy
message_to_agent: >-
    Phase 1 completed as a gate check and the gate FAILED:
    2026-09-28-recursive-schema-types is not implemented (draft-spec, no plan;
    TypeExpr has no Ref variant; lowering emits no $defs; resolve.rs:1326
    rejects `[]` over a union-typed named type — a live probe of the exact
    content_policy reference failed with that error). Do NOT start Phases 2-6.
    Read implementation-log.md first: it holds the baseline accessor/caller
    table, the import-resolver seam map (R2/R3 injection points:
    mod.rs:148 baseline entry has no import expansion today;
    ImportEngine::resolve_namespace at resolve.rs:1141 is the @file-to-disk
    seam; NamespaceKey::File(canonical_path) is the definition identity;
    skip dependencies insertion for embedded namespaces), the content-policy
    constraint confirmation (add with default-features = false; deps-check
    guards the no-reverse-dependency rule; darkmatter/docs/dependencies.md has
    no entry yet), and rulings R1-R8. When the prerequisite lands, re-verify
    with the probe shape (name[](min(1)) loads, validates, survives
    merge_baseline, visible in the typed view) before Phase 2.
---

# Compile Content Policy's Schema into the Base Schema

Darkmatter should check a Markdown document's `content_policy` list without
requiring the author to declare `$schema`. These checks should work in
`md schema validate`, `md compose`, and DMLS, including installed binaries
running outside this repository.

The baseline is the schema inherited by default; a document can add or replace
property declarations through its own `$schema`. This work waits for
`2026-09-28-recursive-schema-types`, the Darkmatter feature that allows a
named union type (a type with alternative shapes) to be used as a list item.
Content Policy needs that capability because each entry is either a compact
rule string or a mapping containing `rule` and `action`.

The `content-policy` package exports
[`EDITOR_SCHEMA`](../../../content-policy/lib/src/lib.rs), the compiled-in
text of its editor schema. Using this constant keeps the schema version aligned
with the parser Darkmatter links. The caller-owned adoption decision in the
**Decided** section of `2026-09-28-content-policy` assigns this work to
Darkmatter and intentionally changes the CLI's validation default.

For example, this ordinary document needs no schema declaration:

```yaml
---
last_updated: 2026-10-04
content_policy:
  - ValidFor(3mo)
  - rule: ValidUntil(2027-01-01)
    action: archive
---
```

Its declaration passes the editor checks. Replacing a rule with
`Duration(3mo)` or the action with `delete` produces a validation problem.
Passing these checks does not establish whether the document is fresh.

## Decided

- **Use the exported artifact.** Add `content-policy` to the Darkmatter
  library with `default-features = false`, without enabling `file-adapter`.
  Content Policy must continue to have no dependency on Darkmatter, including
  with all features enabled. Do not copy its schema or embed its source path
  directly from Darkmatter production code.
- **Type a non-empty list in the runtime baseline.** In the baseline loaded
  today from [darkmatter.yaml](../../docs/schemas/darkmatter.yaml), add:

  ```yaml
  content_policy: "policy[](min(1))@../../../content-policy/schemas/content-policy.yaml"
  ```

  The minimum applies to the list, not each rule string. An empty list is an
  invalid Content Policy declaration. The property remains optional, with no
  default: missing `content_policy` does not insert a policy or `null` during
  composition. Preserve Darkmatter's existing optional-property treatment of
  explicit `null`; the policy parser rejects a null declaration. This is a
  documented difference, not a change to Darkmatter's null convention.
- **Use the built-in baseline at default validation entry points.** The CLI
  change applies to documents both with and without `$schema`. Existing
  library callers that attach the Darkmatter baseline receive the new typing.
  Keep the deliberately unconfigured library constructor
  [`DarkmatterSchemas::new`](../../lib/src/markdown/schemas/mod.rs), which
  callers use to select their own baseline or none. Do not silently attach the
  default to explicitly configured validation APIs.
- **Keep schema precedence.** A custom baseline replaces the built-in one;
  matching triggers and a document's schema retain their existing precedence.
  If a document redefines `content_policy`, its declaration wins. The baseline
  is a default contract, not an unoverrideable enforcement layer.
- **Keep validation passive.** Import preparation may resolve declared schema
  dependencies, but policy checks must not evaluate freshness, execute
  expressions, read watched files, fetch URLs, stamp `last_updated`, or renew
  content. The standard key `content_policy` is the only key added here;
  configurable policy keys and date properties remain Content Policy caller
  options.

> **Reader's note:** The authored global catalog at
> [darkmatter/schemas/darkmatter.yaml](../../schemas/darkmatter.yaml) is a
> different artifact from the runtime baseline. Updating only that catalog
> would leave default document validation unchanged. Similarly, making the
> low-level unconfigured constructor attach a baseline would change callers
> that intentionally validate against another schema. This design changes the
> default entry points while preserving explicit configuration.

## Embedded schema resolution

The embedded baseline currently parses its text without expanding imports.
Adding the policy reference alone therefore fails conversion and can panic
when DMLS first loads the cached JSON Schema. Both the typed schema used for
completion and the compiled schema used for validation must be fully resolved.

Use a small internal table of embedded schema documents, with stable,
repository-relative virtual paths:

| Virtual path | Embedded text |
| --- | --- |
| `darkmatter/docs/schemas/darkmatter.yaml` | Darkmatter's runtime baseline |
| `content-policy/schemas/content-policy.yaml` | Content Policy's `EDITOR_SCHEMA` |

Resolve the baseline's relative import and the policy schema's same-file
references against this table. Reuse the normal named-type parser, import
expansion, and lowering rules; do not create a second grammar. The table is an
internal source for shipped schemas, not a new public file-reference scheme.

- Resolve virtual paths lexically with portable `/` separators. Do not
  canonicalize them as host files, use the process's current directory, or
  require a repository checkout.
- An embedded import must stay within the table's virtual root and name a
  registered document. Missing documents, missing types, and malformed imports
  are errors; never fall back to disk or the network. Apply the prerequisite's
  cycle and definition rules rather than introducing another cycle policy.
- Resolve and lower once per process, preserving the existing shared caches.
  Carry any referenced definitions through baseline merging and DMLS's typed
  schema view, including schema unions. Virtual definition identities must not
  collide with document or trigger definitions resolved from disk.
- Embedded entries are immutable build inputs. Do not add their virtual names
  to filesystem dependency watchers or report them as existing source files.
  Problems in document values must retain baseline origin, the offending
  instance path, and available document source positions.
- Loading the authored baseline from disk through the normal resolver must
  produce equivalent validation behavior and type information. User-supplied
  schema references continue to use Biscuit File's `FileReference` and the
  request's captured resolution context; the embedded table does not intercept
  them.

The **Embedded Schema Reference** findings under `2026-09-28-content-policy`
already demonstrate installed-binary operation and a prototype of about 110
lines. That prototype used a non-union placeholder, so it does not prove the
real policy list or required-field behavior. No additional performance spike
is required: integrate the real artifact and verify cache reuse through tests.

## CLI defaults and compatibility

Update the Darkmatter CLI's
[`load_api`](../../cli/src/commands/schema/validate.rs), which builds the
validator for `md schema validate`, and its argument definition. Add
`--no-baseline-schema` and honor `DARKMATTER_NO_BASELINE_SCHEMA` from the
request snapshot. Use the disable values already accepted by compose:
`1`, `true`, `TRUE`, `yes`, and `YES`.

Choose the baseline in this order:

1. `--no-baseline-schema`: disable baselines, including `BASELINE_SCHEMA`.
   Make it conflict with explicit `--schema` as a CLI usage error.
2. `--schema <path>`: use that custom baseline, even when the disable
   environment variable is set.
3. A truthy `DARKMATTER_NO_BASELINE_SCHEMA`: disable the default and ignore
   `BASELINE_SCHEMA`.
4. `BASELINE_SCHEMA`: use that custom baseline.
5. Otherwise, use the embedded Darkmatter baseline.

Read environment values from the captured request, not a new process lookup.
Resolve custom paths in the launch context and document references in their
document context. A selected custom baseline that cannot load remains an
error; it must not fall back to the built-in baseline.

Disabling the baseline does not disable the document's schema or triggers.
`--no-trigger-schemas` keeps its separate existing meaning. Success without
schema checks means no effective schema remains, not merely that `$schema` is
absent. Keep the JSON output fields and exit codes: `0` for valid, `1` for validation
failure, `2` for schema-load failure, and `3` for document-open or parse failure.
The existing JSON `schema` field identifies a document's string schema
reference; it may remain null when validation used the built-in baseline.
Pretty output must not describe such a document as having no effective schema.

> **Intentional compatibility change:** A document with
> `last_updated: not-a-date` and no `$schema` currently passes this CLI command
> when no custom baseline is selected. It will fail after this feature, as it
> already does in default compose. All existing baseline fields become active,
> not only `content_policy`. Document the opt-out for callers that need the
> former behavior, update assertions that depend on validation without a schema,
> and keep tests for that behavior when explicitly requested. Do not
> loosen the baseline to preserve the old default.

## Schema coverage and parser agreement

Content Policy's
[`Policy::from_declaration`](../../../content-policy/lib/src/model.rs)
parses a policy list and is the authority for declaration validity. Its editor
schema deliberately checks a subset. Test a shared collection of complete
declarations, not isolated strings placed in unrelated properties.

For cases the schema claims to express, the parser and validator must agree.
For documented limitations, record both expected outcomes explicitly so that
new differences fail the test instead of being silently accepted. Cover:

| Cases | Expected contract |
| --- | --- |
| Each supported rule, legal actions, and a mixed compact/long-form list | Both accept; test `Evergreen` alone because the parser forbids combining it with other rules |
| `Duration(3mo)`, invalid units or capitalization, zero/leading-zero durations, nested property references, missing action, unknown action, non-null values that are not lists, empty lists | Both reject |
| Impossible calendar dates, duration overflow, unsupported path prefixes such as `vault:` or URLs, and `{{` inside a path | Parser rejects; editor patterns may accept, as documented |
| `Evergreen` with another rule, extra long-form fields, explicit null, and nullable values allowed by existing schema semantics | Parser rejects; preserve and document the editor schema's existing limits |
| Missing `rule` | Parser rejects; schema outcome depends on the required-field decision below |

Use real YAML shapes, including block lists and quoted flow-list rules with
commas. Validate declaration syntax without providing evidence or a clock;
missing baseline dates or fingerprints belong to evaluation, not these checks.
Do not expand this feature into exact parser enforcement or a new regex
language. Any corrected schema pattern remains owned by Content Policy and
ships through its exported constant.

## Work

1. After `2026-09-28-recursive-schema-types`, verify that union-typed named
   types support list usage, list-level minimum constraints, and their
   definitions survive baseline/trigger/document merging. That prerequisite
   explicitly excludes changes to reused-type requiredness; do not assume it
   resolves the open question below.
2. Add the dependency and embedded source table, expand the actual shipped
   schema, and update the runtime baseline. Ensure every baseline accessor
   returns the resolved representation needed by its callers. Preserve the
   existing public signatures where the prerequisite permits; record any
   necessary API departure and its caller impact in the implementation log.
3. Implement the CLI selection rules, including argument conflicts, captured
   environment access, output, and compatibility tests.
4. Resolve the required-field question before promising missing-rule
   diagnostics. Update the shipped schema and its tests if that work is
   selected; otherwise keep the limitation explicit.
5. Extend
   [the editor-schema tests](../../lib/tests/l1/content_policy_editor_schema.rs)
   to use the default policy list, add the declaration comparison cases, and
   update existing baseline corpus tests that currently parse and convert the
   authored baseline without import expansion. Keep passive shipped-artifact
   coverage as well as normal-entry-point coverage.
6. Update Darkmatter's schema definition, baseline contract, CLI documentation,
   and DMLS schema-support documentation. Update the affected READMEs,
   `darkmatter/docs/dependencies.md`, the repository dependency catalog, and
   the Darkmatter and Content Policy skills. Remove base-schema **planned**
   wording from Content Policy's README and policy-lifecycle page only when
   the adoption works, retaining any remaining parser/schema limitations.
   Current topic pages must explain the behavior without referencing this
   feature directory.

## Acceptance criteria

1. A schema-free document containing the example above passes library baseline
   validation, `md schema validate`, `md compose`, and DMLS. An ordinary
   document with no policy remains valid and gains no policy property.
2. The same entry points reject an unknown rule, unknown action, empty list,
   and wrong list shape. Diagnostics identify the offending list item or
   field, retain typed problem information and baseline origin, and use the
   available source positions. DMLS also completes supported rule forms and
   actions and provides hover information inside list entries.
3. A library baseline test and subprocess tests for the built `md` and DMLS
   run outside the repository with only the test documents available. Baseline
   validation and DMLS completion work without schema files on disk or a
   schema environment override. A second schema load reuses the cached
   resolved baseline.
4. Disk-resolved and embedded copies of the real schema have equivalent
   behavior and type information. Tests cover nested same-file imports,
   missing embedded documents/types, invalid virtual paths, and definition
   isolation when a document or trigger supplies its own schema.
5. Tests prove the CLI precedence above, including a broken custom baseline,
   explicit opt-out, environment opt-out, document/trigger validation after
   opt-out, and a document schema that replaces `content_policy`. JSON fields,
   quiet behavior, and existing failure exit codes remain compatible.
6. The shared declaration cases distinguish agreement from each documented
   schema limitation. Validation leaves document bytes and frontmatter
   unchanged, performs no policy evidence reads, and does not evaluate or
   renew the policy. Compose retains its established pending-value handling.
7. Use the repository Test Toolkit and existing subprocess fixtures for
   isolated documents and environment settings. Run `just test` and `just lint`
   in Darkmatter and Content Policy; these are nextest-based unit and process
   tests. No new real-terminal or browser tier is needed for schema behavior,
   and tests must not activate editor or terminal windows. The virtual source
   logic and subprocess fixtures must be portable to macOS, Linux, native
   Windows, and WSL2; use the existing OS coverage rather than adding CI cells
   or performance workloads for this feature.

## Open Questions

### Should this feature also require `rule` in long-form entries?

Content Policy requires both `rule` and `action`. Its shipped editor schema
currently omits `(required)` on `rule` because Darkmatter rejects constraints
on the union-typed named reference. The recursive-types prerequisite promises
array support but explicitly excludes reused-type requiredness. Leaving this
conditional in the work list would make the delivered contract unpredictable.

- **Add scoped required-field support for a union reference (recommended).**
  Honor `(required)` where `long_form.rule` references `short_form`, without
  changing whether required markers inside a reusable type are inherited.
  **Pros:** expresses the parser's contract at the property that needs it;
  avoids duplicating rule patterns; can reject both missing and null rules.
  **Cons:** adds resolver/lowering work beyond the prerequisite and needs
  tests through named imports, list items, validation, and DMLS.
- **Write `rule` as an inline union with required arms.**
  **Pros:** can use existing inline-union requiredness without a general
  reference change. **Cons:** duplicates the compact-rule schema or requires
  generating it, increasing maintenance and drift risk; must still prove
  that the requirements survive the enclosing named-type import.
- **Defer the missing-rule editor check explicitly.**
  **Pros:** keeps this adoption focused on embedding and defaults; the parser
  remains authoritative. **Cons:** editor validation accepts incomplete
  long-form declarations, and documentation/tests must keep this limitation.

Recommend the first option because it makes presence a constraint on the
referencing property, matching existing schema authoring conventions, while
preserving the separate contract for reused-type markers. The author should
confirm this added scope before implementation; until then, missing-rule
rejection is not an unconditional acceptance criterion.
