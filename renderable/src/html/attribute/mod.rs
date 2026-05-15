pub mod aria;
pub mod rel;

pub struct DomId(String);
pub struct ClassDefinition(String);
/// the unique name component of a `data-{name}` HTML data attribute
pub struct HtmlDataAttribute(String);
