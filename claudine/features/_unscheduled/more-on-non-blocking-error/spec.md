In some cases we will get an error that the agent shares with us which is non-terminating. Recently I saw this message:

```sh
󰀨 OpenCode API failure (AI_APICallError)
Now implementing **Phase 1** (Caching & Early Exits) with a `rust-developer` subagent.
󰀨 OpenCode API failure (AI_APICallError)
⏱️ 15:29 PDT running the prompts/review-implementation.md prompt for 10m
󰀨 OpenCode API failure (AI_APICallError)
󰀨 OpenCode API failure (AI_APICallError)
⏱️ 15:39 PDT running the prompts/review-implementation.md prompt for 20m
 ← Task(successful, Implement phase 1 optimizations)

Phase 1 complete. Incrementing `starting_phase` to 2 for the next phase.
```

This one stood out as something that we really need to be able to provide more details on.

Investigate if there are any additional textual details from OpenCode on this error available on the stream or log that we're listening to. If not then can we create a lookup table with some meaningful text associated with the `AI_APICallError` id?
