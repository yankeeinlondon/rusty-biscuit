use pager_surface::{encoding_a1::Pager1, generated::*};
fn main() {
    let _ = Pager1::<ListRoles>::new().page(2);
}
