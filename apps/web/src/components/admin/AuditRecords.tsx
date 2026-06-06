import { useState, useCallback } from 'react';
import { listAuditRecords } from '../../api/client';
import type { AuditRecord } from '../../api/types';
import AuditRecordDetail from './AuditRecordDetail';

const PAGE_SIZE = 20;

export default function AuditRecords() {
  const [items, setItems] = useState<AuditRecord[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [selectedRecord, setSelectedRecord] = useState<AuditRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const [actorType, setActorType] = useState('');
  const [resourceType, setResourceType] = useState('');
  const [action, setAction] = useState('');

  const load = useCallback(
    async (newOffset: number, isReset: boolean) => {
      setLoading(true);
      setError(null);
      try {
        const res = await listAuditRecords({
          actor_type: actorType || undefined,
          resource_type: resourceType || undefined,
          action: action || undefined,
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
        setError('Failed to load audit records.');
      } finally {
        setLoading(false);
      }
    },
    [actorType, resourceType, action]
  );

  const handleSearch = () => {
    setSearched(false);
    load(0, true);
  };

  const handleLoadMore = () => {
    load(offset + PAGE_SIZE, false);
  };

  if (selectedRecord) {
    return (
      <AuditRecordDetail
        record={selectedRecord}
        onBack={() => setSelectedRecord(null)}
      />
    );
  }

  return (
    <div>
      <div className="search-bar">
        <select
          className="filter-select"
          value={actorType}
          onChange={(e) => setActorType(e.target.value)}
        >
          <option value="">All Actor Types</option>
          <option value="merchant">Merchant</option>
          <option value="administrator">Administrator</option>
          <option value="system">System</option>
          <option value="unknown">Unknown</option>
        </select>
        <select
          className="filter-select"
          value={resourceType}
          onChange={(e) => setResourceType(e.target.value)}
        >
          <option value="">All Resource Types</option>
          <option value="payment">Payment</option>
          <option value="refund">Refund</option>
          <option value="auth">Auth</option>
          <option value="reconciliation">Reconciliation</option>
          <option value="notification_delivery_record">Notification</option>
        </select>
        <select
          className="filter-select"
          value={action}
          onChange={(e) => setAction(e.target.value)}
        >
          <option value="">All Actions</option>
          <option value="auth.authentication_failed">Auth Failure</option>
          <option value="auth.authorization_failed">Authorization Failure</option>
          <option value="payment.created">Payment Created</option>
          <option value="payment.failed">Payment Failed</option>
          <option value="refund.rejected">Refund Rejected</option>
          <option value="notification.retry_requested">Notification Retry</option>
        </select>
        <button className="btn btn-primary" onClick={handleSearch}>
          Search
        </button>
      </div>

      {error && (
        <div className="error-banner" style={{ marginBottom: 16 }}>
          {error}
        </div>
      )}

      {searched && !loading && items.length === 0 && (
        <div className="empty-state">
          <p>No Audit Records found matching your filters.</p>
        </div>
      )}

      {items.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>ID</th>
                <th>Actor</th>
                <th>Action</th>
                <th>Resource</th>
                <th>Occurred</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {items.map((r) => (
                <tr key={r.id}>
                  <td
                    className="clickable-row"
                    style={{ fontFamily: 'monospace', fontSize: 13 }}
                    onClick={() => setSelectedRecord(r)}
                  >
                    {r.id.substring(0, 8)}...
                  </td>
                  <td style={{ fontSize: 13 }}>
                    {r.actor_type}
                    {r.actor_id ? ` (${r.actor_id.substring(0, 8)}...)` : ''}
                  </td>
                  <td style={{ fontSize: 13 }}>{r.action}</td>
                  <td style={{ fontSize: 13 }}>
                    {r.resource_type}/{r.resource_id}
                  </td>
                  <td style={{ fontSize: 13 }}>
                    {new Date(r.occurred_at).toLocaleString()}
                  </td>
                  <td>
                    <button
                      className="btn btn-secondary"
                      style={{ fontSize: 12, padding: '4px 8px' }}
                      onClick={() => setSelectedRecord(r)}
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
          <p>Use the filters above to search Audit Records.</p>
        </div>
      )}
    </div>
  );
}
