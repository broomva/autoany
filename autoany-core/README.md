# autoany_core

EGRI microkernel — **Evaluator-Governed Recursive Improvement** runtime.

A trait-based Rust library for building bounded, evaluator-governed optimization loops over executable artifacts.

## Quick Start

```rust
use autoany_core::*;
use autoany_core::budget::BudgetController;
use autoany_core::evaluator::Evaluator;
use autoany_core::executor::Executor;
use autoany_core::ledger::Ledger;
use autoany_core::loop_engine::EgriLoop;
use autoany_core::proposer::Proposer;
use autoany_core::selector::DefaultSelector;
use autoany_core::spec::PromotionPolicy;

// Implement the four core traits for your domain:
// - Executor: run candidates
// - Evaluator: score outcomes
// - Proposer: generate mutations
// - Selector: promote/discard decisions (DefaultSelector provided)

// Then wire them into the loop:
let mut egri = EgriLoop::new(proposer, executor, evaluator, selector, budget, ledger);
egri.baseline(initial_artifact)?;
let summary = egri.run()?;
```

## Core Traits

| Trait | Role |
|-------|------|
| `Executor` | Run candidate artifacts inside the harness |
| `Evaluator` | Score execution results, check constraints |
| `Proposer` | Generate mutations from current state + history |
| `Selector` | Decide: promote, discard, branch, or escalate |

## Included Components

- **`EgriLoop`** — Full loop orchestrator with lifecycle management
- **`BudgetController`** — Trial and time limits (fails closed)
- **`PromotionController`** — State management with rollback
- **`Ledger`** — Append-only trial records (in-memory or JSONL)
- **`DefaultSelector`** — KeepIfImproves, Threshold, HumanGate, Pareto policies
- **`ProblemSpec`** — Deserializable problem definition

## Core Law

> Do not grant an agent more mutation freedom than your evaluator can reliably judge.

## License

MIT
