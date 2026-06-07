import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import DashboardOverview from '../DashboardOverview';
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

describe('DashboardOverview', () => {
  it('renders Total Payments count (sum of all status counts)', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    const total = 3 + 1 + 10 + 2 + 1;
    expect(screen.getByText(String(total))).toBeDefined();
  });

  it('renders Successful Payments count', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('10')).toBeDefined();
  });

  it('renders Failed Payments count', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('2')).toBeDefined();
  });

  it('does not render "Transaction" as primary label', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    const elements = screen.queryAllByText(/Transaction/);
    expect(elements).toHaveLength(0);
  });

  it('does not render "Customer" text', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    const elements = screen.queryAllByText(/Customer/);
    expect(elements).toHaveLength(0);
  });

  it('renders Create Payment button', () => {
    render(<DashboardOverview summary={summary} onRefresh={() => {}} />);

    expect(screen.getByText('Create Payment')).toBeDefined();
  });
});
