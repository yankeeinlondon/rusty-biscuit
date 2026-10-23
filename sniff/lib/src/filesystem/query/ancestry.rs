//! A process's verified parent chain, built from per-PID reads.
//!
//! No process table is listed or refreshed: each step reads one PID. A
//! parent's lifetime never starts after its child's, so a later lifetime
//! found at a parent's PID (the parent exited and the PID was reused) ends the
//! chain instead of lending its identity.

use super::backend::{LinkError, ProcessLink};
use super::budget::Budget;
use super::report::{
    Ancestor, AncestryEnd, LimitationExample, LimitationKind, Limitations, MAX_ANCESTORS,
};
use crate::performance::{self, counters};

/// PID 0 (kernel or idle) and 1 (init or launchd) root every chain, so they
/// say nothing about who started a process.
fn is_root(pid: u32) -> bool {
    pid <= 1
}

/// Reads one link, counting the reads of a backend that supports them.
pub(crate) fn read(
    reader: &mut impl FnMut(u32) -> Result<ProcessLink, LinkError>,
    pid: u32,
) -> Result<ProcessLink, LinkError> {
    let link = reader(pid);
    if link != Err(LinkError::Unsupported) {
        performance::increment_counter(counters::QUERY_PARENT_LOOKUPS, 1);
    }
    link
}

/// The verified ancestors of `child`, whose own link is `own`, nearest first.
pub(crate) fn chain(
    child: u32,
    own: &ProcessLink,
    budget: &Budget,
    limitations: &mut Limitations,
    reader: &mut impl FnMut(u32) -> Result<ProcessLink, LinkError>,
) -> (Vec<Ancestor>, AncestryEnd) {
    let mut ancestors: Vec<Ancestor> = Vec::new();
    let mut next = own.parent_pid;
    let mut child_order = own.order;
    loop {
        if is_root(next) {
            return (ancestors, AncestryEnd::Root);
        }
        if ancestors.len() == MAX_ANCESTORS {
            return (ancestors, AncestryEnd::Bound);
        }
        // Only a reused PID can close a cycle.
        if next == child || ancestors.iter().any(|a| a.pid == next) {
            return (ancestors, AncestryEnd::ParentExited);
        }
        if budget.expired() {
            limitations.record(
                LimitationKind::BudgetExhausted,
                "the budget expired before a process's parent chain was read",
                1,
                Some(LimitationExample::pid(child)),
            );
            return (ancestors, AncestryEnd::NotRead);
        }
        let link = match read(reader, next) {
            Ok(link) => link,
            Err(LinkError::Gone) => return (ancestors, AncestryEnd::ParentExited),
            Err(LinkError::Unsupported) => return (ancestors, AncestryEnd::Unsupported),
            Err(LinkError::Unreadable(detail)) => {
                limitations.record(
                    LimitationKind::ContextUnavailable,
                    "a parent process could not be read, so its chain stops there",
                    1,
                    Some(LimitationExample::pid(next).with_detail(detail)),
                );
                return (ancestors, AncestryEnd::Unreadable);
            }
        };
        if link.order > child_order {
            return (ancestors, AncestryEnd::ParentExited);
        }
        ancestors.push(Ancestor {
            pid: next,
            name: link.name.map(Into::into),
            app_name: link.app_name.map(Into::into),
        });
        child_order = link.order;
        next = link.parent_pid;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::testing::measure;
    use std::collections::HashMap;
    use std::time::Duration;

    const CHILD: u32 = 500;

    fn link(order: u64, parent_pid: u32, name: &str) -> ProcessLink {
        ProcessLink {
            order,
            parent_pid,
            name: Some(name.into()),
            app_name: None,
        }
    }

    struct Run {
        ancestors: Vec<Ancestor>,
        end: AncestryEnd,
        limitations: Vec<super::super::report::Limitation>,
        reads: Vec<u32>,
    }

    fn run(own: ProcessLink, links: HashMap<u32, Result<ProcessLink, LinkError>>, budget: &Budget) -> Run {
        let mut limitations = Limitations::default();
        let mut reads = Vec::new();
        let mut reader = |pid: u32| {
            reads.push(pid);
            links.get(&pid).cloned().unwrap_or(Err(LinkError::Gone))
        };
        let (ancestors, end) = chain(CHILD, &own, budget, &mut limitations, &mut reader);
        Run {
            ancestors,
            end,
            limitations: limitations.into_vec(),
            reads,
        }
    }

    fn budget() -> Budget {
        Budget::start(Duration::from_secs(60)).unwrap()
    }

    fn pids(run: &Run) -> Vec<u32> {
        run.ancestors.iter().map(|a| a.pid).collect()
    }

    #[test]
    fn a_chain_lists_verified_parents_nearest_first_and_stops_before_pid_1() {
        let links = HashMap::from([
            (400, Ok(link(40, 300, "zsh"))),
            (300, Ok(ProcessLink { app_name: Some("Terminal".into()), ..link(30, 1, "Terminal") })),
        ]);
        let run = run(link(50, 400, "cargo"), links, &budget());
        assert_eq!(pids(&run), vec![400, 300]);
        assert_eq!(run.ancestors[0].name.as_ref().unwrap().display(), "zsh");
        assert_eq!(run.ancestors[1].app_name.as_ref().unwrap().display(), "Terminal");
        assert_eq!(run.end, AncestryEnd::Root);
        assert_eq!(run.reads, vec![400, 300], "PID 1 is never read");
        assert!(run.limitations.is_empty());
    }

    #[test]
    fn a_process_whose_parent_is_pid_0_has_an_empty_rooted_chain() {
        let run = run(link(50, 0, "launchd"), HashMap::new(), &budget());
        assert!(run.ancestors.is_empty());
        assert_eq!(run.end, AncestryEnd::Root);
        assert!(run.reads.is_empty());
    }

    #[test]
    fn the_chain_is_bounded_without_reading_past_the_bound() {
        let deep = MAX_ANCESTORS as u32 + 4;
        let links = (0..deep)
            .map(|step| {
                let pid = 1_000 + step;
                (pid, Ok(link(100 - u64::from(step), pid + 1, "sh")))
            })
            .collect();
        let (run, counts) = measure(|| run(link(200, 1_000, "cargo"), links, &budget()));
        assert_eq!(run.ancestors.len(), MAX_ANCESTORS);
        assert_eq!(run.end, AncestryEnd::Bound);
        assert_eq!(run.reads.len(), MAX_ANCESTORS);
        assert_eq!(counts.get(counters::QUERY_PARENT_LOOKUPS), MAX_ANCESTORS as u64);
    }

    #[test]
    fn a_parent_that_started_after_its_child_is_a_reused_pid_and_ends_the_chain() {
        let links = HashMap::from([
            (400, Ok(link(40, 300, "zsh"))),
            // PID 300 now names a process that started after PID 400.
            (300, Ok(link(41, 1, "imposter"))),
        ]);
        let run = run(link(50, 400, "cargo"), links, &budget());
        assert_eq!(pids(&run), vec![400]);
        assert_eq!(run.end, AncestryEnd::ParentExited);
        assert!(run.limitations.is_empty(), "an exited parent is not a gap");
    }

    #[test]
    fn a_parent_that_started_in_the_same_tick_is_still_the_parent() {
        let links = HashMap::from([(400, Ok(link(50, 1, "zsh")))]);
        let run = run(link(50, 400, "cargo"), links, &budget());
        assert_eq!(pids(&run), vec![400]);
        assert_eq!(run.end, AncestryEnd::Root);
    }

    #[test]
    fn a_cycle_ends_the_chain_instead_of_looping() {
        let links = HashMap::from([
            (400, Ok(link(40, 300, "a"))),
            (300, Ok(link(30, 400, "b"))),
        ]);
        let run = run(link(50, 400, "cargo"), links, &budget());
        assert_eq!(pids(&run), vec![400, 300]);
        assert_eq!(run.end, AncestryEnd::ParentExited);
    }

    #[test]
    fn a_parent_naming_the_child_ends_the_chain() {
        let run = run(link(50, CHILD, "cargo"), HashMap::new(), &budget());
        assert!(run.ancestors.is_empty());
        assert_eq!(run.end, AncestryEnd::ParentExited);
        assert!(run.reads.is_empty());
    }

    #[test]
    fn a_gone_parent_has_exited() {
        let run = run(link(50, 400, "cargo"), HashMap::new(), &budget());
        assert!(run.ancestors.is_empty());
        assert_eq!(run.end, AncestryEnd::ParentExited);
    }

    #[test]
    fn an_unreadable_ancestor_stops_the_chain_with_a_limitation() {
        let links = HashMap::from([
            (400, Ok(link(40, 300, "zsh"))),
            (300, Err(LinkError::Unreadable("denied".into()))),
        ]);
        let run = run(link(50, 400, "cargo"), links, &budget());
        assert_eq!(pids(&run), vec![400]);
        assert_eq!(run.end, AncestryEnd::Unreadable);
        assert_eq!(run.limitations.len(), 1);
        let limitation = &run.limitations[0];
        assert_eq!(limitation.kind, LimitationKind::ContextUnavailable);
        assert_eq!(limitation.examples[0].pid, Some(300));
        assert_eq!(limitation.examples[0].detail.as_deref(), Some("denied"));
    }

    #[test]
    fn an_expired_budget_reads_nothing_and_says_so() {
        let budget = Budget::start(Duration::from_secs(60)).unwrap();
        budget.expire();
        let links = HashMap::from([(400, Ok(link(40, 1, "zsh")))]);
        let run = run(link(50, 400, "cargo"), links, &budget);
        assert!(run.reads.is_empty());
        assert_eq!(run.end, AncestryEnd::NotRead);
        assert_eq!(run.limitations[0].kind, LimitationKind::BudgetExhausted);
    }

    #[test]
    fn an_unsupported_backend_counts_no_lookup() {
        let mut reader = |_pid: u32| Err(LinkError::Unsupported);
        let (result, counts) = measure(|| read(&mut reader, 400));
        assert_eq!(result, Err(LinkError::Unsupported));
        assert_eq!(counts.get(counters::QUERY_PARENT_LOOKUPS), 0);
    }
}
