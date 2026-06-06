import { useState, useCallback } from 'react';
import {
  listNotifications,
  retryNotification,
  ApiClientError,
} from '../../api/client';
import type {
  NotificationDeliveryRecordDetail,
  NotificationStatus,
} from '../../api/types';
import NotificationDetail from './NotificationDetail';

type NotificationStatusFilter = NotificationStatus | '';

const PAGE_SIZE = 20;

export default function NotificationMonitoring() {
  const [status, setStatus] = useState<NotificationStatusFilter>('');
  const [items, setItems] = useState<NotificationDeliveryRecordDetail[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const load = useCallback(
    async (newOffset: number, isReset: boolean) => {
      setLoading(true);
      setError(null);
      try {
        const res = await listNotifications({
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
        setError('Failed to load notifications.');
      } finally {
        setLoading(false);
      }
    },
    [status]
  );

  const handleSearch = () => {
    setSearched(false);
    load(0, true);
  };

  const handleStatusChange = (newStatus: NotificationStatusFilter) => {
    setStatus(newStatus);
    setSearched(false);
  };

  const handleLoadMore = () => {
    load(offset + PAGE_SIZE, false);
  };

  const handleRetry = async (id: string) => {
    try {
      await retryNotification(id);
      load(0, true);
      setSelectedId(null);
    } catch (e) {
      if (e instanceof ApiClientError) {
        if (e.status === 409) {
          setError('Notification is not in failed status. It may have been retried already.');
        } else if (e.status === 404) {
          setError('Notification not found.');
        } else {
          setError('Failed to retry notification.');
        }
      } else {
        setError('Failed to retry notification.');
      }
    }
  };

  const statusClass = (s: string) => {
    switch (s) {
      case 'pending':
        return 'pending';
      case 'processing':
        return 'processing';
      case 'delivered':
        return 'successful';
      case 'failed':
        return 'failed';
      default:
        return '';
    }
  };

  if (selectedId) {
    return (
      <NotificationDetail
        notificationId={selectedId}
        onBack={() => {
          setSelectedId(null);
          load(0, true);
        }}
        onRetry={handleRetry}
      />
    );
  }

  return (
    <div>
      <div className="search-bar">
        <select
          className="filter-select"
          value={status}
          onChange={(e) => handleStatusChange(e.target.value as NotificationStatusFilter)}
        >
          <option value="">All statuses</option>
          <option value="pending">Pending</option>
          <option value="processing">Processing</option>
          <option value="delivered">Delivered</option>
          <option value="failed">Failed</option>
        </select>
        <button className="btn btn-primary" onClick={handleSearch}>
          Filter
        </button>
      </div>

      {error && (
        <div className="error-banner" style={{ marginBottom: 16 }}>
          {error}
        </div>
      )}

      {searched && !loading && items.length === 0 && (
        <div className="empty-state">
          <p>No Notification Delivery Records found.</p>
        </div>
      )}

      {items.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Record ID</th>
                <th>Event Type</th>
                <th>Resource</th>
                <th>Status</th>
                <th>Attempts</th>
                <th>Last Error</th>
                <th>Updated</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {items.map((n) => (
                <tr key={n.id}>
                  <td
                    className="clickable-row"
                    style={{ fontFamily: 'monospace', fontSize: 13 }}
                    onClick={() => setSelectedId(n.id)}
                  >
                    {n.id.substring(0, 8)}...
                  </td>
                  <td style={{ fontSize: 13 }}>{n.event_type}</td>
                  <td style={{ fontSize: 13 }}>{n.resource_type}/{n.resource_id.substring(0, 8)}...</td>
                  <td>
                    <span className={`status-badge ${statusClass(n.status)}`}>
                      {n.status.charAt(0).toUpperCase() + n.status.slice(1)}
                    </span>
                  </td>
                  <td style={{ fontSize: 13 }}>{n.attempt_count}</td>
                  <td style={{ color: 'var(--color-error)', fontSize: 13, maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {n.last_error || '-'}
                  </td>
                  <td style={{ fontSize: 13 }}>{new Date(n.updated_at).toLocaleString()}</td>
                  <td>
                    <button
                      className="btn btn-primary"
                      style={{ fontSize: 12, padding: '4px 8px' }}
                      onClick={() => setSelectedId(n.id)}
                    >
                      Detail
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
          <p>Use the status filter above to find Notification Delivery Records.</p>
        </div>
      )}
    </div>
  );
}
