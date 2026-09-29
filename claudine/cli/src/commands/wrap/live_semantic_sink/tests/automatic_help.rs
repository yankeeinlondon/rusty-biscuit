//! Automatic repetition help driven through the live sink: an early warning
//! reaches the execution's controller before the repetition stop, and the
//! stop keeps its schedule whether help is on, off, or never gets a window.

use super::*;
use crate::steering::automatic::tests::{
    Answer, Recording, automatically_routable, collecting, controller, eventually,
};
use crate::steering::automatic::{AutomaticHelp, NoticeLevel};
use claudine::runaway::{CompiledExitExpressions, ContentDetector, DetectorConfig};
use claudine::steering::contract::{SendOutcome, SteeringOrigin};
use claudine::stream::logs::EarlyTermination;
use serde_json::json;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

fn sink() -> (LiveSemanticSink, Receiver<EarlyTermination>) {
    let dispatch = Box::new(|_event: AgenticEvent, _meta: DispatchEventMeta| {});
    let emit = Box::new(|_line: &str| {});
    let mut sink = LiveSemanticSink::new(
        Provider::Pi,
        EnvironmentContext::default(),
        Path::new("/tmp"),
        Verbosity::Normal,
        Arc::new(Mutex::new(StructuredSummaryDetails::default())),
        dispatch,
        emit,
    );
    sink.set_content_detector(Some(ContentDetector::new(DetectorConfig::default(), CompiledExitExpressions::empty())));
    let (tx, rx) = channel();
    sink.set_trip_sender(tx);
    (sink, rx)
}

fn output(text: &str) -> SemanticEvent {
    SemanticEvent::OutputText { text: text.to_string(), extra: json!({}) }
}

/// Feeds `spam` lines one event at a time and returns the 1-based line on
/// which the repetition stop fired.
fn line_of_stop(sink: &mut LiveSemanticSink, trips: &Receiver<EarlyTermination>, from: usize) -> usize {
    for line in from..=60 {
        sink.on_semantic_event(output("spam\n"));
        if let Ok(trip) = trips.try_recv() {
            assert!(matches!(trip, EarlyTermination::RunawayRepetition { cycle_len: 1, repeats: 30 }), "{trip:?}");
            return line;
        }
    }
    panic!("no repetition stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_warning_is_delivered_before_the_stop_and_the_stop_keeps_its_schedule() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::With(SendOutcome::Queued));
    let (notice, notices) = collecting();
    let (mut sink, trips) = sink();
    sink.set_automatic_help(AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice));

    sink.on_semantic_event(output(&"spam\n".repeat(20)));
    assert!(trips.try_recv().is_err(), "20 repeats are below the stop");
    eventually("the automatic delivery", || recording.count() == 1).await;
    assert_eq!(recording.delivered.lock().unwrap()[0].origin, SteeringOrigin::Automatic);
    eventually("the sent notice", || !notices.lock().unwrap().is_empty()).await;
    assert_eq!(notices.lock().unwrap()[0].0, NoticeLevel::Sent);

    // Acknowledged help is not recovery: the same repetition still stops at 30.
    assert_eq!(line_of_stop(&mut sink, &trips, 21), 30);
    assert_eq!(recording.count(), 1, "one episode, one warning");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_chunk_that_crosses_warning_and_stop_only_stops() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::With(SendOutcome::Queued));
    let (notice, notices) = collecting();
    let (mut sink, trips) = sink();
    sink.set_automatic_help(AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice));

    sink.on_semantic_event(output(&"spam\n".repeat(30)));
    assert!(matches!(trips.try_recv(), Ok(EarlyTermination::RunawayRepetition { .. })));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(recording.count(), 0, "no warning is enqueued once the run is being stopped");
    assert!(notices.lock().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stop_during_delivery_abandons_the_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::Never);
    let (notice, notices) = collecting();
    let (mut sink, trips) = sink();
    sink.set_automatic_help(AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice));

    sink.on_semantic_event(output(&"spam\n".repeat(15)));
    eventually("the warning in flight", || recording.count() == 1).await;
    assert_eq!(line_of_stop(&mut sink, &trips, 16), 30, "a slow send never delays the stop");
    eventually("the abandoned send", || recording.abandoned.load(std::sync::atomic::Ordering::SeqCst) == 1).await;
    assert!(notices.lock().unwrap().is_empty(), "nothing is reported after the stop");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_help_warns_once_and_the_stop_keeps_its_schedule() {
    let (notice, notices) = collecting();
    let (mut sink, trips) = sink();
    sink.set_automatic_help(AutomaticHelp::new(None, notice));
    assert_eq!(line_of_stop(&mut sink, &trips, 1), 30);
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].0, NoticeLevel::Warning);
}

#[test]
fn with_help_off_nothing_is_sent_or_noticed_and_the_stop_keeps_its_schedule() {
    let (mut sink, trips) = sink();
    assert_eq!(line_of_stop(&mut sink, &trips, 1), 30);
}
