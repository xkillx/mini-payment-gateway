import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import AdminOverview from '../AdminOverview';
import type { AdminDashboardSummary } from '../../../api/types';

const summary: AdminDashboardSummary = {
  configured_currency: 'USD',
  generated_at: '2024-01-15T10:00:00Z',
  window_start: '2024-01-14T10:00:00Z',
  window_end: '2024-01-15T10:00:00Z',
  api_status: 'reachable',
  payment_overview: {
    status_counts: { pending: 1, processing: 0, successful: 5, failed: 1, refunded: 0 },
    recent_failed_payments: [],
  },
  refund_overview: {
    status_counts: { pending: 0, processing: 0, completed: 2, failed: 0 },
    recent_failed_refunds: [],
  },
  notification_overview: {
    status_counts: { pending: 0, processing: 0, delivered: 10, failed: 1 },
    recent_failed_notifications: [],
  },
  reconciliation_overview: {
    status_counts: { matched: 3, mismatched: 0, error: 0 },
    recent_attention_reconciliations: [],
  },
  audit_overview: {
    recent_attention_audit_records: [],
  },
  operational_health: {
    payment_processing_success_rate: {
      numerator_count: 5,
      denominator_count: 6,
      in_flight_count: 1,
      rate_percent: 83.3,
    },
    notification_delivery_success_rate: {
      numerator_count: 10,
      denominator_count: 11,
      in_flight_count: 0,
      rate_percent: 90.9,
    },
    reconciliation_completion_rate: {
      numerator_count: 3,
      denominator_count: 3,
      in_flight_count: 0,
      rate_percent: 100.0,
    },
    recent_failed_operations: [],
  },
};

describe('AdminOverview', () => {
  it('renders System Health Overview heading', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('System Health Overview')).toBeDefined();
  });

  it('renders API status as UP when reachable', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('UP')).toBeDefined();
  });

  it('renders API status as DOWN when not reachable', () => {
    const downSummary = { ...summary, api_status: 'unreachable' };
    render(<AdminOverview summary={downSummary} onRefresh={() => {}} />);

    expect(screen.getByText('DOWN')).toBeDefined();
  });

  it('renders 24h window labels', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('24h Window Start')).toBeDefined();
    expect(screen.getByText('24h Window End')).toBeDefined();
  });

  it('renders payment status counts', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('Payments (24h)')).toBeDefined();
  });

  it('renders refund status counts', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('Refunds (24h)')).toBeDefined();
  });

  it('renders notification status counts', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('Notifications')).toBeDefined();
  });

  it('renders Run Manual Reconciliation CTA', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('Run Manual Reconciliation')).toBeDefined();
  });

  it('does not render unsupported uptime percentage label', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.queryByText(/99\.\d+%/)).toBeNull();
    expect(screen.queryByText(/uptime/)).toBeNull();
  });

  it('does not render unsupported latency label', () => {
    render(<AdminOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.queryByText(/latency/)).toBeNull();
  });
});
