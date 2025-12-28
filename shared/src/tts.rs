//! Text-to-Speech utilities
//!
//! Provides cross-platform text-to-speech functionality using the system's
//! native TTS engine. Includes voice selection and blocking speech.

use tts::Tts;

/// Speak a message using the system's text-to-speech engine.
///
/// This function:
/// - Creates a TTS instance
/// - Selects an appropriate English voice (non-compact, non-eloquence)
/// - Speaks the message and blocks until complete
///
/// Errors are silently ignored - TTS is a nice-to-have feature.
///
/// # Arguments
/// * `message` - The text to speak
///
/// # Example
/// ```ignore
/// shared::tts::speak("Hello, world!");
/// ```
pub fn speak(message: &str) {
    if let Ok(mut tts) = Tts::default() {
        // Try to select a good English voice
        if let Ok(voices) = tts.voices() {
            if let Some(voice) = voices.iter().find(|v| {
                let id = v.id().to_lowercase();
                !id.contains("compact")
                    && !id.contains("eloquence")
                    && v.language().starts_with("en")
            }) {
                let _ = tts.set_voice(voice);
            }
        }

        // Speak and wait for completion
        if tts.speak(message, false).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            while tts.is_speaking().unwrap_or(false) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

/// Announce the completion of a task.
///
/// Convenience function that formats and speaks a completion message.
///
/// # Arguments
/// * `task` - Description of the completed task
///
/// # Example
/// ```ignore
/// shared::tts::announce_completion("clap library research");
/// ```
pub fn announce_completion(task: &str) {
    let message = format!("{} has completed", task);
    speak(&message);
}

/// Announce the completion of library research.
///
/// Convenience function specifically for research completion announcements.
///
/// # Arguments
/// * `library` - Name of the library that was researched
///
/// # Example
/// ```ignore
/// shared::tts::announce_research_complete("rig-core");
/// ```
pub fn announce_research_complete(library: &str) {
    let message = format!("Research for the {} library has completed", library);
    speak(&message);
}

/// Format a completion message for a task.
///
/// This is the internal formatting used by `announce_completion`.
/// Exposed for testing purposes.
#[doc(hidden)]
pub fn format_completion_message(task: &str) -> String {
    format!("{} has completed", task)
}

/// Format a research completion message for a library.
///
/// This is the internal formatting used by `announce_research_complete`.
/// Exposed for testing purposes.
#[doc(hidden)]
pub fn format_research_message(library: &str) -> String {
    format!("Research for the {} library has completed", library)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_completion_message_basic() {
        let result = format_completion_message("database migration");
        assert_eq!(result, "database migration has completed");
    }

    #[test]
    fn test_format_completion_message_with_library() {
        let result = format_completion_message("clap library research");
        assert_eq!(result, "clap library research has completed");
    }

    #[test]
    fn test_format_completion_message_empty() {
        let result = format_completion_message("");
        assert_eq!(result, " has completed");
    }

    #[test]
    fn test_format_research_message_basic() {
        let result = format_research_message("tokio");
        assert_eq!(result, "Research for the tokio library has completed");
    }

    #[test]
    fn test_format_research_message_with_dashes() {
        let result = format_research_message("rig-core");
        assert_eq!(result, "Research for the rig-core library has completed");
    }

    #[test]
    fn test_format_research_message_with_spaces() {
        let result = format_research_message("my library");
        assert_eq!(result, "Research for the my library library has completed");
    }

    #[test]
    fn test_format_research_message_empty() {
        let result = format_research_message("");
        assert_eq!(result, "Research for the  library has completed");
    }
}
