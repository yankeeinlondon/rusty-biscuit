---
area: "{{ctx.current_package_area || ''}}"
today: "{{ctx.today}}"
packages: "{{ as_csv(ctx.packages) || '' }}"
start:
    message: "🏃‍♂️ starting a performance review for {{bold}}{{area}}{{reset}} package area"
success:
    say: "performance review for {{area}} package area completed successfully"
    message: "✅ performance review for **{{area}}** package area completed successfully"
failure:
    say: "performance review for {{area}} package area failed to complete"
    message: "❌ performance review for **{{area}}** package area failed to complete!"
---

# Rust Performance Review

You are performing a **senior-level Rust performance review** of the {{area}} package area of this monorepo. This package area includes the packages:

::shell sniff repo packages --package-area {{area}}

Your job is to identify concrete opportunities to improve runtime performance, memory efficiency, async/concurrency behavior, I/O efficiency, dependency overhead, compile-time cost, and scalability.

The review must be evidence-based, prioritized, and actionable.

Assume the audience is an experienced Rust developer. Avoid generic advice. Focus on findings grounded in the actual code.

## Review Goals

Review the project with a focus on the following dimensions.

### 1. Runtime Performance

Look for:

- hot paths with avoidable work
- unnecessary cloning
- avoidable allocations
- repeated computation
- inefficient iteration
- nested loops with poor complexity
- suboptimal data structures
- excessive parsing or serialization
- repeated formatting in loops
- avoidable synchronization
- blocking behavior
- unnecessary dynamic dispatch
- repeated construction of expensive objects

Call out `O(n²)` or worse behavior when the surrounding code suggests it may matter.

Do not over-prioritize micro-optimizations unless they occur in a clear hot path or repeated operation.

### 2. Memory Efficiency

Look for:

- excessive ownership where borrowing would suffice
- unnecessary `String`, `Vec`, `HashMap`, `HashSet`, `PathBuf`, or `serde_json::Value` construction
- large clones of collections, AST-like structures, config objects, parsed documents, or buffers
- APIs that force callers to allocate
- opportunities to use `&str`, `&Path`, slices, iterators, `Cow<'_, str>`, `Arc`, or streaming APIs
- places where caching would reduce memory churn
- places where caching would likely make memory behavior worse

### 3. Async and Concurrency Performance

Look for:

- blocking work inside async functions
- synchronous filesystem or subprocess calls inside async paths
- unbounded task spawning
- unbounded channels
- accidental serialization of independent work
- excessive lock contention
- overly broad lock scopes
- use of `std::sync::Mutex` or `std::sync::RwLock` in async code where inappropriate
- shared mutable state that could be avoided with ownership restructuring
- missed opportunities for bounded concurrency using semaphores, worker pools, `buffer_unordered`, or structured task orchestration

### 4. I/O and Filesystem Behavior

Look for:

- repeated filesystem traversal
- repeated `metadata`, `canonicalize`, `read_dir`, or recursive walk operations
- reading entire files into memory where streaming would be better
- inefficient temp-file usage
- poor batching of I/O operations
- avoidable subprocess invocations
- shell-outs in loops
- synchronous I/O where async I/O is expected
- repeated reads of the same config, manifest, lockfile, markdown file, JSON file, TOML file, YAML file, or project metadata

### 5. Parsing, Serialization, and Text Processing

Look for:

- repeated parsing of the same content
- repeated `serde_json::from_str`, `toml::from_str`, `serde_yaml::from_str`, or similar operations
- repeated regex construction
- expensive regex use where simpler parsing would suffice
- regexes that should be moved to `LazyLock`, `OnceLock`, or equivalent
- costly string concatenation patterns
- `format!` inside loops
- unnecessary conversion between structured formats
- unnecessary conversion between `String`, `&str`, `PathBuf`, `&Path`, `OsString`, and `OsStr`
- repeated markdown parsing, syntax highlighting, rendering, or template expansion

### 6. Data Structures and Algorithms

Evaluate whether the selected data structures fit the usage pattern.

Look for:

- lookup-heavy paths using linear scans
- repeated joins/grouping/deduplication without an index
- repeated graph traversal that could be cached or indexed
- `Vec` where `HashMap`, `HashSet`, `BTreeMap`, `BTreeSet`, or `IndexMap` would be more appropriate
- ordered maps/sets where ordering is not actually required
- hash maps where deterministic ordering is required later
- repeated sorting
- repeated allocation of temporary collections
- avoidable cloning to satisfy ownership instead of changing the algorithm shape
- opportunities to separate “build index once” from “query many times”

### 7. API and Architectural Performance

Look for public or internal APIs that force inefficient usage.

Examples:

- accepting `String` where `&str` is sufficient
- accepting `PathBuf` where `&Path` is sufficient
- accepting `Vec<T>` where `IntoIterator`, `&[T]`, or an iterator would be better
- returning `Vec<T>` where an iterator, stream, or caller-provided buffer would be better
- forcing materialization of intermediate results
- forcing repeated parse/convert/render cycles across module boundaries
- APIs that obscure whether a function is cheap or expensive
- architecture that mixes “prepare once” and “execute many times”
- lack of explicit caching boundaries
- lack of batching in APIs that perform I/O or subprocess work

### 8. Compile-Time and Binary-Size Cost

Only include compile-time findings if they are relevant and supported by code or dependency evidence.

Look for:

- heavy generic monomorphization
- expensive derive/proc macro usage in hot development loops
- overly broad feature flags
- dependencies pulled in for small use cases
- optional dependencies that are not actually optional
- feature leakage from default features
- large crates used for narrow functionality
- excessive binary size from unused features
- places where dynamic dispatch could reduce compile-time bloat without harming runtime-critical code

## Required Method

Do not simply list Rust performance tips.

Inspect the code and produce findings tied to specific files, functions, modules, dependency choices, or call paths.

For every finding, include:

- **Severity:** `High`, `Medium`, or `Low`
- **Category:** `runtime`, `memory`, `async/concurrency`, `I/O`, `parsing/text`, `data-structure`, `API/architecture`, `compile-time`, or `dependency`
- **Location:** file path and function/module name where possible
- **Problem:** what is inefficient
- **Why it matters:** when this cost becomes significant
- **Evidence:** code pattern, call path, loop behavior, allocation behavior, dependency behavior, or complexity analysis
- **Recommendation:** concrete change to make
- **Expected impact:** qualitative estimate of what improves
- **Risk/tradeoff:** maintainability, correctness, API, memory, or migration tradeoff

When evidence is insufficient, say so explicitly and recommend a benchmark or profiling step that would resolve the uncertainty.

## Prioritization Rules

Prioritize findings in this order:

1. Performance issues likely to affect common or high-volume code paths.
2. Algorithmic improvements with clear asymptotic benefit.
3. Avoidable I/O, parsing, cloning, or allocation in loops.
4. Async/concurrency issues that can cause latency spikes or throughput collapse.
5. API design choices that force downstream inefficiency.
6. Dependency, compile-time, or binary-size issues.

Do not give high severity to style-only issues.

Do not give high severity to micro-optimizations unless they occur in a hot path, a loop, a frequently called function, or a high-volume workflow.

## Search Patterns

Use these patterns as starting points for investigation, not as a substitute for understanding the code:

- `.clone()`
- `.to_string()`
- `.to_owned()`
- `.into_owned()`
- `format!`
- `collect::<Vec<_>>()`
- `Regex::new`
- `serde_json::from_str`
- `serde_json::to_string`
- `toml::from_str`
- `serde_yaml::from_str`
- `std::fs::read_to_string`
- `std::fs::read`
- `std::fs::metadata`
- `std::fs::canonicalize`
- `std::fs::read_dir`
- `walkdir`
- `Command::new`
- `tokio::spawn`
- `spawn_blocking`
- `Mutex`
- `RwLock`
- `Arc<Mutex`
- `Arc<RwLock`
- `HashMap`
- `BTreeMap`
- `HashSet`
- `BTreeSet`
- `.sort`
- `.dedup`
- `.join`
- `.lines().collect`
- `.chars().count`
- nested `for` loops
- public functions accepting owned `String`, `PathBuf`, `Vec`, or config structs

For each match, decide whether it is actually a problem. Many matches will be benign.

## Output Format

Produce the review using the following structure.

0. `# Rust Performance Review for {{area}}`

1. `## Executive Summary`

    Summarize the most important performance risks and highest-leverage improvements.

    Include:
    - number of findings by severity
    - top 3 recommendations
    - areas that already appear performance-conscious
    - areas where more measurement is needed

2. `## Findings`

    For each finding:
    - `## Finding Title`

        **Severity:** High | Medium | Low  
         **Category:** runtime | memory | async/concurrency | I/O | parsing/text | data-structure | API/architecture | compile-time | dependency  
         **Location:** `path/to/file.rs`, function/module name

        **Problem**

        Explain the inefficient behavior.

        **Why it matters**

        Explain when this cost becomes significant.

        **Evidence**

        Point to the relevant code pattern, call path, loop behavior, allocation behavior, dependency behavior, or complexity issue.

        **Recommendation**

        Give a concrete change. Include a short code sketch when useful.

        **Expected impact**

        Explain what gets faster, cheaper, or more scalable.

        **Risk/tradeoff**

        Explain any downside, migration concern, correctness risk, or maintainability tradeoff.

    > Note: repeat for each finding.

3. `## Quick Wins`

    List small, low-risk changes that are likely worth doing even before deep benchmarking.

    Use a table:

    | Change | Location | Why it is low risk | Expected benefit |
    | ------ | -------- | ------------------ | ---------------- |

    ## Benchmarking and Profiling Recommendations

    Suggest targeted benchmarks or profiling steps that would validate the important findings.

    Prefer concrete commands or benchmark targets where possible.

    Consider tools such as:
    - `cargo bench`
    - Criterion
    - `cargo flamegraph`
    - `perf`
    - Instruments on macOS
    - `heaptrack`
    - `dhat`
    - `cargo bloat`
    - `cargo llvm-lines`
    - `tokio-console`
    - `hyperfine`

    Only recommend tools that fit the project and the findings.

4. `## Non-Issues / Things I Would Not Change Yet`

    Call out tempting micro-optimizations that are probably not worth doing.

    Also call out code that appears acceptable as-is despite matching common suspicious patterns.

5. `## Suggested Implementation Order`

    Provide a prioritized sequence for addressing the findings.

    Use this format:
    1. Highest impact / lowest risk
    2. High impact but requires design care
    3. Medium-impact cleanup
    4. Benchmarking and validation
    5. Optional compile-time or dependency cleanup

## Review Finalization

To complete this task you must

- save the specified report format to the file "{{area}}/reviews/{{today}}-performance-review/review.md"
- set the `agent` Frontmatter property to "{{env.AGENT}}"
- set the `model` Frontmatter property to "{{env.MODEL}}"
- set the `repo` Frontmatter property to "{{ctx.repo}}"
- set the `created` Frontmatter property to "{{today}} at {{time}}"
- ensure all Frontmatter properties have been saved to "{{area}}/reviews/{{today}}-performance-review/review.md" and that the review content is in the body of the document
