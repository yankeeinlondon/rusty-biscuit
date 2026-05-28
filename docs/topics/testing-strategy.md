# Testing Strategy in Rusty Biscuit

We employ the following _types_ of tests in Rusty Biscuit:

- Style and Formatting: **Lint Tests**
- Functionality: **Unit Tests** and **Integration Tests**
- Performance: **Performance Tests**
- Edge Cases: **Fuzz Testing**

The packages in this library currently have no need for **load testing** but we will add that if the requirements ever change.

## Lint Testing

We provide lint rules to ensure that code is:

1. using idiomatic Rust patterns
2. is using current approaches that match with the Rust edition we're using (2024)

While Unit and Integration tests might be slightly more important, we believe that NO work should be considered complete unless
all lint warnings/errors are removed.

## Style and Formatting

Though not really a form of testing, formatting Rust code often gets lumped into a similar bucket as testing and so we'll cover it here.

- Style and formatting consistency is provided by `cargo fmt`
- We do have a standard format that we like code to take but it is far lower priority than working code
- Developers should NEVER run `cargo fmt` as this just adds "noise" to the development lifecycle by creating a ton of git commits that have no functional or semantic difference
- Instead we will periodically run reformatting tasks across the entire monorepo to bring all code back to the standardized formatting

## Functional Testing

- we use the [cargo-nextest](https://crates.io/crates/cargo-nextest) crate to act as the test runner for both **unit** and **integration** testing in **Rusty Biscuit**

- the nomenclature we use is:

    - **Unit Testing**

        - tests small isolated parts of the code
        - typically a function, method or module
        - when we run "unit tests" that includes the full suite of unit tests (including those designated as "sanity tests")
        - this monorepo has a lot of CLI based applications and you will find us using the terminology "Level 1" testing in many cases:
            - 

    - **Integration Testing**

        - an integration tests verifies that multiple parts of the system work correctly together
        - rather than a single function it wires together separate parts of the system
        - typical examples would include:
            - an application's access to the database
            - an authentication flow for an app
            - filesystem behavior
            - functionality that relies on an set of API calls
            - etc.
        - the MOST common example of integration testing you'll find in Rusty Biscuit is CLI testing where the testing needs to use a "real terminal"; see below in the **Note on CLI Testing**.

    - **Sanity Tests**

        A subset of unit and integration tests (more focused on unit tests but a few integration tests are thrown in where we need integration tests for reasonable coverage). This set of tests must be very fast in execution while trying to provide as wide a functional net as possible. Being able to assert that all "sanity tests" passed in a package area (or across the monorepo) give a good preliminary sense that things are working but should never be used as a substitute for a full unit and integration test run.

        > Note: 
        > 
        > - this does mean that all new tests should be considered for whether they should be added to the "sanity test" suite.
        > - the decision to include this in the sanity test should also be "validated" at the end of any feature implementation to
        >   make sure the test is "fast enough" to be a real candidate for the sanity test. If it turns out to be too slow, the
        >   developer should consider whether another test could be substituted to fill in the coverage gap that the removal of
        >   of this slower test has left behind.

        **Where to use:**

        - while in development a developer might choose to run all "targeted test" on a test for the current functionality being developed/updated and then run all "sanity tests" in parallel to get a sense as to whether the current development process has accidentally broken anything accidentally.
            - to aid in this workflow, every package area provides a `just sanity` recipe
            - when run with no parameters it just runs all of the sanity tests, however, a developer can add as many individual tests as parameters so they can add their own tests into the sanity test
        - once a developer has gotten all their tests to pass (along with the sanity tests validating that existing functionality "probably works"), the developer should complete their work by running `just test` and `just lint` which will make sure ALL tests pass


> **A Note on CLI Testing:**
> 
> - a lot of packages in **Rusty Biscuit** are CLI applications and for these we have adopted a terminology for three levels of integration testing:
> 
>     - **Level 1 [in-process / PTY]** Unit tests, plus tests that spawn the binary in a pseudo-TTY and feed it manufactured input bytes. Useful and necessary, but does NOT verify the terminal emulator's encoder/decoder behaviour — *we* generate those bytes. Cannot catch bugs like "WezTerm does not emit bare-modifier press events because we forgot to push `REPORT_ALL_KEYS_AS_ESCAPE_CODES`.
>     - **Level 2 (run-in-real-terminal with IPC).** Spawn the binary inside an actual terminal emulator (WezTerm, Kitty) or multiplexer (tmux), capture the rendered pane text via the terminal's CLI (`wezterm cli get-text`, `kitty @ get-text`, `tmux capture-pane`). Verifies that glyphs, widths, SGR styling, and scrolling render correctly through the real terminal. Input is still byte-level injected via the terminal's CLI, so the terminal's input encoder is NOT exercised.
>     - **Level 3 (OS keyboard injection).** Real OS keyboard events (`cliclick` on macOS, `xdotool` on Linux) injected into the spawned terminal window. The terminal's input encoder fires — this is the only level that can verify "what bytes does the terminal actually emit when the user presses bare Ctrl?" Required for any UX requirement of the form "when the user holds/presses key X, Y happens." Currently env-gated behind `RUN_LEVEL3=1` because focus stability is platform-specific.

## Performance Testing

### Tooling

- We use the [criterion](https://github.com/bheisler/criterion.rs) crate for performance testing
- Use the 'rust-testing' skill for more information on how to use

### Functional Scoping

Each "package area" in this monorepo will determine what functionality is core to their performance profile and articulate it
in a document located at `{package-area}/docs/performance-testing.md`.

- this document will combine prose (in body) and structured data (in the Frontmatter)
- the body of the document should document what areas are seen as the core blocks/components/groups of functionality that should be performance tested
    - each core block should be an H2 heading in the document
    - it should be 

#### Optional Opt Out

- it is ok for a package area to opt-out of performance testing; temporarily or indefinitely
- when a project is brand new we often have too many moving pieces and it is worth having the functionality mature before we add in performance testing
- when there is no `{package-area}/docs/performance-testing.md` file this is a _implicit_ signal that no performance tests should be included, however, an implicit signal should be discouraged so whenever we run `just bench` we will log a warning that only an "implicit" indication was given for a particular package area
    - "⚠️ the <b>{area}</b> is missing a <yellow>docs/performance-testing.md</yellow> file indicating this areas performance testing strategy"
    - this is just a warning and reminder that this file should be included

#### Drift Detection

- we want to be sure that every `H2` section in a `performance-testing.md` -- which indicates an area to test for performance in -- 

### Lifecycle

-


## Fuzz Testing

**Fuzz testing** takes a program, function, parser, API, etc. and pushes a large volume of random (often malformed) inputs through it.
The **fuzzer** explores the input space automatically and keeps inputs that trigger new code paths or failures. It is especially useful for parsers, serialization/deserialization, compilers, interpreters, protocol implementations, file-format readers, and security-sensitive code where unusual inputs are likely to expose hidden assumptions.

### Usage in Package Areas



### Lifecycle



## CI/CD Testing Strategy

### Functional Testing

- all packages in Rusty Biscuit are intended to be executable on macOS, Linux, and Windows
- no one OS is more or less important than the other, however, there is a natural tendency in developer for macOS to get a larger amount of testing
