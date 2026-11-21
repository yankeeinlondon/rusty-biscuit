//! Bounded groups of shipped REST modules for the tests that check generated
//! Rust output.
//!
//! Generating the whole catalog in one test exceeds the 5 s L1 budget, so
//! each such test is divided into one case per group plus, where the check
//! reads crate-level files (`lib.rs`, `prelude.rs`, `shared.rs`), one case
//! that renders only those. The guard below keeps every registered REST
//! module in exactly one group and every group under [`MAX_GROUP_ENDPOINTS`];
//! when a new API trips the bound, add a group and a case to each divided
//! test.

use schematic_define::RestApi;
use schematic_definitions::registry::{ApiRegistration, registrations, rest_apis};

/// The OpenAI module alone: it is as large as the rest of the catalog.
pub const OPENAI: &[&str] = &["openai"];

/// The modules before `h`.
pub const ANTHROPIC_TO_GITLAB: &[&str] =
    &["anthropic", "artificial_analysis", "bitbucket", "elevenlabs", "gitea", "github", "gitlab"];

/// The modules from `h`, except OpenAI.
pub const HUGGINGFACE_TO_UNFOLDED_CIRCLE: &[&str] = &[
    "huggingface",
    "lmstudio",
    "ollama",
    "emqx",
    "eversolo",
    "samsung_smart_tv",
    "unfolded_circle_core_rest",
];

/// Every group, each covered by one case of every divided test.
pub const GROUPS: &[&[&str]] = &[OPENAI, ANTHROPIC_TO_GITLAB, HUGGINGFACE_TO_UNFOLDED_CIRCLE];

/// The most endpoints one group may hold, a proxy for its generation cost:
/// the OpenAI group's cases are the slowest, at about 2.5 s alone.
pub const MAX_GROUP_ENDPOINTS: usize = 300;

/// The registered REST APIs of `modules`, in registration order, building
/// only those modules' definitions.
pub fn apis(modules: &[&str]) -> Vec<RestApi> {
    let selected: Vec<ApiRegistration> = registrations()
        .iter()
        .filter(|entry| entry.rest().is_some() && modules.contains(&entry.module))
        .copied()
        .collect();
    assert_eq!(
        selected.iter().map(|entry| entry.module).collect::<std::collections::BTreeSet<_>>().len(),
        modules.len(),
        "every module of {modules:?} is registered"
    );
    rest_apis(&selected)
}

#[test]
fn every_rest_module_is_in_exactly_one_bounded_generation_group() {
    let mut registered: Vec<&str> =
        registrations().iter().filter(|entry| entry.rest().is_some()).map(|entry| entry.module).collect();
    registered.sort_unstable();
    registered.dedup();
    let mut grouped: Vec<&str> = GROUPS.iter().flat_map(|group| group.iter().copied()).collect();
    grouped.sort_unstable();
    assert_eq!(grouped, registered, "each registered REST module must be in exactly one group");

    for group in GROUPS {
        let endpoints: usize = apis(group).iter().map(|api| api.endpoints.len()).sum();
        assert!(
            endpoints <= MAX_GROUP_ENDPOINTS,
            "{group:?} holds {endpoints} endpoints, over {MAX_GROUP_ENDPOINTS}; split it into another group"
        );
    }
}
