import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, beforeEach, vi } from 'vitest';
import { useLang } from '../i18n';

beforeEach(() => {
  useLang.setState({ lang: 'it' });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
