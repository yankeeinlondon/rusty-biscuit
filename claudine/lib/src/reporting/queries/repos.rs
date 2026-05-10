use std::collections::{BTreeMap, HashMap, HashSet};

use rusqlite::{Connection, params_from_iter};

use crate::error::Result;
use crate::reporting::types::{
    DateRange, LabeledCount, RepoActivity, ReportingFilters, ReposReport,
};

use super::common::{WhereBuilder, validate_range};

pub(crate) fn repos(
    conn: &Connection,
    range: DateRange,
    filters: &ReportingFilters,
) -> Result<ReposReport> {
    validate_range(range)?;

    let builder = WhereBuilder::default()
        .with_range(range)
        .with_filters(filters);
    let sql = builder.finish(
        r#"
        SELECT timestamp, repo_name, repo_org, branch, head_sha, is_dirty, session_key
        FROM events
        "#,
    ) + " AND repo_name IS NOT NULL ORDER BY repo_name ASC, timestamp ASC";

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(builder.params.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, String>(6)?,
        ))
    })?;

    #[derive(Default)]
    struct RepoAccumulator {
        repo_name: String,
        repo_org: Option<String>,
        event_count: u64,
        sessions: HashSet<String>,
        head_shas: HashSet<String>,
        dirty_transitions: u64,
        last_dirty: Option<bool>,
        branches: BTreeMap<String, u64>,
    }

    // Key repos by org/name composite to avoid conflating org-a/foo and org-b/foo.
    let mut repos: HashMap<String, RepoAccumulator> = HashMap::new();
    for row in rows {
        let (_, repo_name, repo_org, branch, head_sha, is_dirty, session_key) = row?;
        let repo_key = match &repo_org {
            Some(org) => format!("{org}/{repo_name}"),
            None => repo_name.clone(),
        };
        let entry = repos.entry(repo_key).or_default();
        entry.event_count += 1;
        entry.repo_name = repo_name;
        entry.repo_org = entry.repo_org.clone().or(repo_org);
        entry.sessions.insert(session_key);

        if let Some(head_sha) = head_sha {
            entry.head_shas.insert(head_sha);
        }

        if let Some(branch) = branch {
            *entry.branches.entry(branch).or_default() += 1;
        }

        if let Some(is_dirty) = is_dirty {
            let dirty = is_dirty != 0;
            if let Some(previous) = entry.last_dirty
                && previous != dirty
            {
                entry.dirty_transitions += 1;
            }
            entry.last_dirty = Some(dirty);
        }
    }

    let mut items = repos
        .into_values()
        .map(|accumulator| RepoActivity {
            repo_name: accumulator.repo_name,
            repo_org: accumulator.repo_org,
            event_count: accumulator.event_count,
            session_count: accumulator.sessions.len() as u64,
            head_sha_count: accumulator.head_shas.len() as u64,
            dirty_transitions: accumulator.dirty_transitions,
            branches: accumulator
                .branches
                .into_iter()
                .map(|(label, count)| LabeledCount { label, count })
                .collect(),
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .event_count
            .cmp(&left.event_count)
            .then_with(|| left.repo_name.cmp(&right.repo_name))
    });

    Ok(ReposReport {
        range,
        repos: items,
    })
}
