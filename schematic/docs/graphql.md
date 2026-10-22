---
kind: canonical-documentation
---

# GraphQL operations

A REST API definition can also carry GraphQL operations. They share the API's
base address, authentication, error defaults, and generated naming space with
its REST endpoints. This page covers authoring them in `schematic-define`.

## Declare an operation

```rust
use schematic_define::prelude::*;

let mut ctx = DefinitionContext::types_in("::models");
let issues = GraphQlOperation::query("Issues", ISSUES_DOCUMENT)
    .describes("Open issues of a repository")
    .variables::<IssuesVariables>(&mut ctx) // or .no_variables()
    .data::<IssuesData>(&mut ctx)
    .operation_name("Issues")               // optional
    .build()?;

let api = RestApi::builder(&ctx, "GitHub", "https://api.github.com")
    .describes("GitHub")
    .bearer_env(["GITHUB_TOKEN"])
    .endpoints(endpoints)
    .graphql(AddressRule::github())
    .operations([issues])
    .build()?;
```

`describes`, `variables`/`no_variables`, and `data` are required: an operation
missing one does not compile. `GraphQlOperation::mutation(..)` starts a
mutation; the kind is documentation and an export label only, both are sent the
same way.

- **The document is opaque.** It is stored and sent exactly as written. Nothing
  parses it, so an empty or invalid document builds and the server reports the
  problem.
- **Variables** are a registered type serializing to a JSON object (a named
  type or a map; a `Vec<T>` is rejected), or none with `no_variables()`, which
  omits `variables` from the request.
- **Data** is the registered type the reply's `data` decodes as. A type from
  another definition context fails API build with `GitHub operation Issues:
  unresolved schema reference "IssuesData"`.
- **`operation_name`** is sent as `operationName` when set, the empty string
  included, and omitted otherwise.
- **Ids** must be nonempty, unique among the API's operations, and different
  from every endpoint id. Generation also checks that an operation's request
  type (`{id}{suffix}`) collides with no other generated name in its module.

## Address rules

The GraphQL URL derives from the client's base URL by ordered rules; the first
that matches wins. Rules compare whole path segments and ignore a trailing
slash, so a reverse-proxy prefix survives.

| Rule | Example |
| --- | --- |
| `AddressRule::replace_suffix("/api/v3", "/api/graphql")` | `https://ghe.example/prefix/api/v3/` becomes `.../prefix/api/graphql` |
| `AddressRule::append("/graphql")` | `https://api.github.com` becomes `https://api.github.com/graphql` |

`AddressRule::github()` is those two rules in that order. API build rejects an
empty rule list, a rule path that does not start with `/` or has an empty
segment, query, or fragment, and a base URL with a query or fragment.
Resolving the address in the generated client, and a client-builder override
of the whole URL, are **planned**.

## Errors

An operation's non-2xx replies use an error table resolved exactly like an
endpoint's (see [declared errors](responses-and-errors.md#declared-errors)):
`.errors(ErrorSet::new().on(..))` overlays the API defaults and
`.errors(ErrorSet::none())` opts out of them.
`RestApi::effective_operation_errors(&operation)` returns the table. GraphQL
`errors` inside a 2xx reply are separate and are never opted out of.

## Pagination

A connection pages through the existing [pagination model](pagination.md),
with every pager input written into the request body under `variables` and
every response path read relative to the decoded `data`:

```rust
let issues = GraphQlOperation::query("Issues", ISSUES_DOCUMENT)
    .describes("Open issues")
    .variables::<IssuesVariables>(&mut ctx)
    .data::<IssuesData>(&mut ctx)
    .pagination(Collection::at("repository.issues.nodes").item_type("Issue").paginate(
        Strategy::cursor(InputParam::body("variables.after"), CursorSource::body("repository.issues.pageInfo.endCursor"))
            .more_when(MoreSignal::has_more("repository.issues.pageInfo.hasNextPage"))
            .page_size(PageSize::new(InputParam::body("variables.first")).max(100))))
    .build()?;
```

Operation build rejects pagination that cannot work this way: a strategy that
is not declared explicitly (an operation inherits no REST default), a next-link
strategy, a follow-up or session request, alternates, a missing item type, a
pager input that is not a body path under `variables` (`pager input `after`
must be a request-body path under `variables``), and no typed variables. The
generated pager is **planned**.

## Stored form

```json
"graphql": {
  "address": [{"ReplaceSuffix": {"suffix": "/api/v3", "with": "/api/graphql"}}, {"Append": "/graphql"}],
  "operations": [{
    "id": "Issues", "description": "Open issues", "kind": "Query",
    "document": "query Issues(...) { ... }",
    "variables": {"Typed": {"Named": "IssuesVariables"}},
    "data": {"Named": "IssuesData"},
    "operation_name": "Issues"
  }]
}
```

`graphql` is omitted when the API has no GraphQL surface, `operations` when it
is empty, and an operation's `errors`, `no_declared_errors`, `operation_name`,
and `pagination` when unset. `"variables": "Unit"` means no variables.
Generating operation clients, and the OpenAPI and Postman exporters' skip
diagnostics for operations, are **planned**; today generation emits nothing
for an operation.
