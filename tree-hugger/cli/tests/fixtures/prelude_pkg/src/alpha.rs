/// Type exported through the fixture prelude for comment rendering tests.
#[derive(Debug)]
pub struct VerboseType {
    pub field_a: u32,
    pub field_b: String,
}

/// Enum exported through the fixture prelude with aliasing.
#[derive(Debug)]
pub enum VerboseEnum {
    One,
    Two { value: u32 },
}
