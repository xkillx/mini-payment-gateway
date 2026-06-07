import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import PaymentReporting from '../PaymentReporting';
import type { PaymentSummaryReport } from '../../../api/types';

vi.mock('../../../api/client', () => ({
  getPaymentSummaryReport: vi.fn(),
}));

import { getPaymentSummaryReport } from '../../../api/client';

const mockReport: PaymentSummaryReport = {
  configured_currency: 'USD',
  generated_at: '2040-01-04T12:00:00Z',
  period_start: '2040-01-01T00:00:00Z',
  period_end: '2040-01-04T00:00:00Z',
  payment_totals: {
    created_count: 3,
    created_amount_minor: 17000,
    successful_count: 1,
    successful_amount_minor: 10000,
    failed_count: 1,
    failed_attempted_amount_minor: 2000,
  },
  refund_activity: {
    completed_count: 1,
    completed_amount_minor: 10000,
    failed_count: 1,
  },
  trend: [
    {
      bucket_date: '2040-01-01',
      created_count: 3,
      successful_count: 1,
      failed_count: 1,
      successful_amount_minor: 10000,
    },
    {
      bucket_date: '2040-01-02',
      created_count: 0,
      successful_count: 0,
      failed_count: 0,
      successful_amount_minor: 0,
    },
    {
      bucket_date: '2040-01-03',
      created_count: 0,
      successful_count: 0,
      failed_count: 0,
      successful_amount_minor: 0,
    },
  ],
};

describe('PaymentReporting', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads default report on mount with no params', async () => {
    vi.mocked(getPaymentSummaryReport).mockResolvedValue(mockReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(getPaymentSummaryReport).toHaveBeenCalledWith({});
    });
  });

  it('renders metric labels and values', async () => {
    vi.mocked(getPaymentSummaryReport).mockResolvedValue(mockReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText('Created Payments')).toBeDefined();
      expect(screen.getByText('Successful Payments')).toBeDefined();
      expect(screen.getByText('Failed Payments')).toBeDefined();
      expect(screen.getByText('Completed Refunds')).toBeDefined();
    });

    const metricCards = document.querySelectorAll('.metric-count');
    const counts = Array.from(metricCards).map((el) => el.textContent);
    expect(counts).toContain('3');
    expect(counts).toContain('1');
  });

  it('renders trend bucket rows', async () => {
    vi.mocked(getPaymentSummaryReport).mockResolvedValue(mockReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText('Daily Payment Trend')).toBeDefined();
      expect(screen.getByText('2040-01-01')).toBeDefined();
      expect(screen.getByText('2040-01-02')).toBeDefined();
      expect(screen.getByText('2040-01-03')).toBeDefined();
    });
  });

  it('renders period info', async () => {
    vi.mocked(getPaymentSummaryReport).mockResolvedValue(mockReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText(/2040-01-01T00:00:00Z/)).toBeDefined();
      expect(screen.getByText(/2040-01-04T00:00:00Z/)).toBeDefined();
    });
  });

  it('calls API with selected from/to when Apply is clicked', async () => {
    vi.mocked(getPaymentSummaryReport)
      .mockResolvedValueOnce(mockReport)
      .mockResolvedValueOnce(mockReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText('Payment Reporting')).toBeDefined();
    });

    const inputs = screen.getAllByPlaceholderText('YYYY-MM-DDTHH:MM');
    fireEvent.change(inputs[0], { target: { value: '2040-06-01T00:00' } });
    fireEvent.click(screen.getByText('Apply'));

    await waitFor(() => {
      expect(getPaymentSummaryReport).toHaveBeenCalledWith({
        from: '2040-06-01T00:00:00Z',
      });
    });
  });

  it('shows error message on API failure', async () => {
    vi.mocked(getPaymentSummaryReport).mockImplementation(() =>
      Promise.reject(new Error('API error')),
    );

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText('Failed to load payment report.')).toBeDefined();
    });
  });

  it('shows empty state when no report loaded', async () => {
    vi.mocked(getPaymentSummaryReport).mockResolvedValue(null as unknown as PaymentSummaryReport);

    render(<PaymentReporting />);

    await waitFor(() => {
      expect(screen.getByText('No report data available.')).toBeDefined();
    });
  });
});
