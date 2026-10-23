//! Moving a checkout aside before it is deleted. The move is the quarantine:
//! it takes the checkout off its original path in one step, and it is also the
//! probe that a Windows holder blocks. [`placement`] decides whether the
//! checkout is moved or deleted in place, [`naming`] picks the hidden
//! sibling, [`rename`] is the no-replacement move itself, and [`association`]
//! keeps Git's registration on the moved checkout.

pub mod association;
pub mod naming;
pub mod placement;
pub mod rename;
