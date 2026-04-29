Correlate stdout stream events with richer hook or side-channel events for providers where stdout is intentionally filtered, especially OpenCode and possibly Qwen/Gemini.


## What's needed

- A correlation strategy for deduping multiple event feeds and a wrapper/runtime design that can safely consume both channels together.

## Benefit 

Makes Claudine substantially better at permission, question, and model-routing visibility without waiting for upstream stdout changes.


