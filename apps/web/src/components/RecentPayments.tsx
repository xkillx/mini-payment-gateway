import { useState } from 'react';
import { ArrowRightCircle } from 'lucide-react';
import type { DashboardPayment as DashboardPaymentType, PaymentStatus } from '../api/types';
import { formatCurrency, getMerchantReference } from '../utils/format';
import PaymentDetail from './PaymentDetail';

interface RecentPaymentsProps {
  payments: DashboardPaymentType[];
}

export default function RecentPayments({ payments }: RecentPaymentsProps) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  if (selectedId) {
    return <PaymentDetail paymentId={selectedId} onBack={() => setSelectedId(null)} />;
  }

  if (payments.length === 0) {
    return (
      <div className="empty-state">
        <p>No Payments yet.</p>
      </div>
    );
  }

  const statusClass = (s: PaymentStatus) => {
    switch (s) {
      case 'pending': return 'pending';
      case 'processing': return 'processing';
      case 'successful': return 'successful';
      case 'failed': return 'failed';
      case 'refunded': return 'refunded';
    }
  };

  return (
    <div className="table-wrapper">
      <table>
        <thead>
          <tr>
            <th>Payment ID</th>
            <th>Reference</th>
            <th>Amount</th>
            <th>Status</th>
            <th>Created</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {payments.map((p) => (
            <tr key={p.id}>
              <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                {p.id.substring(0, 8)}...
              </td>
              <td>{getMerchantReference(p.metadata) || '-'}</td>
              <td>{formatCurrency(p.amount_minor, p.currency)}</td>
              <td>
                <span className={`status-badge ${statusClass(p.status)}`}>
                  {p.status.charAt(0).toUpperCase() + p.status.slice(1)}
                </span>
              </td>
              <td style={{ fontSize: 13 }}>
                {new Date(p.created_at).toLocaleString()}
              </td>
              <td>
                <button
                  className="btn-icon"
                  title="View payment details"
                  aria-label={`View details for payment ${p.id.substring(0, 8)}`}
                  onClick={() => setSelectedId(p.id)}
                >
                  <ArrowRightCircle size={16} />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
