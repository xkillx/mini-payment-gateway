import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import OperationalHealthView from '../OperationalHealthView';
import type { AdminOperationalHealth } from '../../../api/types';

const operationalHealth: AdminOperationalHealth = {
  payment_processing_success_rate: {
    numerator_count: 8,
    denominator_count: 10,
    in_flight_count: 2,
    rate_percent: 80.0,
  },
  notification_delivery_success_rate: {
    numerator_count: 19,
    denominator_count: 20,
    in_flight_count: 1,
    rate_percent: 95.0,
  },
  reconciliation_completion_rate: {
    numerator_count: 3,
    denominator_count: 4,
    in_flight_count: 0,
    rate_percent: 75.0,
  },
  recent_failed_operations: [],
};

describe('OperationalHealthView', () => {
  it('renders Operational Health heading', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('Operational Health')).toBeDefined();
  });

  it('renders three rate cards', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('Payment Processing Success Rate')).toBeDefined();
    expect(screen.getByText('Notification Delivery Success Rate')).toBeDefined();
    expect(screen.getByText('Reconciliation Completion Rate')).toBeDefined();
  });

  it('renders rate percentages', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('80.0%')).toBeDefined();
    expect(screen.getByText('95.0%')).toBeDefined();
    expect(screen.getByText('75.0%')).toBeDefined();
  });

  it('renders N/A when rate_percent is null', () => {
    const nullHealth: AdminOperationalHealth = {
      ...operationalHealth,
      payment_processing_success_rate: {
        numerator_count: 0,
        denominator_count: 0,
        in_flight_count: 0,
        rate_percent: null,
      },
    };

    render(
      <OperationalHealthView
        operationalHealth={nullHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('N/A')).toBeDefined();
  });

  it('renders in-flight context', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText(/2 in-flight/)).toBeDefined();
  });

  it('does not render severity labels', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.queryByText(/healthy/)).toBeNull();
    expect(screen.queryByText(/degraded/)).toBeNull();
    expect(screen.queryByText(/critical/)).toBeNull();
  });

  it('renders empty state when no failed operations', () => {
    render(
      <OperationalHealthView
        operationalHealth={operationalHealth}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('No failed operations in the last 24 hours.')).toBeDefined();
  });

  it('renders failed operations feed', () => {
    const healthWithFailures: AdminOperationalHealth = {
      ...operationalHealth,
      recent_failed_operations: [
        {
          kind: 'payment',
          id: '00000000-0000-0000-0000-000000000001',
          occurred_at: '2024-01-15T09:00:00Z',
          status: 'failed',
          merchant_id: '00000000-0000-0000-0000-000000000002',
          payment_id: '00000000-0000-0000-0000-000000000001',
          amount_minor: 2500,
          currency: 'USD',
          reason: 'processor_declined',
          event_type: 'payment.failed',
          resource_type: 'payment',
          resource_id: '00000000-0000-0000-0000-000000000001',
          discrepancy_minor: null,
        },
      ],
    };

    render(
      <OperationalHealthView
        operationalHealth={healthWithFailures}
        windowStart="2024-01-14T10:00:00Z"
        windowEnd="2024-01-15T10:00:00Z"
        onOpenOperation={() => {}}
      />
    );

    expect(screen.getByText('Recent Failed Operations')).toBeDefined();
    expect(screen.getByText(/processor_declined/)).toBeDefined();
  });
});
