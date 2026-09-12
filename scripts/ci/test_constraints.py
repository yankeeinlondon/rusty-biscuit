#!/usr/bin/env python3
"""Contract tests for the persisted execution-constraint store (AC17).

The store guards a trigger boundary, so the fixtures here are about refusal:
what blocks a push, what stops blocking it, and what a reader is told. The
store's *location* is Open Question 2 and is deliberately not asserted — every
fixture names the directory explicitly, exactly as the hook suite does.
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

import constraints  # noqa: E402
import schema  # noqa: E402


TODAY = date(2026, 9, 11)
MODULE = Path(__file__).resolve().parent / "constraints.py"


class StoreFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.store = Path(self.temporary_directory.name)

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def write(self, name: str, document: object) -> Path:
        path = self.store / f"{name}.json"
        path.write_text(
            document if isinstance(document, str) else json.dumps(document),
            encoding="utf-8",
        )
        return path

    def constraint(self, **overrides: object) -> dict:
        document = {
            "environment": "wsl2-ubuntu",
            "reason": "do not rerun WSL for this branch",
            "owner": "ken",
            "expiry": "2099-01-01",
        }
        document.update(overrides)
        return document

    def load(self, **overrides: object):
        return constraints.load(str(self.store), today=TODAY, **overrides)


class LoadTests(StoreFixture):
    def test_an_unexpired_constraint_is_active(self) -> None:
        self.write("current", self.constraint())
        active, expired, malformed = self.load()
        self.assertEqual(["wsl2-ubuntu"], [entry.environment for entry in active])
        self.assertEqual([], expired)
        self.assertEqual([], malformed)

    def test_a_past_expiry_is_reported_and_stops_binding(self) -> None:
        self.write("current", self.constraint(expiry="2020-01-01"))
        active, expired, malformed = self.load()
        self.assertEqual([], active)
        self.assertEqual(1, len(expired))
        self.assertEqual([], malformed)

    def test_the_expiry_boundary_is_inclusive_of_today(self) -> None:
        self.write("current", self.constraint(expiry=TODAY.isoformat()))
        active, expired, _ = self.load()
        self.assertEqual(1, len(active), "a constraint expiring today still binds today")
        self.assertEqual([], expired)

    def test_an_empty_or_absent_store_binds_nothing(self) -> None:
        self.assertEqual(([], [], []), self.load())
        self.assertEqual(([], [], []), constraints.load("", today=TODAY))
        self.assertEqual(
            ([], [], []), constraints.load(str(self.store / "absent"), today=TODAY)
        )

    def test_a_missing_field_makes_the_record_malformed_not_absent(self) -> None:
        for field in constraints.REQUIRED_FIELDS:
            with self.subTest(field=field):
                document = self.constraint()
                del document[field]
                self.write("current", document)
                _active, _expired, malformed = self.load()
                self.assertEqual(1, len(malformed))
                self.assertIn(field, malformed[0].describe())

    def test_unreadable_json_is_malformed_rather_than_ignored(self) -> None:
        self.write("current", "{not json")
        _active, _expired, malformed = self.load()
        self.assertEqual(1, len(malformed))

    def test_an_unknown_field_is_rejected_so_a_typo_cannot_widen_a_constraint(self) -> None:
        self.write("current", self.constraint(environments="wsl2-ubuntu"))
        _active, _expired, malformed = self.load()
        self.assertEqual(1, len(malformed))
        self.assertIn("environments", malformed[0].describe())

    def test_a_branch_scoped_constraint_binds_only_that_branch(self) -> None:
        self.write("current", self.constraint(branch="feat/unifi"))
        self.assertEqual(1, len(self.load(branches=["feat/unifi"])[0]))
        self.assertEqual([], self.load(branches=["main"])[0])
        # An unnamed branch means "this checkout is not claiming to be another",
        # so an unfiltered caller still sees it.
        self.assertEqual(1, len(self.load()[0]))

    def test_a_branch_scoped_constraint_binds_when_any_supplied_name_matches(self) -> None:
        # A renamed refspec (`git push origin feature:other`) is validated under
        # `other` but was restricted under `feature`; both names are supplied
        # and the record binds on either.
        self.write("current", self.constraint(branch="feature"))
        self.assertEqual(1, len(self.load(branches=["other", "feature"])[0]))
        self.assertEqual(1, len(self.load(branches=["feature", "other"])[0]))
        self.assertEqual([], self.load(branches=["other", "main"])[0])
        # An empty name is no identity and neither matches nor exempts.
        self.assertEqual(1, len(self.load(branches=[""])[0]))
        self.assertEqual([], self.load(branches=["", "main"])[0])

    def test_a_repository_scoped_constraint_matches_every_spelling_of_its_remote(self) -> None:
        self.write("current", self.constraint(repository="github.com/yankeeinlondon/rusty-biscuit"))
        for remote in (
            "git@github.com:yankeeinlondon/rusty-biscuit.git",
            "https://github.com/yankeeinlondon/rusty-biscuit.git",
            "ssh://git@github.com/yankeeinlondon/rusty-biscuit",
            "github.com/yankeeinlondon/rusty-biscuit",
        ):
            with self.subTest(remote=remote):
                self.assertEqual(1, len(self.load(repository=remote)[0]))
        self.assertEqual([], self.load(repository="git@github.com:someone-else/rusty-biscuit.git")[0])
        # A checkout with no origin cannot prove it is another repository.
        self.assertEqual(1, len(self.load(repository="")[0]))

    def test_a_record_may_spell_its_repository_as_a_full_url(self) -> None:
        self.write("current", self.constraint(repository="https://github.com/yankeeinlondon/rusty-biscuit.git"))
        self.assertEqual(1, len(self.load(repository="git@github.com:yankeeinlondon/rusty-biscuit.git")[0]))
        self.assertEqual([], self.load(repository="git@github.com:yankeeinlondon/other.git")[0])


class SatisfactionTests(StoreFixture):
    """A constraint is satisfied by a plan that schedules nothing it forbids."""

    def plan(self, execution: str, environment: str = "wsl2-ubuntu") -> dict:
        return {
            "cells": [
                {"environment": environment, "gate": "L1", "execution": execution},
                {"environment": "ubuntu-latest", "gate": "L1", "execution": "execute"},
            ]
        }

    def active(self, **overrides: object):
        self.write("current", self.constraint(**overrides))
        return self.load()[0]

    def test_a_scheduled_execution_leaves_the_constraint_unsatisfied(self) -> None:
        self.assertEqual(
            1, len(constraints.unsatisfied(self.active(), self.plan("execute")))
        )

    def test_a_reused_cell_satisfies_the_constraint(self) -> None:
        self.assertEqual([], constraints.unsatisfied(self.active(), self.plan("reuse")))

    def test_an_omitted_cell_satisfies_the_constraint(self) -> None:
        self.assertEqual([], constraints.unsatisfied(self.active(), self.plan("omit")))

    def test_no_plan_satisfies_nothing(self) -> None:
        self.assertEqual(1, len(constraints.unsatisfied(self.active(), None)))

    def test_a_cell_the_planner_already_marked_prohibited_is_unsatisfied(self) -> None:
        # `just ci-local --plan` with the store resolves an unsatisfied cell to
        # `omit`/`prohibited`; that plan must not read as satisfied here.
        plan = {
            "cells": [
                {
                    "environment": "wsl2-ubuntu",
                    "gate": "L1",
                    "execution": "omit",
                    "state": "prohibited",
                }
            ]
        }
        self.assertEqual(1, len(constraints.unsatisfied(self.active(), plan)))

    def test_a_gate_scoped_constraint_ignores_other_gates(self) -> None:
        active = self.active(gate="L2")
        self.assertEqual([], constraints.unsatisfied(active, self.plan("execute")))

    def test_another_environments_execution_does_not_violate_it(self) -> None:
        active = self.active()
        plan = {"cells": [{"environment": "macos-latest", "gate": "L1", "execution": "execute"}]}
        self.assertEqual([], constraints.unsatisfied(active, plan))

    def test_the_planner_input_names_every_forbidden_environment(self) -> None:
        self.write("current", self.constraint())
        self.write("other", self.constraint(environment="windows-latest"))
        active, _expired, _malformed = self.load()
        self.assertEqual(
            ["windows-latest", "wsl2-ubuntu"], constraints.prohibited_environments(active)
        )

    def test_every_forbidden_environment_is_a_known_one(self) -> None:
        self.write("current", self.constraint())
        active, _expired, _malformed = self.load()
        for environment in constraints.prohibited_environments(active):
            self.assertIn(environment, schema.ENVIRONMENTS)


class CommandTests(StoreFixture):
    """The CLI the hook and `just ci-local --plan` actually invoke."""

    def run_check(self, *extra: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(MODULE), "check", "--directory", str(self.store), *extra],
            capture_output=True,
            text=True,
            timeout=30,
        )

    def test_an_empty_store_exits_zero_and_says_nothing(self) -> None:
        result = self.run_check()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual("", result.stdout.strip())

    def test_an_active_constraint_blocks_and_names_itself(self) -> None:
        self.write("current", self.constraint())
        result = self.run_check()
        self.assertNotEqual(0, result.returncode)
        self.assertIn("wsl2-ubuntu", result.stderr)
        self.assertIn("do not rerun WSL for this branch", result.stderr)
        self.assertIn("ken", result.stderr)
        self.assertIn("2099-01-01", result.stderr)

    def test_an_expired_constraint_is_announced_and_does_not_block(self) -> None:
        self.write("current", self.constraint(expiry="2020-01-01"))
        result = self.run_check()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("expired", result.stdout)

    def test_a_malformed_constraint_blocks_rather_than_lapsing(self) -> None:
        self.write("current", "{not json")
        result = self.run_check()
        self.assertNotEqual(
            0,
            result.returncode,
            "an instruction that cannot be read is not one that can be ignored",
        )

    def test_a_plan_that_schedules_nothing_forbidden_unblocks_the_push(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {
                    "cells": [
                        {
                            "environment": "wsl2-ubuntu",
                            "gate": "L1",
                            "execution": "reuse",
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        result = self.run_check("--plan", str(plan))
        self.assertEqual(0, result.returncode, result.stderr)

    def test_a_plan_that_schedules_the_forbidden_cell_still_blocks(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {
                    "cells": [
                        {
                            "environment": "wsl2-ubuntu",
                            "gate": "L1",
                            "execution": "execute",
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("wsl2-ubuntu", result.stderr)

    def test_an_unreadable_plan_blocks_even_with_an_empty_store(self) -> None:
        plan = self.store.parent / "plan.json"
        plan.write_text("{not json", encoding="utf-8")
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode, "an unreadable plan proves nothing")
        self.assertIn("cannot read the resolved plan", result.stderr)

    def test_a_plan_without_readable_cells_blocks(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(json.dumps({"cells": "none"}), encoding="utf-8")
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("no readable 'cells' list", result.stderr)

    def test_a_missing_plan_file_blocks(self) -> None:
        result = self.run_check("--plan", str(self.store.parent / "absent.json"))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("cannot read the resolved plan", result.stderr)

    def test_the_hook_invocation_filters_by_repository_and_branch(self) -> None:
        self.write("other-branch", self.constraint(branch="main"))
        self.write("other-repo", self.constraint(repository="github.com/someone-else/rusty-biscuit"))
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {"cells": [{"environment": "wsl2-ubuntu", "gate": "L1", "execution": "execute"}]}
            ),
            encoding="utf-8",
        )
        result = self.run_check(
            "--plan",
            str(plan),
            "--repository",
            "git@github.com:yankeeinlondon/rusty-biscuit.git",
            "--branch",
            "feat/unifi",
        )
        self.assertEqual(0, result.returncode, result.stderr)
        result = self.run_check("--plan", str(plan), "--branch", "main")
        self.assertNotEqual(0, result.returncode)

    def test_a_repeated_branch_option_binds_a_record_under_any_of_the_names(self) -> None:
        self.write("renamed", self.constraint(branch="feature"))
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {"cells": [{"environment": "wsl2-ubuntu", "gate": "L1", "execution": "execute"}]}
            ),
            encoding="utf-8",
        )
        # The matching name comes FIRST: a parser that kept only the last value
        # would exempt this push.
        result = self.run_check("--plan", str(plan), "--branch", "feature", "--branch", "other")
        self.assertNotEqual(0, result.returncode, "a record under the first name must still bind")
        self.assertIn("wsl2-ubuntu", result.stderr)
        result = self.run_check("--plan", str(plan), "--branch", "other", "--branch", "feature")
        self.assertNotEqual(0, result.returncode, "a record under the last name must still bind")
        result = self.run_check("--plan", str(plan), "--branch", "other", "--branch", "main")
        self.assertEqual(0, result.returncode, result.stderr)

    def test_environments_lists_the_planner_input(self) -> None:
        self.write("current", self.constraint())
        result = subprocess.run(
            [
                sys.executable,
                str(MODULE),
                "environments",
                "--directory",
                str(self.store),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual("wsl2-ubuntu", result.stdout.strip())


class LocationTests(unittest.TestCase):
    """Open Question 2 is unruled, and the code says so in one place."""

    def test_the_default_location_is_still_the_open_question(self) -> None:
        self.assertEqual(
            "",
            constraints.default_directory(),
            "OQ2 (where a constraint is persisted) has no ruling on record; when "
            "one lands, fill default_directory() and delete this fixture",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
