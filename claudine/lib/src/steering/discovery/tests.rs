//! Aggregator behavior with fake sources: merge, order, partial and total
//! failure, deadline, concurrency bound, unknown state, and coverage gaps.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::steering::identity::ExecutionId;

fn process(pid: u32, start: &str) -> ProcessStartIdentity {
    ProcessStartIdentity::new(pid, start).unwrap()
}

fn managed_row(execution: u128, provider: Provider, cwd: &str, key: Option<(ProcessStartIdentity, &str)>) -> SessionListing {
    SessionListing {
        id: SteeringTargetId::Managed { execution: ExecutionId::from_u128(execution) },
        provider,
        name: None,
        cwd: Some(cwd.into()),
        state: ExecutionState::Working,
        origins: vec![ObservationSource::Managed],
        launch_profile: Some("retained-rpc".into()),
        provider_version: Some("0.84.4".into()),
        availability: AvailabilitySummary::unavailable("owner verdict"),
        observed_at: Utc::now(),
        session_key: key.map(|(process, conversation)| (process, conversation.to_string())),
    }
}

fn observation(process: ProcessStartIdentity, conversation: &str, cwd: &str) -> NativeObservation {
    NativeObservation {
        process,
        conversation: conversation.into(),
        name: Some("fixture".into()),
        cwd: Some(cwd.into()),
        state: ExecutionState::Working,
        launch_profile: Some("retained-rpc".into()),
        launch_mode: LaunchMode::NonInteractive,
        provider_version: Some("0.84.4".into()),
    }
}

/// Sessions and per-row problems, or a whole-source failure.
struct Managed(Result<(Vec<SessionListing>, Vec<&'static str>), &'static str>);

impl ManagedSource for Managed {
    fn list(&self) -> DiscoveryFuture<ManagedListing> {
        let result = self.0.clone();
        Box::pin(async move {
            let (sessions, problems) = result?;
            Ok(ManagedListing { sessions, problems: problems.into_iter().map(SourceError::from).collect() })
        })
    }
}

#[derive(Default)]
struct Concurrency {
    now: AtomicUsize,
    max: AtomicUsize,
}

struct Native {
    provider: Provider,
    id: &'static str,
    result: Result<Vec<NativeObservation>, &'static str>,
    delay: Duration,
    hang: bool,
    concurrency: Arc<Concurrency>,
}

impl Native {
    fn ok(provider: Provider, observations: Vec<NativeObservation>) -> Arc<dyn NativeDiscoverer> {
        Arc::new(Self {
            provider,
            id: "fixture-native",
            result: Ok(observations),
            delay: Duration::ZERO,
            hang: false,
            concurrency: Arc::default(),
        })
    }

    fn failing(provider: Provider, message: &'static str) -> Arc<dyn NativeDiscoverer> {
        Arc::new(Self { result: Err(message), ..Self::plain(provider) })
    }

    fn plain(provider: Provider) -> Self {
        Self {
            provider,
            id: "fixture-native",
            result: Ok(Vec::new()),
            delay: Duration::ZERO,
            hang: false,
            concurrency: Arc::default(),
        }
    }
}

impl NativeDiscoverer for Native {
    fn provider(&self) -> Provider {
        self.provider
    }

    fn discovery_id(&self) -> &'static str {
        self.id
    }

    fn discover(&self) -> DiscoveryFuture<Vec<NativeObservation>> {
        let (result, delay, hang, concurrency) = (self.result.clone(), self.delay, self.hang, Arc::clone(&self.concurrency));
        Box::pin(async move {
            let now = concurrency.now.fetch_add(1, Ordering::SeqCst) + 1;
            concurrency.max.fetch_max(now, Ordering::SeqCst);
            if hang {
                std::future::pending::<()>().await;
            }
            tokio::time::sleep(delay).await;
            concurrency.now.fetch_sub(1, Ordering::SeqCst);
            Ok(result?)
        })
    }
}

async fn run(managed: Option<Result<Vec<SessionListing>, &'static str>>, native: Vec<Arc<dyn NativeDiscoverer>>) -> Result<DiscoveryReport, DiscoveryFailed> {
    let managed = managed.map(|result| {
        let listing = result.map(|sessions| (sessions, Vec::new()));
        Arc::new(Managed(listing)) as Arc<dyn ManagedSource>
    });
    discover_with(managed, &native, HostOs::Macos, DISCOVERY_DEADLINE).await
}

#[tokio::test]
async fn identical_sessions_merge_and_rows_sort_by_roster_directory_and_id() {
    let pi_child = process(900, "111");
    let managed = vec![
        managed_row(2, Provider::Kilo, "/b", None),
        managed_row(1, Provider::Pi, "/b", Some((pi_child.clone(), "conversation-1"))),
        managed_row(3, Provider::Pi, "/a", None),
    ];
    let native = vec![
        // Same process and conversation as managed execution 1: one session.
        Native::ok(Provider::Pi, vec![observation(pi_child.clone(), "conversation-1", "/b")]),
        // Same PID, new start marker: a different (PID-reused) session.
        Native::ok(Provider::Pi, vec![observation(process(900, "222"), "conversation-1", "/b")]),
        Native::ok(Provider::Claude, vec![observation(process(5, "1"), "c", "/z")]),
    ];
    let report = run(Some(Ok(managed)), native).await.unwrap();

    let ids: Vec<String> = report.sessions.iter().map(|row| row.id.to_string()).collect();
    assert_eq!(
        ids,
        [
            "native:claude:5:1:c",
            "managed:00000000-0000-0000-0000-000000000003",
            "managed:00000000-0000-0000-0000-000000000001",
            "native:pi:900:222:conversation-1",
            "managed:00000000-0000-0000-0000-000000000002",
        ],
        "roster order puts Pi before Kilo; then directory; then ID"
    );
    let merged = &report.sessions[2];
    assert_eq!(merged.origins, [ObservationSource::Managed, ObservationSource::Native]);
    assert_eq!(merged.availability.reason.as_deref(), Some("owner verdict"), "the owner's verdict is kept");
    assert_eq!(report.sessions[3].origins, [ObservationSource::Native]);
    assert!(report.errors.is_empty());
}

#[tokio::test]
async fn the_same_native_id_from_two_discoverers_is_listed_once() {
    let row = observation(process(7, "70"), "thread", "/w");
    let report = run(None, vec![Native::ok(Provider::Codex, vec![row.clone()]), Native::ok(Provider::Codex, vec![row])])
        .await
        .unwrap();
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].origins, [ObservationSource::Native]);
}

#[tokio::test]
async fn a_failed_source_is_reported_beside_the_others_with_secrets_masked() {
    let report = run(
        Some(Err("daemon refused token=ghp_0123456789abcdefghij0123")),
        vec![Native::ok(Provider::Codex, vec![observation(process(7, "70"), "thread", "/w")])],
    )
    .await
    .unwrap();
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!((report.errors[0].source.as_str(), report.errors[0].message()), ("managed", "daemon refused token=****".into()));
    let json = serde_json::to_value(&report.errors[0]).unwrap();
    assert_eq!(json, serde_json::json!({"source": "managed", "message": "daemon refused token=****"}), "only masked text is rendered");
}

#[tokio::test]
async fn an_unreadable_registration_is_reported_beside_the_readable_ones() {
    let listing = (vec![managed_row(1, Provider::Pi, "/a", None)], vec!["registration 2 names unknown provider `nope`"]);
    let managed: Arc<dyn ManagedSource> = Arc::new(Managed(Ok(listing)));
    let report = discover_with(Some(managed), &[], HostOs::Macos, DISCOVERY_DEADLINE).await.unwrap();
    assert_eq!(report.sessions.len(), 1, "the readable registration is still listed");
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].source, "managed");
    assert_eq!(report.errors[0].message(), "registration 2 names unknown provider `nope`");
}

#[tokio::test]
async fn discovery_fails_only_when_every_source_fails() {
    let failure = run(Some(Err("daemon unreachable")), vec![Native::failing(Provider::Codex, "no registry")])
        .await
        .unwrap_err();
    let sources: Vec<&str> = failure.errors.iter().map(|error| error.source.as_str()).collect();
    assert_eq!(sources, ["codex", "managed"]);

    // An empty listing is a success, not a failure.
    let empty = run(Some(Ok(Vec::new())), Vec::new()).await.unwrap();
    assert!(empty.sessions.is_empty() && empty.errors.is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_source_past_the_deadline_is_an_error_not_a_hang() {
    let hanging: Arc<dyn NativeDiscoverer> = Arc::new(Native { hang: true, ..Native::plain(Provider::Gemini) });
    let started = tokio::time::Instant::now();
    let report = run(Some(Ok(vec![managed_row(1, Provider::Pi, "/a", None)])), vec![hanging]).await.unwrap();
    assert_eq!(started.elapsed(), DISCOVERY_DEADLINE);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].source, "gemini");
    assert_eq!(report.errors[0].message(), "discovery did not finish within 5000 ms");
    assert!(report.errors[0].cause().downcast_ref::<DiscoveryTimedOut>().is_some(), "the typed cause is kept");
}

#[tokio::test(start_paused = true)]
async fn at_most_four_providers_are_discovered_at_once() {
    let concurrency = Arc::new(Concurrency::default());
    let native: Vec<Arc<dyn NativeDiscoverer>> = [Provider::Claude, Provider::Codex, Provider::Gemini, Provider::Goose, Provider::Pi, Provider::Kilo]
        .into_iter()
        .map(|provider| {
            Arc::new(Native { delay: Duration::from_millis(100), concurrency: Arc::clone(&concurrency), ..Native::plain(provider) })
                as Arc<dyn NativeDiscoverer>
        })
        .collect();
    let report = run(None, native).await.unwrap();
    assert!(report.errors.is_empty());
    assert_eq!(concurrency.max.load(Ordering::SeqCst), MAX_CONCURRENT_PROVIDERS);
}

#[tokio::test]
async fn unknown_state_or_profile_is_listed_but_unavailable() {
    let unknown_state = NativeObservation { state: ExecutionState::Unknown, ..observation(process(1, "1"), "a", "/a") };
    let unknown_profile = NativeObservation { launch_profile: None, ..observation(process(2, "2"), "b", "/b") };
    // Codex, because the shipped policy blocks Pi's `retained-rpc` profile,
    // and a reviewed block is reported ahead of an unknown state.
    let report = run(None, vec![Native::ok(Provider::Codex, vec![unknown_state, unknown_profile])]).await.unwrap();
    let reasons: Vec<Option<&str>> = report.sessions.iter().map(|row| row.availability.reason.as_deref()).collect();
    assert_eq!(
        reasons,
        [Some("session state could not be established"), Some("the session's launch profile could not be established")]
    );
    assert_eq!(report.sessions[0].state, ExecutionState::Unknown, "unknown is never guessed as working or idle");
    assert!(report.sessions.iter().all(|row| !row.is_selectable()));
}

#[tokio::test]
async fn unimplemented_native_methods_are_reported_as_gaps() {
    let report = run(Some(Ok(Vec::new())), Vec::new()).await.unwrap();
    let pi: Vec<&str> = report.gaps.iter().filter(|gap| gap.provider == Provider::Pi).map(|gap| gap.discovery_id).collect();
    assert_eq!(pi, ["ordinary-macos-native", "rpc-macos-native"], "native macOS methods only; registrations are not gaps");
    for gap in &report.gaps {
        let record = crate::steering::facts(gap.provider).discovery.iter().find(|d| d.id == gap.discovery_id).unwrap();
        assert_eq!(record.os, HostOs::Macos);
        assert_eq!(record.origin, LaunchOrigin::Native);
        assert_ne!(record.method, DiscoveryMethod::ClaudineRegistration);
    }

    let implemented: Arc<dyn NativeDiscoverer> = Arc::new(Native { id: "rpc-macos-native", ..Native::plain(Provider::Pi) });
    let report = run(Some(Ok(Vec::new())), vec![implemented]).await.unwrap();
    assert!(!report.gaps.iter().any(|gap| gap.discovery_id == "rpc-macos-native" && gap.provider == Provider::Pi));
}

#[test]
fn every_roster_provider_is_ordered_exactly_once() {
    let mut order = crate::steering::roster_order().to_vec();
    assert_eq!(order.len(), crate::provider_id::PROVIDER_COUNT);
    order.sort_by_key(|provider| provider.as_slug());
    order.dedup();
    assert_eq!(order.len(), crate::provider_id::PROVIDER_COUNT);
}

#[test]
fn listing_json_carries_the_specified_fields_and_no_identity_key() {
    let row = managed_row(1, Provider::Pi, "/a", Some((process(1, "1"), "c")));
    let json = serde_json::to_value(&row).unwrap();
    let mut keys: Vec<&str> = json.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "availability",
            "cwd",
            "id",
            "launch_profile",
            "name",
            "observed_at",
            "origins",
            "provider",
            "provider_version",
            "reason",
            "setup_requirements",
            "state"
        ]
    );
    assert_eq!(json["id"], "managed:00000000-0000-0000-0000-000000000001");
    assert_eq!(json["availability"], "unavailable");
    assert_eq!(json["provider"], "pi", "providers serialize as their slug");
    let kimi = SessionListing { provider: Provider::KimiCode, ..row.clone() };
    assert_eq!(serde_json::to_value(&kimi).unwrap()["provider"], "kimi");
    assert_eq!(json["origins"], serde_json::json!(["managed"]));
    assert!(json["name"].is_null(), "unknown values are explicit nulls");
}

