# Autoany

**Evaluator-Governed Recursive Improvement (EGRI)**

Turn ambiguous goals into safe, measurable, rollback-capable recursive improvement systems.

## What is EGRI?

EGRI formalizes the pattern behind [autoresearch](https://github.com/karpathy/autoresearch) as a reusable systems primitive:

```
specify mutable surface → freeze harness → propose mutation → execute under budget
→ score with trusted evaluator → promote / discard / branch → record lineage → repeat
```

The core insight: autoresearch is not "AI doing ML research." It is a **bounded closed-loop optimizer over executable artifacts**. That pattern generalizes to any domain where you have:

1. A **mutable artifact** (code, config, prompt, workflow, parameters)
2. An **executable harness** (run candidates repeatably)
3. A **trusted evaluator** (score outcomes reliably)
4. **Bounded damage** (reject, rollback, sandbox bad candidates)

## Formal Model

A problem instance is a tuple:

```
Π = (X, M, H, E, J, C, B, P, L)
```

| Symbol | Name |
|--------|------|
| X | Artifact state space |
| M | Mutation operators |
| H | Immutable harness |
| E | Execution backend |
| J | Evaluator |
| C | Hard constraints |
| B | Budget policy |
| P | Promotion policy |
| L | Ledger |

## Core Law

> Do not grant an agent more mutation freedom than your evaluator can reliably judge.

## Architecture

Autoany is designed as three layers:

| Layer | Role | What it contains |
|-------|------|-----------------|
| **autoany-skill** | Compiler | Interprets user intent, produces problem-spec, decides runtime components |
| **autoany-core** | Microkernel | Loop orchestration, ledger, executor/evaluator/selector abstractions |
| **problem-instance** | Generated | Actual evaluator, harness, artifact space, operators, domain data |

Currently implemented: **autoany-skill** (the agent skill for problem compilation and scaffolding).

## Domain Mappings

EGRI applies across domains:

- **ML Training** — train.py mutations, val_bpb evaluator (the original autoresearch)
- **RAG Pipelines** — retrieval config, chunking, prompts; judge-scored accuracy
- **Workflow/Ops** — decision graphs, routing policies; replay-evaluated completion rate
- **ETL** — transform logic, schema mappings; data quality evaluator
- **Compiler** — pass ordering, codegen flags; benchmark suite evaluator
- **UI/Product** — copy, layout, flows; A/B or simulator evaluator
- **Prompt Engineering** — system prompts, few-shot examples; judge-scored accuracy

## Agent Skill

The `autoany/` directory contains a standards-aligned agent skill ([skills.sh](https://skills.sh), [agentskills.io](https://agentskills.io)) that teaches agents to:

1. **Compile problems** — turn vague goals into formal `problem-spec.yaml`
2. **Design evaluators first** — before any mutation begins
3. **Constrain mutation surfaces** — smallest viable mutable set
4. **Build harnesses** — immutable execution shells
5. **Run bounded loops** — budget-enforced, ledger-tracked
6. **Distill strategy** — learn from search history, not just individual trials

### Install the skill

```bash
# Copy the skill directory to your Claude skills path
cp -r autoany/ ~/.claude/skills/autoany

# Or use the packaged .skill file
# (distribute autoany.skill to other agents)
```

### Scaffold a new project

```bash
python3 autoany/scripts/autoany_init.py my-project --domain rag --path ./projects
```

## Autonomy Modes

| Mode | Mutate | Execute | Promote | Use when |
|------|--------|---------|---------|----------|
| Suggestion | Propose only | No | No | Evaluator untrusted |
| Sandbox | Yes | Yes | No | Needs human review |
| Auto-promote | Yes | Yes | Yes | Strong evaluator |
| Portfolio | Yes | Yes | Yes | Multiple parallel loops |

## Nested Loops

| Level | What it optimizes |
|-------|------------------|
| 0 | The artifact itself |
| 1 | The mutation/search policy |
| 2 | Budget allocation across loops |
| 3 | The organizational rules governing everything |

## Roadmap

- [x] EGRI formal model and doctrine
- [x] Agent skill with problem compilation procedure
- [x] Domain presets (code, RAG, workflow, ETL, UI)
- [x] Scaffold initializer
- [x] Ledger schema
- [ ] `autoany-core` — reusable loop microkernel
- [ ] Executor abstraction + adapters
- [ ] Evaluator abstraction + adapters
- [ ] Selector (promotion controller)
- [ ] Strategy distiller (Level 1 meta-loop)
- [ ] Portfolio manager (Level 2)

## License

MIT
