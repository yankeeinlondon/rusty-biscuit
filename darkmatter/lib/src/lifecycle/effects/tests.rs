//! The communication seam: channel routing and the terminal render helpers.

use std::sync::Mutex;

use biscuit_terminal::components::prose::LineBreaks;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;

use super::*;

#[derive(Default)]
struct Recorded(Mutex<Vec<String>>);

impl Recorded {
    fn push(&self, call: String) {
        self.0.lock().unwrap().push(call);
    }
}

impl LifecycleEffects for Recorded {
    fn stdout(&self, text: &str) {
        self.push(format!("stdout:{text}"));
    }
    fn stderr(&self, signal: LifecycleSignal, text: &str) {
        self.push(format!("stderr:{}:{text}", signal.property_name()));
    }
    fn info(&self, text: &str) {
        self.push(format!("info:{text}"));
    }
    fn warn(&self, text: &str) {
        self.push(format!("warn:{text}"));
    }
    fn success(&self, text: &str) {
        self.push(format!("success:{text}"));
    }
    fn message(&self, text: &str) {
        self.push(format!("message:{text}"));
    }
    fn speech(&self, text: &str) {
        self.push(format!("speech:{text}"));
    }
    fn effect(&self, name: &str) {
        self.push(format!("effect:{name}"));
    }
    fn notification(&self, title: &str) {
        self.push(format!("notification:{title}"));
    }
}

#[test]
fn every_channel_reaches_its_service_method() {
    let effects = Recorded::default();
    for channel in [
        CommunicationChannel::Say,
        CommunicationChannel::Speak,
        CommunicationChannel::Effect,
        CommunicationChannel::Message,
        CommunicationChannel::Notify,
        CommunicationChannel::Stderr,
        CommunicationChannel::Info,
        CommunicationChannel::Warn,
        CommunicationChannel::Success,
        CommunicationChannel::Stdout,
    ] {
        dispatch_communication(&effects, channel, LifecycleSignal::Failure, "x");
    }

    assert_eq!(
        *effects.0.lock().unwrap(),
        [
            "speech:x",
            "speech:x",
            "effect:x",
            "message:x",
            "notification:x",
            "stderr:failure:x",
            "info:x",
            "warn:x",
            "success:x",
            "stdout:x",
        ]
    );
}

/// The status line a host emitter rendered inline before the helper
/// existed; the helper must stay byte-identical to it.
#[test]
fn the_status_helper_renders_what_the_emitter_rendered() {
    let term = Terminal::new_optimistic(80);
    let text = "Build **done**\nnext row";
    let before = Status::from_prose(text)
        .with_line_breaks(LineBreaks::Hard)
        .state(StatusState::Warning)
        .theme(StatusTheme::Circular)
        .render(&term);

    assert_eq!(render_status_line(StatusState::Warning, text, &term), before);
}

#[test]
fn the_prose_helper_renders_what_the_emitter_rendered() {
    let term = Terminal::new_optimistic(80);
    let text = "plain\nrows";
    let before = Prose::new(text).with_line_breaks(LineBreaks::Hard).render(&term);

    assert_eq!(render_prose_line(text, &term), before);
}
