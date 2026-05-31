# 0001. Database-Enforced Append-Only Audit Records

## Status

Accepted

## Context

Audit Records are compliance evidence for significant gateway actions. Application code can avoid exposing update or delete workflows, but future code paths, maintenance scripts, or accidental repository methods could still mutate existing audit rows if the database permits it.

The product goal for MPG-004 is to make significant system actions traceable and protected from accidental modification.

## Decision

`audit_records` rows will be append-only at the database level. The application may insert Audit Records and read them, but normal `UPDATE` and `DELETE` operations against existing rows must be rejected by a database trigger.

Schema migrations may still alter the `audit_records` table structure when the product evolves.

## Consequences

This improves compliance confidence because traceability does not depend only on application-layer discipline.

Operational repair becomes more deliberate. Corrective changes must be represented by additional records or explicit privileged database maintenance outside normal application workflows.

Future repository and service code must remain insert/read-only for Audit Records.
