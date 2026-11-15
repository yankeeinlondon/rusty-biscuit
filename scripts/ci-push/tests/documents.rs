//! The JSON loader's syntax policy and every document field validator, each
//! walked once through its full shape matrix and then wired to every field
//! that shares it with one invalid shape.

use rstest::rstest;
use serde_json::{Map, Value, json};

use crate::ci_push::{
    ConstraintCell, DocumentError, PullRequest, parse_constraint_plan, parse_constraint_record,
    parse_pull_requests, parse_resolved_plan,
};

/// `control` with one edit applied to its root, serialized again.
fn edited(control: &str, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value: Value = serde_json::from_str(control).expect("fixture is JSON");
    edit(&mut value);
    serde_json::to_vec(&value).expect("serializes")
}

fn set(field: &'static str, to: Value) -> impl FnOnce(&mut Value) {
    move |root| {
        root.as_object_mut().expect("object").insert(field.to_owned(), to);
    }
}

fn remove(field: &'static str) -> impl FnOnce(&mut Value) {
    move |root| {
        root.as_object_mut().expect("object").remove(field);
    }
}

/// Applies `edit` to the first cell of a plan.
fn first_cell(edit: impl FnOnce(&mut Map<String, Value>)) -> impl FnOnce(&mut Value) {
    move |root| edit(root["cells"][0].as_object_mut().expect("cell object"))
}

fn assert_located(error: DocumentError, location: &str, problem: &str) {
    assert_eq!(error.location, location, "{error}");
    assert!(error.problem.contains(problem), "{error}");
}

// --- The one loader: syntax policy, through a constraint record control. ---

#[test]
fn the_constraint_control_record_parses() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let record = parse_constraint_record(control.as_bytes(), "minimal.json").expect("control parses");
    assert_eq!(record.environment, "wsl2-ubuntu");
    assert_eq!(record.reason, "do not rerun WSL for this branch");
    assert_eq!(record.owner, "ken");
    assert_eq!(record.expiry, chrono::NaiveDate::from_ymd_opt(2099, 1, 1).unwrap());
    assert_eq!((record.gate.as_str(), record.repository.as_str(), record.branch.as_str()), ("", "", ""));
}

#[test]
fn the_loader_refuses_a_duplicate_key() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    // serde_json's own `Value` would keep the second spelling and drop the first.
    let text = control.replacen('{', "{\n  \"environment\": \"macos-latest\",", 1);
    let error = parse_constraint_record(text.as_bytes(), "record.json").unwrap_err();
    assert_located(error, "", "duplicate key \"environment\"");
}

#[test]
fn the_loader_refuses_a_comment() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let text = format!("// recorded after the WSL run\n{control}");
    let error = parse_constraint_record(text.as_bytes(), "record.json").unwrap_err();
    assert_located(error, "", "is not valid JSON");
}

#[test]
fn the_loader_refuses_a_trailing_comma_before_a_closing_brace() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let text = control.trim_end().strip_suffix("\n}").unwrap().to_owned() + ",\n}";
    let error = parse_constraint_record(text.as_bytes(), "record.json").unwrap_err();
    assert_located(error, "", "is not valid JSON");
}

#[rstest]
#[case::garbage("garbage")]
#[case::second_document("{}")]
fn the_loader_refuses_content_after_the_value(#[case] trailer: &str) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let text = format!("{control}{trailer}");
    let error = parse_constraint_record(text.as_bytes(), "record.json").unwrap_err();
    assert_located(error, "", "has content after its JSON value");
}

#[rstest]
#[case::empty("")]
#[case::whitespace(" \n\t\n")]
fn the_loader_refuses_an_empty_document(#[case] text: &str) {
    let error = parse_constraint_record(text.as_bytes(), "record.json").unwrap_err();
    assert_located(error, "", "is not valid JSON");
}

#[test]
fn a_document_error_names_the_document_and_the_field() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let error = parse_constraint_record(&edited(control, set("owner", Value::Null)), "store/wsl.json").unwrap_err();
    assert_eq!(error.to_string(), "store/wsl.json: `owner` must be a nonempty string, found null");
}

// --- Required nonempty text: the full matrix through `environment`. ---

#[rstest]
#[case::absent(remove("environment"), "is required")]
#[case::null(set("environment", Value::Null), "found null")]
#[case::number(set("environment", json!(123)), "found a number")]
#[case::boolean(set("environment", json!(true)), "found a boolean")]
#[case::array(set("environment", json!(["wsl2-ubuntu"])), "found an array")]
#[case::object(set("environment", json!({})), "found an object")]
#[case::empty(set("environment", json!("")), "found \"\"")]
fn required_text_refuses_every_shape_but_a_nonempty_string(
    #[case] edit: impl FnOnce(&mut Value),
    #[case] problem: &str,
) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let error = parse_constraint_record(&edited(control, edit), "record.json").unwrap_err();
    assert_located(error, "environment", problem);
}

#[rstest]
#[case::reason("reason")]
#[case::owner("owner")]
#[case::expiry("expiry")]
fn every_required_constraint_field_is_wired_to_required_text(#[case] field: &'static str) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let error = parse_constraint_record(&edited(control, set(field, Value::Null)), "record.json").unwrap_err();
    assert_located(error, field, "found null");
    // An empty string is not absence for a required field either: `reason`
    // and `owner` never inherit the optional fields' wildcard.
    let error = parse_constraint_record(&edited(control, set(field, json!(""))), "record.json").unwrap_err();
    assert_located(error, field, "found \"\"");
}

// --- Calendar dates: `expiry`. ---

#[rstest]
#[case::invalid_day("2099-02-30")]
#[case::invalid_month("2099-13-01")]
#[case::basic_spelling("20990101")]
#[case::week_date("2099-W01-4")]
#[case::unpadded("2099-1-01")]
#[case::with_time("2099-01-01T00:00:00")]
#[case::surrounding_space(" 2099-01-01")]
fn expiry_must_be_a_real_yyyy_mm_dd_date(#[case] expiry: &str) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let error = parse_constraint_record(&edited(control, set("expiry", json!(expiry))), "record.json").unwrap_err();
    assert_located(error, "expiry", "must be a YYYY-MM-DD calendar date");
}

#[test]
fn a_leap_day_is_a_calendar_date() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/minimal.json");
    let record = parse_constraint_record(&edited(control, set("expiry", json!("2028-02-29"))), "record.json").unwrap();
    assert_eq!(record.expiry, chrono::NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());
}

// --- Optional wildcard text: the full matrix through `gate`. ---

#[rstest]
#[case::absent(remove("gate"), "")]
#[case::empty(set("gate", json!("")), "")]
#[case::named(set("gate", json!("L1")), "L1")]
fn an_optional_field_is_a_wildcard_when_absent_or_empty(
    #[case] edit: impl FnOnce(&mut Value),
    #[case] gate: &str,
) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    let record = parse_constraint_record(&edited(control, edit), "record.json").unwrap();
    assert_eq!(record.gate, gate);
    // The edit touched only `gate`.
    assert_eq!(record.branch, "feature");
}

#[rstest]
#[case::null(Value::Null, "found null")]
#[case::number(json!(1), "found a number")]
#[case::boolean(json!(false), "found a boolean")]
#[case::array(json!(["L1"]), "found an array")]
#[case::object(json!({"name": "L1"}), "found an object")]
fn an_optional_field_refuses_null_and_every_non_string(#[case] gate: Value, #[case] problem: &str) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    let error = parse_constraint_record(&edited(control, set("gate", gate)), "record.json").unwrap_err();
    assert_located(error, "gate", problem);
}

#[test]
fn the_scoped_control_record_reads_every_optional_field() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    let record = parse_constraint_record(control.as_bytes(), "scoped.json").unwrap();
    assert_eq!(record.gate, "L1");
    assert_eq!(record.repository, "github.com/yankeeinlondon/rusty-biscuit");
    assert_eq!(record.branch, "feature");
}

#[rstest]
#[case::repository("repository")]
#[case::branch("branch")]
fn every_optional_constraint_field_is_wired_to_the_wildcard_validator(#[case] field: &'static str) {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    let error = parse_constraint_record(&edited(control, set(field, Value::Null)), "record.json").unwrap_err();
    assert_located(error, field, "found null");
    let record = parse_constraint_record(&edited(control, remove(field)), "record.json").unwrap();
    assert_eq!(if field == "branch" { record.branch } else { record.repository }, "");
}

#[test]
fn a_constraint_record_refuses_an_unknown_field() {
    let control = include_str!("../../ci/fixtures/pre-push/constraints/scoped.json");
    // A misspelled `branch` must not widen the record to every branch.
    let error = parse_constraint_record(&edited(control, set("branchh", json!("feature"))), "record.json").unwrap_err();
    assert_located(error, "branchh", "is not a constraint field");
}

#[test]
fn a_constraint_record_must_be_an_object() {
    let error = parse_constraint_record(b"[]", "record.json").unwrap_err();
    assert_located(error, "", "must be an object, found an array");
}

// --- Required arrays: the full matrix through the plan's `cells`. ---

#[rstest]
#[case::absent(remove("cells"), "cells", "is required")]
#[case::null(set("cells", Value::Null), "cells", "must be an array, found null")]
#[case::whole_field_wrong(set("cells", json!({})), "cells", "must be an array, found an object")]
#[case::every_element_wrong(set("cells", json!([123])), "cells[0]", "must be an object, found a number")]
fn the_plan_cells_must_be_an_array_of_cells(
    #[case] edit: impl FnOnce(&mut Value),
    #[case] location: &str,
    #[case] problem: &str,
) {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let error = parse_resolved_plan(&edited(control, edit), "plan.json").unwrap_err();
    assert_located(error, location, problem);
}

#[test]
fn one_wrong_cell_among_valid_cells_is_not_filtered_out() {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let plan = edited(control, |root| root["cells"].as_array_mut().unwrap().push(json!(123)));
    let error = parse_resolved_plan(&plan, "plan.json").unwrap_err();
    assert_located(error, "cells[2]", "must be an object, found a number");
}

#[test]
fn an_empty_cells_array_is_a_plan_with_no_cells() {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let plan = parse_resolved_plan(&edited(control, set("cells", json!([]))), "plan.json").unwrap();
    assert!(plan.cells.is_empty());
}

#[rstest]
#[case::package("package")]
#[case::environment("environment")]
#[case::gate("gate")]
#[case::execution("execution")]
#[case::state("state")]
fn every_plan_cell_identity_field_is_wired_to_required_text(#[case] field: &'static str) {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let error = parse_resolved_plan(&edited(control, first_cell(|cell| { cell.remove(field); })), "plan.json")
        .unwrap_err();
    assert_located(error, &format!("cells[0].{field}"), "is required");
}

#[rstest]
#[case::base("base")]
#[case::head("head")]
fn the_plan_comparison_is_wired_to_required_text(#[case] field: &'static str) {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let error = parse_resolved_plan(&edited(control, set(field, json!(7))), "plan.json").unwrap_err();
    assert_located(error, field, "found a number");
}

#[test]
fn a_resolved_plan_must_be_an_object() {
    let error = parse_resolved_plan(b"[]", "plan.json").unwrap_err();
    assert_located(error, "", "must be an object, found an array");
}

#[test]
fn every_plan_fixture_is_a_valid_resolved_plan_that_keeps_unread_fields() {
    let fixtures = [
        include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json"),
        include_str!("../../ci/fixtures/pre-push/plans/plan-macos-two-packages.json"),
        include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-absent.json"),
        include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json"),
        include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-reused.json"),
    ];
    for fixture in fixtures {
        let plan = parse_resolved_plan(fixture.as_bytes(), "fixture").expect("valid plan");
        assert!(!plan.cells.is_empty());
        // Unknown fields are kept for consumers, not validated away.
        assert!(plan.document.get("schema_version").is_some());
        assert!(plan.document.get("builds").is_some());
        let view = parse_constraint_plan(fixture.as_bytes(), "fixture").expect("valid view");
        assert_eq!(view.len(), plan.cells.len());
    }
}

#[test]
fn the_executing_plan_reads_its_cells_in_order() {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
    let plan = parse_resolved_plan(control.as_bytes(), "plan.json").unwrap();
    let cells: Vec<_> = plan
        .cells
        .iter()
        .map(|cell| (cell.package.as_str(), cell.environment.as_str(), cell.gate.as_str(), cell.execution.as_str(), cell.state.as_str()))
        .collect();
    assert_eq!(
        cells,
        [
            ("alpha", "ubuntu-latest", "L1", "execute", "pending"),
            ("alpha", "wsl2-ubuntu", "L1", "execute", "pending"),
        ]
    );
}

// --- The constraint check's minimal plan view. ---

#[test]
fn the_constraint_view_reads_absent_state_as_no_override() {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-reused.json");
    let plan = edited(control, |root| {
        for cell in root["cells"].as_array_mut().unwrap() {
            cell.as_object_mut().unwrap().remove("state");
        }
    });
    let cells = parse_constraint_plan(&plan, "plan.json").unwrap();
    assert_eq!(
        cells[1],
        ConstraintCell {
            environment: "wsl2-ubuntu".into(),
            gate: "L1".into(),
            execution: "reuse".into(),
            state: None,
        }
    );
}

#[rstest]
#[case::null(Value::Null, "found null")]
#[case::number(json!(3), "found a number")]
#[case::empty(json!(""), "found \"\"")]
fn the_constraint_view_refuses_a_present_but_invalid_state(#[case] state: Value, #[case] problem: &str) {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-reused.json");
    let plan = edited(control, first_cell(|cell| { cell.insert("state".into(), state); }));
    let error = parse_constraint_plan(&plan, "plan.json").unwrap_err();
    assert_located(error, "cells[0].state", problem);
}

#[test]
fn the_constraint_view_is_wired_to_the_array_and_text_validators() {
    let control = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-reused.json");
    let error = parse_constraint_plan(&edited(control, set("cells", Value::Null)), "plan.json").unwrap_err();
    assert_located(error, "cells", "must be an array, found null");
    let error = parse_constraint_plan(&edited(control, first_cell(|cell| { cell.insert("gate".into(), json!(1)); })), "plan.json")
        .unwrap_err();
    assert_located(error, "cells[0].gate", "found a number");
}

// --- `gh pr list` output. ---

#[test]
fn the_pull_request_control_lists_each_number_and_base() {
    let control = include_str!("../../ci/fixtures/pre-push/pull-requests/two-open.json");
    let pulls = parse_pull_requests(control.as_bytes(), "`gh pr list` output").unwrap();
    assert_eq!(
        pulls,
        [
            PullRequest { number: 12, base_branch: "main".into() },
            PullRequest { number: 13, base_branch: "release".into() },
        ]
    );
}

#[test]
fn an_empty_pull_request_list_means_no_open_pull_request() {
    let control = include_str!("../../ci/fixtures/pre-push/pull-requests/none.json");
    assert_eq!(parse_pull_requests(control.as_bytes(), "gh").unwrap(), []);
}

#[test]
fn a_pull_request_tolerates_fields_gh_adds() {
    let control = include_str!("../../ci/fixtures/pre-push/pull-requests/two-open.json");
    let output = edited(control, |root| { root[0]["headRefName"] = json!("feature"); });
    assert_eq!(parse_pull_requests(&output, "gh").unwrap().len(), 2);
}

#[rstest]
#[case::object(json!({}), "", "must be an array, found an object")]
#[case::null(Value::Null, "", "must be an array, found null")]
#[case::every_element_wrong(json!(["12 main"]), "[0]", "must be an object, found a string")]
#[case::one_element_wrong(json!([{"number": 12, "baseRefName": "main"}, 13]), "[1]", "must be an object, found a number")]
fn the_pull_request_list_is_wired_to_the_array_validator(
    #[case] output: Value,
    #[case] location: &str,
    #[case] problem: &str,
) {
    let error = parse_pull_requests(&serde_json::to_vec(&output).unwrap(), "gh").unwrap_err();
    assert_located(error, location, problem);
}

#[rstest]
#[case::absent(None, "is required")]
#[case::null(Some(Value::Null), "found null")]
#[case::zero(Some(json!(0)), "found 0")]
#[case::negative(Some(json!(-12)), "found -12")]
#[case::float(Some(json!(12.0)), "found 12.0")]
#[case::string(Some(json!("12")), "found a string")]
#[case::boolean(Some(json!(true)), "found a boolean")]
fn a_pull_request_number_is_a_positive_integer(#[case] number: Option<Value>, #[case] problem: &str) {
    let control = include_str!("../../ci/fixtures/pre-push/pull-requests/two-open.json");
    let output = edited(control, |root| {
        let pull = root[1].as_object_mut().unwrap();
        match number {
            Some(number) => pull.insert("number".into(), number),
            None => pull.remove("number"),
        };
    });
    let error = parse_pull_requests(&output, "gh").unwrap_err();
    assert_located(error, "[1].number", problem);
}

#[test]
fn a_pull_request_base_is_wired_to_required_text() {
    let control = include_str!("../../ci/fixtures/pre-push/pull-requests/two-open.json");
    let output = edited(control, |root| { root[0]["baseRefName"] = json!(""); });
    let error = parse_pull_requests(&output, "gh").unwrap_err();
    assert_located(error, "[0].baseRefName", "found \"\"");
}
