import { getToken } from '../auth/tokenStore';
import type {
  DashboardSummary,
  AdminDashboardSummary,
  PaymentListResponse,
  PaymentDetail,
  CreatePaymentRequest,
  PaymentResponse,
  RefundListResponse,
  RefundDetail,
  CreateRefundRequest,
  RefundResponse,
  NotificationListResponse,
  NotificationDeliveryRecordDetail,
  ReconciliationListResponse,
  ReconciliationReport,
  Reconciliation,
  RunReconciliationRequest,
  AuditRecordListResponse,
  AuditRecord,
  PaymentSummaryReport,
  ApiError,
} from './types';

const API_BASE = import.meta.env.VITE_API_BASE_URL || 'http://localhost:4000';

function headers(): Record<string, string> {
  const token = getToken();
  const h: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (token) {
    h['Authorization'] = `Bearer ${token}`;
  }
  return h;
}

class ApiClientError extends Error {
  status: number;
  body: ApiError;

  constructor(status: number, body: ApiError) {
    super(body.message);
    this.status = status;
    this.body = body;
    this.name = 'ApiClientError';
  }
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  idempotencyKey?: string,
): Promise<{ status: number; data: T }> {
  const h = headers();
  if (idempotencyKey) {
    h['Idempotency-Key'] = idempotencyKey;
  }

  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: h,
    body: body ? JSON.stringify(body) : undefined,
  });

  const json = await res.json().catch(() => null);

  if (!res.ok) {
    throw new ApiClientError(res.status, json || { code: 'UNKNOWN', message: 'Request failed', request_id: '' });
  }

  return { status: res.status, data: json as T };
}

export async function getDashboardSummary(): Promise<DashboardSummary> {
  const { data } = await request<DashboardSummary>('GET', '/api/v1/dashboard/merchant');
  return data;
}

export async function getAdminDashboardSummary(): Promise<AdminDashboardSummary> {
  const { data } = await request<AdminDashboardSummary>('GET', '/api/v1/dashboard/admin');
  return data;
}

export async function listPayments(params: {
  status?: string;
  search?: string;
  merchantId?: string;
  limit?: number;
  offset?: number;
}): Promise<PaymentListResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.search) searchParams.set('search', params.search);
  if (params.merchantId) searchParams.set('merchant_id', params.merchantId);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<PaymentListResponse>('GET', `/api/v1/payments?${searchParams.toString()}`);
  return data;
}

export async function getPayment(id: string): Promise<PaymentDetail> {
  const { data } = await request<PaymentDetail>('GET', `/api/v1/payments/${id}`);
  return data;
}

export async function createPayment(
  req: CreatePaymentRequest,
  idempotencyKey: string,
): Promise<PaymentResponse> {
  const { data } = await request<PaymentResponse>('POST', '/api/v1/payments', req, idempotencyKey);
  return data;
}

export async function listRefunds(params: {
  status?: string;
  merchantId?: string;
  limit?: number;
  offset?: number;
}): Promise<RefundListResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.merchantId) searchParams.set('merchant_id', params.merchantId);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<RefundListResponse>('GET', `/api/v1/refunds?${searchParams.toString()}`);
  return data;
}

export async function getRefund(id: string): Promise<RefundDetail> {
  const { data } = await request<RefundDetail>('GET', `/api/v1/refunds/${id}`);
  return data;
}

export async function createRefund(
  req: CreateRefundRequest,
  idempotencyKey: string,
): Promise<RefundResponse> {
  const { data } = await request<RefundResponse>('POST', '/api/v1/refunds', req, idempotencyKey);
  return data;
}

export async function listNotifications(params: {
  status?: string;
  limit?: number;
  offset?: number;
}): Promise<NotificationListResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<NotificationListResponse>('GET', `/api/v1/notifications?${searchParams.toString()}`);
  return data;
}

export async function getNotification(id: string): Promise<NotificationDeliveryRecordDetail> {
  const { data } = await request<NotificationDeliveryRecordDetail>('GET', `/api/v1/notifications/${id}`);
  return data;
}

export async function retryNotification(id: string): Promise<NotificationDeliveryRecordDetail> {
  const { data } = await request<NotificationDeliveryRecordDetail>('POST', `/api/v1/notifications/${id}/retry`);
  return data;
}

export async function listReconciliations(params: {
  limit?: number;
  offset?: number;
}): Promise<ReconciliationListResponse> {
  const searchParams = new URLSearchParams();
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<ReconciliationListResponse>('GET', `/api/v1/reconciliation?${searchParams.toString()}`);
  return data;
}

export async function getReconciliationReport(id: string): Promise<ReconciliationReport> {
  const { data } = await request<ReconciliationReport>('GET', `/api/v1/reconciliation/${id}`);
  return data;
}

export async function runReconciliation(req: RunReconciliationRequest): Promise<Reconciliation> {
  const { data } = await request<Reconciliation>('POST', '/api/v1/reconciliation', req);
  return data;
}

export async function listAuditRecords(params: {
  actor_type?: string;
  resource_type?: string;
  resource_id?: string;
  action?: string;
  occurred_from?: string;
  occurred_to?: string;
  limit?: number;
  offset?: number;
}): Promise<AuditRecordListResponse> {
  const searchParams = new URLSearchParams();
  if (params.actor_type) searchParams.set('actor_type', params.actor_type);
  if (params.resource_type) searchParams.set('resource_type', params.resource_type);
  if (params.resource_id) searchParams.set('resource_id', params.resource_id);
  if (params.action) searchParams.set('action', params.action);
  if (params.occurred_from) searchParams.set('occurred_from', params.occurred_from);
  if (params.occurred_to) searchParams.set('occurred_to', params.occurred_to);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<AuditRecordListResponse>('GET', `/api/v1/audit?${searchParams.toString()}`);
  return data;
}

export async function getAuditRecord(id: string): Promise<AuditRecord> {
  const { data } = await request<AuditRecord>('GET', `/api/v1/audit/${id}`);
  return data;
}

export async function getPaymentSummaryReport(params: {
  from?: string;
  to?: string;
}): Promise<PaymentSummaryReport> {
  const searchParams = new URLSearchParams();
  if (params.from) searchParams.set('from', params.from);
  if (params.to) searchParams.set('to', params.to);

  const qs = searchParams.toString();
  const { data } = await request<PaymentSummaryReport>(
    'GET',
    `/api/v1/reporting/payment-summary${qs ? `?${qs}` : ''}`,
  );
  return data;
}

export { ApiClientError };
