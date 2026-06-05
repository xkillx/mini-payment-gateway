# Notification Retry Generations

Notification Delivery Records keep `attempt_count` as the cumulative number of actual outbound Notification Delivery Attempts, but automatic retry exhaustion is evaluated within the current retry generation. Administrator retry overrides start a new generation instead of resetting `attempt_count`, so operators can requeue a terminal failed delivery without erasing history or making `NOTIFICATION_MAX_ATTEMPTS` block the override immediately.

This rejects a lifetime retry budget because a record that had already exhausted `NOTIFICATION_MAX_ATTEMPTS` would fail again without a meaningful new delivery window. It also rejects resetting `attempt_count` because the glossary defines it as the number of recorded Notification Delivery Attempts so far.
