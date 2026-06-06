interface StatusFilterProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  options: { value: string; label: string }[];
}

export default function StatusFilter({ value, onChange, options }: StatusFilterProps) {
  return (
    <select
      className="filter-select"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      aria-label="Status filter"
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}
