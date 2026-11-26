//! The provider selection chain and the model requirement.

use super::*;

/// One link of a provider or model selection chain, as the report lists it.
pub(super) struct Link {
    pub(super) label: String,
    pub(super) state: String,
}

impl Check {
    /// Walks the selection chain; returns the chosen provider when it is
    /// known and installed, recording a finding otherwise.
    pub(super) fn select_provider(&mut self, task: &PromptTask<'_>) -> Option<Provider> {
        let sequence = self.mode == Mode::Sequence;
        let prompt_name = file_name(&task.prompt.resolved_path);
        let root_name = file_name(&self.root_ref().resolved_path);
        let mut links: Vec<(String, Option<String>, &'static str)> = vec![
            ("--provider flag".into(), self.shared.explicit_provider().map(|p| p.to_string()), "not given"),
            ("agent= setter".into(), self.setters.get("agent").map(scalar_text), "not given"),
        ];
        if sequence {
            let unset = if task.step_agent.is_some() { "not set (see `agent:` above)" } else { "not set" };
            links.push(("with.agent".into(), task.with.get("agent").map(scalar_text), unset));
        }
        links.push((
            format!("{prompt_name} frontmatter agent"),
            task.prompt.markdown.frontmatter().as_map().get("agent").map(scalar_text),
            "not set",
        ));
        if sequence && task.prompt.resolved_path != self.root_ref().resolved_path {
            links.push((
                format!("{root_name} agent"),
                self.root_ref().markdown.frontmatter().as_map().get("agent").map(scalar_text),
                "not set",
            ));
        }

        let chosen = links.iter().find_map(|(label, value, _)| value.clone().map(|value| (label.clone(), value)));
        let Some((label, name)) = chosen else {
            let chain = chain_notes(
                links.iter().map(|(label, _, unset)| Link { label: label.clone(), state: (*unset).to_string() }),
            );
            let (severity, message) = if self.stderr_is_tty && !sequence {
                (Severity::Warning, "no provider for this document; an interactive run asks you to choose")
            } else if self.stderr_is_tty {
                (Severity::Warning, "no provider for this task; an interactive run asks you to choose")
            } else if sequence {
                (Severity::Error, "no provider for this task; a non-interactive run fails here")
            } else {
                (Severity::Error, "no provider for this document; a non-interactive run fails here")
            };
            let hint = match &task.step_agent {
                Some(written) => format!("add `with: {{ agent: {written} }}`, or run with `--provider {written}`"),
                None => format!("add `agent: <provider>` to {prompt_name}, or run with `--provider <provider>`"),
            };
            let mut finding =
                Diagnostic::actionable(PROVIDER_UNRESOLVED, severity, message, hint).with_trail(task.trail.clone());
            for note in chain {
                finding = finding.with_note(note);
            }
            self.sink.push(task.subject, finding);
            return None;
        };

        let Some(provider) = Provider::parse_cli_name(&name) else {
            let hint = match Provider::fuzzy_match_cli_name(&name) {
                Some(near) => format!("did you mean `{near}`?"),
                None => format!("use one of: {}", claudine::provider_id::PROVIDERS_DISPLAY_ORDER.iter().map(|p| p.as_slug().to_string()).collect::<Vec<_>>().join(", ")),
            };
            self.sink.push(
                task.subject,
                Diagnostic::actionable(
                    PROVIDER_UNKNOWN,
                    Severity::Error,
                    format!("`{name}` ({label}) is not a known provider"),
                    hint,
                )
                .with_trail(task.trail.clone()),
            );
            self.sink.not_checked(task.subject, "model requirement", "no provider", None);
            return None;
        };
        let binary = provider_info(provider).binary;
        if sniff::programs::ExecutableIndex::build_path_only().find(binary).is_none() {
            self.sink.push(
                task.subject,
                Diagnostic::actionable(
                    PROVIDER_NOT_INSTALLED,
                    Severity::Error,
                    format!("the {provider} provider ({label}) is not installed: `{binary}` is not on PATH"),
                    format!("install {provider}, or choose another provider"),
                )
                .with_trail(task.trail.clone()),
            );
        }
        Some(provider)
    }

    /// Whether the chosen provider needs a model this launch cannot find.
    pub(super) fn check_model(&mut self, task: &PromptTask<'_>, provider: Provider) {
        let info = provider_info(provider);
        // A sequence task always launches non-interactively.
        let non_interactive = self.mode == Mode::Sequence || !self.stderr_is_tty;
        if !(non_interactive && info.model_required_in_non_tty) {
            return;
        }
        let environment = self.invocation.as_ref().map(|invocation| invocation.environment().clone()).unwrap_or_default();
        let found = self.shared.model.is_some()
            || info.model_env_vars.iter().any(|var| environment.get(*var).is_some_and(|value| !value.is_empty()))
            || environment.get("MODEL").is_some_and(|value| !value.is_empty())
            || task.with.get("model").is_some()
            || task.prompt.markdown.frontmatter().as_map().get("model").is_some()
            || crate::commands::wrap::profile::profile_for_provider(provider)
                .and_then(|profile| profile.configured_default_model())
                .is_some();
        if found {
            return;
        }
        let variables = info.model_env_vars.iter().map(|var| format!("`{var}`")).collect::<Vec<_>>();
        let hint = if variables.is_empty() {
            "pass `-m <model>`, or add `model:` to the prompt's frontmatter".to_string()
        } else {
            format!("pass `-m <model>`, set {}, or add `model:` to the prompt's frontmatter", variables.join(" or "))
        };
        self.sink.push(
            task.subject,
            Diagnostic::actionable(
                MODEL_REQUIRED,
                Severity::Error,
                format!("{provider} needs a model in a non-interactive run, and none was found"),
                hint,
            )
            .with_trail(task.trail.clone()),
        );
    }
}

/// The chain's lines, numbered, with the states aligned.
pub(super) fn chain_notes(links: impl Iterator<Item = Link>) -> Vec<String> {
    let links: Vec<Link> = links.collect();
    let width = links.iter().map(|link| link.label.chars().count()).max().unwrap_or(0) + 2;
    links
        .iter()
        .enumerate()
        .map(|(index, link)| format!("{}. {:<width$}{}", index + 1, link.label, link.state))
        .collect()
}
