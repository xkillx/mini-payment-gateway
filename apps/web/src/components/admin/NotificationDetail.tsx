import { useState, useEffect } from 'react';
import { ArrowLeft, RefreshCw } from 'lucide-react';
import { getNotification } from '../../api/client';
import type { NotificationDeliveryRecordDetail } from '../../api/types';

interface NotificationDetailProps {
  notificationId: string;
  onBack: () => void;
  onRetry: (id: string) => Promise<void>;
}

export default function NotificationDetail({
  notificationId,
  onBack,
  onRetry,
}: NotificationDetailProps) {
  const [notification, setNotification] =
    useState<NotificationDeliveryRecordDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [retrying, setRetrying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const n = await getNotification(notificationId);
      setNotification(n);
    } catch {
      setError('Notification not found.');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, [notificationId]);

  const handleRetry = async () => {
    setRetrying(true);
    setError(null);
    try {
      await onRetry(notificationId);
    } catch {
      setError('Failed to retry notification.');
    } finally {
      setRetrying(false);
    }
  };

  if (loading) {
    return <div style={{ padding: 24 }}>Loading...</div>;
  }

  if (error || !notification) {
    return (
      <div style={{ padding: 24 }}>
        <div className="error-banner">{error || 'Notification not found'}</div>
        <button
          className="btn btn-secondary"
          onClick={onBack}
          style={{ marginTop: 12 }}
        >
          <ArrowLeft size={16} />
          Back
        </button>
      </div>
    );
  }

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
    }
  };

  return (
    <div>
      <button
        className="btn btn-secondary"
        onClick={onBack}
        style={{ marginBottom: 16 }}
      >
        <ArrowLeft size={16} />
        Back
      </button>

      <div
        className="section"
        style={{
          background: 'var(--color-surface)',
          padding: 24,
          borderRadius: 'var(--radius)',
          boxShadow: 'var(--shadow)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 20,
          }}
        >
          <h3>Notification Detail</h3>
          <span className={`status-badge ${statusClass(notification.status)}`}>
            {notification.status.charAt(0).toUpperCase() + notification.status.slice(1)}
          </span>
        </div>

        {notification.status === 'failed' && (
          <div className="detail-actions">
            <button
              className="btn btn-primary"
              onClick={handleRetry}
              disabled={retrying}
            >
              <RefreshCw size={16} />
              {retrying ? 'Retrying...' : 'Retry'}
            </button>
          </div>
        )}

        <div className="detail-grid">
          <span className="detail-label">Record ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {notification.id}
          </span>

          <span className="detail-label">Domain Event ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {notification.domain_event_id}
          </span>

          <span className="detail-label">Event Type</span>
          <span className="detail-value">{notification.event_type}</span>

          <span className="detail-label">Resource Type</span>
          <span className="detail-value">{notification.resource_type}</span>

          <span className="detail-label">Resource ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {notification.resource_id}
          </span>

          <span className="detail-label">Payment ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {notification.payment_id}
          </span>

          {notification.refund_id && (
            <>
              <span className="detail-label">Refund ID</span>
              <span className="detail-value" style={{ fontFamily: 'monospace' }}>
                {notification.refund_id}
              </span>
            </>
          )}

          <span className="detail-label">Destination</span>
          <span
            className="detail-value"
            style={{ fontSize: 13, wordBreak: 'break-all' }}
          >
            {notification.destination_url}
          </span>

          <span className="detail-label">Attempt Count</span>
          <span className="detail-value">{notification.attempt_count}</span>

          <span className="detail-label">Retry Generation</span>
          <span className="detail-value">{notification.retry_generation}</span>

          {notification.last_attempt_at && (
            <>
              <span className="detail-label">Last Attempt</span>
              <span className="detail-value">
                {new Date(notification.last_attempt_at).toLocaleString()}
              </span>
            </>
          )}

          {notification.next_retry_at && (
            <>
              <span className="detail-label">Next Retry</span>
              <span className="detail-value">
                {new Date(notification.next_retry_at).toLocaleString()}
              </span>
            </>
          )}

          {notification.last_error && (
            <>
              <span className="detail-label">Last Error</span>
              <span className="detail-value" style={{ color: 'var(--color-error)' }}>
                {notification.last_error}
              </span>
            </>
          )}

          <span className="detail-label">Created</span>
          <span className="detail-value">
            {new Date(notification.created_at).toLocaleString()}
          </span>

          <span className="detail-label">Updated</span>
          <span className="detail-value">
            {new Date(notification.updated_at).toLocaleString()}
          </span>
        </div>
      </div>

      {notification.attempts.length > 0 && (
        <div
          className="section"
          style={{
            background: 'var(--color-surface)',
            padding: 24,
            borderRadius: 'var(--radius)',
            boxShadow: 'var(--shadow)',
            marginTop: 16,
          }}
        >
          <h3 style={{ marginBottom: 16 }}>Attempt History</h3>
          <div className="table-wrapper">
            <table>
              <thead>
                <tr>
                  <th>Attempt</th>
                  <th>Generation</th>
                  <th>Status</th>
                  <th>HTTP Status</th>
                  <th>Error</th>
                  <th>Started</th>
                  <th>Finished</th>
                </tr>
              </thead>
              <tbody>
                {notification.attempts.map((a) => (
                  <tr key={a.id}>
                    <td style={{ fontSize: 13 }}>{a.attempt_number}</td>
                    <td style={{ fontSize: 13 }}>{a.retry_generation}</td>
                    <td>
                      <span className={`status-badge ${statusClass(a.status)}`}>
                        {a.status}
                      </span>
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {a.http_status_code ?? '-'}
                    </td>
                    <td
                      style={{
                        color: 'var(--color-error)',
                        fontSize: 13,
                        maxWidth: 200,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {a.error || '-'}
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {new Date(a.started_at).toLocaleString()}
                    </td>
                    <td style={{ fontSize: 13 }}>
                      {a.finished_at
                        ? new Date(a.finished_at).toLocaleString()
                        : '-'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
