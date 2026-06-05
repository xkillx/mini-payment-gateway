# Development Guide

## Architecture

Modular monolith with domain-first module boundaries:

```
apps/
  api/       - Axum HTTP server
  worker/    - Background job worker (payment processing and notification delivery)
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

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `postgres://payment:payment@localhost:5432/payment_gateway` | PostgreSQL connection string |
| `APP_PORT` | `4000` | HTTP server port |
| `JWT_SECRET` | `dev-secret-change-in-production` | HS256 signing key for JWTs |
| `LOG_LEVEL` | `info` | Log level |
| `WORKER_POLL_INTERVAL_MS` | `1000` | Background worker poll interval |
| `NOTIFICATION_MAX_ATTEMPTS` | `5` | Max notification delivery attempts |
| `NOTIFICATION_RETRY_DELAYS_SECS` | `30,120,600,1800,7200` | Comma-separated retry delay seconds |
| `PAYMENT_CURRENCY` | `USD` | Configured Currency — only this currency is accepted for Payment and Refund amounts |

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

## Notification Destinations (local testing until MPG-028)

Notification Destinations are not seeded by default. To enable notification delivery records during local development, insert an active destination manually:

```sql
INSERT INTO notification_destinations (id, merchant_id, destination_url, is_active)
VALUES (
  '00000000-0000-0000-0000-000000000003',
  '00000000-0000-0000-0000-000000000001',
  'https://example.test/webhook',
  true
);
```

This creates an active destination for the seeded merchant (`00000000-0000-0000-0000-000000000001`). The worker will then project eligible domain events into pending notification delivery records and attempt delivery.

The supported externally delivered event types are `payment.created`, `payment.successful`, `payment.failed`, `refund.created`, and `refund.completed`. The internal `payment.processing` event is not projected for notification delivery.

### Notification Delivery Payload

The worker sends the persisted Domain Event envelope as a JSON `POST` to the Notification Destination:

```json
{
  "event_id": "<domain_events.id>",
  "event_type": "payment.successful",
  "occurred_at": "<domain_events.created_at RFC3339>",
  "resource_type": "payment",
  "resource_id": "<domain_events.aggregate_id>",
  "schema_version": 1,
  "payload": {}
}
```

Delivery is at-least-once. Merchants should deduplicate using `event_id`. Only a `2xx` response marks delivery as successful. Non-`2xx` responses and transport errors are recorded as `last_error` on the notification delivery record and trigger retry or terminal failure per the configured `NOTIFICATION_MAX_ATTEMPTS` and `NOTIFICATION_RETRY_DELAYS_SECS`.

Each delivery attempt is persisted as a `notification_delivery_attempts` row with its outcome, HTTP status, and errors. `last_error` on the record reflects the latest delivery error only; historical failures are preserved in attempt rows.

Automatic retries are bounded per retry generation (not by lifetime `attempt_count`). When an Administrator retries a failed record via the API, the record is requeued (`failed` → `pending`), `retry_generation` increments by one, and a fresh automatic delivery budget begins for the new generation. Cumulative `attempt_count` and prior attempt rows are preserved across generations.


## Testing

```bash
# Unit tests
cargo test --workspace --lib

# Integration tests (requires running PostgreSQL)
cargo test --workspace
```

Integration tests require a PostgreSQL database. Use `make dev-up` to start one via Docker.
