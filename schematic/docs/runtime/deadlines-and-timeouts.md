# Deadlines and Timeouts

Two bounds limit how long a send can take:

- the client's **per-request timeout**, set with `timeout(..)`
  (default `DEFAULT_TIMEOUT`, 30 s), which applies to every send; and
- an optional **shared `Deadline`**, passed through `CallOptions`, which
  bounds a whole operation made of several sends.

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

use schematic_schema::github::ListPullRequestsRequest;
use schematic_schema::prelude::*;

let deadline = Deadline::after(Duration::from_secs(20));
let options = CallOptions::new().deadline(&deadline);

let mut page = 1;
loop {
    let request = ListPullRequestsRequest::new("o", "r").with_page(page);
    let reply = client.send(request, &options).await?; // each send: min(timeout, time left)
    // … stop when the page is short …
    page += 1;
}
```

- Each send is bounded by the smaller of the time remaining and the
  per-request timeout.
- **An expired deadline refuses before sending.** Once it has passed, a
  send fails with `TransportErrorKind::DeadlineExceeded` and no request
  leaves the client.
- `DeadlineExceeded` carries `completed_requests` and `deadline_elapsed`,
  so the error says how far the operation got.

### What counts as a completed request

`deadline.completed_requests()` counts logical sends whose outcome was
classified, by any clone. A send that received an error status counts; so
does a raw send. Redirect hops inside a send, sends that failed before a
response head, and dropped sends do not count.

`Deadline` also offers `remaining()`, `is_expired()`, `elapsed()`, and
`expires_at()`.

## Streamed bodies

A codec may return a value that keeps reading the body after the send has
returned, as a stream of items does. Such a value owns the body, and the
bounds change once it switches to streaming:

- each read must arrive within the per-request timeout of the previous one
  (an idle bound), so a long stream that keeps delivering is not cut off;
- the shared deadline, when the send has one, still bounds the whole
  stream;
- the envelope's `elapsed` stops when the codec returns the value, about
  the time to the response head, not the end of the body. Every buffered
  reply's `elapsed` includes its body read.

The four built-in codecs all buffer the body, so these rules apply to
streaming codecs, which are **planned**.
