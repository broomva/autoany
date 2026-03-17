.PHONY: build test check fmt smoke clean

# === Build ===
build:
	cd autoany-core && cargo build

# === Test ===
test: test-rust test-python

test-rust:
	cd autoany-core && cargo test

test-python:
	python3 -m pytest tests/ -v

# === Lint & Format ===
check: check-rust check-python

check-rust:
	cd autoany-core && cargo fmt --check
	cd autoany-core && cargo clippy -- -D warnings

check-python:
	python3 -m ruff check autoany/scripts/ tests/ examples/ --select E,F,W

fmt:
	cd autoany-core && cargo fmt
	python3 -m ruff format autoany/scripts/ tests/ examples/

# === Smoke Test (quick gate) ===
smoke: build check test

# === Clean ===
clean:
	cd autoany-core && cargo clean
