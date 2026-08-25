    use super::*;
    use crate::markdown::compose::cache::CacheAccessMode;
    use crate::markdown::compose::shell_expansion::types::{
        PipelineRuntime, ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
        ShellExpansionError, ShellExpansionOptions,
    };
    use crate::markdown::compose::ComposeOptions;
    use crate::markdown::frontmatter::Frontmatter;
    use serde_json::json;
    use serial_test::serial;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    fn fm_from_json(data: serde_json::Value) -> Frontmatter {
        let map: crate::markdown::types::FrontmatterMap = match data {
            serde_json::Value::Object(obj) => obj.into_iter().collect(),
            _ => Default::default(),
        };
        Frontmatter::from_map(map)
    }

    fn test_ctx() -> SourceContext {
        SourceContext::new(PathBuf::from("/test"), PathBuf::from("test"), String::new())
    }

    fn execute_frontmatter_shell_expansion(
        frontmatter: &mut Frontmatter,
        options: &ComposeOptions,
        runtime: &mut PipelineRuntime,
        pre_interpolation_snapshot: Option<&std::collections::HashMap<String, String>>,
    ) -> crate::markdown::types::MarkdownResult<FrontmatterShellExpansionReport> {
        super::execute_frontmatter_shell_expansion(
            frontmatter,
            options,
            runtime,
            pre_interpolation_snapshot,
            &test_ctx(),
        )
    }

    struct MockApproval;
    impl ShellApprovalHandler for MockApproval {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    fn make_runtime() -> PipelineRuntime {
        PipelineRuntime::new(16, CacheAccessMode::Off, None)
    }

    fn find_python() -> Option<PathBuf> {
        ["python3", "python"].into_iter().find_map(|candidate| {
            let path = which::which(candidate).ok()?;
            std::process::Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| path)
        })
    }

    #[test]
    fn execute_replaces_frontmatter_value_with_output() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "greeting": "$(echo hello world)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("greeting"), Some(&json!("hello world")));
    }

    #[test]
    fn execute_trims_output_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(echo '  padded  ')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("padded")));
    }

    #[test]
    fn execute_skips_non_shell_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42,
            "cmd": "$(echo result)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("title"), Some(&json!("Hello")));
        assert_eq!(fm.as_map().get("count"), Some(&json!(42)));
        assert_eq!(fm.as_map().get("cmd"), Some(&json!("result")));
    }

    #[test]
    fn execute_no_candidates_returns_empty_report() {
        let mut fm = fm_from_json(json!({
            "title": "Hello",
            "count": 42
        }));
        let options = ComposeOptions::new();
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 0);
        assert_eq!(report.approvals_used, 0);
    }

    #[test]
    fn execute_aborts_on_padded_malformed_whole_value() {
        // A padded whole-value `$(...)` that fails to close is skipped by the
        // strict-start scan (no candidates), so composition takes the no-op
        // early-return path — where the leak guard must still abort with the
        // underlying parse error instead of leaking the raw syntax.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "leaky": "  $(echo ok"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let message = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None) {
            Ok(_) => panic!("expected composition to abort on the malformed whole-value"),
            Err(err) => err.to_string(),
        };
        assert!(
            message.contains("Missing closing ')'"),
            "expected the missing-paren diagnostic, got: {message}"
        );
        // The raw expansion form must not survive into the frontmatter.
        assert_eq!(fm.as_map().get("leaky"), Some(&json!("  $(echo ok")));
    }

    #[test]
    fn execute_expands_padded_whole_value() {
        // A whole-value `$(...)` behind leading/trailing whitespace must be
        // scanned as a candidate and expanded — not skipped and then rejected
        // by the leak guard.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "  $(echo ok)  "
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("ok")));
    }

    #[test]
    fn execute_expands_padded_whole_value_with_suffix() {
        // The suffix grammar (`::no-cache`) is parsed after the same trimming
        // boundary, so a padded value carrying a suffix still expands.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "  $(echo ok)::no-cache  "
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("ok")));
    }

    #[test]
    fn execute_leaves_mixed_literal_unexpanded() {
        // A trimmed value that does not open `$(` stays outside the strict
        // whole-value rule: it is neither expanded nor treated as a leak.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "literal $(echo ok)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 0);
        assert_eq!(fm.as_map().get("val"), Some(&json!("literal $(echo ok)")));
    }

    #[test]
    fn execute_frontmatter_uses_stdout_only() {
        let Some(python) = find_python() else {
            return;
        };

        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": format!(
                "$({} -c \"import sys; sys.stdout.write('out'); sys.stderr.write('warn')\")",
                python.display()
            )
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("val"), Some(&json!("out")));
    }

    #[test]
    fn execute_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 1)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    fn execute_pipeline_timeout_fallback_emits_warning() {
        let temp_dir = TempDir::new().unwrap();
        // `execute_pipeline_detailed` spends one timeout budget on every `&&`
        // segment, so `echo after` must spawn and exit inside it too. The sleep
        // therefore has to overshoot the budget by a wide margin while the
        // budget stays wide enough for a slow spawn under parallel-suite load.
        // The sleeping child is killed at the timeout, so its length is free.
        let mut fm = fm_from_json(json!({
            "val": "$(sleep 10 && echo after)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_secs(2),
            timeout_behavior:
                super::super::shell_expansion::types::ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("val"), Some(&json!("after")));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    #[test]
    #[serial]
    fn execute_frontmatter_commands_concurrently() {
        let Some(python) = find_python() else {
            return;
        };

        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("barrier.py"),
            r#"import pathlib
import sys
import time

pathlib.Path(sys.argv[1]).touch()
while not pathlib.Path(sys.argv[2]).exists():
    time.sleep(0.01)
print("ready", end="")
"#,
        )
        .unwrap();
        let barrier_cmd = |own: &str, other: &str| {
            format!("$({} barrier.py {own} {other})", python.display())
        };
        let mut fm = fm_from_json(json!({
            "one": barrier_cmd("one.ready", "two.ready"),
            "two": barrier_cmd("two.ready", "one.ready")
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: Duration::from_secs(15),
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        // Each command waits until the other has started. Serial execution
        // therefore times out, while concurrent execution releases both sides
        // without depending on scheduler timing or process startup speed.
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let report = pool
            .install(|| {
                execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
            })
            .unwrap();

        assert_eq!(report.replacements, 2);
        assert_eq!(fm.as_map().get("one"), Some(&json!("ready")));
        assert_eq!(fm.as_map().get("two"), Some(&json!("ready")));
    }

    #[test]
    fn ternary_truthy_condition_runs_then_branch() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_falsy_condition_runs_else_branch() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("no")));
    }

    #[test]
    fn ternary_empty_branch_short_circuits_to_empty_string() {
        // Phase 4: the unselected then-branch must still be allowlisted, so
        // pre-approve `echo yes` and pin policy_root to an isolated dir to
        // avoid picking up the repo whitelist. Selecting the empty branch then
        // short-circuits to "" without executing any shell command.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());

        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo yes : '')"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(report.approvals_used, 0);
        assert_eq!(fm.as_map().get("out"), Some(&json!("")));
    }

    #[test]
    fn ternary_then_branch_must_be_allowlisted_even_when_else_selected() {
        // Phase 4.3: an unallowlisted command in the then-branch fails the
        // entire directive even when the else-branch (Empty) is selected at
        // runtime. The then-branch is a multi-token shell command (`echo
        // unapproved`) — the §2 ladder classifies it as a command requiring
        // approval, not as a value branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo unapproved : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("approval"),
            "expected approval-required error for unselected branch, got: {msg}"
        );
    }

    #[test]
    fn ternary_else_branch_must_be_allowlisted_even_when_then_selected() {
        // Phase 4.3: an unallowlisted command in the else-branch fails the
        // entire directive even when the then-branch is selected at runtime.
        // The else-branch is a multi-token shell command (`echo unapproved`).
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? '' : echo unapproved)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("approval"),
            "expected approval-required error for unselected branch, got: {msg}"
        );
    }

    #[test]
    fn ternary_partial_preapproval_fails_with_unselected_branch_command() {
        // Phase 4.3: pre-approving only the then-branch's command is not
        // enough — the else-branch's command must also be in the reachable
        // pre-approved set even though it would not have been selected.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());
        // `echo no` intentionally omitted.

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err = match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None)
        {
            Ok(_) => panic!("expected directive to fail"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("not pre-approved") && msg.contains("echo no"),
            "expected pre-approval rejection naming `echo no`, got: {msg}"
        );
    }

    #[test]
    fn ternary_both_branches_allowlisted_succeeds() {
        // Phase 4.2: when every reachable command is allowlisted, the
        // directive succeeds and the selected branch's output is used.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo yes".to_string());
        approved.insert("echo no".to_string());

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_with_brace_wrapped_condition_resolves_via_seed_state() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_value": true,
            "out": "$({{has_value}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("present")));
    }

    #[test]
    fn ternary_with_stringified_false_condition_selects_else_branch() {
        // Review finding 2: when a frontmatter value is itself rendered from
        // a template (`has_spec: "{{ false }}"`), the post-interpolation
        // value is the string `"false"`. The condition `{{has_spec}}` must
        // be recognized as boolean-false — not as a truthy non-empty string
        // — and select the else-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": "false",
            "spec_file": "$({{has_spec}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("spec_file"), Some(&json!("")));
    }

    #[test]
    fn ternary_with_stringified_true_condition_selects_then_branch() {
        // Symmetric to the false case: a stringified `"true"` condition
        // selects the then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": "true",
            "spec_file": "$({{has_spec}} ? echo present : '')"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("spec_file"), Some(&json!("present")));
    }

    #[test]
    fn ternary_condition_interpolation_cannot_bleed_into_then_branch_executable() {
        // Review finding 1: a condition variable whose value contains
        // top-level `?` / `:` punctuation must not shift the then-branch
        // boundary or let condition text become a branch executable. With
        // the original snapshot anchoring the branch slices, the
        // then-branch runs the static `basename README.md` (or doesn't run
        // at all, depending on how the rewritten condition evaluates), but
        // it never tokenizes anything from the condition text.
        let temp_dir = TempDir::new().unwrap();
        // `cond` is itself the literal string `"true ? date : false"`. Once
        // it lands in the directive value, a naive split-of-resolved would
        // pick `date` as the then-branch executable.
        let mut fm = fm_from_json(json!({
            "cond": "true ? date : false",
            "out": "$(true ? date : false ? basename README.md : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{cond}} ? basename README.md : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename README.md".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
                .unwrap();
        assert_eq!(report.replacements, 1);
        // Result is either `README` (basename ran from the then-branch) or
        // `""` (else-branch selected); either way, `date` from the
        // condition text must not be the executable that ran.
        let out = fm
            .as_map()
            .get("out")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        assert!(
            out == "README" || out.is_empty(),
            "expected branch text to remain anchored to the original snapshot; got {out:?}"
        );
    }

    #[test]
    fn ternary_branch_arg_interpolation_cannot_inject_new_action() {
        // Review 2 finding 1: an interpolation in argument position must
        // not be able to introduce `&& date` (or any other chain
        // continuation) after the executable-interpolation check has
        // already accepted the branch. Even though the then-branch's
        // static executable is `basename`, a malicious `spec` value with
        // an embedded `&& date` would extend the pipeline to two
        // additional actions. The shape-preservation guard must catch the
        // action-count drift and reject the directive.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": true,
            // Crafted value: closes the outer single quote, chains `date`,
            // then reopens a quote so the surrounding directive still
            // tokenizes cleanly.
            "spec": "README.md' && date && echo '",
            "out": "$({{has_spec}} ? basename '{{spec}}' : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{has_spec}} ? basename '{{spec}}' : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename".to_string());
        approved.insert("date".to_string());
        approved.insert("echo".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected directive to fail with a shape-preservation error"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action") || msg.contains("introduce new chain operators"),
            "expected pipeline-shape error mentioning action count, got: {msg}"
        );
    }

    #[test]
    fn ternary_branch_quoted_interpolation_breakout_is_rejected() {
        // Review 2 finding 1: a `{{spec}}` interpolation inside double
        // quotes must not be able to introduce a closing `"` followed by
        // additional commands. Depending on how the resolved text lands,
        // the directive is rejected either by the shape guard (action
        // count drift) or by the tokenizer (unterminated quote). Both
        // refuse to execute the smuggled command — that is what matters.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "has_spec": true,
            "spec": "README.md\" && evil",
            "out": "$({{has_spec}} ? basename \"{{spec}}\" : '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert(
            "out".to_string(),
            "$({{has_spec}} ? basename \"{{spec}}\" : '')".to_string(),
        );

        let mut approved = std::collections::HashSet::new();
        approved.insert("basename".to_string());
        approved.insert("evil".to_string());

        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected quote-breakout to fail"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action")
                || msg.contains("introduce new chain operators")
                || msg.contains("Unterminated double quote")
                || msg.contains("Unterminated single quote"),
            "expected pipeline-shape or tokenizer rejection, got: {msg}"
        );
    }

    #[test]
    fn non_ternary_arg_interpolation_cannot_inject_new_action() {
        // The same static-command-invariant applies to bare (non-ternary)
        // pipelines: an interpolated argument value may not introduce
        // additional `&&` / `||` actions even when the executable token is
        // statically `basename`. In a real compose run, frontmatter
        // interpolation has already rewritten `{{spec}}` before shell
        // expansion runs — so `out`'s value here is the RESOLVED text and
        // the snapshot carries the ORIGINAL with the placeholder intact.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "spec": "README.md' && date && echo '",
            "out": "$(basename 'README.md' && date && echo '')"
        }));
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert("out".to_string(), "$(basename '{{spec}}')".to_string());

        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let err =
            match execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, Some(&snapshot))
            {
                Ok(_) => panic!("expected non-ternary directive to fail"),
                Err(err) => err,
            };
        let msg = err.to_string();
        assert!(
            msg.contains("changed from 1 action") || msg.contains("introduce new chain operators"),
            "expected pipeline-shape error mentioning action count, got: {msg}"
        );
    }

    #[test]
    fn ternary_condition_supports_infix_and() {
        // Review-3 high finding: condition-mode infix `&&` must lower to
        // `and(a, b)` and select the then-branch when both operands are
        // truthy. The non-condition `parse` entrypoint refuses bare `&&`
        // outside `{{ }}` interpolation; only `parse_condition` accepts it.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": true,
            "b": true,
            "out": "$(a && b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_infix_and_selects_else_when_one_falsy() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": true,
            "b": false,
            "out": "$(a && b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(fm.as_map().get("out"), Some(&json!("no")));
        assert_eq!(report.replacements, 1);
    }

    #[test]
    fn ternary_condition_supports_infix_or() {
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "a": false,
            "b": true,
            "out": "$(a || b ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_supports_negation() {
        // Condition-mode parses `!flag` as `not(flag)`.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(!flag ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_condition_supports_comparison() {
        // The spec's "single boolean expression" contract includes
        // comparisons. `count == 3` must evaluate to true and select the
        // then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "count": 3,
            "out": "$(count == 3 ? echo equal : echo unequal)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("equal")));
    }

    #[test]
    fn ternary_condition_supports_nested_expression_ternary() {
        // Condition-mode supports `?:` inside the condition expression
        // itself. Here `(flag ? true : false)` evaluates to `true`, which
        // selects the outer then-branch.
        let temp_dir = TempDir::new().unwrap();
        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$((flag ? true : false) ? echo yes : echo no)"
        }));
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(MockApproval)),
            ..Default::default()
        });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("yes")));
    }

    #[test]
    fn ternary_branch_url_with_colon_argument_executes() {
        // Review 2 finding 2: a bare `:` inside a branch argument (here
        // part of a URL) must not be misclassified as a nested ternary.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo http://example.com".to_string());
        approved.insert("echo none".to_string());

        let mut fm = fm_from_json(json!({
            "flag": true,
            "out": "$(flag ? echo http://example.com : echo none)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("http://example.com")));
    }

    // ── §2 value branches and preflight ───────────────────────────────────

    #[test]
    fn ternary_value_branch_resolves_literal_fallback() {
        // The else-branch is a string-literal value. When selected it resolves
        // to that value with no shell invocation; the then-branch command must
        // still be allowlisted.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo run".to_string());

        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo run : 'fallback')"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(report.approvals_used, 0);
        assert_eq!(fm.as_map().get("out"), Some(&json!("fallback")));
    }

    #[test]
    fn ternary_value_branch_absent_property_resolves_to_empty() {
        // A bare property name that does not exist resolves to `null`, which
        // renders as an empty string.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("echo run".to_string());

        let mut fm = fm_from_json(json!({
            "flag": false,
            "out": "$(flag ? echo run : dm_absent_property_xyz)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("")));
    }

    #[test]
    fn ternary_doc_namespace_branch_resolves_property_over_executable() {
        // `doc.echo` resolves the frontmatter property even though `echo` is a
        // real executable on PATH.
        let temp_dir = TempDir::new().unwrap();
        let mut approved = std::collections::HashSet::new();
        approved.insert("cat README".to_string());

        let mut fm = fm_from_json(json!({
            "flag": false,
            "echo": "property-value",
            "out": "$(flag ? cat README : doc.echo)"
        }));
        let options = ComposeOptions::new()
            .with_pre_approved_commands(approved)
            .with_shell(ShellExpansionOptions {
                policy_root: Some(temp_dir.path().to_path_buf()),
                ..Default::default()
            });
        let mut runtime = make_runtime();

        let report =
            execute_frontmatter_shell_expansion(&mut fm, &options, &mut runtime, None).unwrap();
        assert_eq!(report.replacements, 1);
        assert_eq!(fm.as_map().get("out"), Some(&json!("property-value")));
    }

    #[test]
    fn preflight_enumerates_command_branch_and_excludes_value_branch() {
        // Discovery enumerates reachable command pipelines without evaluating
        // the condition. The string-literal value branch contributes no
        // command, so only the command branch surfaces for approval.
        let directive = super::parse_shell_value(
            "$(some_undefined_flag ? echo yes : 'literal')",
            "out",
            None,
            &test_ctx(),
        )
        .unwrap()
        .unwrap();
        let fm = fm_from_json(json!({}));
        let options = ComposeOptions::new();

        let pipelines =
            super::directive_reachable_pipelines(&directive, &fm, &options, &test_ctx()).unwrap();
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].actions[0].command.executable, "echo");
    }

    #[test]
    fn preflight_excludes_safe_function_branch_from_approval() {
        // A `name(...)` expression function is a safe function — it spawns no
        // process and is never enumerated for approval.
        let directive = super::parse_shell_value(
            "$(flag ? cat file : markdown_body_empty('x'))",
            "out",
            None,
            &test_ctx(),
        )
        .unwrap()
        .unwrap();
        let fm = fm_from_json(json!({}));
        let options = ComposeOptions::new();

        let pipelines =
            super::directive_reachable_pipelines(&directive, &fm, &options, &test_ctx()).unwrap();
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].actions[0].command.executable, "cat");
    }
