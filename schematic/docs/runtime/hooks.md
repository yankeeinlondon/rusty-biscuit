# Response Hooks

Some providers wrap their payload, rename a field between versions, or
return values you always want filtered. Response hooks let a client adjust a
successful JSON reply before your code sees it, without a hand-written
decoder. There are two, both registered on the client:

| Hook | Scope | Sees | Registered with |
| --- | --- | --- | --- |
| global JSON hook | every JSON endpoint of the client | the parsed `serde_json::Value`, before it is decoded | `pre_response_json(hook)` |
| typed mutator | one endpoint | `&mut` the decoded success value | `mutate_response::<Request, _>(hook)` |

```rust
use schematic_schema::github::{GetRepositoryRequest, RepositoryInfo};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{HookError, ResponseMeta};

let client = GitHub::new()
    // Unwrap { "data": { … } } into { … } for every JSON reply.
    .pre_response_json(|_meta: &ResponseMeta, json: serde_json::Value| {
        Ok(json.get("data").cloned().unwrap_or(json))
    })
    // Adjust one endpoint's decoded value.
    .mutate_response::<GetRepositoryRequest, _>(|_meta, repo: &mut RepositoryInfo| {
        repo.full_name.make_ascii_lowercase();
        Ok(())
    });
```

Both hooks receive the reply's [`ResponseMeta`](./metadata-and-redaction.md)
(status, redacted headers, request id). Registering a hook again replaces
the earlier one: one global hook per client, one mutator per endpoint.

## When hooks run

For a **2xx** reply to an endpoint whose codec is `Json`:

```mermaid
flowchart LR
    A["Body read"] --> B["Parse as JSON"]
    B --> C["pre_response_json"]
    C --> D["Decode the success type"]
    D --> E["mutate_response for this endpoint"]
    E --> F["Outcome::Success"]
```

Every other reply skips both hooks: non-2xx statuses (including declared
error bodies), text, binary, empty, and stream endpoints, and every
[raw send](./sending-and-responses.md#the-raw-send). The global hook
therefore only ever sees a body that parsed as JSON.

Two codecs refine this. A multi-shape endpoint runs the global hook only
when the shape chosen for the reply is JSON. A GraphQL operation's global
hook sees the whole `{data, errors}` wrapper, and its mutator sees the
decoded data only when the reply carries no GraphQL errors (see
[GraphQL](../graphql.md#decoding-a-reply)).

A **pager's page** runs the global hook but not the endpoint's mutator. The
pager reads items and next-page cursors from the body the global hook
returns, which is the body the endpoint's definition describes. The mutator
is typed by the endpoint's own response, and a page is decoded as a
`Page<Item>` instead, so it never sees one. A global hook that refuses a
page fails it with `ErrorKind::Hook` and the original bytes.

This distinction applies to `.pages()`, `.fetch_page()`, and `.collect_all()`,
including pages fetched inside a session. An ordinary `client.list_items().await`
still runs both hooks: switching it to `client.list_items().collect_all().await`
skips the typed mutator. Put transformations needed by both paths in the global
JSON hook.

## Misuse does not compile

A mutator is typed by its endpoint's success type and requires the
endpoint's codec to support hooks, which `Json`, `GraphQl`, and a
multi-shape codec whose every shape is JSON do (see
[Codecs and decoding](./codecs.md#hooks-decode_hooked-and-supportshooks)).
Registering a mutator for a text, binary, empty, or stream endpoint, for a
multi-shape endpoint with a non-JSON shape, for another API's request, or
with a closure over the wrong value type is a compile error, not a runtime
surprise.

## Refusing a reply

A hook returns `Result<_, HookError>`. Returning an error makes the send's
outcome a failure of kind `ErrorKind::Hook`, with the **original** body bytes
in `error.body`:

```rust
.pre_response_json(|_meta, json| {
    if json.get("error").is_some() {
        return Err(HookError::new()); // a 200 that is really an error
    }
    Ok(json)
})
```

`HookError` carries no text, because a message could quote the body or a
credential and the envelope's `Display` must stay safe to log. `?` on any
`std::error::Error` converts into it, discarding that error's text.

If the global hook returns a value that no longer decodes into the success
type, the outcome is `MalformedSuccess`, again with the original bytes.

## Not a middleware system

The hooks only see successful JSON replies. They cannot change requests,
retry, or observe transport errors, and the runtime has no general
middleware layer.
