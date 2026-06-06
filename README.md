# Mini Payment Gateway

A payment gateway MVP with a Rust backend and a React Merchant Dashboard.

## Local Startup

Prerequisites: Rust (stable), Docker, Node.js 20+.

```bash
# Start PostgreSQL
make dev-up

# Run migrations and seed data
make migrate

# Start API server (port 4000)
make run-api

# Start worker (payment processing and notification delivery)
make run-worker

# Start the Merchant Dashboard (port 5173)
make run-web
```

## Merchant Dashboard

The Merchant Dashboard is a React app under `apps/web/`. It provides an operational workspace for Merchants to manage Payments and Refunds.

### Quick Start

```bash
# 1. Generate a Merchant JWT
cargo run -p shared-auth --example generate_token -- merchant

# 2. Install and start the frontend
cd apps/web && npm install && npm run dev

# 3. Open http://localhost:5173 and paste the JWT to access the dashboard
```

The dashboard uses `VITE_API_BASE_URL` (default `http://localhost:4000`) to call the Rust API. Administrator tokens are rejected by the Merchant Dashboard.

## Available Make Targets

| Target      | Description                        |
|-------------|------------------------------------|
| `dev-up`    | Start PostgreSQL via Docker        |
| `dev-down`  | Stop PostgreSQL                    |
| `migrate`   | Run all migrations + seed          |
| `seed`      | Run seed data only                 |
| `run-api`   | Start the HTTP API                 |
| `run-worker`| Start the background worker        |
| `run-web`   | Start the Merchant Dashboard        |
| `test`      | Run all tests                      |
| `lint`      | Run clippy checks                  |
| `fmt`       | Check formatting                   |

## Project Structure

```
apps/
  api/       - HTTP API server (axum)
  web/       - React Merchant Dashboard (Vite + TypeScript)
  worker/    - Background job worker
  migrate/   - Migration runner + seed
crates/
  dashboard/          - Merchant Dashboard summary API
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
