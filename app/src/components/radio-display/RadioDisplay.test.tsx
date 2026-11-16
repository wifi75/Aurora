import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RadioDisplayHeader } from './RadioDisplayHeader';
import { RadioDisplay } from './RadioDisplay';
import { DEMO_MODEL, RADIO_DISPLAY_STYLES, useRadioDisplay, type DisplayModel } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { RadioDisplayPicker } from '../../settings/RadioDisplayPicker';
import { useLang } from '../../i18n';
import { getCat } from '../../api/client';

vi.mock('../../api/client', async (orig) => ({ ...(await orig<typeof import('../../api/client')>()), getCat: vi.fn() }));

const NO_METERS: DisplayModel = { ...DEMO_MODEL, meters: null };
const UNKNOWN: DisplayModel = { ...DEMO_MODEL, dialHz: 0, meters: null };

beforeEach(() => {
  localStorage.clear();
  useRadioDisplay.setState({ pref: { style: 'fluorescent', hidden: false, layout: 'single' } });
  useStation.getState().reset();
});

afterEach(() => vi.useRealTimers());

describe('RadioDisplay', () => {
  it.each(RADIO_DISPLAY_STYLES)('draws the %s style at both sizes with a text alternative', (style) => {
    for (const size of ['mini', 'full'] as const) {
      const { container, unmount } = render(<RadioDisplay model={DEMO_MODEL} style={style} size={size} />);
      // i display disegnati sono un svg, quelli nuovi (estesi e quasi quadrati) un riquadro: tutti hanno il ruolo immagine e il testo alternativo
      const shown = container.querySelector('[role="img"]');
      expect(shown).not.toBeNull();
      expect(shown?.getAttribute('aria-label')).toContain('21.074.00');
      expect(shown?.getAttribute('aria-label')).toContain('S7');
      if (shown?.tagName.toLowerCase() === 'svg') expect(shown.getAttribute('viewBox')).toBeTruthy();
      unmount();
    }
  });

  it.each(RADIO_DISPLAY_STYLES)('marks the estimated S-meter in the %s style, in the drawing and in the text alternative', (style) => {
    const model = { ...DEMO_MODEL, meters: { s_points: 6, s_over_db: 0, estimated: true } };
    const { container } = render(<RadioDisplay model={model} style={style} size="full" />);
    expect(screen.getByRole('img').getAttribute('aria-label')).toContain("S-meter stimato dall'audio S6");
    expect(container.textContent).toContain('~S6');
  });

  it('shows no estimate mark with the real meters of the radio', () => {
    const { container } = render(<RadioDisplay model={DEMO_MODEL} style="analog" size="full" />);
    expect(container.textContent).not.toContain('~S');
    expect(screen.getByRole('img').getAttribute('aria-label')).not.toContain('stimato');
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
      // i pannelli FT-2000 e IC-7610 non scrivono piu' la frase sopra gli strumenti: a radio spenta c'e' la scritta grande al centro (CSS)
      if (style !== 'ft2000' && style !== 'ic7610') expect(container.textContent).toContain('Radio non risponde');
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
    const rx = groups.find((g) => g.querySelector('text')?.textContent === 'RX');
    const tx = groups.find((g) => g.querySelector('text')?.textContent === 'TX');
    expect(rx?.getAttribute('opacity')).toBe('0.22');
    expect(tx?.getAttribute('opacity')).toBe('1');
  });
});

describe('RadioDisplayPicker', () => {
  it('offers every style with a preview each and saves the choice', () => {
    render(<RadioDisplayPicker />);
    const radios = within(screen.getByRole('radiogroup', { name: 'Stile del display della radio' })).getAllByRole('radio');
    expect(radios).toHaveLength(RADIO_DISPLAY_STYLES.length);
    expect(screen.getAllByRole('img')).toHaveLength(RADIO_DISPLAY_STYLES.length);
    expect(screen.getByRole('radio', { name: 'Fluorescente Yaesu' })).toBeChecked();
    fireEvent.click(screen.getByRole('radio', { name: 'LCD ambra Kenwood' }));
    expect(useRadioDisplay.getState().pref.style).toBe('amber');
    expect(JSON.parse(localStorage.getItem('aurora-radio-display') ?? '{}')).toEqual({ style: 'amber', hidden: false, layout: 'single' });
  });

  it('hides the display and remembers the style', () => {
    render(<RadioDisplayPicker />);
    fireEvent.click(screen.getByRole('radio', { name: 'LED rosso' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Nascondi il display' }));
    expect(useRadioDisplay.getState().pref).toEqual({ style: 'led', hidden: true, layout: 'single' });
    expect(JSON.parse(localStorage.getItem('aurora-radio-display') ?? '{}')).toEqual({ style: 'led', hidden: true, layout: 'single' });
  });
});

// il display esteso chiede lo stato della radio: serve il client delle richieste
const wrapper = ({ children }: { children: ReactNode }) => <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{children}</QueryClientProvider>;

describe('RadioDisplayHeader', () => {
  const status = { dial_hz: 21_074_000, mode: 'FT8', tx_enabled: false, transmitting: false, decoding: true, de_call: 'AA1AA', de_grid: 'FN42', meters: { s_points: 4 } };

  it('shows the compact display from the station status and nothing when hidden', () => {
    act(() => useStation.setState({ status }));
    const { container, rerender } = render(<RadioDisplayHeader />, { wrapper });
    expect(container.querySelector('svg')?.getAttribute('viewBox')).toBe('0 0 320 52');
    act(() => useRadioDisplay.getState().set({ hidden: true }));
    rerender(<RadioDisplayHeader />);
    expect(container).toBeEmptyDOMElement();
  });

  it('opens a larger copy when the mouse rests on it and closes it when the mouse leaves', () => {
    vi.useFakeTimers();
    act(() => useStation.setState({ status }));
    const { container } = render(<RadioDisplayHeader />, { wrapper });
    const root = container.querySelector('.rd-zoom-host') as HTMLElement;
    expect(screen.queryByRole('group', { name: 'Display della radio ingrandito' })).toBeNull();
    fireEvent.mouseEnter(root);
    act(() => void vi.advanceTimersByTime(300));
    expect(screen.getByRole('group', { name: 'Display della radio ingrandito' })).toBeInTheDocument();
    fireEvent.mouseLeave(root);
    act(() => void vi.advanceTimersByTime(400));
    expect(screen.queryByRole('group', { name: 'Display della radio ingrandito' })).toBeNull();
  });

  it('shows the single FT-2000 panel with clickable indicators in place of the two blocks', () => {
    act(() => useStation.setState({ status }));
    act(() => useRadioDisplay.getState().set({ style: 'ft2000' }));
    const { container } = render(<RadioDisplayHeader />, { wrapper });
    // un solo frontalino (le altre svg sono le icone della testata, come il menu per spostare il display)
    expect(container.querySelectorAll('svg[viewBox="0 0 832 96"]')).toHaveLength(1);
    expect(container.querySelector('.rx-extras')).toBeNull();
    expect(container.textContent).toContain('VFO-A');
  });

  it('draws the IC-7610 panel with the demo data of the radio', () => {
    const { container } = render(<RadioDisplay model={DEMO_MODEL} style="ic7610" />);
    expect(container.textContent).toContain('MAIN');
    expect(container.textContent).toContain('21.076.50');
    expect(container.textContent).toContain('BW 3.0k');
  });

  // il riquadro PWR + SPLIT e' uno solo, uguale in ogni stile, accanto al display e mai nel pulsantino di stacco
  const showStyle = async (style: (typeof RADIO_DISPLAY_STYLES)[number], power: boolean) => {
    const cat = { config: { backend: 'omnirig', model: 0 }, status: { state: 'connected', rig: { split: true, extras: power ? { power_set: 1 } : {} } } };
    vi.mocked(getCat).mockResolvedValue(cat as never);
    act(() => useStation.setState({ status }));
    act(() => useRadioDisplay.getState().set({ style }));
    const view = render(<RadioDisplayHeader />, { wrapper });
    await waitFor(() => expect(document.querySelector('.rd-pwrsplit')).not.toBeNull());
    return view;
  };

  const checkBox = async (style: (typeof RADIO_DISPLAY_STYLES)[number], power: boolean) => {
    await showStyle(style, power);
    // il riquadro compare subito; il valore e lo SPLIT acceso arrivano con lo stato della radio
    await waitFor(() => expect(document.querySelector('.rd-split-btn--on')).not.toBeNull());
    const boxes = document.querySelectorAll('.rd-pwrsplit');
    expect(boxes).toHaveLength(1);
    const box = boxes[0] as HTMLElement;
    // dall'alto: PWR, valore («—» senza potenza), - e +, SPLIT acceso
    expect(Array.from(box.children).map((c) => c.className.split(' ')[0])).toEqual(['rd-pwrsplit-lbl', 'rd-pwrsplit-val', 'rd-pwrsplit-btns', 'rd-split-btn']);
    expect(box.querySelector('.rd-pwrsplit-lbl')?.textContent).toBe('PWR');
    expect(box.querySelector('.rd-pwrsplit-val')?.textContent).toBe(power ? '100%' : '—');
    expect(box.querySelectorAll('.rd-pwrsplit-btn')).toHaveLength(2);
    // un solo tasto SPLIT in tutto il display, niente PWR doppio ne' colonna a parte, il pulsantino di stacco fuori dal riquadro
    expect(document.querySelectorAll('.rd-split-btn')).toHaveLength(1);
    expect(document.querySelector('.rd-side')).toBeNull();
    expect(Array.from(document.querySelectorAll('.rx-chip')).some((c) => c.textContent?.includes('PWR'))).toBe(false);
    expect(box.closest('.rd-detach-corner')).toBeNull();
    expect(box.querySelector('.rd-detach-corner, .dock-detach')).toBeNull();
  };

  it.each(RADIO_DISPLAY_STYLES)('shows one PWR + SPLIT box in the %s style', (style) => checkBox(style, true));
  it.each(RADIO_DISPLAY_STYLES)('shows the same PWR + SPLIT box with a dash instead of the power in the %s style', (style) => checkBox(style, false));

  it('keeps the PWR + SPLIT box narrower and not taller than the smallest display next to it (CSS)', async () => {
    // il foglio di stile si legge dal disco (nei test i CSS importati sono vuoti); si parte dalla cartella dell'app
    const fs = (await import('node:fs')) as { readFileSync: (p: string, enc: string) => string };
    const css = fs.readFileSync('src/styles/radiodisplay.css', 'utf8');
    const rule = /\.rd-pwrsplit \{([^}]*)\}/.exec(css)?.[1] ?? '';
    const rem = (prop: string) => Number(new RegExp(`(?:^|[\\s;])${prop}:\\s*([0-9.]+)rem`).exec(rule)?.[1]);
    // i display piu' bassi (pannelli, estesi e due blocchi) sono alti 5,625rem
    expect(rem('height')).toBeGreaterThan(0);
    expect(rem('height')).toBeLessThanOrEqual(5.625);
    expect(rem('width')).toBeGreaterThan(0);
    expect(rem('width')).toBeLessThanOrEqual(4.4);
    expect(rule).not.toContain('min-height');
  });
});
