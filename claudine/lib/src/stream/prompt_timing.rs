//! Prompt-scoped timing surface used by structured streaming runs.
//!
//! Provides the periodic timing header emitted at `t=0`, `t=10m`, `t=20m`, …
//! and the two fire-once warning messages for `timeout_warn` and
//! `step_timeout_warn`. All three emissions share the same prompt context
//! (absolute path for OSC8, relative display path, humanized durations,
//! local wall-clock time with timezone).
//!
//! Formatting lives here; rendering to `Status::Info` / `Status::Warning`
//! and writing to stderr happens in the CLI layer.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Local, TimeZone};
use chrono_tz::Tz;

/// Everything a streaming run needs to emit the prompt-scoped timing
/// header and its associated warnings.
///
/// Built by the CLI layer once per prompt run and threaded into the
/// structured-stream executor. `None` for wrapper passthrough runs that
/// are not anchored on a prompt file.
#[derive(Debug, Clone)]
pub struct PromptTimingContext {
    /// Absolute path to the prompt source file — converted to the file URL
    /// used as the `href` of every OSC8 link.
    pub absolute_path: PathBuf,
    /// Visible text for the `{prompt}` slot in every user-facing timing
    /// message (header ticks and the two `*_warn` lines). Rendered as
    /// the OSC8 link body.
    pub display_path: String,
    /// Fires one Status WARN once when the prompt has been running this
    /// long.
    pub timeout_warn: Option<Duration>,
    /// Fires one Status WARN per stall episode when the provider has
    /// gone silent for this long.
    pub step_timeout_warn: Option<Duration>,
}

/// Discriminates between the one-time `t=0` header and subsequent ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderKind {
    /// `⏱️ {hh}:{mm}{tz} running the {prompt} prompt` — no duration.
    Zero,
    /// `⏱️ {hh}:{mm}{tz} running the {prompt} prompt for {duration}`.
    Tick,
}

/// Cadence of the periodic timing header.
pub const HEADER_CADENCE: Duration = Duration::from_secs(600);

/// Format a duration per the spec's duration rendering rules.
///
/// - Under 60 seconds: `Ns` (e.g. `45s`).
/// - 1–59 minutes: `Nm` (e.g. `12m`).
/// - 60 minutes or more: `Hh Mm` with a single space between the two
///   segments (e.g. `1h 30m`, `2h 5m`). Never zero-padded; never
///   colon-separated.
///
/// Shared by the periodic header's trailing duration, every `{duration}`
/// occurrence inside `timeout_warn` / `step_timeout_warn` messages, and
/// the "remaining time until hard timeout" segment in the "with timeout
/// also set" variants.
pub fn format_timing_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    if total_secs < 60 {
        format!("{total_secs}s")
    } else if total_secs < 3_600 {
        format!("{}m", total_secs / 60)
    } else {
        let hours = total_secs / 3_600;
        let minutes = (total_secs % 3_600) / 60;
        format!("{hours}h {minutes}m")
    }
}

/// Render the current local time as `{HH}:{MM} {TZ}` (e.g. `14:13 PDT`).
///
/// Resolves the system's IANA timezone once per call via
/// `iana_time_zone::get_timezone()` so `%Z` yields a three-letter
/// abbreviation (PDT/EST/CET/…) rather than the numeric offset that
/// `chrono::Local` alone would produce. Falls back to the numeric
/// offset form `{HH}:{MM} {±HH:MM}` if the system timezone cannot be
/// resolved.
pub fn format_local_time_now() -> String {
    match system_timezone() {
        Some(tz) => format_local_time_at(Local::now().with_timezone(&tz)),
        None => Local::now().format("%H:%M %:z").to_string(),
    }
}

/// Render a specific timestamp as `{HH}:{MM} {TZ}`.
///
/// Exposed primarily for deterministic testing; production call sites
/// should prefer [`format_local_time_now`]. The caller is responsible
/// for picking a timezone whose `%Z` format yields the desired
/// abbreviation (use `chrono_tz::Tz` for named zones; `chrono::Local`
/// will render a numeric offset).
pub fn format_local_time_at<Z>(now: DateTime<Z>) -> String
where
    Z: TimeZone,
    Z::Offset: std::fmt::Display,
{
    now.format("%H:%M %Z").to_string()
}

fn system_timezone() -> Option<Tz> {
    iana_time_zone::get_timezone().ok()?.parse().ok()
}

/// Render the prompt-scoped periodic header in Prose markup.
///
/// The `{prompt}` slot becomes an OSC8 link whose visible text is
/// `display_path` (blue) and whose `href` is a file URL for the absolute
/// prompt path. If the path cannot become a file URL, the text stays unlinked.
pub fn render_header_prose(
    kind: HeaderKind,
    elapsed: Duration,
    ctx: &PromptTimingContext,
) -> String {
    let local_time = format_local_time_now();
    let link = prompt_link(ctx);
    match kind {
        HeaderKind::Zero => {
            format!("\u{23f1}\u{fe0f} {local_time} <i><dim>running the {link} prompt</dim></i>")
        }
        HeaderKind::Tick => {
            let duration = format_timing_duration(elapsed);
            format!(
                "\u{23f1}\u{fe0f} {local_time} <i><dim>running the {link} prompt for</dim></i> {duration}"
            )
        }
    }
}

/// Render the `timeout_warn` message body in Prose markup.
///
/// `elapsed` — wall-clock time since prompt start.
/// `hard_timeout_remaining` — `Some((remaining, deadline))` when a hard
/// `timeout` is also configured; `None` when only the warn is set.
pub fn render_timeout_warn_prose(
    elapsed: Duration,
    hard_timeout_remaining: Option<(Duration, DateTime<Local>)>,
    ctx: &PromptTimingContext,
) -> String {
    let link = prompt_link(ctx);
    let elapsed_str = format_timing_duration(elapsed);
    match hard_timeout_remaining {
        Some((remaining, deadline)) => {
            let deadline_str = deadline.format("%H:%M").to_string();
            let remaining_str = format_timing_duration(remaining);
            format!(
                "the {link} has been running for {elapsed_str}, \
                 this is longer than we'd expect it to take but we won't timeout this prompt \
                 until we reach {deadline_str} in {remaining_str}."
            )
        }
        None => format!(
            "the {link} has been running for {elapsed_str}, \
             this is longer than we'd expect it to take. \
             Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung."
        ),
    }
}

/// Render the `step_timeout_warn` message body in Prose markup.
///
/// `silence` — silence since the last provider event.
/// `hard_step_remaining` — `Some((remaining, deadline))` when a hard
/// `step_timeout` is also configured; `None` when only the warn is set.
pub fn render_step_timeout_warn_prose(
    silence: Duration,
    hard_step_remaining: Option<(Duration, DateTime<Local>)>,
    ctx: &PromptTimingContext,
) -> String {
    let link = prompt_link(ctx);
    let silence_str = format_timing_duration(silence);
    match hard_step_remaining {
        Some((remaining, deadline)) => {
            let deadline_str = deadline.format("%H:%M").to_string();
            let remaining_str = format_timing_duration(remaining);
            format!(
                "the {link} has not produced output for {silence_str}, \
                 this is longer than we'd expect, but we won't abort this step \
                 until we reach {deadline_str} in {remaining_str}."
            )
        }
        None => format!(
            "the {link} has not produced output for {silence_str}, \
             this is longer than we'd expect. \
             Press CTRL+C to terminate this prompt if you're convinced that the prompt has hung."
        ),
    }
}

fn prompt_link(ctx: &PromptTimingContext) -> String {
    let text = escape_prose(&ctx.display_path);
    match url::Url::from_file_path(&ctx.absolute_path) {
        Ok(href) => format!("<a href=\"{href}\"><blue>{text}</blue></a>"),
        Err(()) => format!("<blue>{text}</blue>"),
    }
}

fn escape_prose(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '<' => out.push_str("\\<"),
            '>' => out.push_str("\\>"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx() -> PromptTimingContext {
        PromptTimingContext {
            absolute_path: std::env::temp_dir().join("repo/prompts/run.md"),
            display_path: "prompts/run.md".to_string(),
            timeout_warn: None,
            step_timeout_warn: None,
        }
    }

    #[test]
    fn format_seconds_for_sub_minute() {
        assert_eq!(format_timing_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_timing_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_timing_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn format_minutes_for_sub_hour() {
        assert_eq!(format_timing_duration(Duration::from_secs(60)), "1m");
        assert_eq!(format_timing_duration(Duration::from_secs(12 * 60)), "12m");
        assert_eq!(format_timing_duration(Duration::from_secs(59 * 60)), "59m");
    }

    #[test]
    fn format_hours_and_minutes_when_at_least_one_hour() {
        assert_eq!(format_timing_duration(Duration::from_secs(3_600)), "1h 0m");
        assert_eq!(
            format_timing_duration(Duration::from_secs(3_600 + 30 * 60)),
            "1h 30m"
        );
        assert_eq!(
            format_timing_duration(Duration::from_secs(2 * 3_600 + 5 * 60)),
            "2h 5m"
        );
    }

    #[test]
    fn header_zero_omits_duration_and_trailing_for() {
        let rendered = render_header_prose(HeaderKind::Zero, Duration::ZERO, &ctx());
        assert!(
            rendered.contains("running the"),
            "expected header text: {rendered}"
        );
        assert!(
            rendered.contains("prompt</dim></i>"),
            "Zero header must not include the trailing 'for': {rendered}"
        );
        assert!(
            !rendered.contains(" for "),
            "Zero header must not contain ' for ': {rendered}"
        );
    }

    #[test]
    fn header_tick_includes_duration_and_trailing_for() {
        let rendered = render_header_prose(HeaderKind::Tick, Duration::from_secs(600), &ctx());
        assert!(
            rendered.contains("running the"),
            "expected header text: {rendered}"
        );
        assert!(
            rendered.contains("prompt for</dim></i> 10m"),
            "Tick header must carry trailing 'for' and duration: {rendered}"
        );
    }

    #[test]
    fn header_renders_prompt_as_osc8_blue_link() {
        let rendered = render_header_prose(HeaderKind::Zero, Duration::ZERO, &ctx());
        let href = url::Url::from_file_path(&ctx().absolute_path).unwrap();
        assert!(
            rendered.contains(&format!("<a href=\"{href}\">")),
            "prompt must render as OSC8 link with absolute href: {rendered}"
        );
        assert!(
            rendered.contains("<blue>prompts/run.md</blue>"),
            "visible link text must be the display_path in blue: {rendered}"
        );
    }

    #[test]
    fn prompt_link_encodes_reserved_file_url_characters() {
        let mut context = ctx();
        context.absolute_path = std::env::temp_dir().join("a b#%.md");

        let rendered = prompt_link(&context);

        assert!(
            rendered.contains("a%20b%23%25.md"),
            "expected encoded URL: {rendered}"
        );
    }

    #[test]
    fn prompt_link_falls_back_to_plain_text_for_relative_target() {
        let mut context = ctx();
        context.absolute_path = PathBuf::from("relative.md");

        assert_eq!(prompt_link(&context), "<blue>prompts/run.md</blue>");
    }

    #[test]
    fn timeout_warn_with_hard_timeout_includes_both_durations() {
        let elapsed = Duration::from_secs(45 * 60);
        let remaining = Duration::from_secs(15 * 60);
        let deadline = Local::now();
        let rendered = render_timeout_warn_prose(elapsed, Some((remaining, deadline)), &ctx());
        assert!(
            rendered.contains("running for 45m"),
            "elapsed duration must be rendered: {rendered}"
        );
        assert!(
            rendered.contains("in 15m"),
            "remaining duration must be rendered: {rendered}"
        );
    }

    #[test]
    fn timeout_warn_without_hard_timeout_mentions_ctrl_c() {
        let elapsed = Duration::from_secs(120);
        let rendered = render_timeout_warn_prose(elapsed, None, &ctx());
        assert!(
            rendered.contains("running for 2m"),
            "elapsed must be rendered: {rendered}"
        );
        assert!(
            rendered.contains("Press CTRL+C"),
            "no-hard-timeout variant must mention Ctrl+C: {rendered}"
        );
    }

    #[test]
    fn step_timeout_warn_with_hard_step_timeout_includes_both_durations() {
        let silence = Duration::from_secs(90);
        let remaining = Duration::from_secs(30);
        let deadline = Local::now();
        let rendered = render_step_timeout_warn_prose(silence, Some((remaining, deadline)), &ctx());
        assert!(
            rendered.contains("not produced output for 1m"),
            "silence duration must render in minutes: {rendered}"
        );
        assert!(
            rendered.contains("in 30s"),
            "remaining must be rendered in seconds: {rendered}"
        );
    }

    #[test]
    fn step_timeout_warn_without_hard_step_timeout_mentions_ctrl_c() {
        let silence = Duration::from_secs(75);
        let rendered = render_step_timeout_warn_prose(silence, None, &ctx());
        assert!(rendered.contains("not produced output for 1m"));
        assert!(rendered.contains("Press CTRL+C"));
    }

    #[test]
    fn format_local_time_at_named_tz_yields_abbreviation() {
        let tz: Tz = "America/Los_Angeles".parse().expect("valid IANA name");
        let when = tz
            .with_ymd_and_hms(2026, 7, 15, 14, 13, 0)
            .single()
            .expect("unambiguous local time");
        let rendered = format_local_time_at(when);
        assert_eq!(
            rendered, "14:13 PDT",
            "named-zone DateTime must render %Z as a short abbreviation"
        );
    }

    #[test]
    fn format_local_time_now_renders_non_numeric_tz_on_host() {
        let rendered = format_local_time_now();
        let tz_segment = rendered
            .rsplit_once(' ')
            .map(|(_, tz)| tz)
            .expect("expected 'HH:MM TZ' layout");
        assert!(
            !tz_segment.starts_with('-') && !tz_segment.starts_with('+'),
            "expected a TZ abbreviation (e.g. PDT), got numeric offset: {rendered}"
        );
    }
}
