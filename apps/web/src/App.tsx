import { useState, useEffect, useCallback } from 'react';
import { getToken, setToken, clearToken, hasToken } from './auth/tokenStore';
import { getDashboardSummary, ApiClientError } from './api/client';
import type { DashboardSummary } from './api/types';
import AccessGate from './components/AccessGate';
import AppShell from './components/AppShell';

type AuthState =
  | { kind: 'loading' }
  | { kind: 'no_token' }
  | { kind: 'authenticated'; token: string };

export default function App() {
  const [auth, setAuth] = useState<AuthState>({ kind: 'loading' });
  const [summary, setSummary] = useState<DashboardSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadSummary = useCallback(async () => {
    try {
      const s = await getDashboardSummary();
      setSummary(s);
      setError(null);
    } catch (e) {
      if (e instanceof ApiClientError) {
        if (e.status === 401) {
          clearToken();
          setAuth({ kind: 'no_token' });
          setSummary(null);
          return;
        }
        if (e.status === 403) {
          setError('Merchant access required. Please use a Merchant token.');
          return;
        }
      }
      setError('Failed to load dashboard summary.');
    }
  }, []);

  useEffect(() => {
    if (hasToken()) {
      const token = getToken()!;
      setAuth({ kind: 'authenticated', token });
    } else {
      setAuth({ kind: 'no_token' });
    }
  }, []);

  useEffect(() => {
    if (auth.kind === 'authenticated') {
      loadSummary();
    }
  }, [auth.kind, loadSummary]);

  const handleTokenSubmit = (token: string) => {
    setToken(token);
    setAuth({ kind: 'authenticated', token });
    setError(null);
  };

  const handleLogout = () => {
    clearToken();
    setAuth({ kind: 'no_token' });
    setSummary(null);
    setError(null);
  };

  if (auth.kind === 'loading') {
    return <div className="page-center">Loading...</div>;
  }

  if (auth.kind === 'no_token' || auth.kind === 'authenticated' && !summary && !error) {
    return <AccessGate onTokenSubmit={handleTokenSubmit} error={error} />;
  }

  if (error) {
    return (
      <div className="page-center">
        <div className="error-banner">{error}</div>
        <button className="btn btn-primary" onClick={handleLogout}>
          Return to access gate
        </button>
      </div>
    );
  }

  return (
    <AppShell
      summary={summary!}
      onRefresh={loadSummary}
      onLogout={handleLogout}
    />
  );
}
