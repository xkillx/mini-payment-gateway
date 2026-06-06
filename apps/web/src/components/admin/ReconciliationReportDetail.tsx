import { ArrowLeft } from 'lucide-react';
import type { ReconciliationReport, ReconciliationStatus } from '../../api/types';

interface ReconciliationReportDetailProps {
  report: ReconciliationReport;
  onBack: () => void;
}

export default function ReconciliationReportDetail({
  report,
  onBack,
}: ReconciliationReportDetailProps) {
  const statusClass = (s: ReconciliationStatus) => {
    switch (s) {
      case 'matched':
        return 'successful';
      case 'mismatched':
        return 'failed';
      case 'error':
        return 'pending';
    }
  };

  return (
    <div>
      <button
        className="btn btn-secondary"
        onClick={onBack}
        style={{ marginBottom: 16 }}
      >
        <ArrowLeft size={16} />
        Back
      </button>

      <div
        className="section"
        style={{
          background: 'var(--color-surface)',
          padding: 24,
          borderRadius: 'var(--radius)',
          boxShadow: 'var(--shadow)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 20,
          }}
        >
          <h3>Reconciliation Report</h3>
          <span className={`status-badge ${statusClass(report.status)}`}>
            {report.status.charAt(0).toUpperCase() + report.status.slice(1)}
          </span>
        </div>

        <div className="detail-grid">
          <span className="detail-label">Reconciliation ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {report.id}
          </span>

          <span className="detail-label">Currency</span>
          <span className="detail-value">{report.currency}</span>

          <span className="detail-label">Expected Total</span>
          <span className="detail-value">
            {(report.expected_total_minor / 100).toFixed(2)} {report.currency}
          </span>

          <span className="detail-label">Actual Total</span>
          <span className="detail-value">
            {(report.actual_total_minor / 100).toFixed(2)} {report.currency}
          </span>

          <span className="detail-label">Discrepancy</span>
          <span
            className="detail-value"
            style={{
              color:
                report.discrepancy_minor !== 0
                  ? 'var(--color-error)'
                  : undefined,
            }}
          >
            {(report.discrepancy_minor / 100).toFixed(2)} {report.currency}
          </span>

          <span className="detail-label">Window Start</span>
          <span className="detail-value">
            {new Date(report.window_start).toLocaleString()}
          </span>

          <span className="detail-label">Window End</span>
          <span className="detail-value">
            {new Date(report.window_end).toLocaleString()}
          </span>

          {report.notes && (
            <>
              <span className="detail-label">Notes</span>
              <span className="detail-value">{report.notes}</span>
            </>
          )}

          <span className="detail-label">Run At</span>
          <span className="detail-value">
            {new Date(report.run_at).toLocaleString()}
          </span>

          <span className="detail-label">Created</span>
          <span className="detail-value">
            {new Date(report.created_at).toLocaleString()}
          </span>

          <span className="detail-label">Included Payments</span>
          <span className="detail-value">
            {(report.included_payment_total_minor / 100).toFixed(2)}{' '}
            {report.currency} ({report.included_payments.length} records)
          </span>

          <span className="detail-label">Included Refunds</span>
          <span className="detail-value">
            {(report.included_refund_total_minor / 100).toFixed(2)}{' '}
            {report.currency} ({report.included_refunds.length} records)
          </span>

          <span className="detail-label">Total Records</span>
          <span className="detail-value">{report.included_record_count}</span>
        </div>
      </div>

      {report.included_payments.length > 0 && (
        <div
          className="section"
          style={{
            background: 'var(--color-surface)',
            padding: 24,
            borderRadius: 'var(--radius)',
            boxShadow: 'var(--shadow)',
            marginTop: 16,
          }}
        >
          <h3 style={{ marginBottom: 16 }}>Included Payments</h3>
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Payment ID</th>
                  <th>Merchant ID</th>
                  <th>Amount</th>
                  <th>Reference</th>
                  <th>Occurred</th>
                </tr>
              </thead>
              <tbody>
                {report.included_payments.map((p) => (
                  <tr key={p.payment_id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {p.payment_id.substring(0, 8)}...
                    </td>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {p.merchant_id.substring(0, 8)}...
                    </td>
                    <td>
                      {(p.amount_minor / 100).toFixed(2)} {p.currency}
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {p.metadata &&
                      typeof p.metadata === 'object' &&
                      'merchant_reference' in p.metadata
                        ? String(p.metadata.merchant_reference)
                        : '-'}
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {new Date(p.occurred_at).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {report.included_refunds.length > 0 && (
        <div
          className="section"
          style={{
            background: 'var(--color-surface)',
            padding: 24,
            borderRadius: 'var(--radius)',
            boxShadow: 'var(--shadow)',
            marginTop: 16,
          }}
        >
          <h3 style={{ marginBottom: 16 }}>Included Refunds</h3>
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Refund ID</th>
                  <th>Payment ID</th>
                  <th>Merchant ID</th>
                  <th>Amount</th>
                  <th>Occurred</th>
                </tr>
              </thead>
              <tbody>
                {report.included_refunds.map((r) => (
                  <tr key={r.refund_id}>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {r.refund_id.substring(0, 8)}...
                    </td>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {r.payment_id.substring(0, 8)}...
                    </td>
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {r.merchant_id.substring(0, 8)}...
                    </td>
                    <td>
                      {(r.amount_minor / 100).toFixed(2)} {r.currency}
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {new Date(r.occurred_at).toLocaleString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
