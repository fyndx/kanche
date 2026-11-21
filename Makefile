# SPDX-License-Identifier: MPL-2.0

.DEFAULT_GOAL := help

.PHONY: help tools setup dev fmt lint test test-rust test-integration test-sdk check security

help:
	@echo 'make setup        Verify tool versions and install locked dependencies'
	@echo 'make dev          Run Kanche (health-only without database config)'
	@echo 'make fmt          Format Rust, JavaScript, configuration, and docs'
	@echo 'make lint         Check formatting, Clippy, JavaScript, and docs'
	@echo 'make test         Run Rust tests and process smoke checks'
	@echo 'make test-integration Run disposable PostgreSQL security invariants (Docker)'
	@echo 'make check        Run the same quality checks as CI'
	@echo 'make security     Scan secrets, advisories, and dependency licenses'

tools:
	node scripts/check-tools.mjs

setup: tools
	rustup show active-toolchain
	npm ci
	cargo fetch --locked

dev:
	cargo run --locked

fmt: tools
	cargo fmt --all
	npm run fmt

lint: tools
	cargo fmt --all -- --check
	cargo clippy --locked --all-targets -- -D warnings
	npm run fmt:check
	npm run lint

test-rust:
	cargo test --locked --all-targets
	cargo build --locked
	python3 scripts/smoke-server.py

test-integration:
	python3 scripts/test-integration.py

test-sdk:
	npm run typecheck
	npm run test:sdk
	node scripts/check-contract.mjs

test: test-rust test-sdk test-integration

check: lint test
	cargo doc --locked --no-deps

security: tools
	python3 scripts/install-security-tools.py
	.tools/gitleaks dir --redact --no-banner --exit-code 1 .
	.tools/cargo-deny --locked check
	npm audit --audit-level=high
	npm run licenses
