interface StatusCountGridProps {
  title: string;
  icon: React.ReactNode;
  counts: { label: string; count: number; className: string }[];
}

export default function StatusCountGrid({ title, icon, counts }: StatusCountGridProps) {
  return (
    <div className="section">
      {title && (
        <div className="section-header">
          <h3 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {icon}
            {title}
          </h3>
        </div>
      )}
      <div className="status-count-grid">
        {counts.map((c) => (
          <div key={c.label} className={`status-card ${c.className}`}>
            <span className="count">{c.count}</span>
            <span className="label">{c.label}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
