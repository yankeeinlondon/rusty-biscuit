//! A group leg's private live-frontmatter cell is a view over the group-start
//! state: building one never copies the document that holds the group.

use super::*;
use crate::work_counters::{Work, snapshot};

/// A one-step sequence whose step is a capped group of `legs` shell legs.
fn group_of(legs: usize) -> Fixture {
    let dir = TempDir::new().unwrap();
    let tasks: Vec<Value> = (0..legs)
        .map(|index| json!({ "name": format!("leg-{index}"), "shell": format!("echo {index}") }))
        .collect();
    let step = json!({ "name": "fan-out", "group": { "name": "legs", "max_parallel": 4, "tasks": tasks } });
    let source = write_source(dir.path(), "seq.md", &[("title", json!("T")), ("sequence", json!([step]))], "Body.\n");
    Fixture::build(dir, &source).unwrap()
}

/// The spec's group criterion: per-leg view construction does not grow with
/// the leg count, and every leg still gets its own cell and runs.
#[test]
fn leg_views_copy_the_same_per_leg_at_10_and_100_legs() {
    let mut copied_per_leg = Vec::new();
    for legs in [10usize, 100] {
        let fixture = group_of(legs);
        let recorder = Recorder::default();
        let fake = FakeTaskShell::default();
        let before = snapshot();
        let outcome = fixture.execute(&Wiring::new(&recorder, &fake));
        let work = snapshot().since(&before);
        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let names: Vec<&str> = outcome.group_tasks.iter().map(|leg| leg.name.as_str()).collect();
        assert_eq!(names, (0..legs).map(|index| format!("leg-{index}")).collect::<Vec<_>>());
        assert_eq!(fake.seen.lock().unwrap().len(), legs, "every leg ran its command");
        assert_eq!(work.get(Work::LiveFrontmatterView), legs as u64, "one private cell per leg");
        let copied = work.get(Work::LiveFrontmatterCopy);
        assert_eq!(copied % legs as u64, 0, "{copied} nodes over {legs} legs");
        copied_per_leg.push(copied / legs as u64);
    }
    assert_eq!(copied_per_leg[0], copied_per_leg[1], "nodes copied per leg at 10 and 100 legs");
}
