import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { ThemeToggle } from './ThemeToggle';

describe('ThemeToggle', () => {
  beforeEach(() => {
    delete document.documentElement.dataset.theme;
    window.localStorage.clear();
  });

  it('starts dark and offers the light theme', () => {
    render(<ThemeToggle />);
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(screen.getByRole('button', { name: 'Tema chiaro' })).toBeInTheDocument();
  });

  it('switches to light and back, remembering the choice', () => {
    render(<ThemeToggle />);
    fireEvent.click(screen.getByRole('button', { name: 'Tema chiaro' }));
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(window.localStorage.getItem('aurora-theme')).toBe('light');
    fireEvent.click(screen.getByRole('button', { name: 'Tema scuro' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    expect(window.localStorage.getItem('aurora-theme')).toBe('dark');
  });

  it('restores a saved light theme', () => {
    window.localStorage.setItem('aurora-theme', 'light');
    render(<ThemeToggle />);
    expect(document.documentElement.dataset.theme).toBe('light');
    expect(screen.getByRole('button', { name: 'Tema scuro' })).toBeInTheDocument();
  });
});
