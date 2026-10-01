# Compose Requests

Every Darkmatter call that resolves a file reference runs inside a **compose
request**. The request fixes, once, where `./`, `&`, `^`, `@`, `~`, and
`{{VAR}}` references start and which repository they belong to. Every phase of
the request (validation, pre-flight, composition, every transcluded child)
then resolves against that same answer.

## Preparing a request

A request is built from two things:

- **`ComposeOptions`**: what to do (operations, overrides, shell and remote
  policy).
- **`RequestSnapshot`**: where the request runs. It names the request
  directory and carries the home directory, the environment, any extra `@`
  search roots, and the reference that opened the document.

```rust
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};

// A binary reads its own process once, at its entry point.
let snapshot = RequestSnapshot::from_process()?;
let request = ComposeRequest::prepare(ComposeOptions::new().with_source_file("docs/guide.md"), &snapshot)?;

let md = Markdown::try_from(std::path::Path::new("docs/guide.md"))?;
let preflight = md.compose_preflight(&request)?;
let (composed, report) = md.compose_with(&request)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

A library never reads the process. `RequestSnapshot::new(dir)` starts with no
home directory and an empty environment, so a library call states every input
it depends on:

```rust
use std::collections::HashMap;
use darkmatter::markdown::compose::RequestSnapshot;

let snapshot = RequestSnapshot::new("/work/project")
    .with_home(Some("/home/me".into()))
    .with_env(HashMap::from([("NOTES".into(), "/home/me/notes".into())]));
```

`snapshot.at_request_dir(dir)` keeps the home directory, environment, and
extra roots but anchors the request elsewhere, for example at a document that
lives in another repository.

```mermaid
flowchart LR
    P["RequestSnapshot::from_process()\n(binaries only)"] --> S[RequestSnapshot]
    N["RequestSnapshot::new(dir)\n(libraries, tests)"] --> S
    S --> B["build_resolution_context\n(discover, scope, validate)"]
    O[ComposeOptions] --> R
    B --> R["ComposeRequest\n(required context)"]
    R --> V[reference validation]
    R --> F[pre-flight]
    R --> C[compose]
```

## What the builder does

`build_resolution_context(&snapshot)` is the one way a request obtains a
`FileResolutionContext`. In order, it:

1. Takes the request directory, home directory, and environment from the
   snapshot.
2. Discovers the repository containing the request directory, and its
   packages and package areas. These become the `&` and `^` anchors and the
   launch `@` scope.
3. Adds the snapshot's extra `@` roots (see [magic paths](./magic-paths.md)).
4. Derives the context for the opening reference, when there is one, so a
   document opened as `~/notes/x.md` keeps `~` as its tree root.
5. Validates the result.
6. Logs one `debug` event naming the request directory and where the tree
   root came from (`base_dir_origin`), so a "file not found" report can be
   traced to the context it was resolved in.

`ComposeRequest::prepare` calls the builder. A caller that already holds a
built context (for example one derived for a document in another repository)
uses `ComposeRequest::with_context(options, context)`, which validates it.

## When a request cannot be built

The builder returns `ContextBuildError` instead of letting a bad context fail
later as a missing file:

| Situation | Error | `resolution_failure()` |
|---|---|---|
| The request directory is relative (`RequestSnapshot::new("docs")`) | `Invalid` (`RelativeContextDirectory`) | `MissingContext` |
| The opening reference resolves outside the request's repository (`~/notes/x.md` opened from inside a repository) | `Invalid` (`RepositoryRootNotContainingSource`) | `MissingContext` |
| Repository discovery fails, for example a corrupt `.git/config` | `Discovery` | `MissingContext` |

Finding no repository is not an error: the request directory becomes the
tree root. `resolution_failure()` uses the same classes as biscuit-file's
`FileReferenceError::resolution_failure()`.

## One environment per request

`{{ env.NAME }}`, `ctx.agent`, `ctx.model`, and a `{{NAME}}/file.md` file
reference all read the snapshot's environment. An expression and a file
reference in one request therefore always agree, even if the process
environment changes or differs.
