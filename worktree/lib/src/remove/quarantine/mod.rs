//! Moving a checkout aside before it is deleted. The move is the quarantine:
//! it takes the checkout off its original path in one step, and it is also the
//! probe that a Windows holder blocks. [`naming`] picks the hidden sibling,
//! [`rename`] is the no-replacement move itself, [`association`] keeps
//! Git's registration on the moved checkout, and [`unregister`] removes that
//! registration once the quarantine is verified gone.

pub mod association;
pub mod naming;
pub mod rename;
pub mod unregister;
