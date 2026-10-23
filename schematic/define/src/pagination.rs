//! Pagination: how a list endpoint pages, described once per API.
//!
//! An API declares a default [`Strategy`]; endpoint builders attach a
//! [`Collection`] for wrapper lists, overrides, and explicit opt-outs.
//! [`resolve`] checks the endpoint collection first, then infers a root array
//! from a GET's explicit `Vec<T>` response. [`caller_knobs`] describes controls
//! and [`validate_pagination`] checks strategy and parameter contracts.
//!
//! A strategy is one of four kinds ([`PageStrategy`]: page number, offset,
//! cursor, next link) plus shared dimensions: constants, totals, terminal
//! values, the follow-up request shape, a server-side [`Session`], and
//! [`Partition`]s. Followed links obey the endpoint's effective
//! [`CrossOrigin`](crate::CrossOrigin) policy.
//!
//! ## Examples
//!
//! ```
//! use schematic_define::pagination::{
//!     Collection, MoreSignal, PageSize, Strategy, ValueSource,
//! };
//!
//! let default = Strategy::page_number("page", MoreSignal::link_rel("next"))
//!     .page_size(PageSize::new("per_page").server_default(30).max(100))
//!     .build();
//! let collection = Collection::at("workflow_runs")
//!         .item_type("WorkflowRun")
//!         .paginate(default.clone().total(ValueSource::body("total_count")));
//! ```

mod collection;
mod relay;
mod resolve;
mod selector;
mod strategy;
mod validate;

pub use collection::{Alternate, Collection, Pagination};
pub use relay::Relay;
pub use resolve::{
    AdoptionOutcome, Direction, Knob, ManagedRole, ParamAdoption, ParamLocation, Resolution,
    ResolutionOrigin, ResolvedList, adopted_params, caller_knobs, param_adoptions, resolve,
    root_array_item_type, strategy_knobs,
};
pub(crate) use resolve::{path_params, request_inputs};
pub use selector::{ItemSelector, JsonPath, Step};
pub use strategy::{
    Boundary, Carry, CountUnit, CursorBuilder, CursorLeg, CursorPart, CursorSource,
    CursorStrategy, FixedParam, FollowUp, IndexBase, InputParam, KeepAlive, LinkSource,
    MoreSignal, NextLinkBuilder, NextLinkStrategy, OffsetBuilder, OffsetStrategy,
    PageNumberBuilder, PageNumberStrategy, PageSize, PageStrategy, Partition, Polarity, Session,
    SessionOpen, SessionStep, Strategy, ValueSource,
};
pub use validate::{
    PaginationDiagnostic, PaginationError, PaginationRuleGroup, validate_pagination,
    validate_strategy,
};
