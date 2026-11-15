#!/usr/bin/env python3
"""Decision tests for the permission-change report: `permission_scope.py`'s
`scope` over real temporary Git repositories, and `decide` over checker
reports (unchanged, narrowing, widening, error) with the pull-request comment
it writes."""
from __future__ import annotations

import base64
import io
import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from typing import Any

import permission_scope
from permission_scope import CHECKER_SOURCE, DATA_ENVIRONMENT, code, decide, literal, main, scope
from tool_guard import require_tools

ROOT = Path(__file__).resolve().parents[2]
GIT_ENFORCED_BY = "`_package-ci.yml`'s `test` job for repo-deps, which checks this repository out with Git"

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

    def test_a_base_without_the_trusted_checker_is_uncompared(self) -> None:
        repo = Repository()
        self.addCleanup(repo.close)
        base = repo.commit({"README.md": "before the checker\n"})
        head = repo.commit({"README.md": "after\n"})
        decision = repo.scope(base, head)
        self.assertEqual(decision, {"decision": "uncompared", "reason": "the base commit has no trusted checker (bootstrap)"})

    def test_no_base_commit_is_uncompared(self) -> None:
        for base in ("", "0" * 40):
            self.assertEqual(self.repo.scope(base, self.base)["decision"], "uncompared")

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


BEFORE = {"block_file": ".darkmatter.yaml", "file-read": ["!&secrets/**", "&docs/**"],
          "shell-commands": ["exact pwd", "prefix ls"], "network": "deny"}


def report(reasons: list[dict[str, Any]], after: dict[str, Any] | None = None, **extra: Any) -> str:
    """A checker report for one `.darkmatter.yaml` policy, `BEFORE` → `after`."""
    policy = {"input": ".darkmatter.yaml", "scope": "repository", "before": BEFORE, "after": after or BEFORE}
    document = {
        "base": "a" * 40, "head": "b" * 40, "changed_inputs": [".darkmatter.yaml"],
        "policies": [policy], "reasons": reasons, "potential_widening": bool(reasons),
    }
    document.update(extra)
    return json.dumps(document)


def text(token: str) -> str:
    """The text a literal token carries."""
    assert token.startswith("{{!data:v1:") and token.endswith("}}"), token
    payload = token[len("{{!data:v1:"):-2]
    return base64.urlsafe_b64decode(payload + "=" * (-len(payload) % 4)).decode("utf-8")


WIDENING = [{"input": ".darkmatter.yaml", "kind": "added-allow", "detail": "`shell-commands` gains `prefix rm`"}]
WIDER = dict(BEFORE, **{"shell-commands": ["exact pwd", "prefix ls", "prefix git status", "template git add {{ string }}"],
                        "file-read": ["&docs/**"], "network": "allow"})


class LiteralTests(unittest.TestCase):
    def test_a_value_round_trips_through_its_literal_token(self) -> None:
        for value in ("prefix ls", "git add {{ string }}", "$(touch x)", "", "東京"):
            self.assertEqual(text(literal(value)), value)

    def test_an_inline_code_span_survives_any_backticks(self) -> None:
        self.assertEqual(text(code("ls")), "`ls`")
        self.assertEqual(text(code("echo `x`")), "`` echo `x` ``")
        self.assertEqual(text(code("a`b")), "``a`b``")
        self.assertEqual(text(code("`edge")), "`` `edge ``")


class DecideTests(unittest.TestCase):
    def test_an_unchanged_policy_reports_no_change(self) -> None:
        self.assertEqual(decide(report([]), 0), ({"permission_change": "none"}, None))

    def test_a_change_to_the_checks_own_files_is_not_a_permission_change(self) -> None:
        checker = [{"input": ".github/workflows/ci.yml", "kind": "checker-input-changed", "detail": "an input"}]
        self.assertEqual(decide(report(checker), 0), ({"permission_change": "none"}, None))

    def test_a_widening_lists_each_entry_by_kind(self) -> None:
        outputs, data = decide(report(WIDENING, WIDER), 0)
        self.assertEqual(outputs, {"permission_change": "widens"})
        assert data is not None
        self.assertEqual(data["verdict"], "widens")
        self.assertEqual([text(source) for source in data["sources"]], ["`.darkmatter.yaml`"])
        [policy] = data["policies"]
        self.assertEqual(text(policy["title"]), "`.darkmatter.yaml`")
        shell = policy["shell"]
        self.assertEqual(shell["before_count"], 2)
        self.assertEqual(
            [(rule["form"], text(rule["code"])) for rule in shell["added"]],
            [("prefix", "`git status`"), ("template", "`git add {{ string }}`")],
        )
        self.assertEqual(shell["removed"], [])
        reads = policy["file_read"]
        self.assertEqual([text(entry) for entry in reads["unexcluded"]], ["`&secrets/**`"])
        self.assertEqual(policy["network"], {"changed": True, "before": "deny", "after": "allow"})

    def test_a_narrowing_is_reported_as_one(self) -> None:
        narrower = dict(BEFORE, **{"shell-commands": ["exact pwd", "!prefix git push"]})
        outputs, data = decide(report([], narrower), 0)
        self.assertEqual(outputs, {"permission_change": "narrows"})
        assert data is not None
        shell = data["policies"][0]["shell"]
        self.assertEqual([text(rule["code"]) for rule in shell["removed"]], ["`ls`"])
        self.assertEqual([text(rule["code"]) for rule in shell["excluded"]], ["`git push`"])
        self.assertFalse(data["policies"][0]["file_read"]["changed"])

    def test_a_default_the_trusted_code_cannot_read_is_unknown(self) -> None:
        unreadable = [{"input": "darkmatter/schemas/darkmatter-config.yaml", "kind": "defaults-not-understood",
                       "detail": "the default cannot be read"}]
        outputs, data = decide(report(unreadable), 0)
        self.assertEqual(outputs, {"permission_change": "unknown"})
        assert data is not None
        self.assertEqual(
            [(text(entry["input"]), text(entry["detail"])) for entry in data["unreadable"]],
            [("`darkmatter/schemas/darkmatter-config.yaml`", "the default cannot be read")],
        )

    def test_the_repository_defaults_are_recognized_as_defaults(self) -> None:
        defaults = {"block_file": None, "file-read": ["&**/*.md"], "shell-commands": [], "network": "deny"}
        before = dict(BEFORE, **{"file-read": ["&**/*.md"]})
        policies = [
            {"input": ".darkmatter.yaml", "scope": "repository", "before": before,
             "after": dict(before, **{"file-read": ["&**/*.md", "&notes/**"]})},
            {"input": "darkmatter/schemas/darkmatter-config.yaml", "scope": "repository-entry-defaults",
             "before": defaults, "after": defaults},
        ]
        _, data = decide(report(WIDENING, policies=policies), 0)
        assert data is not None
        self.assertTrue(data["policies"][0]["file_read"]["before_default"])

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
            (report([], policies=None), 0),
        ):
            with self.subTest(text=text, status=status), self.assertRaises(ValueError):
                decide(text, status)

    def test_the_command_writes_the_data_only_for_a_change_and_fails_on_an_error(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            data = Path(directory) / "data.json"
            lines: list[str] = []
            original = permission_scope.os.environ.get(DATA_ENVIRONMENT)
            permission_scope.os.environ[DATA_ENVIRONMENT] = str(data)
            try:
                with redirect_stderr(io.StringIO()):
                    path.write_text(report([]), encoding="utf-8")
                    self.assertEqual(main(["decide", "--report", str(path), "--status", "0"], out=lines.append), 0)
                    self.assertFalse(data.exists(), "no change writes no data")
                    path.write_text(report(WIDENING, WIDER), encoding="utf-8")
                    self.assertEqual(main(["decide", "--report", str(path), "--status", "0"], out=lines.append), 0)
                    self.assertEqual(json.loads(data.read_text(encoding="utf-8"))["verdict"], "widens")
                    self.assertEqual(lines, ["permission_change=none", "permission_change=widens"])
                    missing = Path(directory) / "absent.json"
                    self.assertEqual(main(["decide", "--report", str(missing), "--status", "0"], out=lines.append), 1)
            finally:
                if original is None:
                    permission_scope.os.environ.pop(DATA_ENVIRONMENT, None)
                else:
                    permission_scope.os.environ[DATA_ENVIRONMENT] = original


if __name__ == "__main__":
    unittest.main(verbosity=2)
