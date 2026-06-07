import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import AppShell from '../AppShell';
import type { DashboardSummary } from '../../api/types';

const summary: DashboardSummary = {
  configured_currency: 'USD',
  generated_at: '2024-01-15T10:00:00Z',
  payment_overview: {
    status_counts: {
      pending: 3,
      processing: 1,
      successful: 10,
      failed: 2,
      refunded: 1,
    },
    recent_payments: [],
  },
  refund_overview: {
    status_counts: {
      pending: 0,
      processing: 0,
      completed: 1,
      failed: 0,
    },
    recent_refunds: [],
  },
};

describe('AppShell', () => {
  it('renders PayFlow Mini branding and Merchant Portal label', () => {
    render(<AppShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    expect(screen.getByText('PayFlow Mini')).toBeDefined();
    expect(screen.getByText('Merchant Portal')).toBeDefined();
  });

  it('renders Dashboard, Payments, and Refunds navigation', () => {
    render(<AppShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    const dashboardElements = screen.getAllByText('Dashboard');
    expect(dashboardElements.length).toBeGreaterThanOrEqual(1);

    const paymentElements = screen.getAllByText('Payments');
    expect(paymentElements.length).toBeGreaterThanOrEqual(1);

    const refundElements = screen.getAllByText('Refunds');
    expect(refundElements.length).toBeGreaterThanOrEqual(1);
  });

  it('shows last updated time', () => {
    render(<AppShell summary={summary} onRefresh={() => {}} onLogout={() => {}} />);

    expect(screen.getByText(/Updated:/)).toBeDefined();
  });
});
