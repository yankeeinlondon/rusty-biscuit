use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::harness::model::{
    ApprovedRuntimeCommand, HarnessPermissionProbe, PermissionAssessment, StructuredShape,
};

use super::CheckResult;

pub fn check_write_permission(
    file: &Path,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> CheckResult {
    match std::fs::metadata(file) {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(format!(
                    "{} is a directory; expected a writable file path",
                    file.display()
                ));
            }

            std::fs::OpenOptions::new()
                .write(true)
                .open(file)
                .map(|_| ())
                .map_err(|e| {
                    format!(
                        "cannot write existing file {}: {e} (filesystem write check failed; provider runtime policy may also deny writes)",
                        file.display()
                    )
                })?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            check_parent_allows_file_creation(file)?;
        }
        Err(e) => Err(format!(
            "cannot inspect {} for write access: {e}",
            file.display()
        ))?,
    }

    evaluate_provider_write_policy(file, source_path, permission_probe)
}

fn evaluate_provider_write_policy(
    file: &Path,
    source_path: &Path,
    permission_probe: Option<&dyn HarnessPermissionProbe>,
) -> CheckResult {
    let Some(probe) = permission_probe else {
        return Ok(());
    };

    match probe.can_write(file, source_path) {
        PermissionAssessment::Allowed => Ok(()),
        PermissionAssessment::Denied { reason } => Err(format!(
            "provider runtime policy denies writes to {}: {reason}",
            file.display()
        )),
        PermissionAssessment::Unknown { reason } => Err(format!(
            "provider runtime policy for {} is unknown: {reason}",
            file.display()
        )),
    }
}

fn check_parent_allows_file_creation(file: &Path) -> CheckResult {
    let parent = file
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    if !parent.exists() {
        return Err(format!(
            "cannot create {}: parent directory {} does not exist",
            file.display(),
            parent.display()
        ));
    }

    if !parent.is_dir() {
        return Err(format!(
            "cannot create {}: parent path {} is not a directory",
            file.display(),
            parent.display()
        ));
    }

    let probe_path = parent.join(format!(
        ".claudine-write-probe-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe_path);
            Ok(())
        }
        Err(e) => Err(format!(
            "cannot create {} in {}: {e} (filesystem write check failed; provider runtime policy may also deny writes)",
            file.display(),
            parent.display()
        )),
    }
}

pub(crate) fn check_json_file(file: &Path, shape: Option<&StructuredShape>) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("{} is not valid JSON: {e}", file.display()))?;
    if let Some(expected_shape) = shape {
        check_value_shape(&value, expected_shape, file)?;
    }
    Ok(())
}

pub(crate) fn check_yaml_file(file: &Path, shape: Option<&StructuredShape>) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let yaml = biscuit_file::Yaml::from_str(&content)
        .map_err(|e| format!("{} is not valid YAML: {e}", file.display()))?;
    if let Some(expected_shape) = shape {
        let json_value = yaml
            .as_json()
            .map_err(|e| format!("{} YAML-to-JSON conversion failed: {e}", file.display()))?;
        check_value_shape(&json_value, expected_shape, file)?;
    }
    Ok(())
}

pub(crate) fn check_toml_file(file: &Path) -> CheckResult {
    if !file.exists() || file.is_dir() {
        return Err(format!("file does not exist: {}", file.display()));
    }
    let content =
        fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let _: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("{} is not valid TOML: {e}", file.display()))?;
    Ok(())
}

fn check_value_shape(
    value: &serde_json::Value,
    expected: &StructuredShape,
    file: &Path,
) -> CheckResult {
    let actual = match value {
        serde_json::Value::Array(_) => StructuredShape::Array,
        serde_json::Value::Object(_) => StructuredShape::Object,
        _ => StructuredShape::Scalar,
    };
    if actual == *expected {
        Ok(())
    } else {
        Err(format!(
            "{}: root shape is {actual:?} but expected {expected:?}",
            file.display()
        ))
    }
}

pub(crate) fn check_shell_command(
    command: &ApprovedRuntimeCommand,
    show_stdout: bool,
    show_stderr: bool,
) -> CheckResult {
    let timeout = Duration::from_secs(60);
    let (exit_code, stdout, stderr) = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(crate::harness::shell::execute_approved_command(
            command, None, timeout,
        ))
    })
    .map_err(|e| format!("shell command '{}' failed: {e}", command.raw))?;

    if show_stdout && !stdout.trim().is_empty() {
        eprintln!("{stdout}");
    }
    if show_stderr && !stderr.trim().is_empty() {
        eprintln!("{stderr}");
    }

    if exit_code == 0 {
        Ok(())
    } else {
        Err(format!(
            "shell command '{}' exited with code {exit_code}",
            command.raw
        ))
    }
}
