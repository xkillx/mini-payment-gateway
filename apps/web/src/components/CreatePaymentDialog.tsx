import { useState } from 'react';
import { X } from 'lucide-react';
import { createPayment, ApiClientError } from '../api/client';
import { generateIdempotencyKey } from '../utils/idempotency';

interface CreatePaymentDialogProps {
  currency: string;
  onClose: () => void;
  onCreated: () => void;
}

export default function CreatePaymentDialog({ currency, onClose, onCreated }: CreatePaymentDialogProps) {
  const [amountMinor, setAmountMinor] = useState('');
  const [merchantReference, setMerchantReference] = useState('');
  const [metadataJson, setMetadataJson] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});

  const validate = (): boolean => {
    const errs: Record<string, string> = {};
    const amount = parseInt(amountMinor, 10);
    if (!amountMinor.trim() || isNaN(amount) || amount <= 0 || !Number.isInteger(amount)) {
      errs.amount_minor = 'Amount must be a positive integer in minor units (e.g., 1000 for $10.00).';
    }
    if (!merchantReference.trim()) {
      errs.merchant_reference = 'Merchant Reference is required.';
    }
    if (metadataJson.trim()) {
      try {
        const parsed = JSON.parse(metadataJson);
        if (typeof parsed !== 'object' || Array.isArray(parsed) || parsed === null) {
          errs.metadata = 'Metadata must be a JSON object.';
        }
      } catch {
        errs.metadata = 'Invalid JSON.';
      }
    }
    setFieldErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    setSubmitting(true);
    setError(null);

    let metadata: Record<string, unknown> = { merchant_reference: merchantReference.trim() };

    if (metadataJson.trim()) {
      try {
        const extra = JSON.parse(metadataJson);
        metadata = { ...extra, merchant_reference: merchantReference.trim() };
      } catch {
        // already validated
      }
    }

    try {
      await createPayment(
        {
          amount_minor: parseInt(amountMinor, 10),
          currency,
          metadata,
        },
        generateIdempotencyKey(),
      );
      onCreated();
      onClose();
    } catch (e) {
      if (e instanceof ApiClientError) {
        setError(e.body.message || 'Failed to create payment.');
      } else {
        setError('Network error. Please try again.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <h3>Create Payment</h3>
          <button className="btn-icon" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="amount">Payment Amount (minor units)</label>
            <input
              id="amount"
              type="text"
              inputMode="numeric"
              value={amountMinor}
              onChange={(e) => setAmountMinor(e.target.value)}
              placeholder="1000 (= $10.00 USD)"
            />
            <div style={{ fontSize: 12, color: 'var(--color-text-secondary)', marginTop: 4 }}>
              Enter amount in minor units (e.g., 1000 = $10.00). Currency: {currency}
            </div>
            {fieldErrors.amount_minor && <div className="form-error">{fieldErrors.amount_minor}</div>}
          </div>

          <div className="form-group">
            <label htmlFor="reference">Merchant Reference</label>
            <input
              id="reference"
              type="text"
              value={merchantReference}
              onChange={(e) => setMerchantReference(e.target.value)}
              placeholder="e.g., ORDER-1001"
            />
            {fieldErrors.merchant_reference && <div className="form-error">{fieldErrors.merchant_reference}</div>}
          </div>

          <div className="form-group">
            <label htmlFor="metadata">Additional Metadata (JSON, optional)</label>
            <textarea
              id="metadata"
              value={metadataJson}
              onChange={(e) => setMetadataJson(e.target.value)}
              placeholder='{"key": "value"}'
              rows={3}
            />
            {fieldErrors.metadata && <div className="form-error">{fieldErrors.metadata}</div>}
          </div>

          {error && <div className="error-banner">{error}</div>}

          <div className="form-actions">
            <button className="btn btn-secondary" type="button" onClick={onClose}>
              Cancel
            </button>
            <button className="btn btn-primary" type="submit" disabled={submitting}>
              {submitting ? 'Creating...' : 'Create Payment'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
