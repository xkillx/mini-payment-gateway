# How to Run Mini Payment Gateway

This guide starts the full local stack:

- PostgreSQL database
- Rust API server
- Rust background worker
- React dashboard

## 1. Prerequisites

Install these tools first:

- Rust stable toolchain
- PostgreSQL running locally
- Node.js 20 or newer
- npm

Check them with:

```bash
rustc --version
cargo --version
psql --version
node --version
npm --version
```

## 2. Clone and Enter the Project

```bash
git clone <repository-url>
cd mini-payment-gateway
```

If you already have the repository locally, just enter the project root:

```bash
cd /Users/mac/code/mini-payment-gateway
```

## 3. Create the Local Environment File

Copy the example environment file:

```bash
cp .env.example .env
```

The default local values are:

```env
DATABASE_URL=postgres://payment:payment@localhost:5432/payment_gateway
APP_PORT=4000
JWT_SECRET=dev-secret-change-in-production
LOG_LEVEL=info
WORKER_POLL_INTERVAL_MS=1000
NOTIFICATION_MAX_ATTEMPTS=5
NOTIFICATION_RETRY_DELAYS_SECS=30,120,600,1800,7200
```

The frontend uses `VITE_API_BASE_URL`. If it is not set, it defaults to:

```env
VITE_API_BASE_URL=http://localhost:4000
```

## 4. Prepare the Local PostgreSQL Database

Make sure your local PostgreSQL server is running.

On macOS with Homebrew:

```bash
brew services start postgresql@16
```

If your installed service name is different, check it with:

```bash
brew services list
```

Create the development database user and database:

```bash
createuser payment
psql postgres -c "ALTER USER payment WITH PASSWORD 'payment';"
createdb -O payment payment_gateway
```

If the `payment` user or `payment_gateway` database already exists, those commands can fail safely. Continue to the next step.

Confirm the app can connect:

```bash
psql "postgres://payment:payment@localhost:5432/payment_gateway" -c "SELECT 1;"
```

## 5. Run Migrations and Seed Data

From the project root:

```bash
make migrate
```

This applies all database migrations and seeds the development actors.

Seeded users:

| User | Role | Actor ID |
|---|---|---|
| Merchant | `merchant` | `00000000-0000-0000-0000-000000000001` |
| Administrator | `administrator` | `00000000-0000-0000-0000-000000000002` |

## 6. Start the API Server

Open a new terminal tab in the project root and run:

```bash
make run-api
```

The API listens on:

```text
http://localhost:4000
```

Check the API health endpoint:

```bash
curl http://localhost:4000/health
```

Expected response:

```json
{"status":"ok","version":"0.1.0"}
```

## 7. Start the Worker

Open another terminal tab in the project root and run:

```bash
make run-worker
```

The worker processes pending payments and notification deliveries.

## 8. Install Frontend Dependencies

Open another terminal tab in the project root and run:

```bash
make install-web
```

This runs `npm install` inside `apps/web`.

## 9. Start the Dashboard

From the project root:

```bash
make run-web
```

The dashboard opens at:

```text
http://localhost:5173
```

If the browser does not open automatically, open that URL manually.

## 10. Generate a Login Token

The dashboard expects a JWT. Generate one from the project root.

For the Merchant Dashboard:

```bash
cargo run -p shared-auth --example generate_token -- merchant
```

For the Administrator Dashboard:

```bash
cargo run -p shared-auth --example generate_token -- administrator
```

Copy the generated token, open `http://localhost:5173`, and paste it into the dashboard access screen.

Developer tokens expire after one hour.

## 11. Useful Commands

Run backend tests:

```bash
make test
```

Run frontend tests:

```bash
make test-web
```

Run Rust lint checks:

```bash
make lint
```

Check Rust formatting:

```bash
make fmt
```

Build the frontend:

```bash
make build-web
```

## 12. Stop the Project

Stop the API, worker, and dashboard by pressing `Ctrl+C` in their terminal tabs.

Keep PostgreSQL running if you use it for other local projects. Stop it only if you want to shut down your local database service.

## 13. Reset the Local Database

Use this only when you want to delete local database data and start fresh:

```bash
dropdb payment_gateway
createdb -O payment payment_gateway
make migrate
```

## Troubleshooting

If `make migrate` cannot connect to the database, confirm PostgreSQL is running:

```bash
pg_isready -h localhost -p 5432
```

If port `5432`, `4000`, or `5173` is already in use, stop the other process or change the relevant port configuration.

If the database user or database is missing, recreate them:

```bash
createuser payment
psql postgres -c "ALTER USER payment WITH PASSWORD 'payment';"
createdb -O payment payment_gateway
```

If the dashboard cannot reach the API, confirm the API is healthy:

```bash
curl http://localhost:4000/health
```

If authentication fails, generate a fresh token and make sure the API and token generator use the same `JWT_SECRET` from `.env`.

## Dev Tokens (generated 2026-06-08, expires in 1hr)

**Merchant:**
```
eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIwMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDEiLCJyb2xlIjoibWVyY2hhbnQiLCJtZXJjaGFudF9pZCI6IjAwMDAwMDAwLTAwMDAtMDAwMC0wMDAwLTAwMDAwMDAwMDAwMSIsImV4cCI6MTc4MDkwMzI5M30.jw6l0d0srOFTkOsVGN6ijQscLASoFGk1Vp-BT6a70Dk
```

**Administrator:**
```
eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIwMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDIiLCJyb2xlIjoiYWRtaW5pc3RyYXRvciIsIm1lcmNoYW50X2lkIjpudWxsLCJleHAiOjE3ODA5MDMyOTN9.hEQC39B1ZkDECngpt_UUXuSacJDTfvpkAfGHLfR7ZVU
```
