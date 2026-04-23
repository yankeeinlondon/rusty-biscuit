# Combining buffered output and Hook Streams

Combine streaming runs with a buffered final artifact when the buffered mode exposes richer end-of-run metadata than the stream, such as Qwen's richer `json` result stats.

## Depends On

- A wrapper strategy that can gather richer summaries without deadlocking the subprocess contract or sacrificing live progress

## Benefit 

Lets Claudine keep live feedback while still capturing provider-specific final telemetry that is absent from the streaming path.


