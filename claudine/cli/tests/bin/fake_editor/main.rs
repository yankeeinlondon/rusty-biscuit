//! Test-only native fake editor, for the tests that launch the user's editor
//! through Claudine's lifecycle stacks (`EDITOR` points here). The body is
//! `test_toolkit::fake_editor`; see there for the environment variables that
//! drive it.

fn main() -> std::process::ExitCode {
    test_toolkit::fake_editor::main()
}
