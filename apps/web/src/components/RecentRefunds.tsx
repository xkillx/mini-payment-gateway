import type { DashboardRefund as DashboardRefundType, RefundStatus } from '../api/types';
import { formatCurrency } from '../utils/format';
import { Undo2 } from 'lucide-react';

interface RecentRefundsProps {
  refunds: DashboardRefundType[];
}

export default function RecentRefunds({ refunds }: RecentRefundsProps) {
  if (refunds.length === 0) {
    return (
      <div className="section">
        <div className="section-header">
          <h3 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <Undo2 size={16} />
            Recent Refunds
          </h3>
        </div>
        <div className="empty-state">
          <p>No Refunds yet. Request a refund from an eligible successful Payment.</p>
        </div>
      </div>
    );
  }

  const statusClass = (s: RefundStatus) => {
    switch (s) {
      case 'pending': return 'pending';
      case 'processing': return 'processing';
      case 'completed': return 'completed';
      case 'failed': return 'failed';
    }
  };

  return (
    <div className="section">
      <div className="section-header">
        <h3 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Undo2 size={16} />
          Recent Refunds
        </h3>
      </div>
      <div className="table-wrapper">
        <table>
          <thead>
            <tr>
              <th>Refund ID</th>
              <th>Payment ID</th>
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
                <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                  {r.payment_id.substring(0, 8)}...
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
