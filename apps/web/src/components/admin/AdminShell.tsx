import { useState } from 'react';
import {
  LayoutDashboard,
  CreditCard,
  Undo2,
  Bell,
  Scale,
  FileText,
  RefreshCw,
  LogOut,
  Menu,
} from 'lucide-react';
import type { AdminDashboardSummary } from '../../api/types';
import AdminOverview from './AdminOverview';
import PaymentList from '../PaymentList';
import RefundList from '../RefundList';
import NotificationMonitoring from './NotificationMonitoring';
import ReconciliationReports from './ReconciliationReports';
import AuditRecords from './AuditRecords';

type View =
  | 'overview'
  | 'payments'
  | 'refunds'
  | 'notifications'
  | 'reconciliation'
  | 'audit';

interface AdminShellProps {
  summary: AdminDashboardSummary;
  onRefresh: () => void;
  onLogout: () => void;
}

export default function AdminShell({ summary, onRefresh, onLogout }: AdminShellProps) {
  const [view, setView] = useState<View>('overview');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems: { view: View; label: string; icon: React.ReactNode }[] = [
    { view: 'overview', label: 'Overview', icon: <LayoutDashboard size={20} /> },
    { view: 'payments', label: 'Payments', icon: <CreditCard size={20} /> },
    { view: 'refunds', label: 'Refunds', icon: <Undo2 size={20} /> },
    { view: 'notifications', label: 'Notifications', icon: <Bell size={20} /> },
    { view: 'reconciliation', label: 'Reconciliation', icon: <Scale size={20} /> },
    { view: 'audit', label: 'Audit Records', icon: <FileText size={20} /> },
  ];

  const viewLabel = (v: View) => {
    switch (v) {
      case 'overview': return 'Overview';
      case 'payments': return 'Payments';
      case 'refunds': return 'Refunds';
      case 'notifications': return 'Notifications';
      case 'reconciliation': return 'Reconciliation Reports';
      case 'audit': return 'Audit Records';
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-brand">MPG Admin</div>
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
            <button
              className="btn-icon mobile-menu-btn"
              onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
            >
              <Menu size={20} />
            </button>
            <h1>{viewLabel(view)}</h1>
          </div>
          <div className="topbar-right">
            <span className="last-updated">
              Last updated: {new Date(summary.generated_at).toLocaleTimeString()}
            </span>
            <button className="btn btn-secondary" onClick={onRefresh}>
              <RefreshCw size={16} />
              Refresh
            </button>
          </div>
        </div>

        <div className="content">
          {view === 'overview' && <AdminOverview summary={summary} />}
          {view === 'payments' && (
            <PaymentList currency={summary.configured_currency} mode="administrator" />
          )}
          {view === 'refunds' && (
            <RefundList currency={summary.configured_currency} mode="administrator" />
          )}
          {view === 'notifications' && <NotificationMonitoring />}
          {view === 'reconciliation' && <ReconciliationReports />}
          {view === 'audit' && <AuditRecords />}
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
