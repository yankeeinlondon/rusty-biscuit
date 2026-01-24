# Implementing AI Pipeline

We have had some design reviews but we've not really put into place a fully implemented version of the `ai-pipeline` crate.

## Top-level Features

- `ai-pipeline/gen`
    - updated to leverage the **OpenAI** API definition found in `schematic/define` rather than it's own bespoke calls to this API
    - we have updated the deps since our last build and both `rig-core` and `rmcp` have been updated
        - We need to review the changelog notes below and make sure that any design changes/optimizations imposed by these changes are accounted for
- `ai-pipeline/lib`
    - Full design review of the  to make sure it's fully ready for a production setting
    - Implementation of the library and comprehensive testing
- `ai-pipeline/cli`
    - Implementation of the CLI which can take JSON variants of a "pipeline" and then execute them using the `lib` module
- `ai-pipeline/tui` this will remain as future work

### Library Module

- read the @ai-pipeline/README.md for broad context on what we're doing
- consider the updates to packages [below](#crate-updates)
- make sure that the following are fully documented, implemented and tested:
    - Atomic Operations
        - `UserContent`
        - `Prompt`
    - Interactions
        - `TextInput`
        - `EnumInput`
        - `NumericInput`
        - `Confirmation`
    - Agent Delegation
        - `ClaudeCode`
        - `OpenCode`
    - Operators
        - `Pipeline`
        - `Parallel`

## Crate Updates

### rig-core 0.28.0 → 0.29.0

New Features:

- Agent names in tracing - Agent names now included in tracing spans for better observability
- DeepSeek reasoning content support (non-streaming)
- Reqwest client re-export - Easier access to the underlying HTTP client
- Default max depth for agents - Agents now have a configurable default max depth
- OpenAI embedding improvements - Now supports user and encoding_format parameters
- Agentic loop early termination tracking - Better visibility into why agent loops end
- Dynamic tool addition - New AgentBuilder::tools method for adding tools dynamically
- Custom vector store backend example
- Vector store filter ergonomics (breaking change)

  Bug Fixes:

- Fixed CancelSignal cancellation and reason sharing issues
- Fixed blank base URL handling (no longer prepends forward slash)
- Fixed Gemini streaming functionality

  Sources: https://crates.io/crates/rig-core, https://github.com/0xPlaygrounds/rig

### thiserror 2.0.17 → 2.0.18

Bug Fix:

- Made compatible with project-level needless_lifetimes = "forbid" lint setting
- Ensures macro-generated code doesn't trigger this compiler restriction

  Sources: https://github.com/dtolnay/thiserror/releases

### chrono 0.4.42 → 0.4.43

  New Features:

- NaiveDate::abs_diff method - Calculate absolute difference between dates
- Feature-gated defmt support - Better embedded debugging support

  Performance:

- Faster RFC 3339 parsing - Optimized date/time parsing

  Other:

- Windows-bindgen upgraded to 0.65
- Stabilized doc_auto_cfg feature
- Added doctest for NaiveDate::years_since

  Sources: https://github.com/chronotope/chrono/releases

### rmcp 0.12.0 → 0.13.0 (transitive via rig-core)

  New Features:

- Blanket trait implementations for ClientHandler and ServerHandler traits
- Graceful shutdown - New close() method for graceful connection shutdown
- Pluggable OAuth storage - StateStore trait for custom OAuth state storage
- Task support - Full task implementation per SEP-1686 (async/long-running tool calls)
- SSE polling support via server-side disconnect (SEP-1699)
- Elicitation improvements - Enhanced enum schema handling per SEP-1330
- Optional icons field added to RawResourceTemplate

  Bug Fixes:

- JSON RPC errors now properly bubble up to client during initialization
- Build now works when no features are selected
- Fixed race condition in OneshotTransport (Semaphore instead of Notify)
- Added OpenID Connect discovery support per spec
- Token refresh only attempts if refresh token or expiry time exists

  Sources: https://github.com/modelcontextprotocol/rust-sdk, https://docs.rs/crate/rmcp/latest
