import { useState, useEffect } from 'react';
import { ArrowLeft } from 'lucide-react';
import { getRefund } from '../api/client';
import type { RefundDetail as RefundDetailType } from '../api/types';

interface RefundDetailProps {
  refundId: string;
  onBack: () => void;
}

export default function RefundDetail({ refundId, onBack }: RefundDetailProps) {
  const [refund, setRefund] = useState<RefundDetailType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    getRefund(refundId)
      .then((r) => setRefund(r))
      .catch(() => setError('Refund not found.'))
      .finally(() => setLoading(false));
  }, [refundId]);

  if (loading) {
    return <div style={{ padding: 24 }}>Loading...</div>;
  }

  if (error || !refund) {
    return (
      <div style={{ padding: 24 }}>
        <div className="error-banner">{error || 'Refund not found'}</div>
        <button
          className="btn btn-secondary"
          onClick={onBack}
          style={{ marginTop: 12 }}
        >
          <ArrowLeft size={16} />
          Back
        </button>
      </div>
    );
  }

  const statusClass = (s: string) => {
    switch (s) {
      case 'pending': return 'pending';
      case 'processing': return 'processing';
      case 'completed': return 'successful';
      case 'failed': return 'failed';
      default: return '';
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
          <h3>Refund Detail</h3>
          <span className={`status-badge ${statusClass(refund.status)}`}>
            {refund.status.charAt(0).toUpperCase() + refund.status.slice(1)}
          </span>
        </div>

        <div className="detail-grid">
          <span className="detail-label">Refund ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {refund.id}
          </span>

          <span className="detail-label">Payment ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {refund.payment_id}
          </span>

          <span className="detail-label">Merchant ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {refund.merchant_id}
          </span>

          <span className="detail-label">Amount</span>
          <span className="detail-value">
            {(refund.amount_minor / 100).toFixed(2)} {refund.currency}
          </span>

          <span className="detail-label">Created</span>
          <span className="detail-value">
            {new Date(refund.created_at).toLocaleString()}
          </span>

          <span className="detail-label">Updated</span>
          <span className="detail-value">
            {new Date(refund.updated_at).toLocaleString()}
          </span>
        </div>
      </div>
    </div>
  );
}
