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
