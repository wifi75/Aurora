import { beforeEach, describe, expect, it } from 'vitest';
import { claimSound, resetClaims } from './satAlertDedupe';
import { DEFAULT_ALERT_SETTINGS, type AlertPass, type SatAlertSettings } from './satAlerts';
import { SHOW_AFTER_AOS_S, rememberPasses, satLabel, satLabelStore, type KnownPass, type PassRef } from './satLabel';

const T0 = 1_800_000_000;
const ISS = 25544;
const pass = (over: Partial<AlertPass> = {}): AlertPass => ({ norad: ISS, name: 'ISS', aos_unix: T0 + 600, los_unix: T0 + 1200, max_el_deg: 30, ...over });
const base = { passes: [pass()], known: [] as KnownPass[], dismissed: [] as PassRef[], now: T0, settings: DEFAULT_ALERT_SETTINGS, selected: ISS as number | null, tracked: [ISS], minEl: 5 };
const settings = (p: Partial<SatAlertSettings>): SatAlertSettings => ({ ...DEFAULT_ALERT_SETTINGS, ...p });

describe('satLabel', () => {
  it('non compare prima del preavviso (5 minuti di base)', () => {
    expect(satLabel(base)).toBeNull();
  });

  it('compare nel preavviso con il conto alla rovescia', () => {
    expect(satLabel({ ...base, now: T0 + 300 })).toEqual({ norad: ISS, los: T0 + 1200, name: 'ISS', phase: 'soon', seconds: 300 });
    expect(satLabel({ ...base, now: T0 + 599.2 })?.seconds).toBe(1);
  });

  it('usa leadMin delle impostazioni', () => {
    expect(satLabel({ ...base, settings: settings({ leadMin: 10 }), now: T0 + 5 })?.seconds).toBe(595);
    expect(satLabel({ ...base, settings: settings({ leadMin: 2 }), now: T0 + 300 })).toBeNull();
  });

  it('mai con preavviso 0 o avvisi spenti', () => {
    expect(satLabel({ ...base, settings: settings({ leadMin: 0 }), now: T0 + 590 })).toBeNull();
    expect(satLabel({ ...base, settings: settings({ enabled: false }), now: T0 + 590 })).toBeNull();
  });

  it('mai per satelliti non seguiti', () => {
    expect(satLabel({ ...base, tracked: [], now: T0 + 590 })).toBeNull();
    expect(satLabel({ ...base, tracked: [1234], now: T0 + 590 })).toBeNull();
  });

  it('con solo il satellite scelto conta solo quello', () => {
    expect(satLabel({ ...base, settings: settings({ onlySelected: true }), selected: 999, tracked: [ISS, 999], now: T0 + 590 })).toBeNull();
  });

  it('mai senza passaggi o con elevazione sotto il minimo', () => {
    expect(satLabel({ ...base, passes: [], now: T0 + 590 })).toBeNull();
    expect(satLabel({ ...base, minEl: 40, now: T0 + 590 })).toBeNull();
  });

  it('dopo l AOS mostra il tempo alla LOS e sparisce a AOS + 60 s', () => {
    const known = rememberPasses([], base.passes, T0 + 300);
    expect(known).toEqual([{ norad: ISS, los: T0 + 1200, aos: T0 + 600 }]);
    // l elenco, aggiornato a passaggio iniziato, ha un AOS finto (l istante della richiesta)
    const live = [pass({ aos_unix: T0 + 630 })];
    const after = rememberPasses(known, live, T0 + 630);
    expect(after[0]?.aos).toBe(T0 + 600);
    expect(satLabel({ ...base, passes: live, known: after, now: T0 + 630 })).toEqual({ norad: ISS, los: T0 + 1200, name: 'ISS', phase: 'now', seconds: 570 });
    expect(satLabel({ ...base, passes: live, known: after, now: T0 + 600 + SHOW_AFTER_AOS_S - 1 })?.phase).toBe('now');
    expect(satLabel({ ...base, passes: live, known: after, now: T0 + 600 + SHOW_AFTER_AOS_S })).toBeNull();
  });

  it('aprendo Aurora a passaggio gia iniziato non compare niente', () => {
    const live = [pass({ aos_unix: T0 + 610 })];
    const known = rememberPasses([], live, T0 + 610);
    expect(satLabel({ ...base, passes: live, known, now: T0 + 610 })).toBeNull();
  });

  it('chiusura a mano: vale per quel passaggio, non per il successivo', () => {
    const passes = [pass(), pass({ aos_unix: T0 + 6000, los_unix: T0 + 6600 })];
    const dismissed = [{ norad: ISS, los: T0 + 1200 }];
    expect(satLabel({ ...base, passes, dismissed, now: T0 + 400 })).toBeNull();
    expect(satLabel({ ...base, passes, dismissed, now: T0 + 5800 })).toMatchObject({ phase: 'soon', los: T0 + 6600 });
  });

  it('di due satelliti vince quello che sorge prima; chiuso il primo si vede il secondo', () => {
    const passes = [pass({ norad: 7, name: 'AO-7', aos_unix: T0 + 500, los_unix: T0 + 900 }), pass()];
    const s = { ...base, passes, tracked: [ISS, 7], now: T0 + 400 };
    expect(satLabel(s)?.name).toBe('AO-7');
    expect(satLabel({ ...s, dismissed: [{ norad: 7, los: T0 + 900 }] })?.name).toBe('ISS');
  });
});

describe('archivio e un solo suono', () => {
  beforeEach(() => {
    satLabelStore.reset();
    resetClaims();
  });

  it('dismiss toglie l etichetta e ricorda il passaggio', () => {
    satLabelStore.setLabel({ norad: ISS, los: T0, name: 'ISS', phase: 'soon', seconds: 10 });
    satLabelStore.dismiss({ norad: ISS, los: T0 });
    expect(satLabelStore.get().label).toBeNull();
    expect(satLabelStore.get().dismissed).toEqual([{ norad: ISS, los: T0 }]);
  });

  it('il secondo motore non suona lo stesso avviso, anche con la fine del passaggio un poco diversa', () => {
    expect(claimSound({ kind: 'pre', norad: ISS, los: T0 + 1200 }, T0)).toBe(true);
    expect(claimSound({ kind: 'pre', norad: ISS, los: T0 + 1230 }, T0 + 1)).toBe(false);
    expect(claimSound({ kind: 'aos', norad: ISS, los: T0 + 1200 }, T0 + 1)).toBe(true);
    expect(claimSound({ kind: 'pre', norad: 7, los: T0 + 1200 }, T0 + 1)).toBe(true);
    // il passaggio dopo si suona
    expect(claimSound({ kind: 'pre', norad: ISS, los: T0 + 7000 }, T0 + 6000)).toBe(true);
  });
});
