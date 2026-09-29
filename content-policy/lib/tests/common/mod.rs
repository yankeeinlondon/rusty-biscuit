//! Helpers shared by the integration test binaries.

#![allow(dead_code)]

/// The 23 repository documents migrated from `Duration(...)` and
/// `update_policy`, by short name. Each read is an `include_bytes!`, so an
/// edit to one of them schedules the tests that use this list.
pub const MIGRATED_DOCUMENTS: [(&str, &[u8]); 23] = [
    ("tm/about", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/about.md")),
    ("tm/cmux", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/cmux.md")),
    ("tm/ghostty", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/ghostty.md")),
    ("tm/tmux", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/tmux.md")),
    ("tm/wezterm", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/wezterm.md")),
    ("tm/zellij", include_bytes!("../../../../biscuit-terminal/docs/research/terminal-multiplexing/zellij.md")),
    ("acp/gemini-cli", include_bytes!("../../../../claudine/docs/research/acp/gemini-cli.md")),
    ("acp/json-rpc", include_bytes!("../../../../claudine/docs/research/acp/json-rpc.md")),
    ("acp/kimi-code-cli", include_bytes!("../../../../claudine/docs/research/acp/kimi-code-cli.md")),
    ("playa/Android", include_bytes!("../../../../.claude/skills/playa/audio-programming/Android.md")),
    ("playa/crates", include_bytes!("../../../../.claude/skills/playa/audio-programming/crates.md")),
    ("playa/IOS", include_bytes!("../../../../.claude/skills/playa/audio-programming/IOS.md")),
    ("playa/linux", include_bytes!("../../../../.claude/skills/playa/audio-programming/linux.md")),
    ("playa/macOS", include_bytes!("../../../../.claude/skills/playa/audio-programming/macOS.md")),
    ("playa/typescript-libraries", include_bytes!("../../../../.claude/skills/playa/audio-programming/typescript-libraries.md")),
    ("playa/windows", include_bytes!("../../../../.claude/skills/playa/audio-programming/windows.md")),
    ("sniff/Android", include_bytes!("../../../../sniff/docs/research/audio-programming/Android.md")),
    ("sniff/crates", include_bytes!("../../../../sniff/docs/research/audio-programming/crates.md")),
    ("sniff/IOS", include_bytes!("../../../../sniff/docs/research/audio-programming/IOS.md")),
    ("sniff/linux", include_bytes!("../../../../sniff/docs/research/audio-programming/linux.md")),
    ("sniff/macOS", include_bytes!("../../../../sniff/docs/research/audio-programming/macOS.md")),
    ("sniff/typescript-libraries", include_bytes!("../../../../sniff/docs/research/audio-programming/typescript-libraries.md")),
    ("sniff/windows", include_bytes!("../../../../sniff/docs/research/audio-programming/windows.md")),
];
