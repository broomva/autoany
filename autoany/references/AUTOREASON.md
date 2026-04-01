# Autoreason: Adversarial Debate Evaluation for Subjective Domains

## Motivation

EGRI's formal model `Π = (X, M, H, E, J, C, B, P, L)` assumes that **J (the evaluator)
produces a reliable score**. For objective domains — `val_bpb`, test pass rates, latency
benchmarks — this holds trivially. For subjective domains — writing quality, argument
strength, design coherence, strategic clarity — no scalar metric exists.

EGRI currently handles this by escalating to `HumanGate`, which breaks the autonomy loop.
The system degenerates to "propose and wait for human approval," losing the tight
propose-evaluate-promote cycle that makes EGRI powerful.

### The Sycophancy Problem

Naive LLM-as-judge evaluation fails systematically:

| Prompt framing | Bias | Effect on loop |
|----------------|------|----------------|
| "Improve this" | Sycophantic — always finds something to change | Infinite non-converging edits |
| "Find flaws in this" | Hypercritical — always finds problems | Never promotes anything |
| "Merge the best of both" | Compromising — averages rather than selects | Bland regression to mean |
| "Which is better, A or B?" | Positional — prefers the first/last shown | Score oscillates with presentation order |
| "Rate this 1-10" | Anchoring — scores cluster around 7 | No discrimination between candidates |

These biases mean the loop's output is shaped more by **how you prompt** than by **what's
actually better**. The evaluator becomes a mirror of the framing, not a judge of quality.

### The Solution: Constructed Fitness via Adversarial Debate

Autoreason constructs a synthetic evaluator for subjective domains using the same principle
that makes science work for questions where mathematics cannot provide proofs: **independent
blind peer review**.

Every role is executed by a fresh, isolated LLM agent with no shared context. No agent sees
its own prior output. Labels are randomized. The evaluator is the emergent consensus of
adversarial debate, not a single prompted judgment.

---

## The Protocol

### One Autoreason Round

```
                    ┌─────────────┐
                    │ Original    │
                    │ Task Brief  │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              │                         │
              ▼                         ▼
     ┌────────────────┐       ┌────────────────┐
     │   Version A    │       │   Version A    │
     │  (incumbent)   │       │  (incumbent)   │
     └───────┬────────┘       └───────┬────────┘
             │                        │
             ▼                        │
     ┌────────────────┐               │
     │ Phase 1: ATTACK│               │
     │ (fresh agent)  │               │
     │                │               │
     │ Sees: A only   │               │
     │ Produces:      │               │
     │   critique     │               │
     └───────┬────────┘               │
             │                        │
             ▼                        │
     ┌────────────────┐               │
     │ Phase 2: REVISE│               │
     │ (fresh agent)  │               │
     │                │               │
     │ Sees: task +   │               │
     │   A + critique │               │
     │ Produces:      │               │
     │   Version B    │               │
     └───────┬────────┘               │
             │                        │
             ├────────────────────────┤
             │                        │
             ▼                        ▼
     ┌──────────────────────────────────┐
     │ Phase 3: SYNTHESIZE              │
     │ (fresh agent)                    │
     │                                  │
     │ Sees: "Version 1" + "Version 2"  │
     │   (A and B in randomized order)  │
     │ Produces: Version AB             │
     └───────────────┬──────────────────┘
                     │
                     ▼
     ┌──────────────────────────────────┐
     │ Phase 4: JUDGE                   │
     │ (N fresh agents, independent)    │
     │                                  │
     │ Each judge sees: three versions  │
     │   with randomized labels         │
     │   ("Alpha", "Beta", "Gamma")     │
     │ Each produces: ranked preference │
     │   + justification                │
     └───────────────┬──────────────────┘
                     │
                     ▼
     ┌──────────────────────────────────┐
     │ Phase 5: DECIDE                  │
     │ (deterministic, no LLM)          │
     │                                  │
     │ Majority vote → winner           │
     │ Agreement ratio → confidence     │
     │ Winner becomes new incumbent     │
     │   (or incumbent stays if it won) │
     └─────────────────────────────────┘
```

### Phase Details

#### Phase 1: Attack (Adversarial Critic)

**Context isolation**: Fresh agent. Sees ONLY the current artifact.

**Input**:
- The artifact (Version A)
- Domain-specific critique rubric (optional)

**System prompt** (paraphrased):
> You are a rigorous critic. Your job is to find genuine weaknesses in this work:
> logical gaps, unsupported claims, missing perspectives, structural problems,
> unclear reasoning. Be specific and substantive. Do not nitpick style.

**Output**: Structured critique with categorized issues.

**Why fresh context matters**: If the critic had seen the drafting process, it would
anchor to the author's reasoning and be less likely to identify genuine blind spots.
Freshness ensures independence.

#### Phase 2: Revise (Adversarial Author)

**Context isolation**: Fresh agent. Does NOT see the critic's reasoning process.

**Input**:
- Original task description
- Version A
- The critique from Phase 1

**System prompt** (paraphrased):
> You see an artifact and substantive critique of it. Address the valid criticisms
> to produce an improved version. You may reject critiques that are wrong, but you
> must address each one explicitly. Do not water down the original — make it stronger.

**Output**: Version B — a revision that addresses the critique.

**Why this is better than "improve this"**: The reviser works from specific,
adversarially-generated feedback rather than searching for generic improvements.
The critique provides signal; the reviser provides craft.

#### Phase 3: Synthesize (Blind Merger)

**Context isolation**: Fresh agent. No knowledge of who wrote what, or the critique.

**Input**:
- Two versions labeled neutrally: "Version 1" and "Version 2"
- Labels are randomized (A might be "Version 1" or "Version 2")
- Original task description

**System prompt** (paraphrased):
> You see two versions of the same work. Combine the strongest elements of both
> into a unified version. You are not obligated to include material from both —
> if one version is clearly stronger in a section, use that.

**Output**: Version AB — a synthesis.

**Why randomized labels**: Eliminates "Version 1 must be the original" anchoring.
The synthesizer evaluates content, not position.

#### Phase 4: Judge (Blind Panel)

**Context isolation**: N fresh agents, each independent. No shared deliberation.

**Input per judge**:
- Three versions with randomized labels (e.g., "Alpha", "Beta", "Gamma")
- Label assignment is different per judge to eliminate correlated positional bias
- Original task description
- Evaluation criteria (optional rubric)

**System prompt** (paraphrased):
> Rank these three versions from strongest to weakest on [criteria].
> For each, explain what makes it stronger or weaker than the others.
> Your ranking must be definitive — no ties.

**Output per judge**: Ranked preference (1st, 2nd, 3rd) + justification.

**Panel composition options**:
- Same model, different label orderings (minimum viable)
- Different models from different providers (reduces correlated blind spots)
- Different rubric emphasis per judge (diversity of evaluation criteria)

#### Phase 5: Decide (Deterministic)

No LLM involved. Pure counting.

**Aggregation**: Borda count or simple plurality on first-place votes.

**Output**:
- `winner`: which version (A, B, or AB) won
- `confidence`: judge agreement ratio (e.g., 3/3 = 1.0, 2/3 = 0.67)
- `transcript`: full record of all phases

**Convergence signal**: If `winner == A` (the incumbent), this round produced
no improvement. The `StagnationDetector` increments its counter. If the incumbent
wins N consecutive rounds, the loop has converged — further debate is unlikely
to improve the artifact.

---

## Mapping onto EGRI Architecture

### The Key Insight

Autoreason is **not** a replacement for EGRI. It is a **specific evaluator
construction** that enables EGRI to operate in subjective domains. Everything
else — budget control, promotion, rollback, ledger, safety laws — remains
unchanged.

### Architectural Options

Three ways to integrate autoreason into autoany-core, ordered by invasiveness:

#### Option A: Adapter Pattern (Recommended)

Implement `ComparativeEvaluator` as a new trait. Build a `DebateLoop` variant
that uses it instead of `Evaluator + Selector`.

```
autoany-core/src/
  ├── evaluator.rs              # existing, unchanged
  ├── comparative_evaluator.rs  # NEW — ComparativeEvaluator trait
  ├── debate.rs                 # NEW — autoreason protocol
  ├── debate_loop.rs            # NEW — loop variant using ComparativeEvaluator
  ├── loop_engine.rs            # existing, unchanged
  └── ...
```

**Pros**: Zero changes to existing code. Clean separation. Can test debate independently.
**Cons**: Some loop logic duplicated between `EgriLoop` and `DebateLoop`.

#### Option B: Evaluator Wrapper

Make `AutoreasonEvaluator` implement the existing `Evaluator` trait by storing
the incumbent internally and converting comparison results to scalar scores.

**Pros**: No new loop variant needed. Works with existing `EgriLoop`.
**Cons**: Evaluator becomes stateful (fragile on rollback). Violates the spirit
of evaluator immutability. The incumbent must be kept in sync with
`PromotionController` state — a synchronization bug waiting to happen.

#### Option C: Generic Loop

Make `EgriLoop` generic over evaluation mode (absolute vs comparative).

**Pros**: Single loop implementation. Maximum code reuse.
**Cons**: Complex generic bounds. Every existing user of `EgriLoop` sees more
type parameters. Optimization for code sharing at the cost of readability.

**Recommendation: Option A.** The duplication is small (the loop is ~80 lines of
orchestration) and the conceptual clarity is worth it. `EgriLoop` remains the
workhorse for objective evaluation. `DebateLoop` handles subjective evaluation.
Both share `BudgetController`, `PromotionController`, `Ledger`, and all meta-
optimization modules.

### Dependency Chain

```
Layer 0: Types (no dependencies)
    types.rs additions:
      DebateConfig, DebateRole, CritiqueResult,
      DebateRound, JudgeVote, ComparisonOutcome,
      DebateTranscript, Winner

Layer 1: Traits (depends on Layer 0)
    comparative_evaluator.rs:
      trait ComparativeEvaluator {
          type Artifact;
          fn compare(&self, task: &str, incumbent: &A, candidate: &A)
              -> Result<ComparisonOutcome>;
      }

    llm_backend.rs:
      trait LlmBackend {
          fn generate(&self, system: &str, user: &str) -> Result<String>;
      }
      (abstraction over LLM API — sync trait, async via executor)

Layer 2: Protocol (depends on Layers 0 + 1)
    debate.rs:
      fn attack(backend: &dyn LlmBackend, artifact: &str, rubric: Option<&str>)
          -> Result<CritiqueResult>;
      fn revise(backend: &dyn LlmBackend, task: &str, artifact: &str, critique: &CritiqueResult)
          -> Result<String>;
      fn synthesize(backend: &dyn LlmBackend, task: &str, a: &str, b: &str)
          -> Result<String>;
      fn judge(backend: &dyn LlmBackend, task: &str, versions: &[&str], config: &DebateConfig)
          -> Result<Vec<JudgeVote>>;
      fn decide(votes: &[JudgeVote], version_map: &HashMap<Label, Winner>)
          -> ComparisonOutcome;
      fn autoreason_round(backend: &dyn LlmBackend, task: &str, incumbent: &str, config: &DebateConfig)
          -> Result<DebateRound>;

Layer 3: Evaluator (depends on Layer 2)
    autoreason_evaluator.rs:
      struct AutoreasonEvaluator<B: LlmBackend> {
          backend: B,
          config: DebateConfig,
      }
      impl<B: LlmBackend> ComparativeEvaluator for AutoreasonEvaluator<B> { ... }

Layer 4: Loop (depends on Layers 1 + 3 + existing EGRI primitives)
    debate_loop.rs:
      struct DebateLoop<A, P, X, CE> {
          proposer: P,
          executor: X,
          evaluator: CE,          // ComparativeEvaluator, not Evaluator
          budget: BudgetController,
          promotion: PromotionController<A>,
          ledger: Ledger,
          task: String,           // original task description
          convergence: ConvergenceDetector,
      }

Layer 5: Integration (depends on all above)
    spec.rs: PromotionPolicy::Comparative, DebateSpec in ProblemSpec
    strategy.rs: debate_distill() for Level 1 meta-loop
    Domain mappings: writing, argument, design, strategy
```

### Shared vs New Components

| Component | Status | Notes |
|-----------|--------|-------|
| `BudgetController` | **Shared** | Budget accounts for LLM calls per round (5+ per debate round) |
| `PromotionController` | **Shared** | Unchanged. Stores winning artifact after each round |
| `Ledger` | **Shared** | Debate transcripts stored in `evaluator_metadata` |
| `DeadEndTracker` | **Shared** | Tracks critique patterns that lead nowhere |
| `StagnationDetector` | **Reinterpreted** | Stagnation = convergence. Incumbent wins N times = done |
| `InheritedKnowledge` | **Extended** | Carries forward: which critique styles were productive |
| `strategy::distill()` | **Extended** | New: `debate_distill()` ranks critique effectiveness |
| `Evaluator` trait | **Unchanged** | Still used for objective domains |
| `Selector` trait | **Unchanged** | Not used by `DebateLoop` (debate is both eval + selection) |
| `ComparativeEvaluator` trait | **New** | Compares two artifacts |
| `DebateLoop` | **New** | Loop variant for comparative evaluation |
| `debate.rs` | **New** | Autoreason protocol implementation |
| `LlmBackend` trait | **New** | Abstraction over LLM API calls |

---

## Type Definitions

### Core Types

```rust
/// Configuration for the autoreason debate protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateConfig {
    /// Number of judges in the panel (should be odd for clean majorities).
    pub judge_count: u32,
    /// Consecutive incumbent wins required to declare convergence.
    pub convergence_threshold: u32,
    /// Whether to randomize version labels per judge (strongly recommended).
    pub label_randomization: bool,
    /// Whether to use different LLM providers for different judges.
    pub model_diversity: bool,
    /// Optional rubric for critique and judging (domain-specific).
    pub rubric: Option<String>,
    /// Maximum tokens per phase (controls cost).
    pub max_tokens_per_phase: Option<u32>,
}

/// Which version won a debate round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Winner {
    /// The incumbent (Version A) — no improvement found.
    Incumbent,
    /// The revision (Version B) — critique-driven improvement.
    Revision,
    /// The synthesis (Version AB) — combined strengths.
    Synthesis,
}

/// A single judge's vote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVote {
    /// Judge identifier (for audit trail).
    pub judge_id: String,
    /// Label-to-version mapping this judge saw (for audit trail).
    pub label_map: HashMap<String, Winner>,
    /// Ranked preference: [1st, 2nd, 3rd].
    pub ranking: Vec<Winner>,
    /// Free-text justification.
    pub justification: String,
}

/// Result of the critique phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueResult {
    /// Structured list of identified issues.
    pub issues: Vec<CritiqueIssue>,
    /// Raw critique text (for passing to reviser).
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueIssue {
    pub category: String,       // "logical_gap", "unsupported_claim", "missing_perspective", etc.
    pub severity: IssueSeverity,
    pub description: String,
    pub location: Option<String>, // where in the artifact
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IssueSeverity {
    Critical,   // Undermines the core argument
    Major,      // Significant weakness
    Minor,      // Improvement opportunity
}

/// Result of one complete autoreason round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateRound {
    pub round_number: u32,
    pub incumbent_content: String,
    pub critique: CritiqueResult,
    pub revision_content: String,
    pub synthesis_content: String,
    pub votes: Vec<JudgeVote>,
    pub winner: Winner,
    pub confidence: f64,
    pub winning_content: String,
}

/// Outcome of a comparative evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonOutcome {
    /// Which version won.
    pub winner: Winner,
    /// Judge agreement ratio: 1.0 = unanimous, 0.33 = split.
    pub confidence: f64,
    /// Full debate record (for ledger).
    pub round: DebateRound,
}
```

### Trait Definitions

```rust
/// Compares two artifacts to determine which is better.
///
/// Used when no objective scalar metric exists. The comparison IS the
/// evaluation — there is no separate scoring step.
pub trait ComparativeEvaluator {
    type Artifact;

    /// Compare incumbent and candidate, returning which is better.
    fn compare(
        &self,
        task: &str,
        incumbent: &Self::Artifact,
        candidate: &Self::Artifact,
    ) -> Result<ComparisonOutcome>;
}

/// Abstraction over LLM API calls.
///
/// Each call is stateless — no conversation history. This enforces the
/// context isolation that makes autoreason work.
pub trait LlmBackend: Send + Sync {
    /// Generate a completion from a system prompt and user message.
    /// Each call is independent — no shared conversation state.
    fn generate(&self, system: &str, user: &str) -> Result<String>;

    /// Generate with a specific model (for judge diversity).
    /// Falls back to default model if not supported.
    fn generate_with_model(
        &self,
        model: &str,
        system: &str,
        user: &str,
    ) -> Result<String> {
        // Default: ignore model, use primary
        let _ = model;
        self.generate(system, user)
    }
}
```

---

## The DebateLoop

### Structure

```rust
pub struct DebateLoop<A, P, X, CE>
where
    A: Clone + AsRef<str>,   // Artifact must be representable as text for debate
    P: Proposer<Artifact = A>,
    X: Executor<Artifact = A>,
    CE: ComparativeEvaluator<Artifact = A>,
{
    proposer: P,
    executor: X,
    evaluator: CE,
    budget: BudgetController,
    promotion: PromotionController<A>,
    ledger: Ledger,
    task: String,
    convergence_threshold: u32,
    consecutive_incumbent_wins: u32,
}
```

### Loop Execution

```rust
impl<A, P, X, CE> DebateLoop<A, P, X, CE> {
    pub fn run(&mut self) -> Result<LoopSummary> {
        self.budget.start();

        loop {
            // 1. Budget check
            if let Err(EgriError::BudgetExhausted(msg)) = self.budget.check() {
                info!(reason = %msg, "budget exhausted");
                break;
            }

            // 2. Get incumbent from promotion controller
            let incumbent = self.promotion.current()
                .ok_or(EgriError::NoBaseline)?.clone();

            let parent_state = self.promotion.current_state_id()
                .cloned().unwrap_or_else(StateId::baseline);

            // 3. Propose mutation (optional — can also use debate-generated revisions)
            let (mutation, candidate) = self.proposer.propose(&incumbent, &self.ledger)?;

            // 4. Execute candidate (render/validate — may be identity for text)
            let exec_result = self.executor.execute(&candidate)?;

            // 5. Comparative evaluation via debate
            let comparison = self.evaluator.compare(
                &self.task,
                &incumbent,
                &candidate,
            )?;

            // 6. Convert to EGRI decision
            let (action, winning_artifact) = match comparison.winner {
                Winner::Incumbent => {
                    self.consecutive_incumbent_wins += 1;
                    (Action::Discarded, None)
                }
                Winner::Revision | Winner::Synthesis => {
                    self.consecutive_incumbent_wins = 0;
                    // The winning content may differ from the candidate
                    // (synthesis or revision produced during debate)
                    let winner_artifact: A = /* reconstruct from comparison.round.winning_content */;
                    (Action::Promoted, Some(winner_artifact))
                }
            };

            // 7. Build decision record
            let decision = Decision {
                action,
                reason: format!(
                    "debate: {:?} won (confidence: {:.2}, round: {})",
                    comparison.winner, comparison.confidence, comparison.round.round_number
                ),
                new_state_id: if action == Action::Promoted {
                    Some(StateId::new())
                } else {
                    None
                },
            };

            // 8. Apply promotion
            if let Some(ref winner) = winning_artifact {
                self.promotion.apply_decision(&decision, winner.clone());
            } else {
                self.promotion.apply_decision(&decision, candidate);
            }

            // 9. Record to ledger
            let record = TrialRecord {
                trial_id: TrialId::new(self.budget.used()),
                timestamp: Utc::now(),
                parent_state,
                mutation,
                execution: Some(exec_result),
                outcome: Outcome {
                    score: Score::Scalar(comparison.confidence),
                    constraints_passed: true,
                    constraint_violations: vec![],
                    evaluator_metadata: Some(serde_json::to_value(&comparison.round)?),
                },
                decision,
                strategy_notes: None,
            };
            self.ledger.append(record)?;

            self.budget.consume();

            // 10. Convergence check
            if self.consecutive_incumbent_wins >= self.convergence_threshold {
                info!(
                    rounds = self.consecutive_incumbent_wins,
                    "converged — incumbent won {} consecutive rounds",
                    self.consecutive_incumbent_wins
                );
                break;
            }
        }

        Ok(self.summary())
    }
}
```

### Key Differences from EgriLoop

| Aspect | EgriLoop | DebateLoop |
|--------|----------|------------|
| Evaluation | Score one candidate in isolation | Compare candidate vs incumbent |
| Selection | Separate `Selector` trait | Built into comparative evaluation |
| Termination | Budget exhaustion or escalation | Budget OR convergence (incumbent wins N times) |
| Score meaning | Objective metric (lower/higher = better) | Confidence (judge agreement on winner) |
| Artifact output | May differ from input | May produce novel artifact (synthesis) |
| Cost per trial | 1 executor + 1 evaluator call | 1 executor + 5+ LLM calls |

### Important: The Debate Can Produce New Artifacts

In standard EGRI, the evaluator only scores — it never modifies the artifact.
In autoreason, the debate protocol generates two new versions (B and AB) during
evaluation. If one of those wins, the promoted artifact is something the original
Proposer never generated.

This is a feature, not a bug. The debate protocol is simultaneously:
- An **evaluator** (judges quality)
- A **refiner** (produces improved versions)

But it means `DebateLoop` must handle the case where the winning artifact differs
from the proposer's candidate. The `PromotionController` receives the actual winner,
not necessarily the original candidate.

**Law 3 compliance**: The debate protocol (evaluator) is still immutable — the same
debate config produces the same procedure every time. What changes is the artifact
content, which is the artifact, not the evaluator. The evaluator's structure (critic →
reviser → synthesizer → judge panel) is fixed.

---

## Convergence Theory

### When Does Autoreason Converge?

The loop converges when the judge panel consistently prefers the incumbent over
debate-generated alternatives. This means:

1. The critic cannot find substantive weaknesses
2. The reviser cannot produce meaningful improvements from the critique
3. The synthesizer cannot improve on the incumbent by merging with the revision
4. The judges agree the incumbent is the strongest version

This is analogous to `val_bpb` plateauing in autoresearch — further mutations
don't improve the metric. The difference is that "improvement" is measured by
consensus rather than a scalar comparison.

### Convergence vs Oscillation

**Risk**: On polarizing topics, the loop might oscillate:
- Round 1: Version B wins (takes position X)
- Round 2: Critique attacks X, Version B' wins (takes position Y)
- Round 3: Critique attacks Y, Version B'' wins (takes position X again)

**Mitigations**:
1. **Synthesis phase**: Version AB should capture both perspectives, breaking the cycle
2. **Judge panel diversity**: Multiple judges are less likely to flip-flop than one
3. **Convergence threshold > 1**: Requiring 3 consecutive incumbent wins filters out noise
4. **Ledger analysis**: If the same critique pattern appears repeatedly, `DeadEndTracker`
   flags it. Strategy distillation can detect oscillation patterns.

### Convergence Speed

Empirically (from the multi-agent debate literature):

- **Well-defined quality** (clear writing, sound arguments): 3-5 rounds
- **Ambiguous quality** (creative work, taste-dependent): 5-10 rounds
- **Polarizing topics** (strong opposing views): may not converge cleanly;
  detect via oscillation and escalate to `HumanGate`

Budget planning: assume 5-7 rounds average, 5+ LLM calls per round = 25-35 calls
per artifact optimization. At ~$0.01-0.05 per call, that's $0.25-1.75 per artifact.

---

## Integration with Existing Meta-Optimization

### Stagnation as Convergence

The existing `StagnationDetector` counts consecutive non-improvements and triggers
escalation at a threshold. In debate context, "non-improvement" means the incumbent
won. This is exactly convergence detection:

```rust
// Existing code, zero changes needed:
let status = stagnation_detector.check(&ledger);
match status {
    StagnationStatus::Ok => continue,
    StagnationStatus::Warning(n) => info!("approaching convergence ({n} incumbent wins)"),
    StagnationStatus::Stagnated(n) => {
        info!("converged after {n} consecutive incumbent wins");
        break;
    }
}
```

The only change is **interpretation**: in objective loops, stagnation means "we're
stuck, escalate." In debate loops, stagnation means "we've converged, we're done."

### Strategy Distillation for Debate

The existing `strategy::distill()` ranks mutation operators by success rate. For
debate loops, we need an additional `debate_distill()` that analyzes:

1. **Critique effectiveness**: Which critique categories (logical gaps, missing
   perspectives, unsupported claims) most often led to successful revisions?
2. **Revision patterns**: When the revision won, what structural changes did it make?
3. **Synthesis value**: How often does the synthesis win vs the revision? If synthesis
   rarely wins, consider dropping Phase 3 to save budget.
4. **Judge agreement patterns**: Do judges diverge on specific quality dimensions?
   This reveals where the rubric needs refinement.

```rust
pub struct DebateStrategyReport {
    /// Critique categories ranked by revision success rate.
    pub effective_critiques: Vec<(String, f64)>,
    /// How often each phase's output wins.
    pub phase_win_rates: HashMap<Winner, f64>,
    /// Average judge agreement per round.
    pub mean_confidence: f64,
    /// Rounds where judges diverged significantly.
    pub low_agreement_rounds: Vec<u32>,
    /// Recommended: skip synthesis if its win rate is below threshold?
    pub skip_synthesis: bool,
}
```

### Dead-End Tracking for Debate

The existing `DeadEndTracker` tracks mutation signatures that fail repeatedly.
For debate, it tracks **critique patterns** that lead nowhere:

- Signature: `critique_category:round_context`
- Example: `"missing_perspective:after_synthesis"` — if the critic keeps finding
  "missing perspectives" after synthesis rounds but it never leads to a winning
  revision, that critique direction is exhausted

### Inherited Knowledge for Debate

Cross-run learning (`InheritedKnowledge`) extends naturally:

- Carry forward which critique styles were productive
- Carry forward the final converged artifact as the new baseline
- Carry forward judge agreement patterns (which dimensions are hard to evaluate)

---

## Hybrid Evaluation: Objective + Subjective

Many real problems have BOTH objective and subjective components:

| Domain | Objective component | Subjective component |
|--------|--------------------|--------------------|
| Blog post | Word count, formatting, link validity | Argument quality, clarity, engagement |
| API design | Type safety, endpoint coverage | Ergonomics, naming, conceptual coherence |
| UI/UX | Accessibility score, load time | Visual appeal, flow intuitiveness |
| Prompt engineering | Format compliance, token count | Response quality, helpfulness |

For these, use a **HybridEvaluator** that:

1. Checks hard constraints objectively (existing `Evaluator` + constraints)
2. Runs autoreason debate for the subjective component
3. Combines both into a final `Outcome`

```rust
pub struct HybridEvaluator<OE, CE>
where
    OE: Evaluator,
    CE: ComparativeEvaluator,
{
    objective: OE,      // Checks constraints, computes objective metrics
    subjective: CE,     // Runs debate for quality assessment
}
```

The objective evaluator acts as a **gate**: if hard constraints fail, the candidate
is discarded without running the expensive debate. Only candidates that pass
objective checks proceed to subjective evaluation.

This saves significant budget — if 30% of candidates fail constraints, you save
30% of debate costs.

---

## Problem Spec Extensions

### New Fields in `problem-spec.yaml`

```yaml
# Existing fields unchanged...

# NEW: Debate configuration (required when promotion.policy == "comparative")
debate:
  judge_count: 3
  convergence_threshold: 3
  label_randomization: true
  model_diversity: false
  rubric: |
    Evaluate on: logical coherence, evidence quality, clarity of expression,
    completeness of argument, acknowledgment of counterarguments.
  max_tokens_per_phase: 2000

# NEW: Task description (required for debate — provides the original brief)
task:
  description: |
    Write a compelling analysis of X that addresses Y and considers Z.
  criteria:
    - logical_coherence
    - evidence_quality
    - clarity
    - completeness

# Updated: New promotion policy option
promotion:
  policy: comparative   # NEW — uses autoreason debate instead of scalar comparison
```

### New PromotionPolicy Variant

```rust
pub enum PromotionPolicy {
    KeepIfImproves,
    Pareto,
    Threshold,
    HumanGate,
    Comparative,      // NEW — autoreason debate evaluation
}
```

---

## New Domain Mappings

### Writing / Argumentation

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Text content (essay, blog post, report, argument) |
| Immutable harness | Original task brief, evaluation rubric, debate config |
| Evaluator | Autoreason debate protocol (critic → reviser → synthesizer → judge panel) |
| Constraints | Word count, format requirements, factual accuracy checks (objective) |
| Budget | N debate rounds, token budget, cost cap |
| Promotion | Comparative — debate winner replaces incumbent |
| Ledger | JSONL with full debate transcripts per round |
| Execution | Identity (text is the artifact — no "running" needed) |

### Strategy / Decision Analysis

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Strategic analysis, decision recommendation, risk assessment |
| Immutable harness | Problem context, stakeholder constraints, data sources |
| Evaluator | Autoreason with domain-specific rubric (feasibility, risk coverage, coherence) |
| Constraints | Must address all stakeholder concerns, must cite sources |
| Budget | N debate rounds |
| Promotion | Comparative |
| Ledger | JSONL with debate transcripts + decision lineage |
| Execution | Optional validation step (check cited sources exist, numbers add up) |

### Design / UX (Hybrid)

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | Component specs, layout descriptions, interaction flows |
| Immutable harness | Design system constraints, accessibility requirements |
| Evaluator | HybridEvaluator: objective (a11y score, performance) + subjective (debate on aesthetics, usability) |
| Constraints | WCAG compliance, performance budget, design system consistency |
| Budget | N trials for objective + M debate rounds for subjective |
| Promotion | Comparative (after objective gate) |
| Ledger | JSONL with both objective scores and debate transcripts |
| Execution | Screenshot rendering, a11y audit, Lighthouse |

### Prompt Engineering (Hybrid)

| Component | Concrete form |
|-----------|--------------|
| Mutable artifact | System prompt, few-shot examples, output format |
| Immutable harness | Eval dataset, judge rubric, golden answers |
| Evaluator | HybridEvaluator: objective (format compliance, token cost) + subjective (debate on response quality) |
| Constraints | Token budget per call, format compliance |
| Budget | N trials with eval set + M debate rounds on borderline cases |
| Promotion | Comparative (after objective gate) |
| Ledger | JSONL with both objective scores and debate transcripts |
| Execution | API calls to target model |

---

## Safety Analysis

### EGRI Laws Under Autoreason

| Law | Status | Analysis |
|-----|--------|----------|
| **Law 1: Evaluator Supremacy** | **Strengthened** | N independent judges > 1 prompted judge. Adversarial structure resists gaming. |
| **Law 2: Mutation-Evaluation Proportionality** | **Upheld** | Debate evaluator is expensive but thorough. Cost scales with mutation complexity. |
| **Law 3: Immutability of the Evaluator** | **Upheld with nuance** | The debate protocol (config, rubric, judge count) is immutable. The debate *generates* artifacts, but the evaluation *procedure* doesn't change. |
| **Law 4: Budget Closure** | **Upheld** | BudgetController counts debate rounds. Each round = 1 budget unit. |
| **Law 5: Rollback Guarantee** | **Upheld** | PromotionController stores the winning artifact. Rollback is always available. |

### New Failure Modes

| Failure | Symptom | Remedy |
|---------|---------|--------|
| Judge sycophancy to verbose version | Longer version always wins | Add explicit length-normalization instruction to judge prompt |
| Critique exhaustion | Critic repeats same issues | DeadEndTracker flags repeated critique categories |
| Synthesis blandness | Synthesis always loses (too safe) | Track synthesis win rate; if < 10%, add "prefer boldness" to synthesis prompt |
| Oscillation | A→B→A→B cycle, never converges | Detect via ledger pattern analysis; escalate to HumanGate |
| Budget explosion | 10+ rounds without convergence | BudgetController enforces hard cap; escalate at warning threshold |
| Correlated judge bias | All judges from same model agree on wrong answer | Enable `model_diversity: true` — use different providers |
| Rubric ambiguity | Judges disagree on criteria interpretation | Low confidence signals rubric needs refinement; escalate |

### When NOT to Use Autoreason

- **Objective metrics exist**: Use scalar evaluator. Faster, cheaper, more reliable.
- **Trivial improvements**: If the task is "fix the typo," debate is overkill.
- **Time-critical loops**: Each debate round takes 10-30 seconds (5+ LLM calls).
  For real-time optimization, use scalar evaluation.
- **Highly deterministic domains**: Compiler optimization, numerical methods —
  the benchmark suite IS the evaluator.

Autoreason is for the space where:
1. Quality matters but can't be reduced to a number
2. The cost of getting it wrong justifies the cost of debate (5-35 LLM calls)
3. Human evaluation is too slow for the iteration frequency you need

---

## Cost Model

### Per-Round Cost

| Phase | LLM calls | Typical tokens (in + out) |
|-------|-----------|--------------------------|
| Attack | 1 | ~2000 in, ~1000 out |
| Revise | 1 | ~4000 in, ~2000 out |
| Synthesize | 1 | ~5000 in, ~2000 out |
| Judge (x3) | 3 | ~4000 in, ~500 out each |
| **Total** | **6** | **~20K in, ~5.5K out** |

At Claude Sonnet rates (~$3/M input, ~$15/M output):
- Per round: ~$0.06 input + ~$0.08 output = **~$0.14/round**
- 5-round convergence: **~$0.70 per artifact optimization**
- 10-round convergence: **~$1.40 per artifact optimization**

At GPT-4.1 rates (~$2/M input, ~$8/M output):
- Per round: ~$0.04 input + ~$0.04 output = **~$0.08/round**
- 5-round convergence: **~$0.40 per artifact optimization**

### Budget Planning

```yaml
budget:
  max_trials: 10       # Maximum debate rounds
  cost_budget: 2.00    # Hard cost cap in USD
  token_budget: 300000 # Total token cap across all phases
```

### Optimization: Skip Synthesis When Cheap

If strategy distillation shows synthesis win rate < 10% after the first 3 rounds,
drop Phase 3 for remaining rounds. Saves ~25% of per-round cost.

```rust
if debate_strategy.phase_win_rates[&Winner::Synthesis] < 0.10
    && round_number > 3
{
    // Skip synthesis, judge only A vs B
}
```

---

## Implementation Sequence

### Phase 1: Types and Traits (no runtime dependencies)

```
Add to autoany-core/src/:
  types.rs   → DebateConfig, Winner, JudgeVote, CritiqueResult,
               DebateRound, ComparisonOutcome (+ serde derives)
  comparative_evaluator.rs → ComparativeEvaluator trait
  llm_backend.rs → LlmBackend trait

Update lib.rs → pub mod comparative_evaluator, llm_backend

Depends on: nothing new. Pure type definitions.
Tests: unit tests for serde roundtrips on new types.
```

### Phase 2: Protocol Functions (depends on Phase 1)

```
Add autoany-core/src/debate.rs:
  attack(), revise(), synthesize(), judge(), decide()
  autoreason_round() — orchestrates one full round

Depends on: LlmBackend trait, new types.
Tests: mock LlmBackend that returns canned responses.
  - Test label randomization produces different orderings.
  - Test decide() correctly aggregates votes.
  - Test full round with mock backend.
```

### Phase 3: Evaluator Implementation (depends on Phase 2)

```
Add autoany-core/src/autoreason_evaluator.rs:
  AutoreasonEvaluator<B: LlmBackend> implements ComparativeEvaluator

Depends on: debate.rs, ComparativeEvaluator trait.
Tests: mock backend → verify compare() runs full protocol.
```

### Phase 4: Loop Variant (depends on Phase 3 + existing modules)

```
Add autoany-core/src/debate_loop.rs:
  DebateLoop — uses ComparativeEvaluator, BudgetController,
  PromotionController, Ledger.

Depends on: everything above + existing EGRI modules.
Tests: full integration test with mock backend:
  - Convergence detection (mock judges always pick incumbent after N rounds).
  - Promotion of revision/synthesis winners.
  - Budget exhaustion halts loop.
  - Ledger records full debate transcripts.
```

### Phase 5: Spec and Strategy Extensions (depends on Phase 4)

```
Update autoany-core/src/spec.rs:
  PromotionPolicy::Comparative
  DebateSpec in ProblemSpec

Add debate strategy distillation to strategy.rs:
  debate_distill() → DebateStrategyReport

Update inheritance.rs:
  Carry forward debate strategy reports.

Update DOMAIN-MAPPINGS.md:
  Add Writing, Strategy, Design, Prompt Engineering domains.
```

### Phase 6: Adapter Crates (depends on Phase 4)

```
Update autoany-aios/:
  Route debate LLM calls through Arcan agent sessions.

Update autoany-lago/:
  Persist debate transcripts as EventKind::Custom("egri.debate.*").
  Enable querying: "show critique patterns across runs."
```

### Phase 7: Example Instance (depends on all above)

```
Add examples/blog-post-optimizer/:
  problem-spec.yaml — blog post optimization with debate evaluation
  artifacts/post.md — initial blog post draft
  eval/ — debate config + rubric
  harness/ — format validation (objective) + debate (subjective)
```

---

## Open Questions

### 1. Should the Debate Generate New Artifacts?

The current protocol generates Version B (revision) and Version AB (synthesis)
during evaluation. If one of these wins, the promoted artifact is something the
Proposer never generated. This is powerful but means the evaluator is also a
refiner — blurring the EGRI role boundaries.

**Alternative**: Have the debate only compare Proposer-generated candidates.
The Proposer generates N candidates, the debate ranks them, the best is promoted.
Cleaner role separation but loses the adversarial revision mechanism.

**Recommendation**: Keep artifact generation in the debate. The value of
autoreason comes precisely from the adversarial revision cycle. Document the
role-blurring explicitly and ensure rollback works for debate-generated artifacts.

### 2. Judge Model Diversity

Should all judges use the same model, or different models?

- **Same model, different label orderings**: Cheapest. Tests positional robustness.
- **Different models, same provider**: Moderate cost. Tests model-specific biases.
- **Different providers**: Most expensive. Maximally decorrelated judgments.

**Recommendation**: Default to same model with label randomization. Add
`model_diversity: true` as an option for high-stakes domains. The
`LlmBackend::generate_with_model()` method supports this.

### 3. How to Handle Judge Disagreement

When judges split (e.g., 2:1 or worse), what does it mean?

- **Low confidence on an easy round**: Possible rubric ambiguity. Log for
  strategy distillation.
- **Low confidence throughout**: The quality dimension may be genuinely
  subjective. Consider escalating to `HumanGate` for calibration.
- **Consistent 2:1 splits on specific criteria**: One judge may have a
  systematically different interpretation. Strategy distillation can detect this.

### 4. Feedback to the Proposer

Should the Proposer see debate transcripts from previous rounds?

- **Yes**: The Proposer can learn what the judges value and propose targeted
  mutations. Risk: the Proposer "overfits" to judge preferences.
- **No**: The Proposer generates independently. Slower convergence but more
  diverse exploration.
- **Partial**: The Proposer sees the winning critique categories but not the
  full transcript. Balanced signal without overfitting.

**Recommendation**: Partial. Pass `DebateStrategyReport` to the Proposer's
ledger context, not raw transcripts.

---

## References

### Origin

- [shl0ms (@SHL0MS)](https://x.com/SHL0MS/status/2037939506733523025) — autoreason concept
- [Andrej Karpathy (@karpathy)](https://x.com/karpathy/status/2037921699824607591) — LLM sycophancy observation
- [autoresearch (GitHub)](https://github.com/karpathy/autoresearch) — objective EGRI loop that autoreason extends

### Multi-Agent Debate Literature

- [Peacemaker or Troublemaker: Sycophancy in Multi-Agent Debate](https://arxiv.org/abs/2509.23055) — mixing cooperative/adversarial personas halves sycophancy rates
- [D3: Debate, Deliberate, Decide](https://arxiv.org/abs/2410.04663) — structured debate with budgeted stopping and convergence checks
- [PROClaim: Courtroom-Style Multi-Agent Debate](https://arxiv.org/abs/2603.28488) — specialized roles + evidence protocols, +10pp over standard debate
- [Talk Isn't Always Cheap: Failure Modes in Multi-Agent Debate](https://arxiv.org/abs/2509.05396) — debate can degrade performance; sycophancy drives premature convergence
- [Multi-Agent Debate with Adaptive Stability Detection](https://arxiv.org/abs/2510.12697) — automatic convergence detection in debate
- [Debate-Reflection Cycles in Multi-Agent Systems](https://emergentmind.com/topics/debate-reflection-cycles) — DTE+RCR halves sycophancy rates

### Self-Improvement Literature

- [Self-Refine: Iterative Refinement with Self-Feedback](https://arxiv.org/abs/2303.17651) — same-model feedback loop (autoreason improves on this via role separation)
- [The Karpathy Loop (Fortune)](https://fortune.com/2026/03/17/andrej-karpathy-loop-autonomous-ai-agents-future/) — 700 experiments, 2 days
