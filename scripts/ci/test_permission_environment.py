#!/usr/bin/env python3
"""Mocked GitHub tests for `permission_environment.py`: the dry run, `--apply`
and its read-back, `verify`, and every weak or missing setting. No request
reaches GitHub; `FakeGitHub` answers each one."""
from __future__ import annotations

import copy
import unittest
from typing import Any

import permission_environment as env
from permission_environment import ApiError, PAYLOAD, main, problems

ENDPOINT = "repos/yankeeinlondon/rusty-biscuit/environments/permission-widening"

# What GitHub returns for an environment configured with `PAYLOAD`.
PROTECTED: dict[str, Any] = {
    "name": "permission-widening",
    "can_admins_bypass": False,
    "deployment_branch_policy": None,
    "protection_rules": [
        {
            "type": "required_reviewers",
            "prevent_self_review": False,
            "reviewers": [{"type": "User", "reviewer": {"id": 980757, "login": "yankeeinlondon"}}],
        }
    ],
}


class FakeGitHub:
    """Answers GET and PUT for one environment, recording every request."""

    def __init__(self, environment: dict[str, Any] | None, put_result: dict[str, Any] | None = None,
                 put_error: ApiError | None = None) -> None:
        self.environment = copy.deepcopy(environment)
        self.put_result = put_result
        self.put_error = put_error
        self.requests: list[tuple[str, str, Any]] = []

    def __call__(self, method: str, endpoint: str, payload: dict[str, Any] | None) -> Any:
        self.requests.append((method, endpoint, copy.deepcopy(payload)))
        if endpoint != ENDPOINT:
            raise ApiError(f"unexpected endpoint {endpoint}", 404)
        if method == "GET":
            if self.environment is None:
                raise ApiError("gh: Not Found (HTTP 404)", 404)
            return copy.deepcopy(self.environment)
        if method == "PUT":
            if self.put_error:
                raise self.put_error
            self.environment = copy.deepcopy(self.put_result if self.put_result is not None else PROTECTED)
            return copy.deepcopy(self.environment)
        raise AssertionError(f"this script never sends {method}")


def run(argv: list[str], github: FakeGitHub) -> tuple[int, str]:
    lines: list[str] = []
    status = main(argv, api=github, out=lines.append)
    return status, "\n".join(lines)


def weakened(change) -> dict[str, Any]:
    environment = copy.deepcopy(PROTECTED)
    change(environment)
    return environment


class PayloadTests(unittest.TestCase):
    def test_the_payload_is_the_exact_protection_the_gate_requires(self) -> None:
        self.assertEqual(PAYLOAD, {
            "wait_timer": 0,
            "prevent_self_review": False,
            "reviewers": [{"type": "User", "id": 980757}],
            "can_admins_bypass": False,
            "deployment_branch_policy": None,
        })
        self.assertIn(f"gh api --method PUT {ENDPOINT}", env.apply_command())
        self.assertIn('"prevent_self_review": false', env.apply_command())


class ProblemTests(unittest.TestCase):
    def test_the_configured_environment_has_no_problem(self) -> None:
        self.assertEqual(problems(PROTECTED), [])

    def test_every_weak_or_missing_setting_is_a_problem(self) -> None:
        cases = {
            "missing": None,
            "no rules": weakened(lambda e: e.update(protection_rules=[])),
            "another reviewer": weakened(lambda e: e["protection_rules"][0]["reviewers"].append(
                {"type": "User", "reviewer": {"id": 1, "login": "someone"}})),
            "a team instead": weakened(lambda e: e["protection_rules"][0].update(
                reviewers=[{"type": "Team", "reviewer": {"id": 980757}}])),
            "no reviewers": weakened(lambda e: e["protection_rules"][0].update(reviewers=[])),
            "self review prevented": weakened(lambda e: e["protection_rules"][0].update(prevent_self_review=True)),
            "self review unset": weakened(lambda e: e["protection_rules"][0].pop("prevent_self_review")),
            "admin bypass": weakened(lambda e: e.update(can_admins_bypass=True)),
            "admin bypass unset": weakened(lambda e: e.pop("can_admins_bypass")),
            "wait timer": weakened(lambda e: e["protection_rules"].append({"type": "wait_timer", "wait_timer": 5})),
            "branch policy": weakened(lambda e: e.update(
                deployment_branch_policy={"protected_branches": True, "custom_branch_policies": False})),
        }
        for name, environment in cases.items():
            with self.subTest(name):
                self.assertNotEqual(problems(environment), [], name)

    def test_a_zero_wait_timer_rule_is_not_a_problem(self) -> None:
        environment = weakened(lambda e: e["protection_rules"].append({"type": "wait_timer", "wait_timer": 0}))
        self.assertEqual(problems(environment), [])


class PlanTests(unittest.TestCase):
    def test_a_dry_run_reads_only_and_prints_the_request(self) -> None:
        github = FakeGitHub(None)
        status, output = run([], github)
        self.assertEqual(status, 0)
        self.assertEqual([method for method, _, _ in github.requests], ["GET"])
        self.assertIn("does not exist", output)
        self.assertIn("Dry run: nothing was changed", output)
        self.assertIn(f"gh api --method PUT {ENDPOINT}", output)

    def test_apply_configures_then_verifies_by_reading_back(self) -> None:
        github = FakeGitHub(None)
        status, output = run(["--apply"], github)
        self.assertEqual(status, 0, output)
        self.assertEqual([(m, p) for m, _, p in github.requests], [("GET", None), ("PUT", PAYLOAD), ("GET", None)])
        self.assertIn("now has the required protection", output)

    def test_apply_is_idempotent(self) -> None:
        github = FakeGitHub(PROTECTED)
        for argv in ([], ["--apply"]):
            status, output = run(argv, github)
            self.assertEqual(status, 0)
            self.assertIn("nothing to change", output)
        self.assertEqual([method for method, _, _ in github.requests], ["GET", "GET"], "no PUT once configured")

    def test_apply_repairs_a_weaker_environment(self) -> None:
        github = FakeGitHub(weakened(lambda e: e.update(can_admins_bypass=True)))
        status, _ = run(["--apply"], github)
        self.assertEqual(status, 0)
        self.assertEqual(github.environment, PROTECTED)

    def test_an_accepted_request_that_leaves_a_weaker_environment_fails(self) -> None:
        github = FakeGitHub(None, put_result=weakened(lambda e: e.update(can_admins_bypass=True)))
        status, output = run(["--apply"], github)
        self.assertEqual(status, 1)
        self.assertIn("still differs", output)

    def test_a_rejected_request_fails_without_retrying(self) -> None:
        github = FakeGitHub(None, put_error=ApiError("gh: Must have admin rights (HTTP 403)", 403))
        status, output = run(["--apply"], github)
        self.assertEqual(status, 1)
        self.assertIn("HTTP 403", output)
        self.assertEqual([method for method, _, _ in github.requests], ["GET", "PUT"])

    def test_a_read_error_other_than_absence_fails(self) -> None:
        def broken(method: str, endpoint: str, payload: Any) -> Any:
            raise ApiError("gh: Bad credentials (HTTP 401)", 401)
        lines: list[str] = []
        self.assertEqual(main([], api=broken, out=lines.append), 1)
        self.assertIn("HTTP 401", "\n".join(lines))


class VerifyTests(unittest.TestCase):
    def test_verify_passes_only_for_the_required_protection_and_never_writes(self) -> None:
        for environment, expected in ((PROTECTED, 0), (None, 1), (weakened(lambda e: e.update(can_admins_bypass=True)), 1)):
            github = FakeGitHub(environment)
            status, output = run(["verify"], github)
            self.assertEqual(status, expected, output)
            self.assertTrue(all(method == "GET" for method, _, _ in github.requests))

    def test_apply_is_refused_with_verify(self) -> None:
        with self.assertRaises(SystemExit):
            main(["verify", "--apply"], api=FakeGitHub(None), out=lambda _: None)


class StatusParsingTests(unittest.TestCase):
    def test_the_http_status_is_read_from_gh_stderr(self) -> None:
        completed = type("Completed", (), {"returncode": 1, "stderr": "gh: Not Found (HTTP 404)", "stdout": ""})()
        original = env.subprocess.run
        env.subprocess.run = lambda *args, **kwargs: completed  # type: ignore[assignment]
        try:
            with self.assertRaises(ApiError) as raised:
                env.gh_api("GET", ENDPOINT)
            self.assertEqual(raised.exception.status, 404)
            self.assertIsNone(env.read_environment(env.gh_api, ENDPOINT))
        finally:
            env.subprocess.run = original  # type: ignore[assignment]


if __name__ == "__main__":
    unittest.main(verbosity=2)
