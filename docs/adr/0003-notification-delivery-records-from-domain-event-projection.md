# 0003. Notification Delivery Records from Domain Event Projection

Notification Delivery Records will be created by a worker-run projection from committed Domain Events rather than inline in Payment and Refund write transactions. This keeps Domain Event recording separate from notification pipeline work, allows safe backfill of supported events, and lets duplicate prevention be enforced with idempotent inserts plus database uniqueness for each event and destination.
