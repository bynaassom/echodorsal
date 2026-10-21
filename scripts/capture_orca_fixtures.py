#!/usr/bin/env python3
"""Capture the fixed read surface; --from-cache re-sanitizes a prior local capture."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess

COMMANDS = {
    "status": ["status", "--json"],
    "agent-context": ["agent-context", "--json"],
    "worktrees": ["worktree", "ps", "--json"],
    "tasks": ["orchestration", "task-list", "--json"],
    "gates": ["orchestration", "gate-list", "--json"],
}
ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--from-cache", type=Path, help="directory with private orca-<name>.json captures")
    parser.add_argument("--run-id", help="read an existing Run explicitly; never creates or binds it")
    parser.add_argument("--cache-run-index", type=int, default=0, help="index in prior orca-run-N-tasks/gates.json captures")
    args = parser.parse_args()
    responses = {}
    for name, command in COMMANDS.items():
        if args.run_id and name in {"tasks", "gates"}:
            command = [*command[:-1], "--run", args.run_id, "--json"]
        if args.from_cache:
            cached_name = f"run-{args.cache_run_index}-{name}" if args.run_id and name in {"tasks", "gates"} else name
            raw = (args.from_cache / f"orca-{cached_name}.json").read_bytes()
        else:
            output = subprocess.run([os.environ.get("ORCA_BIN", "orca"), *command],
                                    stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                    stderr=subprocess.DEVNULL, timeout=3, check=False)
            raw = output.stdout
        responses[name] = json.loads(raw)

    status = responses["status"]["result"]
    version = status["runtime"]["appVersion"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("expected a numeric Orca version")
    output = ROOT / "fixtures" / "orca" / version
    output.mkdir(parents=True, exist_ok=True)

    sanitized = {}

    def write(name, value):
        sanitized[name] = value

    def envelope(result):
        return {"ok": True, "result": result}

    state = status["runtime"]["state"]
    write("status", envelope({"app": {"running": bool(status["app"]["running"])},
                             "runtime": {"reachable": bool(status["runtime"]["reachable"]),
                                         "state": state if state in {"ready", "starting", "stopped"} else "unknown",
                                         "appVersion": version}}))
    required = {"status", "agent-context", "worktree ps", "orchestration task-list", "orchestration gate-list"}
    commands = responses["agent-context"]["commands"]
    write("agent-context", {"schemaVersion": responses["agent-context"]["schemaVersion"],
                            "commands": [{"command": c["command"], "flags": [flag for flag in ["json", "run"] if flag in c["flags"]]}
                                         for c in commands if c["command"] in required]})

    hosts = {"local": "local"}

    def host(value):
        if value not in hosts:
            hosts[value] = f"host-{len(hosts)}"
        return hosts[value]

    worktrees = responses["worktrees"]["result"]
    allowed = {"todo", "in-progress", "in-review", "completed"}
    clean = []
    for index, item in enumerate(worktrees["worktrees"], start=1):
        state = item.get("workspaceStatus")
        clean.append({"worktreeId": f"worktree-{index}", "displayName": f"Worktree {index}",
                      "workspaceStatus": state if state in allowed else ("unknown" if state else None),
                      "hostId": host(item["hostId"])})
    write("worktrees", envelope({"worktrees": clean, "totalCount": worktrees["totalCount"],
                                 "truncated": worktrees["truncated"],
                                 "hostScope": {key: [host(h) for h in worktrees["hostScope"][key]]
                                               for key in ["hostIds", "omittedHostIds"]}}))
    for name in ["tasks", "gates"]:
        response = responses[name]
        if not args.run_id:
            if response.get("ok") is not False or response.get("error", {}).get("code") != "run_required":
                raise ValueError("expected run_required without an explicit Run")
            write(name, {"ok": False, "error": {"code": "run_required"}})
            continue
        if response.get("ok") is not True or response["result"]["runId"] != args.run_id:
            raise ValueError("explicit Run response did not match requested Run")
        result = response["result"]
        if name == "tasks":
            allowed = {"pending", "ready", "dispatched", "completed", "failed", "blocked"}
            tasks = [{"id": f"task-{i}", "task_title": f"Task {i}" if t.get("task_title") is not None else None,
                      "status": t["status"] if t["status"] in allowed else "unknown"}
                     for i, t in enumerate(result["tasks"], start=1)]
            write("tasks-run", envelope({"runId": "run-demo", "legacyReadOnly": bool(result["legacyReadOnly"]),
                                         "tasks": tasks, "count": result["count"]}))
        else:
            if result["gates"]:
                raise ValueError("nonempty gates require schema review before capture")
            write("gates-run", envelope({"runId": "run-demo", "gates": [], "count": result["count"]}))
    for name, value in sanitized.items():
        (output / f"{name}.json").write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
    print(f"Sanitized fixtures: fixtures/orca/{version}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired):
        raise SystemExit("Capture failed; inspect the CLI locally. No raw response was published.") from None
