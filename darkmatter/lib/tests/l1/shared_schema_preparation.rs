//! Effective schemas prepared once per observation of local files (L1, in
//! process): while a [`SharedReads`] serves every schema read on the
//! thread, as it does in a dry run's passes, [`DarkmatterSchemas::effective_for`]
//! prepares each distinct `(document layer, context, matched triggers,
//! baseline)` once and serves later requests from that preparation, even
//! from a new `DarkmatterSchemas` (each pass builds its own). Each request
//! still judges its own values, a different schema file is its own
//! preparation, a template's preparation (it depends on the slot session)
//! is never reused, and outside a shared observation every request prepares.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, DataRuntime};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::slots::SlotSession;
use darkmatter::markdown::schemas::slots::grants::{Grant, GrantDecision, GrantScope, GrantTarget};
use darkmatter::work_counters::{Work, WorkSnapshot, snapshot};
use simplified_schema::resolve::buffers::{SharedReads, with_shared_reads};

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let fixture = Self { _temp: temp, root };
        fixture.write("s.yaml", "$schema:\n  count: number(min(1))\n");
        fixture.write("t.yaml", "$schema:\n  count: number(max(2))\n");
        fixture.write("a.yaml", "kind: schema\ntypes:\n  Person:\n    name: string(required)\n");
        fixture.write("b.yaml", "kind: schema\ntypes:\n  Person:\n    age: number(required)\n");
        fixture.write(
            "slot.md",
            "---\n$schema:\n  owner: \"{{ env.KIND == 'a' ? 'Person@./a.yaml' : 'Person@./b.yaml' }}\"\n---\nBody.\n",
        );
        fixture
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.root.join(name), text).expect("write");
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn context(&self, env: &[(&str, &str)]) -> FileResolutionContext {
        FileResolutionContext::new(&self.root)
            .with_env(env.iter().map(|(name, value)| ((*name).to_string(), (*value).to_string())).collect::<HashMap<_, _>>())
    }

    /// `name` written as a document whose `$schema` is `schema` and whose
    /// `count` is `count`.
    fn document(&self, name: &str, schema: &str, count: i64) -> Markdown {
        self.write(name, &format!("---\n$schema: {schema}\ncount: {count}\n---\nBody.\n"));
        self.markdown(name)
    }

    fn markdown(&self, name: &str) -> Markdown {
        let path = self.path(name);
        Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path))
    }
}

/// The preparations and reuses `run` makes.
fn counted(run: impl FnOnce()) -> (u64, u64) {
    let before = snapshot();
    run();
    let work: WorkSnapshot = snapshot().since(&before);
    (work.get(Work::EffectivePreparation), work.get(Work::EffectivePreparationReuse))
}

/// Whether `markdown`'s values satisfy its effective schema, judged by a
/// pass of its own.
fn valid(fixture: &Fixture, markdown: &Markdown) -> bool {
    DarkmatterSchemas::new(fixture.context(&[])).validate(markdown).expect("a schema").valid
}

#[test]
fn one_or_two_occurrences_of_one_schema_are_prepared_once_and_judged_apart() {
    for occurrences in [1, 2] {
        let fixture = Fixture::new();
        let good = fixture.document("good.md", "./s.yaml", 4);
        let shared = SharedReads::default();
        let (prepared, reused) = counted(|| {
            with_shared_reads(&shared, || {
                // Four passes over each occurrence, each with its own
                // `DarkmatterSchemas`, as a dry run's passes are.
                for _ in 0..occurrences {
                    for _ in 0..4 {
                        assert!(valid(&fixture, &good));
                    }
                }
            });
        });
        assert_eq!((prepared, reused), (1, 4 * occurrences - 1), "{occurrences} occurrence(s)");
    }
}

#[test]
fn each_occurrence_keeps_its_own_verdict() {
    let fixture = Fixture::new();
    let good = fixture.document("good.md", "./s.yaml", 4);
    let mut bad = good.clone();
    bad.frontmatter_mut().as_map_mut().insert("count".to_string(), serde_json::json!(0));
    let shared = SharedReads::default();
    let (prepared, _) = counted(|| {
        with_shared_reads(&shared, || {
            assert!(valid(&fixture, &good), "the good occurrence passes");
            let report = DarkmatterSchemas::new(fixture.context(&[])).validate(&bad).expect("a schema");
            assert!(!report.valid, "the bad occurrence is refused by the reused schema");
            assert_eq!(report.problems.len(), 1, "{:?}", report.problems);
            assert!(valid(&fixture, &good), "and the good one still passes");
        });
    });
    assert_eq!(prepared, 1, "one preparation serves both verdicts");
}

#[test]
fn a_different_schema_file_is_its_own_preparation() {
    let fixture = Fixture::new();
    let first = fixture.document("first.md", "./s.yaml", 2);
    let second = fixture.document("second.md", "./t.yaml", 2);
    let shared = SharedReads::default();
    let (prepared, reused) = counted(|| {
        with_shared_reads(&shared, || {
            for _ in 0..2 {
                assert!(valid(&fixture, &first));
                assert!(valid(&fixture, &second));
            }
        });
    });
    assert_eq!((prepared, reused), (2, 2));
    let mut over = second.clone();
    over.frontmatter_mut().as_map_mut().insert("count".to_string(), serde_json::json!(3));
    assert!(!with_shared_reads(&shared, || valid(&fixture, &over)), "t.yaml's own limit applies");
}

#[test]
fn a_template_resolving_to_two_files_is_prepared_for_each_occurrence() {
    let fixture = Fixture::new();
    let slot = fixture.markdown("slot.md");
    let grants = || {
        [fixture.path("a.yaml"), fixture.path("b.yaml")]
            .map(|path| Grant { decision: GrantDecision::Allow, target: GrantTarget::File(path), scope: GrantScope::Once })
    };
    let owner = |kind: &str| {
        let session = SlotSession::new(DataRuntime::new()).with_run_grants(grants());
        let schemas = DarkmatterSchemas::new(fixture.context(&[("KIND", kind)])).with_slot_session(Arc::new(session));
        let effective = schemas.effective_for(&slot).expect("resolves").expect("a schema");
        let text = effective.json_schema.to_string();
        match (text.contains("\"name\""), text.contains("\"age\"")) {
            (true, false) => "a.yaml",
            (false, true) => "b.yaml",
            _ => panic!("{text}"),
        }
    };
    let shared = SharedReads::default();
    let (prepared, reused) = counted(|| {
        with_shared_reads(&shared, || {
            assert_eq!(owner("a"), "a.yaml");
            assert_eq!(owner("b"), "b.yaml");
            assert_eq!(owner("a"), "a.yaml", "a repeat resolves its own slot again");
        });
    });
    assert_eq!((prepared, reused), (3, 0), "a template's preparation depends on its session, so none is reused");
}

#[test]
fn without_a_shared_observation_every_request_prepares() {
    let fixture = Fixture::new();
    let good = fixture.document("good.md", "./s.yaml", 4);
    let (prepared, reused) = counted(|| {
        for _ in 0..3 {
            assert!(valid(&fixture, &good));
        }
    });
    assert_eq!((prepared, reused), (3, 0));
}

#[test]
fn a_schema_read_a_guard_judges_is_prepared_again() {
    let fixture = Fixture::new();
    let good = fixture.document("good.md", "./s.yaml", 4);
    let shared = SharedReads::default();
    let refused = fixture.path("s.yaml");
    let (prepared, reused) = counted(|| {
        with_shared_reads(&shared, || {
            assert!(valid(&fixture, &good));
            // A host's read guard sees every read, so the memo stands aside
            // and the guard's refusal is the result.
            let guard: simplified_schema::resolve::buffers::ReadGuard = std::rc::Rc::new(move |path: &Path| path != refused);
            let guarded = simplified_schema::resolve::buffers::guarded(guard, || DarkmatterSchemas::new(fixture.context(&[])).validate(&good));
            assert!(guarded.is_err(), "the refused read fails the preparation");
        });
    });
    assert_eq!((prepared, reused), (2, 0));
}
