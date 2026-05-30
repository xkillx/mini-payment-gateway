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

| Actor       | Actor ID (sub)                        | Role          | Merchant Account ID                   |
|-------------|----------------------------------------|---------------|----------------------------------------|
| Merchant    | `00000000-0000-0000-0000-000000000001` | merchant      | `00000000-0000-0000-0000-000000000001` |
| Admin       | `00000000-0000-0000-0000-000000000002` | administrator | (none)                                 |

The **Actor ID** (`sub` in JWT) identifies the authenticated party.
The **Merchant Account ID** (`merchant_id` in JWT and DB) identifies the business account owning payments and refunds.

Run `cargo run -p migrate -- seed` to insert seed data (upsert with ON CONFLICT DO UPDATE).

## Authentication

### Bearer JWT Contract

All `/api/v1` routes require `Authorization: Bearer <jwt>`. The JWT must:

- Be signed with HS256 using `JWT_SECRET` from the environment
- Include `sub` (Actor ID UUID), `role` (`merchant` or `administrator`), and `exp` (unexpired)
- Include `merchant_id` (Merchant Account ID UUID) for merchant tokens
- Omit `merchant_id` for administrator tokens

`GET /health` is public (no auth required).

### Role Permissions

| Route family | Merchant | Administrator |
|---|---|---|
| `GET /api/v1/payments`, `GET /api/v1/refunds` | Yes | Yes |
| `POST /api/v1/payments`, `POST /api/v1/refunds` | Yes | No |
| `/api/v1/notifications`, `/api/v1/reconciliation` | No | Yes |
| `/api/v1/audit`, `/api/v1/reporting`, `/api/v1/admin` | No | Yes |

### Generate a Developer Token

```bash
# Merchant token
cargo run -p shared-auth --example generate_token -- merchant

# Administrator token
cargo run -p shared-auth --example generate_token -- administrator
```

Set `JWT_SECRET` env var or it defaults to `dev-secret-change-in-production`.

## Testing

```bash
# Unit tests
cargo test --workspace --lib

# Integration tests (requires running PostgreSQL)
cargo test --workspace
```

Integration tests require a PostgreSQL database. Use `make dev-up` to start one via Docker.
