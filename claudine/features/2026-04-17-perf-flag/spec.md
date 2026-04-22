# Performance Flag (`--perf`)

In Darkmatter's CLI we added a `--perf` flag that was very helpful in understanding the performance characteristics when composing documents.

This flag leveraged traces (adding new traces where necessary to get good metrics) and reported what each part of the pipeline took to complete. Having the same thing for ALL claudine commands is the focus of this feature.

## Supported Commands

All of these CLI invocations should include the new `--perf` CLI flag:

- `claudine {agent} '{prompt}'`
- `claudine compose @file`
- `claudine inline-compose @file`
- `claudine sequence @file`

## Scope of Performance Metrics

The performance metrics must be end-to-end and comprehensive, presented as distinct visual blocks:

- **CLI Overhead**: A detailed breakdown of setup steps including:
    - **Arg Parsing**: Time taken to parse command-line arguments.
    - **Config Loading**: Time taken to load and validate configuration files.
    - **Tracing Init**: Time taken to initialize the tracing/logging subsystem.
    - **Environment Setup**: Time taken to initialize the environment and context.
- **Composition Report**: If document composition occurred (leveraging the `darkmatter` composition report).
- **Agent Execution**: LLM network latency and agent execution time.

## Implementation

The performance tracking will use **Manual Timers** (`std::time::Instant`) to capture metrics. This approach mirrors the implementation in `darkmatter` for consistency and precision.

## Presentation

The performance report should be human-readable and emitted to **stderr** only after the entire command (including the agent's response) is complete. This ensures `stdout` remains clean for piping and matches `darkmatter`'s established behavior.

## Sequence Handling

For `claudine sequence`, timing metrics should be collected silently across all steps of the sequence. A single, aggregated end report must be printed at the very end of the sequence completion.

## Success Criteria

- The `--perf` flag is available on all specified commands (`claudine {agent}`, `compose`, `inline-compose`, `sequence`).
- The performance report is emitted to `stderr` if and only if the `--perf` flag is present.
- The report includes the following distinct metrics:
    - Total elapsed time.
    - CLI overhead (with detailed breakdown: Arg Parsing, Config Loading, Tracing Init, Environment Setup).
    - Composition time (if applicable).
    - Agent response/execution time.
- For sequences, a single aggregated report is displayed at the end, rather than per-step reports.
