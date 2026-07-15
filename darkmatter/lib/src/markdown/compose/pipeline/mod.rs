//! Compose pipeline domain: the driver (orchestrator spine) and the operation
//! registry.
//!
//! - [`operations`] owns the [`ComposeOperation`](operations::ComposeOperation)
//!   enum, its [`ComposePhase`](operations::ComposePhase) grouping, the
//!   authoritative descriptor table, and the default execution order.
//! - The phase-sequencing driver (`run_compose_pipeline*`) lives here; the
//!   per-phase dispatch and stage runners live in [`phases`], both lifted off
//!   `impl Markdown` in `compose/mod.rs`.

pub mod operations;
pub mod phases;

use super::super::Markdown;
use super::super::types::MarkdownResult;
use super::{
    ComposeOperation, ComposeOptions, ComposePhase, ComposeReport, ComposeSource, ComposeWarning,
    EffectiveStateBuilder, abbreviate_path, prepare_frontmatter_for_compose,
};
use super::{
    cache, context, frontmatter_interpolation, frontmatter_shell_expansion, perf, remote,
    schema_validation, shell_expansion, transclusion,
};
use serde_json::{Map, Value};
use tracing::{info, instrument, trace};

impl Markdown {
    /// Internal pipeline runner.
    pub(crate) fn run_compose_pipeline(&mut self, options: ComposeOptions) -> MarkdownResult<ComposeReport> {
        // Resolve persistent cache root if configured
        let persistent_root = options.cache_root.as_ref().map(|root| {
            cache::FileStore::resolve_cache_root(Some(root), options.cache_namespace.as_deref())
        });

        // Reuse the caller-supplied shared runtime when present (so a pre-flight
        // walk and this pass fetch each URL once); otherwise build one whose
        // persistent store is shared with the local compose artifact cache.
        let remote_fetch = options.remote_fetch_runtime();

        let mut runtime = shell_expansion::types::PipelineRuntime::with_remote_fetch(
            options.max_transclusion_depth,
            options.cache_access_mode,
            persistent_root,
            remote_fetch,
        );

        // Eagerly register discovered remote URLs and start fetching. The two
        // discovery paths gate independently: directive (`::file`/`::code`)
        // discovery requires the explicit remote-transclusion opt-in, while
        // URL-typed expression-function arguments (`frontmatter(url)`, …) are a
        // read-side capability enabled whenever remote reads are configured —
        // so a caller that allows a host but never enables transclusion can
        // still prefetch and read its expression URLs.
        if options.remote_reads_enabled() {
            let mut catalog = remote::RemoteUrlCatalog::new();

            if options.allow_remote_transclusion
                && options.is_enabled(ComposeOperation::BlockTransclusion)
            {
                let directives = transclusion::parse_directives(
                    &self.content,
                    self.source_context_for_errors(),
                )
                .unwrap_or_default();
                for entry in
                    remote::discover_remote_urls_from_directives(&directives, &options.source)
                {
                    catalog.add(entry);
                }
            }

            if options.is_enabled(ComposeOperation::Interpolation) {
                for entry in
                    remote::discover_remote_urls_from_expressions(&self.content, &options.source)
                {
                    catalog.add(entry);
                }
            }

            for url in catalog.urls() {
                runtime.remote_fetch.register_and_fetch(url);
            }
        }

        let mut report = self.run_compose_pipeline_internal(options, &mut runtime)?;
        report.cache_stats = Some(runtime.cache.stats());
        report.remote_fetch_stats = Some(runtime.remote_fetch.stats());
        Ok(report)
    }

    /// Internal recursive pipeline runner shared by root and child documents.
    ///
    /// Frontmatter is resolved in a fixed order before the body stages run:
    /// **Interp pass 1 → Schema Validation → Shell Expansion → Interp pass 2**.
    /// Pass 1 resolves `{{ }}` against seed values; schema validation and
    /// coercion run next; `$(...)` frontmatter values then expand; pass 2
    /// resolves any keys that were deferred because they referenced
    /// shell-pending values. Read-side functions and `doc.*` are available in
    /// both passes.
    ///
    /// Executes operations in four phases:
    /// 1. **Inline Pre** (serial): TextReplacement, PageBlocks, Interpolation, ShellExpansion, ShellBlocks
    /// 2. **Transclusion** (prepared serially, resolved concurrently): BlockTransclusion,
    ///    FrontmatterTransclusion, CodeTransclusion, TocLinking, FileLinks
    /// 3. **Inline Post** (serial): Cleanup, Normalization
    /// 4. **Finalization** (root-only serial): LinkNormalization
    #[instrument(skip_all, fields(source = ?options.source))]
    pub(crate) fn run_compose_pipeline_internal(
        &mut self,
        options: ComposeOptions,
        runtime: &mut shell_expansion::types::PipelineRuntime,
    ) -> MarkdownResult<ComposeReport> {
        let source_id = match &options.source {
            ComposeSource::Unknown => None,
            ComposeSource::File(path) => Some(
                std::fs::canonicalize(path)
                    .unwrap_or_else(|_| path.clone())
                    .to_string_lossy()
                    .to_string(),
            ),
            ComposeSource::Url(url) => Some(url.to_string()),
        };

        if let Some(id) = source_id.clone() {
            let path = match &options.source {
                ComposeSource::File(p) => p.clone(),
                ComposeSource::Url(u) => std::path::PathBuf::from(u.to_string()),
                ComposeSource::Unknown => std::path::PathBuf::from("<unknown>"),
            };
            runtime.transclusion.enter(id, path, 1)?;
        }

        let result = (|| {
            let mut report = ComposeReport::new();
            let mut perf = perf::PerfCollector::new(options.perf_enabled);

            let pre_interpolation_snapshot = prepare_frontmatter_for_compose(
                self,
                &options,
                options.is_enabled(ComposeOperation::FrontmatterShellExpansion),
            );

            let shell_expansion_enabled =
                options.is_enabled(ComposeOperation::FrontmatterShellExpansion);

            // Frontmatter Interpolation: resolve {{ }} in frontmatter values
            // before EffectiveState is built, since it mutates frontmatter
            // inputs that drive later stages.
            //
            // When shell expansion is also enabled, defer any templated key
            // that references a shell-pending value (top-level `$(...)`). A
            // second interpolation pass after shell expansion will resolve
            // those keys against the shell-expanded values.
            if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
                let fm_start = perf.is_enabled().then(std::time::Instant::now);
                // Capture the on-disk locus before borrowing frontmatter so a
                // file-reference failure can render an OSC8 link + focused
                // excerpt instead of the late-binding fallback.
                let fm_source_ctx = self.full_source_context_for_errors();
                let fm_report = frontmatter_interpolation::interpolate_frontmatter(
                    self.frontmatter_mut(),
                    options.context(),
                    options.fail_fast,
                    shell_expansion_enabled,
                    Some(options.frontmatter_resolution_context()),
                    &options.exclude_keys,
                )
                .map_err(|e| e.with_on_disk_source(&fm_source_ctx))?;
                report.frontmatter_interpolations_applied = fm_report.replacements;
                report.warnings.extend(fm_report.warnings);
                if let Some(start) = fm_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterInterpolation,
                        start.elapsed(),
                    );
                }
            }

            // Surface the set of deferred keys that are actually present in
            // this document's frontmatter (DM1 metadata). Lets callers
            // distinguish "raw because deferred" from "raw because composition
            // failed" for dry-run labeling and diagnostics.
            if !options.exclude_keys.is_empty() {
                let fm = self.frontmatter().as_map();
                report.deferred_frontmatter_keys = options
                    .exclude_keys
                    .iter()
                    .filter(|k| fm.contains_key(*k))
                    .cloned()
                    .collect();
            }

            // Schema Validation: check frontmatter against $schema or baseline
            // AFTER frontmatter interpolation so template values like
            // `runtime_agent: '{{ env.AGENT }}'` are evaluated to their
            // resolved form before being checked. Runs BEFORE shell
            // expansion so the validator can fail-fast without triggering
            // (potentially expensive or side-effectful) shell commands when
            // the resolved frontmatter is invalid. This stage also coerces
            // schema-recognized scalars (e.g. the string "true" against a
            // boolean field) and writes the real types back into frontmatter,
            // so later stages and the composed output see coerced values.
            //
            // For frontmatter values that depend on shell-expanded inputs,
            // the second interpolation pass below will re-resolve them and
            // the prepare-time consumer (e.g. claudine's `prepare_*_with_schema`)
            // can re-validate the post-shell effective frontmatter. Internal
            // non-terminal passes (shell-command discovery) that strip
            // FrontmatterShellExpansion set
            // `ComposeOptions::defer_shell_pending_schema_problems` so a
            // still-literal `$(...)` value is deferred rather than reported as a
            // final violation here.
            // Trigger-schema registry shared across both schema-validation
            // passes: the first pass (here) scans the `schemas/` ancestry from
            // disk; the post-shell pass below reuses that registry instead of
            // re-walking it (F8). Matching is re-evaluated against the current
            // frontmatter in each pass regardless.
            let mut trigger_registry = None;
            {
                let sv_start = perf.is_enabled().then(std::time::Instant::now);
                schema_validation::run_with_registry(self, &options, &mut trigger_registry)?;
                if let Some(start) = sv_start {
                    perf.record(perf::PerfMetricKind::SchemaValidation, start.elapsed());
                }
            }

            // Pre-Flight validation (v2 design step 4): when the caller
            // supplies a pre-approved command set, verify up-front that it
            // covers every command the condition-blind collector discovers —
            // before any frontmatter `$(...)`, body `::shell`, or shell block
            // executes. This removes the failure mode where an earlier
            // frontmatter command runs before a later body or shell-block
            // command is found unapproved. Root-only (depth 1) because the
            // collector already walks every child; gated on a shell-executing
            // operation being enabled so the collection's own internal inline
            // compose (which disables shell execution) cannot recurse.
            if options.pre_approved_commands.is_some()
                && runtime.transclusion.depth() <= 1
                && (options.is_enabled(ComposeOperation::FrontmatterShellExpansion)
                    || options.is_enabled(ComposeOperation::ShellExpansion)
                    || options.is_enabled(ComposeOperation::ShellBlocks))
            {
                super::preflight::validate_pre_approved(self, &options)?;
            }

            // Frontmatter Shell Expansion: execute $(cmd) in frontmatter values
            // before EffectiveState is built, since the expanded values must be
            // visible to all later stages.
            if shell_expansion_enabled {
                let fse_start = perf.is_enabled().then(std::time::Instant::now);
                let fse_ctx = self.full_source_context_for_errors();
                let fse_report = frontmatter_shell_expansion::execute_frontmatter_shell_expansion(
                    self.frontmatter_mut(),
                    &options,
                    runtime,
                    pre_interpolation_snapshot.as_ref(),
                    &fse_ctx,
                )?;
                report.frontmatter_shell_expansions_applied = fse_report.replacements;
                report.shell_approvals_used += fse_report.approvals_used;
                report.warnings.extend(fse_report.warnings);
                if let Some(start) = fse_start {
                    perf.record(
                        perf::PerfMetricKind::FrontmatterShellExpansion,
                        start.elapsed(),
                    );
                }

                // Second interpolation pass: templated keys that referenced
                // shell-pending values were deferred above. Now that shell
                // expansion has produced concrete values, resolve them.
                if options.is_enabled(ComposeOperation::FrontmatterInterpolation)
                    && fse_report.replacements > 0
                {
                    let fm_start = perf.is_enabled().then(std::time::Instant::now);
                    let fm_source_ctx = self.full_source_context_for_errors();
                    let fm_report = frontmatter_interpolation::interpolate_frontmatter(
                        self.frontmatter_mut(),
                        options.context(),
                        options.fail_fast,
                        false,
                        Some(options.frontmatter_resolution_context()),
                        &options.exclude_keys,
                    )
                    .map_err(|e| e.with_on_disk_source(&fm_source_ctx))?;
                    report.frontmatter_interpolations_applied += fm_report.replacements;
                    report.warnings.extend(fm_report.warnings);
                    if let Some(start) = fm_start {
                        perf.record(
                            perf::PerfMetricKind::FrontmatterInterpolation,
                            start.elapsed(),
                        );
                    }
                }

                // Trigger activation is a function of the current frontmatter
                // snapshot. Re-assemble after shell expansion and interpolation
                // pass 2 so concrete values can activate or deactivate payloads.
                if options.trigger_schemas {
                    schema_validation::run_with_registry(self, &options, &mut trigger_registry)?;
                }
            }

            // Build effective state for replacement/interpolation and condition checks.
            let esb_start = perf.is_enabled().then(std::time::Instant::now);
            let effective_state = EffectiveStateBuilder::new()
                .with_frontmatter(
                    self.frontmatter()
                        .as_map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                )
                .with_external_state(
                    options
                        .external_state
                        .clone()
                        .unwrap_or(Value::Object(Map::new())),
                )
                .with_merge_strategy(crate::markdown::MergeStrategy::PreferDocument)
                .with_replace_parent_wins(options.replace_parent_wins)
                .with_context(options.context().clone())
                .with_allow_ctx_override(options.allow_ctx_override)
                .build()?;
            if let Some(start) = esb_start {
                perf.record(perf::PerfMetricKind::EffectiveStateBuild, start.elapsed());
            }

            // Convert ctx diagnostics to compose warnings
            let source_display = match &options.source {
                ComposeSource::File(p) => abbreviate_path(p),
                ComposeSource::Url(u) => u.to_string(),
                ComposeSource::Unknown => "unknown".to_string(),
            };
            for diag in effective_state.ctx_diagnostics() {
                let warning = match diag {
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys }
                        if colliding_keys.is_empty() =>
                    {
                        // No warning needed when merge succeeded without collisions
                        continue;
                    }
                    context::ContextMergeDiagnostic::UserCtxMerged { colliding_keys } => {
                        let keys_list = colliding_keys.join(", ");
                        ComposeWarning::new(
                            "context",
                            format!(
                                "the <blue>{source_display}</blue> document <i>defines</i> a <inverse>ctx</inverse> property and keys [<dim>{keys_list}</dim>] in the <inverse>ctx</inverse> dictionary conflict with those provided by Darkmatter's normal context dictionary!"
                            ),
                        )
                    }
                    context::ContextMergeDiagnostic::InvalidUserCtxReplaced => ComposeWarning::new(
                        "context",
                        "Document ctx was not an object; replaced with runtime context",
                    ),
                    context::ContextMergeDiagnostic::PartialRuntimeCapture { area, detail } => {
                        ComposeWarning::new(
                            "context",
                            format!("Partial runtime capture for {area}: {detail}"),
                        )
                    }
                };
                report.warnings.push(warning);
            }

            let mut transclusion_ran = false;
            for operation in ComposeOperation::default_order() {
                trace!(operation = ?operation, enabled = options.is_enabled(*operation), "compose: checking operation");
                if !options.is_enabled(*operation) {
                    continue;
                }

                info!(operation = ?operation, phase = ?operation.phase(), "compose: running operation");
                match operation.phase() {
                    ComposePhase::InlinePre => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_pre_operation(
                            *operation,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                            &mut perf,
                        )?;
                        if let Some(start) = op_start
                            && let Some(kind) = operation.perf_metric()
                        {
                            perf.record(kind.to_perf_metric_kind(), start.elapsed());
                        }
                    }
                    ComposePhase::Transclusion => {
                        if transclusion_ran {
                            continue;
                        }

                        let enabled_transclusion_ops = ComposeOperation::default_order()
                            .iter()
                            .copied()
                            .filter(|op| {
                                op.phase() == ComposePhase::Transclusion && options.is_enabled(*op)
                            })
                            .collect::<Vec<_>>();

                        self.run_transclusion_phase(
                            &enabled_transclusion_ops,
                            &effective_state,
                            &options,
                            runtime,
                            &mut report,
                            &mut perf,
                        )?;
                        transclusion_ran = true;
                    }
                    ComposePhase::InlinePost => {
                        let op_start = perf.is_enabled().then(std::time::Instant::now);
                        self.run_inline_post_operation(*operation, &options, &mut report)?;
                        if let Some(start) = op_start
                            && let Some(kind) = operation.perf_metric()
                        {
                            perf.record(kind.to_perf_metric_kind(), start.elapsed());
                        }
                    }
                    ComposePhase::Finalization => {
                        if runtime.transclusion.depth() <= 1 {
                            let op_start = perf.is_enabled().then(std::time::Instant::now);
                            self.run_finalization_operation(*operation, &options, &mut report)?;
                            if let Some(start) = op_start
                                && let Some(kind) = operation.perf_metric()
                            {
                                perf.record(kind.to_perf_metric_kind(), start.elapsed());
                            }
                        }
                    }
                }
            }

            report.max_transclusion_depth = runtime.transclusion.deepest_seen;
            if perf.is_enabled() {
                perf.set_capture_timings(options.context().capture_timings().to_vec());
            }
            report.perf = perf.finish();
            Ok(report)
        })();

        if source_id.is_some() {
            runtime.transclusion.exit();
        }

        result
    }
}
