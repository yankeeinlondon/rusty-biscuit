//! A host's lifecycle globals stay protected from `::map` / `::filter`
//! callback names, through Darkmatter's public collection analysis and the
//! binding view Claudine declares for each lifecycle scope.

use claudine::composition::lifecycle::bindings::{GroupMembership, LifecycleScope, binding_view};
use darkmatter::markdown::compose::collection_blocks::{CollectionAnalysisOptions, analyze_collection_blocks};
use darkmatter::markdown::compose::expression::{BindingView, DeclaredTypes, TypeScope};
use darkmatter::markdown::compose::ComposeOperationSet;

fn analyze(source: &str, view: Option<&BindingView>) -> Result<(), String> {
    let declared = DeclaredTypes::new(None);
    let site = TypeScope::unguarded(&declared);
    let options = CollectionAnalysisOptions {
        operations: ComposeOperationSet::all(),
        binding_view: view,
        // The source is undeclared; the recoveries waive only that requirement.
        allow_untyped_map_variables: true,
        allow_untyped_filter_variables: true,
        source_path: None,
    };
    analyze_collection_blocks(source, &site, &options).map(|_| ()).map_err(|error| error.code().to_string())
}

fn region(kind: &str, callback: &str) -> String {
    format!("::{kind} data \"{callback}\"\nbody\n::end-{kind}\n")
}

#[test]
fn every_declared_lifecycle_global_is_a_protected_callback_name_in_every_scope() {
    for scope in LifecycleScope::ALL {
        let view = binding_view(scope, GroupMembership::Unknown);
        let globals: Vec<String> = view.globals().map(|global| global.root().to_string()).collect();
        assert!(!globals.is_empty(), "{scope:?} declares no globals");
        for global in &globals {
            for kind in ["map", "filter"] {
                let source = region(kind, &format!("{global} => 1"));
                assert_eq!(
                    analyze(&source, Some(&view)),
                    Err("dm.expression.invalid_callback".to_string()),
                    "{scope:?} {kind} {global}"
                );
                // Protection follows the view in force: a plain document view allows the name.
                assert_eq!(analyze(&source, None), Ok(()), "{scope:?} {kind} {global} without the host view");
            }
        }
    }
}

#[test]
fn a_callback_may_read_a_declared_global_it_does_not_shadow() {
    let view = binding_view(LifecycleScope::Event(claudine::composition::lifecycle::LifecycleSignal::Loop), GroupMembership::Unknown);
    for kind in ["map", "filter"] {
        assert_eq!(analyze(&region(kind, "i => loop.count"), Some(&view)), Ok(()), "{kind}");
    }
}
