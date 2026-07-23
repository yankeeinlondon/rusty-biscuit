//! CLI adapter for the install-interview delegate protocol.
//!
//! `CliInstallUi` implements `InstallInterviewDelegate` by rendering each
//! event through the `biscuit-terminal` component library and routing output
//! to stdout (or a test buffer when compiled under `#[cfg(test)]`).

#[cfg(not(test))]
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

/// CLI adapter that renders install-interview events to the terminal.
pub struct CliInstallUi {
    pub terminal: Terminal,
    pub plain: bool,
    #[cfg_attr(not(test), allow(dead_code))]
    pub buffer: Vec<u8>,
}

impl CliInstallUi {
    /// Create a new `CliInstallUi` that writes to stdout.
    pub fn new(terminal: Terminal, plain: bool) -> Self {
        Self {
            terminal,
            plain,
            buffer: Vec::new(),
        }
    }

    /// Emit a string to the appropriate output destination.
    ///
    /// In test builds the string is appended to `self.buffer`. In production
    /// builds it is written to stdout via `print!` followed by a flush.
    fn emit(&mut self, text: &str) {
        let output = if self.plain {
            strip_escape_codes(text)
        } else {
            text.to_owned()
        };

        #[cfg(test)]
        {
            self.buffer.extend_from_slice(output.as_bytes());
        }
        #[cfg(not(test))]
        {
            print!("{output}");
            let _ = std::io::stdout().flush();
        }
    }
}

impl InstallInterviewDelegate for CliInstallUi {
    fn on_event(&mut self, event: &InstallInterviewEvent) -> Result<(), SniffInstallationError> {
        match event {
            InstallInterviewEvent::Announcement { prose } => {
                let rendered = Prose::new(prose.clone()).render(&self.terminal);
                let line = if rendered.ends_with('\n') {
                    rendered
                } else {
                    format!("{rendered}\n")
                };
                self.emit(&line);
            }

            InstallInterviewEvent::ConsentWarning { prose } => {
                let rendered = Prose::new(prose.clone()).render(&self.terminal);
                let line = if rendered.ends_with('\n') {
                    rendered
                } else {
                    format!("{rendered}\n")
                };
                self.emit(&line);
            }

            InstallInterviewEvent::TimeoutWarning { prose } => {
                let rendered = Prose::new(prose.clone()).render(&self.terminal);
                let line = if rendered.ends_with('\n') {
                    rendered
                } else {
                    format!("{rendered}\n")
                };
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
        // Render each choice's prose as a line.
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

/// Ensure a rendered string ends with exactly two newlines (one trailing blank line).
fn ensure_trailing_blank_line(s: &str) -> String {
    let trimmed = s.trim_end_matches('\n');
    format!("{trimmed}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::terminal::Terminal;
    use sniff::programs::{
        InstallInterviewDelegate, InstallInterviewEvent, InstallOutputStream, InstallStatusKind,
    };

    fn ui_capture() -> CliInstallUi {
        CliInstallUi {
            terminal: Terminal::default(),
            plain: true,
            buffer: Vec::new(),
        }
    }

    #[test]
    fn announcement_is_rendered_as_prose() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::Announcement {
            prose: "install <b>rg</b>".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("install"));
        assert!(text.contains("rg"));
    }

    #[test]
    fn stdout_captured_output_is_rendered_in_block_quote() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::CapturedOutput {
            stream: InstallOutputStream::Stdout,
            body: "line1\nline2".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("line1"));
        assert!(text.contains("line2"));
    }

    #[test]
    fn status_success_is_rendered_with_status_component() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::Status {
            kind: InstallStatusKind::Success,
            text: "Ripgrep installed".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("installed"));
    }

    #[test]
    fn timeout_warning_is_rendered_as_prose() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::TimeoutWarning {
            prose: "<yellow>Warning:</yellow> installing <b>rg</b> did not finish within <b>5s</b> \
                    and was terminated. An installer process may still be running."
                .into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(text.contains("did not finish"), "text: {text}");
        assert!(
            text.contains("may still be running"),
            "the detached-descendant hazard must reach the terminal: {text}"
        );
    }

    #[test]
    fn blank_line_is_emitted_after_captured_output() {
        let mut ui = ui_capture();
        ui.on_event(&InstallInterviewEvent::CapturedOutput {
            stream: InstallOutputStream::Stderr,
            body: "boom".into(),
        })
        .unwrap();
        let text = String::from_utf8(ui.buffer).unwrap();
        assert!(
            text.ends_with("\n\n"),
            "expected trailing blank line, got: {:?}",
            text
        );
    }
}
