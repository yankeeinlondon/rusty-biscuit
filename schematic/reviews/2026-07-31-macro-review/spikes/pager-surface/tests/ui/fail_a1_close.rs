use pager_surface::{encoding_a1::Pager1, generated::*};
fn main() {
    Pager1::<ListRoles>::new().pages().close();
}
