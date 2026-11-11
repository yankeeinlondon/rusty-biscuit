//! Interpreter selection without running an interpreter: version parsing,
//! `PATH` lookup, the gates' argv-prefix file, and the all-missing error.
//! Probing real candidates is tested in the `ci-push` integration target.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rstest::rstest;

use crate::ci_push::process::Cancellation;
use crate::ci_push::python::{
    encode_prefix, find_executable, parse_version, PythonSelectionError, Rejection,
    PYTHON_ARGS_FILE_VARIABLE, PYTHON_VARIABLE,
};
use crate::ci_push::PythonCommand;

#[rstest]
#[case("3.12\n", Some((3, 12)))]
#[case("3.10\r\n", Some((3, 10)))]
#[case("2.7", Some((2, 7)))]
#[case("10.0", Some((10, 0)))]
#[case("", None)]
#[case("3", None)]
#[case("3.x", None)]
#[case("3.-1", None)]
#[case("Python 3.12", None)]
#[case("3.12.1", None)]
fn the_version_line_is_major_dot_minor(#[case] stdout: &str, #[case] expected: Option<(u32, u32)>) {
    assert_eq!(parse_version(stdout.as_bytes()), expected);
}

#[test]
fn the_prefix_file_terminates_every_argument_with_nul() {
    let prefix = vec![
        OsString::from("-3"),
        OsString::from("with space"),
        OsString::from(""),
    ];
    assert_eq!(encode_prefix(&prefix).unwrap(), b"-3\0with space\0\0");
    assert_eq!(encode_prefix(&[]).unwrap(), b"");
}

#[test]
fn the_gate_environment_names_the_interpreter_and_its_prefix_file() {
    let staging = tempfile::tempdir().unwrap();
    let python = PythonCommand {
        executable: staging.path().join("bin").join("py"),
        prefix: vec![OsString::from("-3"), OsString::from("-X utf8 ü")],
    };

    let [(python_key, python_value), (file_key, file_value)] =
        python.gate_environment(staging.path()).unwrap();

    assert_eq!(python_key, PYTHON_VARIABLE);
    assert_eq!(PathBuf::from(python_value), python.executable);
    assert_eq!(file_key, PYTHON_ARGS_FILE_VARIABLE);
    let file = PathBuf::from(file_value);
    assert!(file.is_absolute(), "{}", file.display());
    assert_eq!(std::fs::read(file).unwrap(), "-3\0-X utf8 ü\0".as_bytes());
}

fn executable(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    });
    std::fs::write(&path, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn lookup_returns_the_first_absolute_match_on_path() {
    let root = tempfile::tempdir().unwrap();
    let (first, second) = (root.path().join("first"), root.path().join("second"));
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let expected = executable(&second, "python3");
    let path = std::env::join_paths([Path::new("relative-bin"), &first, &second]).unwrap();

    assert_eq!(find_executable(Some(&path), "python3"), Some(expected));
    assert_eq!(find_executable(Some(&path), "python"), None);
    assert_eq!(find_executable(None, "python3"), None);
}

#[cfg(unix)]
#[test]
fn lookup_skips_a_file_without_execute_permission() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let path = executable(root.path(), "python3");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_eq!(
        find_executable(Some(root.path().as_os_str()), "python3"),
        None
    );
}

#[test]
fn with_no_candidate_on_path_one_error_names_every_candidate() {
    let empty = tempfile::tempdir().unwrap();

    let error = PythonCommand::select_with(
        Some(empty.path().as_os_str()),
        Duration::from_secs(2),
        &Cancellation::new(),
    )
    .unwrap_err();

    let PythonSelectionError::NoInterpreter(attempts) = &error else {
        panic!("expected NoInterpreter, got {error:?}");
    };
    let tried: Vec<&str> = attempts
        .iter()
        .map(|attempt| attempt.candidate.as_str())
        .collect();
    let expected: &[&str] = if cfg!(windows) {
        &["python3", "py -3", "python"]
    } else {
        &["python3", "python"]
    };
    assert_eq!(tried, expected);
    assert!(attempts
        .iter()
        .all(|attempt| attempt.rejection == Rejection::NotFound));
    let message = error.to_string();
    for candidate in expected {
        assert!(
            message.contains(&format!("{candidate} not found on PATH")),
            "{message}"
        );
    }
}
