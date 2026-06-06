import { Search } from 'lucide-react';
import StatusFilter from './StatusFilter';
import type { PaymentStatus } from '../api/types';

interface PaymentSearchProps {
  search: string;
  onSearchChange: (value: string) => void;
  status: PaymentStatus | '';
  onStatusChange: (status: PaymentStatus | '') => void;
  onSearch: () => void;
}

export default function PaymentSearch({ search, onSearchChange, status, onStatusChange, onSearch }: PaymentSearchProps) {
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      onSearch();
    }
  };

  return (
    <div className="search-bar">
      <input
        className="search-input"
        type="text"
        value={search}
        onChange={(e) => onSearchChange(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Payment ID / Merchant Reference"
        aria-label="Search payments"
      />
      <StatusFilter
        label="Payment Status"
        value={status}
        onChange={(v) => onStatusChange(v as PaymentStatus | '')}
        options={[
          { value: '', label: 'All statuses' },
          { value: 'pending', label: 'Pending' },
          { value: 'processing', label: 'Processing' },
          { value: 'successful', label: 'Successful' },
          { value: 'failed', label: 'Failed' },
          { value: 'refunded', label: 'Refunded' },
        ]}
      />
      <button className="btn btn-primary" onClick={onSearch}>
        <Search size={16} />
        Search
      </button>
    </div>
  );
}
