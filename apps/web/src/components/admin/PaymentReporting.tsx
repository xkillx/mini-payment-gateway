import { useState, useEffect, useCallback } from 'react';
import { Search, RotateCcw } from 'lucide-react';
import { getPaymentSummaryReport } from '../../api/client';
import type { PaymentSummaryReport } from '../../api/types';
import { formatCurrency } from '../../utils/format';

export default function PaymentReporting() {
  const [report, setReport] = useState<PaymentSummaryReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');

  const loadReport = useCallback(async (params?: { from?: string; to?: string }) => {
    setLoading(true);
    setError(null);
    try {
      const data = await getPaymentSummaryReport(params || {});
      setReport(data);
    } catch {
      setError('Failed to load payment report.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadReport();
  }, [loadReport]);

  const handleApply = () => {
    const params: { from?: string; to?: string } = {};
    if (from) params.from = from + ':00Z';
    if (to) params.to = to + ':00Z';
    loadReport(params);
  };

  const handleReset = () => {
    setFrom('');
    setTo('');
    loadReport();
  };

  return (
    <div>
      <div className="page-header">
        <h2>Payment Reporting</h2>
        <p>Platform-wide aggregate Payment and Refund activity for the selected UTC period.</p>
      </div>

      <div className="search-bar">
        <label style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, fontWeight: 600, color: 'var(--color-text-secondary)' }}>
          From (UTC)
          <input
            type="datetime-local"
            className="search-input"
            value={from}
            onChange={(e) => setFrom(e.target.value)}
            placeholder="YYYY-MM-DDTHH:MM"
            style={{ width: 220 }}
          />
        </label>
        <label style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 13, fontWeight: 600, color: 'var(--color-text-secondary)' }}>
          To (UTC)
          <input
            type="datetime-local"
            className="search-input"
            value={to}
            onChange={(e) => setTo(e.target.value)}
            placeholder="YYYY-MM-DDTHH:MM"
            style={{ width: 220 }}
          />
        </label>
        <button className="btn btn-primary" onClick={handleApply} disabled={loading}>
          <Search size={16} />
          Apply
        </button>
        <button className="btn btn-secondary" onClick={handleReset} disabled={loading}>
          <RotateCcw size={16} />
          Reset
        </button>
      </div>

      {loading && <p style={{ padding: '12px 0', color: 'var(--color-text-secondary)' }}>Loading report...</p>}

      {error && <div className="error-banner">{error}</div>}

      {!loading && !error && report && (
        <>
          <div className="bento-grid">
            <div className="metric-card" style={{ gridColumn: 'span 3' }}>
              <div className="metric-icon primary">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="3" width="20" height="14" rx="2" /><line x1="8" y1="21" x2="16" y2="21" /><line x1="12" y1="17" x2="12" y2="21" /></svg>
              </div>
              <div className="metric-count">{report.payment_totals.created_count}</div>
              <div className="metric-label">Created Payments</div>
              <div className="metric-subtitle">
                {formatCurrency(report.payment_totals.created_amount_minor, report.configured_currency)}
              </div>
            </div>
            <div className="metric-card" style={{ gridColumn: 'span 3' }}>
              <div className="metric-icon success">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" /><polyline points="22 4 12 14.01 9 11.01" /></svg>
              </div>
              <div className="metric-count">{report.payment_totals.successful_count}</div>
              <div className="metric-label">Successful Payments</div>
              <div className="metric-subtitle">
                {formatCurrency(report.payment_totals.successful_amount_minor, report.configured_currency)}
              </div>
            </div>
            <div className="metric-card" style={{ gridColumn: 'span 3' }}>
              <div className="metric-icon error">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" /></svg>
              </div>
              <div className="metric-count">{report.payment_totals.failed_count}</div>
              <div className="metric-label">Failed Payments</div>
              <div className="metric-subtitle">
                Attempted: {formatCurrency(report.payment_totals.failed_attempted_amount_minor, report.configured_currency)}
              </div>
            </div>
            <div className="metric-card" style={{ gridColumn: 'span 3' }}>
              <div className="metric-icon warning">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="1 4 1 10 7 10" /><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" /></svg>
              </div>
              <div className="metric-count">{report.refund_activity.completed_count}</div>
              <div className="metric-label">Completed Refunds</div>
              <div className="metric-subtitle">
                {formatCurrency(report.refund_activity.completed_amount_minor, report.configured_currency)}
              </div>
            </div>
          </div>

          <div className="section">
            <div className="section-header">
              <h3>Daily Payment Trend</h3>
              {report.payment_totals.failed_count > 0 && (
                <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
                  Failed Refunds: {report.refund_activity.failed_count}
                </span>
              )}
            </div>
            <div className="table-wrapper">
              <table>
                <thead>
                  <tr>
                    <th>Date (UTC)</th>
                    <th>Created</th>
                    <th>Successful</th>
                    <th>Successful Amount</th>
                    <th>Failed</th>
                  </tr>
                </thead>
                <tbody>
                  {report.trend.map((bucket) => (
                    <tr key={bucket.bucket_date}>
                      <td>{bucket.bucket_date}</td>
                      <td>{bucket.created_count}</td>
                      <td>{bucket.successful_count}</td>
                      <td>{formatCurrency(bucket.successful_amount_minor, report.configured_currency)}</td>
                      <td>{bucket.failed_count}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          <div className="section" style={{ marginTop: 16 }}>
            <p style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
              Period: {report.period_start} to {report.period_end} |
              Generated: {report.generated_at} |
              Failed Refunds in period: {report.refund_activity.failed_count}
            </p>
          </div>
        </>
      )}

      {!loading && !error && !report && (
        <div className="empty-state">
          <p>No report data available.</p>
        </div>
      )}
    </div>
  );
}
