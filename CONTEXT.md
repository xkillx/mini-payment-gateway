# Mini Payment Gateway

This context defines the business language for a simplified payment gateway so product, engineering, and operations use the same terms.

## Language

**Payment**:
A Payment is the core business record for one merchant charge request and its lifecycle outcome.
_Avoid_: Transaction, payment transaction, payment request

**Payment Status**:
Payment Status expresses where a Payment is in its lifecycle: pending, processing, successful, failed, or refunded.
_Avoid_: Transaction status

**Refund**:
A Refund is a reversal of value against one successful Payment.
_Avoid_: Reversal transaction, chargeback

**Refunded Payment**:
A Refunded Payment is a Payment whose original amount has been fully reversed by refunds.
_Avoid_: Partially refunded payment

**Refund Status**:
Refund Status expresses where a Refund is in its lifecycle: pending, processing, completed, or failed.
_Avoid_: Refund state machine (as a replacement for status term)

**Notification Delivery Record**:
A Notification Delivery Record tracks delivery of one domain event to one destination and its outcome.
_Avoid_: Webhook attempt log, notification job

**Reconciliation**:
Reconciliation is a record of one balance comparison run between expected and actual financial totals.
_Avoid_: Settlement run, accounting sync

**Reconciliation Status**:
Reconciliation Status expresses the outcome of a Reconciliation run: matched, mismatched, or error.
_Avoid_: Reconciliation processing state

**Audit Record**:
An Audit Record is an append-only historical fact about who performed or attempted which action on which resource and when.
_Avoid_: Mutable activity log

**Actor**:
An Actor is an authenticated party that performs actions in the gateway and can be referenced by an Audit Record.
_Avoid_: User, account

**Merchant**:
A Merchant is an Actor role scoped to one Merchant Account and its Payments and Refunds.
_Avoid_: Customer, seller account

**Merchant Account**:
A Merchant Account is the business owner of Payments and Refunds.
_Avoid_: Merchant user, customer account

**Administrator**:
An Administrator is an Actor role with platform operations responsibility across merchants.
_Avoid_: Admin user, superuser

**Unknown Principal**:
An Unknown Principal is a party whose identity could not be authenticated but whose access attempt may still be recorded.
_Avoid_: Anonymous user, guest actor

## Flagged ambiguities

- "Transaction" was used interchangeably with "Payment" in planning docs; use **Payment** as the canonical term unless referring to external processor records in future scope.
- New domain artifacts must use **Payment** terminology; "transaction" is treated as legacy wording in planning docs.
- MVP uses full refunds only; "partial refund" is future scope and must not be implied by "refunded".
- MVP refund behavior is one full Refund per successful Payment; multiple or partial refunds are future scope.
- "User" appears in planning docs, but **Actor** is the canonical term for an authenticated party. Use **Merchant** or **Administrator** when the role matters.
- **Merchant** as an Actor role is distinct from the **Merchant Account** that owns Payments, even when a single MVP actor represents a single Merchant Account.
- Authentication failures may involve an **Unknown Principal**, not an **Actor**. Do not call an unauthenticated party an Actor.

## Example dialogue

- Dev: "Can I process this payment now?"
- Domain expert: "Yes, if the payment is pending and not already processing."
- Dev: "Is this payment refunded?"
- Domain expert: "Only when the full original amount has been refunded."
- Dev: "Can this actor see every payment?"
- Domain expert: "Only if the actor is an Administrator. A Merchant sees Payments for its own Merchant Account."
- Dev: "Who is recorded when authentication fails before we know the party?"
- Domain expert: "Record an Unknown Principal, because no Actor has been authenticated yet."
