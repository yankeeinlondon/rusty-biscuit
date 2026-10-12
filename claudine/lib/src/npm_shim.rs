//! Launching an npm-installed command through the program its `.cmd` shim runs.
//!
//! On Windows, npm installs a global CLI as a `.cmd` batch shim. Rust's
//! standard library refuses to pass a batch file an argument containing a
//! newline ("batch file arguments are invalid"), and Claudine delivers a
//! composed prompt as one argument. Starting the shim's target directly takes
//! `cmd.exe` out of the launch, so the prompt reaches the provider intact.
//!
//! Only the two shapes npm's `cmd-shim` writes are recognized: a native target
//! (`"%dp0%\…\tool.exe" %*`) and an interpreter target
//! (`"%_prog%" "%dp0%\…\cli.js" %*`, where `_prog` is `node.exe` beside the shim
//! or `node` from `PATH`). Any other batch file is launched unchanged.

// The parser is compiled on every OS so its tests run on every CI leg; only
// Windows launches through it.
#![cfg_attr(not(windows), allow(dead_code))]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The program a recognized shim runs, and the arguments it puts before the
/// caller's.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ShimTarget {
    pub(crate) program: OsString,
    pub(crate) leading_args: Vec<OsString>,
}

/// The target of `program` when it is an existing npm `.cmd`/`.bat` shim whose
/// target file exists.
#[cfg(windows)]
pub(crate) fn resolve(program: &std::ffi::OsStr) -> Option<ShimTarget> {
    let path = Path::new(program);
    let is_batch = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"));
    if !is_batch || !path.is_absolute() {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let target = parse(&text, path.parent()?)?;
    // The file the shim would start: the script for an interpreter target.
    let started = target.leading_args.first().unwrap_or(&target.program);
    Path::new(started).is_file().then_some(target)
}

/// The lines every `cmd-shim` shim opens with; they define `%dp0%`.
const PROLOGUE: [&str; 8] = [
    "@ECHO off",
    "GOTO start",
    ":find_dp0",
    "SET dp0=%~dp0",
    "EXIT /b",
    ":start",
    "SETLOCAL",
    "CALL :find_dp0",
];

/// What an interpreter shim's launch line runs before `"%_prog%"`.
const INTERPRETER_LAUNCH: &str = "endLocal & goto #_undefined_# 2>NUL || title %COMSPEC% &";

/// Parse `cmd-shim` output, resolving `%dp0%` against `shim_dir`.
///
/// The whole file must have `cmd-shim`'s structure, so a hand-written wrapper
/// that does its own setup before its launch line is never bypassed.
pub(crate) fn parse(text: &str, shim_dir: &Path) -> Option<ShimTarget> {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    if !PROLOGUE.iter().all(|expected| lines.next() == Some(*expected)) {
        return None;
    }
    let body: Vec<&str> = lines.collect();
    let (launch, setup) = body.split_last()?;
    let launch = launch.strip_suffix("%*")?.trim_end();

    if let Some((before, script)) = launch.split_once("\"%_prog%\"") {
        if before.trim_end() != INTERPRETER_LAUNCH {
            return None;
        }
        let script = dp0_path(quoted(script.trim())?, shim_dir)?;
        let interpreter = interpreter(setup)?;
        let local = shim_dir.join(format!("{interpreter}.exe"));
        let program = if local.is_file() { local.into_os_string() } else { interpreter.into() };
        return Some(ShimTarget { program, leading_args: vec![script.into_os_string()] });
    }

    if !setup.is_empty() {
        return None;
    }
    let target = dp0_path(quoted(launch)?, shim_dir)?;
    target
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        .then(|| ShimTarget { program: target.into_os_string(), leading_args: Vec::new() })
}

/// The contents of `token` when it is exactly one double-quoted string.
fn quoted(token: &str) -> Option<&str> {
    let inner = token.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('"')).then_some(inner)
}

/// `token` as a path under `shim_dir`; `None` unless it starts at `%dp0%` and
/// expands no other variable.
fn dp0_path(token: &str, shim_dir: &Path) -> Option<PathBuf> {
    let relative = token.strip_prefix("%dp0%")?.trim_start_matches('\\');
    (!relative.contains('%')).then(|| shim_dir.join(relative))
}

/// The bare interpreter name from the shim's `SET "_prog=<name>"` fallback,
/// when `setup` is exactly `cmd-shim`'s interpreter-selection block.
fn interpreter<'a>(setup: &[&'a str]) -> Option<&'a str> {
    let name = setup.iter().find_map(|line| {
        line.strip_prefix("SET \"_prog=")?
            .strip_suffix('"')
            .filter(|name| !name.is_empty() && !name.contains(['%', '\\', '/']))
    })?;
    let expected = [
        format!("IF EXIST \"%dp0%\\{name}.exe\" ("),
        format!("SET \"_prog=%dp0%\\{name}.exe\""),
        ") ELSE (".to_string(),
        format!("SET \"_prog={name}\""),
        "SET PATHEXT=%PATHEXT:;.JS;=;%".to_string(),
        ")".to_string(),
    ];
    (setup.len() == expected.len() && setup.iter().zip(&expected).all(|(line, want)| line == want))
        .then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cmd-shim` output for a package whose bin is a native executable
    /// (`opencode-ai`).
    const NATIVE: &str = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\"%dp0%\\node_modules\\opencode-ai\\bin\\opencode.exe\"   %*\r\n";

    /// `cmd-shim` output for a package whose bin is a Node script
    /// (`@openai/codex`).
    const NODE: &str = "@ECHO off\r\nGOTO start\r\n:find_dp0\r\nSET dp0=%~dp0\r\nEXIT /b\r\n:start\r\nSETLOCAL\r\nCALL :find_dp0\r\n\r\nIF EXIST \"%dp0%\\node.exe\" (\r\n  SET \"_prog=%dp0%\\node.exe\"\r\n) ELSE (\r\n  SET \"_prog=node\"\r\n  SET PATHEXT=%PATHEXT:;.JS;=;%\r\n)\r\n\r\nendLocal & goto #_undefined_# 2>NUL || title %COMSPEC% & \"%_prog%\"  \"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js\" %*\r\n";

    fn shim_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn a_native_shim_runs_its_executable_with_no_leading_arguments() {
        let dir = shim_dir();
        assert_eq!(
            parse(NATIVE, dir.path()),
            Some(ShimTarget {
                program: dir.path().join(r"node_modules\opencode-ai\bin\opencode.exe").into_os_string(),
                leading_args: Vec::new(),
            })
        );
    }

    #[test]
    fn a_node_shim_runs_node_from_path_when_none_sits_beside_it() {
        let dir = shim_dir();
        assert_eq!(
            parse(NODE, dir.path()),
            Some(ShimTarget {
                program: "node".into(),
                leading_args: vec![
                    dir.path().join(r"node_modules\@openai\codex\bin\codex.js").into_os_string()
                ],
            })
        );
    }

    #[test]
    fn a_node_shim_prefers_the_node_exe_beside_it() {
        let dir = shim_dir();
        let local = dir.path().join("node.exe");
        std::fs::write(&local, b"").unwrap();
        assert_eq!(parse(NODE, dir.path()).unwrap().program, local.into_os_string());
    }

    #[test]
    fn other_batch_files_are_not_shims() {
        let dir = shim_dir();
        let native_launch = "\"%dp0%\\node_modules\\opencode-ai\\bin\\opencode.exe\"   %*";
        for text in [
            "@echo off\r\necho hello\r\n".to_string(),
            // the launch line alone, without the prologue that defines `%dp0%`
            format!("{native_launch}\r\n"),
            // a wrapper that does its own setup before launching
            NATIVE.replace(native_launch, &format!("SET TOOL_HOME=%dp0%\r\n{native_launch}")),
            // a non-executable native target
            NATIVE.replace("opencode.exe", "opencode.sh"),
            // a target that expands another variable
            NATIVE.replace("opencode-ai", "%TOOL%"),
            // a target not anchored at the shim
            NATIVE.replace("%dp0%\\node_modules", "C:\\tools"),
            // interpreter arguments before the script
            NODE.replace("\"%_prog%\"  \"", "\"%_prog%\" --flag \""),
            // an extra command in the interpreter-selection block
            NODE.replace("SET PATHEXT", "SET NODE_OPTIONS=--inspect\r\n  SET PATHEXT"),
            // an extra command before the interpreter launch
            NODE.replace("title %COMSPEC% &", "title %COMSPEC% & set X=1 &"),
        ] {
            assert_eq!(parse(&text, dir.path()), None, "{text:?}");
        }
    }

    /// The defect this module exists for: the same newline-bearing argument
    /// that `std` refuses to hand a `.cmd` reaches the shim's executable.
    #[cfg(windows)]
    #[test]
    fn a_multi_line_argument_reaches_a_native_shim_target() {
        let dir = shim_dir();
        let bin = dir.path().join(r"node_modules\opencode-ai\bin");
        std::fs::create_dir_all(&bin).unwrap();
        let system = std::env::var_os("SystemRoot").expect("SystemRoot");
        std::fs::copy(Path::new(&system).join(r"System32\whoami.exe"), bin.join("opencode.exe")).unwrap();
        let shim = dir.path().join("opencode.cmd");
        std::fs::write(&shim, NATIVE).unwrap();

        let error = std::process::Command::new(&shim).arg("line one\nline two").status().unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{error}");

        let mut command = crate::child_environment::command(&shim).unwrap();
        assert_eq!(command.get_program(), bin.join("opencode.exe").as_os_str());
        command
            .arg("line one\nline two")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.status().expect("the shim's executable starts with a multi-line argument");
    }
}
