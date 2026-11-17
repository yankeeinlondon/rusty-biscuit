//! Test-only native `echo`: prints its arguments joined by single spaces and
//! a newline. A template shell command runs its program directly, never
//! through a shell, so on Windows `echo` (a `cmd` builtin) is not a program;
//! tests that read a templated `echo`'s output install this as
//! `echo{EXE_SUFFIX}` in the fixture `bin`, which leads the child's `PATH`.

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", words.join(" "));
}
