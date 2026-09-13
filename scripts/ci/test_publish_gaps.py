#!/usr/bin/env python3
"""Contract tests for the accepted-gap check-run publisher (spec section 6, OQ4).

The publisher is presentation only, so the fixtures are about what a reader
sees and what must never be shown: the payload every accepted cell produces,
the cells that are refused because a `neutral` check would misrepresent a
blocking cell, and the exact `gh api` call the hosted job makes.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish_gaps  # noqa: E402


TODAY = date(2026, 9, 12)
MODULE = Path(__file__).resolve().parent / "publish_gaps.py"
HEAD = "0123456789abcdef0123456789abcdef01234567"
REPO = "yankeeinlondon/rusty-biscuit"
SERVER = "https://github.com"

#: A cut-down `environments.json` with the entry the fixtures anchor to.
#: `ubuntu-latest` declares a `tmux` key too (line 7, a boolean), which is
#: what the block-bounded search must not mistake for the Windows entry on
#: line 13.
POLICY_SOURCE = """{
  "schema_version": 1,
  "environments": [
    {
      "name": "ubuntu-latest",
      "capabilities": {
        "tmux": true
      }
    },
    {
      "name": "windows-latest",
      "capabilities": {
        "tmux": {
          "available": false,
          "reason": "no tmux port",
          "owner": "@yankeeinlondon",
          "expiry": "2027-01-31",
          "closes": "features/_unscheduled/windows-l2-ci-leg"
        }
      }
    }
  ]
}
"""
TMUX_LINE = 13
UBUNTU_TMUX_LINE = 7


def gap(**overrides: object) -> dict:
    record = {
        "capability": "tmux",
        "governed": True,
        "policy": ".github/ci/environments.json",
        "owner": "@yankeeinlondon",
        "reason": "no L2 terminal backend is provisionable on Windows",
        "expiry": "2027-01-31",
        "closes": "features/_unscheduled/windows-l2-ci-leg",
    }
    record.update(overrides)
    return record


def cell(package: str, area: str, environment: str, gate: str = "L2", **overrides: object) -> dict:
    record = {
        "package": package,
        "area": area,
        "environment": environment,
        "gate": gate,
        "execution": "omit",
        "origin": "none",
        "state": "accepted-gap",
        "reusable": False,
        "target_kinds": [],
        "compile_coverage_from": "",
        "selection_reason": f"{package} declares the {gate} tier",
        "gap": gap(),
    }
    record.update(overrides)
    return record


def plan(*cells: dict) -> dict:
    return {"schema_version": 2, "cells": list(cells)}


#: Two accepted gaps in `pkg`, one in `other`, and a pending cell in `pkg`
#: that must never be published.
FIXTURE = plan(
    cell("alpha-cli", "pkg", "windows-latest"),
    cell("alpha-cli", "pkg", "ubuntu-latest", gate="L1", state="pending", execution="execute", origin="ci", gap=None),
    cell("alpha-gen", "pkg", "wsl2-ubuntu", gap=gap(closes=None, capability="tmux")),
    cell("beta", "other", "windows-latest"),
)


def build(plan_document: dict, area: str = "pkg", **overrides: object) -> list[dict]:
    options = {
        "head_sha": HEAD,
        "repo": REPO,
        "server": SERVER,
        "today": TODAY,
        "policy_source": POLICY_SOURCE,
    }
    options.update(overrides)
    return publish_gaps.payloads(plan_document, area, **options)


class PayloadTests(unittest.TestCase):
    def test_only_the_areas_accepted_gaps_are_selected(self) -> None:
        names = [run["name"] for run in build(FIXTURE)]
        self.assertEqual(
            [
                "accepted gap: pkg / alpha-cli / L2 (windows-latest)",
                "accepted gap: pkg / alpha-gen / L2 (wsl2-ubuntu)",
            ],
            names,
        )
        self.assertEqual(
            ["accepted gap: other / beta / L2 (windows-latest)"],
            [run["name"] for run in build(FIXTURE, area="other")],
        )
        self.assertEqual([], build(FIXTURE, area="unselected"))

    def test_every_payload_carries_the_required_fields(self) -> None:
        for run in build(FIXTURE):
            with self.subTest(run["name"]):
                self.assertEqual(HEAD, run["head_sha"])
                self.assertEqual("completed", run["status"])
                self.assertEqual("neutral", run["conclusion"])
                self.assertTrue(run["output"]["title"].startswith("ACCEPTED GAP"))
                self.assertTrue(run["output"]["summary"].startswith("**ACCEPTED GAP**"))
                self.assertIn(".github/ci/environments.json", run["details_url"])

    def test_the_summary_names_the_cell_owner_expiry_and_reason(self) -> None:
        run = build(FIXTURE)[0]
        summary = run["output"]["summary"]
        for fact in (
            "`pkg`",
            "`alpha-cli`",
            "`L2`",
            "`windows-latest`",
            "@yankeeinlondon",
            "2027-01-31",
            "no L2 terminal backend is provisionable on Windows",
        ):
            self.assertIn(fact, summary)
        self.assertEqual("ACCEPTED GAP: alpha-cli L2 on windows-latest", run["output"]["title"])

    def test_the_text_carries_revocation_instructions_and_the_closing_work(self) -> None:
        with_closes, without_closes = build(FIXTURE)
        text = with_closes["output"]["text"]
        self.assertIn("## Changing or revoking this acceptance", text)
        self.assertIn("Remove the entry", text)
        self.assertIn("expired entry is a blocking `POLICY GAP`", text)
        self.assertIn("`environments.windows-latest.capabilities.tmux`", text)
        self.assertIn(
            f"[`features/_unscheduled/windows-l2-ci-leg`]({SERVER}/{REPO}/tree/{HEAD}/"
            "features/_unscheduled/windows-l2-ci-leg)",
            text,
        )
        self.assertIn("never alters the run's conclusion", text)
        # A gap that names no `closes` says so instead of linking nothing.
        self.assertIn("names no tracked work", without_closes["output"]["text"])
        self.assertNotIn("_unscheduled", without_closes["output"]["text"])

    def test_the_link_anchors_the_policy_entry_line_when_it_can(self) -> None:
        anchored, unanchored = build(FIXTURE)
        self.assertEqual(
            f"{SERVER}/{REPO}/blob/{HEAD}/.github/ci/environments.json#L{TMUX_LINE}",
            anchored["details_url"],
        )
        # `wsl2-ubuntu` is not in the fixture policy; the link still reaches
        # the file, just without a line.
        self.assertEqual(
            f"{SERVER}/{REPO}/blob/{HEAD}/.github/ci/environments.json",
            unanchored["details_url"],
        )
        # No policy source at all (unreadable file) degrades the same way.
        self.assertTrue(
            all(
                "#L" not in run["details_url"]
                for run in build(FIXTURE, policy_source=None)
            )
        )

    def test_the_line_search_stays_inside_the_environment_block(self) -> None:
        # `ubuntu-latest` has a `tmux` key too (a boolean, one line above the
        # Windows block); the Windows entry must win for `windows-latest` and
        # the Ubuntu one for `ubuntu-latest`.
        self.assertEqual(TMUX_LINE, publish_gaps.policy_line(POLICY_SOURCE, "windows-latest", "tmux"))
        self.assertEqual(UBUNTU_TMUX_LINE, publish_gaps.policy_line(POLICY_SOURCE, "ubuntu-latest", "tmux"))
        self.assertIsNone(publish_gaps.policy_line(POLICY_SOURCE, "windows-latest", "kitty"))
        self.assertIsNone(publish_gaps.policy_line(POLICY_SOURCE, "macos-latest", "tmux"))


class RefusalTests(unittest.TestCase):
    """A `neutral` check for a cell the rollup will block on is a lie; refuse it."""

    def assertRefused(self, document: dict, fragment: str) -> None:
        with self.assertRaises(publish_gaps.Refusal) as caught:
            build(document)
        self.assertIn(fragment, str(caught.exception))

    def test_a_gap_without_a_governance_record_is_refused(self) -> None:
        self.assertRefused(plan(cell("alpha", "pkg", "windows-latest", gap=None)), "no governance record")

    def test_an_ungoverned_gap_is_refused(self) -> None:
        self.assertRefused(
            plan(cell("alpha", "pkg", "windows-latest", gap={"capability": "tmux", "governed": False})),
            "ungoverned",
        )

    def test_an_incomplete_record_is_refused(self) -> None:
        self.assertRefused(plan(cell("alpha", "pkg", "windows-latest", gap=gap(owner=""))), "lacks owner")

    def test_an_expired_acceptance_is_refused(self) -> None:
        self.assertRefused(
            plan(cell("alpha", "pkg", "windows-latest", gap=gap(expiry="2026-09-11"))),
            "expired on 2026-09-11",
        )
        # The boundary: an acceptance expiring today still holds today.
        self.assertEqual(1, len(build(plan(cell("alpha", "pkg", "windows-latest", gap=gap(expiry="2026-09-12"))))))

    def test_an_unreadable_expiry_is_refused(self) -> None:
        self.assertRefused(plan(cell("alpha", "pkg", "windows-latest", gap=gap(expiry="soon"))), "unreadable expiry")

    def test_a_refusal_stops_the_whole_area(self) -> None:
        # The good cell first, the bad one second: nothing is published for
        # either, because the job's failure is what a reader must see.
        document = plan(
            cell("alpha", "pkg", "windows-latest"),
            cell("alpha", "pkg", "wsl2-ubuntu", gap=gap(expiry="2020-01-01")),
        )
        with self.assertRaises(publish_gaps.Refusal):
            build(document)


class CommandFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.plan_path = self.root / "resolved-plan.json"
        self.plan_path.write_text(json.dumps(FIXTURE), encoding="utf-8")
        self.policy_path = self.root / "environments.json"
        self.policy_path.write_text(POLICY_SOURCE, encoding="utf-8")

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def run_publisher(self, *extra: str, area: str = "pkg", env: dict[str, str] | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            [
                sys.executable,
                str(MODULE),
                "--plan",
                str(self.plan_path),
                "--area",
                area,
                "--head-sha",
                HEAD,
                "--repo",
                REPO,
                "--server",
                SERVER,
                "--environments",
                str(self.policy_path),
                "--today",
                TODAY.isoformat(),
                *extra,
            ],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )


class CommandTests(CommandFixture):
    def test_dry_run_writes_the_payloads_and_calls_nothing(self) -> None:
        out = self.root / "payloads.json"
        # An empty PATH: were the dry run to reach for `gh`, it could not find it.
        result = self.run_publisher("--dry-run", "--out", str(out), env={"PATH": ""})
        self.assertEqual(0, result.returncode, result.stderr)
        written = json.loads(out.read_text(encoding="utf-8"))
        self.assertEqual(build(FIXTURE), written)
        self.assertEqual(2, result.stdout.count("would publish neutral check"))

    def test_an_area_without_an_accepted_gap_exits_zero_with_a_note(self) -> None:
        result = self.run_publisher("--dry-run", area="unselected", env={"PATH": ""})
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(
            "publish-gaps: area 'unselected' carries no accepted gap; nothing to publish",
            result.stdout.strip(),
        )

    def test_a_refused_cell_exits_two_and_names_the_cell(self) -> None:
        self.plan_path.write_text(
            json.dumps(plan(cell("alpha", "pkg", "windows-latest", gap=gap(expiry="2020-01-01")))),
            encoding="utf-8",
        )
        result = self.run_publisher("--dry-run", env={"PATH": ""})
        self.assertEqual(2, result.returncode)
        self.assertIn("refusing to publish a neutral check", result.stderr)
        self.assertIn("alpha/windows-latest/L2 acceptance expired on 2020-01-01", result.stderr)

    def test_an_unreadable_plan_exits_one(self) -> None:
        self.plan_path.write_text("{not json", encoding="utf-8")
        result = self.run_publisher("--dry-run", env={"PATH": ""})
        self.assertEqual(1, result.returncode)
        self.assertIn("cannot read the resolved plan", result.stderr)

        self.plan_path.write_text(json.dumps({"schema_version": 2}), encoding="utf-8")
        result = self.run_publisher("--dry-run", env={"PATH": ""})
        self.assertEqual(1, result.returncode)
        self.assertIn("no 'cells' list", result.stderr)


@unittest.skipIf(sys.platform == "win32", "the stub gh is a POSIX executable script")
class GhInvocationTests(CommandFixture):
    """The exact call the hosted job makes, captured by a stub `gh` on a private PATH."""

    def setUp(self) -> None:
        super().setUp()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.log = self.root / "gh-calls.jsonl"
        stub = self.bin / "gh"
        stub.write_text(
            f"#!{sys.executable}\n"
            "import json, os, sys\n"
            "with open(os.environ['GH_STUB_LOG'], 'a', encoding='utf-8') as log:\n"
            "    log.write(json.dumps({'argv': sys.argv[1:], 'stdin': sys.stdin.read()}) + '\\n')\n"
            "sys.exit(int(os.environ.get('GH_STUB_EXIT', '0')))\n",
            encoding="utf-8",
        )
        stub.chmod(0o755)

    def calls(self) -> list[dict]:
        return [json.loads(line) for line in self.log.read_text(encoding="utf-8").splitlines()]

    def environment(self, **extra: str) -> dict[str, str]:
        return {"PATH": str(self.bin), "GH_TOKEN": "stub", "GH_STUB_LOG": str(self.log), **extra}

    def test_one_post_per_cell_to_the_check_runs_endpoint(self) -> None:
        result = self.run_publisher(env=self.environment())
        self.assertEqual(0, result.returncode, result.stderr)
        calls = self.calls()
        self.assertEqual(2, len(calls))
        for call, expected in zip(calls, build(FIXTURE)):
            self.assertEqual(
                ["api", "--method", "POST", f"repos/{REPO}/check-runs", "--input", "-"],
                call["argv"],
            )
            self.assertEqual(expected, json.loads(call["stdin"]))
        self.assertEqual(2, result.stdout.count("published neutral check"))

    def test_a_failed_post_exits_one_and_reports_the_check(self) -> None:
        result = self.run_publisher(env=self.environment(GH_STUB_EXIT="1"))
        self.assertEqual(1, result.returncode)
        self.assertIn("gh api POST repos/yankeeinlondon/rusty-biscuit/check-runs failed", result.stderr)
        self.assertIn("accepted gap: pkg / alpha-cli / L2 (windows-latest)", result.stderr)
        # The first failure stops the loop; the second cell is never attempted.
        self.assertEqual(1, len(self.calls()))

    def test_a_dry_run_never_reaches_gh(self) -> None:
        result = self.run_publisher("--dry-run", env=self.environment())
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertFalse(self.log.exists())


if __name__ == "__main__":
    unittest.main()
