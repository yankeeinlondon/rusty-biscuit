---
area: "{{ctx.current_package_area}}"
---

## Context

You are performing a code review for the **tree-hugger** package area; it has the following packages in it:

- `tree-hugger` - library package
- `tree-hugger-cli` - a CLI which leverages the 'clap' crate and the **tree-hugger** library to allow callers to evaluate their code bases via **tree-sitter** static analysis

### CLI

The CLI's subcommands provide a useful view on the structuring of this functionality:

```txt
Tree Hugger diagnostics and symbol tooling

Usage: hug [OPTIONS] <COMMAND>

Commands:
  functions    List functions in the file(s)
  types        List types in the file(s)
  symbols      List all symbols in the file(s)
  imports      List imported symbols in the file(s)
  classes      List classes and their members
  lint         Run lint diagnostics on the file(s)
  completions  Generate shell completions
  help         Print this message or the help of the given subcommand(s)

Options:
      --language <LANGUAGE>     Force a specific language [possible values: rust, javascript, typescript, go, python, java, php,
                                perl, bash, zsh, c, c++, c#, swift, scala, lua]
      --json                    Output as JSON
      --plain                   Disable colors and hyperlinks (plain text output)
      --comments                Show symbol-level documentation comments in output
      --group-by-file           Group symbol output by file path
      --group-by-module         Group symbol output by module path (directory/module scope)
      --sort-by-kind            Sort symbols by kind before name
      --sort-by-module          Sort symbols by module before other sort keys
      --exclude-files <GLOB>    Glob patterns for files to exclude from scanning
      --exclude-symbols <GLOB>  Glob patterns for symbol names to exclude from output
  -h, --help                    Print help
  -V, --version                 Print version
  ```

## Code Review Dimensions

1. **Correctness**
   - Look for logic bugs, edge-case failures, incorrect assumptions, broken invariants, race conditions, and misuse of APIs.
   - Identify code paths that can panic unexpectedly.
   - Distinguish between acceptable `panic!` usage and places where `Result`-based error handling would be more appropriate.

2. **Rust idioms**
   - Evaluate whether the code follows idiomatic Rust patterns.
   - Call out unidiomatic ownership/borrowing, unnecessary cloning, poor enum usage, awkward trait design, needless indirection, and weak type modeling.
   - Suggest stronger use of the type system where it would materially improve safety or clarity.

3. **Error handling**
   - Review whether recoverable failures use `Result` appropriately.
   - Review whether unrecoverable conditions are justified when they panic.
   - Identify places where error context is missing, errors are swallowed, or error types are poorly designed.
   - Check whether library code and CLI/app code use different error-handling strategies appropriately.

4. **API and module design**
   - Evaluate public API shape, naming, cohesion, visibility, encapsulation, and separation of concerns.
   - Identify modules, traits, functions, or structs that are too large, too coupled, or difficult to reason about.
   - Review whether public interfaces feel stable, composable, and unsurprising.

5. **Safety and unsafe code**
   - Inspect all `unsafe` usage with extreme care.
   - For each `unsafe` block, state:
     - what invariant must hold
     - whether the code appears to uphold it
     - whether the invariant is documented
     - whether the unsafe region is minimized
   - Call out any unsoundness risk or insufficient justification.

6. **Concurrency / async**
   - If applicable, review async code, shared state, locking, Send/Sync assumptions, task cancellation, backpressure, and blocking work inside async contexts.
   - Identify deadlock risk, contention, misuse of channels, and lifetime/ownership issues hidden by synchronization primitives.

7. **Performance**
   - Identify obvious allocation churn, unnecessary copies/clones, inefficient iteration, pathological data structures, or avoidable serialization/parsing overhead.
   - Do not speculate wildly; separate “likely issue” from “needs benchmarking”.
   - Note where a micro-optimization is not worth the complexity.

8. **Testing**
   - Review unit, integration, property, snapshot, fuzz, and regression testing where relevant.
   - Identify missing coverage for edge cases, error paths, boundary conditions, concurrency behavior, parsing/serialization round trips, and public API guarantees.
   - Point out brittle or low-value tests.

9. **Documentation and maintainability**
   - Review rustdoc, README quality, examples, comments, and discoverability.
   - Identify places where invariants, assumptions, lifetimes, safety contracts, or tricky algorithms are under-documented.
   - Call out misleading names or comments that no longer match the code.

10. **Tooling / quality gates**

## Review Method

Use this process:

1. First, build a short mental model of the project:
   - what kind of Rust project it is
   - key crates/modules
   - public entry points
   - critical execution paths
   - where risk is concentrated

2. Then review the highest-risk areas first:
   - unsafe code
   - parsing/serialization
   - concurrency/async
   - error handling boundaries
   - public APIs
   - complex state transitions
   - performance-critical hot paths

3. Prefer findings that are:
   - specific
   - reproducible
   - tied to concrete code
   - explainable in Rust terms

4. Avoid low-value review comments such as:
   - purely stylistic nits unless they affect readability materially
   - trivial renames unless they reduce confusion
   - generic statements like “add more tests” without naming the missing cases

## Output Format

Produce the review in this structure:

### 1. Executive Summary

- 5–10 sentence summary of the project’s quality
- overall risk level: `low`, `medium`, or `high`
- biggest strengths
- biggest concerns
- whether the code seems production-ready, experimental, or fragile

### 2. Key Findings
For each finding, use this exact structure:

#### [Severity: Critical | High | Medium | Low] Short title

- **Location:** file/module/function or best approximation
- **Why it matters:** explain the engineering impact
- **Evidence:** cite the concrete code behavior or pattern you observed
- **Recommendation:** give a precise fix or refactor direction
- **Confidence:** `high`, `medium`, or `low`

Focus on the most important findings first.

### 3. Rust-Idiomaticity Notes

- Brief section for non-critical but meaningful Rust improvements
- Prefer type-system, ownership, trait, and API-shape observations

### 4. Testing Gaps

- Specific missing tests
- Name exact scenarios worth adding

### 5. Unsafe Code Review

- Separate section even if the verdict is “no unsafe usage found”
- If unsafe exists, enumerate each unsafe site and review its safety contract

### 6. Prioritized Next Steps

- Top 3 to 7 recommended follow-up actions in priority order

## Severity Guidance

Use these severity levels consistently:

- **Critical**: likely unsoundness, data corruption, security issue, serious race condition, or clearly broken behavior in an important path
- **High**: significant correctness or API design issue, panic risk in normal operation, major test gap around critical functionality
- **Medium**: maintainability issue, missing edge-case handling, meaningful performance problem, confusing API, incomplete documentation around important behavior
- **Low**: worthwhile cleanup, minor idiomatic improvement, small docs/test polish

## Important Constraints

- Be direct and candid.
- Do not soften serious issues.
- Do not invent facts not grounded in the code.
- If something is uncertain, say so explicitly.
- Distinguish facts from hypotheses.
- Prefer “this appears risky because...” over overclaiming.
- When recommending changes, preserve the project’s likely intent and architecture unless there is a strong reason not to.
- Do not flood the review with style-only comments.
- Optimize for signal density.

Now perform the review and save the results to @tree-hugger/reviews/{{ctx.today}}-comprehensive/review.md 

- now set the `created` frontmatter property of @tree-hugger/reviews/{{ctx.today}}-comprehensive/review.md to "{{ctx.now}}"
- now set the `agent` frontmatter property of @tree-hugger/reviews/{{ctx.today}}-comprehensive/review.md  to "{{env.AGENT}}"
- now set the `model` frontmatter property of @tree-hugger/reviews/{{ctx.today}}-comprehensive/review.md  to "{{env.MODEL}}"
- now set the `yolo` frontmatter property of @tree-hugger/reviews/{{ctx.today}}-comprehensive/review.md to "{{env.YOLO}}"
- now set the `duration` frontmatter property to the total duration that this review took to complete
- now set the `ready` frontmatter property based on whether you believe that the current state of the tree-hugger package area is "production area"

### IMPORTANT

- use the 'rust', 'tree-hugger', and 'rust-testing' skills
