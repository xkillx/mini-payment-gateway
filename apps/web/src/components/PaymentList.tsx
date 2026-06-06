import { useState, useCallback } from 'react';
import type { PaymentStatus } from '../api/types';
import PaymentSearch from './PaymentSearch';
import PaymentDetail from './PaymentDetail';
import { listPayments } from '../api/client';
import type { PaymentListItem } from '../api/types';
import { formatCurrency, getMerchantReference } from '../utils/format';

interface PaymentListProps {
  currency: string;
  mode?: 'merchant' | 'administrator';
  showMerchantFilter?: boolean;
}

const PAGE_SIZE = 20;

export default function PaymentList({
  currency: _currency,
  mode = 'merchant',
  showMerchantFilter = false,
}: PaymentListProps) {
  const [search, setSearch] = useState('');
  const [status, setStatus] = useState<PaymentStatus | ''>('');
  const [merchantId, setMerchantId] = useState('');
  const [items, setItems] = useState<PaymentListItem[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searched, setSearched] = useState(false);

  const isAdmin = mode === 'administrator';

  const load = useCallback(
    async (newOffset: number, isReset: boolean) => {
      setLoading(true);
      setError(null);
      try {
        const params: {
          status?: string;
          search?: string;
          merchantId?: string;
          limit: number;
          offset: number;
        } = {
          status: status || undefined,
          search: search.trim() || undefined,
          limit: PAGE_SIZE,
          offset: newOffset,
        };
        if (isAdmin && merchantId.trim()) {
          params.merchantId = merchantId.trim();
        }
        const res = await listPayments(params);
        if (isReset) {
          setItems(res.items);
        } else {
          setItems((prev) => [...prev, ...res.items]);
        }
        setHasMore(res.items.length === PAGE_SIZE);
        setOffset(newOffset);
        setSearched(true);
      } catch {
        setError('Failed to load payments.');
      } finally {
        setLoading(false);
      }
    },
    [search, status, merchantId, isAdmin]
  );

  const handleSearch = () => {
    setSearched(false);
    load(0, true);
  };

  const handleStatusChange = (newStatus: PaymentStatus | '') => {
    setStatus(newStatus);
    setSearched(false);
  };

  const handleLoadMore = () => {
    load(offset + PAGE_SIZE, false);
  };

  const statusClass = (s: PaymentStatus) => {
    switch (s) {
      case 'pending':
        return 'pending';
      case 'processing':
        return 'processing';
      case 'successful':
        return 'successful';
      case 'failed':
        return 'failed';
      case 'refunded':
        return 'refunded';
    }
  };

  if (selectedId) {
    return (
      <PaymentDetail
        paymentId={selectedId}
        onBack={() => setSelectedId(null)}
        canRequestRefund={!isAdmin}
      />
    );
  }

  return (
    <div>
      <PaymentSearch
        search={search}
        onSearchChange={setSearch}
        status={status}
        onStatusChange={handleStatusChange}
        onSearch={handleSearch}
      />

      {isAdmin && showMerchantFilter && (
        <div className="search-bar">
          <input
            className="search-input"
            type="text"
            placeholder="Filter by Merchant Account ID (UUID)"
            value={merchantId}
            onChange={(e) => setMerchantId(e.target.value)}
          />
          <button className="btn btn-primary" onClick={handleSearch}>
            Filter
          </button>
        </div>
      )}

      {error && (
        <div className="error-banner" style={{ marginBottom: 16 }}>
          {error}
        </div>
      )}

      {searched && !loading && items.length === 0 && (
        <div className="empty-state">
          <p>No Payments found matching your search.</p>
          <p style={{ fontSize: 13 }}>
            Try clearing the search or searching by Payment ID / Merchant Reference.
          </p>
        </div>
      )}

      {items.length > 0 && (
        <div className="table-wrapper">
          <table>
            <thead>
              <tr>
                <th>Payment ID</th>
                {isAdmin && <th>Merchant ID</th>}
                <th>Reference</th>
                <th>Amount</th>
                <th>Status</th>
                <th>Created</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {items.map((p) => (
                <tr
                  key={p.id}
                  className="clickable-row"
                  onClick={() => setSelectedId(p.id)}
                >
                  <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                    {p.id.substring(0, 8)}...
                  </td>
                  {isAdmin && (
                    <td style={{ fontFamily: 'monospace', fontSize: 13 }}>
                      {p.merchant_id.substring(0, 8)}...
                    </td>
                  )}
                  <td>{getMerchantReference(p.metadata) || '-'}</td>
                  <td>
                    {formatCurrency(p.amount_minor, p.currency)}
                  </td>
                  <td>
                    <span className={`status-badge ${statusClass(p.status)}`}>
                      {p.status.charAt(0).toUpperCase() + p.status.slice(1)}
                    </span>
                  </td>
                  <td style={{ fontSize: 13 }}>
                    {new Date(p.created_at).toLocaleString()}
                  </td>
                  <td></td>
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
          <p>
            Use the search above to find Payments by Payment ID or Merchant Reference.
          </p>
        </div>
      )}
    </div>
  );
}
