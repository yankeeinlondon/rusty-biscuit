use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::prelude::StatusBlock;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use biscuit_terminal::utils::layout::{Length, TargetValue};
use claudine::provider::Provider;

use claudine::composition::ProviderTail;
use claudine::harness::ProcessTermination;
use claudine::secrets::Redactor;
use claudine::stream::semantic::SemanticErrorKind;

use crate::commands::wrap::profile::ModelSource;
use crate::commands::wrap::provider_tail_report::{tail_names_switch, tail_redactor, tail_summary};
use crate::log;
use crate::output::native_exit::NativeExit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentErrorCategory {
    Configuration,
    AgentNative,
    ApiRemote,
    Interrupted,
}

impl From<SemanticErrorKind> for AgentErrorCategory {
    fn from(kind: SemanticErrorKind) -> Self {
        match kind {
            SemanticErrorKind::Configuration => AgentErrorCategory::Configuration,
            SemanticErrorKind::AgentNative => AgentErrorCategory::AgentNative,
            SemanticErrorKind::ApiRemote => AgentErrorCategory::ApiRemote,
            SemanticErrorKind::Interrupted => AgentErrorCategory::Interrupted,
            // Unknown maps to AgentNative since it has the broadest "something
            // went wrong with the agent" framing without overclaiming a
            // specific upstream cause.
            SemanticErrorKind::Unknown => AgentErrorCategory::AgentNative,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SuggestionStyle {
    BareList,
    DidYouMean,
}

/// Typed cause distilled from a provider's native (non-stream) process exit.
///
/// This is deliberately **separate** from the structured-stream
/// `stream/providers/vocabulary.rs` classification: that table classifies
/// semantic stream error *events*, whereas these are process-level exits that
/// only the wrapper observes. The two must not be conflated.
///
/// Variants are listed in classification precedence: the first that matches
/// wins, so a stronger cause is never overwritten by a weaker one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeCliCause {
    /// The user interrupted the run.
    Interrupted,
    /// A Claudine timeout stopped the provider.
    TimedOut,
    /// The provider binary (or a command it needed) could not be found.
    MissingBinary,
    /// An authentication or permission failure.
    AuthOrPermission,
    /// The provider's API layer reported an error.
    ApiFailure,
    /// The provider could not resolve the requested model.
    ModelNotFound { suggestions: Option<Vec<String>> },
    /// The provider rejected its argv during argument parsing. Carries the
    /// switch the diagnostic named, when it named one.
    ArgumentRejected { switch: Option<String> },
    /// A required argument was missing from the command.
    MissingArgument,
}

/// A classified exit plus the diagnostic line that decided it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeClassification {
    pub(crate) cause: NativeCliCause,
    /// The provider line whose signature matched; `None` for causes decided
    /// by termination or exit code alone.
    pub(crate) line: Option<String>,
    /// That line's stream already reached the terminal verbatim.
    pub(crate) line_shown: bool,
}

/// Signatures for each text-backed cause. Each entry is backed by a positive
/// and a near-miss fixture in the tests; a phrase that also occurs in
/// ordinary auth or API messages (such as a bare `invalid argument`) is not
/// a signature.
const AUTH_SIGNATURES: &[&str] = &[
    "permission denied",
    "access denied",
    "not authorized",
    "unauthorized",
    "authentication failed",
    "authentication error",
    "authentication required",
    "invalid api key",
];
const MODEL_SIGNATURES: &[&str] = &["providermodelnotfounderror", "model not found", "invalid model"];
const ARGUMENT_SIGNATURES: &[&str] = &[
    "unrecognized argument",
    "unknown flag",
    "unknown option",
    "unexpected argument",
];
const MISSING_ARGUMENT_SIGNATURES: &[&str] = &[
    "missing required argument",
    "the following required arguments were not provided",
];

/// Classify a provider's native process exit.
///
/// Precedence: interruption, timeout, missing binary,
/// authentication/permission, API failure, model not found, argument
/// rejected, missing argument. Text signatures are read from stderr, then
/// stdout. An exit that matches nothing returns `None`; so does one with no
/// captured evidence beyond its exit code.
pub(crate) fn classify_native_exit(exit: &NativeExit) -> Option<NativeClassification> {
    let decided = |cause| {
        Some(NativeClassification {
            cause,
            line: None,
            line_shown: false,
        })
    };
    if exit.termination == ProcessTermination::Interrupted
        || exit.exit_code == 130
        || exit.exit_code == 143
    {
        return decided(NativeCliCause::Interrupted);
    }
    if exit.termination == ProcessTermination::TimedOut {
        return decided(NativeCliCause::TimedOut);
    }
    if exit.termination == ProcessTermination::LaunchFailed {
        return decided(NativeCliCause::MissingBinary);
    }
    if !exit.failed() {
        return None;
    }
    let with_line = |cause, (line, shown): (&str, bool)| NativeClassification {
        cause,
        line: Some(line.to_string()),
        line_shown: shown,
    };
    if exit.exit_code == 127
        && let Some(found) = find_line(exit, |lower| {
            lower.contains("not found") || lower.contains("no such file")
        })
    {
        return Some(with_line(NativeCliCause::MissingBinary, found));
    }
    if let Some(found) = find_signature(exit, AUTH_SIGNATURES) {
        return Some(with_line(NativeCliCause::AuthOrPermission, found));
    }
    // Case-sensitive: the provider's own API-layer prefix.
    if let Some(found) = find_line_raw(exit, |line| line.contains("API Error:")) {
        return Some(with_line(NativeCliCause::ApiFailure, found));
    }
    if let Some(found) = find_signature(exit, MODEL_SIGNATURES) {
        let suggestions = exit.tails().find_map(|tail| parse_model_suggestions(&tail.text));
        return Some(with_line(NativeCliCause::ModelNotFound { suggestions }, found));
    }
    if let Some(found) = find_signature(exit, ARGUMENT_SIGNATURES) {
        let switch = extract_switch(found.0);
        return Some(with_line(NativeCliCause::ArgumentRejected { switch }, found));
    }
    if let Some(found) = find_signature(exit, MISSING_ARGUMENT_SIGNATURES) {
        return Some(with_line(NativeCliCause::MissingArgument, found));
    }
    None
}

fn find_signature<'a>(exit: &'a NativeExit, signatures: &[&str]) -> Option<(&'a str, bool)> {
    find_line(exit, |lower| signatures.iter().any(|signature| lower.contains(signature)))
}

/// The first captured line (stderr before stdout) whose lowercase form
/// satisfies `matches`, with whether its stream was shown.
fn find_line<'a>(exit: &'a NativeExit, matches: impl Fn(&str) -> bool) -> Option<(&'a str, bool)> {
    find_line_raw(exit, |line| matches(&line.to_lowercase()))
}

fn find_line_raw<'a>(exit: &'a NativeExit, matches: impl Fn(&str) -> bool) -> Option<(&'a str, bool)> {
    exit.tails().find_map(|tail| {
        tail.text
            .lines()
            .find(|line| matches(line))
            .map(|line| (line, tail.shown))
    })
}

/// The switch a rejection line names, trimmed of the quotes and punctuation
/// providers wrap it in (`'--foo'`, `"--foo"`, `--foo,`). Only the matched
/// line is read, so a switch quoted elsewhere in the output is never taken.
fn extract_switch(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|candidate| {
        let trimmed =
            candidate.trim_matches(|c: char| matches!(c, '\'' | '"' | ',' | '.' | '`' | ':' | ';'));
        (trimmed.starts_with('-') && trimmed.len() > 1 && trimmed != "--")
            .then(|| trimmed.to_string())
    })
}

/// Whether [`AgentErrorReport::for_native_exit`] would attribute `exit` to
/// `tail`. A launch path that echoes provider output itself asks this first,
/// so a correlated diagnostic reaches the user once, inside the report.
pub(crate) fn attributes_to_tail(exit: &NativeExit, tail: &ProviderTail) -> bool {
    classify_native_exit(exit)
        .is_some_and(|classified| classification_attributes_to_tail(exit, tail, &classified))
}

fn classification_attributes_to_tail(
    exit: &NativeExit,
    tail: &ProviderTail,
    classified: &NativeClassification,
) -> bool {
    !tail.is_empty()
        && exit.failed()
        && match &classified.cause {
            NativeCliCause::ArgumentRejected { switch: None } => true,
            NativeCliCause::ArgumentRejected { switch: Some(switch) } => tail_names_switch(tail, switch),
            _ => false,
        }
}

/// The one report for a provider's terminal native failure, and whether it
/// attributes the failure to the forwarded tail.
#[derive(Debug, Clone)]
pub(crate) struct NativeExitReport {
    pub(crate) report: AgentErrorReport,
    pub(crate) correlated: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentErrorReport {
    pub(crate) provider: Provider,
    pub(crate) exit_code: i32,
    pub(crate) category: AgentErrorCategory,
    pub(crate) summary: String,
    pub(crate) body_list: Option<Vec<String>>,
    pub(crate) footer: Option<String>,
    pub(crate) detail: Option<String>,
    pub(crate) hint: Option<String>,
    pub(crate) suggestions: Option<Vec<String>>,
    pub(crate) suggestion_style: SuggestionStyle,
    #[allow(dead_code)]
    pub(crate) location: Option<String>,
}

impl AgentErrorReport {
    /// Build the report for a provider's terminal native failure.
    ///
    /// Both launch paths call this exactly once per terminal failure. The
    /// report correlates the failure with `tail` only when the tail is
    /// non-empty, the exit failed, the classifier found an argument
    /// rejection, and any switch the rejection names belongs to the tail; a
    /// rejection naming one of Claudine's own injected switches stays a
    /// generic native error. Every other exit keeps its cause's category and
    /// remediation.
    ///
    /// Provider text in the report is masked with [`tail_redactor`] (shared
    /// secret recognizer plus echoes of the tail's sensitive values) and
    /// escaped for the terminal. A line from a stream the user already saw
    /// is never repeated.
    pub(crate) fn for_native_exit(
        provider: Provider,
        exit: &NativeExit,
        tail: &ProviderTail,
        model_source: Option<&ModelSource>,
    ) -> NativeExitReport {
        let classification = classify_native_exit(exit);
        let redactor = tail_redactor(tail);
        let correlated = classification
            .as_ref()
            .filter(|classified| classification_attributes_to_tail(exit, tail, classified));
        // A matched line the user already saw, streamed live or as the
        // failure headline, is not repeated.
        let line_shown = |classification: &NativeClassification| {
            classification.line_shown
                || classification.line.as_deref().is_some_and(|line| {
                    let masked = redactor.redact(line.trim()).to_string();
                    exit.shown_headline
                        .as_deref()
                        .is_some_and(|headline| !masked.is_empty() && headline.contains(&masked))
                })
        };
        let excerpt = |classification: &NativeClassification| {
            classification
                .line
                .as_deref()
                .filter(|_| !line_shown(classification))
                .map(|line| display_safe(&redactor, line))
        };
        if let Some(classified) = correlated {
            let provider_name = crate::output::capitalize_provider(provider);
            let shown_note = if line_shown(classified) {
                " Its own message is shown above."
            } else {
                ""
            };
            let report = Self {
                provider,
                exit_code: exit.exit_code,
                category: AgentErrorCategory::AgentNative,
                summary: format!(
                    "{provider_name} rejected its arguments. This was likely caused by the \
                     forwarded arguments: {}.",
                    tail_summary(tail).phrase()
                ),
                body_list: None,
                footer: None,
                detail: excerpt(classified),
                hint: Some(format!(
                    "Check {provider_name}'s usage for the forwarded arguments.{shown_note}"
                )),
                suggestions: None,
                suggestion_style: SuggestionStyle::DidYouMean,
                location: None,
            };
            return NativeExitReport {
                report,
                correlated: true,
            };
        }

        let report = match classification {
            Some(classified) => {
                let detail = excerpt(&classified);
                cause_report(provider, exit.exit_code, &classified.cause, detail, &redactor, model_source)
            }
            None => {
                let provider_name = crate::output::capitalize_provider(provider);
                let detail = exit
                    .stderr
                    .as_ref()
                    .filter(|tail| !tail.shown)
                    .and_then(|tail| tail.text.lines().find(|line| !line.trim().is_empty()))
                    .map(|line| display_safe(&redactor, line));
                Self::with_summary(
                    provider,
                    exit.exit_code,
                    AgentErrorCategory::AgentNative,
                    format!("{provider_name} exited with error code {}", exit.exit_code),
                    detail,
                )
            }
        };
        NativeExitReport {
            report,
            correlated: false,
        }
    }

    fn with_summary(
        provider: Provider,
        exit_code: i32,
        category: AgentErrorCategory,
        summary: String,
        detail: Option<String>,
    ) -> Self {
        Self {
            provider,
            exit_code,
            category,
            summary,
            body_list: None,
            footer: None,
            detail,
            hint: None,
            suggestions: None,
            suggestion_style: SuggestionStyle::DidYouMean,
            location: None,
        }
    }

    pub(crate) fn no_model_provided(provider: Provider) -> Self {
        let provider_name = crate::output::capitalize_provider(provider);
        Self {
            provider,
            exit_code: 1,
            category: AgentErrorCategory::Configuration,
            summary: format!(
                "No model specified! {provider_name} by default does not specify a model but you can\n\
                 change this behavior by adding a <yellow>model</yellow> property to the <blue>~/.config/opencode/opencode.json</blue> file.\n\
                 You can override/set the default model with any of the following methods:"
            ),
            body_list: Some(
                claudine::provider::provider_info(provider)
                    .model_env_vars
                    .iter()
                    .map(|var| format!("set <yellow>{var}</yellow> to a valid model name"))
                    .chain(std::iter::once(
                        "use the CLI switch <yellow>--model <model></yellow>".to_string(),
                    ))
                    .collect(),
            ),
            footer: Some(
                "Running <yellow>opencode models</yellow> will give you a list of all valid models.\n\
                 Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim> for direct providers\n\
                 like Google or Anthropic but take the form <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim>\n\
                 for aggregators like OpenRouter."
                    .to_string(),
            ),
            detail: None,
            hint: None,
            suggestions: None,
            suggestion_style: SuggestionStyle::BareList,
            location: None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn invalid_model(
        provider: Provider,
        exit_code: i32,
        location: String,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            provider,
            exit_code,
            category: AgentErrorCategory::AgentNative,
            summary: format!(
                "Invalid model specified in {location}! Running <yellow>opencode models</yellow> will give you\n\
                 a list of all valid models. Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim>\n\
                 for direct providers like Google or Anthropic but take the form\n\
                 <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim> for aggregators like OpenRouter."
            ),
            body_list: None,
            footer: None,
            detail: None,
            hint: None,
            suggestions: Some(suggestions),
            suggestion_style: SuggestionStyle::DidYouMean,
            location: Some(location),
        }
    }

    pub(crate) fn render(&self, term: &Terminal) {
        let border_color = match self.category {
            AgentErrorCategory::Configuration => Color::Tailwind(Tailwind::Orange700),
            AgentErrorCategory::AgentNative => Color::Tailwind(Tailwind::Red700),
            AgentErrorCategory::ApiRemote => Color::Tailwind(Tailwind::Red700),
            AgentErrorCategory::Interrupted => Color::Tailwind(Tailwind::Yellow700),
        };

        let label = match self.category {
            AgentErrorCategory::Configuration => "Configuration Error",
            AgentErrorCategory::AgentNative => "Agent Error",
            AgentErrorCategory::ApiRemote => "API Error",
            AgentErrorCategory::Interrupted => "Interrupted",
        };

        let provider_name = crate::output::capitalize_provider(self.provider);

        let mut compose = Compose::default();

        compose.add_prose(Prose::new(format!(
            "<red><bold>{label}</bold></red> <dim>({provider_name}, exit {})</dim>\n{}",
            self.exit_code, self.summary,
        )));

        if let Some(ref items) = self.body_list {
            compose.add_unordered_list(UnorderedList::new(items.clone()));
        }

        if let Some(ref footer) = self.footer {
            compose.add_prose(Prose::new(format!("\n{footer}")));
        }

        if let Some(ref detail) = self.detail {
            compose.add_prose(Prose::new(format!("\n<dim>{detail}</dim>")));
        }

        if let Some(ref hint) = self.hint {
            compose.add_prose(Prose::new(format!("\n<blue>{hint}</blue>")));
        }

        if let Some(ref suggestions) = self.suggestions
            && !suggestions.is_empty()
        {
            let suggestion_items: Vec<String> = suggestions
                .iter()
                .map(|s| format!("<yellow>{s}</yellow>"))
                .collect();
            let list = UnorderedList::new(suggestion_items);
            match self.suggestion_style {
                SuggestionStyle::DidYouMean => {
                    let header =
                        Status::from_prose("Did you mean:".to_string()).state(StatusState::Warning);
                    compose.add_text("\n");
                    compose.add_prose(Prose::new(header.render(term)));
                    compose.add_unordered_list(list);
                }
                SuggestionStyle::BareList => {
                    compose.add_text("\n");
                    compose.add_unordered_list(list);
                }
            }
        }

        let block = StatusBlock::new(StatusState::Error)
            .body(Prose::new(compose.render(term)))
            .border_color(border_color)
            .left_margin(TargetValue::universal(Length::ch(2)))
            .right_margin(TargetValue::universal(Length::ch(2)));

        log::message("");
        log::message(&block.render(term));
        log::message("");
    }
}

/// The generic report for a classified cause. `detail` is the already
/// masked and escaped diagnostic line, absent when the user saw it.
fn cause_report(
    provider: Provider,
    exit_code: i32,
    cause: &NativeCliCause,
    detail: Option<String>,
    redactor: &Redactor,
    model_source: Option<&ModelSource>,
) -> AgentErrorReport {
    let provider_name = crate::output::capitalize_provider(provider);
    let report = |category, summary: String| {
        AgentErrorReport::with_summary(provider, exit_code, category, summary, detail.clone())
    };
    match cause {
        NativeCliCause::Interrupted => AgentErrorReport::with_summary(
            provider,
            exit_code,
            AgentErrorCategory::Interrupted,
            format!("{provider_name} was interrupted by the user"),
            None,
        ),
        NativeCliCause::TimedOut => report(
            AgentErrorCategory::AgentNative,
            format!("{provider_name} was stopped by a timeout"),
        ),
        NativeCliCause::MissingBinary => report(
            AgentErrorCategory::Configuration,
            "A required file or command was not found.".to_string(),
        ),
        NativeCliCause::AuthOrPermission => AgentErrorReport {
            hint: Some("Check API keys and provider authentication configuration.".to_string()),
            ..report(
                AgentErrorCategory::Configuration,
                "An authentication or permission error occurred.".to_string(),
            )
        },
        NativeCliCause::ApiFailure => AgentErrorReport {
            hint: Some("Check API key, rate limits, and service status.".to_string()),
            ..report(
                AgentErrorCategory::ApiRemote,
                "The provider's API layer returned an error.".to_string(),
            )
        },
        NativeCliCause::ModelNotFound { suggestions } => {
            let location = model_source.map(ModelSource::location_string);
            let loc = location.as_deref().unwrap_or("the command line");
            AgentErrorReport {
                suggestions: suggestions.clone(),
                location: location.clone(),
                detail: None,
                ..report(
                    AgentErrorCategory::AgentNative,
                    format!(
                        "Invalid model specified in {loc}! Running <yellow>opencode models</yellow> will give you\n\
                         a list of all valid models. Model names follow the format <dim>[provider]</dim>/<dim>[model]</dim>\n\
                         for direct providers like Google or Anthropic but take the form\n\
                         <dim>[aggregator]</dim>/<dim>[provider]</dim>/<dim>[model]</dim> for aggregators like OpenRouter."
                    ),
                )
            }
        }
        NativeCliCause::ArgumentRejected { switch } => {
            let suffix = switch
                .as_deref()
                .map(|switch| format!(" (`{}`)", display_safe(redactor, switch)))
                .unwrap_or_default();
            report(
                AgentErrorCategory::AgentNative,
                format!("{provider_name} did not recognize a flag{suffix}"),
            )
        }
        NativeCliCause::MissingArgument => report(
            AgentErrorCategory::AgentNative,
            "A required argument was missing from the command.".to_string(),
        ),
    }
}

/// Provider text made safe to place in report markup: recognized and
/// tail-known secrets masked, control characters removed, markup escaped.
fn display_safe(redactor: &Redactor, text: &str) -> String {
    let masked = redactor.redact(text);
    let printable: String = masked.as_str().chars().filter(|c| !c.is_control()).collect();
    Prose::escape_text(printable.trim())
}

fn parse_model_suggestions(stderr: &str) -> Option<Vec<String>> {
    let lower = stderr.to_lowercase();
    let start = lower.find("suggestions:")?;
    let bracket_start = lower[start..].find('[')?;
    let bracket_end = lower[start + bracket_start..].find(']')?;
    let inner = &stderr[start + bracket_start + 1..start + bracket_start + bracket_end];

    let mut items = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    for ch in inner.chars() {
        if ch == '"' {
            if in_quote {
                if !current.is_empty() {
                    items.push(current.clone());
                    current.clear();
                }
                in_quote = false;
            } else {
                in_quote = true;
            }
        } else if in_quote {
            current.push(ch);
        }
    }

    if items.is_empty() { None } else { Some(items) }
}

#[cfg(test)]
mod tests;
