#!/usr/bin/env python3
"""Scratch driver for kimi --wire fixture capture (protocol 1.9).

Spawns `kimi --wire`, sends initialize, then a prompt, auto-resolves
ApprovalRequests and records every stdout line verbatim to a fixture file.
"""

import argparse
import json
import os
import select
import subprocess
import sys
import time

PROTOCOL_VERSION = "1.9"


def make_initialize_request(rpc_id: str, supports_question: bool = False) -> str:
    return json.dumps({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": "initialize",
        "params": {
            "protocol_version": PROTOCOL_VERSION,
            "client": {"name": "claudine", "version": "0.0.0-fixture"},
            "capabilities": {
                "supports_question": supports_question,
                "supports_plan_mode": False,
            },
        },
    })


def make_prompt_request(rpc_id: str, user_input: str) -> str:
    return json.dumps({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": "prompt",
        "params": {"user_input": user_input},
    })


def make_cancel_request(rpc_id: str) -> str:
    return json.dumps({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": "cancel",
        "params": None,
    })


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--prompt", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--max-seconds", type=int, default=240)
    ap.add_argument("--max-idle-seconds", type=float, default=30.0)
    ap.add_argument("--cancel-after", type=float, default=0.0)
    ap.add_argument("--auto-approve", action="store_true",
                    help="Auto-respond to ApprovalRequest envelopes")
    ap.add_argument("--auto-question", action="store_true",
                    help="Auto-respond to QuestionRequest envelopes (requires supports_question)")
    ap.add_argument("--reject-tool-calls", action="store_true",
                    help="Auto-respond to ToolCallRequest with method-not-supported")
    args = ap.parse_args()

    cmd = ["kimi", "--wire"]
    print(f"[driver] spawning: {' '.join(cmd)}", file=sys.stderr)

    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        bufsize=1,
        text=True,
        env={**os.environ, "PYTHONUNBUFFERED": "1"},
    )

    out_lines = []
    rpc_counter = [0]

    def next_rpc_id(prefix: str) -> str:
        rpc_counter[0] += 1
        return f"{prefix}-{rpc_counter[0]}"

    def send(line: str):
        print(f"[driver→kimi] {line[:200]}", file=sys.stderr)
        try:
            proc.stdin.write(line + "\n")
            proc.stdin.flush()
        except Exception as exc:
            print(f"[driver] write failed: {exc}", file=sys.stderr)

    init_id = next_rpc_id("init")
    send(make_initialize_request(init_id, supports_question=args.auto_question))

    state = {
        "init_seen": False,
        "prompt_sent": False,
        "prompt_id": None,
        "cancel_sent": False,
        "turn_done": False,
    }

    start = time.time()
    last_activity = time.time()

    while True:
        if proc.poll() is not None:
            print(f"[driver] kimi exited with code {proc.returncode}", file=sys.stderr)
            break

        if time.time() - start > args.max_seconds:
            print(f"[driver] hard timeout {args.max_seconds}s", file=sys.stderr)
            break

        if state["turn_done"] and time.time() - last_activity > 1.5:
            print("[driver] turn done; closing stdin", file=sys.stderr)
            try:
                proc.stdin.close()
            except Exception:
                pass
            try:
                proc.wait(timeout=8)
            except subprocess.TimeoutExpired:
                proc.terminate()
                proc.wait(timeout=3)
            break

        if (
            args.cancel_after > 0
            and not state["cancel_sent"]
            and time.time() - start > args.cancel_after
            and state["prompt_sent"]
        ):
            print(f"[driver] sending cancel after {args.cancel_after}s", file=sys.stderr)
            send(make_cancel_request(next_rpc_id("cancel")))
            state["cancel_sent"] = True
            last_activity = time.time()

        if time.time() - last_activity > args.max_idle_seconds:
            print(f"[driver] idle timeout {args.max_idle_seconds}s", file=sys.stderr)
            break

        ready, _, _ = select.select([proc.stdout, proc.stderr], [], [], 0.5)
        for stream in ready:
            line = stream.readline()
            if not line:
                continue
            line = line.rstrip("\n\r")
            if stream is proc.stderr:
                print(f"[kimi-stderr] {line[:200]}", file=sys.stderr)
                continue

            print(f"[kimi→driver] {line[:200]}", file=sys.stderr)
            out_lines.append(line)
            last_activity = time.time()

            try:
                obj = json.loads(line)
            except json.JSONDecodeError:
                continue

            rpc_id = obj.get("id")
            method = obj.get("method")
            params = obj.get("params") or {}

            if method == "request" and rpc_id is not None:
                request = params if isinstance(params, dict) else {}
                rtype = request.get("type")
                payload = request.get("payload") or {}
                req_id = payload.get("id") or rpc_id

                if rtype == "ApprovalRequest" and args.auto_approve:
                    send(json.dumps({
                        "jsonrpc": "2.0",
                        "id": rpc_id,
                        "result": {"request_id": req_id, "response": "approve"},
                    }))
                elif rtype == "QuestionRequest" and args.auto_question:
                    answers = {}
                    for q in payload.get("questions") or []:
                        opts = q.get("options") or []
                        if opts:
                            answers[q.get("question", "")] = opts[0].get("label", "")
                    send(json.dumps({
                        "jsonrpc": "2.0",
                        "id": rpc_id,
                        "result": {"request_id": req_id, "answers": answers},
                    }))
                elif rtype == "ToolCallRequest" and args.reject_tool_calls:
                    send(json.dumps({
                        "jsonrpc": "2.0",
                        "id": rpc_id,
                        "error": {"code": -32601, "message": "external tools not supported"},
                    }))
                else:
                    send(json.dumps({
                        "jsonrpc": "2.0",
                        "id": rpc_id,
                        "error": {"code": -32601, "message": f"unsupported request type: {rtype}"},
                    }))
                continue

            if method == "event":
                event = params if isinstance(params, dict) else {}
                etype = event.get("type")
                if etype == "TurnEnd":
                    state["turn_done"] = True
                continue

            if rpc_id == init_id:
                if obj.get("error"):
                    print(f"[driver] initialize errored: {obj['error']}", file=sys.stderr)
                    state["turn_done"] = True
                else:
                    state["init_seen"] = True
                    pid = next_rpc_id("prompt")
                    state["prompt_id"] = pid
                    state["prompt_sent"] = True
                    send(make_prompt_request(pid, args.prompt))
                continue

            if rpc_id == state.get("prompt_id"):
                state["turn_done"] = True
                continue

    if proc.poll() is None:
        try:
            proc.terminate()
            proc.wait(timeout=4)
        except subprocess.TimeoutExpired:
            proc.kill()

    with open(args.out, "w") as fh:
        for line in out_lines:
            fh.write(line + "\n")

    print(f"[driver] wrote {len(out_lines)} lines to {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
