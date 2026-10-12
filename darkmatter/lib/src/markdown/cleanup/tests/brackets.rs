use super::*;

// ==================== Brackets ====================

#[test]
fn bracketed_status_indicators_are_unchanged() {
    for content in [
        "- [0%] started\n- [25%] progress\n- [50%] halfway\n- [75%] almost\n- [100%] done",
        "- [!] blocked task",
    ] {
        assert_eq!(clean(content), content);
    }
}

#[test]
fn link_is_unchanged() {
    let content = "Check out [this link](https://example.com)";
    assert_eq!(clean(content), content);
}

#[test]
fn task_list_checkboxes_are_unchanged() {
    let content = "- [x] done\n- [ ] pending";
    assert_eq!(clean(content), content);
}
