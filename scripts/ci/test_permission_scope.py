#!/usr/bin/env python3
"""Decision tests for the permission-widening gate: `permission_scope.py`'s
`scope` over real temporary Git repositories, `decide` over checker reports
(unchanged, narrowing, widening, error), and `ci.yml`'s own inline
environment-protection filter over fake GitHub responses (protected, weak,
missing)."""
from __future__ import annotations

import copy
import io
import json
import re
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from typing import Any

import permission_scope
from permission_scope import CHECKER_SOURCE, decide, main, scope
from test_permission_environment import PROTECTED, weakened
from tool_guard import require_tools

ROOT = Path(__file__).resolve().parents[2]
GIT_ENFORCED_BY = "`_package-ci.yml`'s `test` job for repo-deps, which checks this repository out with Git"
JQ_ENFORCED_BY = "`_package-ci.yml`'s `test` job for repo-deps, which provisions jq for its companion suites"

INPUTS = {
    "policy-files": [
        {"path": ".darkmatter.yaml", "format": "darkmatter-config"},
        {"path": ".claudine/config.yaml", "format": "permissions-key"},
    ],
    "default-schemas": [{"path": "schemas/config.yaml", "format": "darkmatter-config"}],
    "checker-inputs": ["checker/", "ci.yml", CHECKER_SOURCE],
}


class Repository:
    """A temporary repository whose commits the tests choose."""

    def __init__(self) -> None:
        require_tools("git", enforced_by=GIT_ENFORCED_BY)
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)
        self.git("init", "-q")
        self.git("config", "user.email", "fixture@example.com")
        self.git("config", "user.name", "Fixture")
        self.git("config", "commit.gpgsign", "false")
        self.inputs = self.root / "inputs.json"
        self.inputs.write_text(json.dumps(INPUTS), encoding="utf-8")

    def git(self, *args: str) -> str:
        return subprocess.run(
            ["git", "-C", str(self.root), *args], check=True, capture_output=True, text=True, timeout=30,
        ).stdout.strip()

    def commit(self, files: dict[str, str | None], links: dict[str, str] | None = None) -> str:
        for path, text in files.items():
            target = self.root / path
            if text is None:
                target.unlink()
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
        self.git("add", "-A")
        # Links live only in the index, so no worktree symlink is needed (and
        # the fixture runs unchanged where symlinks need privileges).
        for path, link in (links or {}).items():
            blob = subprocess.run(
                ["git", "-C", str(self.root), "hash-object", "-w", "--stdin"],
                input=link, check=True, capture_output=True, text=True,
            ).stdout.strip()
            self.git("update-index", "--add", "--cacheinfo", f"120000,{blob},{path}")
        self.git("commit", "-q", "--allow-empty", "-m", "revision")
        return self.git("rev-parse", "HEAD")

    def scope(self, base: str, head: str) -> dict[str, str]:
        return scope(self.root, self.inputs, base, head)

    def close(self) -> None:
        self._dir.cleanup()


class ScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.repo = Repository()
        self.addCleanup(self.repo.close)
        self.base = self.repo.commit({CHECKER_SOURCE: "// trusted checker\n", "README.md": "one\n"})

    def test_an_unrelated_change_is_skipped(self) -> None:
        head = self.repo.commit({"README.md": "two\n"})
        self.assertEqual(self.repo.scope(self.base, head)["decision"], "skip")

    def test_the_same_commit_is_skipped(self) -> None:
        self.assertEqual(self.repo.scope(self.base, self.base)["decision"], "skip")

    def test_each_kind_of_listed_input_is_compared(self) -> None:
        for path in (".darkmatter.yaml", ".claudine/config.yaml", "schemas/config.yaml", "checker/nested/x.rs", "ci.yml"):
            with self.subTest(path):
                head = self.repo.commit({path: f"changed {path}\n"})
                decision = self.repo.scope(self.base, head)
                self.assertEqual(decision["decision"], "compare", decision)
                self.assertIn(path, decision["reason"])

    def test_a_deleted_policy_file_is_compared(self) -> None:
        base = self.repo.commit({".darkmatter.yaml": "permissions:\n  network: deny\n"})
        head = self.repo.commit({".darkmatter.yaml": None})
        self.assertEqual(self.repo.scope(base, head)["decision"], "compare")

    def test_a_reference_makes_every_change_compared(self) -> None:
        base = self.repo.commit({".darkmatter.yaml": "permissions: ./anywhere/policy.yaml\n"})
        head = self.repo.commit({"anywhere/policy.yaml": "permissions:\n  network: allow\n"})
        decision = self.repo.scope(base, head)
        self.assertEqual(decision["decision"], "compare", decision)
        self.assertIn("may read other files", decision["reason"])

    def test_a_block_or_flow_mapping_is_not_a_reference(self) -> None:
        for text in ("permissions:\n  network: deny\n", "permissions: {network: deny}\n", "permissions:   # block\n  network: deny\n"):
            with self.subTest(text):
                base = self.repo.commit({".darkmatter.yaml": text})
                head = self.repo.commit({"elsewhere.txt": text})
                self.assertEqual(self.repo.scope(base, head)["decision"], "skip")

    def test_a_committed_link_makes_every_change_compared(self) -> None:
        base = self.repo.commit({}, links={".darkmatter.yaml": "policies/real.yaml"})
        head = self.repo.commit({"policies/real.yaml": "permissions:\n  network: allow\n"})
        self.assertEqual(self.repo.scope(base, head)["decision"], "compare")

    def test_a_base_without_the_trusted_checker_is_potential_widening(self) -> None:
        repo = Repository()
        self.addCleanup(repo.close)
        base = repo.commit({"README.md": "before the checker\n"})
        head = repo.commit({"README.md": "after\n"})
        decision = repo.scope(base, head)
        self.assertEqual(decision, {"decision": "widening", "reason": "the base commit has no trusted checker (bootstrap)"})

    def test_no_base_commit_is_potential_widening(self) -> None:
        for base in ("", "0" * 40):
            self.assertEqual(self.repo.scope(base, self.base)["decision"], "widening")

    def test_an_abbreviated_or_symbolic_commit_is_an_error(self) -> None:
        for base in ("HEAD", self.base[:12]):
            with self.assertRaises(ValueError):
                self.repo.scope(base, self.base)

    def test_the_command_prints_github_outputs(self) -> None:
        head = self.repo.commit({".darkmatter.yaml": "permissions:\n  network: allow\n"})
        lines: list[str] = []
        status = main(["scope", "--repository", str(self.repo.root), "--inputs", str(self.repo.inputs),
                       "--base", self.base, "--head", head], out=lines.append)
        self.assertEqual(status, 0)
        self.assertEqual(lines[0], "decision=compare")


def report(reasons: list[dict[str, Any]], **extra: Any) -> str:
    document = {
        "base": "a" * 40, "head": "b" * 40, "changed_inputs": [".darkmatter.yaml"],
        "policies": [], "reasons": reasons, "potential_widening": bool(reasons),
    }
    document.update(extra)
    return json.dumps(document)


WIDENING = [{"input": ".darkmatter.yaml", "kind": "added-allow", "detail": "`shell-commands` gains `prefix rm`"}]


class DecideTests(unittest.TestCase):
    def test_an_unchanged_or_narrowing_report_needs_no_approval(self) -> None:
        outputs, lines = decide(report([]), 0)
        self.assertEqual(outputs, {"potential_widening": "false"})
        self.assertIn("no approval is needed", lines[-1])

    def test_a_widening_report_needs_an_approval_and_names_each_reason(self) -> None:
        outputs, lines = decide(report(WIDENING), 0)
        self.assertEqual(outputs, {"potential_widening": "true"})
        self.assertTrue(any("prefix rm" in line for line in lines))

    def test_a_failed_or_unreadable_check_is_an_error_never_no_change(self) -> None:
        for text, status in (
            (json.dumps({"error": "`.darkmatter.yaml` at the head commit: invalid"}), 1),
            (report([]), 1),
            ("", 1),
            ("not json", 0),
            ("[]", 0),
            (report([], potential_widening=True), 0),
            (report(WIDENING, potential_widening=False), 0),
            (report([], potential_widening="false"), 0),
        ):
            with self.subTest(text=text, status=status), self.assertRaises(ValueError):
                decide(text, status)

    def test_the_command_fails_the_job_on_an_error_and_writes_the_summary_otherwise(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            summary = Path(directory) / "summary.md"
            path.write_text(report(WIDENING), encoding="utf-8")
            lines: list[str] = []
            original = permission_scope.os.environ.get("GITHUB_STEP_SUMMARY")
            permission_scope.os.environ["GITHUB_STEP_SUMMARY"] = str(summary)
            try:
                with redirect_stderr(io.StringIO()):
                    self.assertEqual(main(["decide", "--report", str(path), "--status", "0"], out=lines.append), 0)
                    self.assertEqual(lines, ["potential_widening=true"])
                    missing = Path(directory) / "absent.json"
                    self.assertEqual(main(["decide", "--report", str(missing), "--status", "0"], out=lines.append), 1)
            finally:
                if original is None:
                    permission_scope.os.environ.pop("GITHUB_STEP_SUMMARY", None)
                else:
                    permission_scope.os.environ["GITHUB_STEP_SUMMARY"] = original
            self.assertIn("prefix rm", summary.read_text(encoding="utf-8"))


def protection_filter() -> str:
    """`ci.yml`'s inline `PROTECTION_FILTER`, folded as YAML folds it."""
    workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
    block = re.search(r"\n( +)PROTECTION_FILTER: >-\n((?:\1  .*\n)+)", workflow)
    assert block, "ci.yml declares PROTECTION_FILTER as a folded scalar"
    return " ".join(line.strip() for line in block.group(2).splitlines())


class WorkflowProtectionFilterTests(unittest.TestCase):
    """The gate that runs before `permission-widening` is requested."""

    def setUp(self) -> None:
        require_tools("jq", enforced_by=JQ_ENFORCED_BY)
        self.filter = protection_filter()

    def verdict(self, environment: Any) -> str:
        return subprocess.run(
            ["jq", self.filter], input=json.dumps(environment), check=True, capture_output=True, text=True, timeout=30,
        ).stdout.strip()

    def test_the_configured_environment_passes(self) -> None:
        self.assertEqual(self.verdict(PROTECTED), "true")

    def test_every_weak_or_missing_setting_fails_closed(self) -> None:
        cases = {
            "no environment body": None,
            "no rules": weakened(lambda e: e.update(protection_rules=[])),
            "another reviewer": weakened(lambda e: e["protection_rules"][0]["reviewers"].append(
                {"type": "User", "reviewer": {"id": 1}})),
            "self review prevented": weakened(lambda e: e["protection_rules"][0].update(prevent_self_review=True)),
            "admin bypass": weakened(lambda e: e.update(can_admins_bypass=True)),
            "wait timer": weakened(lambda e: e["protection_rules"].append({"type": "wait_timer", "wait_timer": 5})),
            "branch policy": weakened(lambda e: e.update(deployment_branch_policy={"protected_branches": True})),
        }
        for name, environment in cases.items():
            with self.subTest(name):
                self.assertEqual(self.verdict(environment), "false", name)

    def test_the_filter_and_the_setup_script_agree(self) -> None:
        import permission_environment
        variants = [copy.deepcopy(PROTECTED), weakened(lambda e: e.update(can_admins_bypass=True)),
                    weakened(lambda e: e["protection_rules"][0].update(reviewers=[]))]
        for environment in variants:
            self.assertEqual(self.verdict(environment) == "true", permission_environment.problems(environment) == [])
        self.assertIn(f'"User:{permission_environment.REVIEWERS[0]["id"]}"', self.filter)


if __name__ == "__main__":
    unittest.main(verbosity=2)
