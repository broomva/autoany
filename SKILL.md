---
name: autoany
description: >
  Evaluator-Governed Recursive Improvement (EGRI) framework for turning ambiguous goals
  into safe, measurable, rollback-capable recursive improvement systems. Use when the user
  wants to: (1) build a self-improving system for any domain (ML, RAG, workflows, ETL, UI,
  compiler tuning, etc.), (2) formalize a vague optimization goal into a bounded loop with
  evaluator, harness, and promotion policy, (3) create an autoresearch-style system beyond
  ML training, (4) design a mutable-artifact + immutable-evaluator architecture, (5) scaffold
  a problem-spec for recursive improvement, (6) turn "make X better" into a safe, auditable
  optimization process. Triggers on: "self-improving", "autoresearch", "autoany", "EGRI",
  "recursive improvement", "optimization loop", "evaluator-governed", "harness + evaluator",
  "mutable artifact", "problem compiler", "benchmark loop", "mutation surface".
---

# Autoany — EGRI Skill

Turn ambiguous user goals into safe, measurable, rollback-capable recursive improvement systems.

## Core Principle

> Do not grant an agent more mutation freedom than your evaluator can reliably judge.

## Operating Procedure

### Phase 1: Problem Compilation

Extract from the user's goal:

1. **Objective** — metric(s) to optimize (scalar or vector)
2. **Hard constraints** — what must never be violated (memory, latency, cost, compliance)
3. **Mutable artifacts** — what the loop may change (the `train.py` equivalent)
4. **Immutable artifacts** — what stays fixed (the `prepare.py` equivalent)
5. **Evaluator** — how to score candidates reliably enough to compare them
6. **Execution backend** — where candidates run (local, container, simulator, API)
7. **Budget** — time, tokens, money, or trial count per candidate
8. **Promotion policy** — keep-if-improves, Pareto, threshold, human-gate
9. **Autonomy mode** — suggestion, sandbox, auto-promote, or portfolio

Produce a `problem-spec.yaml`. See `assets/problem-spec.template.yaml` for the schema and `references/PROBLEM-SPEC.md` for field-by-field semantics.

### Phase 2: Evaluator-First Design

Before touching the mutable artifact:

1. Define the evaluator — what it measures, how it scores, what thresholds matter
2. Build or identify the benchmark / replay set / test suite
3. Establish baseline score by running the current artifact through the evaluator
4. Confirm the evaluator is trusted — if not, fix it before proceeding

**Law:** The evaluator must exist and produce a baseline score before any mutation begins.

### Phase 3: Harness Construction

Build the immutable execution shell:

1. **Execution script** — runs the candidate artifact deterministically
2. **Scoring script** — invokes the evaluator, outputs structured results
3. **Constraint checker** — rejects candidates violating hard constraints
4. **Rollback mechanism** — restores previous state on failure or rejection
5. **Telemetry** — logs trial metadata (duration, resource use, errors)
6. **Ledger** — append-only record of all trials (see `assets/ledger.schema.json`)

### Phase 4: Mutation Surface Definition

1. Identify artifact type (code, config, prompt, graph, parameters)
2. Define mutation operators (edit, replace, compose, parameterize, restructure)
3. Start with the **smallest viable mutation surface** — expand only after baseline is stable
4. Mark everything else as immutable

### Phase 5: Loop Execution

```
x_t = current best artifact state
while budget remains:
    m = propose_mutation(x_t, ledger, strategy)
    x' = apply(m, x_t)
    result = execute(x', harness)
    score = evaluate(result)
    if violates_constraints(result): discard(x'), log("rejected")
    elif promotion_policy(score, x_t_score): promote(x'), x_t = x'
    else: discard(x'), log("no improvement")
    record(ledger, trial_metadata)
```

### Phase 6: Ledger Review and Strategy Distillation

After each batch of trials:

1. Review ledger for patterns (what helped, what failed, what is exhausted)
2. Induce reusable abstractions ("depth increases hurt under this budget")
3. Update search strategy based on accumulated evidence
4. Decide: continue, branch, simplify, or escalate to human

## Autonomy Modes

| Mode | Mutate | Execute | Promote | When to use |
|------|--------|---------|---------|-------------|
| **Suggestion** | Propose only | No | No | Evaluator untrusted or high-risk domain |
| **Sandbox** | Yes | Yes | No | Evaluator exists but promotion needs human review |
| **Auto-promote** | Yes | Yes | Yes | Strong evaluator, bounded damage, clear constraints |
| **Portfolio** | Yes | Yes | Yes | Multiple loops, budget allocation across subproblems |

Default to **sandbox**. Escalate only with explicit user approval.

## Safety Rules

1. Never mutate evaluator and artifact in the same trial
2. Never promote without constraint checks passing
3. Never exceed budget — fail closed, not open
4. Always maintain rollback capability to last promoted state
5. Log every trial, including failures and rejections
6. If evaluator is suspected gamed, halt and escalate

## Domain Adaptation

Read `references/DOMAIN-MAPPINGS.md` for concrete artifact/harness/evaluator choices per domain.

## Formal Model

Read `references/REFERENCE.md` for full EGRI formal model: Pi = (X, M, H, E, J, C, B, P, L).

## Nested Loops and Meta-Optimization

Read `references/META-LOOP.md` for Level 1-3 loops (policy, portfolio, org).

## Scaffold Initialization

```bash
python3 scripts/autoany_init.py <project-name> --domain <code|rag|workflow|etl|ui|generic> --path <output-dir>
```
