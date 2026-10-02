---
fix: 2026-07-13-cli-switches
deferred_perf_measurement: false
---

# Implementation Log: CLI Switches

## Implementation of Review Findings #1

> **started at:** 2026-10-02T01:48:44-07:00

- this implementation is attempting to implement _all_ of the review findings found in 'claudine/fixes/2026-07-13-cli-switches/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Embedded credentials escape argument redaction' at 01:49:16
        - discovered the shared recognizer (`claudine::secrets::mask_secrets`) misses `-csk-…` (no word boundary before `sk-`) and short `sk-` values; added `find_argument_secret_spans`/`mask_argument_token` to `claudine::secrets` so the argument policy and ownership's mismatch value display share one per-token recognizer
        - rewrote `redact_sensitive_args`/`sensitive_arg_values` over one per-token record (shown text + original secret spans), so `tail_redactor` learns the exact embedded value instead of a suffix guess
        - discovered `validate_argv_flags_before_separator` logged the raw argv at WARN; it now logs the redacted argv
        - discovered masked values rendered as empty emphasis (`**`): the direct-wrapper dry-run `Command:` line, its environment list (`AGENT_PARAMS`), and the unhandled-failure headline passed plain text through Prose markup; all three now escape it
        - replaced the non-asserting `review_probe_secret_surfaces` probe with asserting L1 tests in `provider_tail_launch.rs`
        - deferred: the live relay of a structured provider's own stderr (e.g. Codex) is a passthrough of provider output, not an argument display; masking it is outside this class and the interactive inherited path cannot be masked at all
        - updated `docs/topics/argv-normalization.md` and `docs/topics/secret-recognition.md`
        - `just test` (8230 passed) and `just lint` pass
- defect class: argument redaction decided secrecy only at whole-token and flag boundaries, so a credential embedded inside an otherwise ordinary token was displayed unmasked
- class sweep: credential embedded inside an otherwise-retained argument token; sites checked: redact_sensitive_args, sensitive_arg_values/tail_redactor, composition dry_run provider_args, direct-wrapper log_dry_run Command line and env list, log_wrapper_env_details, AGENT_PARAMS (env/mod.rs), direct wrap debug trace (wrap/mod.rs), composition debug trace (pipeline.rs), validate_argv_flags_before_separator warn, ownership TailMismatch display_value, ProviderTail/ProviderTailNotices/ArgumentsAfterFile/OwnedArguments Debug impls, notice switch_names_for_display, resume tail trace, unhandled-failure headline, structured live stderr relay; fixed here: redact_sensitive_args, sensitive_arg_values (tail_redactor), validate_argv_flags_before_separator, ownership display_value, log_dry_run Command/env markup, log_wrapper_env_details markup, report_unhandled_failure markup (dry_run, AGENT_PARAMS, both debug traces fixed through the shared policy); clean: ProviderTail/ProviderTailNotices/ArgumentsAfterFile/OwnedArguments Debug (counts only), notice switch names, resume trace (length only); deferred: structured live stderr relay
- work completed for 'Embedded credentials escape argument redaction' at 02:00:53
- starting the work on 'Flag completion bypasses ownership errors' at 02:01:34
        - discovered the ownership gate in `run_with_context` listed the gated targets with `matches!`, so the flag target (and any future target) silently fell through ungated; replaced it with an exhaustive `match` that states a verdict for every `CompletionTarget` variant
        - added `ownership::committed_arguments_are_owned`, which runs the same partition and type-aware ownership over the words before the cursor, leaving the partial flag out so a valid line still completes `--mod` to `--model` and `--cl` to `--claude`
        - replaced the non-asserting `review_probe_completion_sweep` probe with a table-driven L1 test (3 commands x 3 preceding lines x 6 cursor shapes) plus a flag-control test; confirmed the matrix fails without the new gate
        - updated `docs/topics/completions/shell-completions.md` (table, rules, both diagrams, module table) and `docs/topics/argv-normalization.md`
        - `just test` (8231 passed) and `just lint` pass
- defect class: a completion target that offers suggestions after the composition file was exempt from the ownership verdict, so an error earlier on the line did not suppress its candidates
- class sweep: completion targets offering candidates after the composition file without the ownership verdict; sites checked: Root, CompositionPositional, CompositionProviderFlag, SetterValue, SetterName, Declined, Other (clap fallback), run_composition_provider_flag's clap merge, direct-wrapper completion (classified Other), compose/inline-compose/sequence (one shared engine); fixed here: CompositionProviderFlag (including its clap merge); clean: SetterValue, SetterName, Other (already gated by the full-line check), Root and CompositionPositional (precede the file), Declined (offers nothing), direct wrappers (no ownership; tail always passes)
- work completed for 'Flag completion bypasses ownership errors' at 02:05:37
- starting the work on 'Unknown switch types are described as recognized' at 02:06:08
