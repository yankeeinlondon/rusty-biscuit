//! Symbol-kind, visibility, and static fixture for Rust.

use std::fmt;

// ---- Functions and visibility levels ----

pub fn public_function() {}

pub(crate) fn crate_function() {}

pub(self) fn self_function() {}

fn private_function(parameter_value: u32) -> u32 {
    let local_value = parameter_value + 1;
    local_value
}

// ---- Constants, statics, aliases ----

pub const PUBLIC_CONSTANT: u32 = 1;

const PRIVATE_CONSTANT: u32 = 2;

pub static PUBLIC_STATIC: u32 = 3;

static mut PRIVATE_STATIC_MUT: u32 = 4;

pub type PublicAlias = u32;

type PrivateAlias = PublicAlias;

// ---- Modules ----

pub mod outer_module {
    pub fn module_public_function() {}

    fn module_private_function() {}

    pub(crate) mod crate_inner_module {
        pub(super) fn super_function() {}

        pub(in crate::outer_module) fn in_path_function() {}
    }
}

mod private_module {}

// ---- Struct, union, enum ----

pub struct PublicStruct {
    pub public_field: u32,
    pub(crate) crate_field: u32,
    private_field: u32,
}

struct PrivateStruct {
    pub public_field_in_private_struct: u32,
}

pub union PublicUnion {
    pub union_int_field: u32,
    union_float_field: f32,
}

pub enum PublicEnum {
    UnitVariant,
    TupleVariant(u8),
    StructVariant { variant_field: u16 },
}

enum PrivateEnum {
    PrivateEnumVariant,
}

// ---- Traits ----

pub trait PublicTrait {
    fn required_trait_method(&self) -> u32;

    fn provided_trait_method(&self) {}
}

trait PrivateTrait {
    fn private_trait_method(&self);
}

// ---- Inherent impl: methods, associated functions ----

impl PublicStruct {
    pub fn new_associated_function() -> Self {
        PublicStruct {
            public_field: 0,
            crate_field: 0,
            private_field: 0,
        }
    }

    pub fn instance_method(&self) -> u32 {
        self.private_field
    }

    fn private_instance_method(&mut self) {}

    pub(crate) fn crate_associated_function() {}
}

// ---- Trait implementations ----

impl PublicTrait for PublicStruct {
    fn required_trait_method(&self) -> u32 {
        1
    }
}

impl PrivateTrait for PrivateStruct {
    fn private_trait_method(&self) {}
}

impl fmt::Display for PublicStruct {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "PublicStruct")
    }
}

// ---- Function-local static storage ----

fn function_with_local_static() -> u32 {
    static LOCAL_STATIC: u32 = 5;
    LOCAL_STATIC
}
