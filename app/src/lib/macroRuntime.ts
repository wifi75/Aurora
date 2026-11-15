/**
 * L'esecuzione dei comandi delle macro che la pagina dei modi testuali passa come `cmd` (vedi `macroExtra.ts`): ognuno usa i pezzi di
 * Aurora che gia' ci sono (CAT, WinKeyer, rotore, audio, log, DX cluster, SSTV) attraverso le funzioni che la pagina gli da'.
 */
import {
  ApiError,
  connectCat,
  digiAbort,
  digiCat,
  digiTxClose,
  digiTxOpen,
  disconnectCat,
  getCat,
  getClusterSpots,
  getLogQsos,
  getSstv,
  getSstvImages,
  getSstvTxPhoto,
  postSplitDx,
  putInputLevel,
  putOutputLevel,
  rotatorStop,
  sendClusterCommand,
  sstvImageUrl,
  sstvTransmit,
  digiMacroPlay,
  digiMacroRecord,
  digiWinKeyerSend,
  type DigiSettings,
} from '../api/client';
import { txLiveOptions } from './digiTxLive';

/** Quello che la pagina mette a disposizione dei comandi. */
export interface MacroDeps {
  t: (key: string, vars?: Record<string, string | number>) => string;
  toast: (text: string, kind: 'info' | 'ok' | 'error') => void;
  settings: () => DigiSettings;
  save: (patch: Partial<DigiSettings>) => void;
  rxHz: () => number;
  dialHz: () => number;
  /** La ricezione su questa frequenza audio (`snap`: aggancia il segnale vicino). */
  pickFreq: (hz: number, snap: boolean) => void;
  /** La radio su questa frequenza (Hz). */
  setDial: (hz: number) => void;
  clearRx: () => void;
  clearTx: () => void;
  setTx: (text: string) => void;
  selection: () => string;
  rxText: () => string;
  appendRx: (line: string) => void;
  removeRxLines: (test: (line: string) => boolean) => void;
  setStart: (d: Date) => void;
  qsoCall: () => string;
  qsoGrid: () => string;
  macroNames: () => string[];
  runMacro: (name: string) => boolean;
  view: (name: string, how: 'show' | 'hide' | 'toggle') => boolean;
  extras: { add: () => void; closeAll: () => void; swapFirst: () => void };
  setRxPaused: (paused: boolean) => void;
  rxStyle: (patch: Record<string, string>) => void;
  setMacroMode: (mode: string | null) => void;
  openSettings: (tab: string) => void;
  setCapsOnly: (on: boolean) => void;
  invalidateLog: () => void;
  ignoreBeforeErase: () => void;
}

/** Lo stato dei comandi che dura finche' la pagina e' aperta (blocchi, timer, attese, colore dei segni...). */
export const macroState = {
  modeLocked: false,
  rxLocked: false,
  wordMode: false,
  broadcast: false,
  vfoSync: false,
  vfoCenter: 1500,
  markers: true,
  sstvMode: '',
  rxLog: null as null | { from: number },
  timers: [] as number[],
  onRx: [] as { pattern: RegExp; macro: string }[],
  catChain: Promise.resolve() as Promise<void>,
  told: new Set<string>(),
};

const why = (e: unknown) => (e instanceof ApiError ? (e.message ?? e.code) : e instanceof Error ? e.message : String(e));
const on = (arg: string) => !/^(OFF|0|NO|FALSE)$/i.test(arg.trim());

/** Una volta per pagina: l'avviso di una cosa che Aurora non ha (non a ogni pressione della macro). */
function notInAurora(d: MacroDeps, tag: string, what: string): void {
  if (macroState.told.has(tag)) return;
  macroState.told.add(tag);
  d.toast(d.t('digi.run.notInAurora', { tag, what }), 'info');
}

function fail(d: MacroDeps, tag: string) {
  return (e: unknown) => d.toast(d.t('digi.run.cmdFail', { tag, why: why(e) }), 'error');
}

/** Un comando CAT in coda agli altri della macro (`<CATWAIT>` li distanzia). */
export function queueCat(d: MacroDeps, run: () => Promise<unknown>, tag = 'CAT'): void {
  macroState.catChain = macroState.catChain.then(() => run().then(() => undefined, fail(d, tag)));
}

/** Un testo con `*` come jolly diventa una ricerca (maiuscole e minuscole uguali). */
export function wildcard(text: string): RegExp {
  const esc = text.trim().replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*');
  return new RegExp(esc.startsWith('.*') || esc.endsWith('.*') ? esc : `.*${esc}.*`, 'is');
}

/** Il testo nuovo della ricezione: le attese di `<OnRX>` che lo riconoscono fanno partire la loro macro. */
export function checkOnRx(d: MacroDeps, added: string): void {
  if (added.trim() === '') return;
  for (const w of macroState.onRx) if (w.pattern.test(added)) d.runMacro(w.macro);
}

/** Un'immagine (JPEG, PNG...) portata alla misura del modo SSTV, in PNG. */
async function fitPicture(blob: Blob, width: number, height: number, call: string | null): Promise<Blob> {
  const bmp = await createImageBitmap(blob);
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const g = canvas.getContext('2d');
  if (!g) throw new Error('canvas');
  g.drawImage(bmp, 0, 0, width, height);
  if (call) {
    g.font = `bold ${Math.round(height / 8)}px sans-serif`;
    g.lineWidth = 4;
    g.strokeStyle = '#000';
    g.fillStyle = '#fff';
    g.strokeText(call, 8, height / 8 + 4);
    g.fillText(call, 8, height / 8 + 4);
  }
  return await new Promise<Blob>((ok, ko) => canvas.toBlob((b) => (b ? ok(b) : ko(new Error('png'))), 'image/png'));
}

async function sendPicture(d: MacroDeps, tag: string, blob: Blob, call: string | null): Promise<void> {
  const snap = await getSstv();
  const want = (macroState.sstvMode || snap.settings.default_mode).toLowerCase().replace(/\s+/g, '');
  const mode = snap.modes.find((m) => m.id.toLowerCase() === want || m.name.toLowerCase().replace(/\s+/g, '') === want) ?? snap.modes.find((m) => m.id === snap.settings.default_mode) ?? snap.modes[0];
  if (!mode) throw new Error('SSTV');
  await sstvTransmit(mode.id, await fitPicture(blob, mode.width, mode.height, call), call);
  d.toast(`SSTV ${mode.name}`, 'ok');
  void tag;
}

/** Esegue un comando `cmd`. */
export function runCommand(tag: string, arg: string, d: MacroDeps): void {
  const s = d.settings();
  const n = Number(arg.replace(',', '.'));
  const num = arg.trim() !== '' && Number.isFinite(n) ? n : null;
  switch (tag) {
    case 'SAVESELTEXT': {
      const sel = d.selection().trim();
      if (sel !== '') void navigator.clipboard?.writeText(sel).then(() => d.toast(d.t('digi.run.copied'), 'ok'), fail(d, tag));
      return;
    }
    // la lingua resta per le macro seguenti (vedi DigiPage: `forcedLang`)
    case 'FORCELANGUAGE': return;
    case 'STARTTIME': {
      const m = /^(\d{1,2}):?(\d{2})$/.exec(arg.trim());
      if (!m) return;
      const at = new Date();
      at.setUTCHours(Number(m[1]), Number(m[2]), 0, 0);
      d.setStart(at);
      return;
    }
    case 'TIMER': {
      const [secs = '', macro = ''] = arg.split(',');
      const sec = Number(secs.trim());
      for (const id of macroState.timers) window.clearTimeout(id);
      macroState.timers = [];
      if (!Number.isFinite(sec) || sec <= 0) return;
      d.toast(d.t('digi.run.timer', { s: sec }), 'info');
      macroState.timers.push(
        window.setTimeout(() => {
          if (macro.trim() === '' || !d.runMacro(macro.trim())) d.toast(d.t('digi.run.timer', { s: 0 }), 'info');
        }, sec * 1000),
      );
      return;
    }
    case 'READLOG':
    case 'REFRESH': d.invalidateLog(); return;
    case 'WKD': {
      const call = d.qsoCall().trim().toUpperCase();
      if (call === '') return;
      void getLogQsos().then((r) => {
        const q = r.qsos.filter((x) => x.call.toUpperCase() === call).sort((a, b) => `${b.date}${b.time}`.localeCompare(`${a.date}${a.time}`))[0];
        d.toast(q ? d.t('digi.run.wkd', { call, date: q.date, time: q.time, band: q.band, mode: q.mode }) : d.t('digi.run.wkdNone', { call }), 'info');
      }, fail(d, tag));
      return;
    }
    case 'PRINTQSL':
    case 'QSLPRINT': {
      const w = window.open('', '_blank', 'width=480,height=320');
      if (!w) return;
      const call = d.qsoCall().toUpperCase();
      const now = new Date().toISOString();
      w.document.title = `QSL ${call}`;
      w.document.body.innerHTML = '';
      const pre = w.document.createElement('pre');
      pre.style.font = '16px monospace';
      pre.textContent = `TO: ${call}\nDATE: ${now.slice(0, 10)}  UTC: ${now.slice(11, 16)}\nMHZ: ${(d.dialHz() / 1e6).toFixed(3)}  MODE: ${s.mode.toUpperCase()}\nPSE/TNX QSL`;
      w.document.body.appendChild(pre);
      w.print();
      return;
    }
    case 'SENDEQSL':
    case 'SENDEQSLNP': d.toast(d.t('digi.run.eqsl'), 'info'); return;
    case 'SENDSPOT': {
      const call = d.qsoCall().trim().toUpperCase();
      if (call === '' || d.dialHz() <= 0) return;
      const khz = ((d.dialHz() + d.rxHz()) / 1000).toFixed(1);
      void getClusterSpots(1).then((r) => {
        const server = r.servers.find((x) => x.state === 'connected')?.key;
        if (!server) {
          d.toast(d.t('digi.run.noCluster'), 'info');
          return undefined;
        }
        return sendClusterCommand(server, `DX ${khz} ${call} ${arg || s.mode.toUpperCase()}`.trim()).then(() => d.toast(d.t('digi.run.spotSent', { call, khz }), 'ok'));
      }, fail(d, tag));
      return;
    }
    case 'QSOCMDDDE': notInAurora(d, tag, 'DDE'); return;
    default: return;
  }
}

/** La radio si sposta perche' il segnale in ricezione cada su `center` Hz (la ricezione va li'). */
function zeroBeat(d: MacroDeps, center: number): void {
  const dial = d.dialHz();
  if (dial <= 0) return;
  d.setDial(dial + Math.round(d.rxHz() - center));
  d.pickFreq(center, false);
}

const MARK_PREFIX = '----- ';
/** Una riga di segno nella finestra RX: ora UTC e frequenza. */
export function mark(d: MacroDeps): void {
  if (!macroState.markers) return;
  const now = new Date().toISOString().slice(11, 16);
  const khz = d.dialHz() > 0 ? ` ${((d.dialHz() + d.rxHz()) / 1000).toFixed(1)} kHz` : '';
  d.appendRx(`${MARK_PREFIX}${now}z${khz} -----`);
}

function beepOnce(): void {
  try {
    const ctx = new AudioContext();
    const osc = ctx.createOscillator();
    osc.frequency.value = 1000;
    osc.connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.25);
    window.setTimeout(() => void ctx.close(), 500);
  } catch {
    // senza audio nel browser resta l'avviso a schermo
  }
}

/** Il comando CAT di una macro, in coda (vedi `<CATWAIT>`). */
export function sendCat(d: MacroDeps, cmd: string): void {
  queueCat(d, () => digiCat(cmd));
}
