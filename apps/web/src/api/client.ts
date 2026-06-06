import type {
  DashboardSummary,
  PaymentListResponse,
  PaymentDetail,
  CreatePaymentRequest,
  PaymentResponse,
  RefundListResponse,
  CreateRefundRequest,
  RefundResponse,
  ApiError,
} from './types';

const API_BASE = import.meta.env.VITE_API_BASE_URL || 'http://localhost:4000';

function getToken(): string | null {
  return localStorage.getItem('mpg.merchantDashboard.token');
}

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

export function setToken(token: string) {
  localStorage.setItem('mpg.merchantDashboard.token', token);
}

export function clearToken() {
  localStorage.removeItem('mpg.merchantDashboard.token');
}

export function hasToken(): boolean {
  return getToken() !== null;
}

export async function getDashboardSummary(): Promise<DashboardSummary> {
  const { data } = await request<DashboardSummary>('GET', '/api/v1/dashboard/merchant');
  return data;
}

export async function listPayments(params: {
  status?: string;
  search?: string;
  limit?: number;
  offset?: number;
}): Promise<PaymentListResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.search) searchParams.set('search', params.search);
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
  limit?: number;
  offset?: number;
}): Promise<RefundListResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const { data } = await request<RefundListResponse>('GET', `/api/v1/refunds?${searchParams.toString()}`);
  return data;
}

export async function createRefund(
  req: CreateRefundRequest,
  idempotencyKey: string,
): Promise<RefundResponse> {
  const { data } = await request<RefundResponse>('POST', '/api/v1/refunds', req, idempotencyKey);
  return data;
}

export { ApiClientError };
