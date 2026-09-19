"""Native-Windows process-tree probe mirroring Claudine's Job Object sequence.

This is OS-semantics evidence, not a run of the Claudine binary. It replays the
calls `windows_wait_loop` (claudine/cli/src/commands/wrap/exec/termination/
windows.rs) makes: spawn with CREATE_NEW_PROCESS_GROUP, CreateJobObjectW with
JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, AssignProcessToJobObject after spawn, and
TerminateJobObject on escalation. Temporary files live in the system temp dir
and are removed on exit.

    python probe_tree_windows.py            # prints JSON to stdout
"""

import ctypes
from ctypes import wintypes
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

CREATE_NEW_PROCESS_GROUP = 0x00000200
CREATE_BREAKAWAY_FROM_JOB = 0x01000000
KILL_ON_JOB_CLOSE = 0x00002000
JobObjectExtendedLimitInformation = 9

k32 = ctypes.WinDLL("kernel32", use_last_error=True) if os.name == "nt" else None


class BASIC(ctypes.Structure):
    _fields_ = [("PerProcessUserTimeLimit", ctypes.c_int64), ("PerJobUserTimeLimit", ctypes.c_int64),
                ("LimitFlags", wintypes.DWORD), ("MinimumWorkingSetSize", ctypes.c_size_t),
                ("MaximumWorkingSetSize", ctypes.c_size_t), ("ActiveProcessLimit", wintypes.DWORD),
                ("Affinity", ctypes.c_size_t), ("PriorityClass", wintypes.DWORD),
                ("SchedulingClass", wintypes.DWORD)]


class EXTENDED(ctypes.Structure):
    _fields_ = [("BasicLimitInformation", BASIC), ("IoInfo", ctypes.c_uint64 * 6),
                ("ProcessMemoryLimit", ctypes.c_size_t), ("JobMemoryLimit", ctypes.c_size_t),
                ("PeakProcessMemoryUsed", ctypes.c_size_t), ("PeakJobMemoryUsed", ctypes.c_size_t)]


def _setup():
    k32.CreateJobObjectW.restype = wintypes.HANDLE
    k32.CreateJobObjectW.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    k32.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
    k32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    k32.TerminateJobObject.argtypes = [wintypes.HANDLE, wintypes.UINT]
    k32.OpenProcess.restype = wintypes.HANDLE
    k32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    k32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    k32.CloseHandle.argtypes = [wintypes.HANDLE]
    k32.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]


def make_job():
    job = k32.CreateJobObjectW(None, None)
    info = EXTENDED()
    info.BasicLimitInformation.LimitFlags = KILL_ON_JOB_CLOSE
    ok = k32.SetInformationJobObject(job, JobObjectExtendedLimitInformation, ctypes.byref(info), ctypes.sizeof(info))
    assert ok, ctypes.get_last_error()
    return job


def alive(pid):
    handle = k32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
    if not handle:
        return False
    code = wintypes.DWORD()
    k32.GetExitCodeProcess(handle, ctypes.byref(code))
    k32.CloseHandle(handle)
    return code.value == 259  # STILL_ACTIVE


def kill(pid):
    handle = k32.OpenProcess(0x0001, False, pid)  # PROCESS_TERMINATE
    if handle:
        k32.TerminateProcess(handle, 1)
        k32.CloseHandle(handle)


CHILD = r"""
import json, subprocess, sys, time
out = {}
sleep = [sys.executable, "-c", "import time; time.sleep(60)"]
time.sleep(float(sys.argv[2]))
out["grandchild"] = subprocess.Popen(sleep).pid
try:
    out["breakaway"] = subprocess.Popen(sleep, creationflags=0x01000000).pid
except OSError as error:
    out["breakaway_error"] = str(error)
open(sys.argv[1], "w").write(json.dumps(out))
time.sleep(60)
"""


def spawn_child(workdir, name, spawn_delay):
    marker = workdir / (name + ".json")
    script = workdir / "child.py"
    script.write_text(CHILD)
    child = subprocess.Popen([sys.executable, str(script), str(marker), str(spawn_delay)],
                             creationflags=CREATE_NEW_PROCESS_GROUP)
    return child, marker


def wait_marker(marker, budget=15):
    deadline = time.monotonic() + budget
    while time.monotonic() < deadline:
        if marker.exists() and marker.read_text():
            return json.loads(marker.read_text())
        time.sleep(0.05)
    raise RuntimeError("child never reported")


def scenario(workdir, name, assign_after, spawn_delay, end):
    job = make_job()
    child, marker = spawn_child(workdir, name, spawn_delay)
    time.sleep(assign_after)
    assigned = bool(k32.AssignProcessToJobObject(job, wintypes.HANDLE(child._handle)))
    report = wait_marker(marker)
    if end == "terminate_job":
        k32.TerminateJobObject(job, 1)
    k32.CloseHandle(job)  # the OwnedJob drop; KILL_ON_JOB_CLOSE fires here
    time.sleep(1.0)
    pids = {"child": child.pid, **{k: v for k, v in report.items() if isinstance(v, int)}}
    result = {"assigned": assigned, "survivors": {k: alive(v) for k, v in pids.items()},
              "breakaway_error": report.get("breakaway_error")}
    for pid in pids.values():
        kill(pid)
    return result


def wrapper_main(marker_dir):
    # A stand-in wrapper process that owns the job and is then killed abruptly.
    _setup()
    workdir = Path(marker_dir)
    job = make_job()
    child, marker = spawn_child(workdir, "wrapped", 0.5)
    k32.AssignProcessToJobObject(job, wintypes.HANDLE(child._handle))
    report = wait_marker(marker)
    (workdir / "wrapper-pids.json").write_text(json.dumps({"child": child.pid, **report}))
    time.sleep(60)


def main():
    _setup()
    with tempfile.TemporaryDirectory(prefix="messenger-tree-probe-") as directory:
        workdir = Path(directory)
        out = {"host": os.environ.get("COMPUTERNAME", "?"), "python": sys.version.split()[0]}
        # Assign immediately (as Claudine does); grandchildren spawn afterward.
        out["terminate_job"] = scenario(workdir, "a", 0.0, 0.5, "terminate_job")
        out["job_handle_close_only"] = scenario(workdir, "b", 0.0, 0.5, "close")
        # Grandchild spawned BEFORE assignment: the window Claudine's comment assumes is empty.
        out["late_assignment_race"] = scenario(workdir, "c", 1.5, 0.0, "terminate_job")
        # Abrupt wrapper death: job handle closed by the kernel.
        wrapper = subprocess.Popen([sys.executable, __file__, "--wrapper", str(workdir)])
        pids_file = workdir / "wrapper-pids.json"
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline and not pids_file.exists():
            time.sleep(0.05)
        time.sleep(0.2)
        pids = {k: v for k, v in json.loads(pids_file.read_text()).items() if isinstance(v, int)}
        wrapper.kill()  # TerminateProcess: no destructors, like a crash
        wrapper.wait()
        time.sleep(1.0)
        out["wrapper_terminated"] = {"survivors": {k: alive(v) for k, v in pids.items()}}
        for pid in pids.values():
            kill(pid)
        time.sleep(0.5)
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    if "--wrapper" in sys.argv:
        wrapper_main(sys.argv[sys.argv.index("--wrapper") + 1])
    else:
        main()
