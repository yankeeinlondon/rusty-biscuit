import pty
import os
import time
import subprocess

def master_read(fd):
    data = os.read(fd, 1024)
    return data

pid, fd = pty.fork()
if pid == 0:
    os.execvp("target/debug/question", ["question", "choose-one", "a", "b", "c"])
else:
    time.sleep(0.5)
    os.write(fd, b"\x1b")
    time.sleep(0.5)
    _, status = os.waitpid(pid, 0)
    print(f"Exit code: {os.WEXITSTATUS(status)}")
