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

export interface AdminDashboardSummary {
  configured_currency: string;
  generated_at: string;
  window_start: string;
  window_end: string;
  api_status: string;
  payment_overview: AdminPaymentOverview;
  refund_overview: AdminRefundOverview;
  notification_overview: AdminNotificationOverview;
  reconciliation_overview: AdminReconciliationOverview;
  audit_overview: AdminAuditOverview;
}

export interface AdminPaymentOverview {
  status_counts: PaymentStatusCounts;
  recent_failed_payments: DashboardPayment[];
}

export interface AdminRefundOverview {
  status_counts: RefundStatusCounts;
  recent_failed_refunds: DashboardRefund[];
}

export interface AdminNotificationOverview {
  status_counts: NotificationStatusCounts;
  recent_failed_notifications: AdminNotificationListItem[];
}

export interface AdminReconciliationOverview {
  status_counts: ReconciliationStatusCounts;
  recent_attention_reconciliations: AdminReconciliationListItem[];
}

export interface AdminAuditOverview {
  recent_attention_audit_records: AdminAuditListItem[];
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

export interface NotificationStatusCounts {
  pending: number;
  processing: number;
  delivered: number;
  failed: number;
}

export interface ReconciliationStatusCounts {
  matched: number;
  mismatched: number;
  error: number;
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

export interface AdminNotificationListItem {
  id: string;
  domain_event_id: string;
  event_type: string;
  resource_type: string;
  resource_id: string;
  payment_id: string;
  destination_url: string;
  status: string;
  attempt_count: number;
  last_error: string | null;
  updated_at: string;
}

export interface AdminReconciliationListItem {
  id: string;
  status: string;
  expected_total_minor: number;
  actual_total_minor: number;
  discrepancy_minor: number;
  currency: string;
  window_start: string;
  window_end: string;
  created_at: string;
}

export interface AdminAuditListItem {
  id: string;
  actor_id: string | null;
  actor_type: string;
  action: string;
  resource_type: string;
  resource_id: string;
  details: Record<string, unknown> | null;
  occurred_at: string;
  created_at: string;
}

export type PaymentStatus = 'pending' | 'processing' | 'successful' | 'failed' | 'refunded';

export type RefundStatus = 'pending' | 'processing' | 'completed' | 'failed';

export type NotificationStatus = 'pending' | 'processing' | 'delivered' | 'failed';

export type ReconciliationStatus = 'matched' | 'mismatched' | 'error';

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
  notification_delivery_records: PaymentNotificationDeliveryRecord[];
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

export interface PaymentNotificationDeliveryRecord {
  id: string;
  domain_event_id: string;
  event_type: string;
  destination_url: string;
  status: NotificationStatus;
  attempt_count: number;
  last_attempt_at: string | null;
  next_retry_at: string | null;
  last_error: string | null;
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

export interface RefundDetail {
  id: string;
  payment_id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  status: RefundStatus;
  created_at: string;
  updated_at: string;
}

export interface NotificationListResponse {
  items: NotificationDeliveryRecordDetail[];
  limit: number;
  offset: number;
}

export interface NotificationDeliveryRecordDetail {
  id: string;
  domain_event_id: string;
  event_type: string;
  resource_type: string;
  resource_id: string;
  resource_api_path: string;
  payment_id: string;
  payment_api_path: string;
  refund_id: string | null;
  refund_api_path: string | null;
  destination_url: string;
  status: NotificationStatus;
  attempt_count: number;
  retry_generation: number;
  last_attempt_at: string | null;
  next_retry_at: string | null;
  last_error: string | null;
  created_at: string;
  updated_at: string;
  attempts: NotificationDeliveryAttempt[];
}

export interface NotificationDeliveryAttempt {
  id: string;
  notification_delivery_record_id: string;
  retry_generation: number;
  attempt_number: number;
  status: string;
  http_status_code: number | null;
  error: string | null;
  started_at: string;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ReconciliationListResponse {
  items: Reconciliation[];
  limit: number;
  offset: number;
}

export interface Reconciliation {
  id: string;
  status: ReconciliationStatus;
  expected_total_minor: number;
  actual_total_minor: number;
  discrepancy_minor: number;
  currency: string;
  window_start: string;
  window_end: string;
  notes: string | null;
  run_at: string;
  created_at: string;
}

export interface ReconciliationReport {
  id: string;
  status: ReconciliationStatus;
  expected_total_minor: number;
  actual_total_minor: number;
  discrepancy_minor: number;
  currency: string;
  window_start: string;
  window_end: string;
  notes: string | null;
  run_at: string;
  created_at: string;
  included_payment_total_minor: number;
  included_refund_total_minor: number;
  included_record_count: number;
  included_payments: IncludedPaymentRecord[];
  included_refunds: IncludedRefundRecord[];
}

export interface IncludedPaymentRecord {
  domain_event_id: string;
  payment_id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  occurred_at: string;
  metadata: Record<string, unknown>;
}

export interface IncludedRefundRecord {
  domain_event_id: string;
  refund_id: string;
  payment_id: string;
  merchant_id: string;
  amount_minor: number;
  currency: string;
  occurred_at: string;
}

export interface AuditRecordListResponse {
  items: AuditRecord[];
  limit: number;
  offset: number;
}

export interface AuditRecord {
  id: string;
  actor_id: string | null;
  actor_type: string;
  action: string;
  resource_type: string;
  resource_id: string;
  details: Record<string, unknown> | null;
  occurred_at: string;
  created_at: string;
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

export interface RunReconciliationRequest {
  currency: string;
  window_start: string;
  window_end: string;
  actual_total_minor: number;
  notes?: string;
}

export interface ApiError {
  code: string;
  message: string;
  request_id: string;
  details?: Record<string, unknown>;
}
