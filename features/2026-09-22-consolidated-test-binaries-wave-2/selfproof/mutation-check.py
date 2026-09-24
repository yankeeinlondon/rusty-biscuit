#!/usr/bin/env python3
"""Red-then-green check for the guards Phase 2 added to the consolidation toolkit.

Applies one targeted mutation per guard to a scratch copy of
`scripts/ci/consolidation.py` and runs the guard's test against the original
(must pass) and the mutant (must fail). Run from the repository root. Exits 1
if any guard does not go red on its mutation.
"""
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SRC = Path("scripts/ci")
MUTATIONS = [
    ("unlisted package captured with a default set",
     "else PACKAGE_FEATURE_SETS.get(package)\n", "else PACKAGE_FEATURE_SETS.get(package, ((),))\n",
     "PackageTableTests.test_an_unlisted_package_is_refused_before_any_subprocess"),
    ("include and #[path] literals not repaired",
     "rewritten = PATH_ATTR_LITERAL.sub(repair_literal, INCLUDE_LITERAL.sub(repair_literal, body)) + ending",
     "rewritten = line",
     "MoveTests.test_files_move_and_relative_constructs_keep_their_targets"),
    ("alias ignored: root declares the old target name",
     '        lines.append(f"mod {name};")', "        lines.append(f\"mod {modules[name]['old_target']};\")",
     "MoveTests.test_an_alias_from_the_manifest_becomes_the_module_name"),
    ("common declared in every root",
     'declare_common = uses_common[target["name"]] and common.is_file()', "declare_common = common.is_file()",
     "MoveTests.test_roots_declare_common_only_where_used_and_modules_in_rustfmt_order"),
    ("modules in manifest order, not rustfmt order",
     '    for name in sorted(target["modules"]):', '    for name in target["modules"]:',
     "MoveTests.test_roots_declare_common_only_where_used_and_modules_in_rustfmt_order"),
    ("keep_inner_cfg ignored",
     'if file == old and not row.get("keep_inner_cfg"):', "if file == old:",
     "MoveTests.test_the_inner_cfg_is_kept_only_where_the_row_says_so"),
    ("proptest seed moved beside its module (the wave-1 defect)",
     "current, destination = mapped(seed), proptest_regression_path(mapped(source))",
     'current, destination = mapped(seed), mapped(source).with_suffix(".proptest-regressions")',
     "MoveTests.test_proptest_seeds_move_to_where_proptest_reads_them"),
    ("move mutates before refusing",
     "    if problems:\n        raise ToolError(\"move refused", "    if False:\n        raise ToolError(\"move refused",
     "MoveTests.test_refusals_leave_the_tree_untouched"),
    ("unreplayed seed file accepted",
     "        if _absolute(repo / path) not in live:", "        if False:",
     "CheckProptestTests.test_a_seed_left_beside_its_module_is_never_replayed"),
    ("extra or feature-mismatched Cargo target ignored",
     'failures += [f"{name}: undeclared Cargo test target {extra}" for extra in sorted(actual - declared)]', "pass",
     "CheckMetadataTests.test_each_mismatch_is_named"),
    ("autotests = false not required",
     'if cargo_toml.get("package", {}).get("autotests") is not False:', "if False:",
     "CheckMetadataTests.test_each_mismatch_is_named"),
    ("body change accepted",
     "            if others:\n                failures.append", "            if False:\n                failures.append",
     "BodyDiffTests.test_a_body_change_fails_with_the_line"),
    ("path-literal repair read as a body change",
     'line = INCLUDE_LITERAL.sub(r"\\1<path>\\3", line.replace("super::", "crate::"))', 'line = line.replace("super::", "crate::")',
     "BodyDiffTests.test_structural_edits_pass_and_are_counted"),
    ("shared common tests projected under each module",
     '            if old.get("declares_mod_common"):', "            if False:",
     "SharedAndRuledPlanTests.test_shared_common_tests_are_recorded_once_and_force_no_alias"),
    ("RULED_TARGETS ignored",
     '        if target["name"] in ruled:', "        if False:",
     "SharedAndRuledPlanTests.test_a_ruled_l1_target_joins_the_named_target"),
    ("joining a target with more features unchecked",
     "        elif extra:", "        elif False:",
     "SharedAndRuledPlanTests.test_joining_a_target_with_more_features_needs_no_test_without_them"),
    ("disagreeing shared copies folded silently",
     'if test in target["tests"] and target["tests"][test] != record:', "if False:",
     "CompareTests.test_shared_copies_that_disagree_fail"),
]


def main() -> int:
    bad = 0
    for label, old, new, test in MUTATIONS:
        with tempfile.TemporaryDirectory() as temporary:
            scratch = Path(temporary) / "a" / "b"
            scratch.mkdir(parents=True)
            text = (SRC / "consolidation.py").read_text()
            assert text.count(old) == 1, (label, text.count(old))
            (scratch / "consolidation.py").write_text(text.replace(old, new))
            shutil.copy(SRC / "test_consolidation.py", scratch)
            green, red = (subprocess.run([sys.executable, "-m", "unittest", f"test_consolidation.{test}"], cwd=cwd,
                                         capture_output=True, text=True).returncode for cwd in (SRC, scratch))
            ok = green == 0 and red != 0
            bad += not ok
            print(f"{'OK ' if ok else 'BAD'} {label}: original exit {green}, mutant exit {red} ({test})")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
