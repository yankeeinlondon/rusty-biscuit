//! Plain, render-agnostic data model for the dashboard NOW view, plus
//! the pure fold that turns the daemon's read-RPC payloads into it.
//!
//! Keeping the fold pure (it takes already-extracted `(owner, json)`
//! tuples, never a live client) lets the staleness bucketing and
//! intervention logic be unit-tested with synthetic JSON — no daemon,
//! no gRPC.

use std::collections::BTreeMap;

use serde_json::Value;

/// Sessions on a host whose last successful sync is older than this are
/// no longer trusted: the host renders as [`Staleness::Stale`] and its
/// sessions as "unknown" (dashboard spec D4).
pub const STALENESS_THRESHOLD_MS: i64 = 60_000;

/// The whole-mesh NOW view as of one capture instant.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshSnapshot {
    /// Hex node id of the daemon we queried — the one host that is
    /// always fresh.
    pub local_node_id: String,
    /// Hosts sorted local-first, then by hostname/node id.
    pub hosts: Vec<HostView>,
}

/// One mesh host: its own register plus the synced replicas this daemon
/// holds for it.
#[derive(Debug, Clone, PartialEq)]
pub struct HostView {
    pub node_id: String,
    pub hostname: Option<String>,
    pub is_local: bool,
    pub staleness: Staleness,
    pub capability: Option<HostCapabilitySummary>,
    pub sessions: Vec<SessionRow>,
    pub repo_count: usize,
}

impl HostView {
    /// Whether this host's session data is currently trustworthy.
    /// Stale / never-synced remote hosts render their sessions as
    /// "unknown" rather than as last-known state.
    pub fn sessions_trusted(&self) -> bool {
        matches!(self.staleness, Staleness::Local | Staleness::Fresh { .. })
    }
}

/// Freshness of a host's synced register replicas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Staleness {
    /// This daemon's own registers — no sync involved, always fresh.
    Local,
    /// Remote host synced within [`STALENESS_THRESHOLD_MS`].
    Fresh { age_ms: i64 },
    /// Remote host last synced longer ago than the threshold — its
    /// sessions are shown as unknown.
    Stale { age_ms: i64 },
    /// This host is a direct peer that has never completed a sync round
    /// with us (its `last_synced_unix_ms` is 0).
    NeverSynced,
    /// We hold a register replica for this host but have no direct peer
    /// record for it — the data reached us transitively (synced via
    /// another peer), so we cannot attribute a freshness age to it.
    /// Its sessions are untrusted, same as [`Staleness::Stale`], but the
    /// label stays honest rather than over-claiming "never synced".
    FreshnessUnknown,
}

/// One live session in a host's `sessions-active` register.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionRow {
    pub session_id: String,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub interactive: Option<bool>,
    pub repo_root: Option<String>,
    pub status: Option<String>,
    /// Daemon-projected reducer basis for [`status`](Self::status) —
    /// e.g. `permission_ask`, `interactive_turn_complete`. Provenance the
    /// UI surfaces so an intervention reads with its cause, not bare.
    pub status_basis: Option<String>,
    /// Daemon-projected producer that authored the winning status —
    /// e.g. `sink`, `permission_hook`. Surfaced so a fallback-sink signal
    /// is distinguishable from a first-class hook one.
    pub status_producer: Option<String>,
    /// Whether the launched provider can even emit a permission signal
    /// (`supported` / `unsupported`), recorded at STARTED. Lets the UI
    /// tell "no intervention needed" apart from "can't tell — this agent
    /// has no permission signal" (never rendering absence as a negative).
    pub permission_signal: Option<String>,
    /// Raw `updated_at_unix_ms` the producer stamped on this entry.
    pub updated_at_unix_ms: Option<i64>,
    /// Age of this entry (capture instant − `updated_at_unix_ms`),
    /// populated only for the LOCAL host. Remote wall-clock skew makes
    /// this meaningless across hosts, so it stays `None` there (a
    /// separate Trigger-2 concern). Exposes a crashed producer that left
    /// a phantom entry behind on an otherwise-fresh host.
    pub reported_age_ms: Option<i64>,
    pub intervention: Intervention,
}

/// The tiered "needs human intervention" signal, carrying its basis so
/// the UI can render certainty honestly (dashboard spec D5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intervention {
    /// No attention needed (or the signal is not trustworthy right now).
    None,
    /// Trigger 1 (exact): the session reported it is waiting on the
    /// user — a permission ask / blocked hook that has not cleared.
    NeedsInput,
    /// Trigger 2 (weak): an interactive session went idle after its last
    /// assistant turn completed — likely waiting on the user, but softer
    /// than an outstanding permission ask, so it must never outrank
    /// [`NeedsInput`].
    PossiblyIdle,
}

/// The subset of a host's capability register worth showing inline.
#[derive(Debug, Clone, PartialEq)]
pub struct HostCapabilitySummary {
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub arch: Option<String>,
    pub cpu_cores: Option<i64>,
    pub memory_bytes: Option<i64>,
    pub gpu: Option<String>,
}

/// One host's register replica as `(owner_node_id, json_payload)`.
pub type RegisterBlob<'a> = (&'a str, &'a str);

impl MeshSnapshot {
    /// Fold the daemon's read-RPC payloads into the mesh view.
    ///
    /// `last_synced` maps a direct-peer node id to its
    /// `last_synced_unix_ms` (from `ListPeers`). Presence in this map is
    /// the sole evidence of a *direct* peer relationship: a value of 0
    /// is a direct peer we have never synced with ([`Staleness::NeverSynced`]),
    /// a positive value drives the Fresh/Stale age math, and a node that
    /// only appears via a register replica but is absent here reached us
    /// transitively — it becomes [`Staleness::FreshnessUnknown`] rather
    /// than being mislabeled never-synced. Every peer id also joins the
    /// host union, so a peer with no register documents still renders a
    /// row (with empty sessions). `now_ms` is the local clock — the same
    /// clock that stamped `last_synced`, so the age math is skew-free.
    ///
    /// When `local_only` is set, only the local host is retained.
    pub fn fold(
        local_node_id: &str,
        now_ms: i64,
        last_synced: &BTreeMap<String, i64>,
        active: &[RegisterBlob<'_>],
        capabilities: &[RegisterBlob<'_>],
        repos: &[RegisterBlob<'_>],
        local_only: bool,
    ) -> Self {
        // Union every node id that appears in any register domain, every
        // direct peer, plus ourselves. A capability-only host still shows
        // up; a peer with no register documents at all still gets a row
        // (D4: every remote host is accounted for).
        let mut node_ids: BTreeMap<String, ()> = BTreeMap::new();
        node_ids.insert(local_node_id.to_string(), ());
        for (owner, _) in active.iter().chain(capabilities).chain(repos) {
            node_ids.insert((*owner).to_string(), ());
        }
        for node_id in last_synced.keys() {
            node_ids.insert(node_id.clone(), ());
        }

        let active_by_owner = index(active);
        let capability_by_owner = index(capabilities);
        let repos_by_owner = index(repos);

        let mut hosts: Vec<HostView> = node_ids
            .keys()
            .filter_map(|node_id| {
                let is_local = node_id == local_node_id;
                if local_only && !is_local {
                    return None;
                }
                let staleness = if is_local {
                    Staleness::Local
                } else {
                    match last_synced.get(node_id).copied() {
                        // A direct peer we hold a record for but have
                        // never completed a sync round with.
                        Some(0) => Staleness::NeverSynced,
                        Some(ts) => {
                            let age = (now_ms - ts).max(0);
                            if age <= STALENESS_THRESHOLD_MS {
                                Staleness::Fresh { age_ms: age }
                            } else {
                                Staleness::Stale { age_ms: age }
                            }
                        }
                        // No direct peer record: this replica reached us
                        // transitively, so its freshness is unknown.
                        None => Staleness::FreshnessUnknown,
                    }
                };

                let capability_json = capability_by_owner.get(node_id.as_str());
                let capability = capability_json.and_then(|json| parse_capability(json));
                let hostname = capability_json.and_then(|json| parse_hostname(json));
                let trusted = matches!(staleness, Staleness::Local | Staleness::Fresh { .. });
                let sessions = active_by_owner
                    .get(node_id.as_str())
                    .map(|json| parse_sessions(json, trusted, is_local, now_ms))
                    .unwrap_or_default();
                let repo_count = repos_by_owner
                    .get(node_id.as_str())
                    .map(|json| count_repos(json))
                    .unwrap_or(0);

                Some(HostView {
                    node_id: node_id.clone(),
                    hostname,
                    is_local,
                    staleness,
                    capability,
                    sessions,
                    repo_count,
                })
            })
            .collect();

        // Local first, then by hostname (falling back to node id).
        hosts.sort_by(|a, b| {
            b.is_local
                .cmp(&a.is_local)
                .then_with(|| host_label(a).cmp(host_label(b)))
        });

        Self {
            local_node_id: local_node_id.to_string(),
            hosts,
        }
    }

    /// Sessions on trusted hosts — the only ones we can honestly present
    /// as live. This is what the headline "N sessions" must count.
    pub fn trusted_live_sessions(&self) -> usize {
        self.hosts
            .iter()
            .filter(|h| h.sessions_trusted())
            .map(|h| h.sessions.len())
            .sum()
    }

    /// Sessions on untrusted (stale / freshness-unknown / never-synced)
    /// hosts — last-known observations, never counted as live.
    pub fn last_known_sessions(&self) -> usize {
        self.hosts
            .iter()
            .filter(|h| !h.sessions_trusted())
            .map(|h| h.sessions.len())
            .sum()
    }

    /// Count of sessions flagged as needing input across trusted hosts.
    pub fn needs_input_count(&self) -> usize {
        self.hosts
            .iter()
            .flat_map(|h| &h.sessions)
            .filter(|s| s.intervention == Intervention::NeedsInput)
            .count()
    }

    /// Count of sessions flagged as possibly idle across trusted hosts —
    /// the weaker Trigger-2 signal, reported alongside the strong
    /// needs-input count rather than folded into it.
    pub fn idle_count(&self) -> usize {
        self.hosts
            .iter()
            .flat_map(|h| &h.sessions)
            .filter(|s| s.intervention == Intervention::PossiblyIdle)
            .count()
    }
}

fn host_label(host: &HostView) -> &str {
    host.hostname.as_deref().unwrap_or(&host.node_id)
}

fn index<'a>(blobs: &'a [RegisterBlob<'a>]) -> BTreeMap<&'a str, &'a str> {
    blobs.iter().map(|(owner, json)| (*owner, *json)).collect()
}

fn parse_hostname(fields_json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(fields_json).ok()?;
    value
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_capability(fields_json: &str) -> Option<HostCapabilitySummary> {
    let value: Value = serde_json::from_str(fields_json).ok()?;
    let obj = value.as_object()?;
    let str_field = |k: &str| obj.get(k).and_then(Value::as_str).map(str::to_string);
    Some(HostCapabilitySummary {
        os: str_field("os"),
        os_version: str_field("os_version"),
        arch: str_field("arch"),
        cpu_cores: obj.get("cpu_cores").and_then(Value::as_i64),
        memory_bytes: obj.get("memory").and_then(Value::as_i64),
        gpu: str_field("gpu"),
    })
}

fn count_repos(repos_json: &str) -> usize {
    serde_json::from_str::<Value>(repos_json)
        .ok()
        .and_then(|v| v.as_object().map(serde_json::Map::len))
        .unwrap_or(0)
}

fn parse_sessions(
    sessions_json: &str,
    trusted: bool,
    is_local: bool,
    now_ms: i64,
) -> Vec<SessionRow> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(sessions_json) else {
        return Vec::new();
    };
    let mut rows: Vec<SessionRow> = map
        .into_iter()
        .map(|(session_id, entry)| {
            let str_field = |k: &str| entry.get(k).and_then(Value::as_str).map(str::to_string);
            let status = str_field("status");
            // Only trust the intervention signal for a fresh host — a
            // stale replica's "waiting_on_user" may be an hour old.
            // Strong needs-input outranks the weaker idle tier; both are
            // suppressed on untrusted hosts.
            let intervention = if !trusted {
                Intervention::None
            } else if is_waiting(status.as_deref()) {
                Intervention::NeedsInput
            } else if is_idle(status.as_deref()) {
                Intervention::PossiblyIdle
            } else {
                Intervention::None
            };
            let updated_at_unix_ms = entry.get("updated_at_unix_ms").and_then(Value::as_i64);
            // Skew-free only for the local host; remote clocks make this
            // meaningless, so it stays None there.
            let reported_age_ms = if is_local {
                updated_at_unix_ms.map(|ts| (now_ms - ts).max(0))
            } else {
                None
            };
            SessionRow {
                session_id,
                agent: str_field("agent"),
                model: str_field("model"),
                interactive: entry.get("interactive").and_then(Value::as_bool),
                repo_root: str_field("repo_root"),
                status,
                status_basis: str_field("status_basis"),
                status_producer: str_field("status_producer"),
                permission_signal: str_field("permission_signal"),
                updated_at_unix_ms,
                reported_age_ms,
                intervention,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    rows
}

/// Statuses a producer sets to signal a stuck-on-the-user turn.
fn is_waiting(status: Option<&str>) -> bool {
    matches!(status, Some("waiting_on_user") | Some("blocked"))
}

/// The status Trigger 2's idle hook sets when an interactive session's
/// last assistant turn completed and it is now waiting on the user.
fn is_idle(status: Option<&str>) -> bool {
    matches!(status, Some("idle"))
}
