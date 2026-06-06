# Mini Payment Gateway

A payment gateway MVP with a Rust backend, a React Merchant Dashboard, and a React Administrator Dashboard.

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

# Start the Dashboard (port 5173)
make run-web
```

## Dashboard

The web dashboard is a React app under `apps/web/`. It provides separate workspaces for Merchants and Administrators, selected after JWT role detection.

### Quick Start

```bash
# Generate a Merchant JWT
cargo run -p shared-auth --example generate_token -- merchant

# Generate an Administrator JWT
cargo run -p shared-auth --example generate_token -- administrator

# Install and start the frontend
cd apps/web && npm install && npm run dev

# Open http://localhost:5173 and paste your JWT to access the dashboard
```

The dashboard uses `VITE_API_BASE_URL` (default `http://localhost:4000`) to call the Rust API. Merchant tokens open the Merchant Dashboard, Administrator tokens open the Administrator Dashboard.

## Available Make Targets

| Target      | Description                        |
|-------------|------------------------------------|
| `dev-up`    | Start PostgreSQL via Docker        |
| `dev-down`  | Stop PostgreSQL                    |
| `migrate`   | Run all migrations + seed          |
| `seed`      | Run seed data only                 |
| `run-api`   | Start the HTTP API                 |
| `run-worker`| Start the background worker        |
| `run-web`   | Start the Dashboard                |
| `test`      | Run all tests                      |
| `lint`      | Run clippy checks                  |
| `fmt`       | Check formatting                   |

## Project Structure

```
apps/
  api/       - HTTP API server (axum)
  web/       - React Dashboard (Vite + TypeScript) — Merchant & Administrator workspaces
  worker/    - Background job worker
  migrate/   - Migration runner + seed
crates/
  dashboard/          - Merchant & Administrator Dashboard summary API
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
