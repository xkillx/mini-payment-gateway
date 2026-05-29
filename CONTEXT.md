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
An Audit Record is an append-only historical fact about who performed which action on which resource and when.
_Avoid_: Mutable activity log

## Flagged ambiguities

- "Transaction" was used interchangeably with "Payment" in planning docs; use **Payment** as the canonical term unless referring to external processor records in future scope.
- New domain artifacts must use **Payment** terminology; "transaction" is treated as legacy wording in planning docs.
- MVP uses full refunds only; "partial refund" is future scope and must not be implied by "refunded".
- MVP refund behavior is one full Refund per successful Payment; multiple or partial refunds are future scope.

## Example dialogue

- Dev: "Can I process this payment now?"
- Domain expert: "Yes, if the payment is pending and not already processing."
- Dev: "Is this payment refunded?"
- Domain expert: "Only when the full original amount has been refunded."
