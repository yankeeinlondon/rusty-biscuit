#!/usr/bin/env python3
"""One guard for every host tool the CI test suites shell out to.

A skip that says only "requires just" claims nothing about where the contract
IS checked, and under CI it is a green cell that verified nothing. Two rules
remove that:

* Every caller names the job or workflow that does enforce the contract.
  `enforced_by` is keyword-only and has no default, so a guard that cannot say
  where it is enforced cannot be written.
* The job that provisions a tool declares it with `BISCUIT_REQUIRE_<TOOL>`, and
  an absent tool then FAILS there. The declaration is per job rather than a
  blanket fail under `CI`: `preflight` runs three of these suites on three
  operating systems and provisions neither `sniff` nor `jq`, so a global rule
  would turn macOS and Windows red on every push.

Importable from a suite with no `sys.path` prologue: these suites are invoked
as `python3 scripts/ci/test_x.py`, which puts `scripts/ci` first on the path.
"""

from __future__ import annotations

import os
import shutil
import unittest
from collections.abc import Callable, Mapping


def declaring_variable(tool: str) -> str:
    """The environment variable a job sets to declare `tool` provisioned there."""
    return "BISCUIT_REQUIRE_" + "".join(
        character if character.isalnum() else "_" for character in tool
    ).upper()


def guard_message(missing: list[str], enforced_by: str, detail: str) -> str:
    tools = ", ".join(missing)
    verb = "is" if len(missing) == 1 else "are"
    message = (
        f"{tools} {verb} absent, so this contract did not run here. "
        f"It is enforced by {enforced_by}."
    )
    return f"{message} {detail}" if detail else message


def require_tools(
    *tools: str,
    enforced_by: str,
    detail: str = "",
    locate: Callable[[str], object] = shutil.which,
    environment: Mapping[str, str] | None = None,
) -> None:
    """Skip where a tool is genuinely absent; fail where a job declared it present.

    `locate` is the presence test, `shutil.which` by default. A caller whose
    requirement is stronger than "on PATH" — the scope step needs a Bash that
    can run `mapfile -d`, not any Bash — passes its own resolver so the guard
    stays the single mechanism rather than growing a second one beside it.

    ## Errors

    `AssertionError` when a missing tool's `BISCUIT_REQUIRE_<TOOL>` is set:
    the job claimed to provision it, so its absence is a provisioning
    regression and must be loud. `unittest.SkipTest` otherwise, carrying
    `enforced_by` so the skip says where the contract does run.
    """
    if not tools:
        raise ValueError("require_tools needs at least one tool name")
    variables = os.environ if environment is None else environment
    missing = [tool for tool in tools if not locate(tool)]
    if not missing:
        return
    message = guard_message(missing, enforced_by, detail)
    declared = [tool for tool in missing if variables.get(declaring_variable(tool))]
    if declared:
        names = ", ".join(declaring_variable(tool) for tool in declared)
        raise AssertionError(
            f"{names} declared {', '.join(declared)} provisioned in this job, but "
            f"it is not there. {message}"
        )
    raise unittest.SkipTest(message)


def requires_tools(*tools: str, **guard: object) -> Callable[[type], type]:
    """`require_tools` as a class decorator, run once before the class.

    Deliberately not `unittest.skipUnless`: that decorator can only ever skip,
    and skipping is the half of the contract this module exists to bound.
    """

    def decorate(cls: type) -> type:
        inherited = cls.setUpClass  # type: ignore[attr-defined]

        def setUpClass(bound_class: type) -> None:
            require_tools(*tools, **guard)  # type: ignore[arg-type]
            inherited.__func__(bound_class)

        cls.setUpClass = classmethod(setUpClass)  # type: ignore[assignment]
        return cls

    return decorate
