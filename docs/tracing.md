# Tracing and Observability Design

This document describes the design for comprehensive, multi-level debug logging across the Research and Shared libraries using the `tracing` crate ecosystem.

## Goals

1. **Library Callers**: Can opt into a stream of tracing events without the library imposing subscriber configuration
2. **CLI Users**: Can control verbosity via `-v/-vv/-vvv` flags or `RUST_LOG` environment variable
3. **Tool Debugging**: Visibility into agent tool calls (search, scrape) to diagnose issues like `MaxDepthError`
4. **Production Ready**: Support for structured logging, OpenTelemetry export, and performance monitoring

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Application Layer                           │
│  ┌─────────────────────┐    ┌─────────────────────────────────────┐ │
│  │   research-cli      │    │   Other Consumers (tests, apps)     │ │
│  │   - Configures      │    │   - Configure their own subscribers │ │
│  │     subscriber      │    │   - Filter by target/level          │ │
│  │   - Verbosity flags │    └─────────────────────────────────────┘ │
│  └─────────────────────┘                                            │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Library Layer                                │
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                       research-lib                               ││
│  │  - Emits spans and events (does NOT configure subscribers)       ││
│  │  - Uses #[instrument] for automatic span creation                ││
│  │  - Implements PromptHook for tool call tracing                   ││
│  └─────────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────────┐│
│  │                          shared                                  ││
│  │  - BraveSearchTool: traces searches, results, errors             ││
│  │  - ScreenScrapeTool: traces scrapes, content extraction          ││
│  │  - Emits structured events with consistent field names           ││
│  └─────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

## Key Design Principles

### 1. Libraries Emit, Applications Configure

Libraries (`research-lib`, `shared`) only emit tracing events and spans. They never install or configure subscribers. This follows the principle that the application entry point controls observability configuration.

```rust
// Library code (research-lib) - GOOD
#[tracing::instrument(skip(agent, prompt), fields(phase = 1))]
async fn run_agent_prompt_task(...) {
    tracing::info!(tool_enabled = true, "Starting prompt task");
}

// Library code - BAD (don't do this)
fn init() {
    tracing_subscriber::fmt::init(); // Never in a library!
}
```

### 2. Structured Fields Over String Messages

Use structured fields for machine-readable logs and efficient filtering:

```rust
// GOOD - Structured fields
tracing::info!(
    tool.name = "brave_search",
    tool.query = %query,
    tool.results_count = results.len(),
    tool.duration_ms = elapsed.as_millis(),
    "Search completed"
);

// LESS IDEAL - Unstructured
tracing::info!("Search for '{}' returned {} results in {}ms", query, count, ms);
```

### 3. Spans for Context and Timing

Use spans to group related events and measure durations:

```rust
#[tracing::instrument(
    name = "research_phase",
    skip(prompts, cancelled),
    fields(phase = 1, prompt_count = prompts.len())
)]
async fn run_phase_1(...) {
    // All events inside inherit the span context
    tracing::debug!("Starting parallel prompt execution");
}
```

## Tracing Levels

| Level | CLI Flag | Use Case |
|-------|----------|----------|
| ERROR | (default) | Failures that stop execution |
| WARN | (default) | Recoverable issues, degraded operation |
| INFO | (default for tools) | Tool calls, phase transitions, research progress |
| DEBUG | `-vv` | Tool arguments, API requests, intermediate results |
| TRACE | `-vvv` | Verbose internals, request/response bodies |

**Note:** Tool calls (brave_search, screen_scrape) and research progress are logged at INFO level by default to provide visibility into agent behavior.

### Level Guidelines

**ERROR**
- All prompts failed
- API authentication failures
- Critical file I/O errors

**WARN**
- Individual prompt failures (when others succeed)
- Tool call retries
- Rate limiting

**INFO**
- Phase transitions (Phase 1 → Phase 2)
- Tool calls summary (name, duration, success/fail)
- Research completion stats

**DEBUG**
- Tool call arguments and results
- Agent multi-turn iteration count
- HTTP request/response status codes
- File read/write operations

**TRACE**
- Full request/response bodies
- Parsed content details
- Internal state transitions

## Component Instrumentation

### shared/tools/brave_search.rs

```rust
use tracing::{debug, info, instrument, warn, Span};

impl Tool for BraveSearchTool {
    #[instrument(
        name = "brave_search",
        skip(self),
        fields(
            tool.name = "brave_search",
            tool.query = %args.query,
            tool.count = args.count.unwrap_or(10),
            otel.kind = "client"
        )
    )]
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let start = std::time::Instant::now();

        debug!(country = ?args.country, freshness = ?args.freshness, "Executing search");

        let response = self.client.get(&self.config.endpoint)
            .query(&params)
            .send()
            .await;

        match &response {
            Ok(resp) => {
                let status = resp.status().as_u16();
                Span::current().record("http.status_code", status);
                debug!(http.status_code = status, "Received API response");
            }
            Err(e) => {
                warn!(error = %e, "Search request failed");
            }
        }

        let results = self.parse_response(response?).await?;

        let elapsed = start.elapsed();
        info!(
            tool.results_count = results.len(),
            tool.duration_ms = elapsed.as_millis() as u64,
            "Search completed"
        );

        Ok(results)
    }
}
```

### shared/tools/screen_scrape.rs

```rust
#[instrument(
    name = "screen_scrape",
    skip(self),
    fields(
        tool.name = "screen_scrape",
        tool.url = %args.url,
        tool.formats = ?args.formats,
        otel.kind = "client"
    )
)]
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    debug!(
        only_main_content = args.only_main_content,
        mobile = args.mobile,
        "Starting page scrape"
    );

    // ... scraping logic ...

    info!(
        tool.status_code = response.status_code,
        tool.content_length = response.metadata.content_length,
        tool.duration_ms = response.metadata.duration_ms,
        "Scrape completed"
    );
}
```

### research-lib: PromptHook Implementation

Create a hook that emits tracing events for all agent tool calls:

```rust
use rig::agent::PromptHook;
use tracing::{debug, info, info_span, Instrument, Span};

/// A PromptHook that emits tracing events for agent interactions
#[derive(Clone)]
pub struct TracingPromptHook {
    span: Span,
}

impl TracingPromptHook {
    pub fn new(task_name: &str) -> Self {
        Self {
            span: info_span!("agent_task", task = %task_name),
        }
    }
}

impl<M> PromptHook<M> for TracingPromptHook {
    async fn on_completion_call(
        &self,
        _signal: CancelSignal,
        message: &Message,
        history: &[Message],
    ) {
        debug!(
            parent: &self.span,
            history_len = history.len(),
            "Sending prompt to model"
        );
    }

    async fn on_completion_response(
        &self,
        _signal: CancelSignal,
        _message: &Message,
        response: &CompletionResponse,
    ) {
        debug!(
            parent: &self.span,
            has_tool_calls = !response.tool_calls.is_empty(),
            tool_call_count = response.tool_calls.len(),
            "Received model response"
        );
    }

    async fn on_tool_call(
        &self,
        _signal: CancelSignal,
        name: &str,
        call_id: Option<&str>,
        args: &str,
    ) {
        info!(
            parent: &self.span,
            tool.name = %name,
            tool.call_id = call_id,
            tool.args = %args,
            "Invoking tool"
        );
    }

    async fn on_tool_result(
        &self,
        _signal: CancelSignal,
        name: &str,
        call_id: Option<&str>,
        _args: &str,
        result: &str,
    ) {
        // Truncate result for logging (tool results can be large)
        let result_preview: String = result.chars().take(200).collect();
        let truncated = result.len() > 200;

        info!(
            parent: &self.span,
            tool.name = %name,
            tool.call_id = call_id,
            tool.result_preview = %result_preview,
            tool.result_truncated = truncated,
            tool.result_len = result.len(),
            "Tool returned result"
        );
    }
}
```

### research-lib: Using the Hook

```rust
async fn run_agent_prompt_task<M>(
    name: &'static str,
    // ...
) -> PromptTaskResult
where
    M: CompletionModel,
{
    let hook = TracingPromptHook::new(name);

    let result = agent
        .prompt(&prompt)
        .multi_turn(15)
        .with_hook(hook)  // Attach the tracing hook
        .await;

    // ...
}
```

### research-lib: Phase Instrumentation

```rust
#[instrument(
    name = "research",
    skip(output_dir, questions, cancelled),
    fields(
        topic = %topic,
        question_count = questions.len(),
        tools_enabled = tools_available()
    )
)]
pub async fn research(
    topic: &str,
    output_dir: Option<PathBuf>,
    questions: &[String],
) -> Result<ResearchResult, ResearchError> {

    info!("Starting research session");

    // Phase 1
    {
        let _phase1_span = info_span!("phase_1", prompt_count = total).entered();
        info!("Beginning parallel prompt execution");

        // ... execute prompts ...

        info!(
            succeeded = phase1_succeeded.len(),
            failed = phase1_failed,
            "Phase 1 complete"
        );
    }

    // Phase 2
    {
        let _phase2_span = info_span!("phase_2").entered();
        info!("Generating consolidated outputs");

        // ... generate outputs ...
    }

    info!(
        total_time_secs = total_time,
        total_tokens = total_tokens,
        "Research complete"
    );
}
```

## CLI Subscriber Configuration

### research-cli/src/main.rs

```rust
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::{
    fmt,
    prelude::*,
    filter::EnvFilter,
    layer::SubscriberExt,
};

#[derive(Parser)]
#[command(name = "research")]
struct Cli {
    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output logs as JSON
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

fn init_tracing(verbose: u8, json: bool) {
    // Determine base filter from RUST_LOG or verbosity flags
    // Default shows INFO for tool calls and research progress
    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            // Default: Show INFO for tool calls and research progress
            0 => "warn,research_lib=info,shared::tools=info".to_string(),
            1 => "info,research_lib=info,shared=info".to_string(),
            2 => "info,research_lib=debug,shared=debug".to_string(),
            _ => "debug,research_lib=trace,shared=trace".to_string(),
        },
    };

    let filter = EnvFilter::try_new(&base_filter)
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    if json {
        // JSON output for structured log processing
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json());
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    } else {
        // Human-readable console output
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(true)
                    .with_level(true)
                    .with_thread_ids(false)
                    .with_file(verbose >= 3)
                    .with_line_number(verbose >= 3)
                    .compact()
            );
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    init_tracing(cli.verbose, cli.json);

    tracing::info!("Research CLI starting");

    match cli.command {
        Commands::Library { topic, questions, output } => {
            // ... existing logic ...
        }
    }
}
```

### Example CLI Usage

```bash
# Default - shows tool calls and research progress (INFO for research_lib and shared::tools)
research library tokio

# INFO level everywhere - see all phase transitions and tool summaries
research library tokio -v

# DEBUG level - see tool arguments, API requests, intermediate results
research library tokio -vv

# TRACE level - see everything including response bodies
research library tokio -vvv

# Using RUST_LOG for fine-grained control
RUST_LOG="research_lib=debug,shared::tools=trace" research library tokio

# JSON output for log aggregation
research library tokio --json
```

## Dependencies

### Cargo.toml Updates

**shared/Cargo.toml**
```toml
[dependencies]
tracing = "0.1"

[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-test = "0.2"
```

**research-lib/Cargo.toml**
```toml
[dependencies]
tracing = "0.1"

[dev-dependencies]
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-test = "0.2"
```

**research-cli/Cargo.toml**
```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

## Span and Field Naming Conventions

Follow OpenTelemetry semantic conventions where applicable:

| Field | Description | Example |
|-------|-------------|---------|
| `tool.name` | Name of the tool being called | `"brave_search"` |
| `tool.query` | Search query or URL | `"rust async programming"` |
| `tool.duration_ms` | Execution time in milliseconds | `1234` |
| `tool.results_count` | Number of results returned | `10` |
| `http.status_code` | HTTP response status | `200` |
| `phase` | Research phase number | `1` |
| `task` | Name of the prompt task | `"overview"` |
| `agent.turn` | Multi-turn iteration number | `3` |
| `otel.kind` | OpenTelemetry span kind | `"client"` |

## Testing Tracing Output

Use `tracing-test` for unit tests:

```rust
#[cfg(test)]
mod tests {
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_search_tool_emits_traces() {
        let tool = BraveSearchTool::from_env();
        let args = SearchArgs {
            query: "test".to_string(),
            ..Default::default()
        };

        let _ = tool.call(args).await;

        // Assert traces were emitted
        assert!(logs_contain("brave_search"));
        assert!(logs_contain("Search completed"));
    }
}
```

## Optional: OpenTelemetry Integration

For production deployments, add OpenTelemetry export:

```rust
use tracing_subscriber::prelude::*;
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::sdk::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;

fn init_otel_tracing() {
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("Failed to install OpenTelemetry tracer");

    let subscriber = tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer));

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");
}
```

## Implementation Phases

### Phase 1: Foundation
1. Add `tracing` dependency to `shared` and `research-lib`
2. Add `tracing-subscriber` to `research-cli`
3. Implement CLI verbosity flags and subscriber initialization
4. Add basic `#[instrument]` to top-level functions

### Phase 2: Tool Instrumentation
1. Instrument `BraveSearchTool::call()` with spans and events
2. Instrument `ScreenScrapeTool::call()` with spans and events
3. Create `TracingPromptHook` for agent tool call visibility
4. Attach hook to agent prompt requests

### Phase 3: Research Flow Instrumentation
1. Add spans for Phase 1 and Phase 2
2. Instrument individual prompt tasks
3. Add structured events for success/failure paths
4. Include timing and token count metrics

### Phase 4: Polish
1. Add `tracing-test` for trace assertions
2. Document RUST_LOG patterns for users
3. Consider OpenTelemetry integration for production

## Expected Output Examples

### Default (no flags)
Tool calls and research progress are visible by default:
```
INFO research: Starting research session topic="rig-core"
INFO research::phase_1: Beginning parallel prompt execution prompt_count=5
INFO research_lib: Starting prompt task with tools task="overview"
INFO research_lib: Invoking tool tool.name="brave_search" tool.args="{\"query\":\"rig-core rust\"}"
INFO shared::tools::brave_search: Search completed tool.results_count=10 tool.duration_ms=1234
INFO research_lib: Tool returned result tool.name="brave_search" tool.result_len=5432
INFO research_lib: Task completed successfully task="overview" elapsed_secs=12.3
INFO research::phase_1: Phase 1 complete succeeded=5 failed=0
INFO research::phase_2: Generating consolidated outputs
INFO research: Research complete total_time_secs=45.2 total_tokens=12345
```

### With `-v` (INFO everywhere)
Same as default but includes INFO from all targets (not just research_lib and shared::tools).

### With `-vv` (DEBUG)
```
DEBUG research_lib: Checking for BRAVE_API_KEY
INFO research: Starting research session topic="rig-core"
DEBUG research::phase_1: Creating agent with tools model="gemini-3-flash-preview"
INFO research::phase_1: Beginning parallel prompt execution prompt_count=5
DEBUG shared::tools::brave_search: Executing search query="rig-core rust library" count=10
DEBUG shared::tools::brave_search: Received API response http.status_code=200
INFO shared::tools::brave_search: Search completed tool.results_count=10 tool.duration_ms=892
DEBUG research::agent_task: Received model response has_tool_calls=true tool_call_count=1
INFO research::agent_task: Invoking tool tool.name="brave_search" tool.args="{\"query\":\"rig-core examples\"}"
...
```

## References

- [tracing crate documentation](https://docs.rs/tracing)
- [tracing-subscriber documentation](https://docs.rs/tracing-subscriber)
- [Tokio Tracing Guide](https://tokio.rs/tokio/topics/tracing)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)
- [rig-core telemetry module](https://docs.rs/rig-core/latest/rig/telemetry/)
- [rig-core PromptHook trait](https://docs.rs/rig-core/latest/rig/agent/trait.PromptHook.html)
