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
        <h2>PayFlow Mini</h2>
        <p>Enter your JWT to access the appropriate dashboard.</p>
        <input
          type="text"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          placeholder="Paste your JWT here"
        />
        <button className="btn btn-primary" type="submit" disabled={!token.trim()}>
          Access dashboard
        </button>
        {error && <div className="error-banner">{error}</div>}
      </form>
    </div>
  );
}
