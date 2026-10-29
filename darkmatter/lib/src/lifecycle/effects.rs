//! The communication service: where a lifecycle stack's `stderr`, `info`,
//! `message`, `say`, `effect`, and `notify` actions go.
//!
//! [`LifecycleEffects`] carries text only. Everything that decides *how* a
//! message is delivered — a messaging route, a TTS voice, the stream a line is
//! written to — belongs to the host's implementation, so a document's
//! lifecycle names no delivery configuration. The terminal text channels render
//! through [`render_prose_line`] and [`render_status_line`], which return the
//! rendered string and leave writing it to the implementor.
//!
//! ## Examples
//!
//! ```
//! use std::sync::Mutex;
//!
//! use darkmatter::lifecycle::LifecycleSignal;
//! use darkmatter::lifecycle::actions::CommunicationChannel;
//! use darkmatter::lifecycle::effects::{LifecycleEffects, dispatch_communication};
//!
//! #[derive(Default)]
//! struct Captured(Mutex<Vec<String>>);
//!
//! impl LifecycleEffects for Captured {
//!     fn stdout(&self, text: &str) { self.0.lock().unwrap().push(format!("stdout {text}")); }
//!     fn stderr(&self, _: LifecycleSignal, text: &str) { self.0.lock().unwrap().push(format!("stderr {text}")); }
//!     fn info(&self, _: &str) {}
//!     fn warn(&self, _: &str) {}
//!     fn success(&self, _: &str) {}
//!     fn message(&self, _: &str) {}
//!     fn speech(&self, _: &str) {}
//!     fn effect(&self, _: &str) {}
//!     fn notification(&self, _: &str) {}
//! }
//!
//! let effects = Captured::default();
//! dispatch_communication(&effects, CommunicationChannel::Stderr, LifecycleSignal::Start, "hi");
//! assert_eq!(*effects.0.lock().unwrap(), ["stderr hi"]);
//! ```

use biscuit_terminal::components::prose::LineBreaks;
use biscuit_terminal::components::status::{Status, StatusState, StatusTheme};
use biscuit_terminal::prelude::{Prose, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;

use super::LifecycleSignal;
use super::actions::CommunicationChannel;

/// The host's delivery of lifecycle communication.
///
/// Each method receives text that has already been evaluated and
/// interpolated. A text channel keeps the rows of its text: a newline
/// (authored, or inside an interpolated value) starts a new line.
pub trait LifecycleEffects: Sync {
    /// Write a plain prose line (no status glyph) to **stdout**, the only
    /// lifecycle channel that targets stdout.
    fn stdout(&self, text: &str);

    /// Write a plain prose line (no status glyph) to stderr for `signal`'s
    /// event.
    fn stderr(&self, signal: LifecycleSignal, text: &str);

    /// Write an `Info` status line to stderr, whatever the owning event.
    fn info(&self, text: &str);

    /// Write a `Warning` status line to stderr, whatever the owning event.
    fn warn(&self, text: &str);

    /// Write a `Success` status line to stderr, whatever the owning event.
    fn success(&self, text: &str);

    /// Send a message through the host's messaging route.
    fn message(&self, text: &str);

    /// Speak `text`.
    fn speech(&self, text: &str);

    /// Play the sound effect called `name`.
    fn effect(&self, name: &str);

    /// Fire a desktop notification titled `title`.
    fn notification(&self, title: &str);
}

/// Route an evaluated communication action's `text` to its `channel`.
///
/// `say` and `speak` are the same channel.
pub fn dispatch_communication(
    effects: &dyn LifecycleEffects,
    channel: CommunicationChannel,
    signal: LifecycleSignal,
    text: &str,
) {
    match channel {
        CommunicationChannel::Say | CommunicationChannel::Speak => effects.speech(text),
        CommunicationChannel::Effect => effects.effect(text),
        CommunicationChannel::Message => effects.message(text),
        CommunicationChannel::Notify => effects.notification(text),
        CommunicationChannel::Stderr => effects.stderr(signal, text),
        CommunicationChannel::Info => effects.info(text),
        CommunicationChannel::Warn => effects.warn(text),
        CommunicationChannel::Success => effects.success(text),
        CommunicationChannel::Stdout => effects.stdout(text),
    }
}

/// Render `text` as the statusless prose line of the `stdout` and `stderr`
/// channels: inline styling and links honored, hard line breaks kept.
pub fn render_prose_line(text: &str, term: &Terminal) -> String {
    Prose::new(text).with_line_breaks(LineBreaks::Hard).render(term)
}

/// Render `text` as the status line of the `info`, `warn`, and `success`
/// channels, decorated for `state`.
pub fn render_status_line(state: StatusState, text: &str, term: &Terminal) -> String {
    Status::from_prose(text)
        .with_line_breaks(LineBreaks::Hard)
        .state(state)
        .theme(StatusTheme::Circular)
        .render(term)
}

#[cfg(test)]
mod tests;
