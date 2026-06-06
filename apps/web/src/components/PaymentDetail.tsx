import { useState, useEffect } from 'react';
import { ArrowLeft } from 'lucide-react';
import { getPayment } from '../api/client';
import type { PaymentDetail as PaymentDetailType, PaymentStatus } from '../api/types';
import { formatCurrency, getMerchantReference } from '../utils/format';
import PaymentStatusHistory from './PaymentStatusHistory';
import PaymentMetadataView from './PaymentMetadataView';
import RelatedRefunds from './RelatedRefunds';
import RequestRefundAction from './RequestRefundAction';

interface PaymentDetailProps {
  paymentId: string;
  onBack: () => void;
}

export default function PaymentDetail({ paymentId, onBack }: PaymentDetailProps) {
  const [payment, setPayment] = useState<PaymentDetailType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const p = await getPayment(paymentId);
      setPayment(p);
    } catch {
      setError('Payment not found or not accessible.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [paymentId]);

  if (loading) {
    return <div style={{ padding: 24 }}>Loading...</div>;
  }

  if (error || !payment) {
    return (
      <div style={{ padding: 24 }}>
        <div className="error-banner">{error || 'Payment not found'}</div>
        <button className="btn btn-secondary" onClick={onBack} style={{ marginTop: 12 }}>
          <ArrowLeft size={16} />
          Back
        </button>
      </div>
    );
  }

  const merchantRef = getMerchantReference(payment.metadata);
  const { merchant_reference: _, ...otherMetadata } = (payment.metadata && typeof payment.metadata === 'object')
    ? payment.metadata
    : {};

  const isEligibleForRefund =
    payment.status === 'successful' &&
    (!payment.refunds || payment.refunds.length === 0);

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
    <div>
      <button className="btn btn-secondary" onClick={onBack} style={{ marginBottom: 16 }}>
        <ArrowLeft size={16} />
        Back
      </button>

      <div className="section" style={{ background: 'var(--color-surface)', padding: 24, borderRadius: 'var(--radius)', boxShadow: 'var(--shadow)' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 20 }}>
          <h3>Payment Detail</h3>
          <span className={`status-badge ${statusClass(payment.status)}`}>
            {payment.status.charAt(0).toUpperCase() + payment.status.slice(1)}
          </span>
        </div>

        <div className="detail-actions">
          <RequestRefundAction
            paymentId={payment.id}
            isEligible={isEligibleForRefund}
            hasExistingRefund={payment.refunds && payment.refunds.length > 0}
            onRefundCreated={load}
          />
        </div>

        <div className="detail-grid">
          <span className="detail-label">Payment ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>{payment.id}</span>

          <span className="detail-label">Amount</span>
          <span className="detail-value">{formatCurrency(payment.amount_minor, payment.currency)}</span>

          <span className="detail-label">Currency</span>
          <span className="detail-value">{payment.currency}</span>

          {merchantRef && (
            <>
              <span className="detail-label">Reference</span>
              <span className="detail-value">{merchantRef}</span>
            </>
          )}

          {payment.failure_reason && (
            <>
              <span className="detail-label">Failure Reason</span>
              <span className="detail-value" style={{ color: 'var(--color-error)' }}>{payment.failure_reason}</span>
            </>
          )}

          <span className="detail-label">Created</span>
          <span className="detail-value">{new Date(payment.created_at).toLocaleString()}</span>

          <span className="detail-label">Updated</span>
          <span className="detail-value">{new Date(payment.updated_at).toLocaleString()}</span>
        </div>

        {Object.keys(otherMetadata).length > 0 && (
          <PaymentMetadataView metadata={otherMetadata} />
        )}
      </div>

      {payment.status_history && payment.status_history.length > 0 && (
        <PaymentStatusHistory history={payment.status_history} />
      )}

      {payment.refunds && payment.refunds.length > 0 && (
        <RelatedRefunds refunds={payment.refunds} />
      )}
    </div>
  );
}
