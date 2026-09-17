"""Offline macOS/Linux fixture; tests installed Claudine, not a production runner."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time


ROOT = Path(__file__).resolve().parent


def fake_provider():
    args = sys.argv[1:]
    if args == ["--version"]:
        print("goose 1.0.0-fixture")
        return
    text = " ".join(args)
    record = {
        "discovery": "DISCOVERY_ONLY" in text,
        "history": "PREVIOUS_PROSE" in text,
        "curated": "CURATED_LINK" in text,
        "reconciliation": "RECONCILE" in text,
        "maintenance": "MAINTAIN_SOURCES" in text,
        "review": "INDEPENDENT_REVIEW" in text,
        "pid": os.getpid(),
    }
    with open(os.environ["SPIKE_RECORDS"], "a", encoding="utf-8") as output:
        output.write(json.dumps(record) + "\n")
    if "SLOW_WORKER" in text:
        time.sleep(0.8)
    if "CANCEL_WORKER" in text:
        time.sleep(4)
    if "FAIL_WORKER" in text:
        sys.exit(7)
    print("Fixture source discovery and metadata result.")


def invoke(binary, env, cwd, command):
    start = time.monotonic()
    result = subprocess.run([binary, *command], cwd=cwd, env=env,
                            capture_output=True, text=True, timeout=25)
    return {"command": command, "code": result.returncode,
            "elapsed_seconds": round(time.monotonic() - start, 3),
            "stderr": result.stderr, "stdout": result.stdout}


def resume_probe(binary, env, cwd, ledger_path, document):
    # Proposed coordinator, deliberately not claimed as native Claudine behavior.
    ledger = json.loads(ledger_path.read_text())
    if ledger["used"] >= ledger["limit"] or ledger["consumed_seconds"] >= ledger["seconds"]:
        return {"stopped": True, "ledger": ledger}
    ledger["used"] += 1
    ledger_path.write_text(json.dumps(ledger))
    result = invoke(binary, env, cwd, ["compose", "--goose", document])
    ledger["consumed_seconds"] += result["elapsed_seconds"]
    ledger_path.write_text(json.dumps(ledger))
    return {"stopped": False, "code": result["code"], "ledger": ledger}


def run():
    binary = shutil.which("claudine")
    if not binary:
        raise RuntimeError("Installed Claudine required; do not substitute a simulated CLI")
    if os.name == "nt":
        raise RuntimeError("This disposable fake executable launcher was not ported to Windows")
    with tempfile.TemporaryDirectory(prefix="messenger-orchestration-") as directory:
        workspace = Path(directory)
        private_home = workspace / "home"
        private_home.mkdir()
        bin_dir = workspace / "bin"
        bin_dir.mkdir()
        fake = bin_dir / "goose"
        fake.write_text("#!" + sys.executable + "\n" + Path(__file__).read_text())
        fake.chmod(0o700)
        records = workspace / "records.jsonl"
        # Child-only home isolation mirrors Claudine's documented L1 fixture.
        env = {"HOME": str(private_home), "USERPROFILE": str(private_home),
               "PATH": os.pathsep.join([str(bin_dir), "/usr/bin", "/bin"]),
               "LANG": "C.UTF-8", "NO_COLOR": "1", "TERM": "dumb",
               "CLAUDINE_RENDEZVOUS_REPORT": "false", "PLAYA_DRY_RUN": "1",
               "PLAYA_SPOOL_DIR": str(workspace / "audio"),
               "SPIKE_RECORDS": str(records)}
        accepted = workspace / "accepted.md"
        accepted.write_text("Accepted baseline remains untouched.\n")
        baseline = accepted.read_bytes()
        documents = {
            "discovery.md": "DISCOVERY_ONLY provider.example shared questions schema",
            "reconcile.md": "RECONCILE PREVIOUS_PROSE CURATED_LINK discovery result",
            "maintain.md": "MAINTAIN_SOURCES CURATED_LINK proposed replacements",
            "review.md": "INDEPENDENT_REVIEW PREVIOUS_PROSE changed claims evidence",
            "fail.md": "FAIL_WORKER RECONCILE PREVIOUS_PROSE CURATED_LINK",
            "slow.md": "SLOW_WORKER synthetic source check",
            "cancel.md": "CANCEL_WORKER synthetic stalled source check",
        }
        for name, body in documents.items():
            (workspace / name).write_text("---\nagent: goose\n---\n" + body + "\n")
        sequence = "---\nsequence:\n" + "".join(
            "  - name: " + name + "\n    prompt: " + filename + "\n"
            for name, filename in [("discovery", "discovery.md"),
                                   ("reconciliation", "reconcile.md"),
                                   ("maintenance", "maintain.md"),
                                   ("review", "review.md")]) + "---\n"
        (workspace / "sequence.md").write_text(sequence)
        native = invoke(binary, env, workspace, ["sequence", "--goose", "sequence.md"])
        observed = [json.loads(line) for line in records.read_text().splitlines()] if records.exists() else []
        if native["code"] != 0 or len(observed) != 4:
            raise RuntimeError(json.dumps({"native": native, "observed": observed}))
        assert observed[0]["discovery"] and not observed[0]["history"] and not observed[0]["curated"]
        assert observed[1]["reconciliation"] and observed[1]["history"] and observed[1]["curated"]
        assert observed[2]["maintenance"] and observed[3]["review"]
        assert len({item["pid"] for item in observed}) == 4

        (workspace / "slow-sequence.md").write_text(
            "---\nsequence:\n  - name: first\n    prompt: slow.md\n  - name: second\n    prompt: slow.md\n---\n")
        native_timeout = invoke(binary, env, workspace,
                                ["sequence", "--goose", "--timeout", "1s", "slow-sequence.md"])
        assert native_timeout["code"] == 0, native_timeout["stderr"]
        assert native_timeout["elapsed_seconds"] > 1
        native_cancel = invoke(binary, env, workspace,
                               ["compose", "--goose", "--timeout", "1s", "cancel.md"])
        assert native_cancel["code"] != 0
        assert native_cancel["elapsed_seconds"] < 4
        canceled_pid = json.loads(records.read_text().splitlines()[-1])["pid"]
        try:
            os.kill(canceled_pid, 0)
        except ProcessLookupError:
            cancel_child_gone = True
        else:
            cancel_child_gone = False
        assert cancel_child_gone

        ledger_path = workspace / "ledger.json"
        ledger_path.write_text(json.dumps({"limit": 5, "used": 0, "seconds": 60,
                                          "consumed_seconds": 0}))
        prototype = []
        for document in ["discovery.md", "fail.md", "reconcile.md", "maintain.md", "review.md"]:
            prototype.append(resume_probe(binary, env, workspace, ledger_path, document))
        assert [item["code"] for item in prototype] == [0, 7, 0, 0, 0], prototype
        before = records.read_text()
        # A separate Python process reloads the persisted ledger: no in-memory counter survives.
        restart_env = dict(env, SPIKE_BINARY=binary, SPIKE_LEDGER=str(ledger_path),
                           SPIKE_CWD=str(workspace))
        restart = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--resume-probe"],
                                 env=restart_env, capture_output=True, text=True, timeout=25)
        restarted = json.loads(restart.stdout)
        assert restarted["stopped"] and restarted["ledger"]["used"] == 5
        assert records.read_text() == before
        ledger = dict(restarted["ledger"])
        ledger["limit"] = 10
        ledger["seconds"] = ledger["consumed_seconds"]
        ledger_path.write_text(json.dumps(ledger))
        time_restart = subprocess.run([sys.executable, str(Path(__file__).resolve()), "--resume-probe"],
                                      env=restart_env, capture_output=True, text=True, timeout=25)
        time_stopped = json.loads(time_restart.stdout)
        assert time_stopped["stopped"] and records.read_text() == before
        assert accepted.read_bytes() == baseline
        assert not (workspace / "audio").exists()
        result = {
            "native_version": subprocess.check_output([binary, "--version"], text=True).strip(),
            "native_binary": binary,
            "native_binary_bytes": Path(binary).stat().st_size,
            "source_revision_verified": False,
            "tested_os": "macOS; host identified separately with sniff os --json",
            "native_sequence": {key: value for key, value in native.items() if key not in {"stderr", "stdout"}},
            "native_input_observations": [{key: value for key, value in row.items() if key != "pid"} for row in observed],
            "native_workers_distinct": True,
            "native_timeout_is_not_fleet_deadline": {key: value for key, value in native_timeout.items() if key not in {"stderr", "stdout"}},
            "native_single_worker_timeout": {key: value for key, value in native_cancel.items() if key not in {"stderr", "stdout"}},
            "native_timed_out_child_gone": cancel_child_gone,
            "native_timeout_stderr_present": bool(native_cancel["stderr"].strip()),
            "prototype_only": {"launch_codes_including_retry": [item["code"] for item in prototype],
                               "restart_after_invocation_exhaustion": restarted,
                               "restart_after_consumed_time_exhaustion": time_stopped,
                               "new_launches_after_exhaustion": 0,
                               "clock_assumption": "Only measured execution charged; offline pause policy not decided"},
            "accepted_baseline_unchanged": True,
            "audio_spool_absent": True,
        }
        (ROOT / "results.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    if Path(sys.argv[0]).name == "goose":
        fake_provider()
    elif "--resume-probe" in sys.argv:
        print(json.dumps(resume_probe(os.environ["SPIKE_BINARY"], os.environ,
                                      Path(os.environ["SPIKE_CWD"]),
                                      Path(os.environ["SPIKE_LEDGER"]), "discovery.md")))
    else:
        run()
