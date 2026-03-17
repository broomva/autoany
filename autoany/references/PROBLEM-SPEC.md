# Problem Spec Field Semantics

Each EGRI problem instance is defined in a `problem-spec.yaml` file.

## Required Fields

### `name`
Human-readable identifier for this problem instance.

### `objective`
- `metric`: String — what to optimize (e.g., "minimize validation_loss", "maximize throughput")
- `type`: `scalar` | `vector` — single metric or multi-objective
- `direction`: `minimize` | `maximize` — optimization direction per metric
- `baseline`: Number or null — current score before any mutations (filled after Phase 2)

### `constraints`
List of hard predicates that must hold for every candidate:
```yaml
constraints:
  - "memory_mb <= 48000"
  - "runtime_s <= 300"
  - "no_external_network_calls"
  - "output_format == 'json'"
```
Violations cause immediate rejection. Not tradeoffs — hard boundaries.

### `artifacts`

#### `mutable`
List of files, configs, prompts, or artifact identifiers the loop may modify.
Start with the **smallest viable set**. Each entry should include:
- `path`: File path or identifier
- `type`: `code` | `config` | `prompt` | `graph` | `parameters`
- `description`: What this artifact does and why it is mutable

#### `immutable`
List of artifacts that must NOT be modified during the loop:
- `path`: File path or identifier
- `reason`: Why this must stay fixed (evaluator, data prep, benchmark, etc.)

### `evaluator`
- `script`: Path to the evaluation script or command
- `inputs`: What the evaluator reads (output files, metrics, logs)
- `outputs`: Structured score format (JSON with metric fields)
- `trusted`: `true` | `false` — has the evaluator been validated against known outcomes?
- `baseline_score`: Filled after running evaluator on the initial artifact

### `execution`
- `backend`: `local` | `container` | `simulator` | `api` | `lab`
- `command`: The execution command template
- `timeout_s`: Maximum seconds per trial
- `sandbox`: `true` | `false` — whether execution is isolated

### `budget`
- `max_trials`: Integer — maximum number of mutation attempts
- `time_per_trial_s`: Integer — max seconds per trial execution
- `total_time_s`: Integer or null — overall time cap
- `token_budget`: Integer or null — LLM token cap for the loop
- `cost_budget`: Float or null — monetary cap

### `promotion`
- `policy`: `keep_if_improves` | `pareto` | `threshold` | `human_gate`
- `threshold`: Number or null — minimum improvement to promote
- `require_constraint_check`: `true` (always true, cannot be overridden)

### `autonomy`
- `mode`: `suggestion` | `sandbox` | `auto-promote` | `portfolio`
- `escalation_triggers`: List of conditions that force human review

## Optional Fields

### `search`
- `proposer`: `llm` | `random` | `bayesian` | `evolutionary` | `hybrid`
- `strategy_notes`: Free-text guidance for the mutation proposer

### `ledger`
- `format`: `jsonl` | `sqlite` | `tsv`
- `path`: Where to store the ledger
- `schema`: Path to ledger schema (default: `ledger.schema.json`)

### `domain`
- `preset`: `code` | `rag` | `workflow` | `etl` | `ui` | `generic`
- `notes`: Domain-specific context for the mutation proposer

### `meta`
- `created_by`: Who/what created this spec
- `created_at`: ISO timestamp
- `version`: Spec version (current: `0.1.0`)
- `parent_spec`: Path to parent spec if this is a refinement
