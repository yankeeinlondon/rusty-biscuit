//! Early repetition warnings and recovery: nonterminal signals that must
//! never move, delay, or replace the repetition hard stop.

use super::super::*;
use crate::runaway::Trip;

fn detector_with_limit(max_repeats: usize) -> ContentDetector {
    let cfg = DetectorConfig { max_repeats, ..DetectorConfig::default() };
    ContentDetector::new(cfg, CompiledExitExpressions::empty())
}

/// What one line, fed on its own, produced.
#[derive(Debug, Default)]
struct Timeline {
    /// 1-based line numbers of each signal, in order.
    signals: Vec<(usize, RepetitionSignal)>,
    /// 1-based line number of the trip, when one fired.
    trip: Option<(usize, Trip)>,
}

/// Feeds `lines` one at a time (each gets a `\n`), stopping at a trip.
fn run(det: &mut ContentDetector, lines: &[String]) -> Timeline {
    let mut timeline = Timeline::default();
    for (index, line) in lines.iter().enumerate() {
        let observation = det.observe(&format!("{line}\n"));
        timeline.signals.extend(observation.signals.into_iter().map(|signal| (index + 1, signal)));
        if let Some(trip) = observation.trip {
            timeline.trip = Some((index + 1, trip));
            break;
        }
    }
    timeline
}

/// Line number at which a `feed`-only detector trips: the hard-stop
/// schedule warnings must leave untouched.
fn feed_only_trip_line(max_repeats: usize, lines: &[String]) -> Option<usize> {
    let mut det = detector_with_limit(max_repeats);
    lines.iter().position(|line| det.feed(&format!("{line}\n")).is_some()).map(|index| index + 1)
}

fn repeat(block: &[&str], cycles: usize) -> Vec<String> {
    (0..cycles).flat_map(|_| block.iter().map(|line| (*line).to_string())).collect()
}

fn unique(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|index| format!("{prefix} {index}")).collect()
}

fn warnings(timeline: &Timeline) -> Vec<(usize, usize, usize)> {
    timeline
        .signals
        .iter()
        .filter_map(|(line, signal)| match signal {
            RepetitionSignal::Warning { cycle_len, repeats, .. } => Some((*line, *cycle_len, *repeats)),
            RepetitionSignal::Recovered { .. } => None,
        })
        .collect()
}

#[test]
fn threshold_is_half_the_stop_limit_rounded_up_and_absent_below_two() {
    assert_eq!(warning_threshold(30), Some(15));
    assert_eq!(warning_threshold(5), Some(3));
    assert_eq!(warning_threshold(3), Some(2));
    assert_eq!(warning_threshold(2), Some(1));
    assert_eq!(warning_threshold(1), None);
    assert_eq!(warning_threshold(0), None);
    assert_eq!(warning_threshold(usize::MAX), Some(usize::MAX / 2 + 1), "no overflow");
}

#[test]
fn recovery_needs_at_least_eight_lines_or_twice_the_cycle() {
    assert_eq!(recovery_lines(1), 8);
    assert_eq!(recovery_lines(4), 8);
    assert_eq!(recovery_lines(6), 12);
    assert_eq!(recovery_lines(16), 32);
    assert_eq!(recovery_lines(usize::MAX), usize::MAX, "no overflow");
}

#[test]
fn default_limit_warns_at_fifteen_single_line_repeats_and_still_stops_at_thirty() {
    let lines = repeat(&["I will try again."], 40);
    let timeline = run(&mut detector_with_limit(30), &lines);

    assert_eq!(warnings(&timeline), vec![(15, 1, 15)]);
    assert_eq!(
        timeline.signals[0].1,
        RepetitionSignal::Warning { cycle_len: 1, repeats: 15, stop_limit: 30 }
    );
    let (trip_line, trip) = timeline.trip.expect("continued repetition trips");
    assert_eq!(trip, Trip::RunawayRepetition { cycle_len: 1, repeats: 30 });
    assert_eq!(Some(trip_line), feed_only_trip_line(30, &lines));
    assert_eq!(trip_line, 30);
}

#[test]
fn six_line_block_warns_at_fifteen_cycles_before_the_unchanged_stop() {
    let block = ["Done.", "No more.", "End.", "STOP.", "OK.", "Bye."];
    let mut lines = vec!["This is the final listening.".to_string()];
    lines.extend(repeat(&block, 40));
    let timeline = run(&mut detector_with_limit(30), &lines);

    // Preamble + 15 full cycles.
    assert_eq!(warnings(&timeline), vec![(1 + 15 * 6, 6, 15)]);
    let (trip_line, _) = timeline.trip.expect("trips");
    assert_eq!(Some(trip_line), feed_only_trip_line(30, &lines));
    assert_eq!(trip_line, 1 + 30 * 6);
}

#[test]
fn odd_small_limit_warns_at_three_of_five() {
    let lines = repeat(&["loop"], 10);
    let timeline = run(&mut detector_with_limit(5), &lines);
    assert_eq!(warnings(&timeline), vec![(3, 1, 3)]);
    assert_eq!(timeline.trip.map(|(line, _)| line), Some(5));
}

#[test]
fn a_limit_of_one_or_two_has_no_warning_opportunity() {
    for limit in [1, 2] {
        let lines = repeat(&["loop"], 10);
        let timeline = run(&mut detector_with_limit(limit), &lines);
        assert!(timeline.signals.is_empty(), "limit {limit}: {timeline:?}");
        // Detection needs two copies, which already reaches both limits.
        assert_eq!(timeline.trip.map(|(line, _)| line), Some(2), "limit {limit}");
    }
}

#[test]
fn one_chunk_crossing_warning_and_stop_returns_only_the_stop() {
    let mut det = detector_with_limit(30);
    let observation = det.observe(&"spam\n".repeat(30));
    assert_eq!(observation.trip, Some(Trip::RunawayRepetition { cycle_len: 1, repeats: 30 }));
    assert!(observation.signals.is_empty(), "a hard stop takes priority: {observation:?}");
}

#[test]
fn a_volume_stop_in_the_warning_chunk_also_suppresses_the_warning() {
    let cfg = DetectorConfig { max_lines: 16, ..DetectorConfig::default() };
    let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
    let observation = det.observe(&"spam\n".repeat(17));
    assert!(matches!(observation.trip, Some(Trip::RunawayVolume { .. })));
    assert!(observation.signals.is_empty());
}

#[test]
fn warning_chunk_below_the_stop_returns_the_warning() {
    let mut det = detector_with_limit(30);
    let observation = det.observe(&"spam\n".repeat(20));
    assert_eq!(observation.trip, None);
    assert_eq!(
        observation.signals,
        vec![RepetitionSignal::Warning { cycle_len: 1, repeats: 15, stop_limit: 30 }]
    );
}

#[test]
fn a_line_split_across_chunks_warns_when_it_completes() {
    let mut det = detector_with_limit(30);
    assert!(det.observe(&"spam\n".repeat(14)).signals.is_empty());
    let partial = det.observe("sp");
    assert!(partial.signals.is_empty() && partial.trip.is_none());
    let completed = det.observe("am\n");
    assert_eq!(completed.signals.len(), 1, "{completed:?}");
}

#[test]
fn flush_reports_a_warning_on_a_trailing_partial_line() {
    let mut det = detector_with_limit(30);
    assert!(det.observe(&"spam\n".repeat(14)).signals.is_empty());
    assert!(det.observe("spam").signals.is_empty());
    let flushed = det.observe_flush();
    assert_eq!(flushed.trip, None);
    assert_eq!(flushed.signals.len(), 1);
}

#[test]
fn one_episode_warns_once_even_as_it_continues() {
    let lines = repeat(&["again"], 29);
    let timeline = run(&mut detector_with_limit(30), &lines);
    assert_eq!(warnings(&timeline).len(), 1);
    assert!(timeline.trip.is_none());
}

#[test]
fn recovery_takes_eight_nonblank_unrepeated_lines_and_blank_lines_do_not_count() {
    let mut det = detector_with_limit(30);
    let mut lines = repeat(&["again"], 15);
    // Seven distinct lines, each followed by a blank: seven toward recovery.
    for line in unique("progress", 7) {
        lines.push(line);
        lines.push(String::new());
    }
    let timeline = run(&mut det, &lines);
    assert_eq!(warnings(&timeline).len(), 1);
    assert!(!timeline.signals.iter().any(|(_, signal)| matches!(signal, RepetitionSignal::Recovered { .. })));

    let eighth = run(&mut det, &["progress 7".to_string()]);
    assert_eq!(eighth.signals, vec![(1, RepetitionSignal::Recovered { cycle_len: 1 })]);
}

#[test]
fn after_recovery_a_new_episode_must_reach_its_own_threshold() {
    let mut det = detector_with_limit(30);
    let mut lines = repeat(&["again"], 15);
    lines.extend(unique("progress", 8));
    lines.extend(repeat(&["stuck"], 14));
    let first = run(&mut det, &lines);
    assert_eq!(warnings(&first).len(), 1, "the second episode has only 14 repeats: {first:?}");

    let fifteenth = run(&mut det, &["stuck".to_string()]);
    assert_eq!(warnings(&fifteenth), vec![(1, 1, 15)]);
}

#[test]
fn detected_repetition_resets_recovery_including_a_new_block() {
    let mut det = detector_with_limit(30);
    let mut lines = repeat(&["again"], 15);
    lines.extend(unique("progress", 4));
    // A different, two-line block repeating. Its first three lines are not
    // yet a recognized cycle (7 toward recovery); the fourth is, and resets.
    lines.extend(repeat(&["tick", "tock"], 2));
    lines.extend(unique("after", 7));
    let timeline = run(&mut det, &lines);
    assert_eq!(warnings(&timeline).len(), 1, "the new block earns no warning while warned");
    assert!(!timeline.signals.iter().any(|(_, signal)| matches!(signal, RepetitionSignal::Recovered { .. })));

    let eighth = run(&mut det, &["after 7".to_string()]);
    assert_eq!(eighth.signals, vec![(1, RepetitionSignal::Recovered { cycle_len: 1 })]);
}

#[test]
fn a_multiline_block_freezes_its_length_for_recovery() {
    let mut det = detector_with_limit(30);
    let block = ["Done.", "No more.", "End.", "STOP.", "OK.", "Bye."];
    let mut lines = repeat(&block, 15);
    lines.extend(unique("progress", 3));
    // A shorter block interrupts recovery; the frozen length stays 6.
    lines.extend(repeat(&["x"], 2));
    lines.extend(unique("after", 11));
    let timeline = run(&mut det, &lines);
    assert_eq!(warnings(&timeline), vec![(90, 6, 15)]);
    assert!(!timeline.signals.iter().any(|(_, signal)| matches!(signal, RepetitionSignal::Recovered { .. })));

    let twelfth = run(&mut det, &["after 11".to_string()]);
    assert_eq!(twelfth.signals, vec![(1, RepetitionSignal::Recovered { cycle_len: 6 })]);
}

#[test]
fn a_brief_wording_change_is_not_recovery_and_the_stop_keeps_its_schedule() {
    let mut lines = repeat(&["Retrying the build."], 20);
    lines.push("Retrying the build once more.".to_string());
    lines.extend(repeat(&["Retrying the build."], 40));
    let timeline = run(&mut detector_with_limit(30), &lines);

    assert_eq!(warnings(&timeline).len(), 1, "no second warning without recovery: {timeline:?}");
    let (trip_line, _) = timeline.trip.expect("continued repetition trips");
    assert_eq!(Some(trip_line), feed_only_trip_line(30, &lines));
}

#[test]
fn turn_boundaries_neither_advance_nor_reset_recovery() {
    let mut det = detector_with_limit(30);
    let mut lines = repeat(&["again"], 15);
    lines.extend(unique("progress", 4));
    run(&mut det, &lines);
    det.reset_turn();
    let rest = run(&mut det, &unique("more", 4));
    assert_eq!(rest.signals, vec![(4, RepetitionSignal::Recovered { cycle_len: 1 })]);
}

#[test]
fn recovery_and_a_second_warning_leave_the_second_stop_on_its_schedule() {
    let mut lines = repeat(&["again"], 15);
    lines.extend(unique("progress", 8));
    lines.extend(repeat(&["stuck"], 40));
    let timeline = run(&mut detector_with_limit(30), &lines);

    assert_eq!(warnings(&timeline), vec![(15, 1, 15), (15 + 8 + 15, 1, 15)]);
    let (trip_line, _) = timeline.trip.expect("the second episode trips");
    assert_eq!(trip_line, 15 + 8 + 30);
    assert_eq!(Some(trip_line), feed_only_trip_line(30, &lines));
}

#[test]
fn disabled_repetition_detection_produces_no_signals() {
    let cfg = DetectorConfig { repetition_enabled: false, ..DetectorConfig::default() };
    let mut det = ContentDetector::new(cfg, CompiledExitExpressions::empty());
    let observation = det.observe(&"spam\n".repeat(100));
    assert_eq!(observation, ContentObservation::default());
}
