//! Interactive shell command approval handler for the CLI.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use darkmatter::markdown::compose::expression::functions::ReadKind;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::schemas::slots::grants::{
    Grant, GrantDecision, GrantScope, GrantTarget, SlotGrantDecision, SlotGrantRequest,
};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;

/// CLI approval handler that prompts the user interactively on stderr.
///
/// Claudine installs this same handler, so both prompts behave alike.
pub struct CliShellApprovalHandler;

impl ShellApprovalHandler for CliShellApprovalHandler {
    #[allow(clippy::result_large_err)]
    fn approve(
        &self,
        request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        let stderr = io::stderr();
        let stdin = io::stdin();
        let mut stderr = stderr.lock();
        let mut stdin = stdin.lock();

        approve_with_io(&request, &mut stdin, &mut stderr)
    }

    #[allow(clippy::result_large_err)]
    fn approve_slot_reads(
        &self,
        requests: &[SlotGrantRequest],
    ) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
        let stderr = io::stderr();
        let stdin = io::stdin();
        let mut stderr = stderr.lock();
        let mut stdin = stdin.lock();

        approve_slot_reads_with_io(requests, &mut stdin, &mut stderr)
    }
}

/// Asks about each of `requests` in turn over `input` and `output`, one
/// numbered prompt per read, listing exactly the request's choices.
///
/// This is the slot prompt's testing seam, as [`approve_with_io`] is the
/// shell prompt's. A number that names no listed choice asks again; ended
/// input answers this and every later request with "deny for this run".
///
/// ## Errors
///
/// `PolicyIo` when reading the answer or writing the prompt fails.
#[allow(clippy::result_large_err)]
pub fn approve_slot_reads_with_io<R: BufRead, W: Write>(
    requests: &[SlotGrantRequest],
    input: &mut R,
    output: &mut W,
) -> Result<Vec<SlotGrantDecision>, ShellExpansionError> {
    let mut decisions = Vec::with_capacity(requests.len());
    let mut ended = false;
    for request in requests {
        if ended {
            decisions.push(deny_for_this_run(request));
            continue;
        }
        let decision = ask_slot_read(request, input, output)?;
        ended = decision.is_none();
        decisions.push(decision.unwrap_or_else(|| deny_for_this_run(request)));
    }
    Ok(decisions)
}

/// The "deny for this run" answer to `request`; unanswered when it offers
/// none (no grant can allow the read).
fn deny_for_this_run(request: &SlotGrantRequest) -> SlotGrantDecision {
    request
        .choices
        .iter()
        .find(|grant| grant.decision == GrantDecision::Deny && grant.scope == GrantScope::Once)
        .map_or(SlotGrantDecision::Unanswered, SlotGrantDecision::from_grant)
}

/// One prompt; `None` when the input ended.
#[allow(clippy::result_large_err)]
fn ask_slot_read<R: BufRead, W: Write>(
    request: &SlotGrantRequest,
    input: &mut R,
    output: &mut W,
) -> Result<Option<SlotGrantDecision>, ShellExpansionError> {
    if request.choices.is_empty() {
        return Ok(Some(SlotGrantDecision::Unanswered));
    }
    let io_error = |source| ShellExpansionError::PolicyIo {
        path: request.schema.clone().unwrap_or_else(|| PathBuf::from("<stdin>")),
        source,
    };
    loop {
        write_slot_prompt(output, request).map_err(io_error)?;
        let mut answer = String::new();
        if input.read_line(&mut answer).map_err(io_error)? == 0 {
            return Ok(None);
        }
        let picked = answer
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|index| request.choices.get(index));
        match picked {
            Some(grant) => return Ok(Some(SlotGrantDecision::from_grant(grant))),
            None => {
                let err_msg = Prose::new(format!(
                    "<red>Invalid choice. Please enter 1-{}.</red>",
                    request.choices.len()
                ))
                .render_optimistic(None);
                writeln!(output, "  {err_msg}").map_err(io_error)?;
            }
        }
    }
}

fn write_slot_prompt<W: Write>(output: &mut W, request: &SlotGrantRequest) -> io::Result<()> {
    let label = |text: &str| Prose::new(format!("<dim>{text:<9}</dim>")).render_optimistic(None);
    let header = Prose::new("\u{26a0}  <bold><yellow>Schema Read Approval Required</yellow></bold>")
        .render_optimistic(None);
    let verb = match request.kind {
        ReadKind::Content => "reads",
        ReadKind::Probe => "checks",
        ReadKind::Listing => "lists",
    };
    let read = Prose::new(format!(
        "<bold>{}()</bold> {verb} <bold><cyan>{}</cyan></bold>",
        escape_prose(&request.function),
        escape_prose(&request.location.to_string())
    ))
    .render_optimistic(None);

    write!(output, "\n  {header}\n")?;
    if let Some(schema) = &request.schema {
        let schema = Prose::new(format!("<bold>{}</bold>", escape_prose(&schema.display().to_string())))
            .render_optimistic(None);
        writeln!(output, "  {} {schema}", label("Schema:"))?;
    }
    writeln!(output, "  {} {read}\n", label("Read:"))?;

    let sep = Prose::new("<dim>\u{2502}</dim>").render_optimistic(None);
    let labels: Vec<String> = request.choices.iter().map(slot_choice_label).collect();
    let width = labels.iter().map(String::len).max().unwrap_or(0);
    for (index, (grant, text)) in request.choices.iter().zip(&labels).enumerate() {
        let color = match (grant.decision, grant.scope) {
            (GrantDecision::Allow, GrantScope::Persisted) => "green",
            (GrantDecision::Allow, GrantScope::Once) => "cyan",
            (GrantDecision::Deny, _) => "yellow",
        };
        let number = Prose::new(format!("<{color}>{}</{color}>", index + 1)).render_optimistic(None);
        let effect = Prose::new(format!("<dim>({})</dim>", escape_prose(&slot_choice_effect(grant))))
            .render_optimistic(None);
        writeln!(output, "  {number} {sep} {text:<width$}  {effect}")?;
    }
    writeln!(output)?;
    let prompt_arrow = Prose::new("<bold>></bold>").render_optimistic(None);
    write!(output, "  {prompt_arrow} ")?;
    output.flush()
}

/// What a choice covers, in words. The library offers only allow for this
/// run, "always allow" (a file), and deny for this run.
fn slot_choice_label(grant: &Grant) -> String {
    let what = match &grant.target {
        GrantTarget::Url(_) | GrantTarget::UrlAnyQuery(_) | GrantTarget::Site { .. } => "this URL",
        GrantTarget::File(_) | GrantTarget::Dir(_) => "this file",
    };
    match (grant.decision, grant.scope) {
        (GrantDecision::Allow, GrantScope::Once) => format!("Allow {what} for this run"),
        (GrantDecision::Allow, GrantScope::Persisted) => format!("Always allow {what}"),
        (GrantDecision::Deny, _) => "Deny for this run".to_string(),
    }
}

/// What choosing a line does: "always allow" saves the file to the user's
/// `permissions.file-read`.
fn slot_choice_effect(grant: &Grant) -> String {
    match (grant.decision, grant.scope) {
        (GrantDecision::Allow, GrantScope::Persisted) => {
            format!("{}; saved to your permissions.file-read", grant.target)
        }
        (GrantDecision::Deny, _) => "the slot fails; this run only".to_string(),
        (GrantDecision::Allow, GrantScope::Once) => format!("{}; this run only", grant.target),
    }
}

/// One numbered line of the prompt. Only the choices the request offers are
/// listed, so a number can never select a choice that was not offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    Exact,
    Shape,
    Prefix,
    Once,
    Deny,
}

impl Choice {
    fn offered(request: &ShellApprovalRequest) -> Vec<Choice> {
        let offered = request.offered;
        // A shape cannot be shown or saved without its template.
        let shape = offered.shape && request.template.is_some();
        [
            (offered.exact, Choice::Exact),
            (shape, Choice::Shape),
            (offered.prefix, Choice::Prefix),
            (true, Choice::Once),
            (true, Choice::Deny),
        ]
        .into_iter()
        .filter_map(|(available, choice)| available.then_some(choice))
        .collect()
    }

    fn decision(self) -> ShellApprovalDecision {
        match self {
            Choice::Exact => ShellApprovalDecision::AllowExactPersist,
            Choice::Shape => ShellApprovalDecision::AllowTemplatePersist,
            Choice::Prefix => ShellApprovalDecision::AllowCommandPersist,
            Choice::Once => ShellApprovalDecision::AllowOnce,
            Choice::Deny => ShellApprovalDecision::Deny,
        }
    }

    fn color(self) -> &'static str {
        match self {
            Choice::Exact | Choice::Shape | Choice::Prefix => "green",
            Choice::Once => "cyan",
            Choice::Deny => "yellow",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Choice::Exact => "Allow this exact command",
            Choice::Shape => "Allow this command shape",
            Choice::Prefix => "Allow this program with any arguments",
            Choice::Once => "Allow once",
            Choice::Deny => "Deny for this run",
        }
    }

    /// What choosing this line does, naming each `shell-commands` rule it
    /// saves to the user configuration.
    fn effect(self, request: &ShellApprovalRequest) -> String {
        let shape = request
            .template
            .as_ref()
            .map(|template| template.shape.canonical_text());
        let resolved = request.offered.exact && !request.normalized_exact.is_empty();
        let saves = |rules: String| match &request.save_target {
            Some(_) => format!("saves {rules} to your shell-commands"),
            // Without a home there is no user configuration to save to.
            None => format!("allows {rules} for this run; there is no home to save it to"),
        };
        match self {
            Choice::Exact if !request.chain_executables.is_empty() => saves("an exact rule for each command of the chain".to_string()),
            Choice::Exact => saves(rule("exact", &request.normalized_exact)),
            Choice::Shape => format!(
                "{}; runs as a template",
                saves(rule("template", shape.as_deref().unwrap_or_default()))
            ),
            Choice::Prefix => {
                let rules = prefix_programs(request)
                    .iter()
                    .map(|program| rule("prefix", program))
                    .collect::<Vec<_>>()
                    .join(", ");
                saves(rules)
            }
            Choice::Once if !resolved && shape.is_some() => "this command shape, this session only".to_string(),
            Choice::Once => "this session only".to_string(),
            Choice::Deny => "stops the run; asks again next time".to_string(),
        }
    }
}

/// `keyword text`, quoted for the prompt.
fn rule(keyword: &str, text: &str) -> String {
    format!("\"{keyword} {text}\"")
}

/// Every distinct program the "any arguments" choice saves a `prefix` rule
/// for: each executable of a chain, otherwise the single executable.
fn prefix_programs(request: &ShellApprovalRequest) -> Vec<&str> {
    if request.chain_executables.is_empty() {
        vec![request.executable.as_str()]
    } else {
        request.chain_executables.iter().map(String::as_str).collect()
    }
}

/// Runs one prompt session over `input` and `output`.
///
/// This is the handler's testing seam: a caller supplies scripted input and
/// captures what the user would see. The session re-asks after a number that
/// names no listed choice, and denies when the input ends.
///
/// ## Errors
///
/// `PolicyIo` (naming the terminal) when reading the answer or writing the
/// prompt fails.
#[allow(clippy::result_large_err)]
pub fn approve_with_io<R: BufRead, W: Write>(
    request: &ShellApprovalRequest,
    input: &mut R,
    output: &mut W,
) -> Result<ShellApprovalDecision, ShellExpansionError> {
    let source_desc = match &request.source {
        darkmatter::markdown::compose::ComposeSource::File(p) => p.display().to_string(),
        darkmatter::markdown::compose::ComposeSource::Url(u) => u.to_string(),
        darkmatter::markdown::compose::ComposeSource::Unknown => "<stdin>".to_string(),
    };
    let choices = Choice::offered(request);
    let io_error = |source| ShellExpansionError::PolicyIo { path: PathBuf::from(PROMPT_STREAM), source };

    loop {
        write_prompt(output, request, &source_desc, &choices)?;

        let mut answer = String::new();
        if input.read_line(&mut answer).map_err(io_error)? == 0 {
            // No answer will ever come; refusing is the only safe reading.
            return Ok(ShellApprovalDecision::Deny);
        }

        let picked = answer
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|index| choices.get(index));
        match picked {
            Some(choice) => return Ok(choice.decision()),
            None => {
                let err_msg = Prose::new(format!(
                    "<red>Invalid choice. Please enter 1-{}.</red>",
                    choices.len()
                ))
                .render_optimistic(None);
                writeln!(output, "  {err_msg}").map_err(io_error)?;
            }
        }
    }
}

#[allow(clippy::result_large_err)]
fn write_prompt<W: Write>(
    output: &mut W,
    request: &ShellApprovalRequest,
    source_desc: &str,
    choices: &[Choice],
) -> Result<(), ShellExpansionError> {
    let w = |output: &mut W, args: std::fmt::Arguments| -> Result<(), ShellExpansionError> {
        output
            .write_fmt(args)
            .map_err(|source| ShellExpansionError::PolicyIo { path: PathBuf::from(PROMPT_STREAM), source })
    };
    // Padded before styling so the values line up.
    let label = |text: &str| Prose::new(format!("<dim>{text:<9}</dim>")).render_optimistic(None);
    let command = |text: &str| {
        Prose::new(format!("<bold><cyan>{}</cyan></bold>", escape_prose(text))).render_optimistic(None)
    };

    let header = Prose::new("\u{26a0}  <bold><yellow>Shell Approval Required</yellow></bold>")
        .render_optimistic(None);
    let source_value = Prose::new(format!("<bold>{source_desc}:{}</bold>", request.origin))
        .render_optimistic(None);

    w(output, format_args!("\n  {header}\n"))?;
    w(output, format_args!("  {} {source_value}\n", label("Source:")))?;

    if let Some(ref alias_name) = request.alias_name {
        let alias_value = Prose::new(format!(
            "<bold>{}</bold> <dim>\u{2192}</dim> <bold><cyan>{}</cyan></bold>",
            escape_prose(alias_name),
            escape_prose(&request.raw_command)
        ))
        .render_optimistic(None);
        w(output, format_args!("  {} {alias_value}\n", label("Alias:")))?;
    }

    // A template shows each placeholder as `{{ name: type }}`. Its resolved
    // bytes exist only when the exact choice is offered; an unknown value is
    // never shown as an empty string.
    match &request.template {
        Some(template) => {
            w(output, format_args!("  {} {}\n", label("Command:"), command(&template.display)))?;
            if request.offered.exact && !request.normalized_exact.is_empty() {
                w(
                    output,
                    format_args!("  {} {}\n", label("Resolved:"), command(&request.normalized_exact)),
                )?;
            }
        }
        None => w(output, format_args!("  {} {}\n", label("Command:"), command(&request.raw_command)))?,
    }
    // An exclusion is why a command a rule would allow is asked about; no
    // choice removes it.
    if let Some(exclusion) = &request.exclusion {
        let value = Prose::new(format!(
            "<bold>{}</bold> <dim>in {}</dim>",
            escape_prose(&exclusion.entry),
            escape_prose(&exclusion.file.display().to_string())
        ))
        .render_optimistic(None);
        w(output, format_args!("  {} {value}\n", label("Excluded:")))?;
    }
    w(output, format_args!("\n"))?;

    let sep = Prose::new("<dim>\u{2502}</dim>").render_optimistic(None);
    let width = choices.iter().map(|choice| choice.label().len()).max().unwrap_or(0);
    for (index, choice) in choices.iter().enumerate() {
        let number = Prose::new(format!("<{color}>{}</{color}>", index + 1, color = choice.color()))
            .render_optimistic(None);
        let effect = Prose::new(format!("<dim>({})</dim>", escape_prose(&choice.effect(request))))
            .render_optimistic(None);
        w(
            output,
            format_args!("  {number} {sep} {:<width$}  {effect}\n", choice.label()),
        )?;
    }
    w(output, format_args!("\n"))?;

    let prompt_arrow = Prose::new("<bold>></bold>").render_optimistic(None);
    w(output, format_args!("  {prompt_arrow} "))?;

    output
        .flush()
        .map_err(|source| ShellExpansionError::PolicyIo { path: PathBuf::from(PROMPT_STREAM), source })
}

/// What a prompt I/O failure names instead of a file.
const PROMPT_STREAM: &str = "<terminal>";

/// Escape text so it renders exactly as written inside Prose markup; code
/// spans it marks with backticks stay literal.
fn escape_prose(text: &str) -> String {
    Prose::escape_text_outside_code_spans(text)
}

/// Returns true if interactive prompting is safe.
///
/// Prompting is safe when:
/// 1. stdin is a terminal
/// 2. stderr is a terminal
pub fn can_prompt_interactively() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::compose::shell_expansion::template::{
        OfferedChoices, TemplateApprovalView, TemplateShape,
    };
    use std::io::Cursor;
    use std::path::PathBuf;

    fn request() -> ShellApprovalRequest {
        use darkmatter::markdown::compose::ShellCommandOrigin;
        ShellApprovalRequest {
            source: ComposeSource::File(PathBuf::from("/tmp/doc.md")),
            origin: ShellCommandOrigin::Body { line: 12 },
            raw_command: "echo hello".to_string(),
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
            normalized_exact: "echo hello".to_string(),
            save_target: Some(PathBuf::from("/home/.darkmatter.yaml")),
            exclusion: None,
            alias_name: None,
            chain_executables: Vec::new(),
            offered: darkmatter::markdown::compose::shell_expansion::template::OfferedChoices::resolved_bytes(),
            template: None,
        }
    }

    /// Strips ANSI escape sequences for assertion clarity.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn prompt_accepts_valid_choice() {
        let request = request();
        let mut input = Cursor::new(b"2\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowCommandPersist);
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("Shell Approval Required"));
        assert!(prompt.contains("/tmp/doc.md:line 12"));
        assert!(prompt.contains("echo hello"));
    }

    #[test]
    fn prompt_retries_after_invalid_choice() {
        let request = request();
        let mut input = Cursor::new(b"9\n3\n".to_vec());
        let mut output = Vec::new();

        let decision = approve_with_io(&request, &mut input, &mut output).unwrap();

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("Invalid choice. Please enter 1-4."));
        // Prompt should appear twice (once for invalid, once for retry)
        assert_eq!(prompt.matches("Shell Approval Required").count(), 2);
    }

    #[test]
    fn all_four_choices_produce_correct_decisions() {
        let cases = [
            ("1\n", ShellApprovalDecision::AllowExactPersist),
            ("2\n", ShellApprovalDecision::AllowCommandPersist),
            ("3\n", ShellApprovalDecision::AllowOnce),
            ("4\n", ShellApprovalDecision::Deny),
        ];

        for (input_str, expected) in cases {
            let request = request();
            let mut input = Cursor::new(input_str.as_bytes().to_vec());
            let mut output = Vec::new();

            let decision = approve_with_io(&request, &mut input, &mut output).unwrap();
            assert_eq!(
                decision,
                expected,
                "Input '{}' should produce {:?}",
                input_str.trim(),
                expected
            );
        }
    }

    #[test]
    fn prompt_shows_source_and_command() {
        let request = request();
        let mut input = Cursor::new(b"1\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());

        assert!(prompt.contains("Source:"));
        assert!(prompt.contains("Command:"));
        assert!(prompt.contains("Allow this exact command"));
        assert!(prompt.contains("saves \"exact echo hello\" to your shell-commands"));
        assert!(prompt.contains("Allow this program with any arguments"));
        assert!(prompt.contains("saves \"prefix echo\" to your shell-commands"));
        assert!(prompt.contains("Allow once"));
        assert!(prompt.contains("Deny for this run"));
        assert!(!prompt.contains("lacklist"), "no permanent deny is offered: {prompt}");
        assert!(!prompt.contains("Excluded:"), "nothing excluded it: {prompt}");
        assert!(!prompt.contains("command shape"), "no template, no shape: {prompt}");
    }

    #[test]
    fn prompt_lists_chain_executables_for_chain_request() {
        let mut request = request();
        request.chain_executables = vec!["echo".to_string(), "pwd".to_string()];
        request.raw_command = "echo hello && pwd".to_string();
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());

        // Every distinct program of the chain gets its own prefix rule.
        assert!(
            prompt.contains("saves \"prefix echo\", \"prefix pwd\" to your shell-commands"),
            "expected both programs in prompt: {prompt}"
        );
    }

    #[test]
    fn prompt_shows_url_source() {
        let mut request = request();
        request.source = ComposeSource::Url("https://example.com/doc.md".parse().unwrap());
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("https://example.com/doc.md"));
    }

    #[test]
    fn prompt_shows_stdin_source() {
        let mut request = request();
        request.source = ComposeSource::Unknown;
        let mut input = Cursor::new(b"3\n".to_vec());
        let mut output = Vec::new();

        approve_with_io(&request, &mut input, &mut output).unwrap();
        let prompt = strip_ansi(&String::from_utf8(output).unwrap());
        assert!(prompt.contains("<stdin>"));
    }

    fn template_request(authored: &str, display: &str, offered: OfferedChoices) -> ShellApprovalRequest {
        let template = darkmatter::markdown::compose::shell_expansion::template::parse_authored(authored)
            .expect("a valid template");
        let mut request = request();
        request.raw_command = authored.to_string();
        request.executable = authored.split_whitespace().next().unwrap().to_string();
        request.offered = offered;
        request.template = Some(TemplateApprovalView {
            shape: TemplateShape::from_template(&template),
            display: display.to_string(),
        });
        if !offered.exact {
            request.normalized_exact = String::new();
            request.args = Vec::new();
        }
        request
    }

    fn session(request: &ShellApprovalRequest, input: &str) -> (ShellApprovalDecision, String) {
        let mut input = Cursor::new(input.as_bytes().to_vec());
        let mut output = Vec::new();
        let decision = approve_with_io(request, &mut input, &mut output).unwrap();
        (decision, strip_ansi(&String::from_utf8(output).unwrap()))
    }

    const ALL: OfferedChoices = OfferedChoices {
        exact: true,
        shape: true,
        prefix: true,
    };
    const TEMPLATE_ONLY: OfferedChoices = OfferedChoices {
        exact: false,
        shape: true,
        prefix: true,
    };

    #[test]
    fn a_template_candidate_shows_each_placeholder_with_its_name_and_type() {
        let mut request = template_request(
            "some-command {{ dir }}/build",
            "some-command {{ dir: alphanumeric(allow-slashes) }}/build",
            ALL,
        );
        request.normalized_exact = "some-command src/build".to_string();

        let (_, prompt) = session(&request, "4\n");

        assert!(
            prompt.contains("Command:  some-command {{ dir: alphanumeric(allow-slashes) }}/build"),
            "{prompt}"
        );
        assert!(prompt.contains("Resolved: some-command src/build"), "{prompt}");
        assert!(
            prompt.contains("saves \"template some-command {{ string }}/build\" to your shell-commands"),
            "{prompt}"
        );
    }

    #[test]
    fn the_broad_string_approval_discloses_the_original_validator() {
        let request = template_request(
            "seq {{ count }}",
            "seq {{ count: string (validated as number) }}",
            TEMPLATE_ONLY,
        );

        let (_, prompt) = session(&request, "4\n");

        assert!(prompt.contains("seq {{ count: string (validated as number) }}"), "{prompt}");
        assert!(prompt.contains("\"template seq {{ string }}\""), "{prompt}");
    }

    #[test]
    fn the_exact_choice_is_listed_only_when_offered() {
        let resolved = template_request("ls {{ dir }}", "ls {{ dir: string }}", ALL);
        let (_, prompt) = session(&resolved, "5\n");
        assert!(prompt.contains("1 │ Allow this exact command"), "{prompt}");
        assert!(prompt.contains("5 │ Deny for this run"), "{prompt}");
        assert!(prompt.contains("2 │ Allow this command shape"), "{prompt}");
        assert!(prompt.contains("3 │ Allow this program with any arguments"), "{prompt}");

        let late = template_request(
            "just clean --out {{ out }}/build",
            "just clean --out {{ out: string }}/build",
            TEMPLATE_ONLY,
        );
        let (_, prompt) = session(&late, "4\n");
        assert!(!prompt.contains("exact command"), "{prompt}");
        assert!(!prompt.contains("Resolved:"), "an unknown value is never shown: {prompt}");
        assert!(prompt.contains("1 │ Allow this command shape"), "{prompt}");
        assert!(prompt.contains("this command shape, this session only"), "{prompt}");
        assert!(prompt.contains("4 │ Deny for this run"), "{prompt}");
    }

    #[test]
    fn the_shape_choice_is_listed_only_when_offered() {
        let mut request = template_request("ls {{ dir }}", "ls {{ dir: string }}", ALL);
        request.offered.shape = false;

        let (decision, prompt) = session(&request, "2\n");

        assert!(!prompt.contains("command shape"), "{prompt}");
        assert_eq!(decision, ShellApprovalDecision::AllowCommandPersist);
    }

    #[test]
    fn the_prefix_choice_is_listed_only_when_offered() {
        let mut request = template_request("ls {{ dir }}", "ls {{ dir: string }}", TEMPLATE_ONLY);
        request.offered.prefix = false;

        let (decision, prompt) = session(&request, "2\n");

        assert!(!prompt.contains("any arguments"), "{prompt}");
        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
    }

    #[test]
    fn a_number_naming_no_listed_choice_is_refused_and_asked_again() {
        let request = template_request("ls {{ dir }}", "ls {{ dir: string }}", TEMPLATE_ONLY);

        // Four lines are listed; 5 would have been "Deny" with all five.
        let (decision, prompt) = session(&request, "5\n0\nexact\n3\n");

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        assert_eq!(prompt.matches("Invalid choice. Please enter 1-4.").count(), 3, "{prompt}");
    }

    #[test]
    fn each_listed_choice_returns_its_decision() {
        let cases = [
            (ALL, "1", ShellApprovalDecision::AllowExactPersist),
            (ALL, "2", ShellApprovalDecision::AllowTemplatePersist),
            (ALL, "3", ShellApprovalDecision::AllowCommandPersist),
            (ALL, "4", ShellApprovalDecision::AllowOnce),
            (ALL, "5", ShellApprovalDecision::Deny),
            (TEMPLATE_ONLY, "1", ShellApprovalDecision::AllowTemplatePersist),
            (TEMPLATE_ONLY, "2", ShellApprovalDecision::AllowCommandPersist),
            (TEMPLATE_ONLY, "3", ShellApprovalDecision::AllowOnce),
            (TEMPLATE_ONLY, "4", ShellApprovalDecision::Deny),
        ];
        for (offered, answer, expected) in cases {
            let request = template_request("ls {{ dir }}", "ls {{ dir: string }}", offered);
            let (decision, _) = session(&request, &format!("{answer}\n"));
            assert_eq!(decision, expected, "{offered:?} answer {answer}");
        }
    }

    #[test]
    fn a_template_chain_lists_a_prefix_rule_for_every_program() {
        let mut request = template_request(
            "just build {{ out }} && git add {{ out }}",
            "just build {{ out: string }} && git add {{ out: string }}",
            TEMPLATE_ONLY,
        );
        request.chain_executables = vec!["just".to_string(), "git".to_string()];

        let (_, prompt) = session(&request, "4\n");

        assert!(
            prompt.contains("saves \"prefix just\", \"prefix git\" to your shell-commands"),
            "{prompt}"
        );
    }

    #[test]
    fn an_excluded_command_names_the_exclusion_that_asks() {
        let mut request = request();
        request.exclusion = Some(darkmatter::config::permissions::PolicyExclusion {
            revision: darkmatter::config::load::ConfigRevision::User,
            file: PathBuf::from("/home/.darkmatter.yaml"),
            entry: "!prefix echo".to_string(),
        });

        let (decision, prompt) = session(&request, "3\n");

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        assert!(prompt.contains("Excluded: !prefix echo in /home/.darkmatter.yaml"), "{prompt}");
    }

    #[test]
    fn without_a_home_an_always_choice_approves_the_run_only() {
        let mut request = request();
        request.save_target = None;

        let (decision, prompt) = session(&request, "1\n");

        assert_eq!(decision, ShellApprovalDecision::AllowExactPersist);
        assert!(
            prompt.contains("allows \"exact echo hello\" for this run; there is no home to save it to"),
            "{prompt}"
        );
    }

    #[test]
    fn input_that_ends_without_an_answer_denies() {
        let request = request();

        let (decision, _) = session(&request, "");

        assert_eq!(decision, ShellApprovalDecision::Deny);
    }

    mod slot_reads {
        use super::*;
        use darkmatter::markdown::compose::expression::read_gate::ReadLocation;

        fn grant(decision: GrantDecision, target: GrantTarget, scope: GrantScope) -> Grant {
            Grant { decision, target, scope }
        }

        fn file_request() -> SlotGrantRequest {
            let file = PathBuf::from("/repo/data/os.yaml");
            let allow = GrantDecision::Allow;
            SlotGrantRequest {
                function: "load_data".into(),
                kind: ReadKind::Content,
                location: ReadLocation::Local(file.clone()),
                schema: Some(PathBuf::from("/repo/schemas/sandbox.yaml")),
                choices: vec![
                    grant(allow, GrantTarget::File(file.clone()), GrantScope::Once),
                    grant(allow, GrantTarget::File(file.clone()), GrantScope::Persisted),
                    grant(GrantDecision::Deny, GrantTarget::File(file), GrantScope::Once),
                ],
            }
        }

        fn ask(requests: &[SlotGrantRequest], answers: &str) -> (Vec<SlotGrantDecision>, String) {
            let mut input = Cursor::new(answers.as_bytes().to_vec());
            let mut output = Vec::new();
            let decisions = approve_slot_reads_with_io(requests, &mut input, &mut output).unwrap();
            (decisions, strip_ansi(&String::from_utf8(output).unwrap()))
        }

        #[test]
        fn every_choice_is_numbered_and_always_allow_names_the_permissions() {
            let (decisions, prompt) = ask(&[file_request()], "2\n");
            assert_eq!(
                decisions,
                [SlotGrantDecision::Allow {
                    target: GrantTarget::File(PathBuf::from("/repo/data/os.yaml")),
                    scope: GrantScope::Persisted
                }]
            );
            assert!(prompt.contains("Schema Read Approval Required"), "{prompt}");
            assert!(prompt.contains("/repo/schemas/sandbox.yaml"), "{prompt}");
            assert!(prompt.contains("load_data() reads /repo/data/os.yaml"), "{prompt}");
            for (number, label) in
                [(1, "Allow this file for this run"), (2, "Always allow this file"), (3, "Deny for this run")]
            {
                assert!(prompt.contains(&format!("{number} \u{2502} {label}")), "{label}: {prompt}");
            }
            assert!(prompt.contains("saved to your permissions.file-read"), "{prompt}");
            assert!(!prompt.contains("Always deny"), "no persistent deny: {prompt}");
            assert!(!prompt.contains("folder"), "no folder approval: {prompt}");
        }

        #[test]
        fn deny_for_this_run_is_a_choice_and_the_answer_to_ended_input() {
            let deny_once = SlotGrantDecision::Deny {
                target: GrantTarget::File(PathBuf::from("/repo/data/os.yaml")),
                scope: GrantScope::Once,
            };
            let (picked, _) = ask(&[file_request()], "3\n");
            assert_eq!(picked, std::slice::from_ref(&deny_once));
            // Input ends before the first answer: this and every later
            // request are denied for the run, and only one prompt is shown.
            let (ended, prompt) = ask(&[file_request(), file_request()], "");
            assert_eq!(ended, [deny_once.clone(), deny_once]);
            assert_eq!(prompt.matches("Schema Read Approval Required").count(), 1);
        }

        #[test]
        fn a_number_naming_no_choice_asks_again() {
            let (decisions, prompt) = ask(&[file_request()], "0\n9\n1\n");
            assert_eq!(
                decisions,
                [SlotGrantDecision::Allow {
                    target: GrantTarget::File(PathBuf::from("/repo/data/os.yaml")),
                    scope: GrantScope::Once
                }]
            );
            assert_eq!(prompt.matches("Invalid choice. Please enter 1-3.").count(), 2);
        }

        #[test]
        fn a_url_read_offers_this_run_only() {
            let url = biscuit_file::url::Url::parse("https://example.com/os.yaml?v=2").unwrap();
            let request = SlotGrantRequest {
                function: "load_data".into(),
                kind: ReadKind::Content,
                location: ReadLocation::Remote(url.clone()),
                schema: None,
                choices: vec![
                    grant(GrantDecision::Allow, GrantTarget::Url(url.clone()), GrantScope::Once),
                    grant(GrantDecision::Deny, GrantTarget::Url(url), GrantScope::Once),
                ],
            };
            let (_, prompt) = ask(&[request], "2\n");
            assert!(prompt.contains("1 \u{2502} Allow this URL for this run"), "{prompt}");
            assert!(prompt.contains("2 \u{2502} Deny for this run"), "{prompt}");
            assert!(!prompt.contains("Always"), "a URL is never saved: {prompt}");
            assert!(!prompt.contains("Schema:"), "an inline schema names no file");
        }
    }
}
