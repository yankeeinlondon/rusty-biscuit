use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::Value;
use std::collections::HashSet;
use std::process::Command;

const SHELL_COMMAND: &str = "rustc --print sysroot";

#[test]
fn frontmatter_literal_survives_shell_bracketed_interpolation_passes() {
    let expected_shell_output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .expect("rustc should be available while running Rust tests");
    assert!(expected_shell_output.status.success());
    let expected_shell_output = String::from_utf8(expected_shell_output.stdout)
        .expect("rustc sysroot should be UTF-8")
        .trim()
        .to_string();
    let expected_value = format!("{{{{ x }}}} {expected_shell_output}");

    let source = format!(
        "---\nshell_value: \"$({SHELL_COMMAND})\"\nresult: \"{{{{{{ x }}}}}} {{{{ shell_value }}}}\"\n---\n"
    );
    let markdown: Markdown = source.into();
    let options = ComposeOptions::new()
        .with_pre_approved_commands(HashSet::from([SHELL_COMMAND.to_string()]));

    let (composed, report) = markdown
        .compose_with(options)
        .expect("full compose pipeline should succeed");

    assert_eq!(
        composed.frontmatter().as_map().get("result"),
        Some(&Value::String(expected_value.clone()))
    );
    assert_eq!(
        composed.as_string(),
        format!(
            "---\nshell_value: {expected_shell_output}\nresult: '{{{{ x }}}} {expected_shell_output}'\n---\n"
        )
    );
    assert_eq!(report.frontmatter_interpolations_applied, 1);
    assert_eq!(report.frontmatter_shell_expansions_applied, 1);
    assert_eq!(report.replacements_applied, 0);
    assert!(report.warnings.is_empty());
}
