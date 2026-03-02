/// Type exported through the fixture prelude for comment rendering tests.
pub struct VerboseType {
    pub field_a: u32,
    pub field_b: String,
}

/// Enum exported through the fixture prelude with aliasing.
pub enum VerboseEnum {
    One,
    Two { value: u32 },
}
