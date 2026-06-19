---
prompt: |-
    The `cargo-crap` crate measure cyclometric complexity and combines that with test coverage to provide a "crap" score than is intended to measure code quality or at least code risk.

    Your task is to research the cargo-crap crate and write your findings to the body of this document. Your research must address the following questions:

    - what is the measurement methodology?
    - what makes this measure useful?
    - what situations might make this measurement non-reflective of code quality? Can any of these situations be mitigated?
    - how do you install `cargo-crate`?
    - how do you use `cargo-crate`? How does that vary when you're in a monorepo?
    - how computationally expensive is running this? how much time (roughly) does it take to run this? How much of the time taken is generating the `.lcov` files?
    - any surprises or complexities that developers mention having with this measurement tool? are there any mitigations to these problems?

policy:
    timing: 1 yr
    version: major
last_updated: 2026-05-25
---
## `cargo-crap`

Research into the `cargo-crap` crate reveals a tool designed to measure the **Change Risk Anti-Patterns (CRAP)** metric, specifically tailored for Rust development as a guardrail against "complexity slop"—particularly in AI-assisted coding environments.

### Measurement Methodology

The `cargo-crap` tool calculates a risk score for individual functions by combining static analysis with dynamic test coverage data. It uses the following mathematical formula:

$$CRAP(f) = CC(f)^2 \times (1 - cov(f))^3 + CC(f)$$

* **Cyclomatic Complexity ($CC$):** Measured via static analysis. The tool uses the `syn` crate to parse Rust source code and count independent execution paths (branches, loops, matches).
* **Test Coverage ($cov$):** Extracted from an **LCOV** report. It represents the percentage of code paths exercised by the test suite (expressed as a decimal from 0.0 to 1.0).

The score is heavily weighted toward complexity (squared) and lack of coverage (cubed), meaning the score "explodes" for complex functions that are poorly tested. A score of **30** or higher is generally considered a "CRAP" function that requires immediate refactoring or additional testing.

### What makes this measure useful?

* **AI Development Guardrail:** In environments where AI agents generate code, they may produce "correct" code (that compiles) but is overly complex and untested. `cargo-crap` acts as a "complexity budget" to prevent this.
* **Maintenance Risk Identification:** While the Rust compiler ensures memory safety and type correctness, it is "blind" to maintainability. This tool identifies code that is "safe" in the eyes of the compiler but "risky" to modify because of hidden logic branches.
* **Prioritization:** It helps teams prioritize refactoring efforts by highlighting the specific functions that represent the highest regression risk.

### Situations where the measurement may be non-reflective

While powerful, the CRAP metric has specific blind spots:

* **Cognitive vs. Cyclomatic Complexity:** A long `match` statement with many simple arms has high Cyclomatic Complexity but may be very easy for a human to understand. Conversely, a short function with complex trait bounds and nested `Result` handling might have low CC but high cognitive load.

    * *Mitigation:* Use the tool as a signal for review rather than an absolute rule; exempt stable, simple "dispatch" functions if necessary.

* **Boilerplate and Generated Code:** Auto-generated code or boilerplate might inflate scores without representing real "risk."

    * *Mitigation:* Use the `--exclude` flags or filter the LCOV report to ignore generated files.

* **Trivial Tests:** A function can achieve 100% coverage with tests that don't actually assert meaningful behavior, masking high complexity.

    * *Mitigation:* Combine `cargo-crap` with mutation testing (e.g., `cargo-mutants`) to ensure test quality.

### Installation

The tool is installed via Cargo:

```bash
# Standard installation
cargo install cargo-crap

# For faster installation via pre-built binaries
cargo binstall cargo-crap
```

### Usage

Usage is a two-step process because `cargo-crap` requires external coverage data.

1. **Generate Coverage:** Use `cargo-llvm-cov` to produce an LCOV file.
   ```bash
   cargo llvm-cov --lcov --output-path lcov.info
   ```

2. **Run Analysis:**
   ```bash
   cargo crap --lcov lcov.info
   ```

**Monorepo Considerations:**
In a monorepo, you typically run the coverage command with the `--workspace` flag at the root. `cargo-crap` also supports the `--workspace` flag to ensure it maps source files correctly across multiple crates.

```bash
cargo crap --workspace --lcov lcov.info
```

### Computational Expense

* **Execution Time:** The analysis performed by `cargo-crap` itself is extremely fast (typically **1–5 seconds** for large projects) as it primarily involves string parsing and in-memory joining of data.
* **The LCOV Bottleneck:** The vast majority of the time (often **95% or more**) is spent generating the `lcov.info` file. This involves:

    * **Instrumentation:** Re-compiling the project with coverage hooks.
    * **Test Execution:** Running the entire test suite (the slowest part).
    * **Processing:** Merging `.profraw` files into the LCOV format.

* **Optimization:** To mitigate this, developers often run coverage in a dedicated CI job or use `cargo nextest` to parallelize the test execution phase.

### Surprises and Complexities

* **Path Mapping Mismatches:** A common frustration is when `lcov.info` uses absolute paths (common in CI) while `cargo-crap` expects relative paths.

    * *Mitigation:* `cargo-crap` uses a **two-level index** (canonical hash lookup followed by suffix matching) to automatically resolve these mismatches without manual configuration.

* **Inaccurate CC for Macros:** Since CC is calculated via static analysis of the source, complex macros may have their complexity undercounted if the tool doesn't expand them.

    * *Mitigation:* High-risk logic should generally be moved out of complex macros into testable functions where the CC can be accurately measured.

* **"Preservation by Accumulation":** Developers have noted that AI agents often respond to CRAP scores by adding trivial tests just to lower the score rather than simplifying the code.

    * *Mitigation:* Enforce a "CC threshold" (e.g., no function over CC 10) alongside the CRAP score to force simplification.
