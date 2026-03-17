# Nested Loops and Meta-Optimization

EGRI supports recursive application at multiple levels.

## Loop Levels

### Level 0: Artifact Loop

Optimize the artifact itself. This is the base autoresearch behavior.

```
mutate artifact → execute → evaluate → promote/discard → record → repeat
```

**LLM role:** Hypothesis generation, diagnosis, tradeoff judgment.

### Level 1: Policy Loop

Optimize *how* mutations are proposed. The mutation strategy itself becomes the mutable artifact.

```
mutate search_policy → run N artifact trials → evaluate policy effectiveness → promote/discard policy → record → repeat
```

**Mutable:** Search heuristics, decomposition policies, branching strategy, stopping criteria.
**Evaluator:** Rate of improvement per trial, cost per improvement, diversity of solutions found.
**LLM role:** Theory formation from search history, identifying exhausted branches.

**Example abstractions to induce:**
- "Depth increases hurt under this time budget"
- "Attention pattern changes are high-risk/high-reward"
- "Optimizer changes only help when batch regime changes too"
- "This branch of search is exhausted"

### Level 2: Portfolio Loop

Allocate budget across multiple Level 0/1 loops running in parallel.

```
observe all active loops → reallocate budget → spawn/prune loops → record → repeat
```

**Mutable:** Budget allocation, loop priorities, spawn/prune decisions.
**Evaluator:** Portfolio-level progress rate, resource efficiency, coverage.
**LLM role:** Strategic resource allocation, identifying complementary vs redundant loops.

### Level 3: Org Loop

Optimize the organization code: who explores what, when to branch, when to simplify,
when to exploit vs explore.

```
observe portfolio performance → modify coordination policy → evaluate org effectiveness → record → repeat
```

**Mutable:** Coordination rules, escalation thresholds, team composition, communication protocols.
**Evaluator:** Overall research velocity, discovery rate, resource utilization.
**LLM role:** Meta-optimization — improving the rules that govern the improvement process.

This is what Karpathy means by iterating on `program.md` and building an "autonomous research org."

## When to Use Each Level

| Level | Prerequisite | Trigger |
|-------|-------------|---------|
| 0 | Evaluator exists, mutable artifact defined | Default starting point |
| 1 | Level 0 has run enough trials to show patterns | Improvement rate plateaus |
| 2 | Multiple valid subproblems or approaches exist | Single loop is insufficient |
| 3 | Portfolio is running but coordination is suboptimal | Budget is wasted on redundant work |

## Strategy Distillation

After sufficient Level 0 trials, distill the ledger into explicit learned strategy:

1. **Cluster trials** by mutation type and outcome
2. **Identify winning patterns** — what kinds of mutations reliably help?
3. **Identify dead ends** — what kinds of mutations consistently fail?
4. **Form hypotheses** — why do the patterns hold?
5. **Update search policy** — bias future proposals toward winning patterns
6. **Record distilled strategy** in the ledger as a special entry

This is not just memory. It is theory formation from search history.

## The Three-Layer Architecture

For production systems, separate concerns:

```
autoany-skill     → compiler: interprets user intent, produces problem-spec
autoany-core      → microkernel: loop orchestration, ledger, executor abstraction
problem-instance  → generated: actual evaluator, harness, artifact space, operators
```

**Skill = compiler.** Decides what kind of system to build.
**Core = microkernel.** Provides the reusable loop substrate.
**Instance = generated runtime.** Contains the domain-specific implementation.

## Where LLM Reasoning Is Most Valuable

**High-entropy decisions (use LLM):**
- Choosing representations
- Identifying causal hypotheses
- Designing evaluators
- Translating vague goals into formal specs
- Clustering failures into categories
- Deciding when to branch vs exploit
- Discovering reusable modules across domains

**Low-entropy decisions (use scripts):**
- Brute-force parameter sweeps
- Rerunning the same command
- Parsing fixed-format metrics
- Maintaining append-only logs
- Simple keep/discard comparisons once evaluator is trusted
