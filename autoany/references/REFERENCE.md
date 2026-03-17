# EGRI Formal Model

## Problem Instance

A problem instance is a tuple:

```
Π = (X, M, H, E, J, C, B, P, L)
```

| Symbol | Name | Definition |
|--------|------|------------|
| **X** | Artifact state space | Set of valid artifact states |
| **M** | Mutation operators | Proposal functions over X |
| **H** | Immutable harness | Fixed execution shell specification |
| **E** | Execution backend | Where candidates run (local, container, simulator, lab) |
| **J** | Evaluator | Returns scalar or vector score |
| **C** | Hard constraints | Safety predicates that must hold |
| **B** | Budget policy | Time, money, tokens, or trial count |
| **P** | Promotion policy | Decision rule: keep, discard, branch, escalate |
| **L** | Ledger | Append-only record of trajectories, scores, lineage, failures |

## Canonical Loop

Given current state `x_t`:

1. Propose candidate set `Q_t = {m_i(x_t)}` for `m_i ∈ M`
2. Execute each `q ∈ Q_t` inside `(H, E, B)`
3. Observe outcomes `o(q)`
4. Compute score `s(q) = J(o(q))`
5. Reject any `q` that violates `C`
6. Choose next state `x_{t+1} = P(x_t, Q_t, s, L)`
7. Append all outcomes to `L`

This is abstract enough to cover: hill climbing, Bayesian optimization, beam search,
evolutionary search, bandits, PBT, planner-executor loops, and multi-agent portfolio search.

## Core Laws

### Law 1: Evaluator Supremacy

An optimization loop is only as safe and useful as the evaluator that governs it.

### Law 2: Mutation-Evaluation Proportionality

Do not grant an agent more mutation freedom than your evaluator can reliably judge.

### Law 3: Immutability of the Evaluator

The evaluator and the mutable artifact must never be changed in the same trial.
If both need to change, that is a new problem instance.

### Law 4: Budget Closure

The loop must fail closed when budget is exhausted. No "one more try" exceptions.

### Law 5: Rollback Guarantee

Every promoted state must be recoverable. The system must be able to return to the
last known-good state at any point.

## Minimal Formal Conditions

EGRI works best when four conditions hold:

1. **Mutable artifact exists** — something concrete can be changed
2. **Executable harness exists** — candidates can be run repeatably
3. **Trusted evaluator exists** — outcomes can be scored reliably enough to compare
4. **Bounded damage** — bad candidates can be rejected, rolled back, sandboxed

## Failure Modes

| Failure | Symptom | Remedy |
|---------|---------|--------|
| Evaluator too noisy | Promoted states oscillate | Increase eval samples, use paired comparisons |
| Evaluator gameable | Score improves but real quality degrades | Add holdout set, adversarial checks |
| Mutation surface too large | Search is diffuse, no signal | Shrink surface, decompose into sub-problems |
| Budget too tight | Loop halts before finding signal | Reduce mutation cost or expand budget |
| No rollback | Failed promotion corrupts state | Add versioning before any mutation |
| Reward hacking | Agent optimizes proxy, not intent | Add constraint predicates, human review gates |

## Abstraction Levels

The primitive supports three progressively stronger versions:

- **Version A — Optimize existing artifact:** Tune a training loop, retrieval config, compiler pass
- **Version B — Generate then optimize:** Synthesize baseline + iterate
- **Version C — Synthesize artifact class:** Invent new topology, replace rules engine with learned controller

The hard part is not C. The hard part is making the evaluator strong enough that C
does not devolve into garbage.
