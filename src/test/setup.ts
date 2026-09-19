import '@testing-library/jest-dom/vitest';

// jsdom has no ResizeObserver; the auto-scroll hook needs one to exist.
if (!('ResizeObserver' in globalThis)) {
  (globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

if (!('matchMedia' in window)) {
  Object.defineProperty(window, 'matchMedia', {
    value: () => ({
      matches: false,
      addEventListener: () => {},
      removeEventListener: () => {},
    }),
  });
}
