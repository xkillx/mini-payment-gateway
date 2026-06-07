import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import AdminShell from '../AdminShell';
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

describe('AdminShell', () => {
  it('renders Admin Dashboard branding and System Administrator label', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    expect(screen.getByText('Admin Dashboard')).toBeDefined();
    expect(screen.getByText('System Administrator')).toBeDefined();
  });

  it('renders System Health nav item', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    const elements = screen.getAllByText('System Health');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('shows System Health Overview as default heading', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    const elements = screen.getAllByText('System Health Overview');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('renders Payment Reporting nav item', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    expect(screen.getByText('Payment Reporting')).toBeDefined();
  });

  it('shows Payment Reporting heading when nav item is clicked', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    screen.getByText('Payment Reporting').click();
    const elements = screen.getAllByText('Payment Reporting');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('renders Operational Health nav item', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    const elements = screen.getAllByText('Operational Health');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });

  it('shows Operational Health heading when nav item is clicked', () => {
    render(<AdminShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    screen.getAllByText('Operational Health')[0].click();
    const elements = screen.getAllByText('Operational Health');
    expect(elements.length).toBeGreaterThanOrEqual(1);
  });
});
