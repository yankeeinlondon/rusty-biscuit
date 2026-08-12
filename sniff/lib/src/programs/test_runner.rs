//! Test runner host detector.
//!
//! Combines [`ExecutableIndex`] (global PATH + macOS bundles/Windows install
//! roots) with [`LocalBinIndex`] (project-local bin dirs like
//! `node_modules/.bin`, `vendor/bin`, `.venv/bin`) and the per-runner
//! [`TestRunnerSpec`] to resolve each runner to an [`Availability`].
//!
//! Resolution order (first hit wins; see `test-runner-strategy.md` §3):
//!
//! 1. project-local bin dirs (ecosystem-specific, cwd-relative)
//! 2. global PATH index (`ExecutableIndex`)
//! 3. parent binary for class B/C (`cargo`, `go`, `node`, `python3`, …)
//! 4. none of the above → `Availability::NotFound`

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use strum::{EnumCount, IntoEnumIterator};
use tracing::{debug, instrument};

use crate::executable_index::ExecutableIndex;
use crate::programs::contract::CategoryEnum;
use crate::programs::enums::TestRunner;
use crate::programs::local_bin::LocalBinIndex;
use crate::programs::schema::ProgramMetadata;
use crate::programs::test_runner_spec::{Availability, TEST_RUNNER_SPEC, TestRunnerSpec};

/// Per-runner host resolution result.
///
/// Carries the resolved [`Availability`] alongside the static [`TestRunnerSpec`]
/// metadata so reporters can render ecosystem, kind, and invocation-class
/// annotations next to the availability discriminator without re-reading the
/// catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunnerEntry {
    /// The runner variant.
    pub runner: TestRunner,
    /// Static catalog metadata.
    pub spec: TestRunnerSpec,
    /// Host resolution result.
    pub availability: Availability,
}

impl TestRunnerEntry {
    /// Returns `true` when the runner is runnable from the host.
    #[must_use]
    pub fn is_found(&self) -> bool {
        self.availability.is_found()
    }

    /// Returns the path to the resolved executable, when found on PATH or in
    /// a project bin dir. Returns `None` for `ViaParent` and `NotFound`.
    #[must_use]
    pub fn path(&self) -> Option<&PathBuf> {
        match &self.availability {
            Availability::Installed { path } | Availability::Local { path, .. } => Some(path),
            Availability::ViaParent { .. } | Availability::NotFound => None,
        }
    }
}

impl Serialize for TestRunnerEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let info = self.runner.info();
        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("name", &info.display_name)?;
        map.serialize_entry("binary_name", &info.binary_name)?;
        map.serialize_entry("description", &info.description)?;
        map.serialize_entry("website", &info.website)?;
        map.serialize_entry("ecosystem", &self.spec.ecosystem)?;
        map.serialize_entry("invocation", &self.spec.invocation)?;
        map.serialize_entry("kind", &self.spec.kind)?;
        map.serialize_entry("is_ecosystem_default", &self.spec.is_ecosystem_default)?;

        // Flatten `Availability` so its `availability` discriminator and
        // per-variant fields (`path`, `root`, `parent`) appear at the same
        // level as the other entry fields, matching `test-runner-strategy.md`:
        //   { "name": ..., "availability": "installed", "path": "..." }
        let availability_value =
            serde_json::to_value(&self.availability).map_err(serde::ser::Error::custom)?;
        if let serde_json::Value::Object(obj) = availability_value {
            for (key, value) in obj {
                map.serialize_entry(&key, &value)?;
            }
        }

        map.end()
    }
}

/// Host detection result for the test-runner category.
///
/// Stores one [`TestRunnerEntry`] per variant, indexed by variant ordinal so
/// `entry(runner)` is O(1). The default value reports `NotFound` for every
/// runner; populate it via [`detect_test_runners`] or
/// [`InstalledTestRunners::from_availability`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledTestRunners {
    /// One entry per `TestRunner` variant, indexed by variant ordinal.
    pub entries: Vec<TestRunnerEntry>,
}

impl Default for InstalledTestRunners {
    fn default() -> Self {
        Self::from_availability(
            TestRunner::iter()
                .map(|runner| (runner, Availability::NotFound))
                .collect::<Vec<_>>(),
        )
    }
}

impl InstalledTestRunners {
    /// Build a detector from an iterable of `(runner, availability)` pairs.
    ///
    /// Useful for tests and for fabricating a partial detector from a known
    /// subset of runners.
    #[must_use]
    pub fn from_availability<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (TestRunner, Availability)>,
    {
        let mut entries: Vec<Option<TestRunnerEntry>> =
            (0..TestRunner::COUNT).map(|_| None).collect();
        for (runner, availability) in iter {
            let spec = TEST_RUNNER_SPEC[runner.variant_index()];
            entries[runner.variant_index()] = Some(TestRunnerEntry {
                runner,
                spec,
                availability,
            });
        }
        let entries = entries
            .into_iter()
            .enumerate()
            .map(|(idx, opt)| match opt {
                Some(entry) => entry,
                None => {
                    let runner = TestRunner::iter().nth(idx).expect("valid TestRunner index");
                    let spec = TEST_RUNNER_SPEC[idx];
                    TestRunnerEntry {
                        runner,
                        spec,
                        availability: Availability::NotFound,
                    }
                }
            })
            .collect();
        Self { entries }
    }

    /// Returns the entry for `runner`.
    #[must_use]
    pub fn entry(&self, runner: TestRunner) -> &TestRunnerEntry {
        let idx = runner.variant_index();
        &self.entries[idx]
    }

    /// Returns `true` when `runner` is found on the host (any installed,
    /// local, or via-parent case).
    #[must_use]
    pub fn is_found(&self, runner: TestRunner) -> bool {
        self.entry(runner).is_found()
    }

    /// Returns the entries for runners that are found on the host.
    #[must_use]
    pub fn found(&self) -> Vec<&TestRunnerEntry> {
        self.entries.iter().filter(|e| e.is_found()).collect()
    }
}

impl Serialize for InstalledTestRunners {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.entries.len()))?;
        for entry in &self.entries {
            map.serialize_entry(entry.runner.serde_key(), entry)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for InstalledTestRunners {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::collections::HashMap;

        let raw: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;
        let mut entries: Vec<Option<TestRunnerEntry>> =
            (0..TestRunner::COUNT).map(|_| None).collect();

        for runner in TestRunner::iter() {
            let key = runner.serde_key();
            if let Some(value) = raw.get(key) {
                // Reconstruct the inner Availability shape by collecting the
                // flattened keys back into a single object.
                let availability = serde_json::from_value::<Availability>(value.clone())
                    .unwrap_or(Availability::NotFound);
                let spec = TEST_RUNNER_SPEC[runner.variant_index()];
                entries[runner.variant_index()] = Some(TestRunnerEntry {
                    runner,
                    spec,
                    availability,
                });
            }
        }

        let entries: Vec<TestRunnerEntry> = entries
            .into_iter()
            .enumerate()
            .map(|(idx, opt)| match opt {
                Some(entry) => entry,
                None => {
                    let runner = TestRunner::iter().nth(idx).expect("valid TestRunner index");
                    let spec = TEST_RUNNER_SPEC[idx];
                    TestRunnerEntry {
                        runner,
                        spec,
                        availability: Availability::NotFound,
                    }
                }
            })
            .collect();

        Ok(Self { entries })
    }
}

/// Resolve a single runner against the host using `index`, `local`, and the
/// runner's static spec.
///
/// Resolution order:
///
/// 1. project-local bin dirs (when the spec's ecosystem has roots) →
///    [`Availability::Local`]
/// 2. global `PATH`/macOS-bundle/Windows-fallback index →
///    [`Availability::Installed`]
/// 3. parent binary for class B/C → [`Availability::ViaParent`]
/// 4. none of the above → [`Availability::NotFound`]
#[must_use]
#[instrument(skip_all, fields(runner = %format!("{:?}", runner)))]
pub fn resolve_test_runner(
    runner: TestRunner,
    index: &ExecutableIndex,
    local: &LocalBinIndex,
) -> TestRunnerEntry {
    let info = runner.info();
    let spec = TEST_RUNNER_SPEC[runner.variant_index()];

    // Layer 1: project-local bin search (Node, PHP, Python, Ruby).
    let primary_name = info.binary_name;
    let alt_names = info.alternate_binary_names;

    let local_probe = |name: &str| local.find(spec.ecosystem, name);
    if let Some((path, root)) =
        local_probe(primary_name).or_else(|| alt_names.iter().find_map(|n| local_probe(n)))
    {
        debug!(?path, ?root, "resolved via project-local bin");
        return TestRunnerEntry {
            runner,
            spec,
            availability: Availability::Local { path, root },
        };
    }

    // Layer 2: global PATH / bundle / Windows index.
    if let Some((path, _)) = index
        .find_with_source(primary_name)
        .or_else(|| alt_names.iter().find_map(|n| index.find_with_source(n)))
    {
        debug!(?path, "resolved via global index");
        return TestRunnerEntry {
            runner,
            spec,
            availability: Availability::Installed { path },
        };
    }

    // Layer 3: parent binary for class B/C.
    if let Some(parent) = spec.parent_binary
        && index.find(parent).is_some()
    {
        debug!(parent, "resolved via parent binary");
        return TestRunnerEntry {
            runner,
            spec,
            availability: Availability::ViaParent {
                parent: parent.to_string(),
            },
        };
    }

    TestRunnerEntry {
        runner,
        spec,
        availability: Availability::NotFound,
    }
}

/// Detect every test runner against the host using a shared global index.
///
/// Builds a [`LocalBinIndex`] rooted at `std::env::current_dir()` and probes
/// each runner. Runners without any project-local root on the host fall back
/// to PATH and parent-binary resolution as documented in
/// [`resolve_test_runner`].
#[instrument(skip_all)]
pub fn detect_test_runners(index: Arc<ExecutableIndex>) -> InstalledTestRunners {
    let start_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let local = LocalBinIndex::new(start_dir);

    let entries: Vec<TestRunnerEntry> = TestRunner::iter()
        .map(|runner| resolve_test_runner(runner, &index, &local))
        .collect();

    InstalledTestRunners { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executable_index::ExecutableIndex;
    use crate::test_helpers::make_executable_fixture;

    #[test]
    fn unavailable_runner_returns_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let index = ExecutableIndex::build_path_only();
        let local = LocalBinIndex::new(tmp.path().to_path_buf());

        let entry = resolve_test_runner(TestRunner::Ava, &index, &local);
        assert_eq!(entry.availability, Availability::NotFound);
        assert!(!entry.is_found());
    }

    #[test]
    fn project_local_node_resolves_to_local() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        let bin_dir = project.join("node_modules").join(".bin");
        let expected = make_executable_fixture(&bin_dir.join("vitest"));

        let index = ExecutableIndex::build_path_only();
        let local = LocalBinIndex::new(project.clone());

        let entry = resolve_test_runner(TestRunner::Vitest, &index, &local);
        match entry.availability {
            Availability::Local { ref path, ref root } => {
                assert_eq!(*path, expected);
                assert_eq!(*root, project);
            }
            other => panic!("expected Local, got {:?}", other),
        }
        assert!(entry.is_found());
    }

    #[test]
    fn installed_test_runners_default_is_all_not_found() {
        let runners = InstalledTestRunners::default();
        assert_eq!(runners.entries.len(), TestRunner::COUNT);
        for entry in &runners.entries {
            assert_eq!(entry.availability, Availability::NotFound);
        }
    }

    #[test]
    fn from_availability_yields_one_entry_per_variant() {
        let runners = InstalledTestRunners::from_availability(
            TestRunner::iter()
                .map(|runner| (runner, Availability::NotFound))
                .collect::<Vec<_>>(),
        );
        assert_eq!(runners.entries.len(), TestRunner::COUNT);
    }

    #[test]
    fn found_filters_to_resolved_entries() {
        let runners = InstalledTestRunners::from_availability(
            TestRunner::iter()
                .map(|runner| {
                    let availability = if matches!(runner, TestRunner::CargoTest) {
                        Availability::ViaParent {
                            parent: "cargo".to_string(),
                        }
                    } else {
                        Availability::NotFound
                    };
                    (runner, availability)
                })
                .collect::<Vec<_>>(),
        );
        let found = runners.found();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].runner, TestRunner::CargoTest);
    }

    #[test]
    fn entry_path_returns_path_for_local_and_installed() {
        let path = PathBuf::from("/tmp/vitest");
        let runners = InstalledTestRunners::from_availability(
            [
                (
                    TestRunner::CargoTest,
                    Availability::ViaParent {
                        parent: "cargo".to_string(),
                    },
                ),
                (
                    TestRunner::Nextest,
                    Availability::Installed { path: path.clone() },
                ),
                (
                    TestRunner::GoTest,
                    Availability::Local {
                        path: path.clone(),
                        root: PathBuf::from("/tmp"),
                    },
                ),
            ]
            .into_iter()
            .collect::<Vec<_>>(),
        );

        assert_eq!(runners.entry(TestRunner::Nextest).path(), Some(&path));
        assert_eq!(runners.entry(TestRunner::GoTest).path(), Some(&path));
        assert_eq!(runners.entry(TestRunner::CargoTest).path(), None);
    }

    #[test]
    fn serialize_emits_serde_key_indexed_map() {
        let runners = InstalledTestRunners::from_availability(
            TestRunner::iter()
                .map(|runner| (runner, Availability::NotFound))
                .collect::<Vec<_>>(),
        );
        let json = serde_json::to_string(&runners).expect("serialize");
        assert!(json.contains("\"cargo_test\""));
        assert!(json.contains("\"availability\""));
    }

    #[test]
    fn round_trips_through_json() {
        let runners = InstalledTestRunners::from_availability(
            [
                (
                    TestRunner::CargoTest,
                    Availability::ViaParent {
                        parent: "cargo".to_string(),
                    },
                ),
                (TestRunner::Ava, Availability::NotFound),
            ]
            .into_iter()
            .collect::<Vec<_>>(),
        );

        let json = serde_json::to_string(&runners).expect("serialize");
        let parsed: InstalledTestRunners = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.entries.len(), TestRunner::COUNT);
        assert_eq!(
            parsed.entry(TestRunner::CargoTest).availability,
            Availability::ViaParent {
                parent: "cargo".to_string(),
            },
        );
        assert_eq!(
            parsed.entry(TestRunner::Ava).availability,
            Availability::NotFound,
        );
    }
}
