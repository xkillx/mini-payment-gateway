import { useState } from 'react';
import { X, RefreshCw } from 'lucide-react';
import { runReconciliation, ApiClientError } from '../../api/client';
import type { Reconciliation } from '../../api/types';

interface RunReconciliationDialogProps {
  configuredCurrency: string;
  onClose: () => void;
  onCreated: (reconciliation: Reconciliation) => void;
}

export default function RunReconciliationDialog({
  configuredCurrency,
  onClose,
  onCreated,
}: RunReconciliationDialogProps) {
  const [currency, setCurrency] = useState(configuredCurrency);
  const [windowStart, setWindowStart] = useState('');
  const [windowEnd, setWindowEnd] = useState('');
  const [actualTotal, setActualTotal] = useState('');
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});

  const toIsoString = (value: string): string => {
    if (!value) return '';
    try {
      return new Date(value).toISOString();
    } catch {
      return '';
    }
  };

  const validate = (): boolean => {
    const errs: Record<string, string> = {};

    if (!currency.trim()) {
      errs.currency = 'Currency is required.';
    }

    if (!windowStart.trim()) {
      errs.window_start = 'Window start is required.';
    }

    if (!windowEnd.trim()) {
      errs.window_end = 'Window end is required.';
    }

    if (windowStart.trim() && windowEnd.trim()) {
      const start = new Date(windowStart).getTime();
      const end = new Date(windowEnd).getTime();

      if (isNaN(start)) errs.window_start = 'Invalid date/time.';
      if (isNaN(end)) errs.window_end = 'Invalid date/time.';

      if (!isNaN(start) && !isNaN(end) && start >= end) {
        errs.window_end = 'Window end must be after window start.';
      }
    }

    const total = parseInt(actualTotal, 10);
    if (!actualTotal.trim() || isNaN(total) || !Number.isInteger(total)) {
      errs.actual_total_minor = 'Actual total must be an integer in minor units.';
    }

    setFieldErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    setSubmitting(true);
    setError(null);

    try {
      const result = await runReconciliation({
        currency: currency.trim(),
        window_start: toIsoString(windowStart),
        window_end: toIsoString(windowEnd),
        actual_total_minor: parseInt(actualTotal, 10),
        notes: notes.trim() || undefined,
      });
      onCreated(result);
    } catch (e) {
      if (e instanceof ApiClientError) {
        setError(e.body.message || 'Reconciliation failed.');
      } else {
        setError('Network error. Please try again.');
      }
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 520 }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
          <h3>Manual Reconciliation</h3>
          <button className="btn-icon" onClick={onClose} aria-label="Close dialog">
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="currency">Configured Currency</label>
            <input
              id="currency"
              type="text"
              value={currency}
              onChange={(e) => setCurrency(e.target.value)}
              placeholder="USD"
            />
            {fieldErrors.currency && <div className="form-error">{fieldErrors.currency}</div>}
          </div>

          <div className="form-group">
            <label htmlFor="window_start">Window Start</label>
            <input
              id="window_start"
              type="datetime-local"
              value={windowStart}
              onChange={(e) => setWindowStart(e.target.value)}
            />
            <div className="form-hint">Inclusive start of the Reconciliation Window.</div>
            {fieldErrors.window_start && <div className="form-error">{fieldErrors.window_start}</div>}
          </div>

          <div className="form-group">
            <label htmlFor="window_end">Window End</label>
            <input
              id="window_end"
              type="datetime-local"
              value={windowEnd}
              onChange={(e) => setWindowEnd(e.target.value)}
            />
            <div className="form-hint">Exclusive end of the Reconciliation Window.</div>
            {fieldErrors.window_end && <div className="form-error">{fieldErrors.window_end}</div>}
          </div>

          <div className="form-group">
            <label htmlFor="actual_total">Actual Reconciliation Total (minor units)</label>
            <input
              id="actual_total"
              type="text"
              inputMode="numeric"
              value={actualTotal}
              onChange={(e) => setActualTotal(e.target.value)}
              placeholder="0"
            />
            <div className="form-hint">
              Enter net amount in minor units. May be zero or negative.
            </div>
            {fieldErrors.actual_total_minor && (
              <div className="form-error">{fieldErrors.actual_total_minor}</div>
            )}
          </div>

          <div className="form-group">
            <label htmlFor="notes">Notes (optional)</label>
            <textarea
              id="notes"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Optional reconciliation notes"
              rows={2}
            />
          </div>

          {error && <div className="error-banner" style={{ marginBottom: 16 }}>{error}</div>}

          <div className="form-actions">
            <button className="btn btn-secondary" type="button" onClick={onClose}>
              Cancel
            </button>
            <button className="btn btn-primary" type="submit" disabled={submitting}>
              <RefreshCw size={16} />
              {submitting ? 'Running...' : 'Run Reconciliation'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
