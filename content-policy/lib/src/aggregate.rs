//! Combining entry results into a document status and effective action.

use serde::Serialize;

use crate::model::Action;

/// Overall document status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Every entry was evaluated and none triggered.
    Fresh,
    /// No entry triggered, and at least one could not be evaluated.
    Unknown,
    /// The highest confirmed action is `refresh`.
    Stale,
    /// The highest confirmed action is `archive` or `remove`.
    Expired,
}

/// The three-valued result of one entry, without its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    Triggered,
    NotTriggered,
    Unknown,
}

/// The document-level verdict over every entry result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aggregate {
    pub status: Status,
    /// The highest action among confirmed triggers; unknown entries never
    /// nominate one.
    pub action: Option<Action>,
    /// Every entry produced a confirmed result.
    pub evaluation_complete: bool,
    /// No unknown entry could change [`Aggregate::action`]. It says nothing
    /// about whether every entry was evaluated.
    pub action_resolution_complete: bool,
}

/// Aggregates `(action, result)` pairs. Order never matters.
#[must_use]
pub fn aggregate(results: impl IntoIterator<Item = (Action, ResultKind)>) -> Aggregate {
    let mut triggered: Option<Action> = None;
    let mut unknown: Option<Action> = None;
    for (action, kind) in results {
        match kind {
            ResultKind::Triggered => triggered = triggered.max(Some(action)),
            ResultKind::Unknown => unknown = unknown.max(Some(action)),
            ResultKind::NotTriggered => {}
        }
    }
    let status = match triggered {
        None if unknown.is_some() => Status::Unknown,
        None => Status::Fresh,
        Some(Action::Refresh) => Status::Stale,
        Some(Action::Archive | Action::Remove) => Status::Expired,
    };
    let action_resolution_complete = match (triggered, unknown) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(known), Some(unknown)) => unknown <= known,
    };
    Aggregate {
        status,
        action: triggered,
        evaluation_complete: unknown.is_none(),
        action_resolution_complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [ResultKind; 3] = [
        ResultKind::Triggered,
        ResultKind::NotTriggered,
        ResultKind::Unknown,
    ];

    /// The spec's two aggregation tables, transcribed row by row.
    fn oracle(results: &[(Action, ResultKind)]) -> Aggregate {
        let known = results
            .iter()
            .filter(|(_, kind)| *kind == ResultKind::Triggered)
            .map(|(action, _)| *action)
            .max();
        let unknowns: Vec<Action> = results
            .iter()
            .filter(|(_, kind)| *kind == ResultKind::Unknown)
            .map(|(action, _)| *action)
            .collect();
        // Status table.
        let status = match known {
            None if unknowns.is_empty() => Status::Fresh,
            None => Status::Unknown,
            Some(Action::Refresh) => Status::Stale,
            Some(_) => Status::Expired,
        };
        // Resolution table: one row per (known, unknown) pair; every unknown
        // entry must land on a "Complete" row.
        let row_complete = |unknown: Action| match (known, unknown) {
            (None, _) => false,
            (Some(Action::Refresh), Action::Refresh) => true,
            (Some(Action::Refresh), Action::Archive | Action::Remove) => false,
            (Some(Action::Archive), Action::Refresh | Action::Archive) => true,
            (Some(Action::Archive), Action::Remove) => false,
            (Some(Action::Remove), _) => true,
        };
        Aggregate {
            status,
            action: known,
            evaluation_complete: unknowns.is_empty(),
            action_resolution_complete: unknowns.into_iter().all(row_complete),
        }
    }

    fn combinations(len: usize) -> Vec<Vec<(Action, ResultKind)>> {
        let cells: Vec<(Action, ResultKind)> = Action::ALL
            .into_iter()
            .flat_map(|action| KINDS.map(|kind| (action, kind)))
            .collect();
        (0..len).fold(vec![Vec::new()], |acc, _| {
            acc.into_iter()
                .flat_map(|prefix| {
                    cells.iter().map(move |cell| {
                        let mut next = prefix.clone();
                        next.push(*cell);
                        next
                    })
                })
                .collect()
        })
    }

    fn permutations(items: &[(Action, ResultKind)]) -> Vec<Vec<(Action, ResultKind)>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }
        (0..items.len())
            .flat_map(|index| {
                let mut rest = items.to_vec();
                let head = rest.remove(index);
                permutations(&rest).into_iter().map(move |mut tail| {
                    tail.insert(0, head);
                    tail
                })
            })
            .collect()
    }

    #[test]
    fn every_combination_of_up_to_three_entries_follows_the_tables() {
        let mut checked = 0;
        for len in 1..=3 {
            for combination in combinations(len) {
                assert_eq!(
                    aggregate(combination.iter().copied()),
                    oracle(&combination),
                    "{combination:?}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 9 + 81 + 729);
    }

    #[test]
    fn entry_order_never_changes_the_result() {
        for combination in combinations(3) {
            let expected = aggregate(combination.iter().copied());
            for permutation in permutations(&combination) {
                assert_eq!(aggregate(permutation.iter().copied()), expected, "{permutation:?}");
            }
        }
    }

    #[test]
    fn named_rows() {
        let fresh = aggregate([(Action::Remove, ResultKind::NotTriggered)]);
        assert_eq!(fresh.status, Status::Fresh);
        assert!(fresh.evaluation_complete && fresh.action_resolution_complete);
        assert_eq!(fresh.action, None);

        let unknown = aggregate([(Action::Refresh, ResultKind::Unknown)]);
        assert_eq!((unknown.status, unknown.action), (Status::Unknown, None));
        assert!(!unknown.action_resolution_complete);

        let refresh_vs_unknown_remove = aggregate([
            (Action::Refresh, ResultKind::Triggered),
            (Action::Remove, ResultKind::Unknown),
        ]);
        assert_eq!(refresh_vs_unknown_remove.status, Status::Stale);
        assert_eq!(refresh_vs_unknown_remove.action, Some(Action::Refresh));
        assert!(!refresh_vs_unknown_remove.action_resolution_complete);

        let remove_vs_unknown_refresh = aggregate([
            (Action::Remove, ResultKind::Triggered),
            (Action::Refresh, ResultKind::Unknown),
        ]);
        assert_eq!(remove_vs_unknown_refresh.status, Status::Expired);
        assert!(remove_vs_unknown_refresh.action_resolution_complete);
        assert!(!remove_vs_unknown_refresh.evaluation_complete);
    }
}
