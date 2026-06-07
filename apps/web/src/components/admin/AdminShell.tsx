import { useState } from 'react';
import {
  Activity,
  CreditCard,
  Undo2,
  Bell,
  Scale,
  FileText,
  BarChart3,
  HeartPulse,
  RefreshCw,
  LogOut,
  Menu,
} from 'lucide-react';
import type { AdminDashboardSummary, OperationalHealthFailedOperation } from '../../api/types';
import AdminOverview from './AdminOverview';
import OperationalHealthView from './OperationalHealthView';
import PaymentList from '../PaymentList';
import RefundList from '../RefundList';
import NotificationMonitoring from './NotificationMonitoring';
import ReconciliationReports from './ReconciliationReports';
import AuditRecords from './AuditRecords';
import PaymentReporting from './PaymentReporting';

type View =
  | 'overview'
  | 'operational_health'
  | 'payments'
  | 'refunds'
  | 'notifications'
  | 'reconciliation'
  | 'reporting'
  | 'audit';

interface AdminShellProps {
  summary: AdminDashboardSummary;
  onRefresh: () => void;
  onLogout: () => void;
}

export default function AdminShell({ summary, onRefresh, onLogout }: AdminShellProps) {
  const [view, setView] = useState<View>('overview');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [initialPaymentId, setInitialPaymentId] = useState<string | undefined>(undefined);
  const [initialRefundId, setInitialRefundId] = useState<string | undefined>(undefined);
  const [initialNotificationId, setInitialNotificationId] = useState<string | undefined>(undefined);
  const [initialReconciliationId, setInitialReconciliationId] = useState<string | undefined>(undefined);

  const handleOpenOperation = (op: OperationalHealthFailedOperation) => {
    switch (op.kind) {
      case 'payment':
        setInitialPaymentId(op.id);
        setView('payments');
        break;
      case 'refund':
        setInitialRefundId(op.id);
        setView('refunds');
        break;
      case 'notification_delivery_record':
        setInitialNotificationId(op.id);
        setView('notifications');
        break;
      case 'reconciliation':
        setInitialReconciliationId(op.id);
        setView('reconciliation');
        break;
    }
  };

  const navItems: { view: View; label: string; icon: React.ReactNode }[] = [
    { view: 'overview', label: 'System Health', icon: <Activity size={20} /> },
    { view: 'operational_health', label: 'Operational Health', icon: <HeartPulse size={20} /> },
    { view: 'payments', label: 'Payments', icon: <CreditCard size={20} /> },
    { view: 'refunds', label: 'Refunds', icon: <Undo2 size={20} /> },
    { view: 'notifications', label: 'Notifications', icon: <Bell size={20} /> },
    { view: 'reconciliation', label: 'Reconciliation', icon: <Scale size={20} /> },
    { view: 'reporting', label: 'Payment Reporting', icon: <BarChart3 size={20} /> },
    { view: 'audit', label: 'Audit Records', icon: <FileText size={20} /> },
  ];

  const viewLabel = (v: View) => {
    switch (v) {
      case 'overview': return 'System Health Overview';
      case 'operational_health': return 'Operational Health';
      case 'payments': return 'Payments';
      case 'refunds': return 'Refunds';
      case 'notifications': return 'Notifications';
      case 'reconciliation': return 'Reconciliation';
      case 'reporting': return 'Payment Reporting';
      case 'audit': return 'Audit Records';
    }
  };

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-brand">Admin Dashboard</div>
        <div className="sidebar-subtitle">System Administrator</div>
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
              aria-label="Toggle menu"
            >
              <Menu size={20} />
            </button>
            <h1>{viewLabel(view)}</h1>
          </div>
          <div className="topbar-right">
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
          {view === 'overview' && <AdminOverview summary={summary} onRefresh={onRefresh} />}
          {view === 'operational_health' && (
            <OperationalHealthView
              operationalHealth={summary.operational_health}
              windowStart={summary.window_start}
              windowEnd={summary.window_end}
              onOpenOperation={handleOpenOperation}
            />
          )}
          {view === 'payments' && (
            <PaymentList
              currency={summary.configured_currency}
              mode="administrator"
              searchSeed={initialPaymentId}
              onClearSearchSeed={() => setInitialPaymentId(undefined)}
              initialPaymentId={initialPaymentId}
            />
          )}
          {view === 'refunds' && (
            <RefundList
              currency={summary.configured_currency}
              mode="administrator"
              initialRefundId={initialRefundId}
            />
          )}
          {view === 'notifications' && (
            <NotificationMonitoring initialNotificationId={initialNotificationId} />
          )}
          {view === 'reconciliation' && (
            <ReconciliationReports initialReconciliationId={initialReconciliationId} />
          )}
          {view === 'reporting' && <PaymentReporting />}
          {view === 'audit' && <AuditRecords />}
        </div>
      </div>

      <nav className="mobile-nav">
        {navItems.slice(0, 5).map((item) => (
          <button
            key={item.view}
            className={view === item.view ? 'active' : ''}
            onClick={() => setView(item.view)}
          >
            {item.icon}
            <span style={{ fontSize: 9 }}>{item.label}</span>
          </button>
        ))}
      </nav>
    </div>
  );
}
