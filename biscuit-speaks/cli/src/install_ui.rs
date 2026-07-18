//! Install-interview delegate for the `so-you-say` CLI.
//!
//! Adapts `sniff`'s `InstallInterviewDelegate` protocol to the existing
//! owo-colors + inquire aesthetic used throughout the rest of the CLI.

use inquire::{Confirm, Select};
use owo_colors::OwoColorize;
use sniff::error::SniffInstallationError;
use sniff::programs::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallOutputStream, InstallStatusKind,
    RetryChoice, RetryPrompt,
};

/// Delegate that renders install-interview events to stdout/stderr using the
/// CLI's existing styling conventions.
pub struct SoYouSayInstallUi;

impl SoYouSayInstallUi {
    pub fn new() -> Self {
        Self
    }
}

impl InstallInterviewDelegate for SoYouSayInstallUi {
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
        match event {
            InstallInterviewEvent::Announcement { prose } => {
                println!("{}", strip_prose_tags(prose));
            }
            InstallInterviewEvent::ConsentWarning { prose } => {
                println!();
                println!("{}", strip_prose_tags(prose).yellow());
            }
            InstallInterviewEvent::TimeoutWarning { prose } => {
                println!();
                println!("{}", strip_prose_tags(prose).yellow());
            }
            InstallInterviewEvent::CapturedOutput { stream, body } => {
                let prefix = match stream {
                    InstallOutputStream::Stdout => "│".dimmed().to_string(),
                    InstallOutputStream::Stderr => "│".red().to_string(),
                };
                for line in body.lines() {
                    println!("  {} {}", prefix, line);
                }
                println!();
            }
            InstallInterviewEvent::Status { kind, text } => {
                let (glyph, styled) = match kind {
                    InstallStatusKind::Success => {
                        ("✓".green().bold().to_string(), strip_prose_tags(text))
                    }
                    InstallStatusKind::Error => {
                        ("✗".red().bold().to_string(), strip_prose_tags(text))
                    }
                };
                println!("  {} {}", glyph, styled);
            }
        }
        Ok(())
    }

    fn confirm_remote_script(&mut self, _prose: &str) -> Result<bool, SniffInstallationError> {
        match Confirm::new("Proceed with remote-script install?")
            .with_default(false)
            .prompt()
        {
            Ok(answer) => Ok(answer),
            Err(inquire::InquireError::OperationCanceled) => Ok(false),
            Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
            Err(e) => Err(SniffInstallationError::InstallationError {
                pkg: String::new(),
                cmd: e.to_string(),
            }),
        }
    }

    fn choose_retry(
        &mut self,
        prompt: &RetryPrompt,
    ) -> Result<RetryChoice, SniffInstallationError> {
        for choice in &prompt.choices {
            println!("  {}", strip_prose_tags(&choice.prose));
        }

        let quit_label = "Quit (and try manually if desired)".to_string();
        let labels: Vec<String> = prompt
            .choices
            .iter()
            .map(|c| c.label.clone())
            .chain(std::iter::once(quit_label.clone()))
            .collect();

        match Select::new("How do you want to proceed?", labels).prompt() {
            Ok(selected) if selected == quit_label => Ok(RetryChoice::Quit),
            Ok(selected) => {
                let idx = prompt
                    .choices
                    .iter()
                    .position(|c| c.label == selected)
                    .expect("selected label must exist in choices");
                Ok(RetryChoice::RetryWith(prompt.choices[idx].method.clone()))
            }
            Err(inquire::InquireError::OperationCanceled) => Ok(RetryChoice::Quit),
            Err(inquire::InquireError::OperationInterrupted) => std::process::exit(130),
            Err(e) => Err(SniffInstallationError::InstallationError {
                pkg: String::new(),
                cmd: e.to_string(),
            }),
        }
    }
}

/// Strip the lightweight HTML-like markup that sniff's prose strings use
/// (`<b>`, `<yellow>`, `<a href="...">`, etc.) while preserving the visible
/// text. Good enough for single-line announcements and status lines; we do
/// not try to reproduce color semantics here.
fn strip_prose_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '<' {
            for nc in chars.by_ref() {
                if nc == '>' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_simple_bold_tags() {
        assert_eq!(strip_prose_tags("install <b>rg</b>"), "install rg");
    }

    #[test]
    fn strips_anchor_with_href() {
        let input = r#"see <a href="https://example.com">example.com</a>"#;
        assert_eq!(strip_prose_tags(input), "see example.com");
    }

    #[test]
    fn strips_nested_and_multi_tag() {
        let input = "<yellow>Warning:</yellow> install <b>foo</b>";
        assert_eq!(strip_prose_tags(input), "Warning: install foo");
    }

    #[test]
    fn preserves_plain_text() {
        assert_eq!(strip_prose_tags("plain text"), "plain text");
    }

    #[test]
    fn handles_unterminated_tag() {
        // Unterminated `<` swallows the rest; acceptable fallback.
        assert_eq!(strip_prose_tags("a<b"), "a");
    }
}
