//! Git discovery for the graph: how a branch was integrated, and a lane's
//! first-parent history with its essential commits kept.
//!
//! Every answer is either verified or a [`GatherGap`]. Git's "no" (exit 1) is
//! trusted only in a complete clone: across a shallow boundary it looks the
//! same as a real "no", so there it is a gap.

use std::collections::{BTreeMap, HashMap, HashSet};

use biscuit_terminal::components::git_graph::LaneEntry;
use worktree::git::{git_command, git_command_allow_no_match};

/// History local Git could not establish. The caller draws what it verified
/// and reports the rest as incomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GatherGap;

/// How a branch tip `T` relates to the first candidate lane containing it.
/// `candidate` indexes the candidates passed to [`History::classify`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Integration {
    /// No candidate contains `T`.
    Unmerged,
    /// `merge` brought `T` in as a non-first parent; `first_parent` is the
    /// candidate lane's commit before it.
    MergedDirectly { candidate: usize, merge: String, first_parent: String },
    /// `T` is on the candidate's first-parent chain: the branch has no
    /// history of its own to draw.
    NoSeparateHistory { candidate: usize },
    /// `T` reached the candidate through another branch's merge, so no merge
    /// of its own can be drawn. `first_parent` is the candidate lane's commit
    /// before the first commit containing `T`.
    IntegratedOtherwise { candidate: usize, first_parent: String },
}

/// Where a lane's history ends.
#[derive(Debug, Clone, Copy)]
pub(super) enum Extent<'a> {
    /// A branch lane: its tip's first-parent chain until history reachable
    /// from these commits. Older commits fold into a leading `+N`.
    Until(&'a [&'a str]),
    /// The default lane, which runs to the root. It is drawn down to its
    /// oldest anchor plus that anchor's first parent (so a merge is never the
    /// lane's first commit). `cap_window`: the window, too, stops there.
    Open { cap_window: bool },
}

/// A lane's entries, oldest first.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct LaneHistory {
    pub entries: Vec<LaneEntry>,
    /// The tip commit's time, when the lane has commits.
    pub last_active: Option<i64>,
    /// The anchors drawn on this lane; the others are not on it.
    pub placed: HashSet<String>,
    /// Something the lane shows is unverified: a `+N` count in a shallow
    /// clone (a lower bound), or an anchor distance Git could not give.
    pub gap: bool,
}

/// One gathering pass's view of the repository's history.
#[derive(Debug, Clone, Copy)]
pub(super) struct History {
    shallow: bool,
}

impl History {
    /// Reads whether the repository is shallow. A failed read is a gap, and
    /// the history is then treated as shallow.
    pub fn read() -> (Self, Result<(), GatherGap>) {
        match git_command(&["rev-parse", "--is-shallow-repository"]).as_deref().map(str::trim) {
            Ok("false") => (Self { shallow: false }, Ok(())),
            Ok("true") => (Self { shallow: true }, Ok(())),
            _ => (Self { shallow: true }, Err(GatherGap)),
        }
    }

    /// History assumed complete, for gathering that makes no graph.
    pub fn complete() -> Self {
        Self { shallow: false }
    }

    /// Whether `ancestor` is reachable from `descendant` (or is it).
    pub fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, GatherGap> {
        match git_command_allow_no_match(&["merge-base", "--is-ancestor", ancestor, descendant]) {
            Ok(Some(_)) => Ok(true),
            Ok(None) if !self.shallow => Ok(false),
            _ => Err(GatherGap),
        }
    }

    /// The best common ancestor; `Ok(None)` for unrelated histories.
    pub fn merge_base(&self, a: &str, b: &str) -> Result<Option<String>, GatherGap> {
        match git_command_allow_no_match(&["merge-base", a, b]) {
            Ok(Some(sha)) if is_object_id(&sha) => Ok(Some(sha)),
            Ok(None) if !self.shallow => Ok(None),
            _ => Err(GatherGap),
        }
    }

    /// How `tip` was integrated into the first of `candidates` (lane tips, in
    /// priority order) that contains it.
    ///
    /// `C` is the oldest commit on the candidate's first-parent chain that
    /// descends from `tip`: the leading run of `rev-list --first-parent
    /// --parents T..X` that is also in `rev-list --ancestry-path T..X`.
    /// Containment is monotone along a first-parent chain, so the run is
    /// exactly the chain's commits containing `tip`.
    pub fn classify(&self, tip: &str, candidates: &[&str]) -> Result<Integration, GatherGap> {
        for (candidate, lane_tip) in candidates.iter().enumerate() {
            if !self.is_ancestor(tip, lane_tip)? {
                continue;
            }
            let range = format!("{tip}..{lane_tip}");
            let chain = parse_parent_lines(&git_command(&["rev-list", "--first-parent", "--parents", &range, "--"]).map_err(|_| GatherGap)?)?;
            if chain.is_empty() {
                // `T..X` is empty only when `T` is `X`; a shallow walk can
                // also come back empty, so there it proves nothing.
                if self.shallow {
                    return Err(GatherGap);
                }
                return Ok(Integration::NoSeparateHistory { candidate });
            }
            let descendants: HashSet<String> = parse_object_ids(
                &git_command(&["rev-list", "--ancestry-path", &range, "--"]).map_err(|_| GatherGap)?,
            )?
            .into_iter()
            .collect();
            let (merge, parents) = chain
                .iter()
                .take_while(|(sha, _)| descendants.contains(sha))
                .last()
                .ok_or(GatherGap)?;
            let first_parent = parents.first().ok_or(GatherGap)?;
            return Ok(if first_parent == tip {
                Integration::NoSeparateHistory { candidate }
            } else if parents[1..].iter().any(|parent| parent == tip) {
                Integration::MergedDirectly {
                    candidate,
                    merge: merge.clone(),
                    first_parent: first_parent.clone(),
                }
            } else {
                Integration::IntegratedOtherwise {
                    candidate,
                    first_parent: first_parent.clone(),
                }
            });
        }
        if self.shallow {
            // Unreachable in practice (`is_ancestor` already refused), kept
            // so a shallow "no" can never read as unmerged.
            return Err(GatherGap);
        }
        Ok(Integration::Unmerged)
    }

    /// The first-parent chain of `tip` within `extent`: the newest `window`
    /// commits, every anchor on the chain at its real position, and `+N`
    /// squares for the runs between them.
    ///
    /// An anchor's position is `rev-list --first-parent --count A..tip`,
    /// verified by one batched lookup of `tip~<position>`; an anchor that fails
    /// is not on this lane and is left out of [`LaneHistory::placed`]. `Err` is
    /// only for a lane whose newest commits cannot be read.
    pub fn first_parent_entries(&self, tip: &str, extent: Extent, window: usize, anchors: &[&str]) -> Result<LaneHistory, GatherGap> {
        let stop = match extent {
            Extent::Until(stop) => stop,
            Extent::Open { .. } => &[][..],
        };
        let mut shown = self.newest(tip, stop, window)?;
        let mut gap = false;
        let length = match extent {
            Extent::Until(_) if shown.len() < window => Some(shown.len()),
            Extent::Until(_) => match count_first_parent(tip, stop) {
                Some(count) => Some(count.max(shown.len())),
                None => {
                    gap = true;
                    Some(shown.len())
                }
            },
            Extent::Open { .. } => None,
        };

        let mut anchors: Vec<&str> = anchors.iter().copied().collect::<HashSet<_>>().into_iter().collect();
        anchors.sort_unstable();
        let mut placed: HashMap<String, usize> = anchors
            .iter()
            .filter_map(|anchor| {
                let position = shown.iter().position(|commit| commit.sha == *anchor)?;
                Some((anchor.to_string(), position))
            })
            .collect();
        // A branch lane shorter than its window is fully shown, so no other
        // anchor can be on it.
        let may_be_beyond = !matches!(length, Some(length) if length <= shown.len());
        let mut beyond: BTreeMap<usize, Commit> = BTreeMap::new();
        let unplaced: Vec<&str> = anchors.iter().copied().filter(|anchor| !placed.contains_key(*anchor)).collect();
        if may_be_beyond && !unplaced.is_empty() {
            let (verified, failed) = Self::locate(tip, &unplaced, shown.len(), length);
            gap |= failed;
            for (anchor, commit, position) in verified {
                placed.insert(anchor, position);
                beyond.insert(position, commit);
            }
        }

        if let Extent::Open { cap_window } = extent
            && let Some(&oldest) = placed.values().max()
        {
            if cap_window {
                shown.truncate(oldest + 2);
            }
            // The oldest anchor's first parent, unless already shown.
            let below = oldest + 1;
            if below >= shown.len() && !beyond.contains_key(&below) {
                let anchor = shown.get(oldest).or_else(|| beyond.get(&oldest));
                if let Some(parent) = anchor.and_then(|commit| commit.first_parent.clone()) {
                    beyond.insert(
                        below,
                        Commit {
                            sha: parent,
                            time: None,
                            first_parent: None,
                        },
                    );
                }
            }
        }

        let last_active = shown.first().and_then(|commit| commit.time);
        let mut positions: BTreeMap<usize, String> = beyond.into_iter().map(|(position, commit)| (position, commit.sha)).collect();
        for (position, commit) in shown.into_iter().enumerate() {
            positions.insert(position, commit.sha);
        }
        let entries = lay_out(&positions, length);
        if self.shallow && entries.iter().any(|entry| matches!(entry, LaneEntry::Elided(_))) {
            // A shallow count stops at the boundary, so `+N` is a lower bound.
            gap = true;
        }
        Ok(LaneHistory {
            entries,
            last_active,
            placed: placed.into_keys().collect(),
            gap,
        })
    }

    /// The newest `window` commits of `tip`'s first-parent chain before
    /// `stop`, newest first.
    fn newest(&self, tip: &str, stop: &[&str], window: usize) -> Result<Vec<Commit>, GatherGap> {
        let max = window.to_string();
        let mut args = vec!["log", "--first-parent", "--format=%H %ct %P", "--max-count", &max, tip];
        if !stop.is_empty() {
            args.push("--not");
            args.extend_from_slice(stop);
        }
        args.push("--");
        let output = git_command(&args).map_err(|_| GatherGap)?;
        output.lines().filter(|line| !line.is_empty()).map(parse_commit_line).collect()
    }

    /// Positions (at or past `from`, and before `length` when bounded) of the
    /// anchors on `tip`'s first-parent chain. Also returns whether a distance
    /// could not be read. An anchor that is on no drawn lane needs no report
    /// here: `GitGraph` accounts for every undrawn fork, merge, and label.
    fn locate(tip: &str, anchors: &[&str], from: usize, length: Option<usize>) -> (Vec<(String, Commit, usize)>, bool) {
        let distances: Vec<Option<usize>> = std::thread::scope(|scope| {
            let handles: Vec<_> = anchors
                .iter()
                .map(|anchor| {
                    scope.spawn(move || {
                        let range = format!("{anchor}..{tip}");
                        git_command(&["rev-list", "--first-parent", "--count", &range, "--"])
                            .ok()
                            .and_then(|count| count.trim().parse::<usize>().ok())
                    })
                })
                .collect();
            handles.into_iter().map(|handle| handle.join().ok().flatten()).collect()
        });
        let failed = distances.iter().any(Option::is_none);
        let candidates: Vec<(&str, usize)> = anchors
            .iter()
            .zip(&distances)
            .filter_map(|(anchor, distance)| Some((*anchor, (*distance)?)))
            .filter(|(_, distance)| *distance >= from && length.is_none_or(|length| *distance < length))
            .collect();
        if candidates.is_empty() {
            return (Vec::new(), failed);
        }

        // `--ignore-missing` drops a `tip~d` past the root instead of failing
        // the batch. An anchor among the answers sits at its own distance,
        // since one chain position names one commit.
        let revisions: Vec<String> = candidates.iter().map(|(_, distance)| format!("{tip}~{distance}")).collect();
        let mut args = vec!["log", "--no-walk=unsorted", "--ignore-missing", "--format=%H %P"];
        args.extend(revisions.iter().map(String::as_str));
        args.push("--");
        let found: HashMap<String, Option<String>> = match git_command(&args) {
            Ok(output) => output
                .lines()
                .filter_map(|line| {
                    let mut ids = line.split_whitespace();
                    let sha = ids.next()?.to_string();
                    Some((sha, ids.next().map(str::to_string)))
                })
                .collect(),
            Err(_) => {
                return (Vec::new(), true);
            }
        };
        let verified = candidates
            .into_iter()
            .filter_map(|(anchor, distance)| {
                let first_parent = found.get(anchor)?.clone();
                Some((
                    anchor.to_string(),
                    Commit {
                        sha: anchor.to_string(),
                        time: None,
                        first_parent,
                    },
                    distance,
                ))
            })
            .collect();
        (verified, failed)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Commit {
    sha: String,
    time: Option<i64>,
    first_parent: Option<String>,
}

/// `rev-list --first-parent --count tip --not stop`.
fn count_first_parent(tip: &str, stop: &[&str]) -> Option<usize> {
    let mut args = vec!["rev-list", "--first-parent", "--count", tip];
    if !stop.is_empty() {
        args.push("--not");
        args.extend_from_slice(stop);
    }
    args.push("--");
    git_command(&args).ok()?.trim().parse().ok()
}

/// Entries oldest first from commits by position (0 = tip). The gaps between
/// them, and for a bounded lane the run below the oldest up to `length`, are
/// `+N` squares.
fn lay_out(positions: &BTreeMap<usize, String>, length: Option<usize>) -> Vec<LaneEntry> {
    let mut entries = Vec::with_capacity(positions.len() + 2);
    let mut next_older = match (length, positions.keys().next_back()) {
        (Some(length), Some(&oldest)) => length.max(oldest + 1),
        (_, Some(&oldest)) => oldest + 1,
        (_, None) => return entries,
    };
    for (&position, sha) in positions.iter().rev() {
        let hidden = next_older - position - 1;
        if hidden > 0 {
            entries.push(LaneEntry::Elided(hidden));
        }
        entries.push(LaneEntry::Commit(sha.clone()));
        next_older = position;
    }
    entries
}

/// A SHA-1 or SHA-256 object ID in Git's lowercase hex.
fn is_object_id(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn parse_object_ids(output: &str) -> Result<Vec<String>, GatherGap> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| is_object_id(line).then(|| line.to_string()).ok_or(GatherGap))
        .collect()
}

/// `<sha> <parent>…` lines; a line that does not parse is a gap.
fn parse_parent_lines(output: &str) -> Result<Vec<(String, Vec<String>)>, GatherGap> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut ids = line.split(' ');
            let sha = ids.next().filter(|sha| is_object_id(sha)).ok_or(GatherGap)?;
            let parents: Vec<String> = ids.map(|id| is_object_id(id).then(|| id.to_string()).ok_or(GatherGap)).collect::<Result<_, _>>()?;
            Ok((sha.to_string(), parents))
        })
        .collect()
}

/// `<sha> <commit time> <parent>…` (a root commit has no parent); a line that
/// does not parse is a gap.
fn parse_commit_line(line: &str) -> Result<Commit, GatherGap> {
    let mut fields = line.split_whitespace();
    let sha = fields.next().filter(|sha| is_object_id(sha)).ok_or(GatherGap)?;
    let time = fields.next().and_then(|time| time.parse().ok()).ok_or(GatherGap)?;
    let first_parent = match fields.next() {
        Some(parent) if is_object_id(parent) => Some(parent.to_string()),
        Some(_) => return Err(GatherGap),
        None => None,
    };
    Ok(Commit {
        sha: sha.to_string(),
        time: Some(time),
        first_parent,
    })
}
