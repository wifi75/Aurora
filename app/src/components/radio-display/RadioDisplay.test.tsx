import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RadioDisplayHeader } from './RadioDisplayHeader';
import { RadioDisplay } from './RadioDisplay';
import { DEMO_MODEL, RADIO_DISPLAY_STYLES, useRadioDisplay, type DisplayModel } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { RadioDisplayPicker } from '../../settings/RadioDisplayPicker';
import { useLang } from '../../i18n';

const NO_METERS: DisplayModel = { ...DEMO_MODEL, meters: null };
const UNKNOWN: DisplayModel = { ...DEMO_MODEL, dialHz: 0, meters: null };

beforeEach(() => {
  localStorage.clear();
  useRadioDisplay.setState({ pref: { style: 'fluorescent', hidden: false } });
  useStation.getState().reset();
});

afterEach(() => vi.useRealTimers());

describe('RadioDisplay', () => {
  it.each(RADIO_DISPLAY_STYLES)('draws the %s style at both sizes with a text alternative', (style) => {
    for (const size of ['mini', 'full'] as const) {
      const { container, unmount } = render(<RadioDisplay model={DEMO_MODEL} style={style} size={size} />);
      const svg = container.querySelector('svg');
      expect(svg).toHaveAttribute('role', 'img');
      expect(svg?.getAttribute('aria-label')).toContain('21.074.00');
      expect(svg?.getAttribute('aria-label')).toContain('S7');
      expect(svg?.getAttribute('viewBox')).toBeTruthy();
      unmount();
    }
  });

  it('says the meters are not available instead of inventing them', () => {
    render(<RadioDisplay model={NO_METERS} style="analog" />);
    expect(screen.getByRole('img').getAttribute('aria-label')).toContain('strumenti non disponibili');
    expect(screen.getByRole('img').getAttribute('aria-label')).not.toContain('S-meter');
  });

  it.each(RADIO_DISPLAY_STYLES)('shows dashes and the no-radio message in the %s style', (style) => {
    for (const size of ['mini', 'full'] as const) {
      const { container, unmount } = render(<RadioDisplay model={UNKNOWN} style={style} size={size} />);
      expect(screen.getByRole('img')).toHaveAttribute('aria-label', 'Radio non risponde');
      expect(container.textContent).toContain('Radio non risponde');
      unmount();
    }
  });

  it('follows the language for the alternative text', () => {
    useLang.setState({ lang: 'en' });
    render(<RadioDisplay model={DEMO_MODEL} style="led" />);
    expect(screen.getByRole('img').getAttribute('aria-label')).toContain('Frequency 21.074.00 MHz');
  });

  it('lights TX instead of RX while transmitting', () => {
    const { container } = render(<RadioDisplay model={{ ...DEMO_MODEL, transmitting: true }} style="fluorescent" size="mini" />);
    expect(screen.getByRole('img').getAttribute('aria-label')).toContain('trasmissione');
    const groups = [...container.querySelectorAll('g[opacity]')];
    const rx = groups.find((g) => g.textContent === 'RX');
    const tx = groups.find((g) => g.textContent === 'TX');
    expect(rx?.getAttribute('opacity')).toBe('0.22');
    expect(tx?.getAttribute('opacity')).toBe('1');
  });
});

describe('RadioDisplayPicker', () => {
  it('offers the five styles with a preview each and saves the choice', () => {
    render(<RadioDisplayPicker />);
    const radios = screen.getAllByRole('radio');
    expect(radios).toHaveLength(5);
    expect(screen.getAllByRole('img')).toHaveLength(5);
    expect(screen.getByRole('radio', { name: 'Fluorescente Yaesu' })).toBeChecked();
    fireEvent.click(screen.getByRole('radio', { name: 'LCD ambra Kenwood' }));
    expect(useRadioDisplay.getState().pref.style).toBe('amber');
    expect(JSON.parse(localStorage.getItem('aurora-radio-display') ?? '{}')).toEqual({ style: 'amber', hidden: false });
  });

  it('hides the display and remembers the style', () => {
    render(<RadioDisplayPicker />);
    fireEvent.click(screen.getByRole('radio', { name: 'LED rosso' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Nascondi il display' }));
    expect(useRadioDisplay.getState().pref).toEqual({ style: 'led', hidden: true });
    expect(JSON.parse(localStorage.getItem('aurora-radio-display') ?? '{}')).toEqual({ style: 'led', hidden: true });
  });
});

describe('RadioDisplayHeader', () => {
  const status = { dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'FN42', meters: { s_points: 4 } };

  it('shows the compact display from the station status and nothing when hidden', () => {
    act(() => useStation.setState({ status }));
    const { container, rerender } = render(<RadioDisplayHeader />);
    expect(screen.getByRole('button', { name: /21\.074\.00/ })).toBeInTheDocument();
    expect(container.querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 320 52');
    act(() => useRadioDisplay.getState().set({ hidden: true }));
    rerender(<RadioDisplayHeader />);
    expect(container).toBeEmptyDOMElement();
  });

  it('opens the enlarged version on click and closes it with Esc', () => {
    act(() => useStation.setState({ status }));
    render(<RadioDisplayHeader />);
    const trigger = screen.getByRole('button');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('group', { name: 'Display della radio ingrandito' }).querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 680 150');
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('group', { name: 'Display della radio ingrandito' })).toBeNull();
  });

  it('opens on hover after a short delay and closes when the mouse leaves', () => {
    vi.useFakeTimers();
    act(() => useStation.setState({ status }));
    const { container } = render(<RadioDisplayHeader />);
    const root = container.firstElementChild as HTMLElement;
    fireEvent.mouseEnter(root);
    expect(screen.queryByRole('group', { name: 'Display della radio ingrandito' })).toBeNull();
    act(() => void vi.advanceTimersByTime(300));
    expect(screen.getByRole('group', { name: 'Display della radio ingrandito' })).toBeInTheDocument();
    fireEvent.mouseLeave(root);
    act(() => void vi.advanceTimersByTime(300));
    expect(screen.queryByRole('group', { name: 'Display della radio ingrandito' })).toBeNull();
  });

  it('keeps it open on hover leave when opened with a click', () => {
    vi.useFakeTimers();
    act(() => useStation.setState({ status }));
    const { container } = render(<RadioDisplayHeader />);
    const root = container.firstElementChild as HTMLElement;
    fireEvent.click(screen.getByRole('button'));
    fireEvent.mouseLeave(root);
    act(() => void vi.advanceTimersByTime(500));
    expect(screen.getByRole('group', { name: 'Display della radio ingrandito' })).toBeInTheDocument();
  });
});
