import { useState, useEffect, useCallback } from 'react';
import { getToken, setToken, clearToken, hasToken } from './auth/tokenStore';
import { getTokenRole, type DashboardRole } from './auth/tokenRole';
import {
  getDashboardSummary,
  getAdminDashboardSummary,
  ApiClientError,
} from './api/client';
import type { DashboardSummary, AdminDashboardSummary } from './api/types';
import AccessGate from './components/AccessGate';
import AppShell from './components/AppShell';
import AdminShell from './components/admin/AdminShell';

type AuthState =
  | { kind: 'loading' }
  | { kind: 'no_token' }
  | { kind: 'no_role'; token: string }
  | { kind: 'authenticated'; token: string; role: DashboardRole };

export default function App() {
  const [auth, setAuth] = useState<AuthState>({ kind: 'loading' });
  const [merchantSummary, setMerchantSummary] = useState<DashboardSummary | null>(null);
  const [adminSummary, setAdminSummary] = useState<AdminDashboardSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadSummary = useCallback(async (role: DashboardRole) => {
    try {
      if (role === 'merchant') {
        const s = await getDashboardSummary();
        setMerchantSummary(s);
        setAdminSummary(null);
      } else {
        const s = await getAdminDashboardSummary();
        setAdminSummary(s);
        setMerchantSummary(null);
      }
      setError(null);
    } catch (e) {
      if (e instanceof ApiClientError) {
        if (e.status === 401) {
          clearToken();
          setAuth({ kind: 'no_token' });
          setMerchantSummary(null);
          setAdminSummary(null);
          return;
        }
        if (e.status === 403) {
          setError(
            role === 'administrator'
              ? 'Administrator access required. Please use an Administrator token.'
              : 'Merchant access required. Please use a Merchant token.'
          );
          return;
        }
      }
      setError('Failed to load dashboard summary.');
    }
  }, []);

  useEffect(() => {
    if (hasToken()) {
      const token = getToken()!;
      const role = getTokenRole(token);
      if (role) {
        setAuth({ kind: 'authenticated', token, role });
      } else {
        setAuth({ kind: 'no_role', token });
      }
    } else {
      setAuth({ kind: 'no_token' });
    }
  }, []);

  useEffect(() => {
    if (auth.kind === 'authenticated') {
      loadSummary(auth.role);
    }
  }, [auth.kind, auth, loadSummary]);

  const handleTokenSubmit = (token: string) => {
    const role = getTokenRole(token);
    if (!role) {
      setError('Unrecognized JWT role. Please use a valid Merchant or Administrator token.');
      return;
    }
    setToken(token);
    setAuth({ kind: 'authenticated', token, role });
    setError(null);
  };

  const handleLogout = () => {
    clearToken();
    setAuth({ kind: 'no_token' });
    setMerchantSummary(null);
    setAdminSummary(null);
    setError(null);
  };

  if (auth.kind === 'loading') {
    return <div className="page-center">Loading...</div>;
  }

  if (auth.kind === 'no_token' || (auth.kind === 'authenticated' && !merchantSummary && !adminSummary && !error)) {
    return <AccessGate onTokenSubmit={handleTokenSubmit} error={error} />;
  }

  if (auth.kind === 'no_role') {
    return (
      <div className="page-center">
        <div className="error-banner">
          Unrecognized JWT role. Please use a valid Merchant or Administrator token.
        </div>
        <button className="btn btn-primary" onClick={handleLogout}>
          Return to access gate
        </button>
      </div>
    );
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

  if (auth.role === 'administrator' && adminSummary) {
    return (
      <AdminShell
        summary={adminSummary}
        onRefresh={() => loadSummary('administrator')}
        onLogout={handleLogout}
      />
    );
  }

  if (auth.role === 'merchant' && merchantSummary) {
    return (
      <AppShell
        summary={merchantSummary}
        onRefresh={() => loadSummary('merchant')}
        onLogout={handleLogout}
      />
    );
  }

  return null;
}
