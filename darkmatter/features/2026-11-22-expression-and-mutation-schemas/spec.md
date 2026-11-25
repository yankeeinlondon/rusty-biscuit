---
kind: feature
created: 2026-11-22
status: finalized-spec
area: darkmatter
owner: Ken Snyder <ken@ken.net>
related:
    - 2026-11-22-host-registration-v2
    - 2026-11-22-claudine-doctor
depends-on:
    - 2026-11-22-claudine-doctor
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
    reviewed-by: string -> the agent and model used in the spec review
    reviewed-on: date -> the date the spec was reviewed
    review-iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented-by: string -> the agent who implemented the plan
reviewed: true
reviewed-by: "codex/gpt-6.2-sol"
reviewed-on: "2026-11-22"
review-iterations: 6
needs-rulings: false
implemented: true
implemented-by: "codex/gpt-6.2-sol"
---

# Expression and mutation schemas

## Why this exists

Darkmatter already writes down its built-in expression signatures and
signature-backed mutations in [functions.yaml](../../schemas/partials/functions.yaml)
and [mutations.yaml](../../schemas/partials/mutations.yaml). Both are tagged
`kind: schema`, the same tag used for ordinary schema libraries. A host (a
program that embeds Darkmatter, such as Claudine) lists its expression catalogs
and their generated behavior files in a separate manifest field.

Give the two catalogs their own kinds so the host registration's `files` list
can identify them the same way it identifies other registered schema files.
Keep the signatures as the source for call checking and documentation, and
keep execution with the program that implements each callable. This work follows
host registration v2 and the Claudine doctor command that checks its published
files.

## What each kind describes

| Kind | What a document author can use |
|---|---|
| `expression-schema` | A value-producing call such as `kebab_case(name)` or `load_data("hosts.yaml")`. Evaluating a call may observe the environment or read a file or network resource under the existing execution rules. |
| `mutation-schema` | A signature-backed lifecycle action such as `set_frontmatter`, which can change runtime state, write a file, or send a request. Lifecycle actions can produce results too. |

The context of the call decides how it runs. Existing built-in expressions that
are also accepted as lifecycle actions keep that spelling and behavior. A new
host expression declaration makes the function available in expression positions,
including expressions in action operands, without adding a lifecycle action key.
Mutation declarations are never callable from ordinary expressions.

> **Reader note.** Darkmatter already generates much of
> [lifecycle.yaml](../../schemas/partials/lifecycle.yaml) from the two catalogs.
> Keep that file as the library of lifecycle data shapes. The existing file
> also describes control flow, shell actions, communication, and host services
> whose syntax and execution have their own rules. For example, the signature
> for a single state assignment does not describe the lifecycle `set` action,
> which accepts a mapping of destinations to values.

## Catalog format and validation

```yaml
kind: expression-schema
categories:
    - Text -> Transform text.
types:
    kebab_case: |-
        function(
            parameters(x: string | null);
            returns(string | null);
            category(Text);
        ) -> Converts text to kebab-case.
```

```yaml
kind: mutation-schema
categories:
    - Files -> Update a Markdown file.
types:
    set_frontmatter: |-
        function(
            parameters(file: string, prop: string, value: unknown | null);
            returns(unknown | null);
            category(Files);
            fallible;
        ) -> Sets a frontmatter property and returns its prior value, or null.
```

- Each file is one YAML mapping. The allowed keys are `kind`, optional
  `description`, `categories`, and `types`. Both `categories` and `types` are
  required, and `types` must contain at least one callable. Every callable must
  name a declared category, and overloads that name a category must agree on it.
  Reject duplicate keys, unknown fields, and a callable entry that mixes
  signatures with ordinary types.
- Each callable is a direct `function(...)` declaration or a union of
  signatures under one name. Keep the
  [Simplified Schema function rules](../../../simplified-schema/docs/topics/language/unions.md#function-signatures)
  for overloads, optional and rest parameters, explicit nulls, and narrowing.
  A mutation declaration cannot use `narrows(...)`, which exists to describe what
  an expression guard proves. Ordinary entries in `types` are local helpers for
  parameter and return types. Keep the current callable catalog's local named
  type resolution. Hosts may use inline examples, and Darkmatter keeps its
  existing embedded file-form examples.
- Simplified Schema ships the YAML kind definitions and checks the common
  document structure and type syntax. Darkmatter checks callable semantics and
  binding rules through the same catalog reader in registration, `md schema
  validate`, and the editor. Diagnostics point to the declaration or field that
  failed and include the other location when two declarations conflict.
- Catalogs describe callable contracts. They have no `$schema` export, cannot be
  selected as a document's schema or used as a trigger, and are not import
  libraries. The manifest's separate types-only `kind: schema` files and the
  `Name@darkmatter:lifecycle` imports continue to serve document schemas.
  Checking a catalog never evaluates examples, executes code, resolves template
  slots, or grants access to files or the network.

## Host registration and execution

### Publish and load the declarations

- The manifest moves to `format_version: 3`. Put a host's
  `expression-schema` catalogs in `files` and remove the `functions` field, including empty lists. Keep the
  published manifest path and the separate `lifecycle` field introduced by host
  registration v2. A reader and publisher must agree on the format. An
  unsupported-version diagnostic names the version read and tells the user to
  update both tools and republish. Setup republishes in place, and the Claudine
  doctor repair uses the same publisher.
- A registered host expression catalog named `project.yaml` has one required
  sibling, `project.facts.json`. Find it by that naming rule when loading the
  catalog from `files`. The sibling is part of the published inventory, like the
  lifecycle artifact, and `files` itself lists the YAML schema members. Retain
  the existing strict facts format and its check against the catalog's exact
  bytes. It carries aliases and binding rules such as values that must be
  rejected before conversion. The narrowing clause in the schema remains the
  source of what a guard proves.
- Load the companions using the same manifest folder boundary and local file
  resolution rules as the schema members. Record their paths for editor reloads,
  including expected paths when a missing companion causes refusal. Load each
  registration once per configuration revision and reuse it per document.
  A missing, stale, or invalid required companion, invalid member, or name
  conflict refuses the whole host with a `dm.host.registration` diagnostic.
- Apply the current expression name matching rules, including case-insensitive
  lookup. Reject repeated canonical names or aliases across a host's expression
  catalogs and collisions with built-in names or aliases. Each host has its own
  set, so different hosts can use the same name. Only the selected host's
  declarations apply to a document. Manifest order never decides which callable
  wins.
- **Hosts cannot add lifecycle actions in this spec.** `mutation-schema`
  describes Darkmatter's built-in actions only. A host manifest that lists a
  `mutation-schema` file is refused with a `dm.host.registration` diagnostic
  saying host-defined actions are not supported. Host services that exist
  today (such as `say` and `retry`) are unchanged. Letting a host register
  its own actions, with a runtime handler, event admission, and narrowing
  after the action, is a separate later spec.
- Build the published inventory and the `claudine registrations` listing from
  the same embedded set. Enumerate the two schema kinds and any expression
  facts companions in the inventory's file kinds. Verify the source set before
  publishing. The doctor command must recognize every published member, report
  missing or modified companions, and use its existing repair path to restore
  them.

> **Reader note.** Removing the manifest field removes the manually listed
> catalog-and-facts pairs. Keeping the generated facts file preserves checks
> that a signature alone cannot express, and lets the editor use the host's
> binding rules without running it.

### Keep the runtime boundary

- Darkmatter still implements its built-ins in Rust. Hosts bind their own
  expression handlers in their Rust process using the same catalog they
  publish. Check that every declared host expression has a handler and every
  handler has a declaration when building the host's function set. Generate the
  facts companion from that same set so the editor and the host agree on
  accepted calls. Preserve existing argument binding, eager host expression
  evaluation, and return checks. The registration supplies analysis data to
  `md` and the editor. When `md compose` evaluates a host-only expression, it
  reports that the function requires the host runtime.
- Keep the generated lifecycle artifact responsible for event names, allowed
  actions, and which globals each event sees. Keep existing expression scope,
  lifecycle action restrictions, and execution permissions. Defer `when` on
  expression, mutation, and namespace schemas together. A second source of
  event availability could let the editor accept an action the host rejects.
- Host registration v2 removes legacy schema-root host discovery, so a
  host's callable catalogs reach Darkmatter only through a registered
  manifest. A catalog file found in a schema root is never an implicit
  registration.

## Built-in migration

1. Retag the existing built-in catalogs in place. Keep the names, signatures,
   aliases, examples, and runtime rules. Update the catalog readers and shipped
   schema preparation, including their source fallbacks, to accept the new
   kinds. Regenerate built-in expression docs and shipped artifacts with the
   existing recipes.
2. Keep generating the regular YAML operand shapes in `lifecycle.yaml` from the
   parsed signatures. Preserve short and long forms, overload selection,
   deferred operands, `no_error`, and event-specific action rules. Keep the
   hand-authored lifecycle shapes and existing import names. Retagging should
   leave the generated lifecycle data contract unchanged.
3. Update the Darkmatter and Simplified Schema language and registration docs,
   the Claudine publication and doctor docs, and the corresponding skills.
   Publish caller skills whose guidance changes.
   Document the coordinated upgrade and show how a host ships a declaration
   with its runtime binding.

## Acceptance checks

- The built-in catalogs retain their signatures, aliases, and binding behavior.
  A read such as `load_data` still runs under the existing read rules, and existing
  lifecycle schemas still import and validate the same action shapes. Shipped
  artifacts match their generators.
- The editor and `md schema validate` report the same catalog errors, including an
  invalid overload, duplicate key, or case-insensitive name collision within a
  catalog. The catalog is never applied to Markdown frontmatter as a schema.
- A registered host's expression type and behavior facts reach the editor and
  passive CLI checks, and a different host or ordinary Darkmatter document
  receives neither. A stale or missing facts file refuses the profile, a repaired
  publication restores it, and only the host's runtime executes the host
  expression.
- The publisher, listing, and doctor agree on all published members. Verify the
  unsupported-version diagnostic and the upgrade in the existing install
  location.
- A host manifest that lists a `mutation-schema` file is refused, and the
  built-in actions keep their current event rules and operand checks.
