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

/// How a branch tip `T` relates to the candidate lane [`History::classify`]
/// chose. `candidate` indexes the candidates passed to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Integration {
    /// No candidate contains `T`.
    Unmerged,
    /// `merge` brought `T` in as a non-first parent; `first_parent` is the
    /// candidate lane's commit before it. `after_indirect`: an earlier
    /// candidate contains `T` only indirectly, so its tip is no fork
    /// reference for the branch.
    MergedDirectly {
        candidate: usize,
        merge: String,
        first_parent: String,
        after_indirect: bool,
    },
    /// `T` is on the candidate's first-parent chain: the branch has no
    /// history of its own to draw.
    NoSeparateHistory { candidate: usize },
    /// `T` reached the candidate through another branch's merge, so no merge
    /// of its own can be drawn. `merge` is the first commit on the candidate
    /// lane containing `T` (that other branch's merge); `first_parent` is the
    /// lane's commit before it.
    IntegratedOtherwise {
        candidate: usize,
        merge: String,
        first_parent: String,
    },
}

/// Where a lane's history ends.
#[derive(Debug, Clone, Copy)]
pub(super) enum Extent<'a> {
    /// A branch lane: the window [`History::lane_window`] read. Older commits
    /// fold into a leading `+N`.
    Until(&'a LaneWindow),
    /// The default lane, which runs to the root, showing its newest `window`
    /// commits. It is drawn down to its oldest anchor plus that anchor's
    /// first parent (so a merge is never the lane's first commit).
    /// `cap_window`: the window, too, stops there.
    Open { window: usize, cap_window: bool },
}

/// A branch lane's newest commits before its stop history, read once so the
/// boundary lookup and the lane's entries share it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct LaneWindow {
    /// Newest first.
    shown: Vec<Commit>,
    /// Commits on the lane; `shown.len()` when they could not be counted.
    length: usize,
    /// Whether `length` was read or is a lower bound (a gap).
    counted: bool,
    /// Lane commits past `shown` whose positions are already verified.
    known: Vec<Boundary>,
}

impl LaneWindow {
    /// Commits at verified positions the lane places without asking Git.
    pub fn with_known(mut self, known: Vec<Boundary>) -> Self {
        self.known = known;
        self
    }
}

/// The first commit on a lane tip's first-parent chain inside its stop
/// history, `distance` first parents below the tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Boundary {
    pub sha: String,
    pub distance: usize,
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

    /// Whether the repository is shallow, as read (or assumed after a
    /// failed read).
    pub fn is_shallow(&self) -> bool {
        self.shallow
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

    /// How `tip` was integrated into `candidates` (lane tips, in priority
    /// order).
    ///
    /// The first candidate with a direct merge or a first-parent match wins.
    /// An indirect match is weaker: the first one is kept only if no later
    /// candidate has either. A gap before any indirect match is the result,
    /// even if a later candidate would merge `tip` directly, since that does
    /// not prove which connection should win; a gap after one returns it.
    ///
    /// `C` is the oldest commit on the candidate's first-parent chain that
    /// descends from `tip`: the leading run of `rev-list --first-parent
    /// --parents T..X` that is also in `rev-list --ancestry-path T..X`.
    /// Containment is monotone along a first-parent chain, so the run is
    /// exactly the chain's commits containing `tip`.
    pub fn classify(&self, tip: &str, candidates: &[&str]) -> Result<Integration, GatherGap> {
        let mut indirect = None;
        for (candidate, lane_tip) in candidates.iter().enumerate() {
            match self.integration_into(tip, candidate, lane_tip, indirect.is_some()) {
                Ok(None) => {}
                Ok(Some(found @ Integration::IntegratedOtherwise { .. })) => {
                    indirect.get_or_insert(found);
                }
                Ok(Some(found)) => return Ok(found),
                Err(gap) => return indirect.ok_or(gap),
            }
        }
        if let Some(found) = indirect {
            return Ok(found);
        }
        if self.shallow {
            // Unreachable in practice (`is_ancestor` already refused), kept
            // so a shallow "no" can never read as unmerged.
            return Err(GatherGap);
        }
        Ok(Integration::Unmerged)
    }

    /// How `tip` relates to one candidate lane; `None` when the lane does not
    /// contain it.
    fn integration_into(&self, tip: &str, candidate: usize, lane_tip: &str, after_indirect: bool) -> Result<Option<Integration>, GatherGap> {
        if !self.is_ancestor(tip, lane_tip)? {
            return Ok(None);
        }
        let range = format!("{tip}..{lane_tip}");
        let chain = parse_parent_lines(&git_command(&["rev-list", "--first-parent", "--parents", &range, "--"]).map_err(|_| GatherGap)?)?;
        if chain.is_empty() {
            // `T..X` is empty only when `T` is `X`; a shallow walk can
            // also come back empty, so there it proves nothing.
            if self.shallow {
                return Err(GatherGap);
            }
            return Ok(Some(Integration::NoSeparateHistory { candidate }));
        }
        // Every commit on the chain descends from `tip`, so when its oldest
        // commit's first parent is `tip`, the leading run is the whole chain
        // and `--ancestry-path` would only confirm it.
        if chain.last().and_then(|(_, parents)| parents.first()).is_some_and(|parent| parent == tip) {
            return Ok(Some(Integration::NoSeparateHistory { candidate }));
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
        Ok(Some(if first_parent == tip {
            Integration::NoSeparateHistory { candidate }
        } else if parents[1..].iter().any(|parent| parent == tip) {
            Integration::MergedDirectly {
                candidate,
                merge: merge.clone(),
                first_parent: first_parent.clone(),
                after_indirect,
            }
        } else {
            Integration::IntegratedOtherwise {
                candidate,
                merge: merge.clone(),
                first_parent: first_parent.clone(),
            }
        }))
    }

    /// The first-parent chain of `tip` within `extent`: its newest commits,
    /// every anchor on the chain at its real position, and `+N` squares for
    /// the runs between them.
    ///
    /// An anchor the window already verified ([`LaneWindow::with_known`]) is
    /// placed as is. Any other anchor's position is `rev-list --first-parent
    /// --count A..tip`, verified by one batched lookup of `tip~<position>`; an
    /// anchor that fails
    /// is not on this lane and is left out of [`LaneHistory::placed`]. A default
    /// lane whose newest commits cannot be read is a gap that still places its
    /// anchors, so the lane keeps every verified fork, merge, and label.
    pub fn first_parent_entries(&self, tip: &str, extent: Extent, anchors: &[&str]) -> LaneHistory {
        let (mut shown, length, mut gap, known) = match extent {
            Extent::Until(lane) => (lane.shown.clone(), Some(lane.length), !lane.counted, lane.known.as_slice()),
            Extent::Open { window, .. } => match self.newest(tip, &[], window) {
                Ok(shown) => (shown, None, false, &[][..]),
                Err(GatherGap) => (Vec::new(), None, true, &[][..]),
            },
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
        let mut beyond: BTreeMap<usize, Commit> = BTreeMap::new();
        for boundary in known {
            if anchors.contains(&boundary.sha.as_str()) && !placed.contains_key(&boundary.sha) {
                placed.insert(boundary.sha.clone(), boundary.distance);
                beyond.insert(
                    boundary.distance,
                    Commit {
                        sha: boundary.sha.clone(),
                        time: None,
                        first_parent: None,
                    },
                );
            }
        }
        // A branch lane shorter than its window is fully shown, so no other
        // anchor can be on it.
        let may_be_beyond = !matches!(length, Some(length) if length <= shown.len());
        let unplaced: Vec<&str> = anchors.iter().copied().filter(|anchor| !placed.contains_key(*anchor)).collect();
        if may_be_beyond && !unplaced.is_empty() {
            let (verified, failed) = Self::locate(tip, &unplaced, shown.len(), length);
            gap |= failed;
            for (anchor, commit, position) in verified {
                placed.insert(anchor, position);
                beyond.insert(position, commit);
            }
        }

        if let Extent::Open { cap_window, .. } = extent
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
        LaneHistory {
            entries,
            last_active,
            placed: placed.into_keys().collect(),
            gap,
        }
    }

    /// A branch lane's newest `window` commits before `stop`'s history, and
    /// its length: counted only when the window is full, since a shorter
    /// window is the whole lane. A failed count is a gap in the window, not
    /// an `Err`, which is only for a window that cannot be read.
    pub fn lane_window(&self, tip: &str, stop: &[&str], window: usize) -> Result<LaneWindow, GatherGap> {
        let shown = self.newest(tip, stop, window)?;
        let (length, counted) = if shown.len() < window {
            (shown.len(), true)
        } else {
            match count_first_parent(tip, stop) {
                Some(count) => (count.max(shown.len()), true),
                None => (shown.len(), false),
            }
        };
        Ok(LaneWindow {
            shown,
            length,
            counted,
            known: Vec::new(),
        })
    }

    /// The lane's boundary: the first parent of its oldest commit. `None`
    /// when the lane is empty or ends at a root commit.
    ///
    /// A fully shown lane already has that parent; otherwise one `log` reads
    /// the parents of the lane's oldest commit, `tip~(length - 1)`. A missing
    /// parent in a shallow clone is the shallow cut, not a root, so it is a
    /// gap.
    pub fn boundary(&self, tip: &str, lane: &LaneWindow) -> Result<Option<Boundary>, GatherGap> {
        if !lane.counted {
            return Err(GatherGap);
        }
        if lane.length == 0 {
            return Ok(None);
        }
        let first_parent = if lane.shown.len() == lane.length {
            lane.shown.last().and_then(|oldest| oldest.first_parent.clone())
        } else {
            let oldest = format!("{tip}~{}", lane.length - 1);
            let output = git_command(&["log", "--no-walk=unsorted", "--ignore-missing", "--format=%H %P", &oldest, "--"]).map_err(|_| GatherGap)?;
            let mut lines = parse_parent_lines(&output)?.into_iter();
            let (_, parents) = lines.next().ok_or(GatherGap)?;
            parents.into_iter().next()
        };
        match first_parent {
            Some(sha) => Ok(Some(Boundary {
                sha,
                distance: lane.length,
            })),
            None if self.shallow => Err(GatherGap),
            None => Ok(None),
        }
    }

    /// How many first parents below `tip` the commit `recorded` is, when it
    /// is on `tip`'s first-parent chain. `None` for a value that is not a full
    /// object ID (checked before any Git call), an object that is missing in
    /// a complete clone, a non-commit, or a commit off the chain. A missing
    /// object in a shallow clone may be beyond the cut, so it is a gap.
    pub fn chain_distance(&self, tip: &str, recorded: &str) -> Result<Option<usize>, GatherGap> {
        if !is_object_id(recorded) {
            return Ok(None);
        }
        // `--ignore-missing` makes an unknown `recorded` count the whole
        // chain instead of failing, so a failure here is a real one.
        let distance: usize = git_command(&["rev-list", "--first-parent", "--count", "--ignore-missing", tip, "--not", recorded, "--"])
            .map_err(|_| GatherGap)?
            .trim()
            .parse()
            .map_err(|_| GatherGap)?;
        let at = format!("{tip}~{distance}");
        let found = git_command(&["log", "--no-walk=unsorted", "--ignore-missing", "--format=%H", &at, "--"]).map_err(|_| GatherGap)?;
        if found.trim() == recorded {
            return Ok(Some(distance));
        }
        if !self.shallow {
            return Ok(None);
        }
        match git_command_allow_no_match(&["cat-file", "-e", recorded]) {
            Ok(Some(_)) => Ok(None),
            _ => Err(GatherGap),
        }
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
