import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import RunReconciliationDialog from '../RunReconciliationDialog';
import { runReconciliation } from '../../../api/client';

vi.mock('../../../api/client', () => ({
  runReconciliation: vi.fn(),
  ApiClientError: class extends Error {
    status: number;
    body: { code: string; message: string; request_id: string };
    constructor(status: number, message: string) {
      super(message);
      this.status = status;
      this.body = { code: 'TEST_ERROR', message, request_id: 'req-1' };
      this.name = 'ApiClientError';
    }
  },
}));

const mockedRunReconciliation = vi.mocked(runReconciliation);

describe('RunReconciliationDialog', () => {
  const defaultProps = {
    configuredCurrency: 'USD',
    onClose: vi.fn(),
    onCreated: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the dialog with pre-filled currency', () => {
    render(<RunReconciliationDialog {...defaultProps} />);

    const currencyInput = screen.getByLabelText(/Configured Currency/i) as HTMLInputElement;
    expect(currencyInput.value).toBe('USD');
  });

  it('shows validation error when submitting empty form', async () => {
    render(<RunReconciliationDialog {...defaultProps} />);

    fireEvent.click(screen.getByText('Run Reconciliation'));

    await waitFor(() => {
      expect(screen.getByText('Actual total must be an integer in minor units.')).toBeDefined();
    });
  });

  it('blocks submission when window end is before window start', async () => {
    render(<RunReconciliationDialog {...defaultProps} />);

    fireEvent.change(screen.getByLabelText('Window Start'), {
      target: { value: '2024-01-15T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Window End'), {
      target: { value: '2024-01-14T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Actual Reconciliation Total (minor units)'), {
      target: { value: '1000' },
    });

    fireEvent.click(screen.getByText('Run Reconciliation'));

    await waitFor(() => {
      expect(screen.getByText('Window end must be after window start.')).toBeDefined();
    });
  });

  it('calls runReconciliation with correct payload when form is valid', async () => {
    const mockResult = {
      id: 'rec-1',
      status: 'matched' as const,
      expected_total_minor: 5000,
      actual_total_minor: 5000,
      discrepancy_minor: 0,
      currency: 'USD',
      window_start: '2024-01-14T10:00:00.000Z',
      window_end: '2024-01-15T10:00:00.000Z',
      notes: 'test notes',
      run_at: '2024-01-15T10:30:00Z',
      created_at: '2024-01-15T10:30:00Z',
    };
    vi.mocked(mockedRunReconciliation).mockResolvedValueOnce(mockResult);

    render(<RunReconciliationDialog {...defaultProps} />);

    fireEvent.change(screen.getByLabelText('Window Start'), {
      target: { value: '2024-01-14T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Window End'), {
      target: { value: '2024-01-15T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Actual Reconciliation Total (minor units)'), {
      target: { value: '-500' },
    });
    fireEvent.change(screen.getByLabelText('Notes (optional)'), {
      target: { value: 'test notes' },
    });

    fireEvent.click(screen.getByText('Run Reconciliation'));

    await waitFor(() => {
      expect(mockedRunReconciliation).toHaveBeenCalledWith({
        currency: 'USD',
        window_start: expect.any(String),
        window_end: expect.any(String),
        actual_total_minor: -500,
        notes: 'test notes',
      });
    });

    expect(defaultProps.onCreated).toHaveBeenCalledWith(mockResult);
  });

  it('allows negative actual total', async () => {
    const mockResult = {
      id: 'rec-2',
      status: 'mismatched' as const,
      expected_total_minor: 5000,
      actual_total_minor: -1000,
      discrepancy_minor: 6000,
      currency: 'USD',
      window_start: '2024-01-14T10:00:00.000Z',
      window_end: '2024-01-15T10:00:00.000Z',
      notes: null,
      run_at: '2024-01-15T10:30:00Z',
      created_at: '2024-01-15T10:30:00Z',
    };
    vi.mocked(mockedRunReconciliation).mockResolvedValueOnce(mockResult);

    render(<RunReconciliationDialog {...defaultProps} />);

    fireEvent.change(screen.getByLabelText('Window Start'), {
      target: { value: '2024-01-14T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Window End'), {
      target: { value: '2024-01-15T10:00' },
    });
    fireEvent.change(screen.getByLabelText('Actual Reconciliation Total (minor units)'), {
      target: { value: '-1000' },
    });

    fireEvent.click(screen.getByText('Run Reconciliation'));

    await waitFor(() => {
      expect(mockedRunReconciliation).toHaveBeenCalledWith(
        expect.objectContaining({ actual_total_minor: -1000 })
      );
    });
  });
});
