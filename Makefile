.PHONY: help fmt check rust-test sdk-ts sdk-py sdk-go docs deployment-check

help:
	@printf '%s\n' 'fmt: check authored Rust formatting' 'check: deny Rust lint warnings' 'rust-test: workspace tests and real journeys' 'sdk-ts: install locked JS dependencies and test SDKs' 'sdk-py: test an already-built Python extension in target/python-sdk-venv' 'sdk-go: test the Go client against the real daemon' 'docs: check catalogs/links and build mdBook' 'deployment-check: offline manifest validation (requires Compose and Helm)'

fmt:
	cargo fmt --all --check

check:
	cargo clippy --workspace --all-targets --locked -- -D warnings

rust-test:
	cargo build --workspace --locked
	cargo test --workspace --locked

sdk-ts:
	npm ci --prefix sdk/typescript
	npm --prefix sdk/typescript test

sdk-py:
	cargo build --locked -p hm-cli
	target/python-sdk-venv/bin/python -m pytest sdk/python/tests -q

sdk-go:
	cargo build --locked -p hm-cli
	go test -race -count=1 -timeout 10m ./sdk/go/...

docs:
	cargo run --locked -p hm-eval -- docs-gate
	mdbook build docs

deployment-check:
	python3 deploy/verify.py --require-tools
