# Core Domain Model — Mini Payment Gateway

> **MPG-001** · Version 1.0 · MVP Scope

---

## 1. Entities and Attributes

### 1.1 Payment

The primary aggregate for the charge lifecycle. Created when a merchant initiates a charge request.

| Attribute | Type | Required | Description |
|---|---|---|---|
| `payment_id` | UUID | Yes | Unique identifier for the payment |
| `merchant_id` | UUID | Yes | Owner of this payment |
| `amount_minor` | Integer | Yes | Amount in minor units (e.g., cents) |
| `currency` | String(3) | Yes | ISO 4217 currency code (single configured currency for MVP) |
| `status` | PaymentStatus | Yes | Current lifecycle position |
| `failure_reason` | String | No | Reason for failure if status is `failed` |
| `idempotency_key` | String | Yes | Client-supplied key for idempotent creation |
| `metadata` | JSON | Yes | Merchant-supplied creation context for reconciliation and lookup |
| `created_at` | Timestamp | Yes | When the payment was created |
| `updated_at` | Timestamp | Yes | When the payment was last modified |

### 1.2 Refund

A full reversal of value against one successful Payment. MVP supports one full refund per Payment.

| Attribute | Type | Required | Description |
|---|---|---|---|
| `refund_id` | UUID | Yes | Unique identifier for the refund |
| `payment_id` | UUID | Yes | The payment being refunded |
| `amount_minor` | Integer | Yes | Amount in minor units (equals original payment amount in MVP) |
| `currency` | String(3) | Yes | ISO 4217 currency code (matches payment currency) |
| `status` | RefundStatus | Yes | Current lifecycle position |
| `failure_reason` | String | No | Reason for failure if status is `failed` |
| `idempotency_key` | String | No | Client-supplied key for idempotent creation |
| `created_at` | Timestamp | Yes | When the refund was created |
| `updated_at` | Timestamp | Yes | When the refund was last modified |

### 1.3 Notification Delivery Record

Tracks delivery of one domain event to one destination.

| Attribute | Type | Required | Description |
|---|---|---|---|
| `record_id` | UUID | Yes | Unique identifier for this delivery record |
| `event_id` | UUID | Yes | The domain event this record corresponds to |
| `destination` | String | Yes | Target delivery endpoint or address |
| `event_type` | String | Yes | The type of domain event being delivered |
| `status` | NotificationStatus | Yes | Current delivery status |
| `retry_count` | Integer | Yes | Number of delivery attempts so far |
| `last_error` | String | No | Error message from the last failed attempt |
| `last_attempt_at` | Timestamp | No | Timestamp of the most recent delivery attempt |
| `next_retry_at` | Timestamp | No | When the next retry is scheduled (if applicable) |
| `created_at` | Timestamp | Yes | When the record was created |
| `updated_at` | Timestamp | Yes | When the record was last modified |

### 1.4 Reconciliation

A record of one balance comparison run.

| Attribute | Type | Required | Description |
|---|---|---|---|
| `reconciliation_id` | UUID | Yes | Unique identifier for this reconciliation run |
| `expected_amount_minor` | Integer | Yes | Expected balance computed from successful payments and refunds |
| `actual_amount_minor` | Integer | Yes | Actual balance provided by the operator or external source |
| `discrepancy_minor` | Integer | Yes | Difference between expected and actual (actual − expected) |
| `currency` | String(3) | Yes | ISO 4217 currency code |
| `status` | ReconciliationStatus | Yes | Outcome of the reconciliation run |
| `notes` | String | No | Operator notes or context |
| `run_at` | Timestamp | Yes | When the reconciliation was executed |
| `created_at` | Timestamp | Yes | When the record was created |

### 1.5 Audit Record

An append-only historical fact about a performed action.

| Attribute | Type | Required | Description |
|---|---|---|---|
| `audit_id` | UUID | Yes | Unique identifier for this audit record |
| `actor_id` | UUID | No | Who performed the action (absent when principal is unknown) |
| `actor_type` | String | Yes | Category of actor: `merchant`, `administrator`, `system`, `unknown` |
| `action` | String | Yes | The action performed (e.g., `payment.created`, `refund.completed`) |
| `resource_type` | String | Yes | Type of resource acted upon (e.g., `auth`, `payment`, `refund`) |
| `resource_id` | String | Yes | Identifier of the resource acted upon (text, not necessarily UUID) |
| `details` | JSON | No | Immutable payload with action-specific context |
| `occurred_at` | Timestamp | Yes | When the action occurred |
| `created_at` | Timestamp | Yes | When the audit record was persisted |

---

## 2. Status Vocabularies and State Transitions

### 2.1 Payment Status

```
pending  →  processing  →  successful
                         →  failed
successful  →  refunded
```

| Current | Allowed Next | Precondition | Side Effect |
|---|---|---|---|
| `pending` | `processing` | Payment is not already processing | Emit `payment.processing` (internal); start processing workflow |
| `processing` | `successful` | Processing completed without error | Emit `payment.successful`; unlock refund eligibility |
| `processing` | `failed` | Processing completed with error | Emit `payment.failed`; record `failure_reason` |
| `successful` | `refunded` | Refund has been completed against this payment | Emit `payment.refunded` |
| `failed` | — | Terminal state | No further transitions allowed |
| `refunded` | — | Terminal state | No further transitions allowed |

**Terminal statuses:** `failed`, `refunded`

### 2.2 Refund Status

```
pending  →  processing  →  completed
                         →  failed
```

| Current | Allowed Next | Precondition | Side Effect |
|---|---|---|---|
| `pending` | `processing` | Refund is not already processing | Start refund processing workflow |
| `processing` | `completed` | Processing completed without error | Emit `refund.completed`; transition linked payment to `refunded` |
| `processing` | `failed` | Processing completed with error | Emit `refund.failed`; record `failure_reason`; payment remains `successful` |
| `completed` | — | Terminal state | No further transitions allowed |
| `failed` | — | Terminal state | No further transitions allowed |

**Terminal statuses:** `completed`, `failed`

### 2.3 Notification Delivery Status

| Status | Description |
|---|---|
| `pending` | Awaiting first delivery attempt |
| `processing` | Delivery attempt is in progress |
| `delivered` | Successfully delivered to destination |
| `failed` | All delivery attempts exhausted |

Transitions: `pending → processing → delivered | failed`

### 2.4 Reconciliation Status

| Status | Description |
|---|---|
| `matched` | Expected and actual balances are equal (discrepancy = 0) |
| `mismatched` | Expected and actual balances differ (discrepancy ≠ 0) |
| `error` | Reconciliation run encountered an error |

Transitions: reconciliation records are created with a final status; no lifecycle transitions.

---

## 3. Invariants and Business Rules

### 3.1 Entity Invariants

- **I-1:** A Payment belongs to exactly one merchant (`payment.merchant_id`).
- **I-2:** A Refund belongs to exactly one Payment; ownership is derived from the linked payment.
- **I-3:** At most one Refund record per Payment in MVP scope.
- **I-4:** A Refund can only be created against a Payment with status `successful`.
- **I-5:** The refund amount must equal the original payment amount in MVP (full refund only).
- **I-6:** A Payment transitions to `refunded` only after a Refund reaches `completed`.
- **I-7:** Terminal statuses (`failed`, `refunded` for Payment; `completed`, `failed` for Refund) cannot transition further.
- **I-8:** Audit Records are append-only; existing records cannot be modified or deleted.
- **I-9:** Domain event records are immutable after creation.
- **I-10:** Money values are stored as integers in minor units with an explicit ISO 4217 currency code.
- **I-11:** MVP enforces a single configured currency; payments with unsupported currencies are rejected at the command boundary.

### 3.2 Scenario Examples

#### Scenario A: Successful Payment → Completed Refund → Payment Refunded

1. Merchant creates payment. Status: `pending`.
2. System processes payment successfully. Status: `successful`.
3. Merchant creates refund. Status: `pending` (refund).
4. System processes refund successfully. Status: `completed` (refund).
5. Payment status transitions to `refunded`.

**Outcome:** Payment is `refunded`. No further actions are possible on this payment.

#### Scenario B: Successful Payment → Failed Refund → Payment Remains Successful

1. Merchant creates payment. Status: `pending`.
2. System processes payment successfully. Status: `successful`.
3. Merchant creates refund. Status: `pending` (refund).
4. System processes refund, encounters error. Status: `failed` (refund); `failure_reason` recorded.
5. Payment status remains `successful`.

**Outcome:** Payment stays `successful`. Failed-refund retry is future scope and must not create a second Refund record in MVP.

#### Scenario C: Duplicate Create Payment with Same Idempotency Key

1. Merchant sends create payment request with idempotency key `ABC123`.
2. Payment created. Status: `pending`. Response returns payment ID.
3. Merchant sends identical request again with same idempotency key `ABC123`.
4. System detects duplicate key. Returns existing payment record without creating a new one.

**Outcome:** Idempotent replay — no duplicate payment. Same payment ID returned.

If the same Merchant sends the same idempotency key with a different Payment Amount, Configured Currency, or Payment Metadata, the command is rejected as a conflict and no new side effects are produced.

---

## 4. Domain Events

### 4.1 Event Envelope

Every domain event uses the following immutable envelope:

| Field | Type | Required | Description |
|---|---|---|---|
| `event_id` | UUID | Yes | Unique identifier for this event instance |
| `event_type` | String | Yes | Fully qualified event type name |
| `occurred_at` | Timestamp | Yes | When the event occurred (wall-clock time) |
| `resource_id` | UUID | Yes | Primary resource identifier (payment_id or refund_id) |
| `resource_type` | String | Yes | `payment` or `refund` |
| `idempotency_key` | String | No | The idempotency key from the originating command, if any |
| `payload` | JSON | Yes | Event-type-specific data |
| `schema_version` | Integer | Yes | Version of the event schema (MVP: `1`) |

### 4.2 MVP Event Types

#### `payment.created`

- **Trigger:** Payment entity created via create payment command.
- **Payload:** `payment_id`, `merchant_id`, `amount_minor`, `currency`, `metadata`, `idempotency_key`, `created_at`

#### `payment.processing`

- **Trigger:** Payment transitions from `pending` to `processing`.
- **Payload:** `payment_id`, `amount_minor`, `currency`, `processing_started_at`

#### `payment.successful`

- **Trigger:** Payment transitions from `processing` to `successful`.
- **Payload:** `payment_id`, `amount_minor`, `currency`, `processed_at`

#### `payment.failed`

- **Trigger:** Payment transitions from `processing` to `failed`.
- **Payload:** `payment_id`, `amount_minor`, `currency`, `failure_reason`, `processed_at`

#### `refund.created`

- **Trigger:** Refund entity created via create refund command.
- **Payload:** `refund_id`, `payment_id`, `amount_minor`, `currency`, `created_at`

#### `refund.completed`

- **Trigger:** Refund transitions from `processing` to `completed`.
- **Payload:** `refund_id`, `payment_id`, `amount_minor`, `currency`, `processed_at`

### 4.3 Notification Mapping

Externally delivered domain event types trigger creation of a Notification Delivery Record with `status: pending`.

| Event Type | Notification Record Created | Destination |
|---|---|---|
| `payment.created` | Yes | Merchant-configured endpoint |
| `payment.processing` | No | Internal lifecycle event only |
| `payment.successful` | Yes | Merchant-configured endpoint |
| `payment.failed` | Yes | Merchant-configured endpoint |
| `refund.created` | Yes | Merchant-configured endpoint |
| `refund.completed` | Yes | Merchant-configured endpoint |

---

## 5. Command Contracts

### 5.1 Create Payment

| Aspect | Detail |
|---|---|
| **Idempotency key required** | Yes |
| **Preconditions** | Merchant is authenticated; amount > 0 in minor units; currency matches configured single currency; metadata is an object when supplied |
| **Output** | Payment entity with status `pending` |
| **Conflict (duplicate key)** | Equivalent replay returns existing payment without side effects; same key with different amount, currency, or metadata is rejected as a conflict |
| **Events emitted** | `payment.created` |

### 5.2 Process Payment

| Aspect | Detail |
|---|---|
| **Idempotency key required** | No (triggered by system; idempotency enforced by state guard) |
| **Preconditions** | Payment status is `pending` |
| **Output** | Payment transitions to `processing`, then `successful` or `failed` |
| **Events emitted** | `payment.processing`, then `payment.successful` or `payment.failed` |

### 5.3 Create Refund

| Aspect | Detail |
|---|---|
| **Idempotency key required** | Yes |
| **Preconditions** | Payment status is `successful`; no Refund exists for the Payment |
| **Output** | Refund entity with status `pending` |
| **Conflict (duplicate key)** | Return existing refund record without side effects |
| **Events emitted** | `refund.created` |

### 5.4 Process Refund

| Aspect | Detail |
|---|---|
| **Idempotency key required** | No (triggered by system; idempotency enforced by state guard) |
| **Preconditions** | Refund status is `pending`; linked payment status is `successful` |
| **Output** | Refund transitions to `processing`, then `completed` or `failed`; if `completed`, payment transitions to `refunded` |
| **Events emitted** | `refund.completed` or `refund.failed` |

---

## 6. Acceptance Matrix

| MPG-001 Criterion | Artifact Section | Status |
|---|---|---|
| Payment entity supports `pending`, `processing`, `successful`, `failed`, `refunded` | §1.1, §2.1 | ✓ |
| Refund entity supports lifecycle status tracking | §1.2, §2.2 | ✓ |
| Notification delivery record can track event type, delivery status, retry count, and failure reason | §1.3 | ✓ |
| Reconciliation record can store expected balance, actual balance, discrepancy amount, status, and report metadata | §1.4 | ✓ |
| Audit record can reference actor, action, target resource, timestamp, and immutable event details | §1.5 | ✓ |
| Domain model documents allowed state transitions | §2.1, §2.2 | ✓ |
