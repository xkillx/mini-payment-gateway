# 0002. Database-Enforced Append-Only Domain Events

## Status

Accepted

## Context

Domain Events are immutable facts about Payment and Refund lifecycle changes. Notification Delivery Records, Payment Status History, and future integrations may depend on these facts staying stable after they are recorded.

## Decision

`domain_events` rows will be append-only at the database level. The application may insert Domain Events and read them, but normal `UPDATE` and `DELETE` operations against existing rows must be rejected by a database trigger.

Schema migrations may still alter the `domain_events` table structure when the product evolves.

## Consequences

This keeps lifecycle facts trustworthy independently of application-layer discipline.

Operational correction must be represented by an additional Domain Event or an explicit privileged database maintenance action outside normal application workflows.

Future repository and service code must remain insert/read-only for Domain Events.
