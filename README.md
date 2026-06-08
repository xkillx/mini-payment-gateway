# Mini Payment Gateway

Mini Payment Gateway is a local payment gateway MVP built with a Rust backend, PostgreSQL, and a React dashboard. It models common payment platform workflows such as payment creation, refunds, notification delivery, reconciliation, audit records, and operational reporting.

This project is designed for learning, experimentation, and portfolio demonstration. It does not connect to a real payment processor.

## Features

- Merchant dashboard for viewing payment and refund activity
- Administrator dashboard for system health, reporting, notifications, reconciliation, audit records, and actor management
- Authenticated HTTP API with merchant and administrator roles
- Payment lifecycle tracking with status history
- Refund request and tracking flow
- Notification delivery records with retry support
- Reconciliation reports for comparing expected and actual totals
- Database migrations and deterministic seed data for local development
- Rust and React test coverage

## Tech Stack

| Area | Technology |
| --- | --- |
| Backend | Rust, Axum, Tokio |
| Database | PostgreSQL, SQLx |
| Frontend | React, TypeScript, Vite |
| Auth | JWT bearer tokens |
| Local services | Docker Compose |
| Testing | Cargo test, Vitest |

## Prerequisites

Install these before running the project:

- Rust stable toolchain
- Docker and Docker Compose
- Node.js 20 or newer
- npm

## Quick Start

From the repository root, run the following steps.

### 1. Create a local environment file

```bash
cp .env.example .env
```

The `.env` file is ignored by Git. Keep real secrets out of the repository.

### 2. Start PostgreSQL

```bash
make dev-up
```

This starts a local PostgreSQL container on port `5432`.

### 3. Install frontend dependencies

```bash
make install-web
```

### 4. Run migrations and seed data

```bash
make migrate
```

This applies all database migrations and creates deterministic local seed data.

### 5. Start the API

Open a new terminal in the repository root and run:

```bash
make run-api
```

The API listens on `http://localhost:4000`.

You can verify it is running with:

```bash
curl http://localhost:4000/health
```

### 6. Start the worker

Open another terminal in the repository root and run:

```bash
make run-worker
```

The worker processes background payment and notification jobs.

### 7. Start the dashboard

Open another terminal in the repository root and run:

```bash
make run-web
```

The dashboard runs at `http://localhost:5173`.

## Dashboard Access

The dashboard asks for a JWT. Generate a local token, paste it into the access screen, and the app will open the correct workspace based on the token role.

Generate a merchant token:

```bash
cargo run -p shared-auth --example generate_token -- merchant
```

Generate an administrator token:

```bash
cargo run -p shared-auth --example generate_token -- administrator
```

Seeded development users:

| User | Role | Actor ID | Merchant Account ID |
| --- | --- | --- | --- |
| Merchant | `merchant` | `00000000-0000-0000-0000-000000000001` | `00000000-0000-0000-0000-000000000001` |
| Administrator | `administrator` | `00000000-0000-0000-0000-000000000002` | None |

## API Overview

The public health check is available without authentication:

```text
GET /health
```

All `/api/v1` routes require `Authorization: Bearer <jwt>`.

| Route family | Purpose | Access |
| --- | --- | --- |
| `/api/v1/payments` | Create, list, and view payments | Merchant and administrator |
| `/api/v1/refunds` | Create, list, and view refunds | Merchant and administrator |
| `/api/v1/dashboard/merchant` | Merchant dashboard summary | Merchant |
| `/api/v1/dashboard/admin` | Administrator dashboard summary | Administrator |
| `/api/v1/notifications` | Notification monitoring and retries | Administrator |
| `/api/v1/reconciliation` | Reconciliation reports | Administrator |
| `/api/v1/audit` | Audit record search and details | Administrator |
| `/api/v1/reporting` | Payment reporting | Administrator |
| `/api/v1/admin` | Actor administration | Administrator |

The OpenAPI contract is maintained in [docs/api/openapi.yaml](docs/api/openapi.yaml).

## Environment Variables

The root `.env.example` file contains the default local configuration.

| Variable | Default | Description |
| --- | --- | --- |
| `DATABASE_URL` | `postgres://payment:payment@localhost:5432/payment_gateway` | PostgreSQL connection string |
| `APP_PORT` | `4000` | HTTP API port |
| `JWT_SECRET` | `dev-secret-change-in-production` | JWT signing secret for local development |
| `LOG_LEVEL` | `info` | Application log level |
| `WORKER_POLL_INTERVAL_MS` | `1000` | Worker polling interval in milliseconds |
| `NOTIFICATION_MAX_ATTEMPTS` | `5` | Maximum notification delivery attempts per retry generation |
| `NOTIFICATION_RETRY_DELAYS_SECS` | `30,120,600,1800,7200` | Notification retry delays in seconds |
| `PAYMENT_CURRENCY` | `USD` | Accepted payment and refund currency |

The frontend reads `VITE_API_BASE_URL`. If it is not set, it defaults to `http://localhost:4000`.

## Common Commands

| Command | Description |
| --- | --- |
| `make dev-up` | Start PostgreSQL with Docker Compose |
| `make dev-down` | Stop PostgreSQL |
| `make migrate` | Apply migrations and seed data |
| `make seed` | Re-run seed data only |
| `make run-api` | Start the Rust API server |
| `make run-worker` | Start the background worker |
| `make install-web` | Install frontend dependencies |
| `make run-web` | Start the React dashboard |
| `make test` | Run Rust tests |
| `make test-web` | Run frontend tests |
| `make build-web` | Build the frontend |
| `make lint` | Run Rust clippy checks |
| `make fmt` | Check Rust formatting |

## Testing

Run Rust tests:

```bash
make test
```

Run frontend tests:

```bash
make test-web
```

Rust integration tests require PostgreSQL. Start it first with:

```bash
make dev-up
```

## Project Structure

```text
apps/
  api/       HTTP API server
  migrate/   Migration runner and seed command
  web/       React dashboard
  worker/    Background worker

crates/
  admin/                 Actor administration domain
  audit/                 Audit record domain
  dashboard/             Dashboard summary APIs
  notifications/         Notification delivery domain
  payments/              Payment domain
  reconciliation/        Reconciliation domain
  refunds/               Refund domain
  reporting/             Reporting domain
  shared-auth/           JWT parsing and auth types
  shared-config/         Environment configuration
  shared-db/             Database connection and seed logic
  shared-http/           Shared HTTP errors and middleware
  shared-observability/  Logging and tracing setup

docs/
  api/                   OpenAPI documentation
  adr/                   Architecture decision records
  domain-model/          Domain model documentation
  designs/               Product and UI design references
```

## Publishing to GitHub

Before pushing this repository to GitHub:

1. Confirm `.env` is not staged. It is already listed in `.gitignore`.
2. Commit `.env.example` so other developers can create their own local config.
3. Run the checks that are practical for your machine:

```bash
make fmt
make lint
make test
make test-web
```

4. Add a `LICENSE` file if you want to make the repository open source.
5. Add a GitHub remote and push the branch.

```bash
git remote add origin https://github.com/<your-username>/<your-repo>.git
git push -u origin main
```

Replace `<your-username>` and `<your-repo>` with your GitHub account and repository name.

## Further Reading

- [Development guide](docs/development.md)
- [API documentation](docs/api/README.md)
- [Product requirements](docs/PRD.md)
- [Core domain model](docs/domain-model/core-domain-model.md)
