interface PaymentMetadataViewProps {
  metadata: Record<string, unknown>;
}

export default function PaymentMetadataView({ metadata }: PaymentMetadataViewProps) {
  const entries = Object.entries(metadata);

  if (entries.length === 0) return null;

  return (
    <div style={{ marginTop: 20 }}>
      <h4 style={{ fontSize: 14, marginBottom: 12, color: 'var(--color-text-secondary)' }}>Additional Metadata</h4>
      <div className="metadata-list">
        {entries.map(([key, value]) => (
          <div key={key} className="metadata-row">
            <span className="metadata-key">{key}</span>
            <span style={{ fontFamily: 'monospace', fontSize: 12 }}>
              {typeof value === 'object' ? JSON.stringify(value) : String(value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
