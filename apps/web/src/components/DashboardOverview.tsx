import { useState } from 'react';
import { Wallet, CheckCircle, XCircle, Undo2, AlertTriangle, Plus } from 'lucide-react';
import type { DashboardSummary } from '../api/types';
import RecentPayments from './RecentPayments';
import RecentRefunds from './RecentRefunds';
import CreatePaymentDialog from './CreatePaymentDialog';

interface DashboardOverviewProps {
  summary: DashboardSummary;
  onRefresh: () => void;
}

export default function DashboardOverview({ summary, onRefresh }: DashboardOverviewProps) {
  const [showCreatePayment, setShowCreatePayment] = useState(false);
  const { payment_overview, refund_overview } = summary;

  const totalPayments =
    payment_overview.status_counts.pending +
    payment_overview.status_counts.processing +
    payment_overview.status_counts.successful +
    payment_overview.status_counts.failed +
    payment_overview.status_counts.refunded;

  const totalRefunds =
    refund_overview.status_counts.pending +
    refund_overview.status_counts.processing +
    refund_overview.status_counts.completed +
    refund_overview.status_counts.failed;

  return (
    <div>
      <div className="overview-header">
        <h2>Overview</h2>
        <p>Operational summary for your merchant account.</p>
      </div>

      <div className="bento-grid">
        <div className="metric-card" style={{ gridColumn: 'span 5' }}>
          <div className="metric-icon primary">
            <Wallet size={20} />
          </div>
          <span className="metric-count">{totalPayments}</span>
          <span className="metric-label">Total Payments</span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 4' }}>
          <div className="metric-icon success">
            <CheckCircle size={20} />
          </div>
          <span className="metric-count">{payment_overview.status_counts.successful}</span>
          <span className="metric-label">Successful Payments</span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 3' }}>
          <div className="metric-icon error">
            <XCircle size={20} />
          </div>
          <span className="metric-count">{payment_overview.status_counts.failed}</span>
          <span className="metric-label">Failed Payments</span>
        </div>
      </div>

      <div className="bento-grid">
        <div className="metric-card" style={{ gridColumn: 'span 6' }}>
          <div className="metric-icon primary">
            <Undo2 size={20} />
          </div>
          <span className="metric-count">{totalRefunds}</span>
          <span className="metric-label">Total Refunds</span>
          <span className="metric-subtitle">
            {refund_overview.status_counts.completed} completed, {refund_overview.status_counts.pending} pending
          </span>
        </div>

        <div className="metric-card" style={{ gridColumn: 'span 6' }}>
          <div className="metric-icon warning">
            <AlertTriangle size={20} />
          </div>
          <span className="metric-count">{refund_overview.status_counts.failed}</span>
          <span className="metric-label">Failed Refunds</span>
        </div>
      </div>

      <div className="section-header" style={{ marginTop: 8 }}>
        <h3>Recent Payments</h3>
        <button className="btn btn-primary" onClick={() => setShowCreatePayment(true)}>
          <Plus size={16} />
          Create Payment
        </button>
      </div>

      <RecentPayments payments={payment_overview.recent_payments} />

      <div className="section-header" style={{ marginTop: 8 }}>
        <h3>Recent Refunds</h3>
      </div>

      <RecentRefunds refunds={refund_overview.recent_refunds} />

      {showCreatePayment && (
        <CreatePaymentDialog
          currency={summary.configured_currency}
          onClose={() => setShowCreatePayment(false)}
          onCreated={onRefresh}
        />
      )}
    </div>
  );
}
