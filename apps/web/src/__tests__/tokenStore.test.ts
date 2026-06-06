import { describe, it, expect, beforeEach } from 'vitest';

const mockLocalStorage = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { store = {}; },
  };
})();

Object.defineProperty(globalThis, 'localStorage', { value: mockLocalStorage });

import { getToken, setToken, clearToken, hasToken } from '../auth/tokenStore';

describe('tokenStore', () => {
  beforeEach(() => {
    mockLocalStorage.clear();
  });

  it('returns null when no token set', () => {
    expect(getToken()).toBeNull();
    expect(hasToken()).toBe(false);
  });

  it('stores and retrieves token', () => {
    setToken('test-jwt-token');
    expect(getToken()).toBe('test-jwt-token');
    expect(hasToken()).toBe(true);
  });

  it('clears token', () => {
    setToken('test-jwt-token');
    clearToken();
    expect(getToken()).toBeNull();
    expect(hasToken()).toBe(false);
  });

  it('uses new storage key', () => {
    setToken('test-jwt-token');
    expect(mockLocalStorage.getItem('mpg.dashboard.token')).toBe('test-jwt-token');
  });

  it('migrates legacy key on read', () => {
    mockLocalStorage.setItem('mpg.merchantDashboard.token', 'legacy-token');
    expect(getToken()).toBe('legacy-token');
    expect(mockLocalStorage.getItem('mpg.dashboard.token')).toBe('legacy-token');
    expect(mockLocalStorage.getItem('mpg.merchantDashboard.token')).toBeNull();
  });

  it('removes legacy key on write', () => {
    mockLocalStorage.setItem('mpg.merchantDashboard.token', 'old-token');
    setToken('new-token');
    expect(mockLocalStorage.getItem('mpg.dashboard.token')).toBe('new-token');
    expect(mockLocalStorage.getItem('mpg.merchantDashboard.token')).toBeNull();
  });

  it('clear removes both keys', () => {
    mockLocalStorage.setItem('mpg.dashboard.token', 'current');
    mockLocalStorage.setItem('mpg.merchantDashboard.token', 'legacy');
    clearToken();
    expect(mockLocalStorage.getItem('mpg.dashboard.token')).toBeNull();
    expect(mockLocalStorage.getItem('mpg.merchantDashboard.token')).toBeNull();
    expect(hasToken()).toBe(false);
  });
});
