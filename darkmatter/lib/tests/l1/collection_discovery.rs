//! Approval, child discovery, and cache behavior of `::map` / `::filter`
//! bodies, through `Markdown::compose_preflight` and `Markdown::compose_with`.
//!
//! Boundary: the library's public compose API over real temporary files; shell
//! commands are never run by a preflight and are approved by a recording
//! handler where a compose reaches them. No terminal or browser behavior.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use darkmatter::markdown::compose::preflight::ComposePreflightReport;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::template::{HoleTiming, ModeRequirement};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOperation, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

fn options_for(dir: &Path, path: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(path.to_path_buf())
        .with_shell_policy_root(dir)
        .with_shell_working_directory(dir)
}

fn preflight(dir: &Path, path: &Path) -> Result<ComposePreflightReport, MarkdownError> {
    Markdown::try_from(path)
        .unwrap()
        .compose_preflight(&crate::request_support::request(options_for(dir, path)))
}

fn root() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    (dir, root)
}

const NUMBERS: &str = "---\n$schema:\n  data: number[](required)\ndata: [1, 2]\n---\n";

fn hole_types(report: &ComposePreflightReport) -> Vec<String> {
    report
        .candidates
        .iter()
        .flat_map(|candidate| candidate.template.as_ref().expect("a valid template").placeholders())
        .map(|placeholder| placeholder.typing.display_type())
        .collect()
}

// ── local templates ──────────────────────────────────────────────────────

mod local_templates {
    use super::*;

    fn candidate_of(body: &str) -> ComposePreflightReport {
        let (_guard, root) = root();
        let path = write(&root, "doc.md", &format!("{NUMBERS}{body}"));
        preflight(&root, &path).unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    fn a_local_hole_is_template_required_and_typed_by_the_collection_declaration() {
        let report = candidate_of("::map data\n::shell echo {{ i }}\n::end-map\n");
        let [candidate] = report.candidates.as_slice() else { panic!("{:?}", report.candidates) };
        assert_eq!(candidate.requirement, ModeRequirement::TemplateRequired);
        assert!(candidate.resolved.is_none(), "a local has no resolved bytes");
        let template = candidate.template.as_ref().unwrap();
        let timings: Vec<_> = template.placeholders().map(|hole| hole.timing.clone()).collect();
        assert_eq!(timings, [HoleTiming::Local]);
        assert_eq!(hole_types(&report), ["string (validated as number)"]);
        assert!(report.entries.is_empty(), "no resolved command is approved for a local");
    }

    #[test]
    fn explicit_and_implicit_reads_and_reads_inside_expressions_agree() {
        let report = candidate_of(
            "::map data\n::shell echo {{ local.i }}\n::shell echo {{ i + 1 }}\n::shell echo {{ fm.data }}\n::end-map\n",
        );
        let timings: Vec<_> = report
            .candidates
            .iter()
            .map(|candidate| {
                candidate.template.as_ref().unwrap().placeholders().next().unwrap().timing.clone()
            })
            .collect();
        assert_eq!(timings, [HoleTiming::Local, HoleTiming::Local, HoleTiming::ApprovalTime]);
        // `fm.data` is a document read: it is never a local, even beside one.
        assert_eq!(report.candidates[2].requirement, ModeRequirement::AnyMode);
        assert_eq!(hole_types(&report)[0], "string (validated as number)");
        // An expression is not a bare local: the scalar fallback applies.
        assert_eq!(hole_types(&report)[1], "string");
    }

    #[test]
    fn an_empty_or_gated_off_collection_is_still_scanned_for_templates() {
        let (_guard, root) = root();
        let empty = write(
            &root,
            "empty.md",
            "---\n$schema:\n  data: number[](required)\ndata: []\n---\n::map data\n::shell echo {{ i }}\n::end-map\n",
        );
        assert_eq!(preflight(&root, &empty).unwrap().candidates.len(), 1);
        let gated = write(
            &root,
            "gated.md",
            &format!("{NUMBERS}::map data when=\"false\"\n::shell echo {{{{ i }}}}\n::end-map\n"),
        );
        assert_eq!(preflight(&root, &gated).unwrap().candidates.len(), 1);
        let filtered = write(
            &root,
            "filtered.md",
            &format!("{NUMBERS}::filter data \"n => false\"\n::shell echo {{{{ n }}}}\n::end-filter\n"),
        );
        assert_eq!(preflight(&root, &filtered).unwrap().candidates.len(), 1);
    }

    #[test]
    fn a_shell_command_without_a_local_keeps_its_resolved_bytes() {
        let report = candidate_of("::map data\n::shell echo fixed\n::end-map\n");
        assert_eq!(report.entries.len(), 1);
        assert_eq!(report.entries[0].normalized, "echo fixed");
        assert_eq!(report.candidates[0].requirement, ModeRequirement::AnyMode);
    }

    #[test]
    fn an_unrelated_approval_time_hole_still_fails_when_it_is_missing() {
        let (_guard, root) = root();
        let path = write(
            &root,
            "doc.md",
            &format!("{NUMBERS}::map data\n::shell echo {{{{ i }}}} {{{{ absent }}}}\n::end-map\n"),
        );
        let error = preflight(&root, &path).expect_err("`absent` is not a local and has no value");
        match &error {
            MarkdownError::ShellExpansion(inner) => assert!(
                matches!(&**inner, ShellExpansionError::MissingValue { expression, .. } if expression == "absent"),
                "{error}"
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_local_cannot_supply_the_executable() {
        let (_guard, root) = root();
        let path = write(&root, "doc.md", &format!("{NUMBERS}::map data\n::shell {{{{ i }}}} arg\n::end-map\n"));
        let error = preflight(&root, &path).expect_err("an executable is never a hole");
        assert!(matches!(error, MarkdownError::ShellExpansion(_)), "{error:?}");
    }

    struct Approve(Mutex<Vec<String>>);

    impl ShellApprovalHandler for Approve {
        fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
            self.0.lock().unwrap().push(
                request.template.map_or(request.normalized_exact, |template| template.display),
            );
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    fn compose(body: &str, items: &str) -> (Result<String, MarkdownError>, Vec<String>) {
        let (_guard, root) = root();
        let document = format!("---\n$schema:\n  data: any[](required)\ndata: {items}\n---\n{body}");
        let path = write(&root, "doc.md", &document);
        let recorder = Arc::new(Approve(Mutex::default()));
        let result = Markdown::try_from(path.as_path()).unwrap().compose_with(&crate::request_support::request(
            options_for(&root, &path).with_shell_approval_handler(recorder.clone()),
        ));
        let approvals = recorder.0.lock().unwrap().clone();
        (result.map(|(composed, _)| composed.content().to_string()), approvals)
    }

    #[test]
    fn a_local_argument_is_one_argv_element_per_copy_and_approved_once_as_a_template() {
        let (output, approvals) = compose(
            "::map data\n::shell echo {{ i }}\n::end-map\n",
            "['a b', ';touch x', 3]",
        );
        let output = output.unwrap_or_else(|error| panic!("{error}"));
        for expected in ["a b", ";touch x", "3"] {
            assert!(output.contains(expected), "{output}");
        }
        assert_eq!(approvals.len(), 1, "one authored template, however many copies: {approvals:?}");
    }

    #[test]
    fn a_null_or_container_local_fails_the_shell_input_check_and_runs_nothing() {
        for item in ["null", "[1]", "{a: 1}"] {
            let (output, _) = compose("::map data\n::shell echo {{ i }}\n::end-map\n", &format!("[{item}]"));
            let error = output.expect_err(item);
            assert!(matches!(error.without_iteration(), MarkdownError::ShellExpansion(_)), "{item}: {error:?}");
            let [record] = error_diagnostics(&error).try_into().expect("one record");
            assert_eq!(record.iteration_path.to_string(), "data[0]", "{item}: {record:?}");
            assert_eq!(record.source.as_ref().map(|source| source.line), Some(7), "{item}: {record:?}");
        }
    }
}

// ── scoped children ──────────────────────────────────────────────────────

mod scoped_children {
    use super::*;

    fn names_doc(body: &str, names: &str) -> String {
        format!("---\n$schema:\n  names: string[](required)\nnames: {names}\n---\n{body}")
    }

    #[test]
    fn every_candidate_child_is_discovered_with_its_authored_site_and_item() {
        let (_guard, root) = root();
        write(&root, "a.md", "::shell echo from-a\n");
        write(&root, "b.md", "::shell echo from-b\n");
        let path = write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[a.md, b.md]"));
        let report = preflight(&root, &path).unwrap_or_else(|error| panic!("{error}"));
        let normalized: Vec<_> = report.entries.iter().map(|entry| entry.normalized.as_str()).collect();
        assert_eq!(normalized, ["echo from-a", "echo from-b"]);
        let sites: Vec<_> = report
            .preflight_graph
            .edges
            .iter()
            .map(|edge| {
                let site = edge.site.as_ref().expect("a repeated site");
                (site.line, site.iteration.to_string())
            })
            .collect();
        assert_eq!(sites, [(7, "names[0]".to_string()), (7, "names[1]".to_string())]);
    }

    #[test]
    fn children_are_discovered_before_gates_filters_and_stopping() {
        let (_guard, root) = root();
        write(&root, "a.md", "::shell echo from-a\n");
        write(&root, "b.md", "::shell echo from-b\n");
        for (header, local, closer) in [
            ("::filter names \"n => false\"", "n", "::end-filter"),
            ("::map names when=\"false\"", "i", "::end-map"),
            ("::map names while=\"n => false\"", "i", "::end-map"),
            ("::map names filter=\"n => n == 'zzz'\"", "i", "::end-map"),
        ] {
            let path = write(&root, "doc.md", &names_doc(&format!("{header}\n::file {{{{ {local} }}}}\n{closer}\n"), "[a.md, b.md]"));
            let report = preflight(&root, &path).unwrap_or_else(|error| panic!("{header}: {error}"));
            assert_eq!(report.entries.len(), 2, "{header}: {:?}", report.entries);
        }
    }

    #[test]
    fn discovery_consumes_none_of_the_execution_allowances() {
        let (_guard, root) = root();
        write(&root, "a.md", "plain a\n");
        write(&root, "b.md", "plain b\n");
        let path = write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[a.md, b.md]"));
        let one_visit = || options_for(&root, &path).with_max_iteration_visits(std::num::NonZeroU64::new(1).unwrap());
        let report = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_preflight(&crate::request_support::request(one_visit()))
            .unwrap_or_else(|error| panic!("discovery visited two items against a one-visit limit: {error}"));
        assert_eq!(report.preflight_graph.edges.len(), 2);
        let error = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(one_visit()))
            .expect_err("execution is charged");
        assert!(matches!(error, MarkdownError::Collection(_)), "{error:?}");
    }

    #[test]
    fn an_empty_source_has_no_children_and_no_error() {
        let (_guard, root) = root();
        let path = write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[]"));
        let report = preflight(&root, &path).unwrap();
        assert!(report.preflight_graph.edges.is_empty());
    }

    #[test]
    fn one_target_with_different_explicit_values_is_two_children() {
        let (_guard, root) = root();
        write(&root, "child.md", "---\ntag: none\n---\n::shell echo {{ tag }}\n");
        let path = write(
            &root,
            "doc.md",
            &names_doc("::map names\n::file ./child.md set.tag=\"{{ i }}\"\n::end-map\n", "[x, y]"),
        );
        let report = preflight(&root, &path).unwrap_or_else(|error| panic!("{error}"));
        let edges = &report.preflight_graph.edges;
        assert_eq!(edges.len(), 2);
        assert_ne!(edges[0].parameters, edges[1].parameters);
        let normalized: Vec<_> = report.entries.iter().map(|entry| entry.normalized.as_str()).collect();
        assert_eq!(normalized, ["echo x", "echo y"], "each value's commands are approved");
    }

    #[test]
    fn a_source_a_shell_command_has_yet_to_produce_is_rejected_at_its_target() {
        let (_guard, root) = root();
        write(&root, "a.md", "plain\n");
        let path = write(
            &root,
            "doc.md",
            "---\n$schema:\n  names: string[]\nnames: \"$(printf a.md)\"\n---\n::map names\n::file {{ i }}\n::end-map\n",
        );
        let error = preflight(&root, &path).expect_err("a pending source cannot name a child");
        match &error {
            MarkdownError::ShellExpansion(inner) => assert!(
                matches!(&**inner, ShellExpansionError::DynamicCommandShape { key, .. } if key == "names"),
                "{error}"
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_target_discovered_for_no_site_is_not_approved_late() {
        let (_guard, root) = root();
        write(&root, "a.md", "plain a\n");
        write(&root, "b.md", "plain b\n");
        write(&root, "c.md", "plain c\n");
        let path = write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[a.md, b.md]"));
        let report = preflight(&root, &path).unwrap();
        // The source changes after discovery: the new target has no edge.
        write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[a.md, c.md]"));
        let options = options_for(&root, &path).with_preflight_graph(report.preflight_graph.clone());
        let error = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(options))
            .expect_err("c.md was never discovered");
        assert!(
            matches!(error.without_iteration(), MarkdownError::ShellExpansion(inner) if matches!(**inner, ShellExpansionError::NotPreApproved { .. })),
            "{error}"
        );
        let [record] = error_diagnostics(&error).try_into().expect("one record");
        assert_eq!(record.iteration_path.to_string(), "names[1]", "{record:?}");

        // Control: the document discovery walked composes.
        write(&root, "doc.md", &names_doc("::map names\n::file {{ i }}\n::end-map\n", "[a.md, b.md]"));
        let options = options_for(&root, &path).with_preflight_graph(report.preflight_graph);
        let (composed, _) = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(options))
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(composed.content().contains("plain a") && composed.content().contains("plain b"));
    }

    #[test]
    fn multiline_child_output_stays_inside_the_list_item_it_was_written_in() {
        let (_guard, root) = root();
        write(&root, "a.md", "line one\nline two\n");
        let path = write(
            &root,
            "doc.md",
            &names_doc("::map names\n- entry\n  ::file {{ i }}\n::end-map\n", "[a.md]"),
        );
        let (composed, _) = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(
                options_for(&root, &path).disable(ComposeOperation::Cleanup),
            ))
            .unwrap_or_else(|error| panic!("{error}"));
        let output = composed.content();
        assert!(output.contains("- entry\n  line one\n  line two"), "{output}");
    }
}

// ── cache semantics ──────────────────────────────────────────────────────

mod cache {
    use std::num::NonZeroU64;

    use super::*;

    fn limit(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).unwrap()
    }

    fn compose_with_visits(visits: u64) -> Result<String, MarkdownError> {
        let (_guard, root) = root();
        write(
            &root,
            "child.md",
            "---\n$schema:\n  data: number[](required)\ndata: [1, 2]\n---\n::map data\n- {{ i }}\n::end-map\n",
        );
        let path = write(
            &root,
            "doc.md",
            "---\n$schema:\n  names: string[](required)\nnames: [x, y]\n---\n::map names\n::file ./child.md\n::end-map\n",
        );
        Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(
                options_for(&root, &path).with_max_iteration_visits(limit(visits)),
            ))
            .map(|(composed, _)| composed.content().to_string())
    }

    #[test]
    fn a_reused_child_replays_the_visits_its_first_composition_paid() {
        // Two parent visits, the child's two visits computed once, and the
        // same two replayed for the reuse.
        let output = compose_with_visits(6).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(output.matches("- 1").count(), 2, "{output}");
        let error = compose_with_visits(5).expect_err("the reuse is not free");
        assert!(matches!(error, MarkdownError::Collection(_)), "{error:?}");
    }

    #[test]
    fn equal_values_in_another_key_order_do_not_share_a_cached_child() {
        let (_guard, root) = root();
        write(
            &root,
            "child.md",
            "---\n$schema:\n  cfg: '{ a: number(required), b: number(required) }(required)'\n---\n::map cfg\n- {{ i.key }}\n::end-map\n",
        );
        let path = write(
            &root,
            "doc.md",
            "::file ./child.md set.cfg='{\"a\":1,\"b\":2}'\n\n::file ./child.md set.cfg='{\"b\":2,\"a\":1}'\n",
        );
        let (composed, _) = Markdown::try_from(path.as_path())
            .unwrap()
            .compose_with(&crate::request_support::request(options_for(&root, &path)))
            .unwrap_or_else(|error| panic!("{error}"));
        let output = composed.content();
        let first = output.find("- a").expect("a");
        let second = output.rfind("- a").expect("a again");
        assert!(output.find("- b").unwrap() > first, "{output}");
        assert!(output.rfind("- b").unwrap() < second, "the second copy iterates b first: {output}");
    }
}

// ── reference validation ─────────────────────────────────────────────────

mod reference_validation {
    use darkmatter::markdown::reference::ReferenceGraphOptions;
    use darkmatter::markdown::reference::validate::ReferenceValidationOptions;

    use super::*;

    #[test]
    fn validation_reads_repeated_bodies_without_running_callbacks_or_shell() {
        let (_guard, root) = root();
        write(&root, "a.md", "# A\n");
        write(&root, "b.md", "# B\n");
        let path = write(
            &root,
            "doc.md",
            "---\n$schema:\n  names: string[](required)\nnames: [a.md, b.md]\n---\n\
             ::map names\n::shell touch ran-{{ i }}\n\n::file {{ i }}\n::end-map\n",
        );
        let compose = crate::request_support::request(options_for(&root, &path));
        let graph = ReferenceGraphOptions::with_compose(&compose);
        let report = Markdown::try_from(path.as_path())
            .unwrap()
            .validate_references(ReferenceValidationOptions::with_graph(graph))
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(report.is_valid(), "{:?}", report.issues);
        assert!(!root.join("ran-a.md").exists() && !root.join("ran-b.md").exists());
    }
}
