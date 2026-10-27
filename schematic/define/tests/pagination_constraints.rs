//! Lowering a strategy's page size into the adopted parameter's value rules,
//! and rejecting declared rules that disagree with it.

use schematic_define::constraints::Constraints;
use schematic_define::pagination::{
    MoreSignal, PageSize, PageSizeConflict, PaginationError, ParamLocation, Strategy,
    adopted_page_size, page_size_constraints, validate_pagination,
};
use schematic_define::{DefinitionBundle, DefinitionContext, Endpoint, ParamDef, QueryParamType, RestApi};
use serde_json::Value;

const RULE: &str = "page-size-constraint-conflict";

fn strategy(size: PageSize) -> Strategy {
    Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(size).build()
}

/// `ListItems`, a root-array GET declaring `per_page`, under a default
/// page-number strategy sized by `size`.
fn build(size: PageSize, per_page: ParamDef) -> Result<RestApi, String> {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::get("ListItems", "/items").describes("Items")
        .returns_json::<Vec<serde_json::Value>>(&mut ctx)
        .query(per_page)
        .build().map_err(|error| error.to_string())?;
    RestApi::builder(&ctx, "Items", "https://example.test").describes("Items").no_auth()
        .default_pagination(strategy(size)).endpoints([endpoint]).build().map_err(|error| error.to_string())
}

fn per_page() -> ParamDef { ParamDef::new("per_page", QueryParamType::Integer) }

fn bounded() -> PageSize { PageSize::new("per_page").min(5).max(100) }

fn build_error(size: PageSize, rules: Constraints) -> String {
    build(size, per_page().constraints(rules)).expect_err("the declared rules disagree with the page size")
}

#[test]
fn agreeing_declared_bounds_pass() {
    let api = build(bounded(), per_page().constraints(Constraints::new().minimum(5).maximum("100.0"))).unwrap();
    let adopted = adopted_page_size(&api, &api.endpoints[0]).unwrap();
    assert_eq!(adopted.constraints.maximum.unwrap().as_str(), "100");
}

#[test]
fn param_without_rules_or_with_empty_rules_passes() {
    assert!(build(bounded(), per_page()).is_ok());
    assert!(build(bounded(), per_page().constraints(Constraints::new())).is_ok());
}

#[test]
fn format_only_passes_and_survives_into_effective_rules() {
    let api = build(bounded(), per_page().constraints(Constraints::new().format("int32"))).unwrap();
    let adopted = adopted_page_size(&api, &api.endpoints[0]).unwrap();
    assert_eq!((adopted.location, adopted.name.as_str()), (ParamLocation::Query, "per_page"));
    assert_eq!(adopted.constraints, Constraints::new().format("int32").minimum(5).maximum(100));
}

#[test]
fn conflicting_minimum_fails() {
    let error = build_error(bounded(), Constraints::new().minimum(1).maximum(100));
    assert!(error.contains(RULE) && error.contains("`per_page`"), "{error}");
    assert!(error.contains("minimum 1") && error.contains("min 5"), "{error}");
}

#[test]
fn conflicting_maximum_fails() {
    let error = build_error(bounded(), Constraints::new().minimum(5).maximum(50));
    assert!(error.contains(RULE) && error.contains("maximum 50") && error.contains("max 100"), "{error}");
}

#[test]
fn only_maximum_declared_fails_as_one_sided() {
    let error = build_error(bounded(), Constraints::new().maximum(100));
    assert!(error.contains(RULE) && error.contains("minimum none") && error.contains("min 5"), "{error}");
}

#[test]
fn only_minimum_declared_while_strategy_has_max_fails_as_one_sided() {
    let error = build_error(bounded(), Constraints::new().minimum(5));
    assert!(error.contains(RULE) && error.contains("maximum none") && error.contains("max 100"), "{error}");
}

#[test]
fn maximum_declared_while_strategy_has_none_fails() {
    let error = build_error(PageSize::new("per_page"), Constraints::new().minimum(1).maximum(100));
    assert!(error.contains(RULE) && error.contains("maximum 100") && error.contains("max none"), "{error}");
}

#[test]
fn extra_keyword_on_adopted_param_fails() {
    let error = build_error(bounded(), Constraints::new().multiple_of(5));
    assert!(error.contains(RULE) && error.contains("`per_page`") && error.contains("`multipleOf`"), "{error}");
}

#[test]
fn conflict_is_a_located_parameter_diagnostic() {
    let mut api = build(bounded(), per_page()).unwrap();
    api.endpoints[0].params.as_mut().unwrap().query[0].constraints = Some(Box::new(Constraints::new().maximum(10).multiple_of(2)));
    let diagnostics = validate_pagination(&api);
    let located: Vec<_> = diagnostics.iter().map(|diagnostic| (diagnostic.endpoint.as_deref(), &diagnostic.error)).collect();
    assert_eq!(located, vec![
        (Some("ListItems"), &PaginationError::PageSizeConstraintConflict { param: "per_page".into(), conflict: PageSizeConflict::Bounds {
            declared_minimum: None, declared_maximum: Some("10".into()), min: 5, max: Some(100) } }),
        (Some("ListItems"), &PaginationError::PageSizeConstraintConflict { param: "per_page".into(),
            conflict: PageSizeConflict::Unenforceable("multipleOf") }),
    ]);
    assert_eq!(diagnostics[0].error.rule(), RULE);
}

#[test]
fn lowering_returns_the_strategy_bounds() {
    assert_eq!(page_size_constraints(&bounded()), Constraints::new().minimum(5).maximum(100));
    assert_eq!(page_size_constraints(&PageSize::new("limit")), Constraints::new().minimum(1));
}

#[test]
fn api_build_error_names_endpoint_rule_and_parameter() {
    let error = build_error(bounded(), Constraints::new().minimum(5).maximum(50));
    assert!(error.contains("ListItems") && error.contains(RULE) && error.contains("`per_page`"), "{error}");
}

#[test]
fn bundle_load_reports_a_disagreeing_page_size_rule() {
    let canonical = DefinitionBundle::new(build(bounded(), per_page()).unwrap()).unwrap().to_json();
    assert!(DefinitionBundle::from_json(&canonical).is_ok());
    let mut value: Value = serde_json::from_str(&canonical).unwrap();
    value["api"]["endpoints"][0]["params"]["query"][0]["constraints"] =
        serde_json::to_value(Constraints::new().minimum(5).maximum(50)).unwrap();
    let error = DefinitionBundle::from_json(&value.to_string()).unwrap_err().to_string();
    assert!(error.contains("ListItems") && error.contains(RULE) && error.contains("`per_page`"), "{error}");
    assert!(error.contains("maximum 50") && error.contains("max 100"), "{error}");
}
