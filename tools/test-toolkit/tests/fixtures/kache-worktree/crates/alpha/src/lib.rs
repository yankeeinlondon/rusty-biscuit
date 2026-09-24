pub fn base(value: u64) -> u64 {
    value.wrapping_mul(31).wrapping_add(7)
}
