---
sequence: "@claudine/docs/research/_provider-tasks.yaml"
research_prompt: "@claudine/docs/research/sandboxing/providers/_research.md"
---

# Provider Sandboxing Research Fleet

Researches the sandboxing each supported provider supplies by default and the
additional isolation available through its own supported settings. Writes one
report per active provider to this directory, using the canonical roster's slug.

Each prompt owns freshness checks and validates its report before accepting
success. An unhandled failure stops the sequence; rerunning skips reports
confirmed against the current contract within 14 days.

From `claudine/`, run the fleet or select a provider for a pilot:

```sh
just research sandboxing/providers
just research sandboxing/providers only=claude
```

Validation requires `md` and Python 3 with PyYAML available on the run's PATH.

The reports distinguish enforced isolation from approval policy, default
behavior from optional modes, and macOS, Linux, native Windows, and WSL2.
Third-party sandbox wrappers are outside this topic's scope.
