import { useState } from 'react';
import { CreditCard, Undo2, Plus } from 'lucide-react';
import type { DashboardSummary } from '../api/types';
import StatusCountGrid from './StatusCountGrid';
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

  return (
    <div>
      <div className="section-header" style={{ marginBottom: 16 }}>
        <h3 style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <CreditCard size={16} />
          Payment Overview
        </h3>
        <button className="btn btn-primary" onClick={() => setShowCreatePayment(true)}>
          <Plus size={16} />
          Create Payment
        </button>
      </div>

      <StatusCountGrid
        title=""
        icon={null}
        counts={[
          { label: 'Pending', count: payment_overview.status_counts.pending, className: 'pending' },
          { label: 'Processing', count: payment_overview.status_counts.processing, className: 'processing' },
          { label: 'Successful', count: payment_overview.status_counts.successful, className: 'successful' },
          { label: 'Failed', count: payment_overview.status_counts.failed, className: 'failed' },
          { label: 'Refunded', count: payment_overview.status_counts.refunded, className: 'refunded' },
        ]}
      />

      <RecentPayments payments={payment_overview.recent_payments} />

      <StatusCountGrid
        title="Refund Overview"
        icon={<Undo2 size={16} />}
        counts={[
          { label: 'Pending', count: refund_overview.status_counts.pending, className: 'pending' },
          { label: 'Processing', count: refund_overview.status_counts.processing, className: 'processing' },
          { label: 'Completed', count: refund_overview.status_counts.completed, className: 'completed' },
          { label: 'Failed', count: refund_overview.status_counts.failed, className: 'failed' },
        ]}
      />

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
