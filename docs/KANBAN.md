# Mini Payment Gateway Kanban Tickets

Source: [PRD.md](./PRD.md)

## Board Columns

### Ready

No tickets currently assigned.

### Backlog

- [MPG-007](#mpg-007-list-search-and-filter-payments)
- [MPG-008](#mpg-008-process-payment-transaction)
- [MPG-009](#mpg-009-record-payment-events)
- [MPG-010](#mpg-010-create-refund-request)
- [MPG-011](#mpg-011-prevent-invalid-and-duplicate-refunds)
- [MPG-012](#mpg-012-view-refund-history)
- [MPG-013](#mpg-013-create-notification-event-pipeline)
- [MPG-014](#mpg-014-deliver-notifications)
- [MPG-015](#mpg-015-retry-failed-notifications)
- [MPG-016](#mpg-016-track-notification-delivery-status)
- [MPG-017](#mpg-017-run-manual-reconciliation)
- [MPG-018](#mpg-018-generate-reconciliation-report)
- [MPG-019](#mpg-019-build-merchant-dashboard)
- [MPG-020](#mpg-020-build-admin-dashboard)
- [MPG-021](#mpg-021-build-transaction-reporting)
- [MPG-022](#mpg-022-add-operational-health-view)
- [MPG-023](#mpg-023-add-mvp-test-coverage)
- [MPG-024](#mpg-024-create-linkedin-technical-writeup)

### In Progress

No tickets currently assigned.

### Done

- [MPG-001](#mpg-001-define-core-domain-model)
- [MPG-002](#mpg-002-set-up-project-foundation)
- [MPG-003](#mpg-003-implement-authentication-and-user-roles)
- [MPG-004](#mpg-004-create-audit-logging)
- [MPG-005](#mpg-005-create-payment-request)
- [MPG-006](#mpg-006-view-payment-details)

---

## MVP Tickets

### MPG-001: Define Core Domain Model

**Type:** Architecture  
**Priority:** P0  
**User:** Developer  
**Goal:** Establish the core entities and state machines for payments, refunds, events, reconciliation, and audit records.

**Acceptance Criteria**

- Payment entity supports `pending`, `processing`, `successful`, `failed`, and `refunded` states.
- Refund entity supports lifecycle status tracking.
- Notification delivery record can track event type, delivery status, retry count, and failure reason.
- Reconciliation record can store expected balance, actual balance, discrepancy amount, status, and report metadata.
- Audit record can reference the actor, action, target resource, timestamp, and immutable event details.
- Domain model documents allowed state transitions.

**Dependencies:** None

---

### MPG-002: Set Up Project Foundation

**Type:** Engineering  
**Priority:** P0  
**User:** Developer  
**Goal:** Create the application structure needed to support the MVP features.

**Acceptance Criteria**

- Application has clear modules for payments, refunds, notifications, reconciliation, audit, reporting, and administration.
- Database migration or schema setup exists for MVP entities.
- Environment configuration is documented.
- Local development run instructions are available.
- Basic error handling and request validation patterns are established.

**Dependencies:** MPG-001

---

### MPG-003: Implement Authentication and User Roles

**Type:** Security  
**Priority:** P0  
**User:** Merchant, Administrator  
**Goal:** Protect sensitive payment operations and separate merchant and administrator access.

**Acceptance Criteria**

- Users must authenticate before accessing protected routes or dashboard views.
- Merchant users can access merchant payment and refund workflows.
- Administrator users can access monitoring, reconciliation, notification, and audit views.
- Unauthorized requests return clear access errors.
- Authentication and authorization failures are audit logged.

**Dependencies:** MPG-002, MPG-004

---

### MPG-004: Create Audit Logging

**Type:** Compliance  
**Priority:** P0  
**User:** Administrator  
**Goal:** Make significant system actions traceable and protected from accidental modification.

**Acceptance Criteria**

- Audit logs are created for payment creation, payment processing, refund requests, refund completion, reconciliation execution, and administrative actions.
- Audit logs include actor, action, resource, timestamp, and details.
- Audit history can be viewed by administrators.
- Existing audit records cannot be modified through normal application workflows.
- Audit logging failures do not silently hide critical business actions.

**Dependencies:** MPG-001, MPG-002

---

### MPG-005: Create Payment Request

**Type:** Feature  
**Priority:** P0  
**User:** Merchant  
**Goal:** Allow merchants to create payment requests.

**Acceptance Criteria**

- Merchant can submit a payment request with required amount and payment metadata.
- New payments are created with `pending` status.
- Payment creation validates required fields and rejects invalid amounts.
- Payment creation generates an audit log.
- Payment creation emits a `payment.created` event.
- API response returns the payment identifier and current status.

**Dependencies:** MPG-003, MPG-004, MPG-013

---

### MPG-006: View Payment Details

**Type:** Feature  
**Priority:** P0  
**User:** Merchant, Administrator  
**Goal:** Allow users to inspect a single payment and its lifecycle.

**Acceptance Criteria**

- Authorized users can view payment amount, status, timestamps, and metadata.
- Payment detail includes status history.
- Payment detail includes related refund records when available.
- Payment detail includes related notification delivery records when available.
- Missing or unauthorized payments return clear errors.

**Dependencies:** MPG-005

---

### MPG-007: List, Search, and Filter Payments

**Type:** Feature  
**Priority:** P1  
**User:** Merchant, Administrator  
**Goal:** Allow users to find and monitor payment transactions.

**Acceptance Criteria**

- Users can list payments with pagination.
- Users can filter payments by status.
- Users can search payments by identifier or merchant reference.
- Administrators can monitor all payments.
- Merchants can only view their own payments.
- Lookup performance remains responsive for MVP data volume.

**Dependencies:** MPG-005, MPG-003

---

### MPG-008: Process Payment Transaction

**Type:** Feature  
**Priority:** P0  
**User:** System  
**Goal:** Simulate payment processing and record the transaction outcome.

**Acceptance Criteria**

- Pending payments can transition to `processing`.
- Processing payments can transition to `successful` or `failed`.
- Invalid state transitions are rejected.
- Duplicate processing attempts are prevented.
- Processing outcome and reason are recorded.
- Each status transition is audit logged.

**Dependencies:** MPG-005, MPG-004

---

### MPG-009: Record Payment Events

**Type:** Eventing  
**Priority:** P0  
**User:** System  
**Goal:** Generate consistent events for payment lifecycle changes.

**Acceptance Criteria**

- `payment.created` event is generated when a payment is created.
- `payment.successful` event is generated when a payment succeeds.
- `payment.failed` event is generated when a payment fails.
- Events include payment identifier, event type, occurred timestamp, and payload.
- Events are persisted before notification delivery is attempted.
- Event generation is idempotent for duplicate lifecycle operations.

**Dependencies:** MPG-005, MPG-008, MPG-013

---

### MPG-010: Create Refund Request

**Type:** Feature  
**Priority:** P0  
**User:** Merchant  
**Goal:** Allow merchants to request refunds for completed payments.

**Acceptance Criteria**

- Merchant can create a refund for a successful payment.
- Refund request validates amount and payment ownership.
- Refund request creates a refund record with trackable status.
- Refund request emits a `refund.created` event.
- Refund request creates an audit log.
- Refund response returns refund identifier and current status.

**Dependencies:** MPG-008, MPG-003, MPG-004, MPG-013

---

### MPG-011: Prevent Invalid and Duplicate Refunds

**Type:** Reliability  
**Priority:** P0  
**User:** Merchant, System  
**Goal:** Enforce refund business rules and prevent duplicate refund processing.

**Acceptance Criteria**

- Refunds are rejected for payments that are not successful.
- Refund amount cannot exceed the original payment amount.
- Duplicate refund requests are prevented through idempotency or unique business constraints.
- Invalid refund attempts return actionable error messages.
- Rejected refund attempts are audit logged.

**Dependencies:** MPG-010

---

### MPG-012: View Refund History

**Type:** Feature  
**Priority:** P1  
**User:** Merchant, Administrator  
**Goal:** Allow users to review refund activity and outcomes.

**Acceptance Criteria**

- Users can list refunds with pagination.
- Users can filter refunds by status.
- Refund detail shows amount, status, linked payment, timestamps, and outcome.
- Administrators can view all refunds.
- Merchants can only view refunds for their own payments.

**Dependencies:** MPG-010, MPG-011

---

### MPG-013: Create Notification Event Pipeline

**Type:** Eventing  
**Priority:** P0  
**User:** System  
**Goal:** Convert domain events into notification delivery records.

**Acceptance Criteria**

- Supported events are `payment.created`, `payment.successful`, `payment.failed`, `refund.created`, and `refund.completed`.
- Each supported event creates a notification delivery record.
- Notification records start with a pending delivery status.
- Event payload is stored or referenced for delivery.
- Duplicate notification records are prevented for the same event and destination.

**Dependencies:** MPG-001, MPG-002

---

### MPG-014: Deliver Notifications

**Type:** Feature  
**Priority:** P0  
**User:** System, Merchant  
**Goal:** Deliver payment and refund notifications to configured destinations.

**Acceptance Criteria**

- Pending notifications can be delivered by the system.
- Successful delivery marks the notification as delivered.
- Failed delivery records the failure reason.
- Delivery attempts are timestamped.
- Delivery behavior is testable without real external payment or banking integrations.

**Dependencies:** MPG-013

---

### MPG-015: Retry Failed Notifications

**Type:** Reliability  
**Priority:** P0  
**User:** Administrator, System  
**Goal:** Make notification delivery reliable when attempts fail.

**Acceptance Criteria**

- Failed notifications can be retried.
- Retry count is tracked.
- Retry result updates delivery status.
- Retry attempts preserve prior failure history.
- Administrators can trigger or observe retries for failed deliveries.

**Dependencies:** MPG-014, MPG-016

---

### MPG-016: Track Notification Delivery Status

**Type:** Admin  
**Priority:** P1  
**User:** Administrator  
**Goal:** Give administrators visibility into notification delivery outcomes.

**Acceptance Criteria**

- Administrator can view pending, delivered, failed, and retried notifications.
- Notification detail includes event type, destination, retry count, last error, and timestamps.
- Notification list can be filtered by delivery status.
- Delivery status view links back to the related payment or refund.

**Dependencies:** MPG-013, MPG-014

---

### MPG-017: Run Manual Reconciliation

**Type:** Feature  
**Priority:** P0  
**User:** Administrator  
**Goal:** Allow administrators to compare transaction records against financial records.

**Acceptance Criteria**

- Administrator can start a manual reconciliation process.
- Process calculates expected balances from successful payments and refunds.
- Process compares expected balances against provided actual balances.
- Discrepancies are detected automatically.
- Reconciliation execution creates an audit log.

**Dependencies:** MPG-005, MPG-010, MPG-004

---

### MPG-018: Generate Reconciliation Report

**Type:** Reporting  
**Priority:** P1  
**User:** Administrator  
**Goal:** Produce a durable record of reconciliation results.

**Acceptance Criteria**

- Reconciliation report includes expected balance, actual balance, discrepancy amount, status, and execution timestamp.
- Historical reconciliation reports are available to administrators.
- Reports identify mismatched records when possible.
- Reports can be opened from the admin dashboard.

**Dependencies:** MPG-017

---

### MPG-019: Build Merchant Dashboard

**Type:** UI  
**Priority:** P1  
**User:** Merchant  
**Goal:** Give merchants a focused view for payment and refund workflows.

**Acceptance Criteria**

- Dashboard shows payment overview.
- Dashboard shows refund overview.
- Merchant can search transactions.
- Merchant can track payment and refund statuses.
- Merchant can access payment creation and refund request actions.
- Dashboard only displays data belonging to the authenticated merchant.

**Dependencies:** MPG-005, MPG-006, MPG-007, MPG-010, MPG-012, MPG-003

---

### MPG-020: Build Admin Dashboard

**Type:** UI  
**Priority:** P1  
**User:** Administrator  
**Goal:** Give administrators operational visibility across the platform.

**Acceptance Criteria**

- Dashboard shows system health overview.
- Dashboard shows transaction monitoring.
- Dashboard shows refund monitoring.
- Dashboard shows notification monitoring.
- Dashboard links to reconciliation reports.
- Dashboard links to audit logs.

**Dependencies:** MPG-007, MPG-012, MPG-016, MPG-018, MPG-004, MPG-022

---

### MPG-021: Build Transaction Reporting

**Type:** Reporting  
**Priority:** P2  
**User:** Administrator  
**Goal:** Summarize platform activity and payment trends.

**Acceptance Criteria**

- Report shows total payments.
- Report shows successful payments.
- Report shows failed payments.
- Report shows refund activity.
- Report shows simple payment trend data.
- Report data can be filtered by date range.

**Dependencies:** MPG-005, MPG-008, MPG-010

---

### MPG-022: Add Operational Health View

**Type:** Observability  
**Priority:** P2  
**User:** Administrator  
**Goal:** Surface platform health and reliability signals.

**Acceptance Criteria**

- Health view shows transaction processing success rate.
- Health view shows notification delivery success rate.
- Health view shows reconciliation completion rate.
- Health view shows recent failed operations.
- Health data is available to administrators only.

**Dependencies:** MPG-008, MPG-014, MPG-017, MPG-003

---

### MPG-023: Add MVP Test Coverage

**Type:** Quality  
**Priority:** P0  
**User:** Developer  
**Goal:** Protect core payment gateway behavior from regression.

**Acceptance Criteria**

- Tests cover valid and invalid payment creation.
- Tests cover payment status transitions and duplicate processing prevention.
- Tests cover refund business rules and duplicate refund prevention.
- Tests cover notification creation, delivery failure, and retry behavior.
- Tests cover reconciliation discrepancy detection.
- Tests cover role-based access for merchant and administrator workflows.

**Dependencies:** MPG-005, MPG-008, MPG-011, MPG-015, MPG-017, MPG-003

---

### MPG-024: Create LinkedIn Technical Writeup

**Type:** Documentation  
**Priority:** P2  
**User:** Developer  
**Goal:** Turn the project into clear technical content that explains the fintech concepts demonstrated.

**Acceptance Criteria**

- Writeup summarizes the mini payment gateway project.
- Writeup explains transaction lifecycle management.
- Writeup explains notification retry handling.
- Writeup explains why reconciliation matters.
- Writeup highlights backend engineering, distributed systems, reliability, and product thinking skills.

**Dependencies:** MVP feature completion

---

## Future Backlog

### MPG-025: Support Partial Refunds

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 2

**Acceptance Criteria**

- Multiple partial refunds can be issued against one successful payment.
- Total refunded amount cannot exceed the original payment amount.
- Payment status reflects partial versus full refund state.

---

### MPG-026: Add Scheduled Reconciliation

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 2

**Acceptance Criteria**

- Reconciliation can run automatically on a schedule.
- Scheduled runs create the same report format as manual runs.
- Failed scheduled runs are visible to administrators.

---

### MPG-027: Add Advanced Reporting

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 2

**Acceptance Criteria**

- Reporting supports richer filters and trend breakdowns.
- Administrators can inspect payment, refund, notification, and reconciliation metrics together.
- Reports can be exported or shared.

---

### MPG-028: Add Merchant Self-Service Settings

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 2

**Acceptance Criteria**

- Merchants can manage notification destination settings.
- Merchants can update basic account preferences.
- Setting changes are audit logged.

---

### MPG-029: Add Multi-Currency Support

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 3

**Acceptance Criteria**

- Payments can be created with supported currency codes.
- Refunds preserve the original payment currency.
- Reporting and reconciliation account for currency.

---

### MPG-030: Add Fraud Detection and Risk Scoring

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 3

**Acceptance Criteria**

- Payments can receive a risk score.
- High-risk payments can be flagged for review.
- Fraud-related decisions are auditable.

---

### MPG-031: Add Chargeback Management

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 3

**Acceptance Criteria**

- Chargebacks can be recorded against successful payments.
- Chargeback status can be tracked.
- Chargeback activity appears in reporting and audit logs.

---

### MPG-032: Add Settlement Engine

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 3

**Acceptance Criteria**

- Successful payments can be grouped into settlement batches.
- Settlement status can be tracked.
- Reconciliation accounts for settlement records.

---

### MPG-033: Add Multi-Tenant Architecture

**Type:** Enhancement  
**Priority:** Future  
**Target:** Version 3

**Acceptance Criteria**

- Data is isolated by tenant.
- Administrator views can filter by tenant.
- Merchant access is limited to tenant-scoped data.

