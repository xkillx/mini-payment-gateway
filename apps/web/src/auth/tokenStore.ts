const TOKEN_KEY = 'mpg.dashboard.token';
const LEGACY_TOKEN_KEY = 'mpg.merchantDashboard.token';

function migrateLegacyToken() {
  const legacy = localStorage.getItem(LEGACY_TOKEN_KEY);
  if (legacy) {
    localStorage.setItem(TOKEN_KEY, legacy);
    localStorage.removeItem(LEGACY_TOKEN_KEY);
  }
}

export function getToken(): string | null {
  migrateLegacyToken();
  return localStorage.getItem(TOKEN_KEY);
}

export function setToken(token: string) {
  localStorage.removeItem(LEGACY_TOKEN_KEY);
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(LEGACY_TOKEN_KEY);
}

export function hasToken(): boolean {
  migrateLegacyToken();
  return localStorage.getItem(TOKEN_KEY) !== null;
}
