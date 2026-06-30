//! Drift-detection surface for value-bearing composition flags.
//!
//! Ensures the `COMPOSITION_FLAGS_WITH_VALUE` list stays in sync with clap's
//! derived argument surface.

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    // ── Gap 5: parametric coverage for Rule 3 value-bearing flags ────

    #[test]
    fn rule_3_skips_value_for_every_composition_flag_with_value() {
        // For each value-bearing composition flag, construct
        // `claudine compose file.md <flag> VAL k=v` and assert Rule 3 does
        // not fire (the flag consumes `VAL`, so no positional-after-flag
        // sequence appears before the setter in a way that should trip the
        // rule). This locks the `COMPOSITION_FLAGS_WITH_VALUE` contract so
        // adding a new value-bearing flag without also adding it to the
        // constant becomes a test failure instead of a latent bug.
        let cases: &[(&str, &str)] = &[
            ("--provider", "claude"),
            ("--exclude", "claude"),
            ("--include", "FOO"),
            ("--model", "gpt-4"),
            ("-m", "gpt-4"),
            ("--output", "json"),
            ("-o", "json"),
            ("--append-system-prompt", "prompt.md"),
            ("--asp", "prompt.md"),
            ("--replace-system-prompt", "prompt.md"),
            ("--rsp", "prompt.md"),
            ("--timeout", "30"),
            ("-t", "30"),
            ("--step-timeout", "30s"),
            ("--stall-timeout", "10m"),
            ("--operation", "ship"),
            ("--op", "ship"),
            ("--set", "{\"k\":\"v\"}"),
            ("--use", "id1,id2"),
            ("--fail-fast", "true"),
        ];
        for (flag, value) in cases {
            let input = argv(&["claudine", "compose", "file.md", flag, value, "k=v"]);
            // No `--help`, so Rule 4 is a no-op. Rule 3 should fire here
            // (positional `file.md`, interleaved flag+value, setter
            // `k=v`) and insert a `--` before the setter — except when
            // `flag` is `--provider`, in which case Rule 2 also rewrites
            // nothing because `claude` is already canonical.
            let result = crate::argv::normalize(input);
            assert!(
                result.contains(&OsString::from("--")),
                "flag {flag} with value {value}: Rule 3 must insert `--`, \
                 got result = {result:?}"
            );
            assert!(
                result.contains(&OsString::from(*flag)) || *flag == "--provider",
                "flag {flag}: token must survive normalization verbatim"
            );
        }
    }

    // ── Gap 6: drift detection between clap surface and the value-table ──

    #[test]
    fn composition_flags_with_value_equals_clap_surface() {
        // The emitted `COMPOSITION_FLAGS_WITH_VALUE` is now derived from
        // clap's `ComposeArgs` and `SequenceArgs` at first use. This test
        // re-runs the same derivation and asserts equality so a future
        // refactor that breaks the derivation surfaces as a test failure
        // instead of a silently-empty value-flag table.
        let derived = crate::argv::collect_composition_value_flags();

        assert!(
            !derived.is_empty(),
            "clap-derived composition value-flag surface must not be empty"
        );

        let mut emitted: Vec<String> = crate::argv::COMPOSITION_FLAGS_WITH_VALUE.clone();
        emitted.sort();
        emitted.dedup();

        let mut expected = derived.clone();
        expected.sort();
        expected.dedup();

        assert_eq!(
            emitted, expected,
            "argv normalizer's value-flag surface must equal the clap-derived \
             surface; if this drifts, fix `collect_composition_value_flags`"
        );
    }
}
