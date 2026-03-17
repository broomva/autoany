# Domain Mappings

Concrete EGRI instantiations for common domains.

## Code Optimization (ML Training)

The original autoresearch pattern.

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | `train.py` — model architecture, optimizer, hyperparameters, training loop |
| Immutable harness | `prepare.py` — data prep, tokenizer, dataset splits |
| Evaluator | `val_bpb` (validation bits-per-byte) |
| Constraints | VRAM <= budget, runtime <= 5 min, no external deps |
| Budget | Fixed time per trial, fixed trial count |
| Promotion | Keep if val_bpb improves without constraint violation |
| Ledger | `results.tsv` + git commit history |
| Execution | Local GPU |

## Code Optimization (General)

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Source files, compiler flags, build configs |
| Immutable harness | Test suite, benchmark suite, CI pipeline |
| Evaluator | Test pass rate, benchmark throughput, binary size, compile time |
| Constraints | All tests pass, no regressions on key benchmarks |
| Budget | N trials, M minutes per trial |
| Promotion | Keep if primary metric improves and all tests pass |
| Ledger | JSONL with trial ID, diff hash, scores, duration |
| Execution | Local or container |

## RAG Pipeline

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Retrieval config, chunking strategy, prompts, reranker settings, embedding model choice |
| Immutable harness | Document corpus, golden eval set, judge config |
| Evaluator | Answer accuracy (judge-scored), retrieval recall@k, latency, cost per query |
| Constraints | Latency < threshold, cost < budget, no hallucinated sources |
| Budget | N eval runs, token budget |
| Promotion | Keep if accuracy improves without latency/cost regression |
| Ledger | JSONL with query ID, retrieved docs, answer, judge score |
| Execution | API calls or local inference |

## Workflow / Operations

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Decision graph, routing policy, prompts, retry logic, escalation thresholds |
| Immutable harness | Replay log corpus, simulation environment, sandbox |
| Evaluator | Completion rate, error rate, latency, compliance score, cost |
| Constraints | No compliance violations, no data leakage, rollback on failure |
| Budget | N replayed cases, wall-clock time |
| Promotion | Keep if completion rate improves without compliance regression |
| Ledger | JSONL with case ID, decision path, outcome, scores |
| Execution | Replay executor or sandboxed live execution |

## ETL Pipeline

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Transform logic, schema mappings, dedup rules, validation rules |
| Immutable harness | Source data snapshot, expected output snapshot, data quality checks |
| Evaluator | Row accuracy, schema conformance, throughput, error rate |
| Constraints | Zero data loss, schema must match target, idempotent |
| Budget | N pipeline runs on test data |
| Promotion | Keep if accuracy improves and zero data loss maintained |
| Ledger | JSONL with run ID, row counts, error counts, duration |
| Execution | Local or container with test data |

## UI / Product Optimization

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Copy, layout, flow structure, component config, recommendation logic |
| Immutable harness | A/B testing platform, user simulator, screenshot comparison |
| Evaluator | Conversion rate, task completion time, error rate, accessibility score |
| Constraints | WCAG compliance, no broken flows, performance budget |
| Budget | N simulated sessions or A/B test duration |
| Promotion | Keep if primary metric improves; human gate recommended |
| Ledger | JSONL with variant ID, session data, scores |
| Execution | Browser automation or user simulator |

**Note:** UI optimization is noisier than other domains because humans are in the loop
and rewards are delayed. Default to **suggestion** or **sandbox** autonomy mode.

## Compiler Optimization

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Pass ordering, inlining heuristics, scheduling, codegen flags |
| Immutable harness | Compiler + benchmark suite (SPEC, Embench, custom) |
| Evaluator | Runtime, code size, compile time, energy consumption |
| Constraints | All benchmarks must compile and pass correctness checks |
| Budget | N compilation + benchmark cycles |
| Promotion | Pareto over runtime and code size |
| Ledger | JSONL with pass config hash, benchmark scores, compile time |
| Execution | Local or CI |

## Prompt Engineering

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | System prompt, few-shot examples, output format instructions |
| Immutable harness | Eval dataset, judge prompt/model, scoring rubric |
| Evaluator | Judge accuracy, format compliance, latency, token cost |
| Constraints | Token budget per call, no prohibited content patterns |
| Budget | N eval runs, token/cost budget |
| Promotion | Keep if judge score improves without cost regression |
| Ledger | JSONL with prompt version, eval scores, token counts |
| Execution | API calls |
