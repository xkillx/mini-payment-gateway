# Mini Payment Gateway

A backend-only payment gateway MVP built with Rust (axum, tokio, sqlx, PostgreSQL).

## Local Startup

Prerequisites: Rust (stable), Docker.

```bash
# Start PostgreSQL
make dev-up

# Run migrations and seed data
make migrate

# Start API server (port 4000)
make run-api

# Start worker (payment processing and notification delivery)
make run-worker
```

## Available Make Targets

| Target      | Description                        |
|-------------|------------------------------------|
| `dev-up`    | Start PostgreSQL via Docker        |
| `dev-down`  | Stop PostgreSQL                    |
| `migrate`   | Run all migrations + seed          |
| `seed`      | Run seed data only                 |
| `run-api`   | Start the HTTP API                 |
| `run-worker`| Start the background worker        |
| `test`      | Run all tests                      |
| `lint`      | Run clippy checks                  |
| `fmt`       | Check formatting                   |

## Project Structure

```
apps/
  api/       - HTTP API server (axum)
  worker/    - Background job worker
  migrate/   - Migration runner + seed
crates/
  shared-config/     - Environment config
  shared-db/         - DB connection + seed
  shared-http/       - HTTP primitives (error envelope, middleware)
  shared-auth/       - JWT claim parsing
  shared-observability/ - Logging/tracing init
  payments/          - Payment domain
  refunds/           - Refund domain
  notifications/     - Notification delivery domain
  reconciliation/    - Reconciliation domain
  audit/             - Audit record domain
  reporting/         - Reporting domain
  admin/             - Admin/actor domain
```

## Environment Variables

See `.env.example` for all required variables.
