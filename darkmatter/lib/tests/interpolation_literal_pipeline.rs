use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::Value;
use std::collections::HashSet;
use std::process::Command;

#[test]
fn frontmatter_literal_survives_shell_bracketed_interpolation_passes() {
    let executable = std::env::current_exe().expect("test executable path should be available");
    let portable_executable = biscuit_file::to_portable_string(&executable);
    let shell_command = format!("\"{portable_executable}\" --list");
    let approved_command = format!("{portable_executable} --list");
    let expected_shell_output = Command::new(&executable)
        .arg("--list")
        .output()
        .expect("test executable should run");
    assert!(expected_shell_output.status.success());
    let expected_shell_output = String::from_utf8(expected_shell_output.stdout)
        .expect("test listing should be UTF-8")
        .trim()
        .to_string();
    let expected_value = format!("{{{{ x }}}} {expected_shell_output}");

    let source = format!(
        "---\nshell_value: \"$({shell_command})\"\nresult: \"{{{{{{ x }}}}}} {{{{ shell_value }}}}\"\n---\n"
    );
    let markdown: Markdown = source.into();
    let options = ComposeOptions::new()
        .with_pre_approved_commands(HashSet::from([approved_command]));

    let (composed, report) = markdown
        .compose_with(options)
        .expect("full compose pipeline should succeed");

    assert_eq!(
        composed.frontmatter().as_map().get("result"),
        Some(&Value::String(expected_value.clone()))
    );
    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    assert_eq!(report.replacements_applied, 0);
    assert!(report.warnings.is_empty());
}
