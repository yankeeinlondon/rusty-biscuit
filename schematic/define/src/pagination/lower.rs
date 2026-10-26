//! Lowering of a strategy's page size into value constraints on the
//! parameter it adopts.

use super::resolve::{ManagedRole, ParamLocation, adopted_params, resolve};
use super::strategy::PageSize;
use super::validate::{PageSizeConflict, declared_param};
use crate::constraints::{ConstraintNumber, Constraints};
use crate::params::ParamDef;
use crate::types::{Endpoint, RestApi};

/// The value rules a page size implies: `minimum` is the size's `min`, and
/// `maximum` its `max` when it has one.
///
/// These are the bounds the pager enforces on the size it sends, so a
/// declared parameter the strategy adopts as its page size is documented
/// and exported with them.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{PageSize, page_size_constraints};
///
/// let rules = page_size_constraints(&PageSize::new("per_page").min(5).max(100));
/// assert_eq!(rules.minimum.unwrap().as_str(), "5");
/// assert_eq!(rules.maximum.unwrap().as_str(), "100");
/// assert!(page_size_constraints(&PageSize::new("per_page")).maximum.is_none());
/// ```
pub fn page_size_constraints(size: &PageSize) -> Constraints {
    let rules = Constraints::new().minimum(size.min);
    match size.max {
        Some(max) => rules.maximum(max),
        None => rules,
    }
}

/// The declared page-size parameter an endpoint's resolved strategy adopts,
/// with its effective value rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptedPageSize {
    pub location: ParamLocation,
    /// The name as the endpoint declares it.
    pub name: String,
    /// The declared rules (a valid definition carries at most `format`
    /// besides bounds that agree) with `minimum` and `maximum` taken from
    /// [`page_size_constraints`].
    pub constraints: Constraints,
}

/// The page-size parameter `endpoint`'s primary strategy adopts in `api`,
/// and its effective constraints.
///
/// `None` when the endpoint is not paginated, its strategy has no page size,
/// or the endpoint declares no parameter the size adopts. Alternates adopt
/// nothing and are ignored.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{
///     MoreSignal, PageSize, ParamLocation, Strategy, adopted_page_size,
/// };
/// use schematic_define::{DefinitionContext, Endpoint, ParamDef, QueryParamType, RestApi};
///
/// let mut ctx = DefinitionContext::types_in("::models");
/// let endpoint = Endpoint::get("ListItems", "/items").describes("Items")
///     .returns_json::<Vec<serde_json::Value>>(&mut ctx)
///     .query(ParamDef::new("per_page", QueryParamType::Integer))
///     .build().unwrap();
/// let api = RestApi::builder(&ctx, "Items", "https://example.test").describes("Items").no_auth()
///     .default_pagination(Strategy::page_number("page", MoreSignal::NonEmptyPage)
///         .page_size(PageSize::new("per_page").max(100)).build())
///     .endpoints([endpoint]).build().unwrap();
///
/// let adopted = adopted_page_size(&api, &api.endpoints[0]).unwrap();
/// assert_eq!((adopted.location, adopted.name.as_str()), (ParamLocation::Query, "per_page"));
/// assert_eq!(adopted.constraints.maximum.unwrap().as_str(), "100");
/// ```
pub fn adopted_page_size(api: &RestApi, endpoint: &Endpoint) -> Option<AdoptedPageSize> {
    let resolution = resolve(api, endpoint);
    let strategy = resolution.strategy()?;
    let size = strategy.kind.size()?;
    let adoption = adopted_params(endpoint, strategy).into_iter().find(|adoption| adoption.role == ManagedRole::PageSize)?;
    let lowered = page_size_constraints(size);
    let mut constraints = declared_param(endpoint, adoption.location, &adoption.name)
        .and_then(|param| param.constraints.as_deref().cloned())
        .unwrap_or_default();
    constraints.minimum = lowered.minimum;
    constraints.maximum = lowered.maximum;
    Some(AdoptedPageSize { location: adoption.location, name: adoption.name, constraints })
}

/// How a declared page-size parameter's own rules disagree with `size`.
///
/// Bounds must be absent together or equal the size's exactly; every other
/// value keyword is a rule the pager, which chooses the value, cannot honor.
/// `format` is an annotation and always allowed.
pub(super) fn page_size_conflicts(param: &ParamDef, size: &PageSize) -> Vec<PageSizeConflict> {
    let Some(rules) = param.constraints.as_ref().filter(|rules| !rules.is_empty()) else {
        return Vec::new();
    };
    // Exhaustive so a new keyword must be classified here.
    let Constraints {
        min_length, max_length, pattern, pattern_unenforced: _, format: _, minimum, maximum,
        exclusive_minimum, exclusive_maximum, multiple_of, min_items, max_items, unique_items,
        allowed_values, constant, items,
    } = &**rules;

    let mut conflicts = Vec::new();
    if minimum.is_some() || maximum.is_some() {
        let min_ok = minimum.as_ref().is_some_and(|declared| same_number(declared, size.min));
        let max_ok = match (maximum, size.max) {
            (None, None) => true,
            (Some(declared), Some(max)) => same_number(declared, max),
            _ => false,
        };
        if !(min_ok && max_ok) {
            conflicts.push(PageSizeConflict::Bounds {
                declared_minimum: minimum.as_ref().map(ToString::to_string),
                declared_maximum: maximum.as_ref().map(ToString::to_string),
                min: size.min,
                max: size.max,
            });
        }
    }
    let unenforceable = [
        ("exclusiveMinimum", exclusive_minimum.is_some()),
        ("exclusiveMaximum", exclusive_maximum.is_some()),
        ("multipleOf", multiple_of.is_some()),
        ("enum", allowed_values.is_some()),
        ("const", constant.is_some()),
        ("minLength", min_length.is_some()),
        ("maxLength", max_length.is_some()),
        ("pattern", pattern.is_some()),
        ("minItems", min_items.is_some()),
        ("maxItems", max_items.is_some()),
        ("uniqueItems", *unique_items),
        ("items", items.is_some()),
    ];
    conflicts.extend(
        unenforceable.into_iter().filter(|(_, present)| *present).map(|(keyword, _)| PageSizeConflict::Unenforceable(keyword)),
    );
    conflicts
}

/// Whether `declared` is exactly `value`; malformed text never is.
fn same_number(declared: &ConstraintNumber, value: u32) -> bool {
    match (declared.exact(), ConstraintNumber::from(value).exact()) {
        (Ok(declared), Ok(value)) => declared == value,
        _ => false,
    }
}
