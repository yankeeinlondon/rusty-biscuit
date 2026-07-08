//! Root-level subcommand menu for `claudine <TAB>`.
//!
//! Phase 1 of the `2026-04-24-improved-shell-completions` feature. The
//! candidate set is a fixed, spec-ordered list — no fuzzy matching, no
//! alphabetical reshuffle. Users see the same grouping every time they
//! press `<TAB>` at the subcommand slot:
//!
//! 1. Composition — `compose`, `inline-compose`, `sequence`.
//! 2. Wrappers — one token per compiled provider in
//!    `PROVIDERS_DISPLAY_ORDER` (currently `claude`, `codex`, `gemini`,
//!    `goose`, `kimi`, `opencode`, `qwen`, `kilo`, `pi`, `antigravity`),
//!    **derived from the provider catalog** so a newly onboarded provider
//!    appears automatically — never hand-maintained here.
//! 3. Shared resources — `skills`, `commands`, `agents`, `mcp`.
//! 4. Hooks / actions — `hooks`, `actions`.
//! 5. Administration — `sync`, `uninstall`, `providers`, `logs`,
//!    `completions`, `config`.
//! 6. `init` — **only** when no configuration exists that the wizard
//!    would overwrite. See [`should_offer_init`].
//!
//! The only flag emitted at the root slot is `--help`. Global flags
//! (`--verbose`, `--debug`, `--plain`) are discoverable from
//! `claudine --help` and intentionally not listed here.

use claudine::provider::PROVIDERS_DISPLAY_ORDER;

use crate::completion::engine::{RootContext, RootPartial};

/// The wrapper subcommand tokens, one per compiled provider in display
/// order — each provider's primary CLI alias, which equals its wrapper
/// subcommand name. Derived from the catalog so onboarding a provider adds
/// its `claudine <slug>` completion automatically (mirrors the derived argv
/// Rule 1 in [`crate::argv`]).
pub(crate) fn wrapper_tokens() -> Vec<&'static str> {
    PROVIDERS_DISPLAY_ORDER
        .iter()
        .filter_map(|provider| provider.cli_aliases().first().copied())
        .collect()
}

/// Render the root-slot candidate list for a given partial and context.
///
/// - `RootPartial::Empty` → the full menu.
/// - `RootPartial::Word(prefix)` → menu filtered by prefix match.
/// - `RootPartial::FlagLike(prefix)` → only `--help` when the prefix is a
///   compatible partial form.
pub(crate) fn render(partial: &RootPartial, ctx: &RootContext) -> Vec<String> {
    match partial {
        RootPartial::FlagLike(prefix) => flag_candidates(prefix),
        RootPartial::Empty => full_menu(ctx),
        RootPartial::Word(prefix) => full_menu(ctx)
            .into_iter()
            .filter(|c| c.starts_with(prefix))
            .collect(),
    }
}

/// Return `--help` when the partial is a compatible prefix of it.
///
/// Accepted triggers are `-`, `--`, `-h`, `--h`, `--he`, `--hel`, `--help`.
/// Anything else (e.g. `--version`, `-x`) yields zero candidates so the
/// shell falls back to its default flag completion.
fn flag_candidates(prefix: &str) -> Vec<String> {
    if prefix == "-" || prefix == "-h" || "--help".starts_with(prefix) {
        vec!["--help".to_string()]
    } else {
        Vec::new()
    }
}

/// Assemble the full root menu in spec order.
pub(crate) fn full_menu(ctx: &RootContext) -> Vec<String> {
    let mut menu: Vec<&str> = Vec::with_capacity(24);

    // 1. Composition.
    menu.extend_from_slice(&["compose", "inline-compose", "sequence"]);
    // 2. Wrappers — derived from the provider catalog (see wrapper_tokens),
    //    so kilo/pi/antigravity and any future provider appear automatically.
    menu.extend(wrapper_tokens());
    // 3. Shared resources.
    menu.extend_from_slice(&["skills", "commands", "agents", "mcp"]);
    // 4. Hooks and actions. (Spec says "hooks and events"; the clap surface
    //    registers `actions`, which is what we offer here.)
    menu.extend_from_slice(&["hooks", "actions"]);
    // 5. Administration.
    menu.extend_from_slice(&[
        "sync",
        "uninstall",
        "providers",
        "logs",
        "completions",
        "config",
    ]);

    if should_offer_init(ctx) {
        menu.push("init");
    }

    menu.into_iter().map(String::from).collect()
}

/// Decide whether `init` should appear in the menu.
///
/// The rule mirrors the spec literally: when in a repo, offer `init` only
/// if neither user nor repo config exists. When outside a repo, offer
/// `init` only if no user config exists.
pub(crate) fn should_offer_init(ctx: &RootContext) -> bool {
    if ctx.in_repo {
        !ctx.user_config_exists && !ctx.repo_config_exists
    } else {
        !ctx.user_config_exists
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(user: bool, repo: bool, in_repo: bool) -> RootContext {
        RootContext {
            user_config_exists: user,
            repo_config_exists: repo,
            in_repo,
        }
    }

    // -- should_offer_init -----------------------------------------------

    #[test]
    fn init_offered_when_no_config_outside_repo() {
        assert!(should_offer_init(&ctx(false, false, false)));
    }

    #[test]
    fn init_hidden_when_user_config_exists_outside_repo() {
        assert!(!should_offer_init(&ctx(true, false, false)));
    }

    #[test]
    fn init_offered_inside_repo_with_no_configs() {
        assert!(should_offer_init(&ctx(false, false, true)));
    }

    #[test]
    fn init_hidden_inside_repo_when_user_config_present() {
        assert!(!should_offer_init(&ctx(true, false, true)));
    }

    #[test]
    fn init_hidden_inside_repo_when_repo_config_present() {
        assert!(!should_offer_init(&ctx(false, true, true)));
    }

    #[test]
    fn init_hidden_inside_repo_when_both_configs_present() {
        assert!(!should_offer_init(&ctx(true, true, true)));
    }

    // -- full_menu -------------------------------------------------------

    #[test]
    fn full_menu_starts_with_composition_in_spec_order() {
        let menu = full_menu(&ctx(true, true, true));
        assert_eq!(&menu[0..3], &["compose", "inline-compose", "sequence"]);
    }

    #[test]
    fn full_menu_wrappers_are_catalog_derived_in_display_order() {
        let menu = full_menu(&ctx(true, true, true));
        let expected = wrapper_tokens();
        assert_eq!(&menu[3..3 + expected.len()], expected.as_slice());
        // Regression guard: the three Phase-H providers must be present
        // (they were the ones missing from the old hand-maintained list).
        for slug in ["kilo", "pi", "antigravity"] {
            assert!(menu.iter().any(|c| c == slug), "{slug} missing from root menu");
        }
    }

    /// Index of the first post-wrapper group (`skills`), computed from the
    /// derived wrapper count so these assertions never drift as providers
    /// are added.
    fn post_wrappers() -> usize {
        3 + wrapper_tokens().len()
    }

    #[test]
    fn full_menu_has_shared_resources_in_spec_order() {
        let menu = full_menu(&ctx(true, true, true));
        let n = post_wrappers();
        assert_eq!(&menu[n..n + 4], &["skills", "commands", "agents", "mcp"]);
    }

    #[test]
    fn full_menu_has_hooks_and_actions() {
        let menu = full_menu(&ctx(true, true, true));
        let n = post_wrappers() + 4;
        assert_eq!(&menu[n..n + 2], &["hooks", "actions"]);
    }

    #[test]
    fn full_menu_has_administration_in_spec_order() {
        let menu = full_menu(&ctx(true, true, true));
        let n = post_wrappers() + 6;
        assert_eq!(
            &menu[n..n + 6],
            &[
                "sync",
                "uninstall",
                "providers",
                "logs",
                "completions",
                "config",
            ],
        );
    }

    #[test]
    fn full_menu_includes_init_when_no_config_exists() {
        let menu = full_menu(&ctx(false, false, false));
        assert_eq!(menu.last().map(String::as_str), Some("init"));
    }

    #[test]
    fn full_menu_excludes_init_when_user_config_present_outside_repo() {
        let menu = full_menu(&ctx(true, false, false));
        assert!(!menu.iter().any(|c| c == "init"));
    }

    #[test]
    fn full_menu_excludes_init_when_repo_config_present() {
        let menu = full_menu(&ctx(false, true, true));
        assert!(!menu.iter().any(|c| c == "init"));
    }

    #[test]
    fn full_menu_excludes_init_when_both_configs_present() {
        let menu = full_menu(&ctx(true, true, true));
        assert!(!menu.iter().any(|c| c == "init"));
    }

    // -- render ----------------------------------------------------------

    #[test]
    fn render_empty_returns_full_menu() {
        let got = render(&RootPartial::Empty, &ctx(true, true, true));
        assert_eq!(got.first().map(String::as_str), Some("compose"));
        assert!(got.contains(&"config".to_string()));
    }

    #[test]
    fn render_word_prefix_filters_by_prefix() {
        let got = render(
            &RootPartial::Word("com".to_string()),
            &ctx(true, true, true),
        );
        assert_eq!(got, vec!["compose", "commands", "completions"]);
    }

    #[test]
    fn render_word_prefix_returns_single_match() {
        let got = render(
            &RootPartial::Word("agents".to_string()),
            &ctx(true, true, true),
        );
        assert_eq!(got, vec!["agents"]);
    }

    #[test]
    fn render_flag_dash_offers_help() {
        let got = render(
            &RootPartial::FlagLike("-".to_string()),
            &ctx(true, true, true),
        );
        assert_eq!(got, vec!["--help"]);
    }

    #[test]
    fn render_flag_double_dash_offers_help() {
        let got = render(
            &RootPartial::FlagLike("--".to_string()),
            &ctx(true, true, true),
        );
        assert_eq!(got, vec!["--help"]);
    }

    #[test]
    fn render_flag_short_h_offers_help() {
        let got = render(
            &RootPartial::FlagLike("-h".to_string()),
            &ctx(true, true, true),
        );
        assert_eq!(got, vec!["--help"]);
    }

    #[test]
    fn render_flag_partial_help_offers_help() {
        for prefix in ["--h", "--he", "--hel", "--help"] {
            let got = render(
                &RootPartial::FlagLike(prefix.to_string()),
                &ctx(true, true, true),
            );
            assert_eq!(got, vec!["--help"], "prefix `{prefix}` should yield --help");
        }
    }

    #[test]
    fn render_flag_unrelated_returns_empty() {
        let got = render(
            &RootPartial::FlagLike("--version".to_string()),
            &ctx(true, true, true),
        );
        assert!(
            got.is_empty(),
            "unrelated flag must not surface --help: {got:?}"
        );
    }

    #[test]
    fn render_word_prefix_matches_init_when_offered() {
        let got = render(
            &RootPartial::Word("ini".to_string()),
            &ctx(false, false, false),
        );
        assert_eq!(got, vec!["init"]);
    }

    #[test]
    fn render_word_prefix_excludes_init_when_gated_off() {
        let got = render(
            &RootPartial::Word("ini".to_string()),
            &ctx(true, true, true),
        );
        assert!(got.is_empty());
    }
}
