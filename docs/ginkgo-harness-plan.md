# Ginkgo harness implementation

Owner request: implement three verification improvements and open PRs.

Sources: workspace PR 1321, stored SPA prompt and report. SPA symbol labels in
handoff do not match stored source; do not repeat unverified specifics.

Dependency trace:
- skills/governance/cross-review/scripts/review_packet.py validates explicit
  claim states and evidence-linked findings; references/claim-funnel.md supplies
  isolated dispatch briefs. autoreview and peer-review use this contract.
- skills/tooling/prove-the-negative/scripts/verdict.py retains PASS/FAIL/INVALID
  exits; additive JSON output separates claim conclusion from run status.
- skills/tooling/attempt-audit/scripts/attempt_audit.py reports observation and
  coverage independently. Static absence of findings never refutes all defects.
- skills/orchestration/handback/scripts/capability_plan.py gates executable work
  on a catalog entry, tool/input/output schema references and readiness evidence.
- autoany-core Outcome gains typed optional control-aware evidence; selectors and
  loop baseline/promotion reject inconclusive or inconsistent evidence. Legacy
  outcomes deserialize unchanged. Ledger schema and scaffold match runtime.

Validation: CLI valid/invalid fixtures, regressions, mutation checks, source
lint, Autoany Rust/Python tests, and independent read-only review on pinned SHAs.
No merge is requested; stop at linked PRs with review/check state disclosed.
