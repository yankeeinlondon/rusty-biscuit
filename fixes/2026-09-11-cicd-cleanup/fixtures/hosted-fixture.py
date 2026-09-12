#!/usr/bin/env python3
"""Prepare an inert overlay for a reduced hosted workflow integration experiment.

Run `prepare DESTINATION` from the candidate checkout. Apply that overlay only
in an isolated full checkout after reviewing its manifest. No Git notes,
commits, pushes, dispatches, or product tests are performed by this program.
"""
from __future__ import annotations

import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import xml.etree.ElementTree as ET

ROOT = Path.cwd()
CASES = {
    "accepted": "fixture-accepted",
    "unaccepted": "fixture-unaccepted",
    "missing-report": "fixture-missing",
    "setup-failure": "fixture-setup",
    "reused-failure": "fixture-reused-failure",
    "accepted-reused-failure": "fixture-accepted-reused",
    "empty": None,
}
FAILURES = {"fixture-accepted", "fixture-unaccepted", "fixture-reused-failure", "fixture-accepted-reused"}


def command(*args, input=None):
    return subprocess.run(args, input=input, text=True, capture_output=True, check=True).stdout


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def workload(package, tier="L1", phase="gate", stage=None, environment="ubuntu-latest"):
    if not package.startswith("fixture-"):
        raise ValueError("fixture workload refuses a product package")
    if phase == "setup":
        return int(package == "fixture-setup")
    if package == "fixture-missing":
        return 1
    start = time.monotonic()
    failure = None
    try:
        assert sum([1, 2]) == (4 if package in FAILURES else 3), "intentional integration fixture assertion"
    except AssertionError as error:
        failure = str(error)
    duration = time.monotonic() - start
    if tier != "L1":
        return int(failure is not None)
    stage = stage or ROOT / "target/nextest/ci-reports"
    stage.mkdir(parents=True, exist_ok=True)
    suites = ET.Element("testsuites")
    suite = ET.SubElement(suites, "testsuite", name=package, tests="1", failures=str(int(failure is not None)), errors="0", skipped="0", time=str(duration))
    case = ET.SubElement(suite, "testcase", name="fixture_assertion", classname=package, time=str(duration))
    if failure:
        ET.SubElement(case, "failure", message=failure)
    ET.ElementTree(suites).write(stage / f"{package}.xml", encoding="unicode")
    record = {"tier": tier, "package": package, "xml": f"{package}.xml", "exit_code": int(failure is not None),
              "environment": environment, "duration_s": int(duration), "report_present": True}
    with (stage / "manifest.jsonl").open("a") as output:
        output.write(json.dumps(record) + "\n")
    return int(failure is not None)


def scope(case):
    sys.path.insert(0, str(ROOT / "scripts/ci"))
    import affected_scope as planner
    import schema
    case = case or json.loads((ROOT / "fixture-case.json").read_text())["case"]
    primary = CASES[case]
    names = [primary, "fixture-peer", "fixture-nested", "fixture-reused-pass", "fixture-gap"] if primary else []
    packages = []
    for name in names:
        area = "fixture/nested" if name == "fixture-nested" else name
        ci = {"tests": {"tiers": ["L1", "L2"], "l2-backends": ["wezterm"]}} if name == "fixture-gap" else {}
        packages.append({"id": name, "name": name, "manifest_path": str(ROOT / area / "lib/Cargo.toml"),
                         "metadata": {"ci": ci}, "targets": [{"kind": ["lib"]}]})
    metadata = {"workspace_members": names, "packages": packages, "resolve": {"nodes": [{"id": name, "deps": []} for name in names]}}
    measured = json.loads((ROOT / "fixture-evidence/outcomes.json").read_text())
    local_environment = measured["environment"]
    environments = [entry for entry in planner.load_environments(ROOT / ".github/ci/environments.json") if entry["name"] in {"ubuntu-latest", local_environment}]
    policy = planner.package_ci_policy(planner.workspace_packages(metadata), {entry["runner"] for entry in environments}, ROOT)
    accepted = [cell for cell in measured["cells"] if cell["package"] in names]
    reused_packages = {cell["package"] for cell in accepted}
    paths = [str(Path(package["manifest_path"]).parent.relative_to(ROOT) / "src/lib.rs") for package in packages]
    plan = planner.calculate_scope(paths, ROOT, metadata, environments, policy, accepted_cells=accepted)
    # The reduced experiment tests L1 normalization and gap routing only.
    plan["cells"] = [cell for cell in plan["cells"] if cell["gate"] != "lint" and
                     cell["environment"] == (local_environment if cell["package"] in reused_packages else "ubuntu-latest")]
    for package in plan["packages"]:
        package["gates"] = [gate for gate in package["gates"] if gate != "lint"]
    errors = schema.validate_resolved_plan(plan)
    if errors:
        raise ValueError(errors)
    projection = planner.legacy_scope_document(plan)
    projection["gap_areas"] = sorted({cell["area"] for cell in plan["cells"] if cell["state"] == "accepted-gap"})
    assert all(not record["wsl"] for record in projection["matrix"])
    assert all(cell["environment"] == "ubuntu-latest" for cell in plan["cells"] if cell["execution"] == "execute")
    write_json(ROOT / "resolved-plan.json", plan)
    write_json(ROOT / "scope.json", projection)
    with Path(os.environ["GITHUB_OUTPUT"]).open("a") as output:
        for key in ["scheduled_areas", "area_slugs", "area_matrix", "gap_areas"]:
            output.write(f"{key}={json.dumps(projection[key], separators=(',', ':'))}\n")
        output.write(f"has_packages={str(bool(names)).lower()}\n")


def prepare(destination):
    destination = Path(destination).resolve()
    if destination == ROOT or ROOT in destination.parents:
        raise ValueError("output must be outside the candidate checkout")
    destination.mkdir(parents=True, exist_ok=True)
    if any(destination.iterdir()):
        raise ValueError("output directory must be empty")
    host = json.loads(command("sniff", "os", "--json"))
    environment = {"MacOS": "macos-latest", "Windows": "windows-latest", "Linux": "ubuntu-latest"}.get(host["os_type"])
    if not environment or "wsl" in str(host).lower():
        raise ValueError("fixture preparation requires a native supported host; no WSL fixture execution is authorized")
    measurements = []
    for name in ["fixture-reused-pass", "fixture-reused-failure", "fixture-accepted-reused"]:
        stage = destination / "fixture-evidence" / name
        failed = workload(name, stage=stage, environment=environment)
        measurement = json.loads((stage / "manifest.jsonl").read_text().splitlines()[-1])
        measurements.append({"package": name, "environment": environment, "gate": "L1", "origin": "local",
                             "outcome": "fail" if failed else "pass", "completion": "complete", "exit_code": failed,
                             "counts": {"total": 1, "passed": 1-failed, "failed": failed, "errored": 0, "skipped": 0},
                             "duration_s": measurement["duration_s"], "failed_tests": [f"{name}::fixture_assertion"] if failed else [],
                             "evidence": f"integration-fixture:fixture-evidence/{name} (measured locally; not a product receipt)",
                             "host": {"os": host["os_type"], "kernel": host["kernel"], "report_dir": str(stage)}})
    write_json(destination / "fixture-evidence/outcomes.json", {"environment": environment, "cells": measurements})
    workflows = ROOT / ".github/workflows"
    ci = json.loads(command("bf", str(workflows / "ci.yml"), "--json"))
    package = json.loads(command("bf", str(workflows / "_package-ci.yml"), "--json"))
    changes = []
    for job_name, job in package["jobs"].items():
        for index, step in enumerate(job.get("steps", [])):
            if step.get("name") == "Record producer status" or str(step.get("uses", "")).startswith(("actions/checkout@", "actions/upload-artifact@")):
                continue
            before = copy.deepcopy(step)
            tier = {"test": "L1", "test-l2": "L2", "test-browser": "browser"}.get(job_name, job_name)
            if step.get("continue-on-error"):
                step.pop("uses", None)
                step.pop("with", None)
                step["shell"] = "bash"
                step["run"] = f'python3 scripts/ci/hosted_fixture.py workload "${{{{ inputs.package }}}}" {tier}'
            else:
                step.pop("uses", None)
                step.pop("with", None)
                step["shell"] = "bash"
                step["run"] = 'python3 scripts/ci/hosted_fixture.py setup "${{ inputs.package }}"' if job_name == "test" and step.get("name") == "Set up the pinned Rust toolchain" else "true"
            changes.append({"job": job_name, "step_index": index, "before": before, "after": step})
    area = copy.deepcopy(ci["jobs"]["area-ci"])
    area["needs"] = ["scope"]
    gate = copy.deepcopy(ci["jobs"]["ci-gate"])
    gate["needs"] = ["scope", "area-ci"]
    gate["steps"][0]["env"]["RESULTS"] = "scope:${{ needs.scope.result }}\narea-ci:${{ needs.area-ci.result }}\n"
    top = {"name": "ci-fixture", "on": {"workflow_dispatch": {"inputs": {"case": {"type": "choice", "required": True, "options": list(CASES)}}},
                                         "pull_request": {"branches": ["codex/cicd-review7-fixture-base"]}},
           "permissions": {"contents": "read"}, "jobs": {"scope": {"runs-on": "ubuntu-latest", "outputs": {
               key: "${{ steps.fixture.outputs." + key + " }}" for key in ["has_packages", "scheduled_areas", "area_slugs", "area_matrix", "gap_areas"]},
               "steps": [{"uses": "actions/checkout@v5"}, {"name": "Prepare explicit integration fixture plan", "id": "fixture", "env": {"FIXTURE_CASE": "${{ inputs.case }}"},
                          "run": 'python3 scripts/ci/hosted_fixture.py scope "$FIXTURE_CASE"'},
                         *[step for step in ci["jobs"]["scope"]["steps"] if step.get("name") in {"Upload the resolved package policy", "Upload the resolved execution plan"}],
                         {"uses": "actions/upload-artifact@v6", "with": {"name": "fixture-evidence", "path": "fixture-evidence", "if-no-files-found": "ignore"}}]},
               "area-ci": area, "ci-gate": gate}}
    for name, document in [("ci.yml", top), ("_package-ci.yml", package)]:
        path = destination / ".github/workflows" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(command("bf", "--input-format", "json", "--yaml", input=json.dumps(document)))
    for name in ["_area-ci.yml", "_wsl-ci.yml"]:
        shutil.copyfile(workflows / name, destination / ".github/workflows" / name)
    runtime = destination / "scripts/ci/hosted_fixture.py"
    runtime.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(Path(__file__), runtime)
    write_json(destination / "fixture-case.json", {"case": "accepted"})
    baseline = 'schema_version = 2\n'
    for name, measured_environment in [("fixture-accepted", "ubuntu-latest"), ("fixture-accepted-reused", environment)]:
        baseline += f'\n[[failure]]\npackage = "{name}"\nenvironment = "{measured_environment}"\ntier = "L1"\nowner = "fixture"\nreason = "Controlled integration assertion; not product acceptance"\nsource_run = "fixture"\nexpiry = "2027-01-31"\n'
    path = destination / ".github/ci/ci-baseline.toml"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(baseline)
    write_json(destination / "fixture-validation.json", {
        "status": "prepared-not-hosted", "candidate_head": command("git", "rev-parse", "HEAD").strip(),
        "candidate_workflow_blobs": {name: command("git", "hash-object", str(workflows / name)).strip() for name in ["ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml"]},
        "candidate_input_blobs": {str(path.relative_to(ROOT)): command("git", "hash-object", str(path)).strip()
                                  for path in sorted([*ROOT.glob("scripts/ci/*.py"), *ROOT.glob("scripts/*.rs"), Path(__file__).resolve(), ROOT / "scripts/Cargo.toml", ROOT / "scripts/Cargo.lock", ROOT / ".github/ci/environments.json", ROOT / "rust-toolchain.toml"]) if path.is_file()},
        "unchanged": ["_area-ci.yml", "_wsl-ci.yml", "producer status steps", "artifact upload steps", "package job graph/permissions", "ci-gate shell"],
        "deltas": {"top": "explicit fixture scope; only scope and area-ci retained as gate dependencies; dispatch and one fixture-base PR trigger", "package_steps": changes, "baseline": "two explicitly named fixture failures only"},
        "restrictions": ["Ubuntu-only fixture execution; reused cells retain sniff-detected local environment", "wsl=false; fourth-level skip only, no WSL runtime proof", "no product suites", "no validation or scope Git notes", "apply only to isolated full candidate tree; never push existing product CI triggers"],
        "activation": {"method": "apply subcommand on an isolated full candidate checkout; then review every outgoing trigger and active constraint before any push or dispatch",
                       "remove_unrelated_workflows": [path.name for path in workflows.iterdir() if path.suffix in {".yml", ".yaml"} and path.name not in {"ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml"}],
                       "retained_trigger": "ci.yml workflow_dispatch and pull_request targeting codex/cicd-review7-fixture-base only; reusable workflows workflow_call only",
                       "forbidden_shortcut": "Do not push feat/unifi to bootstrap this fixture; its product plan schedules prohibited WSL work."},
        "expected_gate": {case: "success" if case in {"accepted", "accepted-reused-failure", "empty"} else "failure" for case in CASES},
        "required_observations": ["area-ci (fixture/nested) / fixture-nested label resolved", "fixture-reused-pass result without L1 producer", "gap publisher runs only fixture-gap; neutral cell check", "per-area result slices available; no cross-area acceptance", "accepted and accepted-reused failure green; unaccepted and reused failure red", "missing report and setup failure block", "empty scope skips area-ci and gate passes"],
        "artifacts": ["ci-scope", "ci-resolved-plan", "fixture-evidence", "status-fixture-*-L1-ubuntu-latest", "junit-fixture-*-L1-ubuntu-latest", "ci-results-fixture--nested"],
        "not_proven": ["product workloads", "scope or Git-note receipt verification (fixture outcomes are measured locally but seeded directly into the plan)", "WSL runtime", "PR-event labels until separately run on an isolated fixture PR", "hosted behavior until dispatch records are collected"]})
    print(destination)


def apply(overlay, checkout):
    overlay, checkout = Path(overlay).resolve(), Path(checkout).resolve()
    if checkout == ROOT or ROOT in checkout.parents or not (checkout / ".git").exists():
        raise ValueError("apply requires an isolated full Git checkout outside the candidate checkout")
    manifest = json.loads((overlay / "fixture-validation.json").read_text())
    for relative, expected in manifest["candidate_input_blobs"].items():
        if command("git", "hash-object", str(checkout / relative)).strip() != expected:
            raise ValueError(f"candidate input differs: {relative}")
    for name, expected in manifest["candidate_workflow_blobs"].items():
        if command("git", "hash-object", str(checkout / ".github/workflows" / name)).strip() != expected:
            raise ValueError(f"candidate workflow differs: {name}")
    for path in (checkout / ".github/workflows").iterdir():
        if path.suffix in {".yml", ".yaml"} and path.name not in {"ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml"}:
            path.unlink()
    shutil.copytree(overlay, checkout, dirs_exist_ok=True)
    print("Fixture overlay applied; no commit, push, dispatch, or Git note publication performed.")


def verify(overlay):
    """Run each fixture through the shipped status shell and Rust rollup locally."""
    global ROOT
    repo = ROOT
    overlay = Path(overlay).resolve()
    binary = repo / "scripts/target/debug/ci-rollup"
    workflow = json.loads(command("bf", str(repo / ".github/workflows/_package-ci.yml"), "--json"))
    status = next(step["run"] for step in workflow["jobs"]["test"]["steps"] if step.get("name") == "Record producer status")
    prior_output = os.environ.get("GITHUB_OUTPUT")
    results = {}
    try:
        for case in CASES:
            with tempfile.TemporaryDirectory(prefix="hosted-fixture-verify-") as temporary:
                ROOT = Path(temporary)
                shutil.copytree(repo / "scripts/ci", ROOT / "scripts/ci", ignore=shutil.ignore_patterns("__pycache__"))
                shutil.copytree(repo / ".github/ci", ROOT / ".github/ci")
                shutil.copytree(overlay / "fixture-evidence", ROOT / "fixture-evidence")
                os.environ["GITHUB_OUTPUT"] = str(ROOT / "output")
                scope(case)
                plan = json.loads((ROOT / "resolved-plan.json").read_text())
                artifacts = ROOT / "artifacts"
                artifacts.mkdir()
                for cell in plan["cells"]:
                    if cell["execution"] != "execute" or cell["state"] != "pending":
                        continue
                    name = cell["package"]
                    runner = ROOT / "runner" / name
                    runner.mkdir(parents=True)
                    setup = workload(name, phase="setup")
                    outcome = 1 if setup else workload(name, stage=artifacts / f"junit-{name}")
                    environment = {**os.environ, "RUNNER_TEMP": str(runner), "PACKAGE": name, "JOB": "L1", "ENVIRONMENT": "ubuntu-latest",
                                   "JOB_STATUS": "failure" if setup else "success", "GATE": "skipped" if setup else ("failure" if outcome else "success"),
                                   "COMPANION": "skipped", "COMPANION_SUITES": "[]"}
                    subprocess.run(["bash", "-c", status], env=environment, check=True, capture_output=True)
                    shutil.copytree(runner / "status", artifacts / f"status-{name}")
                verdicts = {}
                for area in plan["areas"]:
                    name = area["area"]
                    output = ROOT / f"{name.replace('/', '--')}.json"
                    run = subprocess.run([str(binary), "rollup", "--artifacts", str(artifacts), "--plan", str(ROOT / "resolved-plan.json"),
                                          "--policy", str(ROOT / "scope.json"), "--environments", str(ROOT / ".github/ci/environments.json"),
                                          "--area", name, "--out", str(output), "--run-id", "fixture"], capture_output=True, text=True)
                    if run.returncode not in (0, 2):
                        raise RuntimeError(run.stderr)
                    verdict = subprocess.run([str(binary), "verdict", "--results", str(output), "--area", name,
                                              "--baseline", str(overlay / ".github/ci/ci-baseline.toml")], capture_output=True, text=True)
                    if verdict.returncode not in (0, 2):
                        raise RuntimeError(verdict.stderr)
                    verdicts[name] = verdict.returncode
                expected = case in {"accepted", "accepted-reused-failure", "empty"}
                if (not any(verdicts.values())) != expected:
                    raise RuntimeError(f"{case}: unexpected verdicts {verdicts}")
                results[case] = verdicts
    finally:
        ROOT = repo
        if prior_output is None:
            os.environ.pop("GITHUB_OUTPUT", None)
        else:
            os.environ["GITHUB_OUTPUT"] = prior_output
    write_json(overlay / "fixture-local-verification.json", {"boundary": "local synthetic workflow steps and real rollup; no hosted execution", "area_exit_codes": results})
    print(f"Verified {len(results)} fixture scenarios; hosted verification remains required.")


if __name__ == "__main__":
    operation, *arguments = sys.argv[1:]
    if operation == "prepare":
        prepare(*arguments)
    elif operation == "apply":
        apply(*arguments)
    elif operation == "verify":
        verify(*arguments)
    elif operation == "scope":
        scope(*arguments)
    elif operation == "setup":
        sys.exit(workload(arguments[0], phase="setup"))
    elif operation == "workload":
        sys.exit(workload(*arguments))
    else:
        raise ValueError(f"unknown operation: {operation}")
