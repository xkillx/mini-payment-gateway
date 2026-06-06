import { ArrowLeft } from 'lucide-react';
import type { AuditRecord } from '../../api/types';

interface AuditRecordDetailProps {
  record: AuditRecord;
  onBack: () => void;
}

export default function AuditRecordDetail({
  record,
  onBack,
}: AuditRecordDetailProps) {
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
        <h3 style={{ marginBottom: 20 }}>Audit Record Detail</h3>

        <div className="detail-grid">
          <span className="detail-label">Record ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {record.id}
          </span>

          <span className="detail-label">Actor Type</span>
          <span className="detail-value">{record.actor_type}</span>

          <span className="detail-label">Actor ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {record.actor_id || 'N/A'}
          </span>

          <span className="detail-label">Action</span>
          <span className="detail-value">{record.action}</span>

          <span className="detail-label">Resource Type</span>
          <span className="detail-value">{record.resource_type}</span>

          <span className="detail-label">Resource ID</span>
          <span className="detail-value" style={{ fontFamily: 'monospace' }}>
            {record.resource_id}
          </span>

          <span className="detail-label">Occurred At</span>
          <span className="detail-value">
            {new Date(record.occurred_at).toLocaleString()}
          </span>

          <span className="detail-label">Created At</span>
          <span className="detail-value">
            {new Date(record.created_at).toLocaleString()}
          </span>
        </div>

        {record.details && (
          <div style={{ marginTop: 16 }}>
            <span className="detail-label" style={{ display: 'block', marginBottom: 8 }}>
              Details
            </span>
            <pre
              style={{
                background: 'var(--color-neutral-bg)',
                padding: 12,
                borderRadius: 6,
                fontSize: 13,
                overflow: 'auto',
                maxHeight: 400,
                fontFamily: 'monospace',
              }}
            >
              {JSON.stringify(record.details, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
