//! `has_alias`, `has_builtin_function`, `has_user_function`, and `can_execute`:
//! login-shell classification through the request's
//! [`ShellProbe`](crate::markdown::compose::shell_expansion::probe::ShellProbe)
//! (R8-R10).

use serde_json::Value;

use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::shell_expansion::probe::ShellClasses;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding {
        canonical: "has_alias",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(has_alias_fn)),
    },
    FunctionBinding {
        canonical: "has_builtin_function",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(has_builtin_function_fn)),
    },
    FunctionBinding {
        canonical: "has_user_function",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(has_user_function_fn)),
    },
    FunctionBinding {
        canonical: "can_execute",
        aliases: &[],
        evaluation: EvaluationMode::Context,
        handler: Some(FunctionHandler::Context(can_execute_fn)),
    },
];

/// Like `has_command`, a null, non-string, or empty name is simply not a
/// command, so it reads as `false` without launching the shell.
fn classify(args: &[Value], context: &ResolutionContext) -> ShellClasses {
    match args.first() {
        Some(Value::String(name)) => context.shell_probe.classify(name),
        _ => ShellClasses::default(),
    }
}

fn has_alias_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(Value::Bool(classify(args, context).alias))
}

fn has_builtin_function_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(Value::Bool(classify(args, context).builtin))
}

fn has_user_function_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    Ok(Value::Bool(classify(args, context).function))
}

/// `has_binary` first: a name found without the shell never launches a
/// profile, and one launch answers the other three disjuncts together.
fn can_execute_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    if super::has_command_fn(args, context)? == Value::Bool(true) {
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(classify(args, context).any()))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::dispatch_fs;
    use crate::markdown::compose::expression::ResolutionContext;
    use crate::markdown::compose::shell_expansion::probe::ShellProbe;

    fn context(probe: ShellProbe) -> ResolutionContext {
        ResolutionContext { shell_probe: probe, ..ResolutionContext::default() }
    }

    fn call(name: &str, argument: Value, context: &ResolutionContext) -> Value {
        dispatch_fs(name, &[argument], context)
            .expect("registered context function")
            .expect("shell probes never error")
    }

    #[test]
    fn non_string_and_empty_names_are_false_without_a_launch() {
        let context = context(ShellProbe::from_environment(&std::collections::HashMap::from([(
            "SHELL".to_string(),
            "/bin/bash".to_string(),
        )])));
        for function in ["has_alias", "has_builtin_function", "has_user_function", "can_execute"] {
            for argument in [Value::Null, json!(1), json!(["cd"]), json!("")] {
                assert_eq!(call(function, argument.clone(), &context), json!(false), "{function}({argument})");
            }
        }
        assert_eq!(context.shell_probe.launches(), 0);
    }

    #[cfg(unix)]
    mod unix {
        use std::collections::HashMap;
        use std::path::{Path, PathBuf};
        use std::time::{Duration, Instant};

        use serde_json::json;

        use super::{call, context};
        use crate::markdown::compose::shell_expansion::probe::ShellProbe;

        fn executable(path: &Path, body: &str) {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(path, body).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        /// A POSIX-`sh` stand-in named `bash`, so the bash dialect is selected,
        /// that answers with the kinds listed in `STUB_KINDS`.
        fn stub_bash(dir: &Path) -> PathBuf {
            let shell = dir.join("bash");
            executable(
                &shell,
                "#!/bin/sh\nfor kind in $STUB_KINDS; do echo \"darkmatter-shell-probe:$kind\"; done\n",
            );
            shell
        }

        fn stub_probe(shell: &Path, kinds: &str) -> ShellProbe {
            ShellProbe::from_environment(&HashMap::from([(
                "SHELL".to_string(),
                shell.to_string_lossy().into_owned(),
            )]))
            .with_child_environment("STUB_KINDS", Some(kinds))
            .with_timeout(Duration::from_secs(30))
        }

        /// AC11: every combination of the four disjuncts, with the binary
        /// check short-circuiting before any shell launch.
        #[test]
        fn can_execute_is_the_or_of_all_sixteen_combinations() {
            let dir = tempfile::tempdir().unwrap();
            let shell = stub_bash(dir.path());
            let binary = dir.path().join("present-binary");
            executable(&binary, "#!/bin/sh\n");
            let missing = dir.path().join("missing-binary");

            for mask in 0_u8..16 {
                let (is_binary, is_alias, is_builtin, is_function) =
                    (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0, mask & 8 != 0);
                let kinds = [(is_alias, "alias"), (is_builtin, "builtin"), (is_function, "function")]
                    .into_iter()
                    .filter_map(|(on, kind)| on.then_some(kind))
                    .collect::<Vec<_>>()
                    .join(" ");
                let name = if is_binary { &binary } else { &missing };
                let context = context(stub_probe(&shell, &kinds));

                let result = call("can_execute", json!(name.to_string_lossy()), &context);

                let expected = is_binary || is_alias || is_builtin || is_function;
                assert_eq!(result, json!(expected), "binary={is_binary} kinds=[{kinds}]");
                assert_eq!(
                    context.shell_probe.launches(),
                    usize::from(!is_binary),
                    "binary={is_binary}: a found binary must not launch the shell"
                );
            }
        }

        #[test]
        fn each_predicate_reads_only_its_own_kind() {
            let dir = tempfile::tempdir().unwrap();
            let shell = stub_bash(dir.path());
            for (kinds, alias, builtin, function) in [
                ("alias", true, false, false),
                ("builtin", false, true, false),
                ("function", false, false, true),
                ("builtin function", false, true, true),
                ("file keyword", false, false, false),
            ] {
                let context = context(stub_probe(&shell, kinds));
                assert_eq!(call("has_alias", json!("x"), &context), json!(alias), "{kinds}");
                assert_eq!(call("has_builtin_function", json!("x"), &context), json!(builtin), "{kinds}");
                assert_eq!(call("has_user_function", json!("x"), &context), json!(function), "{kinds}");
            }
        }

        #[test]
        fn a_failing_or_hanging_shell_is_false() {
            let dir = tempfile::tempdir().unwrap();
            let failing = dir.path().join("zsh");
            executable(&failing, "#!/bin/sh\necho darkmatter-shell-probe:alias\nexit 3\n");
            let context_failing = context(stub_probe(&failing, ""));
            assert_eq!(call("has_alias", json!("ll"), &context_failing), json!(false));

            let hanging = dir.path().join("fish");
            executable(&hanging, "#!/bin/sh\necho darkmatter-shell-probe:alias\nsleep 60\n");
            let context_hanging = context(stub_probe(&hanging, "").with_timeout(Duration::from_millis(300)));
            let started = Instant::now();
            assert_eq!(call("can_execute", json!("ll"), &context_hanging), json!(false));
            assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
        }
    }
}
