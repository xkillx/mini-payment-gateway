import type { PaymentRefundSummary } from '../api/types';
import { formatCurrency } from '../utils/format';

interface RelatedRefundsProps {
  refunds: PaymentRefundSummary[];
}

export default function RelatedRefunds({ refunds }: RelatedRefundsProps) {
  const statusClass = (s: string) => {
    switch (s) {
      case 'pending': return 'pending';
      case 'processing': return 'processing';
      case 'completed': return 'completed';
      case 'failed': return 'failed';
      default: return '';
    }
  };

  return (
    <div className="section" style={{ background: 'var(--color-surface)', padding: 24, borderRadius: 'var(--radius)', boxShadow: 'var(--shadow)' }}>
      <h3 style={{ marginBottom: 16 }}>Related Refunds</h3>
      <div className="table-wrapper" style={{ boxShadow: 'none' }}>
        <table>
          <thead>
            <tr>
              <th>Refund ID</th>
              <th>Amount</th>
              <th>Status</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {refunds.map((r) => (
              <tr key={r.id}>
                <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                  {r.id.substring(0, 8)}...
                </td>
                <td>{formatCurrency(r.amount_minor, r.currency)}</td>
                <td>
                  <span className={`status-badge ${statusClass(r.status)}`}>
                    {r.status.charAt(0).toUpperCase() + r.status.slice(1)}
                  </span>
                </td>
                <td style={{ fontSize: 13 }}>
                  {new Date(r.created_at).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
