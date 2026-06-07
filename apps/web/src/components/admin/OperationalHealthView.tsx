import type { AdminOperationalHealth, OperationalHealthFailedOperation } from '../../api/types';

interface OperationalHealthViewProps {
  operationalHealth: AdminOperationalHealth;
  windowStart: string;
  windowEnd: string;
  onOpenOperation: (op: OperationalHealthFailedOperation) => void;
}

function formatRate(ratePercent: number | null): string {
  if (ratePercent === null) return 'N/A';
  return `${ratePercent.toFixed(1)}%`;
}

function rateStatusClass(ratePercent: number | null): string {
  if (ratePercent === null) return '';
  return ratePercent >= 95 ? 'successful' : 'failed';
}

function kindLabel(kind: string): string {
  switch (kind) {
    case 'payment': return 'Payment';
    case 'refund': return 'Refund';
    case 'notification_delivery_record': return 'Notification';
    case 'reconciliation': return 'Reconciliation';
    default: return kind;
  }
}

export default function OperationalHealthView({
  operationalHealth,
  windowStart,
  windowEnd,
  onOpenOperation,
}: OperationalHealthViewProps) {
  const {
    payment_processing_success_rate,
    notification_delivery_success_rate,
    reconciliation_completion_rate,
    recent_failed_operations,
  } = operationalHealth;

  return (
    <div>
      <div className="overview-header">
        <h2>Operational Health</h2>
        <p>
          Platform reliability signals over the rolling 24-hour health window.
        </p>
      </div>

      <div className="system-health-grid">
        <div className="status-card">
          <span className="count" style={{ fontSize: 15 }}>
            {new Date(windowStart).toLocaleString()}
          </span>
          <span className="label">Window Start (inclusive)</span>
        </div>
        <div className="status-card">
          <span className="count" style={{ fontSize: 15 }}>
            {new Date(windowEnd).toLocaleString()}
          </span>
          <span className="label">Window End (exclusive)</span>
        </div>
      </div>

      <div className="bento-grid">
        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8, fontFamily: 'var(--font-display)' }}>
            Payment Processing Success Rate
          </h3>
          <span className={`metric-count ${rateStatusClass(payment_processing_success_rate.rate_percent)}`}>
            {formatRate(payment_processing_success_rate.rate_percent)}
          </span>
          <span className="metric-subtitle">
            {payment_processing_success_rate.numerator_count} successful / {payment_processing_success_rate.denominator_count} terminal
            {payment_processing_success_rate.in_flight_count > 0 && (
              <> &middot; {payment_processing_success_rate.in_flight_count} in-flight</>
            )}
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8, fontFamily: 'var(--font-display)' }}>
            Notification Delivery Success Rate
          </h3>
          <span className={`metric-count ${rateStatusClass(notification_delivery_success_rate.rate_percent)}`}>
            {formatRate(notification_delivery_success_rate.rate_percent)}
          </span>
          <span className="metric-subtitle">
            {notification_delivery_success_rate.numerator_count} delivered / {notification_delivery_success_rate.denominator_count} terminal
            {notification_delivery_success_rate.in_flight_count > 0 && (
              <> &middot; {notification_delivery_success_rate.in_flight_count} in-flight</>
            )}
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 8, fontFamily: 'var(--font-display)' }}>
            Reconciliation Completion Rate
          </h3>
          <span className={`metric-count ${rateStatusClass(reconciliation_completion_rate.rate_percent)}`}>
            {formatRate(reconciliation_completion_rate.rate_percent)}
          </span>
          <span className="metric-subtitle">
            {reconciliation_completion_rate.numerator_count} completed / {reconciliation_completion_rate.denominator_count} accepted
            {reconciliation_completion_rate.in_flight_count > 0 && (
              <> &middot; {reconciliation_completion_rate.in_flight_count} in-flight</>
            )}
          </span>
        </div>
      </div>

      {recent_failed_operations.length === 0 ? (
        <div className="empty-state">
          <p>No failed operations in the last 24 hours.</p>
        </div>
      ) : (
        <div className="attention-panel">
          <div className="attention-panel-header">
            <h3>Recent Failed Operations</h3>
          </div>
          {recent_failed_operations.map((op) => (
            <button
              key={`${op.kind}-${op.id}`}
              className="attention-item"
              onClick={() => onOpenOperation(op)}
              style={{
                width: '100%',
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                textAlign: 'left',
                fontFamily: 'inherit',
              }}
            >
              <span className="attention-dot error" />
              <div className="attention-body">
                <span className="attention-title">
                  {kindLabel(op.kind)} &middot; {op.status}
                </span>
                <span className="attention-meta">
                  {op.reason && <>{op.reason} &middot; </>}
                  {op.amount_minor != null && op.currency && (
                    <>{(op.amount_minor / 100).toFixed(2)} {op.currency} &middot; </>
                  )}
                  {new Date(op.occurred_at).toLocaleString()}
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
