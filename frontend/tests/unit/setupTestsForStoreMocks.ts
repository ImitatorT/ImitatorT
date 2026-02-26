// Setup file for mocking external dependencies before modules are imported
import { jest } from '@jest/globals';

// Mock the backend store before importing any modules that depend on it
jest.mock('../src/stores/backendStore', () => ({
  getApiUrl: jest.fn((path) => `http://localhost:8080${path}`),
  validateBackendUrl: jest.fn(),
  setBackendUrl: jest.fn(),
}));

// Mock localStorage
const localStorageMock = (() => {
  let store: Record<string, string> = {};
  return {
    getItem: (key: string) => store[key] || null,
    setItem: (key: string, value: string) => {
      store[key] = value.toString();
    },
    removeItem: (key: string) => {
      delete store[key];
    },
    clear: () => {
      store = {};
    },
  };
})();

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
});

// Mock document.cookie
let cookieStore: Record<string, string> = {};
Object.defineProperty(document, 'cookie', {
  get: () => Object.entries(cookieStore).map(([key, value]) => `${key}=${value}`).join('; '),
  set: (val: string) => {
    const [keyValue] = val.split(';');
    const [key, value] = keyValue.split('=');
    cookieStore[key.trim()] = value;
  },
});

// Clear cookies after each test
beforeEach(() => {
  cookieStore = {};
  localStorageMock.clear();
});

afterEach(() => {
  cookieStore = {};
  localStorageMock.clear();
});