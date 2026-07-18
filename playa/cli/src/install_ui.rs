use std::io::Write;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use sniff::error::SniffInstallationError;
use sniff::programs::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallOutputStream, InstallStatusKind,
    RetryChoice, RetryPrompt,
};

pub struct CliInstallUi {
    pub terminal: Terminal,
    pub plain: bool,
}

impl CliInstallUi {
    pub fn new(terminal: Terminal, plain: bool) -> Self {
        Self { terminal, plain }
    }

    /// Renders a prose-carrying event body, newline-terminated so consecutive
    /// events never share a line.
    fn render_prose_line(&self, prose: &str) -> String {
        let rendered = Prose::new(prose.to_owned()).render(&self.terminal);
        if rendered.ends_with('\n') {
            rendered
        } else {
            format!("{rendered}\n")
        }
    }

    fn emit(&mut self, text: &str) {
        let output = if self.plain {
            strip_escape_codes(text)
        } else {
            text.to_owned()
        };
        print!("{output}");
        let _ = std::io::stdout().flush();
    }
}

impl InstallInterviewDelegate for CliInstallUi {
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
        match event {
            InstallInterviewEvent::Announcement { prose }
            | InstallInterviewEvent::ConsentWarning { prose }
            | InstallInterviewEvent::TimeoutWarning { prose } => {
                let line = self.render_prose_line(prose);
                self.emit(&line);
            }

            InstallInterviewEvent::CapturedOutput {
                stream: InstallOutputStream::Stdout,
                body,
            } => {
                let rendered = BlockQuote::from(body.as_str())
                    .with_left_block_color(Color::Tailwind(Tailwind::Gray500))
                    .render(&self.terminal);
                let with_blank = ensure_trailing_blank_line(&rendered);
                self.emit(&with_blank);
            }

            InstallInterviewEvent::CapturedOutput {
                stream: InstallOutputStream::Stderr,
                body,
            } => {
                let rendered = BlockQuote::from(body.as_str())
                    .with_left_block_color(Color::Tailwind(Tailwind::Red500))
                    .render(&self.terminal);
                let with_blank = ensure_trailing_blank_line(&rendered);
                self.emit(&with_blank);
            }

            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Success,
                text,
            } => {
                let rendered = Status::from_prose(text.clone())
                    .state(StatusState::Success)
                    .theme(StatusTheme::Circular)
                    .render(&self.terminal);
                let line = if rendered.ends_with('\n') {
                    rendered
                } else {
                    format!("{rendered}\n")
                };
                self.emit(&line);
            }

            InstallInterviewEvent::Status {
                kind: InstallStatusKind::Error,
                text,
            } => {
                let rendered = Status::from_prose(text.clone())
                    .state(StatusState::Error)
                    .theme(StatusTheme::Circular)
                    .render(&self.terminal);
                let line = if rendered.ends_with('\n') {
                    rendered
                } else {
                    format!("{rendered}\n")
                };
                self.emit(&line);
            }
        }

        Ok(())
    }

    fn confirm_remote_script(&mut self, _prose: &str) -> Result<bool, SniffInstallationError> {
        match inquire::Confirm::new("Proceed with remote-script install?")
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
            let rendered = Prose::new(choice.prose.clone()).render(&self.terminal);
            let line = if rendered.ends_with('\n') {
                rendered
            } else {
                format!("{rendered}\n")
            };
            self.emit(&line);
        }

        let quit_label = "Quit (and try manually if desired)".to_string();
        let labels: Vec<String> = prompt
            .choices
            .iter()
            .map(|c| c.label.clone())
            .chain(std::iter::once(quit_label.clone()))
            .collect();

        match inquire::Select::new("How do you want to proceed?", labels).prompt() {
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

fn ensure_trailing_blank_line(s: &str) -> String {
    let trimmed = s.trim_end_matches('\n');
    format!("{trimmed}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_warning_prose_reaches_the_rendered_line() {
        let ui = CliInstallUi::new(Terminal::default(), true);

        let line = ui.render_prose_line(
            "The installer was stopped at its deadline; it may have left partial changes.",
        );

        let plain = strip_escape_codes(&line);
        assert!(plain.contains("stopped at its deadline"), "got: {plain}");
        assert!(line.ends_with('\n'));
    }
}
