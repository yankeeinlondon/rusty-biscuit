# Deadlines and Timeouts

Two bounds limit how long a send can take:

- the client's **per-request timeout**, set with `timeout(..)`
  (default `DEFAULT_TIMEOUT`, 30 s), which applies to every send; and
- an optional **shared `Deadline`**, passed to each call with
  `.deadline(&deadline)` (or to `send_raw` with `CallOptions::deadline`),
  which bounds a whole operation made of several sends.

## One expiration per send

When a send starts, the runtime computes **one** expiration: the earlier of
"now plus the timeout" and the deadline, if there is one. Everything the
send waits for uses that same instant: the first response head, every
redirect hop, and reading a buffered body.

| What expires | Before the head | After the head |
| --- | --- | --- |
| the per-request timeout | `Err(TransportErrorKind::Timeout)` | `Ok`, `ErrorKind::BodyInterrupted` with the bytes that arrived |
| the shared deadline | `Err(TransportErrorKind::DeadlineExceeded { .. })` | `Ok`, `ErrorKind::BodyInterrupted` with the bytes that arrived |
| either, between redirect hops | | `Ok`, `ErrorKind::RedirectFailed` with category `Timeout` or `DeadlineExceeded` |

Dropping a send's future cancels the work it has in flight.

## Sharing a deadline across sends

Create a `Deadline` once and pass it to every send of the operation. Its
clones share one expiration on the monotonic clock and one count of
completed sends:

```rust
use std::time::Duration;

use schematic_schema::prelude::*;

let deadline = Deadline::after(Duration::from_secs(20));

for (owner, repo) in repositories {
    let repository = client
        .get_repository(owner, repo)
        .deadline(&deadline)
        .await?; // each send: min(timeout, time left)
    // …
}
```

- Each send is bounded by the smaller of the time remaining and the
  per-request timeout.
- **An expired deadline refuses before sending.** Once it has passed, a
  send fails with `TransportErrorKind::DeadlineExceeded` and no request
  leaves the client.
- `DeadlineExceeded` carries `completed_requests` and `deadline_elapsed`,
  so the error says how far the operation got.

A pager takes the deadline once, through `.deadline(&deadline)` on the call
or on the pager, and every page request shares it. When the deadline expires
before a page's response head arrives, the pager returns
`PagerError::DeadlineExceeded` with the number of pages it received, and
`collect_all` reports `IncompleteReason::DeadlineExceeded` with their items.
No further page is sent. A deadline that expires while a page's body is
still arriving cuts that body off, so the page fails with
`ErrorKind::BodyInterrupted`.

A session scope (`with_session`) closes its session with a request that the
deadline does not bound. The close gets its own grace, 5 seconds unless
`.close_grace(..)` sets another, further limited by the per-request
timeout, so it is still sent after the deadline expired. The open and renew
requests share the deadline like pages do. The grace never gives a page
more time:

```rust
let deadline = Deadline::after(Duration::from_secs(10));
let hits = es.search(index).deadline(&deadline).close_grace(Duration::from_secs(2))
    .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
    .await;
// Pages stop at 10 s; the close may then take up to 2 s more.
```

### What counts as a completed request

`deadline.completed_requests()` counts logical sends whose outcome was
classified, by any clone. A send that received an error status counts; so
does a raw send. Redirect hops inside a send, sends that failed before a
response head, and dropped sends do not count.

`Deadline` also offers `remaining()`, `is_expired()`, `elapsed()`, and
`expires_at()`.

## Streamed bodies

A [stream codec](./codecs.md#streams-sse-ndjson-and-bytestream) returns a
value that keeps reading the body after the send has returned. Such a value
owns the body, and the bounds change once it switches to streaming:

- the per-request timeout becomes an **idle bound between items**: the next
  item must arrive within it of the previous item, or of the response head
  for the first. A stream that keeps delivering runs past it, so a stream
  yielding one item per second outlives a 2 s timeout. Bytes that trickle
  in without completing an item do not restart it;
- the shared deadline, when the send has one, bounds the whole stream;
  neither an item nor a poll extends it;
- each expiry ends the stream with one terminal item of its own:
  `StreamError::IdleTimeout` for the idle bound and
  `StreamError::DeadlineExceeded` for the deadline. It is never a
  `TransportError`, because the envelope was already returned;
- bytes the server has already sent are read even when the caller polls
  late, so a slow consumer never causes an idle timeout while data waits;
- the envelope's `elapsed` stops when the codec returns the value, about
  the time to the response head, not the end of the body. Every buffered
  reply's `elapsed` includes its body read.

```mermaid
sequenceDiagram
    participant C as Caller
    participant S as Server
    S->>C: response head (envelope returned, idle clock starts)
    S->>C: event 1 (item; idle clock restarts)
    S-->>C: partial bytes (no item; clock keeps running)
    Note over C: idle bound passes
    C->>C: Err(IdleTimeout), then None
```

Before the head arrives, the usual transport rules apply: a stalled head is
a `TransportError` with kind `Timeout` or `DeadlineExceeded`.
