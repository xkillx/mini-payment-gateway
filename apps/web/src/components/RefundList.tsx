import { useState, useCallback } from 'react';
import type { RefundStatus, RefundListItem } from '../api/types';
import { listRefunds } from '../api/client';
import { formatCurrency } from '../utils/format';
import StatusFilter from './StatusFilter';

interface RefundListProps {
  currency: string;
}

const PAGE_SIZE = 20;

export default function RefundList({ currency: _currency }: RefundListProps) {
  const [status, setStatus] = useState<RefundStatus | ''>('');
  const [items, setItems] = useState<RefundListItem[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const load = useCallback(async (newOffset: number, isReset: boolean) => {
    setLoading(true);
    setError(null);
    try {
      const res = await listRefunds({
        status: status || undefined,
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
      setError('Failed to load refunds.');
    } finally {
      setLoading(false);
    }
  }, [status]);

  const handleSearch = () => {
    setSearched(false);
    load(0, true);
  };

  const handleStatusChange = (newStatus: string) => {
    setStatus(newStatus as RefundStatus | '');
    setSearched(false);
  };

  const handleLoadMore = () => {
    load(offset + PAGE_SIZE, false);
  };

  const statusClass = (s: RefundStatus) => {
    switch (s) {
      case 'pending': return 'pending';
      case 'processing': return 'processing';
      case 'completed': return 'completed';
      case 'failed': return 'failed';
    }
  };

  return (
    <div>
      <div className="search-bar">
        <StatusFilter
          label="Refund Status"
          value={status}
          onChange={handleStatusChange}
          options={[
            { value: '', label: 'All statuses' },
            { value: 'pending', label: 'Pending' },
            { value: 'processing', label: 'Processing' },
            { value: 'completed', label: 'Completed' },
            { value: 'failed', label: 'Failed' },
          ]}
        />
        <button className="btn btn-primary" onClick={handleSearch}>
          Filter
        </button>
      </div>

      {error && <div className="error-banner" style={{ marginBottom: 16 }}>{error}</div>}

      {searched && !loading && items.length === 0 && (
        <div className="empty-state">
          <p>No Refunds found. Request a refund from an eligible successful Payment.</p>
        </div>
      )}

      {items.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Refund ID</th>
                <th>Payment ID</th>
                <th>Amount</th>
                <th>Status</th>
                <th>Created</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {items.map((r) => (
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
                  <td style={{ fontSize: 13 }}>
                    {new Date(r.updated_at).toLocaleString()}
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
          <p>Use the status filter above to find Refunds.</p>
        </div>
      )}
    </div>
  );
}
