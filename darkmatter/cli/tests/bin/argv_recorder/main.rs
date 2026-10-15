//! Test-only native executable that records its argument vector, for the
//! shell template tests that prove one argument per placeholder. The body is
//! `test_toolkit::argv_recorder`; see there for the record format.

fn main() -> std::process::ExitCode {
    test_toolkit::argv_recorder::main()
}
