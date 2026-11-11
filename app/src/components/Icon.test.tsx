import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Icon } from './Icon';

describe('Icon', () => {
  it('disegna un svg decorativo nascosto agli screen reader', () => {
    const { container } = render(
      <button type="button" aria-label="Impostazioni">
        <Icon name="gear" />
      </button>,
    );
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg?.getAttribute('aria-hidden')).toBe('true');
    expect(svg?.getAttribute('stroke')).toBe('currentColor');
    expect(container.querySelector('button')?.textContent).toBe('');
  });

  it('ogni icona ha almeno un tracciato', () => {
    for (const name of ['gear', 'refresh', 'chevron-down'] as const) {
      const { container, unmount } = render(<Icon name={name} />);
      expect(container.querySelectorAll('path').length).toBeGreaterThan(0);
      unmount();
    }
  });
});
