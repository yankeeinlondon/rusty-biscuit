use super::*;
use std::thread;

#[test]
fn a_thread_with_no_scope_is_never_closed() {
    assert!(!current_closed());
    publish(|snapshot| snapshot.turn_complete = Some(TurnResult::default()));
}

#[test]
fn closing_a_scope_closes_it_for_the_thread_that_entered_it() {
    let scope = RunScope::default();
    let reader_scope = scope.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (closed_tx, closed_rx) = std::sync::mpsc::channel();
    let reader = thread::spawn(move || {
        let _guard = reader_scope.enter();
        assert!(!current_closed());
        entered_tx.send(()).unwrap();
        closed_rx.recv().unwrap();
        current_closed()
    });

    entered_rx.recv().unwrap();
    scope.close();
    closed_tx.send(()).unwrap();

    assert!(reader.join().unwrap(), "the reader sees its run is over");
    assert!(!current_closed(), "the closing thread never entered the scope");
}

#[test]
fn leaving_the_scope_stops_publishing_into_it() {
    let scope = RunScope::default();
    {
        let _guard = scope.enter();
        publish(|snapshot| {
            snapshot.terminal_error = Some(("api_remote".into(), "overloaded".into()));
        });
    }
    publish(|snapshot| snapshot.turn_complete = Some(TurnResult::default()));

    let snapshot = scope.snapshot();
    assert_eq!(
        snapshot.terminal_error,
        Some(("api_remote".to_string(), "overloaded".to_string()))
    );
    assert_eq!(snapshot.turn_complete, None);
}
