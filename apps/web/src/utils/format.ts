export function formatCurrency(amountMinor: number, currency: string): string {
  const major = amountMinor / 100;
  return `${major.toFixed(2)} ${currency}`;
}

export function formatMinorUnits(amountMinor: number): string {
  const major = amountMinor / 100;
  return major.toFixed(2);
}

export function camelToDisplay(s: string): string {
  return s
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (c) => c.toUpperCase())
    .trim();
}

export function generateIdempotencyKey(): string {
  return `dashboard-${crypto.randomUUID()}`;
}

export function getMerchantReference(metadata: Record<string, unknown>): string | null {
  if (typeof metadata.merchant_reference === 'string' && metadata.merchant_reference.length > 0) {
    return metadata.merchant_reference;
  }
  return null;
}
