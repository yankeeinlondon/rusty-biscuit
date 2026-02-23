use claudine::events::Provider;
use color_eyre::eyre::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum YoloMapping {
    Flag {
        flag: &'static str,
        aliases: &'static [&'static str],
    },
    FlagValue {
        flag: &'static str,
        value: &'static str,
        aliases: &'static [&'static str],
    },
    EnvVar {
        key: &'static str,
        value: &'static str,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonInteractiveMapping {
    AppendFlag {
        flag: &'static str,
    },
    EnsureEntrypoint {
        entrypoint: &'static str,
        aliases: &'static [&'static str],
    },
    EnsurePromptMode {
        provider: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderProfile {
    pub(crate) wrapper: &'static str,
    pub(crate) binary: &'static str,
    pub(crate) provider: Provider,
    pub(crate) agent_env: &'static str,
    pub(crate) yolo_mapping: YoloMapping,
    pub(crate) non_interactive_mapping: NonInteractiveMapping,
}

const PROFILES: &[ProviderProfile] = &[
    ProviderProfile {
        wrapper: "claude",
        binary: "claude",
        provider: Provider::Claude,
        agent_env: "claude",
        yolo_mapping: YoloMapping::Flag {
            flag: "--dangerously-skip-permissions",
            aliases: &[],
        },
        non_interactive_mapping: NonInteractiveMapping::AppendFlag { flag: "--print" },
    },
    ProviderProfile {
        wrapper: "codex",
        binary: "codex",
        provider: Provider::Codex,
        agent_env: "codex",
        yolo_mapping: YoloMapping::Flag {
            flag: "--dangerously-bypass-approvals-and-sandbox",
            aliases: &["--yolo"],
        },
        non_interactive_mapping: NonInteractiveMapping::EnsureEntrypoint {
            entrypoint: "exec",
            aliases: &["e"],
        },
    },
    ProviderProfile {
        wrapper: "gemini",
        binary: "gemini",
        provider: Provider::Gemini,
        agent_env: "gemini",
        yolo_mapping: YoloMapping::FlagValue {
            flag: "--approval-mode",
            value: "yolo",
            aliases: &["--yolo", "-y"],
        },
        non_interactive_mapping: NonInteractiveMapping::EnsurePromptMode { provider: "gemini" },
    },
    ProviderProfile {
        wrapper: "kimi",
        binary: "kimi",
        provider: Provider::KimiCode,
        agent_env: "kimi",
        yolo_mapping: YoloMapping::Flag {
            flag: "--yolo",
            aliases: &[],
        },
        non_interactive_mapping: NonInteractiveMapping::AppendFlag { flag: "--print" },
    },
    ProviderProfile {
        wrapper: "qwen",
        binary: "qwen",
        provider: Provider::QwenCode,
        agent_env: "qwen",
        yolo_mapping: YoloMapping::Flag {
            flag: "--yolo",
            aliases: &[],
        },
        non_interactive_mapping: NonInteractiveMapping::EnsurePromptMode { provider: "qwen" },
    },
    ProviderProfile {
        wrapper: "opencode",
        binary: "opencode",
        provider: Provider::OpenCode,
        agent_env: "opencode",
        yolo_mapping: YoloMapping::Unsupported,
        non_interactive_mapping: NonInteractiveMapping::EnsureEntrypoint {
            entrypoint: "run",
            aliases: &[],
        },
    },
    ProviderProfile {
        wrapper: "goose",
        binary: "goose",
        provider: Provider::Goose,
        agent_env: "goose",
        yolo_mapping: YoloMapping::EnvVar {
            key: "GOOSE_MODE",
            value: "auto",
        },
        non_interactive_mapping: NonInteractiveMapping::EnsureEntrypoint {
            entrypoint: "run",
            aliases: &[],
        },
    },
];

pub(crate) fn profile_for_wrapper(wrapper: &str) -> Option<&'static ProviderProfile> {
    PROFILES.iter().find(|profile| profile.wrapper == wrapper)
}

pub(crate) fn has_supported_yolo(profile: &ProviderProfile) -> bool {
    !matches!(profile.yolo_mapping, YoloMapping::Unsupported)
}

pub(crate) fn apply_yolo_mapping(
    profile: &ProviderProfile,
    args: &mut Vec<String>,
    env_overrides: &mut Vec<(String, String)>,
) -> Result<Option<String>> {
    match profile.yolo_mapping {
        YoloMapping::Flag { flag, aliases } => {
            if profile.wrapper == "qwen"
                && let Some(value) = option_value(args, "--approval-mode")
                && !value.eq_ignore_ascii_case("yolo")
            {
                bail!(
                    "--yolo conflicts with existing '--approval-mode {}' for {}",
                    value,
                    profile.wrapper
                );
            }

            if has_any_flag(args, flag, aliases) {
                return Ok(None);
            }

            args.push(flag.to_string());
            Ok(None)
        }
        YoloMapping::FlagValue {
            flag,
            value,
            aliases,
        } => {
            if has_any_flag(args, flag, aliases) {
                if let Some(existing_value) = option_value(args, flag)
                    && !existing_value.eq_ignore_ascii_case(value)
                {
                    bail!(
                        "--yolo conflicts with existing '{} {}' for {}",
                        flag,
                        existing_value,
                        profile.wrapper
                    );
                }
                return Ok(None);
            }

            args.push(flag.to_string());
            args.push(value.to_string());
            Ok(None)
        }
        YoloMapping::EnvVar { key, value } => {
            if env_overrides.iter().any(|(override_key, override_value)| {
                override_key == key && override_value == value
            }) {
                return Ok(None);
            }

            env_overrides.push((key.to_string(), value.to_string()));
            Ok(None)
        }
        YoloMapping::Unsupported => Ok(Some(format!(
            "--yolo is not supported for '{}' and was ignored",
            profile.wrapper
        ))),
    }
}

pub(crate) fn reject_direct_yolo_passthrough(
    profile: &ProviderProfile,
    args: &[String],
) -> Result<()> {
    match profile.yolo_mapping {
        YoloMapping::Flag { flag, aliases } => {
            if has_any_flag(args, flag, aliases) {
                bail!(
                    "do not pass \x1b[34m{}\x1b[0m directly to claudine {}; use Claudine's \x1b[34m--yolo\x1b[0m or \x1b[34m-y\x1b[0m switches instead. Claudine uses this CLI convention for all agents it provides a wrapper to.",
                    flag,
                    profile.wrapper
                );
            }
            Ok(())
        }
        YoloMapping::FlagValue {
            flag,
            value,
            aliases,
        } => {
            if has_any_flag(args, flag, aliases)
                && option_value(args, flag)
                    .is_some_and(|existing| existing.eq_ignore_ascii_case(value))
            {
                bail!(
                    "do not pass \x1b[34m{} {}\x1b[0m directly to claudine {}; use Claudine's \x1b[34m--yolo\x1b[0m or \x1b[34m-y\x1b[0m switches instead. Claudine uses this CLI convention for all agents it provides a wrapper to.",
                    flag,
                    value,
                    profile.wrapper
                );
            }
            Ok(())
        }
        YoloMapping::EnvVar { .. } | YoloMapping::Unsupported => Ok(()),
    }
}

pub(crate) fn apply_non_interactive_mapping(
    profile: &ProviderProfile,
    args: &mut Vec<String>,
) -> Result<()> {
    match profile.non_interactive_mapping {
        NonInteractiveMapping::AppendFlag { flag } => {
            if !has_flag(args, flag) {
                args.push(flag.to_string());
            }
            Ok(())
        }
        NonInteractiveMapping::EnsureEntrypoint {
            entrypoint,
            aliases,
        } => {
            if args
                .first()
                .is_some_and(|first| first == entrypoint || aliases.contains(&first.as_str()))
            {
                return Ok(());
            }

            args.insert(0, entrypoint.to_string());
            Ok(())
        }
        NonInteractiveMapping::EnsurePromptMode { provider } => {
            if has_flag(args, "-i") || has_flag(args, "--prompt-interactive") {
                bail!(
                    "--non-interactive conflicts with interactive prompt mode for {}",
                    provider
                );
            }

            if has_flag(args, "-p") || has_flag(args, "--prompt") || has_non_flag_positional(args) {
                return Ok(());
            }

            bail!(
                "--non-interactive for {} requires a prompt (positional or --prompt/-p)",
                provider
            );
        }
    }
}

pub(crate) fn apply_non_interactive_defaults(profile: &ProviderProfile, args: &mut Vec<String>) {
    if profile.wrapper != "opencode" {
        return;
    }

    if has_flag(args, "--model") || has_flag(args, "-m") {
        return;
    }

    let model = non_empty_env_var("OPENCODE_MODEL")
        .or_else(|| non_empty_env_var("MODEL"))
        .unwrap_or_else(|| "minimax/MiniMax-M2.5-highspeed".to_string());

    args.push("--model".to_string());
    args.push(model);
}

fn non_empty_env_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn has_non_flag_positional(args: &[String]) -> bool {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }

        if arg == "--" {
            return true;
        }

        if arg == "-m"
            || arg == "--model"
            || arg == "--output-format"
            || arg == "-o"
            || arg == "--auth-type"
            || arg == "--sandbox-image"
        {
            skip_next = true;
            continue;
        }

        if !arg.starts_with('-') {
            return true;
        }
    }

    false
}

fn has_any_flag(args: &[String], primary: &str, aliases: &[&str]) -> bool {
    if has_flag(args, primary) {
        return true;
    }

    aliases.iter().any(|alias| has_flag(args, alias))
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter()
        .any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))
}

fn option_value(args: &[String], option: &str) -> Option<String> {
    for (index, arg) in args.iter().enumerate() {
        if arg == option {
            return args.get(index + 1).cloned();
        }
        if let Some(value) = arg.strip_prefix(&format!("{option}=")) {
            return Some(value.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codex_non_interactive_ensures_exec_once() {
        let profile = profile_for_wrapper("codex").unwrap();
        let mut args = vec!["exec".to_string(), "--json".to_string()];

        apply_non_interactive_mapping(profile, &mut args).unwrap();
        apply_non_interactive_mapping(profile, &mut args).unwrap();

        assert_eq!(args, vec!["exec", "--json"]);
    }

    #[test]
    fn codex_non_interactive_prepends_exec() {
        let profile = profile_for_wrapper("codex").unwrap();
        let mut args = vec!["--json".to_string(), "summarize".to_string()];

        apply_non_interactive_mapping(profile, &mut args).unwrap();

        assert_eq!(args, vec!["exec", "--json", "summarize"]);
    }

    #[test]
    fn gemini_yolo_mapping_is_idempotent() {
        let profile = profile_for_wrapper("gemini").unwrap();
        let mut args = vec!["--approval-mode".to_string(), "yolo".to_string()];
        let mut env_overrides = Vec::new();

        apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap();
        apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap();

        assert_eq!(args, vec!["--approval-mode", "yolo"]);
    }

    #[test]
    fn gemini_yolo_conflicts_with_non_yolo_approval_mode() {
        let profile = profile_for_wrapper("gemini").unwrap();
        let mut args = vec!["--approval-mode".to_string(), "default".to_string()];
        let mut env_overrides = Vec::new();

        let error = apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap_err();
        assert!(error.to_string().contains("conflicts"));
    }

    #[test]
    fn qwen_non_interactive_rejects_prompt_interactive() {
        let profile = profile_for_wrapper("qwen").unwrap();
        let mut args = vec!["-i".to_string(), "task".to_string()];

        let error = apply_non_interactive_mapping(profile, &mut args).unwrap_err();
        assert!(error.to_string().contains("conflicts"));
    }

    #[test]
    fn opencode_yolo_warns_without_mutating_args() {
        let profile = profile_for_wrapper("opencode").unwrap();
        let mut args = vec!["run".to_string(), "status".to_string()];
        let mut env_overrides = Vec::new();

        let warning = apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap();
        assert!(warning.unwrap().contains("ignored"));
        assert_eq!(args, vec!["run", "status"]);
    }

    #[test]
    fn opencode_non_interactive_defaults_add_model_when_missing() {
        let profile = profile_for_wrapper("opencode").unwrap();
        let mut args = vec!["run".to_string(), "status".to_string()];

        apply_non_interactive_defaults(profile, &mut args);

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"minimax/MiniMax-M2.5-highspeed".to_string()));
    }

    #[test]
    fn opencode_non_interactive_defaults_respect_existing_model_flag() {
        let profile = profile_for_wrapper("opencode").unwrap();
        let mut args = vec![
            "run".to_string(),
            "--model".to_string(),
            "user-choice".to_string(),
        ];

        apply_non_interactive_defaults(profile, &mut args);

        let model_flags = args.iter().filter(|arg| arg.as_str() == "--model").count();
        assert_eq!(model_flags, 1);
        assert!(args.contains(&"user-choice".to_string()));
    }

    #[test]
    fn goose_yolo_env_override_is_idempotent() {
        let profile = profile_for_wrapper("goose").unwrap();
        let mut args = Vec::new();
        let mut env_overrides = Vec::new();

        apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap();
        apply_yolo_mapping(profile, &mut args, &mut env_overrides).unwrap();

        let unique: HashSet<_> = env_overrides.into_iter().collect();
        assert_eq!(unique.len(), 1);
        assert!(unique.contains(&("GOOSE_MODE".to_string(), "auto".to_string())));
    }

    #[test]
    fn direct_provider_yolo_flag_is_rejected_with_guidance() {
        let profile = profile_for_wrapper("claude").unwrap();
        let args = vec!["--dangerously-skip-permissions".to_string()];

        let error = reject_direct_yolo_passthrough(profile, &args).unwrap_err();
        let message = error.to_string();

        assert!(message.contains("do not pass"));
        assert!(message.contains("--dangerously-skip-permissions"));
        assert!(message.contains("--yolo"));
    }
}
