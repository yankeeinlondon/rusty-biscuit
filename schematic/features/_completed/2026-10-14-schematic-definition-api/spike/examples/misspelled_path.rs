use schematic_registration_spike::{DefinitionContext, models, rust_path};
fn main() {
    let _ = DefinitionContext::default().register_as::<models::Model>(rust_path!(models::DoesNotExist));
}
