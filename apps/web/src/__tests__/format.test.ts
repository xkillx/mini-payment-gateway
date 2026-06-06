import { describe, it, expect } from 'vitest';
import { formatCurrency, formatMinorUnits, getMerchantReference, generateIdempotencyKey } from '../utils/format';

describe('formatCurrency', () => {
  it('formats minor units to major currency display', () => {
    expect(formatCurrency(1000, 'USD')).toBe('10.00 USD');
    expect(formatCurrency(0, 'USD')).toBe('0.00 USD');
    expect(formatCurrency(99, 'USD')).toBe('0.99 USD');
    expect(formatCurrency(10050, 'EUR')).toBe('100.50 EUR');
  });
});

describe('formatMinorUnits', () => {
  it('converts minor units to decimal string', () => {
    expect(formatMinorUnits(1000)).toBe('10.00');
    expect(formatMinorUnits(0)).toBe('0.00');
    expect(formatMinorUnits(1)).toBe('0.01');
  });
});

describe('getMerchantReference', () => {
  it('returns merchant_reference when present', () => {
    expect(getMerchantReference({ merchant_reference: 'ORD-123' })).toBe('ORD-123');
  });

  it('returns null when missing', () => {
    expect(getMerchantReference({})).toBeNull();
    expect(getMerchantReference({ merchant_reference: '' })).toBeNull();
    expect(getMerchantReference({ merchant_reference: 123 })).toBeNull();
  });
});

describe('generateIdempotencyKey', () => {
  it('generates a key with dashboard prefix', () => {
    const key = generateIdempotencyKey();
    expect(key.startsWith('dashboard-')).toBe(true);
    expect(key.length).toBeGreaterThan(10);
  });

  it('generates unique keys', () => {
    const a = generateIdempotencyKey();
    const b = generateIdempotencyKey();
    expect(a).not.toBe(b);
  });
});
