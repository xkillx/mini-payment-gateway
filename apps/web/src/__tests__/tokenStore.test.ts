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

  it('uses correct storage key', () => {
    setToken('test-jwt-token');
    expect(mockLocalStorage.getItem('mpg.merchantDashboard.token')).toBe('test-jwt-token');
  });
});
