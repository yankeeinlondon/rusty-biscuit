#!/usr/bin/env python3
"""
Wire-mode scratch driver for Kimi CLI fixture capture.

Spawns `kimi --wire`, drives the JSON-RPC initialize + prompt handshake,
auto-approves any ApprovalRequest envelopes, and records every line of
stdout verbatim to the specified output file.

Usage:
    python wire-driver.py "Hi how are you? My name is Bob." wire-greet.jsonl
    python wire-driver.py "Run ls and tell me what you see." wire-tool-shell.jsonl
    python wire-driver.py "Use the explore subagent to summarize what files exist." wire-subagent.jsonl
"""

import json
import subprocess
import sys
import uuid


def send(stdin, obj: dict) -> None:
    line = json.dumps(obj, separators=(",", ":"))
    stdin.write(line + "\n")
    stdin.flush()
    print(f"  >> {line[:120]}{'...' if len(line) > 120 else ''}", file=sys.stderr)


def main() -> int:
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <prompt> <output.jsonl>", file=sys.stderr)
        return 2

    prompt_text = sys.argv[1]
    out_path = sys.argv[2]

    # Prompt IDs
    init_id = f"init-{uuid.uuid4().hex[:8]}"
    prompt_id = f"prompt-{uuid.uuid4().hex[:8]}"

    print(f"Spawning kimi --wire ...", file=sys.stderr)
    proc = subprocess.Popen(
        ["kimi", "--wire"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    with open(out_path, "w") as out_f:
        # 1. Send initialize
        send(
            proc.stdin,
            {
                "jsonrpc": "2.0",
                "id": init_id,
                "method": "initialize",
                "params": {
                    "protocol_version": "1.9",
                    "client": {"name": "claudine-wire-driver", "version": "0.1.0"},
                    "capabilities": {
                        "supports_question": False,
                        "supports_plan_mode": False,
                    },
                },
            },
        )

        # 2. Wait for initialize response
        while True:
            line = proc.stdout.readline()
            if not line:
                print("EOF before initialize response", file=sys.stderr)
                return 1
            out_f.write(line)
            out_f.flush()
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("id") == init_id:
                print(f"  << initialize response: {json.dumps(msg)[:200]}", file=sys.stderr)
                break

        # 3. Send prompt
        send(
            proc.stdin,
            {
                "jsonrpc": "2.0",
                "id": prompt_id,
                "method": "prompt",
                "params": {"user_input": prompt_text},
            },
        )

        # 4. Pump events and auto-approve
        while True:
            line = proc.stdout.readline()
            if not line:
                break
            out_f.write(line)
            out_f.flush()

            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue

            # Auto-approve ApprovalRequest
            if msg.get("method") == "request":
                req = msg.get("params", {}).get("request", {})
                if req.get("type") == "ApprovalRequest":
                    req_id = msg.get("id")
                    payload_id = req.get("payload", {}).get("id")
                    print(f"  -- Auto-approving request {req_id}", file=sys.stderr)
                    send(
                        proc.stdin,
                        {
                            "jsonrpc": "2.0",
                            "id": req_id,
                            "result": {
                                "request_id": payload_id or req_id,
                                "response": "approve",
                            },
                        },
                    )

            # Detect prompt response (end of turn)
            if msg.get("id") == prompt_id and ("result" in msg or "error" in msg):
                print(f"  << prompt response: {json.dumps(msg)[:200]}", file=sys.stderr)
                break

        # 5. Drain any trailing lines briefly
        proc.stdin.close()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait(timeout=5)

    print(f"Wrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
