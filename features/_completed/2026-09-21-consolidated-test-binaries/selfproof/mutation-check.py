#!/usr/bin/env python3
"""Red-then-green check for the consolidation toolkit oracles (Phase 2 self-proof).

Applies one targeted mutation per guard to a scratch copy of
`scripts/ci/consolidation.py` and runs the guard's test against the original
(must pass) and the mutant (must fail). Run from the repository root.
"""
import subprocess, shutil, tempfile, sys
from pathlib import Path
SRC = Path("scripts/ci")
MUTATIONS = [
    ("count-only compare", "lost = sorted(b_sets[name] - a_sets[name])\n                    gained = sorted(a_sets[name] - b_sets[name])",
     "lost = [] if len(b_sets[name]) == len(a_sets[name]) else ['count']\n                    gained = []",
     "CompareTests.test_identity_loss_masked_by_identity_gain_fails_with_the_identities"),
    ("missing snapshot row ignored", 'failures.append(f"{key[0]}: no mapping row (the rule places it at {key[1]})")', "pass",
     "CheckSnapshotsTests.test_a_missing_mapping_row_fails"),
    ("cfg not required on declaration", "if attribute not in declared:\n                    failures.append", "if False:\n                    failures.append",
     "CheckAttributesTests.test_a_cfg_missing_from_the_declaration_fails"),
    ("alias hazards ignored", "            if hazards:\n                marker = name_marker", "            if False:\n                marker = name_marker",
     "PlanTests.test_a_marker_name_that_would_move_tests_gets_a_neutral_alias"),
    ("unmapped module passes through", "        if old is None or not separator:\n            return None", "        if old is None or not separator:\n            return (binary_id, test)",
     "CompareTests.test_an_unmapped_module_and_a_duplicate_normalization_fail"),
    ("evaluator agreement skipped", "    failures += check_evaluator_agreement(package, captures, parsed, filters)", "    pass",
     "PlanTests.test_the_evaluator_must_agree_with_every_nextest_verdict"),
    ("platform-absent always not-evaluated", "        if len(hosts) < 2:", "        if True:",
     "CompareTests.test_platform_absent_needs_other_hosts"),
    ("skipped binary read as changed build", '            if suite.get("status") == "skipped":', '            if False:',
     "CaptureDocumentTests.test_a_binary_skipped_by_a_package_filter_takes_listed_mismatch_verdicts"),
]
for label, old, new, test in MUTATIONS:
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp) / "a" / "b"; d.mkdir(parents=True)
        text = (SRC / "consolidation.py").read_text()
        assert text.count(old) == 1, (label, text.count(old))
        (d / "consolidation.py").write_text(text.replace(old, new))
        shutil.copy(SRC / "test_consolidation.py", d)
        green = subprocess.run([sys.executable, "-m", "unittest", f"test_consolidation.{test}"], cwd=SRC, capture_output=True, text=True).returncode
        red = subprocess.run([sys.executable, "-m", "unittest", f"test_consolidation.{test}"], cwd=d, capture_output=True, text=True).returncode
        print(f"{'OK ' if green == 0 and red != 0 else 'BAD'} {label}: original exit {green}, mutant exit {red} ({test})")
