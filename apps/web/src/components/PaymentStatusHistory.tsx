import type { PaymentStatusHistoryEntry as HistoryEntry, PaymentStatus } from '../api/types';

interface PaymentStatusHistoryProps {
  history: HistoryEntry[];
}

export default function PaymentStatusHistory({ history }: PaymentStatusHistoryProps) {
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
    <div className="section" style={{ background: 'var(--color-surface)', padding: 24, borderRadius: 'var(--radius)', boxShadow: 'var(--shadow)' }}>
      <h3 style={{ marginBottom: 16 }}>Status History</h3>
      <div className="history-list">
        {history.map((entry, i) => (
          <div key={i} className="history-entry">
            <span className="history-time">
              {new Date(entry.occurred_at).toLocaleString()}
            </span>
            <div>
              <span className={`status-badge ${statusClass(entry.status)}`}>
                {entry.status.charAt(0).toUpperCase() + entry.status.slice(1)}
              </span>
              <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginTop: 2 }}>
                {entry.source_event_type}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
