//! Moving a checkout aside before it is deleted. The move is the quarantine:
//! it takes the checkout off its original path in one step, and it is also the
//! probe that a Windows holder blocks. [`rename`] is the no-replacement move
//! itself.

pub mod rename;
