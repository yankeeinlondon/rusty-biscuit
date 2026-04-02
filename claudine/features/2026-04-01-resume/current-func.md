# Resume in Claudine: Current State and Vision

## What resume means

Resume is one of four recovery handlers (`retry`, `resume`, `redirect`, `deviate`) in the harness module. When a composition run fails -- post-check failure, timeout, or non-zero exit -- the harness resolves a handler. If that handler is `resume`, it continues the *same agent session* rather than starting fresh.

## The mechanics (as designed)

1. **Session ID capture** -- During any non-interactive provider execution, Claudine's structured stream parser captures the provider's session ID (e.g., Claude's conversation ID, Codex's exec ID).

2. **`validate_resume()`** -- Already implemented in `harness/handlers.rs`. Checks two preconditions:
   - Provider supports resume (`resume_supported` from the agent capability catalog)
   - A session ID was captured from the prior attempt

3. **Provider-specific resume argv** -- Each provider profile builds its own CLI args:

   | Provider | Resume invocation |
   |----------|-------------------|
   | Claude | `claude -r <session_id> --print` |
   | Codex | `codex exec resume <session_id>` |
   | Kimi | `kimi --resume <session_id> --print` |
   | Qwen | `qwen --resume <session_id>` |
   | Gemini/Goose/OpenCode/Roo | Not supported -- returns `ResumeUnsupported` |

4. **Handler prompt** -- `resume` *requires* a `prompt` field (unlike `retry` which has a default). The author must be explicit about what the agent should do next.

5. **`set` overlays** -- Like retry, resume can mutate frontmatter state in-memory for the next attempt.

6. **`redirect { resume: true }`** -- Redirect can optionally continue the existing session instead of starting fresh context for the new document.

## What's implemented vs. stubbed

Per review3.md (the most recent review), resume **is now functional** -- it uses provider-specific resume argv, which was the major gap identified in review2. The remaining issues are:

- **`redirect { resume: true }` silently downgrades** to a fresh redirect when the provider doesn't support resume, instead of failing explicitly
- **Shell approval context** isn't source-aware during resume/retry loops (uses default policy root)
- **Resume metadata capture** is incomplete -- the compose-refactor spec called for preserving enough session context (provider, session ID, timestamp, file ref) to later support a standalone `claudine resume` command, but the end-of-session record is missing file-level context

## The future: standalone `claudine resume`

The compose-refactor spec explicitly calls this out as a follow-on feature. The idea is:

- Claudine would offer a `claudine resume` subcommand that lists recent composition sessions and lets the user resume one interactively (or by ID)
- This requires the metadata capture work mentioned above -- enough context from each run to present a useful recent-session list
- The current refactor is expected to *capture* the metadata, not *build the UI*

## Two distinct resume surfaces

1. **Handler-driven resume** (implemented) -- automatic recovery inside the harness loop when a document declares `handle_timeout: { resume: { prompt: "..." } }`
2. **User-driven resume** (future) -- the `claudine resume` subcommand for manually picking up where a session left off

The first is working. The second is designed but not yet built, and depends on completing the session metadata capture gap identified in the compose-refactor reviews.
