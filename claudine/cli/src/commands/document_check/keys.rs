//! Claudine's schema-key verdicts on the documents a sequence check loaded,
//! judged once per file and attached to each task occurrence that uses them.

use std::collections::HashSet;

use claudine::composition::CheckedTask;
use darkmatter::findings::TrailFrame;

use super::*;
use super::tasks::property_segments;

/// One key verdict of Claudine's shipped schemas on a loaded document,
/// judged once per invocation and attached to every task occurrence that
/// uses the object it names.
pub(super) struct KeyVerdict {
    /// The file it was judged in.
    pub(super) file: PathBuf,
    /// The owning object's instance path, as JSON Pointer segments.
    pub(super) owner: Vec<String>,
    /// The same path as the schema wrote it (`/sequence/1/group`).
    pub(super) instance_path: String,
    /// The key it is about.
    pub(super) key: Option<String>,
    /// Whether it refuses the key (removed or unknown), not advice.
    pub(super) refusal: bool,
    /// The row, located in [`Self::file`].
    pub(super) row: Diagnostic,
}

impl KeyVerdict {
    /// The key's full authored property in its file
    /// (`sequence[1].group.max_paralel`).
    pub(super) fn property(&self) -> String {
        let mut property = String::new();
        for segment in self.owner.iter().chain(self.key.as_ref()) {
            match segment.parse::<usize>() {
                Ok(index) => property.push_str(&format!("[{index}]")),
                Err(_) if property.is_empty() => property.push_str(segment),
                Err(_) => {
                    property.push('.');
                    property.push_str(segment);
                }
            }
        }
        property
    }
}

/// The segments of a JSON Pointer such as `/sequence/1/group`.
pub(super) fn pointer_segments(pointer: &str) -> Vec<String> {
    pointer
        .split('/')
        .skip(1)
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// An authored property such as `sequence[1].group` as JSON Pointer
/// segments.
pub(super) fn property_pointer(property: &str) -> Vec<String> {
    property_segments(property)
        .into_iter()
        .map(|segment| match segment {
            YamlPathSegment::Key(key) => key,
            YamlPathSegment::Index(index) => index.to_string(),
        })
        .collect()
}

/// The tasks `verdict` belongs to: every occurrence of the innermost task
/// its file authors at or above the object it names; else every task whose
/// reference read the part of the file holding that object (a task file, a
/// whole group or sequence file, a selected catalog entry). Empty for an
/// object no occurrence uses, such as an unselected catalog entry.
pub(super) fn key_owners(tasks: &[CheckedTask], verdict: &KeyVerdict) -> Vec<usize> {
    let encloses = |property: &str| verdict.owner.starts_with(&property_pointer(property));
    let direct = tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| task.site.file == verdict.file && encloses(&task.site.property))
        .map(|(index, task)| (index, property_pointer(&task.site.property).len()));
    let mut deepest: Vec<(usize, usize)> = Vec::new();
    for (index, depth) in direct {
        match deepest.first().map(|(_, held)| *held) {
            Some(held) if held > depth => {}
            Some(held) if held == depth => deepest.push((index, depth)),
            _ => deepest = vec![(index, depth)],
        }
    }
    if !deepest.is_empty() {
        return deepest.into_iter().map(|(index, _)| index).collect();
    }
    tasks
        .iter()
        .enumerate()
        .filter(|(_, task)| {
            task.loaded.iter().any(|loaded| loaded.path == verdict.file && encloses(&loaded.property))
        })
        .map(|(index, _)| index)
        .collect()
}

impl Check {
    /// The key verdicts of Claudine's shipped schemas on `file` as
    /// authored: a removed or undeclared key (an error, which the parser
    /// would refuse at its own boundary, often before the rest is read) and
    /// near-miss advice on an open step (a warning), once per owner and key.
    /// A task, group, catalog, or sequence YAML file is judged by its
    /// `kind`, a Markdown one by its triggers. Typed values are left to the
    /// parser and each prompt's own composition, since a templated value is
    /// not final here.
    pub(super) fn judge_keys(&self, file: &Path) -> Vec<KeyVerdict> {
        use simplified_schema::ValidationProblemCode;
        let is_root = self.root.as_ref().is_some_and(|root| root.resolved_path == file);
        let Some(context) = self
            .invocation
            .as_ref()
            .and_then(|invocation| invocation.derive_source(file).ok())
            .map(|source| source.file_resolution_context().clone())
        else {
            return Vec::new();
        };
        let schemas = claudine::composition::schema::bundle::with_claudine_registration(
            darkmatter::markdown::schemas::DarkmatterSchemas::new(context),
        );
        let report = if composition::is_yaml_source(file) {
            let text = if is_root {
                self.root_ref().original_text.clone()
            } else {
                match std::fs::read_to_string(file) {
                    Ok(text) => text,
                    Err(_) => return Vec::new(),
                }
            };
            match schemas.validate_yaml_document(&text, file) {
                Ok(darkmatter::markdown::schemas::YamlOutcome::Validated(validation)) => validation.report,
                _ => return Vec::new(),
            }
        } else if is_root {
            match schemas.validate(&self.root_ref().markdown) {
                Ok(report) => report,
                Err(_) => return Vec::new(),
            }
        } else {
            let Ok(markdown) = darkmatter::markdown::Markdown::try_from(file) else { return Vec::new() };
            match schemas.validate(&markdown) {
                Ok(report) => report,
                Err(_) => return Vec::new(),
            }
        };
        let keyed = report
            .problems
            .iter()
            .filter(|problem| matches!(problem.code, ValidationProblemCode::RemovedKey | ValidationProblemCode::UnknownKey))
            .map(|problem| (problem, Severity::Error))
            .chain(report.key_advice.iter().map(|advice| (advice, Severity::Warning)));
        let mut seen = HashSet::new();
        let mut verdicts = Vec::new();
        for (problem, severity) in keyed {
            if !seen.insert((problem.code.diagnostic_code(), problem.path.clone(), problem.offending_property.clone())) {
                continue;
            }
            let source = match (problem.line, problem.column) {
                (Some(line), column) => {
                    DiagnosticSource::at(Some(file.to_path_buf()), line as usize, column.unwrap_or(1) as usize)
                }
                (None, _) => DiagnosticSource::file_only(Some(file.to_path_buf())),
            };
            verdicts.push(KeyVerdict {
                file: file.to_path_buf(),
                owner: pointer_segments(&problem.path),
                instance_path: problem.path.clone(),
                key: problem.offending_property.clone(),
                refusal: severity == Severity::Error,
                row: Diagnostic::projected(problem.code.diagnostic_code(), severity, problem.message.clone(), problem.hint.clone())
                    .with_source(source),
            });
        }
        verdicts
    }

    /// Records `verdicts` for `subject`, each once with `trail` and, when
    /// `at_key`, its key's full authored property as the trail's last
    /// property. A removed or unknown key is remembered, so the parser's
    /// refusal of the same key, or a formal file's aggregate refusal made of
    /// such keys only, is not reported a second time.
    pub(super) fn push_key_rows(&mut self, subject: SubjectId, trail: &Trail, verdicts: &[&KeyVerdict], at_key: bool) {
        for verdict in verdicts {
            let mut frames = trail.frames().to_vec();
            if at_key && let Some(TrailFrame::Host { property, .. }) = frames.last_mut() {
                *property = verdict.property();
            }
            let id = self.sink.push(subject, verdict.row.clone().with_trail(Trail::new(frames)));
            if verdict.refusal {
                self.key_problem_paths.entry(verdict.file.clone()).or_default().insert(verdict.instance_path.clone());
                if let Some(key) = &verdict.key {
                    self.keyed.entry((verdict.file.clone(), key.clone())).or_default().push((subject, id));
                }
            }
        }
    }
}

#[cfg(test)]
mod key_owner_tests {
    use claudine::composition::{LoadedDocument, TaskSite};
    use claudine::composition::sequence::preflight::TaskIdentity;

    use super::*;

    fn task(file: &str, property: &str, loaded: &[(&str, &str)]) -> CheckedTask {
        CheckedTask {
            site: TaskSite {
                identity: TaskIdentity::root(0),
                name: property.to_string(),
                file: PathBuf::from(file),
                property: property.to_string(),
                frames: Vec::new(),
                row: None,
            },
            item: Value::Null,
            executable: None,
            prompt: None,
            with: Map::new(),
            with_evaluated: Map::new(),
            with_late: Vec::new(),
            provider: None,
            sequence_document: None,
            seq: None,
            failures: Vec::new(),
            loaded: loaded
                .iter()
                .map(|(path, property)| LoadedDocument { path: PathBuf::from(path), property: property.to_string() })
                .collect(),
        }
    }

    fn verdict(file: &str, pointer: &str, key: &str) -> KeyVerdict {
        KeyVerdict {
            file: PathBuf::from(file),
            owner: pointer_segments(pointer),
            instance_path: pointer.to_string(),
            key: Some(key.to_string()),
            refusal: true,
            row: Diagnostic::projected("dm.schema.unknown_key", Severity::Error, "x", None),
        }
    }

    /// The innermost task a file authors above the key owns it, at every
    /// occurrence; else each task whose reference read the part holding it;
    /// a part no task uses owns nothing.
    #[test]
    fn a_verdict_belongs_to_the_occurrences_that_use_its_object() {
        let tasks = vec![
            task("seq.md", "sequence[0]", &[("child.yaml", "")]),
            task("child.yaml", "sequence[0]", &[]),
            task("seq.md", "sequence[1]", &[("child.yaml", "")]),
            task("child.yaml", "sequence[0]", &[]),
            task("seq.md", "sequence[2]", &[("cat.yaml", "groups[1]")]),
            task("seq.md", "sequence[3]", &[("t.yaml", "")]),
        ];
        assert_eq!(key_owners(&tasks, &verdict("child.yaml", "/sequence/0", "promt")), vec![1, 3]);
        assert_eq!(key_owners(&tasks, &verdict("child.yaml", "", "fail_fast")), vec![0, 2]);
        assert_eq!(key_owners(&tasks, &verdict("cat.yaml", "/groups/1/tasks/0", "whem")), vec![4]);
        assert_eq!(key_owners(&tasks, &verdict("cat.yaml", "/groups/0/tasks/0", "whem")), Vec::<usize>::new());
        assert_eq!(key_owners(&tasks, &verdict("t.yaml", "", "whem")), vec![5]);
        assert_eq!(key_owners(&tasks, &verdict("seq.md", "/sequence/1/group", "max_paralel")), vec![2]);
    }

    #[test]
    fn a_verdicts_property_is_its_keys_authored_path() {
        assert_eq!(verdict("seq.md", "/sequence/1/group", "max_paralel").property(), "sequence[1].group.max_paralel");
        assert_eq!(verdict("t.yaml", "", "whem").property(), "whem");
        assert_eq!(verdict("cat.yaml", "/groups/1/tasks/0", "whem").property(), "groups[1].tasks[0].whem");
    }
}
