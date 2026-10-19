//! Interactive shell command approval handler for the CLI.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use std::io::{self, BufRead, IsTerminal, Write};

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
    Blacklist,
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
            (true, Choice::Blacklist),
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
            Choice::Blacklist => ShellApprovalDecision::BlacklistPersist,
        }
    }

    fn color(self) -> &'static str {
        match self {
            Choice::Exact | Choice::Shape | Choice::Prefix => "green",
            Choice::Once => "cyan",
            Choice::Deny => "yellow",
            Choice::Blacklist => "red",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Choice::Exact => "Allow this exact command",
            Choice::Shape => "Allow this command shape",
            Choice::Prefix => "Allow this program with any arguments",
            Choice::Once => "Allow once",
            Choice::Deny => "Deny",
            Choice::Blacklist => "Blacklist and stop",
        }
    }

    /// What choosing this line does, naming each policy rule it saves.
    fn effect(self, request: &ShellApprovalRequest) -> String {
        let shape = request
            .template
            .as_ref()
            .map(|template| template.shape.canonical_text());
        let resolved = request.offered.exact && !request.normalized_exact.is_empty();
        match self {
            Choice::Exact if !request.chain_executables.is_empty() => {
                "saves an exact rule for each command of the chain to the whitelist".to_string()
            }
            Choice::Exact => format!("saves {} to the whitelist", rule("exact", &request.normalized_exact)),
            Choice::Shape => format!(
                "saves {} to the whitelist; runs as a template",
                rule("template", shape.as_deref().unwrap_or_default())
            ),
            Choice::Prefix => {
                let rules = prefix_programs(request)
                    .iter()
                    .map(|program| rule("prefix", program))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("saves {rules} to the whitelist")
            }
            Choice::Once if !resolved && shape.is_some() => "this command shape, this session only".to_string(),
            Choice::Once => "this session only".to_string(),
            Choice::Deny => "stops the run".to_string(),
            Choice::Blacklist => match (resolved, shape) {
                (true, _) if !request.chain_executables.is_empty() => {
                    "saves an exact rule for each command of the chain to the blacklist; stops the run".to_string()
                }
                (true, _) => format!(
                    "saves {} to the blacklist; stops the run",
                    rule("exact", &request.normalized_exact)
                ),
                (false, Some(shape)) => format!("saves {} to the blacklist; stops the run", rule("template", &shape)),
                (false, None) => format!(
                    "saves {} to the blacklist; stops the run",
                    rule("exact", &request.normalized_exact)
                ),
            },
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
/// `PolicyIo` (naming the whitelist path) when reading the answer or writing
/// the prompt fails.
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
    let io_error = |source| ShellExpansionError::PolicyIo {
        path: request.whitelist_path.clone(),
        source,
    };

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
    let p = &request.whitelist_path;
    let w = |output: &mut W, args: std::fmt::Arguments| -> Result<(), ShellExpansionError> {
        output
            .write_fmt(args)
            .map_err(|source| ShellExpansionError::PolicyIo {
                path: p.to_path_buf(),
                source,
            })
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
        .map_err(|source| ShellExpansionError::PolicyIo {
            path: request.whitelist_path.clone(),
            source,
        })
}

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
            whitelist_path: PathBuf::from("/tmp/.darkmatter-shell-whitelist"),
            blacklist_path: PathBuf::from("/tmp/.darkmatter-shell-blacklist"),
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
        assert!(prompt.contains("Invalid choice. Please enter 1-5."));
        // Prompt should appear twice (once for invalid, once for retry)
        assert_eq!(prompt.matches("Shell Approval Required").count(), 2);
    }

    #[test]
    fn all_five_choices_produce_correct_decisions() {
        let cases = [
            ("1\n", ShellApprovalDecision::AllowExactPersist),
            ("2\n", ShellApprovalDecision::AllowCommandPersist),
            ("3\n", ShellApprovalDecision::AllowOnce),
            ("4\n", ShellApprovalDecision::Deny),
            ("5\n", ShellApprovalDecision::BlacklistPersist),
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
        assert!(prompt.contains("saves \"exact echo hello\" to the whitelist"));
        assert!(prompt.contains("Allow this program with any arguments"));
        assert!(prompt.contains("saves \"prefix echo\" to the whitelist"));
        assert!(prompt.contains("Allow once"));
        assert!(prompt.contains("Deny"));
        assert!(prompt.contains("Blacklist and stop"));
        assert!(prompt.contains("saves \"exact echo hello\" to the blacklist"));
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
            prompt.contains("saves \"prefix echo\", \"prefix pwd\" to the whitelist"),
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
            prompt.contains("saves \"template some-command {{ string }}/build\" to the whitelist"),
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
        assert!(
            prompt.contains("saves \"template just clean --out {{ string }}/build\" to the blacklist"),
            "{prompt}"
        );
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

        // Five lines are listed; 6 would have been "Blacklist" with all six.
        let (decision, prompt) = session(&request, "6\n0\nexact\n3\n");

        assert_eq!(decision, ShellApprovalDecision::AllowOnce);
        assert_eq!(prompt.matches("Invalid choice. Please enter 1-5.").count(), 3, "{prompt}");
    }

    #[test]
    fn each_listed_choice_returns_its_decision() {
        let cases = [
            (ALL, "1", ShellApprovalDecision::AllowExactPersist),
            (ALL, "2", ShellApprovalDecision::AllowTemplatePersist),
            (ALL, "3", ShellApprovalDecision::AllowCommandPersist),
            (ALL, "4", ShellApprovalDecision::AllowOnce),
            (ALL, "5", ShellApprovalDecision::Deny),
            (ALL, "6", ShellApprovalDecision::BlacklistPersist),
            (TEMPLATE_ONLY, "1", ShellApprovalDecision::AllowTemplatePersist),
            (TEMPLATE_ONLY, "2", ShellApprovalDecision::AllowCommandPersist),
            (TEMPLATE_ONLY, "3", ShellApprovalDecision::AllowOnce),
            (TEMPLATE_ONLY, "4", ShellApprovalDecision::Deny),
            (TEMPLATE_ONLY, "5", ShellApprovalDecision::BlacklistPersist),
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
            prompt.contains("saves \"prefix just\", \"prefix git\" to the whitelist"),
            "{prompt}"
        );
    }

    #[test]
    fn input_that_ends_without_an_answer_denies() {
        let request = request();

        let (decision, _) = session(&request, "");

        assert_eq!(decision, ShellApprovalDecision::Deny);
    }
}
