//! The slot session and the process-wide resolution cache, in-process.
//!
//! Each case builds a real Git repository in a temp dir and resolves a real
//! template (`simplified-schema`'s resolver) through [`SlotSession`] with the
//! real expression engine, automatic grant, and grant store. A "command" is
//! one session; a cache hit across commands is a second session over the
//! same template. Evaluations are counted by the `SlotResolution` work
//! counter, so the counting cases need the `work-counters` feature that
//! `just test` enables.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde_json::Value;
use simplified_schema::SchemaError;
use simplified_schema::slots::PendingSchema;

use crate::markdown::schemas::slots::cache::InputCheck;
use crate::markdown::schemas::slots::session::{SessionMode, SlotSession};
use crate::markdown::compose::DataRuntime;
use crate::markdown::compose::expression::data::{Observation, ObservationKind};

const OS_LIST: &str = "- macOS\n- Linux\n- Windows\n";
const SCHEMA: &str = "schemas/sandbox.yaml";
const DATA: &str = "data/os.yaml";

fn schema_with(expression: &str) -> String {
    format!("kind: schema\n$schema:\n    os: enum({{{{ {expression} }}}}; required)\n")
}

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed in {}", root.display());
}

/// A committed repository holding the schema and its data file.
struct Repo {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new(schema: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("repo");
        let repo = Self { _temp: temp, root };
        repo.write(SCHEMA, schema);
        repo.write(DATA, OS_LIST);
        git(&repo.root, &["init", "-q", "-b", "main"]);
        git(&repo.root, &["add", "-A"]);
        git(&repo.root, &["commit", "-q", "-m", "fixture"]);
        repo
    }

    fn reading_data() -> Self {
        Self::new(&schema_with("load_data(\"../data/os.yaml\", \"string[]\")"))
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn context(&self, env: &[(&str, &str)]) -> biscuit_file::FileResolutionContext {
        let env: HashMap<String, String> = env.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect();
        biscuit_file::FileResolutionContext::new(&self.root).with_env(env)
    }

    /// The template as a fresh resolution reads it now.
    fn pending(&self) -> PendingSchema {
        let reference = serde_yaml_ng::Value::String(biscuit_file::to_portable_string(&self.path(SCHEMA)));
        match simplified_schema::resolve::resolve_yaml_schema(
            &reference,
            &self.root,
            &self.context(&[]),
            crate::markdown::schemas::embedded::SHIPPED_SOURCES,
        ) {
            Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("a pending handle"),
            other => panic!("expected a template, got {other:?}"),
        }
    }

    /// A new command's session, judging automatic grants against this
    /// repository.
    fn session(&self) -> Arc<SlotSession> {
        Arc::new(SlotSession::new(DataRuntime::new()).with_repository_root(Some(self.root.clone())))
    }

    /// A new command's session holding a run-only grant for the whole
    /// repository folder, so an edited (dirty) file may still be read.
    #[cfg(feature = "work-counters")]
    fn granted_session(&self, data: DataRuntime) -> Arc<SlotSession> {
        use crate::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
        let grant = Grant { decision: GrantDecision::Allow, target: GrantTarget::Dir(self.root.clone()), scope: GrantScope::Once };
        Arc::new(SlotSession::new(data).with_repository_root(Some(self.root.clone())).with_run_grants([grant]))
    }
}

fn resolve(session: &Arc<SlotSession>, repo: &Repo, env: &[(&str, &str)]) -> Result<Value, SchemaError> {
    session.resolve(&repo.pending(), "test-template", &repo.context(env), false).map(|resolved| resolved.json_schema.clone())
}

#[cfg(feature = "work-counters")]
fn members(schema: &Value) -> Value {
    schema["properties"]["os"]["enum"].clone()
}

/// The code of every slot failure `error` reports.
#[cfg(feature = "work-counters")]
fn codes(error: &SchemaError) -> Vec<&'static str> {
    use simplified_schema::slots::SlotResolutionError;
    let SchemaError::SlotResolution(failure) = error else {
        panic!("expected a slot resolution failure, got {error:?}");
    };
    match failure.error.as_ref() {
        SlotResolutionError::Failed(failed) => failed.iter().map(|(_, failure)| failure.code.unwrap_or("none")).collect(),
        other => panic!("expected failed slots, got {other:?}"),
    }
}

#[cfg(feature = "work-counters")]
fn counted<T>(work: crate::work_counters::Work, run: impl FnOnce() -> T) -> (T, u64) {
    let before = crate::work_counters::snapshot();
    let result = run();
    (result, crate::work_counters::snapshot().since(&before).get(work))
}

#[cfg(feature = "work-counters")]
fn evaluations<T>(run: impl FnOnce() -> T) -> (T, u64) {
    counted(crate::work_counters::Work::SlotResolution, run)
}

#[cfg(feature = "work-counters")]
mod reuse {
    use serde_json::json;

    use super::*;
    use crate::markdown::schemas::slots::codes::*;
    use crate::work_counters::Work;

    #[test]
    fn a_second_command_is_served_from_the_cache_without_evaluating_or_compiling() {
        let repo = Repo::reading_data();
        let cold_session = repo.session();
        let (cold, cold_runs) = evaluations(|| resolve(&cold_session, &repo, &[]).unwrap());
        assert_eq!(cold_runs, 1);
        assert_eq!(members(&cold), json!(["macOS", "Linux", "Windows"]));

        let warm_session = repo.session();
        let ((warm, runs), compiles) = counted(Work::ValidatorCompile, || evaluations(|| resolve(&warm_session, &repo, &[]).unwrap()));
        assert_eq!((runs, compiles), (0, 0));
        assert_eq!(warm, cold);
    }

    #[test]
    fn a_hit_reports_the_read_set_a_cold_run_reports() {
        let repo = Repo::reading_data();
        let cold_data = DataRuntime::new();
        let cold = Arc::new(SlotSession::new(cold_data.clone()).with_repository_root(Some(repo.root.clone())));
        resolve(&cold, &repo, &[]).unwrap();
        let warm_data = DataRuntime::new();
        let warm = Arc::new(SlotSession::new(warm_data.clone()).with_repository_root(Some(repo.root.clone())));
        let (_, runs) = evaluations(|| resolve(&warm, &repo, &[]).unwrap());
        assert_eq!(runs, 0);
        let expected = vec![biscuit_file::to_portable_string(&repo.path(DATA))];
        assert_eq!(cold_data.read_set(), expected);
        assert_eq!(warm_data.read_set(), expected);
    }

    #[test]
    fn one_command_pins_its_snapshot() {
        let repo = Repo::reading_data();
        let session = repo.session();
        let first = resolve(&session, &repo, &[]).unwrap();
        let (again, runs) = evaluations(|| resolve(&session, &repo, &[]).unwrap());
        assert_eq!((runs, again), (0, first));
    }

    #[test]
    fn an_edited_data_file_is_a_miss() {
        let repo = Repo::reading_data();
        resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap();
        repo.write(DATA, "- macOS\n- Linux\n");
        let (resolved, runs) = evaluations(|| resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap());
        assert_eq!(runs, 1);
        assert_eq!(members(&resolved), json!(["macOS", "Linux"]));
    }

    #[test]
    fn an_edited_schema_file_is_a_miss() {
        let repo = Repo::reading_data();
        resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap();
        repo.write(SCHEMA, &schema_with("load_data(\"../data/os.yaml\", \"string[]\") || []"));
        let (resolved, runs) = evaluations(|| resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap());
        assert_eq!(runs, 1);
        assert_eq!(members(&resolved), json!(["macOS", "Linux", "Windows"]));
    }

    #[test]
    fn a_cached_failure_is_served_until_the_missing_file_appears() {
        let repo = Repo::new(&schema_with("load_data(\"../data/missing.yaml\", \"string[]\")"));
        let failed = resolve(&repo.session(), &repo, &[]).unwrap_err();
        // A missing file keeps its typed not-found cause under the slot's code.
        let code = codes(&failed)[0];
        assert_eq!(code, SLOT_FAILED_CODE);

        let (again, runs) = evaluations(|| resolve(&repo.session(), &repo, &[]));
        assert_eq!(runs, 0, "the failure is cached with its missing probe");
        assert_eq!(codes(&again.unwrap_err()), vec![code]);

        repo.write("data/missing.yaml", "- Haiku\n");
        let (recovered, runs) = evaluations(|| resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]));
        assert_eq!(runs, 1);
        assert_eq!(members(&recovered.unwrap()), json!(["Haiku"]));
    }

    #[test]
    fn a_repaired_data_file_recovers() {
        let repo = Repo::reading_data();
        repo.write(DATA, "- [unclosed\n");
        let failed = resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap_err();
        assert!(codes(&failed)[0].starts_with("dm.data."), "{failed:?}");
        repo.write(DATA, OS_LIST);
        let (recovered, runs) = evaluations(|| resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]));
        assert_eq!(runs, 1);
        assert_eq!(members(&recovered.unwrap()), json!(["macOS", "Linux", "Windows"]));
    }

    #[test]
    fn a_changed_environment_input_is_a_miss() {
        let repo = Repo::new(&schema_with("[env.SANDBOX_OS]"));
        let first = resolve(&repo.session(), &repo, &[("SANDBOX_OS", "macOS")]).unwrap();
        assert_eq!(members(&first), json!(["macOS"]));
        let (_, runs) = evaluations(|| resolve(&repo.session(), &repo, &[("SANDBOX_OS", "macOS")]).unwrap());
        assert_eq!(runs, 0);
        let (changed, runs) = evaluations(|| resolve(&repo.session(), &repo, &[("SANDBOX_OS", "Linux")]).unwrap());
        assert_eq!((runs, members(&changed)), (1, json!(["Linux"])));
    }

    #[test]
    fn narrower_grants_are_not_served_a_result_broader_ones_produced() {
        let repo = Repo::reading_data();
        repo.write(DATA, "- macOS\n- Linux\n- Windows\n- FreeBSD\n");
        resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap();
        let (narrowed, runs) = evaluations(|| resolve(&repo.session(), &repo, &[]));
        assert_eq!(runs, 1, "the dirty file's read is not covered, so the template resolves again");
        assert_eq!(codes(&narrowed.unwrap_err()), vec![SLOT_APPROVAL_REQUIRED_CODE]);
    }

    /// A session whose grants do not cover a cached resolution's reads is
    /// refused it before any of its inputs is read again.
    #[test]
    fn an_uncovered_cache_entry_is_refused_before_its_inputs_are_reread() {
        let repo = Repo::reading_data();
        repo.write(DATA, "- macOS\n- Linux\n- Windows\n- FreeBSD\n");
        resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]).unwrap();
        repo.write(DATA, "- macOS\n- Linux\n- Windows\n- BeOS\n");
        let (narrowed, rehashes) = counted(Work::SlotInputRehash, || resolve(&repo.session(), &repo, &[]));
        assert_eq!(rehashes, 0, "the uncovered entry's data file was read again");
        assert_eq!(codes(&narrowed.unwrap_err()), vec![SLOT_APPROVAL_REQUIRED_CODE]);
    }

    #[test]
    fn a_volatile_slot_resolves_once_per_command() {
        let repo = Repo::new(&schema_with("[remote_vendor() || \"none\"]"));
        let session = repo.session();
        let (_, runs) = evaluations(|| resolve(&session, &repo, &[]).unwrap());
        assert_eq!(runs, 1);
        let (_, runs) = evaluations(|| resolve(&session, &repo, &[]).unwrap());
        assert_eq!(runs, 0, "pinned within the command");
        let (_, runs) = evaluations(|| resolve(&repo.session(), &repo, &[]).unwrap());
        assert_eq!(runs, 1, "never reused by another command");
    }

    #[test]
    fn an_approval_failure_is_never_cached() {
        let repo = Repo::reading_data();
        repo.write(DATA, "- macOS\n");
        assert_eq!(codes(&resolve(&repo.session(), &repo, &[]).unwrap_err()), vec![SLOT_APPROVAL_REQUIRED_CODE]);
        let (granted, runs) = evaluations(|| resolve(&repo.granted_session(DataRuntime::new()), &repo, &[]));
        assert_eq!((runs, members(&granted.unwrap())), (1, json!(["macOS"])));
    }

    /// Q7/Q17: after the run's own step rewrote the data file (the memo
    /// generation moved), the pin is re-checked and the template resolves
    /// again, keeping the automatic grant decided earlier in the run even
    /// though the file is now dirty.
    #[test]
    fn a_rewrite_during_the_run_re_resolves_under_the_grant_decided_earlier() {
        let repo = Repo::reading_data();
        let data = DataRuntime::new();
        let session = Arc::new(SlotSession::new(data.clone()).with_repository_root(Some(repo.root.clone())));
        resolve(&session, &repo, &[]).unwrap();
        repo.write(DATA, "- macOS\n- Linux\n- Windows\n- FreeBSD\n");
        data.invalidate_memo();
        let (resolved, runs) = evaluations(|| resolve(&session, &repo, &[]).unwrap());
        assert_eq!(runs, 1);
        assert_eq!(members(&resolved), json!(["macOS", "Linux", "Windows", "FreeBSD"]));
        // A command that did not decide the grant earlier needs approval.
        assert_eq!(codes(&resolve(&repo.session(), &repo, &[]).unwrap_err()), vec![SLOT_APPROVAL_REQUIRED_CODE]);
    }

    #[test]
    fn an_unrelated_write_keeps_the_pin() {
        let repo = Repo::reading_data();
        let data = DataRuntime::new();
        let session = Arc::new(SlotSession::new(data.clone()).with_repository_root(Some(repo.root.clone())));
        let first = resolve(&session, &repo, &[]).unwrap();
        repo.write("notes.txt", "unrelated\n");
        data.invalidate_memo();
        let (again, runs) = evaluations(|| resolve(&session, &repo, &[]).unwrap());
        assert_eq!((runs, again), (0, first));
    }

    #[test]
    fn a_failed_re_resolution_keeps_its_data_code_and_publishes_nothing() {
        let repo = Repo::reading_data();
        let data = DataRuntime::new();
        let session = repo.granted_session(data.clone());
        resolve(&session, &repo, &[]).unwrap();
        repo.write(DATA, "- [unclosed\n");
        data.invalidate_memo();
        let failed = resolve(&session, &repo, &[]).unwrap_err();
        assert!(codes(&failed)[0].starts_with("dm.data."), "{failed:?}");
    }
}

#[test]
fn a_passive_session_resolves_nothing() {
    let repo = Repo::reading_data();
    let data = DataRuntime::new();
    let session = Arc::new(SlotSession::new(data.clone()).with_mode(SessionMode::Passive));
    assert!(matches!(resolve(&session, &repo, &[]), Err(SchemaError::UnresolvedSlots(_))));
    assert!(data.read_set().is_empty());
}

#[test]
fn a_pins_only_pass_uses_only_a_pinned_snapshot() {
    let repo = Repo::reading_data();
    let session = repo.session();
    let pending = repo.pending();
    let context = repo.context(&[]);
    assert!(matches!(session.resolve(&pending, "t", &context, true), Err(SchemaError::UnresolvedSlots(_))));
    let pinned = session.resolve(&pending, "t", &context, false).unwrap();
    assert_eq!(session.resolve(&pending, "t", &context, true).unwrap().json_schema, pinned.json_schema);
}

/// Q19: across commands an unchanged `stat` stands for unchanged bytes, and
/// a changed `stat` makes the check rehash.
#[test]
fn a_cross_command_check_trusts_an_unchanged_stat_and_rehashes_a_changed_one() {
    let temp = tempfile::tempdir().unwrap();
    let path = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("os.yaml");
    std::fs::write(&path, "abc\n").unwrap();
    let observation = Observation {
        entry: biscuit_file::to_portable_string(&path),
        kind: ObservationKind::File,
        hash: Some(biscuit_hash::blake3_hash_bytes(b"abc\n")),
    };
    let check = InputCheck::capture(observation);
    let stamp = std::fs::metadata(&path).unwrap().modified().unwrap();

    // Same size, same modification time, different bytes: trusted by stat
    // (the documented limitation), caught by a rehash.
    std::fs::write(&path, "xyz\n").unwrap();
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(stamp).unwrap();
    assert!(check.holds(true));
    assert!(!check.holds(false));

    // A changed modification time rehashes: equal bytes still hold.
    std::fs::write(&path, "abc\n").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(stamp + std::time::Duration::from_secs(5))
        .unwrap();
    assert!(check.holds(true));
    std::fs::write(&path, "abd\n").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(stamp + std::time::Duration::from_secs(9))
        .unwrap();
    assert!(!check.holds(true));
}

/// An editor buffer open for a data file replaces the file, and its edits
/// leave the file's `stat` unchanged: the check rehashes the buffer instead
/// of trusting the `stat`, so a cached resolution of the old text misses.
#[test]
fn an_open_buffer_with_other_text_fails_the_check_though_the_stat_is_unchanged() {
    use simplified_schema::resolve::buffers::{self, SourceBuffers};
    let temp = tempfile::tempdir().unwrap();
    let path = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("os.yaml");
    std::fs::write(&path, "- Linux\n").unwrap();
    let check = InputCheck::capture(Observation {
        entry: biscuit_file::to_portable_string(&path),
        kind: ObservationKind::File,
        hash: Some(biscuit_hash::blake3_hash_bytes(b"- Linux\n")),
    });
    assert!(check.holds(true), "control: nothing changed");
    let mut edited = SourceBuffers::default();
    edited.insert(&path, std::sync::Arc::from("- Linux\n- Haiku\n"));
    assert!(!buffers::with(&edited, || check.holds(true)), "the unsaved edit is a change");
    let mut same = SourceBuffers::default();
    same.insert(&path, std::sync::Arc::from("- Linux\n"));
    assert!(buffers::with(&same, || check.holds(true)), "a buffer equal to the read bytes holds");
}

/// A read of an open buffer captures no `stat`: the file's `stat` says
/// nothing about the buffer's bytes, so once the buffer closes the check
/// rehashes the disk file and a resolution of the unsaved text misses.
#[test]
fn a_read_of_an_open_buffer_is_rehashed_once_the_buffer_closes() {
    use simplified_schema::resolve::buffers::{self, SourceBuffers};
    let temp = tempfile::tempdir().unwrap();
    let path = biscuit_file::canonicalize_simplified(temp.path()).unwrap().join("os.toml");
    std::fs::write(&path, "os = [\"Linux\"]\n").unwrap();
    let unsaved = "os = [\"Linux\", \"Haiku\"]\n";
    let mut open = SourceBuffers::default();
    open.insert(&path, std::sync::Arc::from(unsaved));
    let check = buffers::with(&open, || {
        InputCheck::capture(Observation {
            entry: biscuit_file::to_portable_string(&path),
            kind: ObservationKind::File,
            hash: Some(biscuit_hash::blake3_hash_bytes(unsaved.as_bytes())),
        })
    });
    assert!(buffers::with(&open, || check.holds(true)), "control: the buffer is unchanged");
    assert!(!check.holds(true), "the closed buffer leaves the disk file, which differs");
}

/// R2-F1: what a resolution consumed besides the files it read decides
/// whether another command may be served it. Each case resolves in one
/// command, changes exactly one captured observation, executable, or Git
/// remote, and resolves in a new command (the cache path); a cold control
/// resolves the changed request in a fresh repository, where nothing is
/// cached.
#[cfg(feature = "work-counters")]
mod consumed {
    use chrono::TimeZone;
    use serde_json::json;
    use sniff::filesystem::repo::{Package, RepoInfo};
    use sniff::network::ScopedIpAddr;
    use sniff::programs::{AiCli, ExecutableSource, InstalledAiClients};

    use super::*;
    use crate::markdown::compose::context::capture::{CapturedObservations, ClockReading};

    /// A schema whose `os` enum is the slot `expression`, written in a
    /// double-quoted YAML scalar so the expression may hold ` : `.
    fn quoted_schema(expression: &str) -> String {
        format!("kind: schema\n$schema:\n    os: \"enum({{{{ {expression} }}}}; required)\"\n")
    }

    /// A new command's session observing `observations`.
    fn observing(repo: &Repo, observations: CapturedObservations) -> Arc<SlotSession> {
        let mut session = SlotSession::new(DataRuntime::new()).with_repository_root(Some(repo.root.clone()));
        session.observations = Arc::new(crate::markdown::schemas::slots::observations::SessionObservations::supplied(observations));
        Arc::new(session)
    }

    fn outcome(session: &Arc<SlotSession>, repo: &Repo) -> Result<Value, SchemaError> {
        resolve(session, repo, &[]).map(|schema| members(&schema))
    }

    fn members_in(session: &Arc<SlotSession>, repo: &Repo) -> Value {
        outcome(session, repo).unwrap_or_else(|error| panic!("resolution failed: {error:?}"))
    }

    /// The captured-observation case: `before` resolves to `old`, a second
    /// command with the same observations is served from the cache, a
    /// command observing `after` resolves again to `new`, and so does a cold
    /// resolution observing `after`.
    fn assert_captured_change(
        schema: &str,
        before: impl Fn() -> CapturedObservations,
        after: impl Fn() -> CapturedObservations,
        old: Value,
        new: Value,
    ) {
        let repo = Repo::new(schema);
        assert_eq!(members_in(&observing(&repo, before()), &repo), old);
        let (again, runs) = evaluations(|| members_in(&observing(&repo, before()), &repo));
        assert_eq!((runs, again), (0, old), "unchanged observations are a hit");
        let (changed, runs) = evaluations(|| members_in(&observing(&repo, after()), &repo));
        assert_eq!((runs, changed), (1, new.clone()), "a changed observation is a miss");

        let cold = Repo::new(schema);
        assert_eq!(members_in(&observing(&cold, after()), &cold), new, "cold control");
    }

    fn clock_on(day: u32) -> CapturedObservations {
        let utc = chrono::Utc.with_ymd_and_hms(2024, 6, day, 12, 0, 0).unwrap();
        CapturedObservations::for_test_clock(ClockReading::at(utc, chrono::FixedOffset::east_opt(0).unwrap()))
    }

    /// R2-F1.1: every date function reads the clock through one dispatcher.
    #[test]
    fn a_new_captured_day_is_a_miss_for_a_date_function() {
        assert_captured_change(
            &quoted_schema("is_today('2024-06-16') ? ['macOS'] : ['Windows']"),
            || clock_on(16),
            || clock_on(17),
            json!(["macOS"]),
            json!(["Windows"]),
        );
    }

    /// The same captured day at another time answers the same, so it is a
    /// hit: the re-check compares the answer, not the instant.
    #[test]
    fn another_time_on_the_same_captured_day_is_a_hit() {
        let repo = Repo::new(&quoted_schema("is_today('2024-06-16') ? ['macOS'] : ['Windows']"));
        assert_eq!(members_in(&observing(&repo, clock_on(16)), &repo), json!(["macOS"]));
        let later = || {
            let utc = chrono::Utc.with_ymd_and_hms(2024, 6, 16, 18, 30, 0).unwrap();
            CapturedObservations::for_test_clock(ClockReading::at(utc, chrono::FixedOffset::east_opt(0).unwrap()))
        };
        let (again, runs) = evaluations(|| members_in(&observing(&repo, later()), &repo));
        assert_eq!((runs, again), (0, json!(["macOS"])));
    }

    fn addresses(texts: &[&str]) -> CapturedObservations {
        let parsed: Vec<ScopedIpAddr> = texts.iter().map(|text| text.parse().unwrap()).collect();
        CapturedObservations::for_test_addresses(&parsed)
    }

    /// R2-F1.4.
    #[test]
    fn changed_captured_ipv4_addresses_are_a_miss() {
        assert_captured_change(
            &schema_with("ipv4()"),
            || addresses(&["10.1.1.1"]),
            || addresses(&["10.1.1.2"]),
            json!(["10.1.1.1"]),
            json!(["10.1.1.2"]),
        );
    }

    /// R2-F1.5.
    #[test]
    fn changed_captured_ipv6_addresses_are_a_miss() {
        assert_captured_change(
            &schema_with("ipv6()"),
            || addresses(&["2001:db8::1"]),
            || addresses(&["2001:db8::2"]),
            json!(["2001:db8::1"]),
            json!(["2001:db8::2"]),
        );
    }

    /// A repository whose one package is `data/child`, named `name`, in the
    /// area `area`.
    fn packages(root: &Path, name: &str, area: &str) -> CapturedObservations {
        let package = Package {
            name: name.to_owned(),
            relative: "data/child".to_owned(),
            package_area: area.to_owned(),
            path: root.join("data").join("child"),
            ..Package::default()
        };
        let info = RepoInfo { is_monorepo: true, root: root.to_path_buf(), packages: Some(vec![package]), ..RepoInfo::default() };
        CapturedObservations::for_test_packages(root, &info)
    }

    /// [`assert_captured_change`] for a package lookup, whose observations
    /// name paths under the fixture's root.
    fn assert_package_change(function: &str, old: (&str, &str), new: (&str, &str), expect_old: &str, expect_new: &str) {
        let schema = schema_with(&format!("[{function}(\"../data/child/os.yaml\")]"));
        let repo = Repo::new(&schema);
        let root = repo.root.clone();
        assert_eq!(members_in(&observing(&repo, packages(&root, old.0, old.1)), &repo), json!([expect_old]));
        let (again, runs) = evaluations(|| members_in(&observing(&repo, packages(&root, old.0, old.1)), &repo));
        assert_eq!((runs, again), (0, json!([expect_old])), "an unchanged lookup is a hit");
        let (changed, runs) = evaluations(|| members_in(&observing(&repo, packages(&root, new.0, new.1)), &repo));
        assert_eq!((runs, changed), (1, json!([expect_new])), "a changed lookup is a miss");

        let cold = Repo::new(&schema);
        let cold_root = cold.root.clone();
        assert_eq!(members_in(&observing(&cold, packages(&cold_root, new.0, new.1)), &cold), json!([expect_new]), "cold control");
    }

    /// R2-F1.6.
    #[test]
    fn a_changed_captured_package_is_a_miss() {
        assert_package_change("package", ("alpha", "data"), ("beta", "data"), "alpha", "beta");
    }

    /// R2-F1.7.
    #[test]
    fn a_changed_captured_package_area_is_a_miss() {
        assert_package_change("package_area", ("alpha", "data"), ("alpha", "data/child"), "data", "data/child");
    }

    /// R2-F1.8.
    #[test]
    fn a_changed_agentic_cli_scan_is_a_miss() {
        let kimi = || {
            CapturedObservations::for_test_agentic_clis(InstalledAiClients::default().with_program(
                AiCli::KimiCli,
                PathBuf::from("/fixture/bin/kimi"),
                ExecutableSource::Path,
            ))
        };
        assert_captured_change(
            &quoted_schema("[has_agentic_cli('kimi') ? 'present' : 'absent']"),
            kimi,
            || CapturedObservations::for_test_agentic_clis(InstalledAiClients::default()),
            json!(["present"]),
            json!(["absent"]),
        );
    }

    /// An executable file at a portable absolute path under `dir`.
    fn executable(dir: &Path) -> String {
        let path = dir.join(if cfg!(windows) { "fixture-tool.exe" } else { "fixture-tool" });
        std::fs::write(&path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        biscuit_file::to_portable_string(&path)
    }

    /// R2-F1.2 and R2-F1.3: `has_command` and `has_binary` share one probe;
    /// an executable removed between commands, with the same argument and
    /// `PATH`, is a miss.
    fn assert_executable_removal(function: &str) {
        let tools = tempfile::tempdir().unwrap();
        let tool = executable(tools.path());
        let schema = quoted_schema(&format!("[{function}('{tool}') ? 'installed' : 'missing']"));
        let repo = Repo::new(&schema);
        assert_eq!(members_in(&repo.session(), &repo), json!(["installed"]));
        let (again, runs) = evaluations(|| members_in(&repo.session(), &repo));
        assert_eq!((runs, again), (0, json!(["installed"])), "a present executable is a hit");
        std::fs::remove_file(tools.path().join(tool.rsplit('/').next().unwrap())).unwrap();
        let (changed, runs) = evaluations(|| members_in(&repo.session(), &repo));
        assert_eq!((runs, changed), (1, json!(["missing"])), "a removed executable is a miss");

        let cold = Repo::new(&schema);
        assert_eq!(members_in(&cold.session(), &cold), json!(["missing"]), "cold control");
    }

    #[test]
    fn a_removed_executable_is_a_miss_for_has_command() {
        assert_executable_removal("has_command");
    }

    #[test]
    fn a_removed_executable_is_a_miss_for_has_binary() {
        assert_executable_removal("has_binary");
    }

    /// A schema loading the OS list under `Os@./types.yaml`, whose pattern
    /// is the slot `pattern`.
    fn contract_repo(pattern: &str) -> Repo {
        let repo = Repo::new(&schema_with("load_data(\"../data/os.yaml\", \"Os@./types.yaml\")"));
        repo.write("schemas/types.yaml", &format!("kind: schema\ntypes:\n  Os: \"string(pattern({{{{ {pattern} }}}}))[]\"\n"));
        git(&repo.root, &["add", "-A"]);
        git(&repo.root, &["commit", "-q", "-m", "types"]);
        repo
    }

    /// R2-F1.9: the captured day a nested contract's slot read is part of
    /// the outer resolution.
    #[test]
    fn a_nested_contracts_captured_day_is_part_of_the_outer_entry() {
        let pattern = "is_today('2024-06-16') ? '.*' : '^Windows$'";
        let repo = contract_repo(pattern);
        assert_eq!(members_in(&observing(&repo, clock_on(16)), &repo), json!(["macOS", "Linux", "Windows"]));
        let (again, runs) = evaluations(|| members_in(&observing(&repo, clock_on(16)), &repo));
        assert_eq!((runs, again), (0, json!(["macOS", "Linux", "Windows"])), "the same day is a hit");
        let (changed, runs) = evaluations(|| outcome(&observing(&repo, clock_on(17)), &repo));
        assert_eq!(runs, 2, "a new day evaluates the slot and its contract again");
        assert!(changed.is_err(), "the new day's contract rejects the data: {changed:?}");

        let cold = contract_repo(pattern);
        assert!(outcome(&observing(&cold, clock_on(17)), &cold).is_err(), "cold control");
    }

    fn set_origin(repo: &Repo, url: &str) {
        let _ = Command::new("git").args(["remote", "remove", "origin"]).current_dir(&repo.root).output();
        git(&repo.root, &["remote", "add", "origin", url]);
    }

    /// R2-F1.10: a volatile call in a nested contract keeps the outer
    /// resolution to its command.
    #[test]
    fn a_nested_contracts_volatile_call_is_never_served_to_another_command() {
        let pattern = "remote_vendor() == 'github' ? '.*' : '^Windows$'";
        let repo = contract_repo(pattern);
        set_origin(&repo, "https://github.com/fixture/sandbox.git");
        assert_eq!(members_in(&repo.session(), &repo), json!(["macOS", "Linux", "Windows"]));
        set_origin(&repo, "https://gitlab.com/fixture/sandbox.git");
        let (changed, runs) = evaluations(|| outcome(&repo.session(), &repo));
        assert_eq!(runs, 2, "another command evaluates the slot and its contract again");
        assert!(changed.is_err(), "the GitLab origin's contract rejects the data: {changed:?}");

        let cold = contract_repo(pattern);
        set_origin(&cold, "https://gitlab.com/fixture/sandbox.git");
        assert!(outcome(&cold.session(), &cold).is_err(), "cold control");
    }

    /// A contract served from the command's data memo still makes the
    /// enclosing resolution depend on what the contract's slots consumed:
    /// the same template under a second identity, resolved later in the same
    /// command, loads through the memoized contract, and that resolution is
    /// not served to a command on another day.
    #[test]
    fn a_memoized_nested_contract_replays_what_its_slots_consumed() {
        let repo = contract_repo("is_today('2024-06-16') ? '.*' : '^Windows$'");
        let resolve_as = |session: &Arc<SlotSession>, template: &str| {
            session.resolve(&repo.pending(), template, &repo.context(&[]), false).map(|resolved| members(&resolved.json_schema))
        };
        let first = observing(&repo, clock_on(16));
        resolve_as(&first, "first-template").unwrap();
        let (served, runs) = evaluations(|| resolve_as(&first, "second-template").unwrap());
        assert_eq!(served, json!(["macOS", "Linux", "Windows"]));
        assert_eq!(runs, 1, "only the outer slot ran: the contract came from the memo");
        let (changed, runs) = evaluations(|| resolve_as(&observing(&repo, clock_on(17)), "second-template"));
        assert_eq!(runs, 2, "a new day resolves the template and its contract again");
        assert!(changed.is_err(), "the new day's contract rejects the data: {changed:?}");
    }

    /// R2-F1.11: a volatile call in a root-union arm that resolution reaches
    /// only in a later round keeps the result to its command.
    #[test]
    fn a_later_root_union_rounds_volatile_call_is_never_served_to_another_command() {
        let schema = "kind: schema\n$schema:\n  - kind: literal(a)\n    os: enum({{ ['macOS'] }})\n  - ./arm.yaml\n";
        let arm = "$schema:\n  kind: literal(b)\n  os: enum({{ [remote_vendor()] }}; required)\n";
        let build = || {
            let repo = Repo::new(schema);
            repo.write("schemas/arm.yaml", arm);
            git(&repo.root, &["add", "-A"]);
            git(&repo.root, &["commit", "-q", "-m", "arm"]);
            repo
        };
        let vendor = |session: &Arc<SlotSession>, repo: &Repo| resolve(session, repo, &[]).unwrap().to_string();
        let repo = build();
        set_origin(&repo, "https://github.com/fixture/sandbox.git");
        assert!(vendor(&repo.session(), &repo).contains("github"));
        set_origin(&repo, "https://gitlab.com/fixture/sandbox.git");
        let (changed, runs) = evaluations(|| vendor(&repo.session(), &repo));
        assert_eq!(runs, 2, "another command evaluates both arms again");
        assert!(changed.contains("gitlab") && !changed.contains("github"), "{changed}");

        let cold = build();
        set_origin(&cold, "https://gitlab.com/fixture/sandbox.git");
        assert!(vendor(&cold.session(), &cold).contains("gitlab"), "cold control");
    }
}

/// R3-F1: a nested contract's reads keep the contract file that defined
/// them, so a cached outer result is served to another command only when
/// that command's grants allow each read for the contract's own file. The
/// shared contract lives outside the repository (no automatic grant, Q13);
/// the outer schema is committed inside it (automatic grant). Each case
/// resolves with broad one-run grants, then from a new command granting only
/// the shared folder (warm), then the same narrowed request on a fresh
/// fixture (cold); warm and cold must agree.
#[cfg(feature = "work-counters")]
mod nested_grants {
    use serde_json::json;

    use super::*;
    use crate::markdown::schemas::slots::codes::*;
    use crate::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};

    const PATTERN: &str = "data/pattern.yaml";

    /// The repository, plus a shared folder beside it (outside it).
    struct Shared {
        repo: Repo,
        shared: PathBuf,
    }

    impl Shared {
        /// A committed repository whose schema loads its OS list under
        /// `Os@<shared>/types.yaml`; `types` (and the other `files`) are
        /// written into the shared folder, after `{repo}` and `{shared}` are
        /// replaced with those folders' absolute paths.
        fn new(types: &str, files: &[(&str, &str)]) -> Self {
            let probe = Repo::new("kind: schema\n");
            let shared = probe.root.parent().unwrap().join("shared");
            let fill = |text: &str| {
                text.replace("{repo}", &biscuit_file::to_portable_string(&probe.root))
                    .replace("{shared}", &biscuit_file::to_portable_string(&shared))
            };
            let contract = format!("Os@{}", biscuit_file::to_portable_string(&shared.join("types.yaml")));
            probe.write(SCHEMA, &schema_with(&format!("load_data(\"../data/os.yaml\", \"{contract}\")")));
            probe.write(PATTERN, "- '.*'\n");
            git(&probe.root, &["add", "-A"]);
            git(&probe.root, &["commit", "-q", "-m", "shared contract"]);
            std::fs::create_dir_all(&shared).unwrap();
            std::fs::write(shared.join("types.yaml"), fill(types)).unwrap();
            for (name, text) in files {
                std::fs::write(shared.join(name), fill(text)).unwrap();
            }
            Self { repo: probe, shared }
        }

        fn session(&self, targets: Vec<GrantTarget>) -> Arc<SlotSession> {
            let grants = targets
                .into_iter()
                .map(|target| Grant { decision: GrantDecision::Allow, target, scope: GrantScope::Once });
            Arc::new(SlotSession::new(DataRuntime::new()).with_repository_root(Some(self.repo.root.clone())).with_run_grants(grants))
        }

        /// One-run grants for the shared folder and `extra`.
        fn broad(&self, extra: GrantTarget) -> Arc<SlotSession> {
            self.session(vec![GrantTarget::Dir(self.shared.clone()), extra])
        }

        /// A one-run grant for the shared folder only.
        fn narrow(&self) -> Arc<SlotSession> {
            self.session(vec![GrantTarget::Dir(self.shared.clone())])
        }

        fn pattern_file(&self) -> GrantTarget {
            GrantTarget::File(self.repo.path(PATTERN))
        }

        fn resolve_as(&self, session: &Arc<SlotSession>, template: &str) -> Result<Value, SchemaError> {
            session
                .resolve(&self.repo.pending(), template, &self.repo.context(&[]), false)
                .map(|resolved| members(&resolved.json_schema))
        }
    }

    fn assert_refused(outcome: Result<Value, SchemaError>, what: &str) {
        match outcome {
            Ok(value) => panic!("{what}: expected a refusal, got {value}"),
            Err(error) => assert_eq!(codes(&error), vec![SLOT_APPROVAL_REQUIRED_CODE], "{what}"),
        }
    }

    /// Broad, then narrowed warm, then narrowed cold, for the shared
    /// contract `types` (and `files`), whose reads `extra` grants.
    fn assert_nested_read_needs_its_grant(types: &str, files: &[(&str, &str)], extra: impl Fn(&Shared) -> GrantTarget) {
        let fixture = Shared::new(types, files);
        let granted = fixture.resolve_as(&fixture.broad(extra(&fixture)), "test-template");
        assert_eq!(granted.unwrap(), json!(["macOS", "Linux", "Windows"]), "the broad grants resolve");
        assert_refused(fixture.resolve_as(&fixture.narrow(), "test-template"), "warm");

        let cold = Shared::new(types, files);
        assert_refused(cold.resolve_as(&cold.narrow(), "test-template"), "cold control");
    }

    const CONTENT: &str = "kind: schema\ntypes:\n  Os: |-\n    string(pattern({{ load_data(\"{repo}/data/pattern.yaml\", \"string[]\")[0] }}))[]\n";
    const PROBE: &str =
        "kind: schema\ntypes:\n  Os: |-\n    string(pattern({{ file_exists(\"{repo}/data/pattern.yaml\") ? '.*' : '^Windows$' }}))[]\n";

    /// R3-F1.1, .5, .6: a nested content read.
    #[test]
    fn a_nested_contracts_content_read_needs_its_own_grant_warm_and_cold() {
        assert_nested_read_needs_its_grant(CONTENT, &[], Shared::pattern_file);
    }

    /// R3-F1.2: a nested existence probe.
    #[test]
    fn a_nested_contracts_existence_probe_needs_its_own_grant_warm_and_cold() {
        assert_nested_read_needs_its_grant(PROBE, &[], Shared::pattern_file);
    }

    /// R3-F1.3: a contract nested inside the shared contract.
    #[test]
    fn a_deeper_nested_contracts_probe_needs_its_own_grant_warm_and_cold() {
        let types = "kind: schema\ntypes:\n  Os: |-\n    string(pattern({{ load_data(\"{repo}/data/pattern.yaml\", \"Pattern@./inner.yaml\")[0] }}))[]\n";
        let inner =
            "kind: schema\ntypes:\n  Pattern: |-\n    string(pattern({{ file_exists(\"{repo}/data/pattern.yaml\") ? '.*' : '^Windows$' }}))[]\n";
        assert_nested_read_needs_its_grant(types, &[("inner.yaml", inner)], Shared::pattern_file);
    }

    /// R3-F1.4: a contract served from the command's memo replays its
    /// reads with the contract's file.
    #[test]
    fn a_memoized_nested_contract_replays_its_reads_for_another_command() {
        let fixture = Shared::new(CONTENT, &[]);
        let broad = fixture.broad(fixture.pattern_file());
        fixture.resolve_as(&broad, "first-template").unwrap();
        let (served, runs) = evaluations(|| fixture.resolve_as(&broad, "second-template"));
        assert_eq!(served.unwrap(), json!(["macOS", "Linux", "Windows"]));
        assert_eq!(runs, 1, "only the outer slot ran: the contract came from the memo");
        assert_refused(fixture.resolve_as(&fixture.narrow(), "second-template"), "warm");

        let cold = Shared::new(CONTENT, &[]);
        assert_refused(cold.resolve_as(&cold.narrow(), "second-template"), "cold control");
    }

    /// R3-F1.7 control: a template outside the repository read directly.
    #[test]
    fn a_shared_template_read_directly_needs_its_grant_warm_and_cold() {
        let direct = "kind: schema\n$schema:\n    os: enum({{ load_data(\"{repo}/data/os.yaml\", \"string[]\") }}; required)\n";
        let run = |fixture: &Shared, session: &Arc<SlotSession>| {
            let path = fixture.shared.join("direct.yaml");
            let reference = serde_yaml_ng::Value::String(biscuit_file::to_portable_string(&path));
            let pending = match simplified_schema::resolve::resolve_yaml_schema(
                &reference,
                &fixture.repo.root,
                &fixture.repo.context(&[]),
                crate::markdown::schemas::embedded::SHIPPED_SOURCES,
            ) {
                Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved.pending.expect("a pending handle"),
                other => panic!("expected a template, got {other:?}"),
            };
            session.resolve(&pending, "direct-template", &fixture.repo.context(&[]), false).map(|resolved| members(&resolved.json_schema))
        };
        let fixture = Shared::new(CONTENT, &[("direct.yaml", direct)]);
        let os = GrantTarget::File(fixture.repo.path(DATA));
        assert_eq!(run(&fixture, &fixture.broad(os)).unwrap(), json!(["macOS", "Linux", "Windows"]));
        assert_refused(run(&fixture, &fixture.narrow()), "warm");

        let cold = Shared::new(CONTENT, &[("direct.yaml", direct)]);
        assert_refused(run(&cold, &cold.narrow()), "cold control");
    }

    /// R3-F1.10 control: a nested listing needs an explicit grant.
    #[test]
    fn a_nested_contracts_listing_needs_an_explicit_grant_warm_and_cold() {
        let listing = "kind: schema\ntypes:\n  Os: |-\n    string(pattern({{ length(find_files(\"{repo}/data/*.yaml\")) > 0 ? '.*' : '^Windows$' }}))[]\n";
        assert_nested_read_needs_its_grant(listing, &[], |fixture| GrantTarget::Dir(fixture.repo.path("data")));
    }
}
