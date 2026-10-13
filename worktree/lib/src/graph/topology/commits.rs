//! The repository's commit graph read in-process with `gix`, so the graph's
//! history questions start no git process (one costs ~25–47 ms on Windows,
//! and a gather asks a dozen or more).
//!
//! Every answer is the one Git gives for the command [`History`](super::History)
//! would otherwise run: reachability is painted down from both commits in
//! commit-date order, as `git merge-base` does, and a commit listed in the
//! `shallow` file has no parents, as Git grafts it. A repository whose parents
//! Git rewrites (replace refs, a grafts file) is left to Git.

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use gix::ObjectId;

use super::GatherGap;

/// One commit as Git's history walk sees it.
#[derive(Debug, Clone)]
pub(in crate::graph) struct Node {
    /// First parent first; empty for a root or a shallow-cut commit.
    pub parents: Vec<ObjectId>,
    /// Committer time, Unix seconds (`%ct`).
    pub time: i64,
}

/// The commits one gathering pass has read, shared by its threads.
pub(in crate::graph) struct Commits {
    state: Mutex<State>,
    shallow: bool,
}

impl std::fmt::Debug for Commits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commits").field("shallow", &self.shallow).finish_non_exhaustive()
    }
}

struct State {
    repo: gix::Repository,
    nodes: HashMap<ObjectId, Node>,
    /// The `shallow` file's commits, whose parents Git does not read.
    cut: HashSet<ObjectId>,
    /// First-parent chains by tip, read as far as asked.
    chains: HashMap<ObjectId, Chain>,
}

#[derive(Default)]
struct Chain {
    /// `ids[0]` is the tip.
    ids: Vec<ObjectId>,
    /// The chain reached a commit without parents.
    ended: bool,
}

const PARENT1: u8 = 1;
const PARENT2: u8 = 2;
const STALE: u8 = 4;
const RESULT: u8 = 8;

impl Commits {
    /// The repository containing `dir`, or `None` when `gix` cannot open it
    /// or Git would rewrite parents there (a grafts file or replace refs).
    pub fn open(dir: &Path) -> Option<Self> {
        let mut repo = crate::git_metadata::open(dir)?;
        if repo.common_dir().join("info").join("grafts").exists() || has_replace_refs(&repo)? {
            return None;
        }
        // Git reads the `shallow` file when it exists, even an empty one.
        let shallow = repo.shallow_file().is_file();
        let cut = match repo.shallow_commits().ok()? {
            Some(commits) => commits.iter().copied().collect(),
            None => HashSet::new(),
        };
        repo.object_cache_size_if_unset(4 * 1024 * 1024);
        Some(Self {
            state: Mutex::new(State { repo, nodes: HashMap::new(), cut, chains: HashMap::new() }),
            shallow,
        })
    }

    /// What `git rev-parse --is-shallow-repository` prints.
    pub fn is_shallow(&self) -> bool {
        self.shallow
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether `ancestor` is reachable from `descendant` (or is it). A gap
    /// only when a commit cannot be read; a shallow "no" is the caller's to
    /// distrust.
    pub fn reaches(&self, ancestor: &str, descendant: &str) -> Result<bool, GatherGap> {
        let mut state = self.state();
        let (ancestor, descendant) = (parse(ancestor)?, parse(descendant)?);
        state.reaches(ancestor, descendant)
    }

    /// `git merge-base a b`: the newest best common ancestor, `None` for
    /// unrelated histories.
    pub fn merge_base(&self, a: &str, b: &str) -> Result<Option<String>, GatherGap> {
        let mut state = self.state();
        state.merge_base(parse(a)?, parse(b)?).map(|base| base.map(|id| id.to_string()))
    }

    /// The commit `index` first parents below `tip`, with its node; `None`
    /// past the chain's end.
    pub fn chain_at(&self, tip: &str, index: usize) -> Result<Option<(String, Node)>, GatherGap> {
        let mut state = self.state();
        let tip = parse(tip)?;
        match state.chain_at(tip, index)? {
            Some(id) => {
                let node = state.node(id)?.clone();
                Ok(Some((id.to_string(), node)))
            }
            None => Ok(None),
        }
    }

    /// `git rev-list --first-parent --count <tip> --not <stops>`: how many of
    /// `tip`'s first-parent chain come before the first commit that a stop
    /// reaches, read no further than `limit`.
    pub fn leading_run(&self, tip: &str, stops: &[&str], limit: Option<usize>) -> Result<usize, GatherGap> {
        let mut state = self.state();
        let tip = parse(tip)?;
        let stops: Vec<ObjectId> = stops.iter().map(|stop| parse(stop)).collect::<Result<_, _>>()?;
        // Git refuses an unknown revision on either side.
        state.node(tip)?;
        for stop in &stops {
            state.node(*stop)?;
        }
        state.leading_run(tip, &stops, limit)
    }

    /// The position of `commit` on `tip`'s first-parent chain; `None` when
    /// it is not there, including when it is no readable commit.
    pub fn position(&self, tip: &str, commit: &str) -> Result<Option<usize>, GatherGap> {
        let mut state = self.state();
        let tip = parse(tip)?;
        state.node(tip)?;
        let Ok(commit) = parse(commit) else {
            return Ok(None);
        };
        if state.node(commit).is_err() {
            return Ok(None);
        }
        let distance = state.leading_run(tip, &[commit], None)?;
        Ok((state.chain_at(tip, distance)? == Some(commit)).then_some(distance))
    }

    /// Whether the object database has `id` at all (`git cat-file -e`).
    pub fn has_object(&self, id: &str) -> bool {
        let state = self.state();
        parse(id).is_ok_and(|id| state.repo.has_object(id))
    }

    /// The commits among `chain` (newest first, each containing the next's
    /// descendant) that `ancestor` reaches: always a leading run, since
    /// containment is monotone along a first-parent chain. Its length.
    pub fn containing_run(&self, ancestor: &str, chain: &[String]) -> Result<usize, GatherGap> {
        let mut state = self.state();
        let ancestor = parse(ancestor)?;
        let chain: Vec<ObjectId> = chain.iter().map(|id| parse(id)).collect::<Result<_, _>>()?;
        let (mut low, mut high) = (0, chain.len());
        while low < high {
            let middle = (low + high) / 2;
            if state.reaches(ancestor, chain[middle])? {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        Ok(low)
    }
}

impl State {
    fn node(&mut self, id: ObjectId) -> Result<&Node, GatherGap> {
        if !self.nodes.contains_key(&id) {
            let commit = self.repo.find_commit(id).map_err(|_| GatherGap)?;
            let decoded = commit.decode().map_err(|_| GatherGap)?;
            let parents = if self.cut.contains(&id) { Vec::new() } else { decoded.parents().collect() };
            let time = decoded.time().map_err(|_| GatherGap)?.seconds;
            self.nodes.insert(id, Node { parents, time });
        }
        Ok(&self.nodes[&id])
    }

    fn chain_at(&mut self, tip: ObjectId, index: usize) -> Result<Option<ObjectId>, GatherGap> {
        loop {
            let chain = self.chains.entry(tip).or_default();
            if let Some(id) = chain.ids.get(index) {
                return Ok(Some(*id));
            }
            if chain.ended {
                return Ok(None);
            }
            let last = chain.ids.last().copied();
            let next = match last {
                None => Some(tip),
                Some(last) => self.node(last)?.parents.first().copied(),
            };
            let chain = self.chains.entry(tip).or_default();
            match next {
                Some(id) => chain.ids.push(id),
                None => chain.ended = true,
            }
        }
    }

    /// Whether any of `stops` reaches `commit`.
    fn excluded(&mut self, commit: ObjectId, stops: &[ObjectId]) -> Result<bool, GatherGap> {
        for stop in stops {
            if self.reaches(commit, *stop)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The first index on `tip`'s chain that a stop reaches (or the chain's
    /// length), capped at `limit`. Exclusion is monotone along the chain, so
    /// a doubling probe and a bisection find it.
    fn leading_run(&mut self, tip: ObjectId, stops: &[ObjectId], limit: Option<usize>) -> Result<usize, GatherGap> {
        if stops.is_empty() {
            let mut length = 0;
            while limit.is_none_or(|limit| length < limit) && self.chain_at(tip, length)?.is_some() {
                length += 1;
            }
            return Ok(length);
        }
        // Every index below `low` is included; `high` is excluded, the end,
        // or the limit.
        let mut low = 0;
        let mut step = 1;
        let mut high = loop {
            let probe = low + step - 1;
            if let Some(limit) = limit
                && probe >= limit
            {
                break limit;
            }
            match self.chain_at(tip, probe)? {
                None => break self.chains[&tip].ids.len(),
                Some(id) if self.excluded(id, stops)? => break probe,
                Some(_) => {
                    low = probe + 1;
                    step *= 2;
                }
            }
        };
        while low < high {
            let middle = (low + high) / 2;
            match self.chain_at(tip, middle)? {
                // Past the chain's end, as below the limit, nothing is listed.
                None => high = middle,
                Some(id) if self.excluded(id, stops)? => high = middle,
                Some(_) => low = middle + 1,
            }
        }
        Ok(low)
    }

    fn reaches(&mut self, ancestor: ObjectId, descendant: ObjectId) -> Result<bool, GatherGap> {
        if ancestor == descendant {
            self.node(ancestor)?;
            return Ok(true);
        }
        let paint = self.paint_down_to_common(ancestor, descendant, true)?;
        Ok(paint.flags.get(&ancestor).is_some_and(|flags| flags & PARENT2 != 0))
    }

    fn merge_base(&mut self, a: ObjectId, b: ObjectId) -> Result<Option<ObjectId>, GatherGap> {
        if a == b {
            self.node(a)?;
            return Ok(Some(a));
        }
        let paint = self.paint_down_to_common(a, b, false)?;
        let mut bases: Vec<ObjectId> = paint
            .results
            .into_iter()
            .filter(|id| paint.flags.get(id).is_none_or(|flags| flags & STALE == 0))
            .collect();
        if bases.len() > 1 {
            // A base another base reaches is not a best one.
            let candidates = bases.clone();
            let mut kept = Vec::with_capacity(candidates.len());
            for base in &candidates {
                let mut redundant = false;
                for other in candidates.iter().filter(|other| *other != base) {
                    if self.reaches(*base, *other)? {
                        redundant = true;
                        break;
                    }
                }
                if !redundant {
                    kept.push(*base);
                }
            }
            bases = kept;
        }
        // Git lists the bases newest first, an equal date after the ones it
        // already has, and prints the first.
        let mut ordered: Vec<(i64, ObjectId)> = Vec::with_capacity(bases.len());
        for base in bases {
            let time = self.node(base)?.time;
            let at = ordered.iter().position(|(listed, _)| *listed < time).unwrap_or(ordered.len());
            ordered.insert(at, (time, base));
        }
        Ok(ordered.first().map(|(_, id)| *id))
    }

    /// Git's `paint_down_to_common`: `one` and `two` are painted down in
    /// commit-date order (ties first come, first served) until only commits
    /// both reach remain queued. With `until_one_reached`, it stops as soon as
    /// `two` reaches `one`.
    fn paint_down_to_common(&mut self, one: ObjectId, two: ObjectId, until_one_reached: bool) -> Result<Paint, GatherGap> {
        let mut flags: HashMap<ObjectId, u8> = HashMap::new();
        let mut queue: BinaryHeap<Queued> = BinaryHeap::new();
        let mut sequence = 0u64;
        let mut results = Vec::new();
        for (id, side) in [(one, PARENT1), (two, PARENT2)] {
            let time = self.node(id)?.time;
            *flags.entry(id).or_default() |= side;
            queue.push(Queued { time, order: std::cmp::Reverse(sequence), id });
            sequence += 1;
        }
        while queue.iter().any(|queued| flags.get(&queued.id).is_none_or(|flags| flags & STALE == 0)) {
            let Some(Queued { id, .. }) = queue.pop() else { break };
            let own = flags.get(&id).copied().unwrap_or(0);
            let mut paint = own & (PARENT1 | PARENT2 | STALE);
            if paint == PARENT1 | PARENT2 {
                if own & RESULT == 0 {
                    flags.insert(id, own | RESULT);
                    results.push(id);
                }
                paint |= STALE;
            }
            let parents = self.node(id)?.parents.clone();
            for parent in parents {
                if flags.get(&parent).is_some_and(|flags| flags & paint == paint) {
                    continue;
                }
                let time = self.node(parent)?.time;
                *flags.entry(parent).or_default() |= paint;
                queue.push(Queued { time, order: std::cmp::Reverse(sequence), id: parent });
                sequence += 1;
            }
            if until_one_reached && flags.get(&one).is_some_and(|flags| flags & PARENT2 != 0) {
                break;
            }
        }
        Ok(Paint { flags, results })
    }
}

struct Paint {
    flags: HashMap<ObjectId, u8>,
    /// Commits both sides reached, in the order found.
    results: Vec<ObjectId>,
}

/// A queued commit: newest first, then first queued first.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Queued {
    time: i64,
    order: std::cmp::Reverse<u64>,
    id: ObjectId,
}

fn parse(id: &str) -> Result<ObjectId, GatherGap> {
    ObjectId::from_hex(id.as_bytes()).map_err(|_| GatherGap)
}

/// Whether any ref lives under `refs/replace/`; `None` when refs cannot be
/// read.
fn has_replace_refs(repo: &gix::Repository) -> Option<bool> {
    let platform = repo.references().ok()?;
    let mut replaced = platform.prefixed("refs/replace/").ok()?;
    Some(replaced.next().is_some())
}
