#!/usr/bin/env python3
"""
EGRI loop harness for the ticket classifier.

Manages: versioning, evaluation, promotion, ledger, rollback.
The mutation step is manual (or agent-driven) — this harness handles everything else.

Usage:
    python3 harness/run_loop.py baseline   # Record baseline
    python3 harness/run_loop.py evaluate   # Run trial
    python3 harness/run_loop.py promote    # Promote current
    python3 harness/run_loop.py rollback       # Rollback to last promoted artifact
    python3 harness/run_loop.py status         # Show current loop status
    python3 harness/run_loop.py ledger         # Print ledger summary
"""

import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone

PROJECT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ARTIFACT = os.path.join(PROJECT_DIR, "artifacts", "classifier.py")
BEST_ARTIFACT = os.path.join(PROJECT_DIR, "harness", "best_classifier.py")
EVALUATOR = os.path.join(PROJECT_DIR, "eval", "evaluate.py")
LEDGER = os.path.join(PROJECT_DIR, "ledger.jsonl")
STATE_FILE = os.path.join(PROJECT_DIR, "harness", "state.json")


def load_state():
    if os.path.exists(STATE_FILE):
        with open(STATE_FILE) as f:
            return json.load(f)
    return {"trial_count": 0, "best_score": None, "budget": 10, "baseline_score": None}


def save_state(state):
    with open(STATE_FILE, "w") as f:
        json.dump(state, f, indent=2)


def run_evaluator():
    result = subprocess.run(
        [sys.executable, EVALUATOR, ARTIFACT],
        capture_output=True,
        text=True,
        timeout=10,
    )
    if result.returncode != 0:
        return {"score": 0.0, "error": result.stderr, "constraints_passed": False}
    return json.loads(result.stdout)


def append_ledger(entry):
    with open(LEDGER, "a") as f:
        f.write(json.dumps(entry) + "\n")


def cmd_baseline():
    state = load_state()
    result = run_evaluator()
    state["baseline_score"] = result["score"]
    state["best_score"] = result["score"]
    save_state(state)

    # Save baseline as the first "best"
    shutil.copy2(ARTIFACT, BEST_ARTIFACT)

    entry = {
        "trial_id": "baseline",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "parent_state": None,
        "mutation": {"operator": "none", "description": "Initial baseline measurement"},
        "outcome": result,
        "decision": {"action": "promoted", "reason": "baseline establishment"},
    }
    append_ledger(entry)

    print(
        f"Baseline score: {result['score']:.2%} ({result['correct']}/{result['total']})"
    )
    print(f"Duration: {result['duration_s']}s")
    if result.get("errors"):
        print(f"Misclassified: {len(result['errors'])} tickets")
        for e in result["errors"][:5]:
            print(
                f"  '{e['text'][:50]}...' expected={e['expected']} got={e['predicted']}"
            )


def cmd_evaluate():
    state = load_state()

    if state["best_score"] is None:
        print("Error: run 'baseline' first")
        sys.exit(1)

    if state["trial_count"] >= state["budget"]:
        print(f"Budget exhausted ({state['budget']} trials). Loop complete.")
        sys.exit(0)

    result = run_evaluator()
    state["trial_count"] += 1
    score = result["score"]
    best = state["best_score"]

    # Promotion decision
    if not result.get("constraints_passed", True):
        action = "rejected"
        reason = f"constraint violation: {result.get('constraint_violations', [])}"
    elif score > best:
        action = "promoted"
        reason = f"improved {best:.2%} -> {score:.2%}"
        state["best_score"] = score
        shutil.copy2(ARTIFACT, BEST_ARTIFACT)
    else:
        action = "discarded"
        reason = f"no improvement ({score:.2%} <= {best:.2%})"

    save_state(state)

    entry = {
        "trial_id": f"trial-{state['trial_count']:03d}",
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "parent_state": f"trial-{state['trial_count'] - 1:03d}"
        if state["trial_count"] > 1
        else "baseline",
        "mutation": {"operator": "manual", "description": "Agent-applied mutation"},
        "outcome": {
            "score": score,
            "correct": result["correct"],
            "total": result["total"],
            "duration_s": result["duration_s"],
            "constraints_passed": result.get("constraints_passed", True),
        },
        "decision": {"action": action, "reason": reason},
    }
    append_ledger(entry)

    icon = {"promoted": "+", "discarded": "=", "rejected": "X"}[action]
    t = state["trial_count"]
    c, n = result["correct"], result["total"]
    print(f"[{icon}] Trial {t}/{state['budget']}: {score:.2%} ({c}/{n})")
    print(f"    {action}: {reason}")

    if result.get("errors"):
        print(f"    Misclassified ({len(result['errors'])}):")
        for e in result["errors"][:5]:
            expected, got = e["expected"], e["predicted"]
            print(f"      '{e['text'][:50]}...' {expected}->{got}")
        if len(result["errors"]) > 5:
            print(f"      ... and {len(result['errors']) - 5} more")


def cmd_rollback():
    if not os.path.exists(BEST_ARTIFACT):
        print("Error: no promoted state to rollback to")
        sys.exit(1)
    shutil.copy2(BEST_ARTIFACT, ARTIFACT)
    print("Rolled back to last promoted artifact.")


def cmd_status():
    state = load_state()
    print(
        f"Baseline:  {state['baseline_score']:.2%}"
        if state["baseline_score"] is not None
        else "Baseline:  not set"
    )
    print(
        f"Best:      {state['best_score']:.2%}"
        if state["best_score"] is not None
        else "Best:      not set"
    )
    print(f"Trials:    {state['trial_count']}/{state['budget']}")
    remaining = state["budget"] - state["trial_count"]
    print(f"Remaining: {remaining}")


def cmd_ledger():
    if not os.path.exists(LEDGER):
        print("No ledger entries yet.")
        return
    with open(LEDGER) as f:
        entries = [json.loads(line) for line in f if line.strip()]
    print(f"{'Trial':<12} {'Score':>8} {'Action':<12} {'Reason'}")
    print("-" * 70)
    for e in entries:
        tid = e["trial_id"]
        score = e["outcome"].get("score", 0)
        action = e["decision"]["action"]
        reason = e["decision"].get("reason", "")
        print(f"{tid:<12} {score:>7.2%} {action:<12} {reason}")


COMMANDS = {
    "baseline": cmd_baseline,
    "evaluate": cmd_evaluate,
    "rollback": cmd_rollback,
    "status": cmd_status,
    "ledger": cmd_ledger,
}

if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in COMMANDS:
        print(f"Usage: {sys.argv[0]} <{'|'.join(COMMANDS.keys())}>")
        sys.exit(1)
    COMMANDS[sys.argv[1]]()
