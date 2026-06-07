import { useState } from 'react';
import type { AdminDashboardSummary, Reconciliation } from '../../api/types';
import RunReconciliationDialog from './RunReconciliationDialog';

interface AdminOverviewProps {
  summary: AdminDashboardSummary;
  onRefresh: () => void;
}

export default function AdminOverview({ summary, onRefresh }: AdminOverviewProps) {
  const [showReconciliation, setShowReconciliation] = useState(false);

  const {
    api_status,
    window_start,
    window_end,
    configured_currency,
    payment_overview,
    refund_overview,
    notification_overview,
    reconciliation_overview,
    audit_overview,
  } = summary;

  const totalPayments =
    payment_overview.status_counts.pending +
    payment_overview.status_counts.processing +
    payment_overview.status_counts.successful +
    payment_overview.status_counts.failed +
    payment_overview.status_counts.refunded;

  const totalRefunds =
    refund_overview.status_counts.pending +
    refund_overview.status_counts.processing +
    refund_overview.status_counts.completed +
    refund_overview.status_counts.failed;

  const totalNotifications =
    notification_overview.status_counts.pending +
    notification_overview.status_counts.processing +
    notification_overview.status_counts.delivered +
    notification_overview.status_counts.failed;

  const totalReconciliations =
    reconciliation_overview.status_counts.matched +
    reconciliation_overview.status_counts.mismatched +
    reconciliation_overview.status_counts.error;

  return (
    <div>
      <div className="overview-header">
        <h2>System Health Overview</h2>
        <p>Platform-wide operational signals for the last 24 hours.</p>
      </div>

      <div className="system-health-grid">
        <div className={`status-card ${api_status === 'reachable' ? 'successful' : 'failed'}`}>
          <span className="count">{api_status === 'reachable' ? 'UP' : 'DOWN'}</span>
          <span className="label">API Status</span>
        </div>
        <div className="status-card">
          <span className="count" style={{ fontSize: 15 }}>
            {new Date(window_start).toLocaleString()}
          </span>
          <span className="label">24h Window Start</span>
        </div>
        <div className="status-card">
          <span className="count" style={{ fontSize: 15 }}>
            {new Date(window_end).toLocaleString()}
          </span>
          <span className="label">24h Window End</span>
        </div>
      </div>

      <div className="bento-grid">
        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <div className="metric-icon primary">
            <span style={{ fontSize: 18, fontWeight: 700 }}>{configured_currency}</span>
          </div>
          <span className="metric-count">{totalPayments}</span>
          <span className="metric-label">Payments (24h)</span>
          <span className="metric-subtitle">
            {payment_overview.status_counts.successful} successful, {payment_overview.status_counts.failed} failed
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <div className="metric-icon warning">
            <span style={{ fontSize: 18, fontWeight: 700 }}>{configured_currency}</span>
          </div>
          <span className="metric-count">{totalRefunds}</span>
          <span className="metric-label">Refunds (24h)</span>
          <span className="metric-subtitle">
            {refund_overview.status_counts.completed} completed, {refund_overview.status_counts.failed} failed
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <div className="metric-icon success">
            <span style={{ fontSize: 18 }}>&#9993;</span>
          </div>
          <span className="metric-count">{totalNotifications}</span>
          <span className="metric-label">Notifications</span>
          <span className="metric-subtitle">
            {notification_overview.status_counts.delivered} delivered, {notification_overview.status_counts.failed} failed
          </span>
        </div>
      </div>

      <div className="bento-grid">
        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <div className="metric-icon success">
            <span style={{ fontSize: 18 }}>&#9878;</span>
          </div>
          <span className="metric-count">{totalReconciliations}</span>
          <span className="metric-label">Reconciliations</span>
          <span className="metric-subtitle">
            {reconciliation_overview.status_counts.matched} matched, {reconciliation_overview.status_counts.mismatched} mismatched, {reconciliation_overview.status_counts.error} error
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4', display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center' }}>
          <button className="btn btn-primary" onClick={() => setShowReconciliation(true)} style={{ width: '100%' }}>
            <span style={{ fontSize: 18, marginRight: 8 }}>&#8635;</span>
            Run Manual Reconciliation
          </button>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))', gap: 16, marginTop: 8 }}>
        {payment_overview.recent_failed_payments.length > 0 && (
          <div className="attention-panel">
            <div className="attention-panel-header">
              <h3>Recent Failed Payments</h3>
            </div>
            {payment_overview.recent_failed_payments.map((p) => (
              <div key={p.id} className="attention-item">
                <span className="attention-dot error" />
                <div className="attention-body">
                  <span className="attention-title" title={p.id}>
                    Payment {p.id.substring(0, 8)}...
                  </span>
                  <span className="attention-meta">
                    {(p.amount_minor / 100).toFixed(2)} {p.currency} &middot; {p.failure_reason || 'No reason'} &middot; {new Date(p.updated_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {refund_overview.recent_failed_refunds.length > 0 && (
          <div className="attention-panel">
            <div className="attention-panel-header">
              <h3>Recent Failed Refunds</h3>
            </div>
            {refund_overview.recent_failed_refunds.map((r) => (
              <div key={r.id} className="attention-item">
                <span className="attention-dot error" />
                <div className="attention-body">
                  <span className="attention-title" title={r.id}>
                    Refund {r.id.substring(0, 8)}...
                  </span>
                  <span className="attention-meta">
                    {(r.amount_minor / 100).toFixed(2)} {r.currency} &middot; Payment {r.payment_id.substring(0, 8)}... &middot; {new Date(r.updated_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {notification_overview.recent_failed_notifications.length > 0 && (
          <div className="attention-panel">
            <div className="attention-panel-header">
              <h3>Recent Failed Notifications</h3>
            </div>
            {notification_overview.recent_failed_notifications.map((n) => (
              <div key={n.id} className="attention-item">
                <span className="attention-dot error" />
                <div className="attention-body">
                  <span className="attention-title" title={n.id}>
                    {n.event_type}
                  </span>
                  <span className="attention-meta">
                    Record {n.id.substring(0, 8)}... &middot; {n.last_error || 'No error'} &middot; {new Date(n.updated_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {reconciliation_overview.recent_attention_reconciliations.length > 0 && (
          <div className="attention-panel">
            <div className="attention-panel-header">
              <h3>Reconciliations Needing Attention</h3>
            </div>
            {reconciliation_overview.recent_attention_reconciliations.map((r) => (
              <div key={r.id} className="attention-item">
                <span className={`attention-dot ${r.status === 'mismatched' ? 'error' : 'warning'}`} />
                <div className="attention-body">
                  <span className="attention-title" title={r.id}>
                    {r.status} &middot; Discrepancy: {(r.discrepancy_minor / 100).toFixed(2)} {r.currency}
                  </span>
                  <span className="attention-meta">
                    {new Date(r.created_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {audit_overview.recent_attention_audit_records.length > 0 && (
          <div className="attention-panel">
            <div className="attention-panel-header">
              <h3>Recent Audit Records</h3>
            </div>
            {audit_overview.recent_attention_audit_records.map((r) => (
              <div key={r.id} className="attention-item">
                <span className="attention-dot primary" />
                <div className="attention-body">
                  <span className="attention-title">
                    {r.action} by {r.actor_type}
                  </span>
                  <span className="attention-meta">
                    {r.resource_type}/{r.resource_id} &middot; {new Date(r.occurred_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {showReconciliation && (
        <RunReconciliationDialog
          configuredCurrency={configured_currency}
          onClose={() => setShowReconciliation(false)}
          onCreated={(_reconciliation: Reconciliation) => {
            setShowReconciliation(false);
            onRefresh();
          }}
        />
      )}
    </div>
  );
}
