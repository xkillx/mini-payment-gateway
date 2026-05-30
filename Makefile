.PHONY: dev-up dev-down migrate seed run-api run-worker test lint fmt

dev-up:
	docker compose up -d

dev-down:
	docker compose down

migrate:
	cargo run -p migrate -- up

seed:
	cargo run -p migrate -- seed

run-api:
	cargo run -p api

run-worker:
	cargo run -p worker

test:
	cargo test --workspace

lint:
	cargo clippy --all-targets --all-features -- -D warnings

fmt:
	cargo fmt --check
