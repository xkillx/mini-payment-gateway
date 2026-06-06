export type DashboardRole = 'administrator' | 'merchant';

interface JwtPayload {
  role?: string;
}

function base64UrlDecode(str: string): string {
  const base64 = str.replace(/-/g, '+').replace(/_/g, '/');
  const decoded = atob(base64);
  return decodeURIComponent(
    decoded.split('').map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)).join('')
  );
}

export function getTokenRole(token: string): DashboardRole | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    const payload = JSON.parse(base64UrlDecode(parts[1])) as JwtPayload;
    if (payload.role === 'administrator') return 'administrator';
    if (payload.role === 'merchant') return 'merchant';
    return null;
  } catch {
    return null;
  }
}
