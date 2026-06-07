import { useState, useCallback, useEffect } from 'react';
import { listReconciliations, getReconciliationReport } from '../../api/client';
import type { Reconciliation, ReconciliationReport, ReconciliationStatus } from '../../api/types';
import ReconciliationReportDetail from './ReconciliationReportDetail';

const PAGE_SIZE = 20;

interface ReconciliationReportsProps {
  initialReconciliationId?: string;
}

export default function ReconciliationReports({
  initialReconciliationId,
}: ReconciliationReportsProps = {}) {
  const [items, setItems] = useState<Reconciliation[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [selectedReport, setSelectedReport] = useState<ReconciliationReport | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [reportLoading, setReportLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const load = useCallback(async (newOffset: number, isReset: boolean) => {
    setLoading(true);
    setError(null);
    try {
      const res = await listReconciliations({
        limit: PAGE_SIZE,
        offset: newOffset,
      });
      if (isReset) {
        setItems(res.items);
      } else {
        setItems((prev) => [...prev, ...res.items]);
      }
      setHasMore(res.items.length === PAGE_SIZE);
      setOffset(newOffset);
      setSearched(true);
    } catch {
      setError('Failed to load reconciliations.');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (initialReconciliationId) {
      handleSelectReport(initialReconciliationId);
    }
  }, [initialReconciliationId]);

  const handleLoad = () => {
    setSearched(false);
    load(0, true);
  };

  const handleLoadMore = () => {
    load(offset + PAGE_SIZE, false);
  };

  const handleSelectReport = async (id: string) => {
    setSelectedId(id);
    setReportLoading(true);
    setError(null);
    try {
      const report = await getReconciliationReport(id);
      setSelectedReport(report);
    } catch {
      setError('Failed to load reconciliation report.');
      setSelectedReport(null);
    } finally {
      setReportLoading(false);
    }
  };

  const handleBack = () => {
    setSelectedReport(null);
    setSelectedId(null);
  };

  const statusClass = (s: ReconciliationStatus) => {
    switch (s) {
      case 'matched':
        return 'matched';
      case 'mismatched':
        return 'mismatched';
      case 'error':
        return 'pending';
    }
  };

  if (reportLoading) {
    return <div style={{ padding: 24 }}>Loading report...</div>;
  }

  if (selectedReport) {
    return <ReconciliationReportDetail report={selectedReport} onBack={handleBack} />;
  }

  return (
    <div>
      <div className="search-bar">
        <button className="btn btn-primary" onClick={handleLoad}>
          Load Reconciliations
        </button>
      </div>

      {error && (
        <div className="error-banner" style={{ marginBottom: 16 }}>
          {error}
        </div>
      )}

      {searched && !loading && items.length === 0 && (
        <div className="empty-state">
          <p>No Reconciliation records found.</p>
        </div>
      )}

      {items.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>Status</th>
                <th>Expected</th>
                <th>Actual</th>
                <th>Discrepancy</th>
                <th>Window</th>
                <th>Run At</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {items.map((r) => (
                <tr key={r.id}>
                  <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                    {r.id.substring(0, 8)}...
                  </td>
                  <td>
                    <span className={`status-badge ${statusClass(r.status)}`}>
                      {r.status.charAt(0).toUpperCase() + r.status.slice(1)}
                    </span>
                  </td>
                  <td>{(r.expected_total_minor / 100).toFixed(2)} {r.currency}</td>
                  <td>{(r.actual_total_minor / 100).toFixed(2)} {r.currency}</td>
                  <td
                    style={{
                      color:
                        r.discrepancy_minor !== 0
                          ? 'var(--color-error)'
                          : undefined,
                    }}
                  >
                    {(r.discrepancy_minor / 100).toFixed(2)} {r.currency}
                  </td>
                  <td style={{ fontSize: 13 }}>
                    {new Date(r.window_start).toLocaleString()}
                    <br />
                    {new Date(r.window_end).toLocaleString()}
                  </td>
                  <td style={{ fontSize: 13 }}>{new Date(r.run_at).toLocaleString()}</td>
                  <td>
                    <button
                      className={`btn ${selectedId === r.id ? 'btn-primary' : 'btn-secondary'}`}
                      style={{ fontSize: 12, padding: '4px 8px' }}
                      onClick={() => handleSelectReport(r.id)}
                    >
                      Report
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {hasMore && (
            <div className="pagination">
              <button
                className="btn btn-secondary"
                onClick={handleLoadMore}
                disabled={loading}
              >
                {loading ? 'Loading...' : 'Load more'}
              </button>
            </div>
          )}
        </div>
      )}

      {!searched && !loading && (
        <div className="empty-state">
          <p>Click "Load Reconciliations" to view historical Reconciliation records.</p>
        </div>
      )}
    </div>
  );
}
