//! Legacy internal execution models for tests of the pre-container runner.
//!
//! Public grammar is always parsed after removing retired authoring fields.
//! The old stacks and serial scheduler are then installed on inert nodes so
//! their existing execution contracts remain covered until runtime migration.

use super::*;
use darkmatter::markdown::compose::EffectiveStateBuilder;

pub(crate) struct RuntimeFixture {
    pub source: ResolvedCompositionSource,
    originals: HashMap<PathBuf, Value>,
    documents: HashMap<PathBuf, Arc<ReferencedDocument>>,
}

impl RuntimeFixture {
    pub fn new(source: &ResolvedCompositionSource, directory: &Path) -> Self {
        let mut result = Self {
            source: source.clone(), originals: HashMap::new(), documents: HashMap::new(),
        };
        result.read_documents(directory);
        let mut root = Value::Object(source.markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect());
        result.originals.insert(canonical(&source.resolved_path), root.clone());
        sanitize(&mut root);
        *result.source.markdown.frontmatter_mut().as_map_mut() = root.as_object().unwrap().iter().map(|(key, value)| (key.clone(), value.clone())).collect();
        result
    }

    fn read_documents(&mut self, directory: &Path) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { self.read_documents(&path); continue; }
            if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml" | "json" | "json5")) { continue; }
            let Ok(loaded) = super::super::data::load_document(&path) else { continue };
            let path = canonical(&path);
            self.originals.insert(path.clone(), loaded.value.clone());
            let mut value = loaded.value;
            sanitize(&mut value);
            self.documents.insert(path, Arc::new(ReferencedDocument {
                text: Vec::new(), value, orders: Arc::new(loaded.orders),
                checked: std::sync::Mutex::new(Vec::new()), catalog_index: std::sync::OnceLock::new(),
            }));
        }
    }

    pub fn build(
        &self,
        plan: &SequencePlan,
        context: ComposeContext,
        resolution: &biscuit_file::FileResolutionContext,
    ) -> Result<PreflightGraph, CompositionError> {
        let mut loader = Loader::new(&self.source, context, resolution.clone());
        loader.documents = self.documents.clone();
        loader.walk_plan(plan)?;
        let mut steps = std::mem::take(&mut loader.graph.steps);
        for (index, step) in steps.iter_mut().enumerate() {
            loader.enter_step(plan, index);
            if let Some(task) = &mut step.task {
                self.restore(task, &mut loader, false)?;
            }
        }
        loader.graph.steps = steps;
        Ok(loader.graph)
    }

    fn restore(&self, task: &mut PreflightTask, loader: &mut Loader<'_>, in_group: bool) -> Result<(), CompositionError> {
        let root = &self.originals[&canonical(&task.origin_path)];
        if let Some(raw) = root.pointer(task.authored.pointer()) {
            task.setup = raw.get("setup").cloned();
            task.teardown = raw.get("teardown").cloned();
        }
        let state = EffectiveStateBuilder::new()
            .with_shared_frontmatter(loader.step_frontmatter.clone())
            .with_context(loader.context.clone()).with_allow_ctx_override(true).build().unwrap();
        let scope = SiteScope {
            origin: &task.origin_path.clone(), label: &task.label.clone(),
            property: task.diagnostic.task_property.clone(), state: &state,
            placement: Placement { position: task.position, in_group }, guard: None,
        };
        loader.collect_lifecycle_shell(task, &scope)?;
        if let PreflightAction::Group(group) = &mut task.action {
            let group_root = &self.originals[&canonical(&group.origin_path)];
            let raw = if group_root.get("kind").and_then(Value::as_str) == Some("group") {
                Some(group_root)
            } else if group_root.get("kind").and_then(Value::as_str) == Some("group-catalog") {
                group_root.get("groups").and_then(|groups| groups.get(&group.name))
            } else {
                group_root.pointer(task.authored.child("group").pointer())
            };
            group.execution = match raw.and_then(|raw| raw.get("execution")).and_then(Value::as_str) {
                Some("parallel") => GroupExecution::Parallel,
                _ => GroupExecution::Serial,
            };
            for child in &mut group.tasks { self.restore(child, loader, true)?; }
        }
        Ok(())
    }
}

fn sanitize(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let is_task = ["shell", "prompt", "side_effect", "group", "task"].iter().any(|key| map.contains_key(*key));
            if is_task { map.remove("setup"); map.remove("teardown"); }
            if map.contains_key("tasks") {
                map.remove("execution");
                if let Some(Value::Array(tasks)) = map.get_mut("tasks") {
                    for (index, task) in tasks.iter_mut().enumerate() {
                        if let Some(task) = task.as_object_mut() {
                            task.entry("name").or_insert_with(|| Value::String(format!("fixture-leg-{index}")));
                        }
                    }
                }
            }
            for value in map.values_mut() { sanitize(value); }
        }
        Value::Array(values) => { for value in values { sanitize(value); } }
        _ => {}
    }
}
