# Mini Payment Gateway

This context defines the business language for a simplified payment gateway so product, engineering, and operations use the same terms.

## Language

**Payment**:
A Payment is the core business record for one merchant charge request and its lifecycle outcome.
_Avoid_: Transaction, payment transaction, payment request

**Payment Metadata**:
Payment Metadata is merchant-supplied creation context attached to a Payment for merchant reconciliation and lookup. It is not Payment lifecycle state and does not change payment processing outcome.
_Avoid_: Payment notes, mutable payment details, processing state

**Merchant Reference**:
A Merchant Reference is a merchant-supplied lookup value that connects a Payment to the merchant's own records. It is not the gateway Payment identifier and does not identify the create command.
_Avoid_: Payment ID, Idempotency Key, transaction reference

**Idempotency Key**:
An Idempotency Key is a merchant-supplied value that identifies one create command so a repeated submission does not create a duplicate Payment or Refund.
_Avoid_: Request ID, payment ID, retry token

**Configured Currency**:
Configured Currency is the single currency the gateway accepts for Payment and Refund amounts in MVP.
_Avoid_: Merchant-selected currency, multi-currency amount

**Payment Amount**:
A Payment Amount is the value of a Payment expressed in minor units of the Configured Currency.
_Avoid_: Decimal amount, display amount, converted amount

**Payment Status**:
Payment Status expresses where a Payment is in its lifecycle: pending, processing, successful, failed, or refunded.
_Avoid_: Transaction status

**Payment Status History**:
Payment Status History is the ordered record of Payment Statuses a Payment has reached during its lifecycle, including status changes caused by completed Refunds.
_Avoid_: Transaction history, audit history

**Payment Processing Outcome**:
A Payment Processing Outcome is the System Principal's result for processing a Payment: successful or failed. A failed outcome may include a failure reason; merchant-supplied Payment Metadata does not decide the outcome.
_Avoid_: Merchant-selected outcome, metadata-driven outcome

**Refund**:
A Refund is a reversal of value against one successful Payment.
_Avoid_: Reversal transaction, chargeback

**Rejected Refund Attempt**:
A Rejected Refund Attempt is an authenticated Merchant attempt to create a Refund that passes request-shape validation but violates refund business rules, so no Refund is created.
_Avoid_: Failed refund, refund.failed, declined refund

**Refunded Payment**:
A Refunded Payment is a Payment whose original amount has been fully reversed by refunds.
_Avoid_: Partially refunded payment

**Refund Status**:
Refund Status expresses where a Refund is in its lifecycle: pending, processing, completed, or failed.
_Avoid_: Refund state machine (as a replacement for status term)

**Notification Delivery Record**:
A Notification Delivery Record tracks delivery of one domain event to one destination and its outcome.
_Avoid_: Webhook attempt log, notification job

**Notification Destination**:
A Notification Destination is a Merchant Account's configured endpoint for receiving externally delivered Domain Events. One Domain Event may create one Notification Delivery Record per active Notification Destination.
_Avoid_: Webhook setting, callback URL, notification job target

**Notification Delivery Payload**:
A Notification Delivery Payload is the externally delivered representation of one Domain Event for a Notification Destination. It carries the Domain Event identity, type, occurrence time, Resource identity, schema version, and event-specific payload.
_Avoid_: Ping payload, raw event payload, webhook body

**Notification Delivery Attempt**:
A Notification Delivery Attempt is one System Principal try to deliver a Notification Delivery Payload to a Notification Destination.
_Avoid_: Retry count, webhook call, notification job run

**Notification Delivery Error**:
A Notification Delivery Error is the operational error observed on the most recent failed Notification Delivery Attempt.
_Avoid_: Payment failure reason, refund failure reason, business decline reason

**Domain Event**:
A Domain Event is an immutable fact that a meaningful Payment or Refund lifecycle change occurred. Domain Events may be used to create Notification Delivery Records, but they are not themselves delivery attempts.
_Avoid_: Webhook, notification, audit record

**Reconciliation**:
Reconciliation is a record of one balance comparison run for one Configured Currency over one explicit Reconciliation Window.
_Avoid_: Settlement run, accounting sync

**Manual Reconciliation**:
A Manual Reconciliation is an Administrator-initiated Reconciliation across all Merchant Accounts for one Configured Currency and Reconciliation Window.
_Avoid_: Merchant reconciliation, scheduled reconciliation

**Reconciliation Window**:
A Reconciliation Window is the time range whose successful Payment outcomes and completed Refund outcomes are included in one Reconciliation.
_Avoid_: Report date, settlement period

**Expected Reconciliation Total**:
An Expected Reconciliation Total is the gateway-computed net financial total for a Reconciliation Window: successful Payments less completed Refunds.
_Avoid_: Actual balance, processor total

**Actual Reconciliation Total**:
An Actual Reconciliation Total is the administrator-provided financial total being compared with the Expected Reconciliation Total for the same Configured Currency and Reconciliation Window.
_Avoid_: Expected balance, computed total

**Reconciliation Status**:
Reconciliation Status expresses the outcome of a Reconciliation run: matched, mismatched, or error.
_Avoid_: Reconciliation processing state

**Reconciliation Report**:
A Reconciliation Report is the administrator-facing detail view of a Reconciliation and any identifiable mismatched records. In MVP, identifiable mismatched records means the gateway-side Payments and Refunds included in the Reconciliation Window, not records proven to disagree with an external line-item source.
_Avoid_: Manual reconciliation run, settlement file

**Audit Record**:
An Audit Record is an append-only historical fact about who performed or attempted which action on which resource and when.
_Avoid_: Mutable activity log

**Actor**:
An Actor is an authenticated party that performs actions in the gateway and can be referenced by an Audit Record.
_Avoid_: User, account

**Resource**:
A Resource is the domain or operational object that an Audit Record says was acted upon.
_Avoid_: Target

**Merchant**:
A Merchant is an Actor role scoped to one Merchant Account and its Payments and Refunds.
_Avoid_: Customer, seller account

**Merchant Account**:
A Merchant Account is the business owner of Payments and Refunds.
_Avoid_: Merchant user, customer account

**Administrator**:
An Administrator is an Actor role with platform operations responsibility across merchants.
_Avoid_: Admin user, superuser

**Administrative Action**:
An Administrative Action is an administrator-initiated state-changing platform operation.
_Avoid_: Administrator read, admin page view

**Unknown Principal**:
An Unknown Principal is a party whose identity could not be authenticated but whose access attempt may still be recorded.
_Avoid_: Anonymous user, guest actor

**System Principal**:
A System Principal is the gateway itself acting without an authenticated human or merchant Actor.
_Avoid_: System user, service account

## Flagged ambiguities

- "Transaction" was used interchangeably with "Payment" in planning docs; use **Payment** as the canonical term unless referring to external processor records in future scope.
- New domain artifacts must use **Payment** terminology; "transaction" is treated as legacy wording in planning docs.
- "Transaction history" in planning docs means **Payment Status History** when discussing a Payment's lifecycle statuses.
- "Completed payment" in planning docs means a **Payment** with **Payment Status** `successful`; Payment does not have a `completed` status.
- "Payment metadata" in planning docs means **Payment Metadata**, not a separate Payment Request object or mutable lifecycle details. Payment Metadata is part of the create command identified by an **Idempotency Key**.
- A **Payment Processing Outcome** is decided by the **System Principal**; do not infer it from **Payment Metadata**.
- An **Idempotency Key** identifies a create command from the Merchant; it is not the Payment identifier and does not replace the Actor or Merchant Account identity.
- **Configured Currency** is a gateway-level MVP constraint; Payments in any other currency are rejected rather than converted.
- "Amount" in planning docs means **Payment Amount** in minor units; decimal or display-form money is not accepted at the command boundary.
- MVP uses full refunds only; "partial refund" is future scope and must not be implied by "refunded".
- MVP refund behavior is one Refund record per successful Payment; retrying after a failed Refund and multiple or partial refunds are future scope.
- "Refund request" in planning docs means the Merchant action that creates a **Refund**; do not introduce a separate Refund Request domain object.
- "Refund requested" in planning docs and legacy code means the **Refund** was created; use **refund.created** for the action and Domain Event name.
- A **Rejected Refund Attempt** is not a **Refund** with **Refund Status** `failed`; `failed` belongs to refund processing after a Refund exists.
- "Refund outcome" in refund history planning means **Refund Status**; do not introduce a separate Refund Outcome concept for MVP.
- Creating a **Refund** is a Merchant action; Administrator refund access is for monitoring and inspection, not initiation.
- "User" appears in planning docs, but **Actor** is the canonical term for an authenticated party. Use **Merchant** or **Administrator** when the role matters.
- **Merchant** as an Actor role is distinct from the **Merchant Account** that owns Payments, even when a single MVP actor represents a single Merchant Account.
- Authentication failures may involve an **Unknown Principal**, not an **Actor**. Do not call an unauthenticated party an Actor.
- Authentication failures may include unverified identity claims, but those claims do not establish an **Actor**.
- Planning docs sometimes call the audited object a "target"; use **Resource** as the canonical term when describing what an Audit Record refers to.
- System-initiated work uses a **System Principal**, not an **Actor** or **Unknown Principal**.
- Emitting a **Domain Event** means recording the immutable domain fact; creating a **Notification Delivery Record** for that event is separate notification pipeline work.
- `payment.processing` is a **Domain Event** for the Payment lifecycle even when it is not externally delivered; not every **Domain Event** creates a **Notification Delivery Record**.
- "Notification payload" in planning docs means a **Notification Delivery Payload**, not only the event-specific **Domain Event** payload or a test ping body.
- "Failed delivery records the failure reason" in planning docs means recording a **Notification Delivery Error**; do not use Payment or Refund **failure_reason** language for notification transport outcomes.
- "Retry count" in planning docs means the **Notification Delivery Record** attempt count: the number of recorded Notification Delivery Attempts so far, not a separate retry-only counter.
- An Administrator retry request requeues a **Notification Delivery Record**; it is an **Administrative Action**, not a **Notification Delivery Attempt** until the **System Principal** tries delivery again.
- "Retried notifications" in planning docs means **Notification Delivery Records** with retry activity visible in their attempt history and retry generation; it is not a separate **Notification Status**.
- When a **Notification Delivery Record** is created from a Refund **Domain Event**, the direct **Resource** is the Refund and the owning Payment remains the related Payment context.
- A **Notification Delivery Payload** does not introduce a universal top-level **Idempotency Key**; command-specific Idempotency Keys appear only when they are part of the delivered event-specific payload.
- "Merchant reference" in planning docs means **Merchant Reference**, not the gateway Payment identifier, Idempotency Key, or arbitrary Payment Metadata.
- "Payment identifier" in list/search planning means the Payment ID, not the **Idempotency Key** used to identify a create command.
- A **Reconciliation** compares one **Expected Reconciliation Total** with one **Actual Reconciliation Total** for the same **Configured Currency** and **Reconciliation Window**; it is not an all-time or cross-currency total.
- **Manual Reconciliation** is platform-wide in MVP; merchant-level reconciliation is future scope.
- A **Reconciliation Window** uses financial outcome time: when a Payment became successful or a Refund became completed. Window start is inclusive and window end is exclusive.
- A **Reconciliation Window** is expressed as exact timestamp instants, not business dates or timezone-specific calendar days.
- An **Actual Reconciliation Total** is a net amount in minor units after refunds are subtracted. It may be zero or negative.
- **Manual Reconciliation** produces a run-level Reconciliation result. Item-level mismatch detail belongs to a **Reconciliation Report**.
- "Mismatched records" in MPG-018 means gateway-side included Payments and Refunds that explain the Expected Reconciliation Total; it does not imply external line-item matching in MVP.
- A valid accepted **Manual Reconciliation** should create a **Reconciliation** even when the comparison cannot complete, using **Reconciliation Status** `error` when possible. Invalid requests are rejected without creating a Reconciliation.
- A **Reconciliation** is matched only when the discrepancy is exactly zero minor units; any non-zero discrepancy is mismatched.
- Every accepted **Manual Reconciliation** that creates a **Reconciliation** must also create an **Audit Record** that references that Reconciliation.
- An Administrator may run **Manual Reconciliation** more than once for the same Configured Currency and Reconciliation Window. Each run is a separate historical Reconciliation.

## Example dialogue

- Dev: "Can I process this payment now?"
- Domain expert: "Yes, if the payment is pending and not already processing."
- Dev: "Can the merchant metadata make this payment fail?"
- Domain expert: "No. A Payment Processing Outcome is decided by the System Principal, not by Payment Metadata."
- Dev: "Is this payment refunded?"
- Domain expert: "Only when the full original amount has been refunded."
- Dev: "The merchant tried to refund a pending payment. Is that a failed refund?"
- Domain expert: "No. That is a Rejected Refund Attempt because no Refund exists yet."
- Dev: "Can this actor see every payment?"
- Domain expert: "Only if the actor is an Administrator. A Merchant sees Payments for its own Merchant Account."
- Dev: "Who is recorded when authentication fails before we know the party?"
- Domain expert: "Record an Unknown Principal, because no Actor has been authenticated yet."
- Dev: "Where does a Notification Delivery Record send an event?"
- Domain expert: "To the Merchant Account's active Notification Destination; the record tracks delivery of that Domain Event to that destination."
- Dev: "Should the merchant receive only the Domain Event payload?"
- Domain expert: "No. Send a Notification Delivery Payload so the merchant can identify the Domain Event and then read its event-specific payload."
- Dev: "Can this Manual Reconciliation use all payments since launch?"
- Domain expert: "No. It compares one Expected Reconciliation Total with one Actual Reconciliation Total for a specific Reconciliation Window."
