pub fn top(value: u64) -> u64 {
    kache_fixture_beta::middle(value).wrapping_add(1)
}
