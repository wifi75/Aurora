import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { collectSnapshot, installUiDiagnostics, recentErrors, sendUiSnapshot, snapshotText } from './uiDiagnostics';

describe('uiDiagnostics', () => {
  beforeEach(() => {
    window.localStorage.clear();
    if (!window.matchMedia) {
      Object.defineProperty(window, 'matchMedia', { value: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }), configurable: true });
    }
  });
  afterEach(() => vi.restoreAllMocks());

  it('describes the window and the page without the content of the saved values', () => {
    window.localStorage.setItem('aurora-secret-test', 'IU3ABC JN80zz');
    const snap = collectSnapshot() as Record<string, any>;
    expect(snap.finestra.interna).toEqual([window.innerWidth, window.innerHeight]);
    expect(snap.finestra.schermo).toHaveLength(2);
    expect(typeof snap.finestra.scala_dello_schermo).toBe('number');
    expect(snap.pagina).toMatch(/^#/);
    // dell'archivio del browser si dicono nomi e dimensione, mai il contenuto
    expect(snap.archivio_del_browser.aurora['aurora-secret-test']).toBe('IU3ABC JN80zz'.length);
    expect(snapshotText()).not.toContain('IU3ABC');
  });

  it('keeps the route without its parameters (they may carry codes)', () => {
    window.location.hash = '#/impostazioni?code=ABC123';
    expect((collectSnapshot() as Record<string, unknown>).pagina).toBe('#/impostazioni');
    window.location.hash = '';
  });

  it('remembers the errors of the page, also the ones written to the console, and stays within the size the program accepts', () => {
    // prima si silenzia la console vera, poi si installa la raccolta (che ci si appoggia sopra)
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
    installUiDiagnostics();
    console.error('pagina in errore', new Error('Minified React error #310'));
    window.dispatchEvent(new ErrorEvent('error', { message: 'boom', filename: 'http://x/assets/App.js', lineno: 3, colno: 9 }));
    for (let i = 0; i < 80; i++) console.error(`errore ${i} ${'x'.repeat(380)}`);
    const found = recentErrors();
    expect(found.length).toBeLessThanOrEqual(30);
    expect(found.some((e) => e.kind === 'console' && e.text.includes('errore 79'))).toBe(true);
    expect(snapshotText().length).toBeLessThanOrEqual(14_000);
    expect(() => JSON.parse(snapshotText())).not.toThrow();
  });

  it('sends the snapshot to the program and never throws when the program does not answer', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{"ok":true}'));
    vi.stubGlobal('fetch', fetchMock);
    await sendUiSnapshot();
    expect(fetchMock).toHaveBeenCalledWith('/api/v1/diagnostics/ui-snapshot', expect.objectContaining({ method: 'POST' }));
    fetchMock.mockRejectedValue(new Error('rete'));
    await expect(sendUiSnapshot()).resolves.toBeUndefined();
    vi.unstubAllGlobals();
  });
});
