export interface DashboardSummary {
  configured_currency: string;
  generated_at: string;
  payment_overview: PaymentOverview;
  refund_overview: RefundOverview;
}

export interface PaymentOverview {
  status_counts: PaymentStatusCounts;
  recent_payments: DashboardPayment[];
}

export interface RefundOverview {
  status_counts: RefundStatusCounts;
  recent_refunds: DashboardRefund[];
}

export interface PaymentStatusCounts {
  pending: number;
  processing: number;
  successful: number;
  failed: number;
  refunded: number;
}

export interface RefundStatusCounts {
  pending: number;
  processing: number;
  completed: number;
  failed: number;
}

export interface DashboardPayment {
  id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: PaymentStatus;
  metadata: Record<string, unknown>;
  failure_reason: string | null;
  created_at: string;
  updated_at: string;
}

export interface DashboardRefund {
  id: string;
  payment_id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: RefundStatus;
  created_at: string;
  updated_at: string;
}

export type PaymentStatus = 'pending' | 'processing' | 'successful' | 'failed' | 'refunded';

export type RefundStatus = 'pending' | 'processing' | 'completed' | 'failed';

export interface PaymentListResponse {
  items: PaymentListItem[];
  limit: number;
  offset: number;
}

export interface PaymentListItem {
  id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: PaymentStatus;
  metadata: Record<string, unknown>;
  failure_reason: string | null;
  created_at: string;
  updated_at: string;
}

export interface PaymentDetail {
  id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: PaymentStatus;
  metadata: Record<string, unknown>;
  failure_reason: string | null;
  created_at: string;
  updated_at: string;
  status_history: PaymentStatusHistoryEntry[];
  refunds: PaymentRefundSummary[];
}

export interface PaymentStatusHistoryEntry {
  status: PaymentStatus;
  source_event_type: string;
  domain_event_id: string;
  occurred_at: string;
  details?: Record<string, unknown>;
}

export interface PaymentRefundSummary {
  id: string;
  payment_id: string;
  amount_minor: number;
  currency: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface RefundListResponse {
  items: RefundListItem[];
  limit: number;
  offset: number;
}

export interface RefundListItem {
  id: string;
  payment_id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: RefundStatus;
  created_at: string;
  updated_at: string;
}

export interface CreatePaymentRequest {
  amount_minor: number;
  currency: string;
  metadata: Record<string, unknown>;
}

export interface PaymentResponse {
  id: string;
  amount_minor: number;
  currency: string;
  status: PaymentStatus;
  metadata: Record<string, unknown>;
  failure_reason: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateRefundRequest {
  payment_id: string;
}

export interface RefundResponse {
  id: string;
  payment_id: string;
  amount_minor: number;
  currency: string;
  status: RefundStatus;
  created_at: string;
  updated_at: string;
}

export interface ApiError {
  code: string;
  message: string;
  request_id: string;
  details?: Record<string, unknown>;
}
