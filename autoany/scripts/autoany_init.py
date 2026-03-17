#!/usr/bin/env python3
"""
Autoany scaffold initializer.

Bootstraps a minimal EGRI project directory for a new problem instance.

Usage:
    python3 autoany_init.py <project-name> --domain <code|rag|workflow|etl|ui|generic> --path <output-dir>
    python3 autoany_init.py --help
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone

DOMAIN_PRESETS = {
    "code": {
        "objective": {"metric": "minimize test_failure_rate", "type": "scalar", "direction": "minimize"},
        "mutable_artifact": {"path": "src/", "type": "code", "description": "Source code under optimization"},
        "immutable_artifact": {"path": "tests/", "reason": "Test suite — evaluator must not change during trials"},
        "evaluator_script": "eval/run_eval.sh",
        "execution_backend": "local",
        "timeout_s": 300,
        "proposer": "llm",
    },
    "rag": {
        "objective": {"metric": "maximize answer_accuracy", "type": "scalar", "direction": "maximize"},
        "mutable_artifact": {"path": "config/retrieval.yaml", "type": "config", "description": "Retrieval config, chunking, prompts, reranker"},
        "immutable_artifact": {"path": "eval/golden_set.jsonl", "reason": "Golden eval set — ground truth for scoring"},
        "evaluator_script": "eval/judge.py",
        "execution_backend": "api",
        "timeout_s": 120,
        "proposer": "llm",
    },
    "workflow": {
        "objective": {"metric": "maximize completion_rate", "type": "scalar", "direction": "maximize"},
        "mutable_artifact": {"path": "workflow/policy.yaml", "type": "graph", "description": "Decision graph, routing, escalation thresholds"},
        "immutable_artifact": {"path": "eval/replay_logs/", "reason": "Replay corpus — fixed test cases"},
        "evaluator_script": "eval/replay_eval.py",
        "execution_backend": "local",
        "timeout_s": 60,
        "proposer": "llm",
    },
    "etl": {
        "objective": {"metric": "maximize row_accuracy", "type": "scalar", "direction": "maximize"},
        "mutable_artifact": {"path": "transforms/", "type": "code", "description": "Transform logic, schema mappings, dedup rules"},
        "immutable_artifact": {"path": "fixtures/", "reason": "Source and expected output snapshots"},
        "evaluator_script": "eval/data_quality.py",
        "execution_backend": "container",
        "timeout_s": 180,
        "proposer": "llm",
    },
    "ui": {
        "objective": {"metric": "maximize task_completion_rate", "type": "scalar", "direction": "maximize"},
        "mutable_artifact": {"path": "src/components/", "type": "code", "description": "UI components, copy, layout, flow"},
        "immutable_artifact": {"path": "eval/scenarios/", "reason": "User simulation scenarios"},
        "evaluator_script": "eval/ui_eval.py",
        "execution_backend": "local",
        "timeout_s": 60,
        "proposer": "llm",
    },
    "generic": {
        "objective": {"metric": "", "type": "scalar", "direction": "minimize"},
        "mutable_artifact": {"path": "", "type": "code", "description": ""},
        "immutable_artifact": {"path": "", "reason": ""},
        "evaluator_script": "eval/run_eval.sh",
        "execution_backend": "local",
        "timeout_s": 300,
        "proposer": "llm",
    },
}


def generate_problem_spec(name: str, domain: str) -> str:
    preset = DOMAIN_PRESETS[domain]
    obj = preset["objective"]
    mut = preset["mutable_artifact"]
    imm = preset["immutable_artifact"]
    now = datetime.now(timezone.utc).isoformat()

    return f"""# Autoany Problem Spec — {name}
# Domain preset: {domain}
# Generated: {now}

name: "{name}"

objective:
  metric: "{obj['metric']}"
  type: {obj['type']}
  direction: {obj['direction']}
  baseline: null

constraints:
  - "runtime_s <= {preset['timeout_s']}"

artifacts:
  mutable:
    - path: "{mut['path']}"
      type: {mut['type']}
      description: "{mut['description']}"
  immutable:
    - path: "{imm['path']}"
      reason: "{imm['reason']}"

evaluator:
  script: "{preset['evaluator_script']}"
  inputs: []
  outputs: {{}}
  trusted: false
  baseline_score: null

execution:
  backend: {preset['execution_backend']}
  command: ""
  timeout_s: {preset['timeout_s']}
  sandbox: true

budget:
  max_trials: 50
  time_per_trial_s: {preset['timeout_s']}
  total_time_s: null
  token_budget: null
  cost_budget: null

promotion:
  policy: keep_if_improves
  threshold: null
  require_constraint_check: true

autonomy:
  mode: sandbox
  escalation_triggers:
    - "evaluator_score_degrades_3_consecutive_trials"
    - "constraint_violation_detected"
    - "budget_75_percent_exhausted_without_improvement"

search:
  proposer: {preset['proposer']}
  strategy_notes: ""

ledger:
  format: jsonl
  path: "./ledger.jsonl"

domain:
  preset: {domain}
  notes: ""

meta:
  created_by: "autoany_init"
  created_at: "{now}"
  version: "0.1.0"
  parent_spec: null
"""


def generate_eval_script() -> str:
    return """#!/usr/bin/env bash
# Autoany evaluator stub
# Replace this with your actual evaluation logic.
# Must output JSON to stdout with at least a "score" field.
set -euo pipefail

echo '{"score": 0.0, "constraints_passed": true, "constraint_violations": []}'
"""


def generate_readme(name: str) -> str:
    return f"""# {name}

An Autoany (EGRI) project — evaluator-governed recursive improvement.

## Structure

```
{name}/
├── problem-spec.yaml    # Problem definition
├── eval/                # Evaluator (immutable during trials)
│   └── run_eval.sh      # Evaluation script
├── artifacts/           # Mutable artifacts
├── ledger.jsonl         # Trial ledger (append-only)
└── harness/             # Execution harness
```

## Quick Start

1. Edit `problem-spec.yaml` to define your problem
2. Implement the evaluator in `eval/`
3. Place your baseline artifact in `artifacts/`
4. Run the evaluator on the baseline to get `baseline_score`
5. Begin the EGRI loop

## Core Law

> Do not grant an agent more mutation freedom than your evaluator can reliably judge.
"""


def main():
    parser = argparse.ArgumentParser(
        description="Bootstrap an Autoany (EGRI) project scaffold."
    )
    parser.add_argument("name", help="Project name")
    parser.add_argument(
        "--domain",
        choices=list(DOMAIN_PRESETS.keys()),
        default="generic",
        help="Domain preset (default: generic)",
    )
    parser.add_argument(
        "--path",
        default=".",
        help="Output directory (default: current directory)",
    )
    args = parser.parse_args()

    project_dir = os.path.join(args.path, args.name)

    if os.path.exists(project_dir):
        print(f"Error: {project_dir} already exists.", file=sys.stderr)
        sys.exit(1)

    # Create directories
    dirs = [
        project_dir,
        os.path.join(project_dir, "eval"),
        os.path.join(project_dir, "artifacts"),
        os.path.join(project_dir, "harness"),
    ]
    for d in dirs:
        os.makedirs(d, exist_ok=True)

    # Write problem-spec.yaml
    spec_path = os.path.join(project_dir, "problem-spec.yaml")
    with open(spec_path, "w") as f:
        f.write(generate_problem_spec(args.name, args.domain))

    # Write evaluator stub
    eval_path = os.path.join(project_dir, "eval", "run_eval.sh")
    with open(eval_path, "w") as f:
        f.write(generate_eval_script())
    os.chmod(eval_path, 0o755)

    # Write ledger schema
    schema_src = os.path.join(os.path.dirname(os.path.dirname(__file__)), "assets", "ledger.schema.json")
    schema_dst = os.path.join(project_dir, "ledger.schema.json")
    if os.path.exists(schema_src):
        import shutil
        shutil.copy2(schema_src, schema_dst)
    else:
        # Inline minimal schema
        with open(schema_dst, "w") as f:
            json.dump({"$schema": "https://json-schema.org/draft/2020-12/schema", "title": "Autoany Trial Ledger Entry", "type": "object"}, f, indent=2)

    # Write README
    readme_path = os.path.join(project_dir, "README.md")
    with open(readme_path, "w") as f:
        f.write(generate_readme(args.name))

    # Write empty ledger
    ledger_path = os.path.join(project_dir, "ledger.jsonl")
    with open(ledger_path, "w") as f:
        pass  # Empty file

    # Output structured result
    result = {
        "status": "success",
        "project": args.name,
        "domain": args.domain,
        "path": os.path.abspath(project_dir),
        "files_created": [
            spec_path,
            eval_path,
            schema_dst,
            readme_path,
            ledger_path,
        ],
        "next_steps": [
            "Edit problem-spec.yaml to define your problem",
            "Implement the evaluator in eval/",
            "Place baseline artifact in artifacts/",
            "Run evaluator on baseline to get baseline_score",
            "Begin the EGRI loop",
        ],
    }
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
