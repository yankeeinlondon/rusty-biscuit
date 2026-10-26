use std::collections::HashMap;
use schematic_registration_spike::{DefinitionContext, models, rust_path};
fn main() {
    let mut context = DefinitionContext::default();
    let alias = context.register_as::<models::Model>(rust_path!(::schematic_registration_spike::models::Alias)).unwrap();
    let renamed = context.register_as::<models::Renamed>(rust_path!(::schematic_registration_spike::models::Reexport)).unwrap();
    let nested = context.register::<Option<Vec<HashMap<String, models::Model>>>>().unwrap();
    let foreign = context.register::<std::time::Duration>().unwrap();
    let generic = context.register::<std::ops::Range<u64>>().unwrap();
    context.finish().unwrap();
    let ordinary = DefinitionContext::types_in("::schematic_registration_spike::models").register::<models::Model>().unwrap();
    println!("fn main() {{");
    for descriptor in [ordinary, alias, renamed, nested, foreign, generic] {
        println!("    let _: ::std::option::Option<{}> = None;", descriptor.rust_path());
    }
    println!("}}");
}
