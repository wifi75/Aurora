import { describe, expect, it } from 'vitest';
import {
  ALERT_STORAGE_KEY,
  DEFAULT_ALERT_SETTINGS,
  FADE_S,
  FLASH_S,
  alertViews,
  applyMarks,
  dueAlerts,
  loadAlertSettings,
  mergePasses,
  remember,
  sanitizeAlertSettings,
  saveAlertSettings,
  type AlertInput,
  type AlertMark,
  type AlertPass,
  type AlertRecord,
  type LiveSat,
  type SatAlertSettings,
} from './satAlerts';
import { alertSoundDef, playAlert } from './satAlertSound';

const T0 = 1_800_000_000;
const ISS = 25544;
const settings = (over: Partial<SatAlertSettings> = {}): SatAlertSettings => ({ ...DEFAULT_ALERT_SETTINGS, ...over });
const pass = (over: Partial<AlertPass> = {}): AlertPass => ({ norad: ISS, name: 'ISS', aos_unix: T0 + 600, los_unix: T0 + 1200, max_el_deg: 40, ...over });
const live = (...v: [number, boolean][]): Map<number, LiveSat> => new Map(v.map(([n, visible]) => [n, { visible, name: n === ISS ? 'ISS' : `SAT${n}` }]));

const base = (over: Partial<AlertInput> = {}): AlertInput => ({
  passes: [pass()],
  now: T0,
  prevNow: T0 - 1,
  settings: settings(),
  selected: ISS,
  tracked: [ISS],
  live: live([ISS, false]),
  prevLive: live([ISS, false]),
  minEl: 5,
  log: [],
  ...over,
});

const kinds = (input: AlertInput) => dueAlerts(input).map((e) => e.kind);

describe('preavviso', () => {
  it('non scatta prima della finestra di preavviso', () => {
    expect(kinds(base({ now: T0 + 600 - 5 * 60 - 1 }))).toEqual([]);
  });

  it('scatta quando mancano meno di N minuti, con i secondi all\'AOS', () => {
    const ev = dueAlerts(base({ now: T0 + 600 - 200 }));
    expect(ev).toHaveLength(1);
    expect(ev[0]).toMatchObject({ kind: 'pre', norad: ISS, name: 'ISS', secondsToAos: 200 });
  });

  it('con preavviso 0 non scatta mai', () => {
    expect(kinds(base({ settings: settings({ leadMin: 0 }), now: T0 + 599 }))).toEqual([]);
  });

  it('non scatta se il passaggio e\' gia\' iniziato', () => {
    expect(kinds(base({ now: T0 + 600 }))).toEqual([]);
  });

  it('non ripete lo stesso preavviso', () => {
    const input = base({ now: T0 + 400 });
    const first = dueAlerts(input);
    const log = remember([], first, input.now);
    expect(dueAlerts({ ...input, now: input.now + 1, prevNow: input.now, log })).toEqual([]);
  });

  it('ignora i passaggi che non raggiungono l\'elevazione minima', () => {
    expect(kinds(base({ passes: [pass({ max_el_deg: 3 })], now: T0 + 400, minEl: 5 }))).toEqual([]);
  });
});

describe('AOS e LOS', () => {
  it('AOS esatto: scatta quando il satellite diventa visibile', () => {
    const ev = dueAlerts(base({ now: T0 + 610, prevNow: T0 + 609, prevLive: live([ISS, false]), live: live([ISS, true]) }));
    expect(ev.map((e) => e.kind)).toEqual(['aos']);
    expect(ev[0]?.los).toBe(T0 + 1200);
  });

  it('LOS: scatta quando smette di essere visibile', () => {
    const ev = dueAlerts(base({ now: T0 + 1100, prevNow: T0 + 1099, prevLive: live([ISS, true]), live: live([ISS, false]) }));
    expect(ev.map((e) => e.kind)).toEqual(['los']);
  });

  it('niente avviso se lo stato non cambia', () => {
    expect(kinds(base({ prevLive: live([ISS, true]), live: live([ISS, true]), now: T0 + 700 }))).toEqual([]);
  });

  it('niente avviso al primo campione (satellite mai visto prima)', () => {
    expect(kinds(base({ prevLive: new Map(), live: live([ISS, true]), now: T0 + 700 }))).toEqual([]);
  });

  it('non perde l\'evento se il browser e\' rimasto fermo pochi secondi', () => {
    const ev = dueAlerts(base({ now: T0 + 640, prevNow: T0 + 615, prevLive: live([ISS, false]), live: live([ISS, true]) }));
    expect(ev.map((e) => e.kind)).toEqual(['aos']);
  });

  it('dopo una pausa lunga non recupera gli eventi vecchi', () => {
    const input = base({ now: T0 + 5000, prevNow: T0 + 600, prevLive: live([ISS, false]), live: live([ISS, true]) });
    expect(kinds(input)).toEqual([]);
  });

  it('orologio che salta indietro: nessun avviso, poi riparte', () => {
    expect(kinds(base({ now: T0 + 100, prevNow: T0 + 700, prevLive: live([ISS, false]), live: live([ISS, true]) }))).toEqual([]);
  });

  it('orologio che salta in avanti oltre l\'AOS: niente preavviso fuori tempo', () => {
    expect(kinds(base({ now: T0 + 3600, prevNow: T0 }))).toEqual([]);
  });

  it('nessun doppione: lo stesso passaggio riconsegnato con aos = adesso non rifa l\'AOS', () => {
    const input = base({ now: T0 + 610, prevNow: T0 + 609, prevLive: live([ISS, false]), live: live([ISS, true]) });
    const first = dueAlerts(input);
    const log = remember([], first, input.now);
    // dopo l'aggiornamento dei passaggi il passaggio in corso ha l'inizio uguale all'istante della richiesta e la fine di poco diversa
    const again = pass({ aos_unix: T0 + 640, los_unix: T0 + 1201.4 });
    const flicker = dueAlerts({ ...input, passes: [again], now: T0 + 641, prevNow: T0 + 640, prevLive: live([ISS, false]), live: live([ISS, true]), log });
    expect(flicker).toEqual([]);
  });

  it('un passaggio successivo dello stesso satellite da\' di nuovo l\'avviso', () => {
    const log: AlertRecord[] = [{ kind: 'aos', norad: ISS, los: T0 + 1200 }];
    const later = pass({ aos_unix: T0 + 6000, los_unix: T0 + 6600 });
    const ev = dueAlerts(base({ passes: [later], now: T0 + 6010, prevNow: T0 + 6009, prevLive: live([ISS, false]), live: live([ISS, true]), log }));
    expect(ev.map((e) => e.kind)).toEqual(['aos']);
  });
});

describe('quali satelliti', () => {
  it('un satellite non spuntato non da\' avvisi', () => {
    expect(kinds(base({ tracked: [], selected: null, now: T0 + 400 }))).toEqual([]);
    expect(kinds(base({ tracked: [], selected: null, now: T0 + 610, prevNow: T0 + 609, live: live([ISS, true]) }))).toEqual([]);
  });

  it('con "solo il satellite scelto" ignora gli altri', () => {
    const other = pass({ norad: 7530, name: 'AO-7' });
    const input = base({ passes: [pass(), other], tracked: [ISS, 7530], settings: settings({ onlySelected: true }), now: T0 + 400 });
    expect(dueAlerts(input).map((e) => e.norad)).toEqual([ISS]);
  });

  it('con "solo il satellite scelto" vale anche se non e\' spuntato', () => {
    expect(kinds(base({ tracked: [], settings: settings({ onlySelected: true }), now: T0 + 400 }))).toEqual(['pre']);
  });

  it('con gli avvisi spenti non succede niente', () => {
    expect(kinds(base({ settings: settings({ enabled: false }), now: T0 + 400 }))).toEqual([]);
    expect(alertViews({ ...base({ settings: settings({ enabled: false }), now: T0 + 400 }), marks: new Map() })).toEqual([]);
  });
});

describe('registro e segni', () => {
  it('toglie dal registro i passaggi finiti da piu\' di un\'ora', () => {
    const log: AlertRecord[] = [{ kind: 'aos', norad: ISS, los: T0 - 4000 }, { kind: 'aos', norad: ISS, los: T0 + 100 }];
    expect(remember(log, [], T0)).toEqual([{ kind: 'aos', norad: ISS, los: T0 + 100 }]);
  });

  it('applyMarks segna AOS e LOS', () => {
    const m = applyMarks(new Map(), [{ kind: 'aos', norad: ISS, name: 'ISS', los: T0 + 1200 }], T0 + 610);
    expect(m.get(ISS)).toEqual({ aosAt: T0 + 610, aosLos: T0 + 1200 });
    const m2 = applyMarks(m, [{ kind: 'los', norad: ISS, name: 'ISS', los: T0 + 1200 }], T0 + 1200);
    expect(m2.get(ISS)).toMatchObject({ aosAt: T0 + 610, losAt: T0 + 1200 });
  });

  it('mergePasses toglie i doppioni dello stesso passaggio', () => {
    const merged = mergePasses([pass()], [pass({ aos_unix: T0 + 300, los_unix: T0 + 1201 }), pass({ aos_unix: T0 + 6000, los_unix: T0 + 6600 })]);
    expect(merged).toHaveLength(2);
  });
});

describe('fasi della grafica', () => {
  const views = (now: number, liveSats: Map<number, LiveSat>, marks: Map<number, AlertMark> = new Map(), log: AlertRecord[] = [], s = settings()) => alertViews({ ...base({ now, live: liveSats, log, settings: s }), marks });

  it('niente prima del preavviso', () => {
    expect(views(T0 + 100, live([ISS, false]))).toEqual([]);
  });

  it('preavviso con il conto alla rovescia', () => {
    expect(views(T0 + 400, live([ISS, false]))).toEqual([{ norad: ISS, name: 'ISS', phase: 'pre', secondsToAos: 200 }]);
  });

  it('dopo l\'orizzonte ma prima dell\'area resta in preavviso solo se era stato dato', () => {
    expect(views(T0 + 620, live([ISS, false]))).toEqual([]);
    const log: AlertRecord[] = [{ kind: 'pre', norad: ISS, los: T0 + 1200 }];
    expect(views(T0 + 620, live([ISS, false]), new Map(), log)[0]).toMatchObject({ phase: 'pre', secondsToAos: 0 });
  });

  it('lampo nei secondi dopo l\'AOS, poi anello fisso', () => {
    const marks = new Map<number, AlertMark>([[ISS, { aosAt: T0 + 650, aosLos: T0 + 1200 }]]);
    expect(views(T0 + 650 + FLASH_S - 1, live([ISS, true]), marks)[0]?.phase).toBe('flash');
    expect(views(T0 + 650 + FLASH_S + 1, live([ISS, true]), marks)[0]?.phase).toBe('live');
  });

  it('passaggio gia\' in corso all\'apertura della pagina: anello fisso, nessun lampo', () => {
    expect(views(T0 + 700, live([ISS, true]))[0]?.phase).toBe('live');
  });

  it('dissolvimento alla LOS e poi niente (e il preavviso non ritorna)', () => {
    const marks = new Map<number, AlertMark>([[ISS, { aosAt: T0 + 650, aosLos: T0 + 1200, losAt: T0 + 1100, losLos: T0 + 1200 }]]);
    const log: AlertRecord[] = [{ kind: 'pre', norad: ISS, los: T0 + 1200 }];
    expect(views(T0 + 1100 + FADE_S - 1, live([ISS, false]), marks, log)[0]?.phase).toBe('fade');
    expect(views(T0 + 1100 + FADE_S + 1, live([ISS, false]), marks, log)).toEqual([]);
  });
});

describe('impostazioni nel browser', () => {
  const store = (initial?: string) => {
    const data: Record<string, string> = initial === undefined ? {} : { [ALERT_STORAGE_KEY]: initial };
    return { data, getItem: (k: string) => data[k] ?? null, setItem: (k: string, v: string) => void (data[k] = v) };
  };
  const broken = { getItem: () => { throw new Error('no'); }, setItem: () => { throw new Error('no'); } };

  it('salva e rilegge', () => {
    const s = store();
    saveAlertSettings(settings({ volume: 20, leadMin: 3, soundAos: 'arpeggio', onlySelected: true }), s);
    expect(loadAlertSettings(s)).toMatchObject({ volume: 20, leadMin: 3, soundAos: 'arpeggio', onlySelected: true });
  });

  it('funziona anche se la memoria non risponde', () => {
    expect(loadAlertSettings(broken)).toEqual(DEFAULT_ALERT_SETTINGS);
    expect(() => saveAlertSettings(settings(), broken)).not.toThrow();
    expect(loadAlertSettings(null)).toEqual(DEFAULT_ALERT_SETTINGS);
  });

  it('un valore guasto riprende quello di base', () => {
    expect(loadAlertSettings(store('{non json'))).toEqual(DEFAULT_ALERT_SETTINGS);
    expect(sanitizeAlertSettings({ volume: 900, leadMin: -4, soundPre: 'fischio', enabled: 'si' })).toEqual({ ...DEFAULT_ALERT_SETTINGS, volume: 100, leadMin: 0 });
  });
});

describe('suoni', () => {
  it('campanella: due note che salgono; radar: tre bip crescenti; arpeggio: quattro note', () => {
    const notes = (s: 'bell' | 'radar' | 'arpeggio') => alertSoundDef(s, 'aos').tones.filter((t) => (t.gain ?? 1) > 0.3 || s !== 'bell');
    const bell = alertSoundDef('bell', 'aos').tones.filter((t) => t.gain === undefined);
    expect(bell).toHaveLength(2);
    expect(bell[1]!.hz).toBeGreaterThan(bell[0]!.hz);
    const radar = notes('radar');
    expect(radar).toHaveLength(3);
    expect(radar[2]!.hz).toBeGreaterThan(radar[0]!.hz);
    expect(radar[2]!.gain).toBeGreaterThan(radar[0]!.gain!);
    expect(notes('arpeggio')).toHaveLength(4);
  });

  it('il preavviso e\' piu\' corto; la LOS ha due note che scendono', () => {
    expect(alertSoundDef('arpeggio', 'pre').tones).toHaveLength(2);
    const los = alertSoundDef('arpeggio', 'los').tones;
    expect(los).toHaveLength(2);
    expect(los[1]!.hz).toBeLessThan(los[0]!.hz);
  });

  it('playAlert programma gli oscillatori solo se l\'audio e\' sbloccato e il volume non e\' zero', () => {
    let started = 0;
    const node = () => {
      const n: Record<string, unknown> = {};
      const f = () => n;
      Object.assign(n, { connect: () => n, disconnect: f, start: () => void started++, stop: f, frequency: { setValueAtTime: f, exponentialRampToValueAtTime: f, value: 0 }, gain: { setValueAtTime: f, linearRampToValueAtTime: f, exponentialRampToValueAtTime: f, value: 0 }, type: 'sine' });
      return n;
    };
    const ctx = (state: string) => ({ state, currentTime: 0, destination: {}, createOscillator: node, createGain: node, createBiquadFilter: node, resume: () => Promise.resolve() }) as unknown as AudioContext;
    expect(playAlert('aos', 'radar', 60, ctx('suspended'))).toBe(false);
    expect(playAlert('aos', 'radar', 0, ctx('running'))).toBe(false);
    expect(playAlert('aos', 'off', 60, ctx('running'))).toBe(false);
    expect(started).toBe(0);
    expect(playAlert('aos', 'radar', 60, ctx('running'))).toBe(true);
    expect(started).toBe(3);
  });
});
