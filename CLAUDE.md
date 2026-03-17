# Autoany — EGRI Framework

## Project Structure

- `autoany/` — Agent skill (Python): problem compilation, scaffolding, EGRI doctrine
- `autoany-core/` — Rust microkernel: loop orchestrator, traits, ledger, budget, promotion
- `examples/` — End-to-end demos (ticket-classifier)

## Commands

```bash
make check       # Lint + format check (Rust clippy + fmt, Python ruff)
make test        # Run all tests (Rust + Python)
make build       # Build Rust crate
make smoke       # Quick validation: build + check + test
make fmt         # Auto-format all code
```

## Architecture Rules

1. **Skill = compiler** — `autoany/` teaches agents to design EGRI systems
2. **Core = microkernel** — `autoany-core/` is domain-agnostic loop substrate
3. **Instance = generated** — domain logic lives in user projects, not in core
4. Never add domain-specific logic to `autoany-core/`
5. All trait implementations belong in user crates or `examples/`

## Rust Conventions

- Edition 2024
- All public items must have doc comments
- `cargo clippy -- -D warnings` must pass
- `cargo fmt --check` must pass
- Integration tests in `tests/`, unit tests inline with `#[cfg(test)]`

## Python Conventions

- Scripts must support `--help` and output JSON to stdout
- No external dependencies in skill scripts (stdlib only)

## EGRI Safety Laws (enforced in code)

1. Never mutate evaluator and artifact in the same trial
2. Never promote without constraint checks
3. Budget fails closed — no "one more try"
4. Rollback must always be available
5. Every trial is logged to the ledger
