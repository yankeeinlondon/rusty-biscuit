use claudine::reporting::SyncSummary;

use crate::log;

pub(super) fn render_sync_summary(summary: &SyncSummary) {
    log::data("Sync Summary");
    log::data(&format!(
        "Files scanned:               {}",
        summary.files_scanned
    ));
    log::data(&format!(
        "Files rebuilt:               {}",
        summary.files_rebuilt
    ));
    log::data(&format!(
        "Events inserted:             {}",
        summary.events_inserted
    ));
    log::data(&format!(
        "Events skipped:              {}",
        summary.events_skipped
    ));
    log::data(&format!(
        "Anonymous session fallbacks: {}",
        summary.anonymous_session_fallbacks
    ));
    log::data(&format!(
        "Parse failures:              {}",
        summary.parse_failures
    ));

    if !summary.failures.is_empty() {
        log::data("");
        log::data("Failures");
        for failure in &summary.failures {
            log::data(&format!(
                "- {}:{} {}",
                failure.source_file, failure.line_number, failure.message
            ));
        }
    }
}
