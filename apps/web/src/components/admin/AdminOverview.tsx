import type { AdminDashboardSummary } from '../../api/types';

interface AdminOverviewProps {
  summary: AdminDashboardSummary;
}

export default function AdminOverview({ summary }: AdminOverviewProps) {
  const {
    api_status,
    window_start,
    window_end,
    payment_overview,
    refund_overview,
    notification_overview,
    reconciliation_overview,
    audit_overview,
  } = summary;

  return (
    <div>
      <div className="status-count-grid" style={{ marginBottom: 16 }}>
        <div className={`status-card ${api_status === 'reachable' ? 'successful' : 'failed'}`}>
          <span className="count">{api_status === 'reachable' ? 'UP' : 'DOWN'}</span>
          <span className="label">API Status</span>
        </div>
        <div className="status-card">
          <span className="count" style={{ fontSize: 16, fontFamily: 'monospace' }}>
            {new Date(window_start).toLocaleString()}
          </span>
          <span className="label">Window Start</span>
        </div>
        <div className="status-card">
          <span className="count" style={{ fontSize: 16, fontFamily: 'monospace' }}>
            {new Date(window_end).toLocaleString()}
          </span>
          <span className="label">Window End (24h)</span>
        </div>
      </div>

      <div className="section">
        <div className="section-header">
          <h3>Payments (24h)</h3>
        </div>
        <div className="status-count-grid">
          <div className="status-card pending">
            <span className="count">{payment_overview.status_counts.pending}</span>
            <span className="label">Pending</span>
          </div>
          <div className="status-card processing">
            <span className="count">{payment_overview.status_counts.processing}</span>
            <span className="label">Processing</span>
          </div>
          <div className="status-card successful">
            <span className="count">{payment_overview.status_counts.successful}</span>
            <span className="label">Successful</span>
          </div>
          <div className="status-card failed">
            <span className="count">{payment_overview.status_counts.failed}</span>
            <span className="label">Failed</span>
          </div>
          <div className="status-card refunded">
            <span className="count">{payment_overview.status_counts.refunded}</span>
            <span className="label">Refunded</span>
          </div>
        </div>
        {payment_overview.recent_failed_payments.length > 0 && (
          <div className="table-wrapper" style={{ marginTop: 12 }}>
            <table>
              <thead>
                <tr>
                  <th>Payment ID</th>
                  <th>Amount</th>
                  <th>Failure Reason</th>
                  <th>Updated</th>
                </tr>
              </thead>
              <tbody>
                {payment_overview.recent_failed_payments.map((p) => (
                  <tr key={p.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>{p.id.substring(0, 8)}...</td>
                    <td>{(p.amount_minor / 100).toFixed(2)} {p.currency}</td>
                    <td style={{ color: 'var(--color-error)', fontSize: 13 }}>
                      {p.failure_reason || '-'}
                    </td>
                    <td style={{ fontSize: 13 }}>{new Date(p.updated_at).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="section">
        <div className="section-header">
          <h3>Refunds (24h)</h3>
        </div>
        <div className="status-count-grid">
          <div className="status-card pending">
            <span className="count">{refund_overview.status_counts.pending}</span>
            <span className="label">Pending</span>
          </div>
          <div className="status-card processing">
            <span className="count">{refund_overview.status_counts.processing}</span>
            <span className="label">Processing</span>
          </div>
          <div className="status-card completed">
            <span className="count">{refund_overview.status_counts.completed}</span>
            <span className="label">Completed</span>
          </div>
          <div className="status-card failed">
            <span className="count">{refund_overview.status_counts.failed}</span>
            <span className="label">Failed</span>
          </div>
        </div>
        {refund_overview.recent_failed_refunds.length > 0 && (
          <div className="table-wrapper" style={{ marginTop: 12 }}>
            <table>
              <thead>
                <tr>
                  <th>Refund ID</th>
                  <th>Amount</th>
                  <th>Updated</th>
                </tr>
              </thead>
              <tbody>
                {refund_overview.recent_failed_refunds.map((r) => (
                  <tr key={r.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>{r.id.substring(0, 8)}...</td>
                    <td>{(r.amount_minor / 100).toFixed(2)} {r.currency}</td>
                    <td style={{ fontSize: 13 }}>{new Date(r.updated_at).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="section">
        <div className="section-header">
          <h3>Notifications</h3>
        </div>
        <div className="status-count-grid">
          <div className="status-card pending">
            <span className="count">{notification_overview.status_counts.pending}</span>
            <span className="label">Pending</span>
          </div>
          <div className="status-card processing">
            <span className="count">{notification_overview.status_counts.processing}</span>
            <span className="label">Processing</span>
          </div>
          <div className="status-card successful">
            <span className="count">{notification_overview.status_counts.delivered}</span>
            <span className="label">Delivered</span>
          </div>
          <div className="status-card failed">
            <span className="count">{notification_overview.status_counts.failed}</span>
            <span className="label">Failed</span>
          </div>
        </div>
        {notification_overview.recent_failed_notifications.length > 0 && (
          <div className="table-wrapper" style={{ marginTop: 12 }}>
            <table>
              <thead>
                <tr>
                  <th>Record ID</th>
                  <th>Event Type</th>
                  <th>Destination</th>
                  <th>Error</th>
                  <th>Updated</th>
                </tr>
              </thead>
              <tbody>
                {notification_overview.recent_failed_notifications.map((n) => (
                  <tr key={n.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>{n.id.substring(0, 8)}...</td>
                    <td style={{ fontSize: 13 }}>{n.event_type}</td>
                    <td style={{ fontSize: 13, maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {n.destination_url}
                    </td>
                    <td style={{ color: 'var(--color-error)', fontSize: 13 }}>{n.last_error || '-'}</td>
                    <td style={{ fontSize: 13 }}>{new Date(n.updated_at).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="section">
        <div className="section-header">
          <h3>Reconciliation (24h)</h3>
        </div>
        <div className="status-count-grid">
          <div className="status-card successful">
            <span className="count">{reconciliation_overview.status_counts.matched}</span>
            <span className="label">Matched</span>
          </div>
          <div className="status-card failed">
            <span className="count">{reconciliation_overview.status_counts.mismatched}</span>
            <span className="label">Mismatched</span>
          </div>
          <div className="status-card pending">
            <span className="count">{reconciliation_overview.status_counts.error}</span>
            <span className="label">Error</span>
          </div>
        </div>
        {reconciliation_overview.recent_attention_reconciliations.length > 0 && (
          <div className="table-wrapper" style={{ marginTop: 12 }}>
            <table>
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Status</th>
                  <th>Discrepancy</th>
                  <th>Window</th>
                  <th>Created</th>
                </tr>
              </thead>
              <tbody>
                {reconciliation_overview.recent_attention_reconciliations.map((r) => (
                  <tr key={r.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>{r.id.substring(0, 8)}...</td>
                    <td>
                      <span className={`status-badge ${r.status === 'mismatched' ? 'failed' : 'pending'}`}>
                        {r.status}
                      </span>
                    </td>
                    <td>{(r.discrepancy_minor / 100).toFixed(2)} {r.currency}</td>
                    <td style={{ fontSize: 13 }}>
                      {new Date(r.window_start).toLocaleString()} - {new Date(r.window_end).toLocaleString()}
                    </td>
                    <td style={{ fontSize: 13 }}>{new Date(r.created_at).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>

      <div className="section">
        <div className="section-header">
          <h3>Recent Attention Audit Records (24h)</h3>
        </div>
        {audit_overview.recent_attention_audit_records.length === 0 ? (
          <div className="empty-state" style={{ padding: 16 }}>
            <p>No attention-worthy audit records in the last 24 hours.</p>
          </div>
        ) : (
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Action</th>
                  <th>Actor</th>
                  <th>Resource</th>
                  <th>Occurred</th>
                </tr>
              </thead>
              <tbody>
                {audit_overview.recent_attention_audit_records.map((r) => (
                  <tr key={r.id}>
                    <td style={{ fontSize: 13 }}>{r.action}</td>
                    <td style={{ fontSize: 13 }}>{r.actor_type}</td>
                    <td style={{ fontSize: 13 }}>{r.resource_type}/{r.resource_id}</td>
                    <td style={{ fontSize: 13 }}>{new Date(r.occurred_at).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
