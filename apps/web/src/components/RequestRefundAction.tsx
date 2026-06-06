import { useState } from 'react';
import { Undo2 } from 'lucide-react';
import { createRefund, ApiClientError } from '../api/client';
import { generateIdempotencyKey } from '../utils/idempotency';

interface RequestRefundActionProps {
  paymentId: string;
  isEligible: boolean;
  hasExistingRefund: boolean;
  onRefundCreated: () => void;
}

export default function RequestRefundAction({
  paymentId,
  isEligible,
  hasExistingRefund,
  onRefundCreated,
}: RequestRefundActionProps) {
  const [confirming, setConfirming] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleRequest = async () => {
    setSubmitting(true);
    setError(null);
    try {
      await createRefund(
        { payment_id: paymentId },
        generateIdempotencyKey(),
      );
      onRefundCreated();
      setConfirming(false);
    } catch (e) {
      if (e instanceof ApiClientError) {
        setError(e.body.message || 'Failed to request refund.');
      } else {
        setError('Network error. Please try again.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  if (hasExistingRefund) {
    return (
      <button className="btn btn-secondary" disabled>
        <Undo2 size={16} />
        Refund already requested
      </button>
    );
  }

  if (!isEligible) {
    return (
      <button className="btn btn-secondary" disabled title="Payment must be successful to request a refund">
        <Undo2 size={16} />
        Refund not available
      </button>
    );
  }

  if (confirming) {
    return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <p className="confirm-text">
          Confirm you want to request a full refund for this payment.
          This will reverse the full payment amount.
        </p>
        <div style={{ display: 'flex', gap: 8 }}>
          <button
            className="btn btn-primary"
            onClick={handleRequest}
            disabled={submitting}
          >
            {submitting ? 'Requesting...' : 'Confirm Refund'}
          </button>
          <button
            className="btn btn-secondary"
            onClick={() => {
              setConfirming(false);
              setError(null);
            }}
          >
            Cancel
          </button>
        </div>
        {error && <div className="error-banner">{error}</div>}
      </div>
    );
  }

  return (
    <button className="btn btn-primary" onClick={() => setConfirming(true)}>
      <Undo2 size={16} />
      Request Full Refund
    </button>
  );
}
