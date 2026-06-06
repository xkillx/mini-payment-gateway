import { useState } from 'react';

interface AccessGateProps {
  onTokenSubmit: (token: string) => void;
  error: string | null;
}

export default function AccessGate({ onTokenSubmit, error }: AccessGateProps) {
  const [token, setToken] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = token.trim();
    if (trimmed) {
      onTokenSubmit(trimmed);
    }
  };

  return (
    <div className="page-center">
      <form className="access-gate" onSubmit={handleSubmit}>
        <h2>Merchant Dashboard</h2>
        <p>Enter a Merchant JWT to access your dashboard.</p>
        <input
          type="text"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder="Paste your Merchant JWT here"
        />
        <button className="btn btn-primary" type="submit" disabled={!token.trim()}>
          Enter dashboard
        </button>
        {error && <div className="error-banner">{error}</div>}
      </form>
    </div>
  );
}
