import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import type { PanelId } from '../lib/layout';
import { PHONE_TABS, readPhoneTab, savePhoneTab } from '../lib/phone';
import { PhoneTabs } from './PhoneTabs';

const titles: Record<PanelId, string> = { waterfall: 'Cascata', station: 'Comandi', tx: 'Trasmissione', cq: 'CQ e novità', all: 'Attività di banda', log: 'Log', cluster: 'DX Cluster', dxcc: 'DXCC' };

beforeEach(() => localStorage.clear());

describe('PhoneTabs', () => {
  it('shows one panel at a time, starting with the band activity', () => {
    render(<PhoneTabs titles={titles} render={(id) => <p>contenuto {id}</p>} />);
    expect(screen.getByText('contenuto all')).toBeInTheDocument();
    expect(screen.queryByText('contenuto tx')).toBeNull();
    expect(screen.getAllByRole('tab')).toHaveLength(PHONE_TABS.length);
    expect(screen.getByRole('tab', { name: 'Attività di banda' })).toHaveAttribute('aria-selected', 'true');
  });

  it('switches panel on a tap and remembers it for the next visit', () => {
    const { unmount } = render(<PhoneTabs titles={titles} render={(id) => <p>contenuto {id}</p>} />);
    fireEvent.click(screen.getByRole('tab', { name: 'Trasmissione' }));
    expect(screen.getByText('contenuto tx')).toBeInTheDocument();
    expect(screen.queryByText('contenuto all')).toBeNull();
    unmount();
    render(<PhoneTabs titles={titles} render={(id) => <p>contenuto {id}</p>} />);
    expect(screen.getByText('contenuto tx')).toBeInTheDocument();
  });

  it('puts the extra block above the panel it belongs to', () => {
    render(<PhoneTabs titles={titles} render={(id) => <p>contenuto {id}</p>} extra={{ station: <p>comandi di ricezione</p> }} />);
    expect(screen.queryByText('comandi di ricezione')).toBeNull();
    fireEvent.click(screen.getByRole('tab', { name: 'Comandi' }));
    expect(screen.getByText('comandi di ricezione')).toBeInTheDocument();
  });

  it('moves between the tabs with the arrow keys', () => {
    render(<PhoneTabs titles={titles} render={(id) => <p>contenuto {id}</p>} />);
    fireEvent.keyDown(screen.getByRole('tab', { name: 'Attività di banda' }), { key: 'ArrowRight' });
    expect(screen.getByText('contenuto cq')).toBeInTheDocument();
    fireEvent.keyDown(screen.getByRole('tab', { name: 'CQ e novità' }), { key: 'ArrowLeft' });
    fireEvent.keyDown(screen.getByRole('tab', { name: 'Attività di banda' }), { key: 'ArrowLeft' });
    expect(screen.getByText('contenuto log')).toBeInTheDocument();
  });
});

describe('phone tab storage', () => {
  it('falls back to the first tab for an unknown or unreadable value', () => {
    expect(readPhoneTab({ getItem: () => 'inesistente' })).toBe('all');
    expect(readPhoneTab({ getItem: () => { throw new Error('no'); } })).toBe('all');
    expect(readPhoneTab(null)).toBe('all');
    const store: Record<string, string> = {};
    savePhoneTab({ setItem: (k, v) => { store[k] = v; } }, 'cluster');
    expect(readPhoneTab({ getItem: (k) => store[k] ?? null })).toBe('cluster');
    expect(() => savePhoneTab({ setItem: () => { throw new Error('no'); } }, 'tx')).not.toThrow();
  });
});
