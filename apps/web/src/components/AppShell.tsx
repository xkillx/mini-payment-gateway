import { useState } from 'react';
import { LayoutDashboard, CreditCard, Undo2, RefreshCw, LogOut, Menu, Search } from 'lucide-react';
import type { DashboardSummary } from '../api/types';
import DashboardOverview from './DashboardOverview';
import PaymentList from './PaymentList';
import RefundList from './RefundList';

type View = 'dashboard' | 'payments' | 'refunds';

interface AppShellProps {
  summary: DashboardSummary;
  onRefresh: () => void;
  onLogout: () => void;
}

export default function AppShell({ summary, onRefresh, onLogout }: AppShellProps) {
  const [view, setView] = useState<View>('dashboard');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [topSearch, setTopSearch] = useState('');
  const [paymentListKey, setPaymentListKey] = useState(0);

  const navItems: { view: View; label: string; icon: React.ReactNode }[] = [
    { view: 'dashboard', label: 'Dashboard', icon: <LayoutDashboard size={20} /> },
    { view: 'payments', label: 'Payments', icon: <CreditCard size={20} /> },
    { view: 'refunds', label: 'Refunds', icon: <Undo2 size={20} /> },
  ];

  const handleTopSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (topSearch.trim()) {
      setView('payments');
      setPaymentListKey((k) => k + 1);
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-brand">PayFlow Mini</div>
        <div className="sidebar-subtitle">Merchant Portal</div>
        <nav className="sidebar-nav">
          {navItems.map((item) => (
            <button
              key={item.view}
              className={view === item.view ? 'active' : ''}
              onClick={() => {
                setView(item.view);
                setMobileMenuOpen(false);
              }}
            >
              {item.icon}
              {item.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <button onClick={onLogout}>
            <LogOut size={20} />
            Logout
          </button>
        </div>
      </aside>

      <div className="main-area">
        <div className="topbar">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button className="btn-icon mobile-menu-btn" onClick={() => setMobileMenuOpen(!mobileMenuOpen)} aria-label="Toggle menu">
              <Menu size={20} />
            </button>
            <h1>
              {view === 'dashboard' && 'Dashboard'}
              {view === 'payments' && 'Payments'}
              {view === 'refunds' && 'Refunds'}
            </h1>
          </div>
          <div className="topbar-right">
            <form onSubmit={handleTopSearch} style={{ display: 'flex', gap: 6 }}>
              <input
                className="search-input"
                type="text"
                value={topSearch}
                onChange={(e) => setTopSearch(e.target.value)}
                placeholder="Search by Payment ID / Merchant Reference"
                aria-label="Search payments"
                style={{ minWidth: 220 }}
              />
              <button className="btn btn-secondary" type="submit" title="Search payments" aria-label="Search">
                <Search size={16} />
              </button>
            </form>
            <span className="last-updated">
              Updated: {new Date(summary.generated_at).toLocaleTimeString()}
            </span>
            <button className="btn btn-secondary" onClick={onRefresh}>
              <RefreshCw size={16} />
              Refresh
            </button>
          </div>
        </div>

        <div className="content">
          {view === 'dashboard' && <DashboardOverview summary={summary} onRefresh={onRefresh} />}
          {view === 'payments' && (
            <PaymentList
              key={paymentListKey}
              currency={summary.configured_currency}
              searchSeed={view === 'payments' && paymentListKey > 0 ? topSearch : undefined}
              onClearSearchSeed={() => setTopSearch('')}
            />
          )}
          {view === 'refunds' && <RefundList currency={summary.configured_currency} />}
        </div>
      </div>

      <nav className="mobile-nav">
        {navItems.map((item) => (
          <button
            key={item.view}
            className={view === item.view ? 'active' : ''}
            onClick={() => setView(item.view)}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </nav>
    </div>
  );
}
