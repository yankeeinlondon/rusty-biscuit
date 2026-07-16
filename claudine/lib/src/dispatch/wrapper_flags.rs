use std::path::Path;

use crate::events::EnvironmentContext;

pub(super) fn runtime_repo_root(env: &EnvironmentContext) -> Option<&Path> {
    env.git
        .as_ref()
        .map(|git| git.repo_root.as_path())
        .or_else(|| env.repo.as_ref().map(|repo| repo.root.as_path()))
}

pub(super) fn wrapper_interactive_flag() -> Option<String> {
    wrapper_interactive_flag_from(|key| std::env::var(key).ok())
}

pub(super) fn wrapper_yolo_flag() -> Option<String> {
    wrapper_flag_from(&["YOLO", "CLAUDINE_YOLO"], |key| std::env::var(key).ok())
}

pub(super) fn wrapper_interactive_flag_from<F>(lookup: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    wrapper_flag_from(&["INTERACTIVE", "CLAUDINE_INTERACTIVE"], lookup)
}

pub(super) fn wrapper_flag_from<F>(keys: &[&str], lookup: F) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    keys.iter()
        .copied()
        .filter_map(lookup)
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}
