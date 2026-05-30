# Development Guide

## Architecture

Modular monolith with domain-first module boundaries:

```
apps/
  api/       - Axum HTTP server
  worker/    - Background job worker (notification delivery)
  migrate/   - Migration runner + seed

crates/
  shared-config/       - Environment config (AppConfig)
  shared-db/           - DB connection pool + seed logic
  shared-http/         - Error envelope, middleware, route helpers
  shared-auth/         - JWT claim parsing + actor context
  shared-observability/- Tracing/logging initialization
  payments/            - Payment domain (models, repo, service, routes)
  refunds/             - Refund domain
  notifications/       - Notification delivery domain
  reconciliation/      - Reconciliation domain
  audit/               - Audit record domain
  reporting/           - Reporting domain
  admin/               - Admin/actor domain
```

## Crate Ownership

Each domain crate owns its migrations in `crates/<name>/migrations/`.
Migration files use global timestamps (`YYYYMMDDHHMMSS__name.sql`).
The top-level `apps/migrate` runner discovers all migrations, sorts by timestamp, and applies them.

## Migration Process

```bash
# Apply all pending migrations + seed data
cargo run -p migrate -- up

# Check if all migrations are applied
cargo run -p migrate -- check

# Re-run seed data only
cargo run -p migrate -- seed
```

## Seed Data

Two actors are seeded with deterministic IDs:

| Actor       | ID                                   | Role          |
|-------------|--------------------------------------|---------------|
| Merchant    | `00000000-0000-0000-0000-000000000001` | merchant      |
| Admin       | `00000000-0000-0000-0000-000000000002` | administrator |

Run `cargo run -p migrate -- seed` to insert seed data (idempotent via ON CONFLICT DO NOTHING).

## Testing

```bash
# Unit tests
cargo test --workspace --lib

# Integration tests (requires running PostgreSQL)
cargo test --workspace
```

Integration tests require a PostgreSQL database. Use `make dev-up` to start one via Docker.
