pub struct VerboseType {
    pub field_a: u32,
    pub field_b: String,
}

pub enum VerboseEnum {
    One,
    Two { value: u32 },
}
