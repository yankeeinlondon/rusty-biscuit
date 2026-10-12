//! GitHub check status for open PRs: the GraphQL batch query, the strict
//! response reader, and the rule that maps a commit's rollup counts onto a
//! [`PrCheckStatus`].
//!
//! Everything here is pure; the request itself goes through
//! [`FocusedProviderClient`](super::FocusedProviderClient).

use std::fmt;

use serde::de::{Deserialize, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

use super::blocking::PrCheckStatus;

/// Most PR lookups one GraphQL request carries.
pub(crate) const BATCH_SIZE: usize = 100;

/// Check-run states that need attention.
const CHECK_RUN_FAILING: [&str; 6] = [
    "FAILURE",
    "TIMED_OUT",
    "CANCELLED",
    "ACTION_REQUIRED",
    "STARTUP_FAILURE",
    "STALE",
];
const CHECK_RUN_RUNNING: [&str; 4] = ["QUEUED", "IN_PROGRESS", "PENDING", "WAITING"];
const CHECK_RUN_ACCEPTABLE: [&str; 3] = ["SUCCESS", "SKIPPED", "NEUTRAL"];
const STATUS_FAILING: [&str; 2] = ["ERROR", "FAILURE"];
const STATUS_RUNNING: [&str; 2] = ["EXPECTED", "PENDING"];
const STATUS_ACCEPTABLE: [&str; 1] = ["SUCCESS"];

/// The query for one batch of `count` PR lookups.
///
/// Repository identity and PR numbers are variables (`$owner`, `$name`,
/// `$n0`…), so no provider text is ever part of the query string.
pub(crate) fn batch_query(count: usize) -> String {
    let variables: String = (0..count).map(|index| format!(",$n{index}:Int!")).collect();
    let lookups: String = (0..count)
        .map(|index| format!("p{index}:pullRequest(number:$n{index}){{...F}}"))
        .collect();
    format!(
        "query($owner:String!,$name:String!{variables}){{repository(owner:$owner,name:$name){{{lookups}}}}}\
         fragment F on PullRequest{{state commits(last:1){{nodes{{commit{{oid statusCheckRollup{{\
         contexts(first:1){{checkRunCount statusContextCount \
         checkRunCountsByState{{state count}} statusContextCountsByState{{state count}}}}}}}}}}}}}}"
    )
}

/// The variables for a batch of lookups.
pub(crate) fn batch_variables(owner: &str, name: &str, numbers: &[u64]) -> Value {
    let mut variables = Map::new();
    variables.insert("owner".to_string(), Value::from(owner));
    variables.insert("name".to_string(), Value::from(name));
    for (index, number) in numbers.iter().enumerate() {
        variables.insert(format!("n{index}"), Value::from(*number));
    }
    Value::Object(variables)
}

/// Why a whole batch yielded no usable answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RequestWide(pub(crate) &'static str);

/// Reads one batch response into a status per lookup, in lookup order.
///
/// `expected_heads[i]` is the REST head SHA of lookup `i`; a PR whose latest
/// commit differs is unknown. A record the response lacks, or one that is
/// not a usable open PR, is unknown for that PR alone.
///
/// ## Errors
///
/// [`RequestWide`] when the body is not one JSON document with distinct keys,
/// names a rate limit, or carries no repository object: nothing in it can be
/// attributed to individual PRs.
pub(crate) fn read_batch(
    body: &[u8],
    expected_heads: &[&str],
) -> Result<Vec<PrCheckStatus>, RequestWide> {
    let Strict(document) =
        serde_json::from_slice(body).map_err(|_| RequestWide("malformed GraphQL response"))?;
    let rate_limited = document
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| {
            errors.iter().any(|error| {
                error.get("type").and_then(Value::as_str) == Some("RATE_LIMITED")
            })
        });
    if rate_limited {
        return Err(RequestWide("GraphQL rate limit"));
    }
    let Some(repository) = document
        .get("data")
        .and_then(|data| data.get("repository"))
        .and_then(Value::as_object)
    else {
        return Err(RequestWide("GraphQL response has no repository"));
    };
    Ok(expected_heads
        .iter()
        .enumerate()
        .map(|(index, head)| {
            repository
                .get(&format!("p{index}"))
                .map_or(PrCheckStatus::Unknown, |record| record_status(record, head))
        })
        .collect())
}

fn record_status(record: &Value, expected_head: &str) -> PrCheckStatus {
    if record.get("state").and_then(Value::as_str) != Some("OPEN") {
        return PrCheckStatus::Unknown;
    }
    let latest = record
        .get("commits")
        .and_then(|commits| commits.get("nodes"))
        .and_then(Value::as_array)
        .and_then(|nodes| nodes.last())
        .and_then(|node| node.get("commit"));
    let Some(commit) = latest else {
        return PrCheckStatus::Unknown;
    };
    let same_head = commit
        .get("oid")
        .and_then(Value::as_str)
        .is_some_and(|oid| oid.eq_ignore_ascii_case(expected_head));
    match commit.get("statusCheckRollup") {
        Some(rollup) if same_head => status_from_rollup(rollup),
        _ => PrCheckStatus::Unknown,
    }
}

/// Maps one `statusCheckRollup` object onto a status.
///
/// A failing count wins over everything, then a running count. Passed needs
/// a nonzero total whose per-state counts add up to the declared totals and
/// name only acceptable states. Anything else, including a null rollup, a
/// state this code does not know, `COMPLETED` (a check run without its
/// conclusion), and missing or malformed counts, is unknown: unusable
/// evidence never becomes a failure.
pub(crate) fn status_from_rollup(rollup: &Value) -> PrCheckStatus {
    let Some(contexts) = rollup.get("contexts").filter(|value| value.is_object()) else {
        return PrCheckStatus::Unknown;
    };
    let runs = Tally::read(
        contexts,
        ("checkRunCount", "checkRunCountsByState"),
        [&CHECK_RUN_FAILING, &CHECK_RUN_RUNNING, &CHECK_RUN_ACCEPTABLE],
    );
    let statuses = Tally::read(
        contexts,
        ("statusContextCount", "statusContextCountsByState"),
        [&STATUS_FAILING, &STATUS_RUNNING, &STATUS_ACCEPTABLE],
    );
    if runs.failing + statuses.failing > 0 {
        PrCheckStatus::Failing
    } else if runs.running + statuses.running > 0 {
        PrCheckStatus::Running
    } else if runs.sound && statuses.sound && runs.total() + statuses.total() > 0 {
        PrCheckStatus::Passed
    } else {
        PrCheckStatus::Unknown
    }
}

/// The counts of one kind of context (check runs or commit statuses).
struct Tally {
    failing: u64,
    running: u64,
    acceptable: u64,
    /// Every entry was well formed and named a known state, and the entries
    /// add up to the declared total.
    sound: bool,
}

impl Tally {
    fn total(&self) -> u64 {
        self.failing + self.running + self.acceptable
    }

    fn read(contexts: &Value, (total_key, counts_key): (&str, &str), states: [&[&str]; 3]) -> Self {
        let mut tally = Self {
            failing: 0,
            running: 0,
            acceptable: 0,
            sound: true,
        };
        let declared = contexts.get(total_key).and_then(Value::as_u64);
        let Some(entries) = contexts.get(counts_key).and_then(Value::as_array) else {
            tally.sound = false;
            return tally;
        };
        let mut sum: u64 = 0;
        for entry in entries {
            let parsed = entry
                .get("state")
                .and_then(Value::as_str)
                .zip(entry.get("count").and_then(Value::as_u64));
            let Some((state, count)) = parsed else {
                tally.sound = false;
                continue;
            };
            sum = sum.saturating_add(count);
            let bucket = if states[0].contains(&state) {
                &mut tally.failing
            } else if states[1].contains(&state) {
                &mut tally.running
            } else if states[2].contains(&state) {
                &mut tally.acceptable
            } else {
                tally.sound = false;
                continue;
            };
            *bucket = bucket.saturating_add(count);
        }
        if declared != Some(sum) {
            tally.sound = false;
        }
        tally
    }
}

/// A JSON document that rejects a repeated object key at any depth.
///
/// A plain `serde_json::Value` keeps the last of two repeated keys.
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;

        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = Strict;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("any JSON value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Strict, E> {
                Ok(Strict(Value::Bool(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Strict, E> {
                Ok(Strict(Value::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Strict, E> {
                Ok(Strict(Value::from(value)))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Strict, E> {
                Ok(Strict(Value::from(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Strict, E> {
                Ok(Strict(Value::String(value.to_string())))
            }

            fn visit_unit<E>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Strict, A::Error> {
                let mut items = Vec::new();
                while let Some(Strict(item)) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Strict(Value::Array(items)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Strict, A::Error> {
                let mut object = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    let Strict(value) = map.next_value()?;
                    if object.insert(key, value).is_some() {
                        return Err(A::Error::custom("duplicate key"));
                    }
                }
                Ok(Strict(Value::Object(object)))
            }
        }

        deserializer.deserialize_any(StrictVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rollup(runs: &[(&str, u64)], statuses: &[(&str, u64)]) -> Value {
        let counts = |entries: &[(&str, u64)]| -> Vec<Value> {
            entries
                .iter()
                .map(|(state, count)| json!({"state": state, "count": count}))
                .collect()
        };
        json!({"contexts": {
            "checkRunCount": runs.iter().map(|(_, count)| count).sum::<u64>(),
            "statusContextCount": statuses.iter().map(|(_, count)| count).sum::<u64>(),
            "checkRunCountsByState": counts(runs),
            "statusContextCountsByState": counts(statuses),
        }})
    }

    #[test]
    fn every_check_run_state_maps_per_the_evidence_table() {
        for state in CHECK_RUN_FAILING {
            assert_eq!(status_from_rollup(&rollup(&[("SUCCESS", 3), (state, 1)], &[])), PrCheckStatus::Failing, "{state}");
        }
        for state in CHECK_RUN_RUNNING {
            assert_eq!(status_from_rollup(&rollup(&[("SUCCESS", 3), (state, 1)], &[])), PrCheckStatus::Running, "{state}");
        }
        for state in CHECK_RUN_ACCEPTABLE {
            assert_eq!(status_from_rollup(&rollup(&[(state, 2)], &[])), PrCheckStatus::Passed, "{state}");
        }
    }

    #[test]
    fn every_commit_status_state_maps_per_the_evidence_table() {
        for state in STATUS_FAILING {
            assert_eq!(status_from_rollup(&rollup(&[], &[("SUCCESS", 1), (state, 1)])), PrCheckStatus::Failing, "{state}");
        }
        for state in STATUS_RUNNING {
            assert_eq!(status_from_rollup(&rollup(&[], &[("SUCCESS", 1), (state, 1)])), PrCheckStatus::Running, "{state}");
        }
        assert_eq!(status_from_rollup(&rollup(&[], &[("SUCCESS", 1)])), PrCheckStatus::Passed);
    }

    #[test]
    fn failure_wins_over_running_and_a_zero_count_contributes_nothing() {
        assert_eq!(
            status_from_rollup(&rollup(&[("IN_PROGRESS", 1), ("FAILURE", 1)], &[])),
            PrCheckStatus::Failing
        );
        assert_eq!(
            status_from_rollup(&rollup(&[("FAILURE", 0), ("SUCCESS", 1)], &[])),
            PrCheckStatus::Passed
        );
    }

    #[test]
    fn unrecognized_and_ambiguous_states_prevent_green_but_never_invent_a_failure() {
        for state in ["COMPLETED", "NEW_STATE_FROM_GITHUB"] {
            assert_eq!(status_from_rollup(&rollup(&[("SUCCESS", 1), (state, 1)], &[])), PrCheckStatus::Unknown, "{state}");
        }
        assert_eq!(status_from_rollup(&rollup(&[("IN_PROGRESS", 1), ("COMPLETED", 1)], &[])), PrCheckStatus::Running);
    }

    #[test]
    fn missing_null_zero_or_inconsistent_evidence_is_unknown() {
        assert_eq!(status_from_rollup(&Value::Null), PrCheckStatus::Unknown);
        assert_eq!(status_from_rollup(&json!({})), PrCheckStatus::Unknown);
        assert_eq!(status_from_rollup(&json!({"contexts": null})), PrCheckStatus::Unknown);
        assert_eq!(status_from_rollup(&rollup(&[], &[])), PrCheckStatus::Unknown);
        let mut mismatch = rollup(&[("SUCCESS", 1)], &[]);
        mismatch["contexts"]["checkRunCount"] = json!(2);
        assert_eq!(status_from_rollup(&mismatch), PrCheckStatus::Unknown);
        let mut negative = rollup(&[("SUCCESS", 1)], &[]);
        negative["contexts"]["checkRunCountsByState"][0]["count"] = json!(-1);
        assert_eq!(status_from_rollup(&negative), PrCheckStatus::Unknown);
    }

    #[test]
    fn the_query_names_only_generated_aliases_and_variables() {
        let query = batch_query(2);
        assert!(query.contains("p0:pullRequest(number:$n0)"));
        assert!(query.contains("p1:pullRequest(number:$n1)"));
        assert!(query.contains("$owner:String!,$name:String!,$n0:Int!,$n1:Int!"));
        assert_eq!(
            batch_variables("acme", "project", &[7, 9]),
            json!({"owner": "acme", "name": "project", "n0": 7, "n1": 9})
        );
    }
}
