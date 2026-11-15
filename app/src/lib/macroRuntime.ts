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
    case 'LOCKMODE': macroState.modeLocked = true; return;
    case 'UNLOCKMODE': macroState.modeLocked = false; return;
    case 'LOCKRX': macroState.rxLocked = true; return;
    case 'UNLOCKRX': macroState.rxLocked = false; return;
    case 'LOCKRXTOGGLE': macroState.rxLocked = !macroState.rxLocked; return;
    case 'MODEMACROS': {
      const m = arg.trim().toLowerCase().replace(/\.mc$/, '').replace(/^bpsk/, 'psk');
      d.setMacroMode(['psk31', 'psk63', 'rtty', 'cw'].includes(m) ? m : null);
      return;
    }
    case 'WORDMODE': macroState.wordMode = on(arg); txLiveOptions.wordMode = macroState.wordMode; return;
    case 'CAPITALSONLY': d.setCapsOnly(on(arg)); return;
    case 'REVERSEPADDLE': if (s.winkeyer) d.save({ winkeyer: { ...s.winkeyer, paddle_swap: !s.winkeyer.paddle_swap } }); return;
    case 'SNAPNOW': d.pickFreq(d.rxHz(), true); return;
    case 'INV': d.save({ rtty_reverse: !s.rtty_reverse }); return;
    case 'INVON': d.save({ rtty_reverse: true }); return;
    case 'INVOFF': d.save({ rtty_reverse: false }); return;
    case 'FILTER': {
      const f = arg.trim().toUpperCase();
      if (f === 'NONE') d.save({ passband_on: false });
      else d.save({ passband_on: true });
      return;
    }
    case 'BROADCASTMODE': macroState.broadcast = on(arg); return;
    case 'TOGGLECHANNEL': d.extras.swapFirst(); return;
    case 'ESCAPE': void digiAbort(); return;
    case 'PTT': {
      if (on(arg)) void digiTxOpen(' ').catch(fail(d, tag));
      else void digiTxClose().catch(fail(d, tag));
      return;
    }
    case 'SET': {
      const [what = '', value = ''] = arg.split(':');
      const v = Number(value.replace(',', '.'));
      const key = what.trim().toUpperCase();
      const val = value.trim().toUpperCase();
      if (key === 'BW' && Number.isFinite(v) && v > 0) {
        if (s.mode === 'cw') d.save({ cw_bandwidth_hz: [50, 100, 150, 250, 500].reduce((a, b) => (Math.abs(b - v) < Math.abs(a - v) ? b : a)) });
        else d.save({ passband_on: true, passband_hz: Math.round(Math.min(1000, Math.max(20, v))) });
      } else if (key === 'SHIFT' && v > 0) d.save({ rtty_shift_hz: v });
      else if ((key === 'BAUD' || key === 'BAUDRATE') && v > 0 && s.mode === 'rtty') d.save({ rtty_baud: v });
      else if ((key === 'WPM' || key === 'SPEED') && v > 0) d.save({ cw_wpm: Math.round(Math.min(60, Math.max(5, v))), cw_wpm_follow_rx: false });
      else if (key === 'SQUELCH') d.save({ squelch_on: on(val) });
      else if (key === 'SUBMODE' && !macroState.modeLocked) {
        const m = val.replace(/^BPSK/, 'PSK');
        const mode = m === 'PSK31' ? 'psk31' : m === 'PSK63' ? 'psk63' : m === 'RTTY' ? 'rtty' : m === 'CW' ? 'cw' : null;
        if (mode) d.save({ mode });
        else notInAurora(d, `SET SUBMODE:${val}`, val);
      }
      // TONES, FILTER e FONT di Hell e MFSK: modi che Aurora non ha, si saltano
      return;
    }
    case 'SPZOOM': d.view('wideWaterfall', num !== null && num >= 2 ? 'show' : 'hide'); return;
    case 'SPSHIFT': if (num !== null && !macroState.rxLocked) d.pickFreq(Math.max(200, Math.min(3500, num)), false); return;
    case 'AMPLITUDE': if (num !== null) void putOutputLevel(Math.max(0, num)).catch(fail(d, tag)); return;
    case 'ZFQ': zeroBeat(d, 1500); return;
    case 'FQB': if (num !== null && num > 0 && d.dialHz() > 0) void postSplitDx(d.dialHz(), Math.round(num * 1000)).catch(fail(d, tag)); return;
    case 'CATBAD': d.toast(d.t('digi.run.cmdFail', { tag: 'CATCMDHEX', why: arg }), 'error'); return;
    case 'CATWAIT': {
      const ms = Math.max(0, Math.min(10_000, num ?? 0));
      macroState.catChain = macroState.catChain.then(() => new Promise((ok) => window.setTimeout(ok, ms)));
      return;
    }
    case 'CATTRANSVERTER': d.openSettings('cat'); return;
    case 'CATVFOSYNCSHIFT': if (num !== null && num > 100) macroState.vfoCenter = num; return;
    case 'CATVFOSYNCONCE': zeroBeat(d, macroState.vfoCenter); return;
    case 'CATENABLEVFOSYNC': macroState.vfoSync = true; zeroBeat(d, macroState.vfoCenter); return;
    case 'CATDISABLEVFOSYNC': macroState.vfoSync = false; return;
    case 'TOGGLECATPORT':
    case 'TOGGLEPTTPORT':
      void getCat().then((c) => (c.status.state === 'connected' ? disconnectCat() : connectCat())).catch(fail(d, tag));
      return;
    case 'TOGGLESOUNDDEV': d.view('restartRx', 'toggle'); return;
    case 'INPUTVOLUME': if (num === null) d.openSettings('audio'); else void putInputLevel({ volume_pct: Math.max(0, Math.min(100, num)), auto: false }).catch(fail(d, tag)); return;
    case 'OUTPUTVOLUME': if (num === null) d.openSettings('audio'); else void putOutputLevel(Math.max(0, num)).catch(fail(d, tag)); return;
    case 'WKTEXT': void digiWinKeyerSend({ text: arg }).catch(fail(d, tag)); return;
    case 'WKHEX': void digiWinKeyerSend({ hex: arg }).catch(fail(d, tag)); return;
    case 'WKPOT':
    case 'WKNOPOT': if (s.winkeyer) d.save({ winkeyer: { ...s.winkeyer, use_pot: tag === 'WKPOT' } }); return;
    case 'ROTORSTOP': void rotatorStop().catch(fail(d, tag)); return;
    case 'ROTORBAD': d.toast(d.t('digi.run.cmdFail', { tag: 'ROTORCMD', why: arg }), 'error'); return;
    case 'NEWRXWINDOW':
    case 'NEWRXWINDOW1':
    case 'NEWRXWINDOW2': d.extras.add(); return;
    case 'CLOSERXWINDOWS': d.extras.closeAll(); return;
    case 'CLEAR': d.clearRx(); d.clearTx(); return;
    case 'SHOW':
    case 'HIDE':
    case 'TOGGLE': {
      const what = arg.trim().toUpperCase();
      if (!d.view(what, tag === 'SHOW' ? 'show' : tag === 'HIDE' ? 'hide' : 'toggle')) notInAurora(d, `${tag}:${what}`, what);
      return;
    }
    case 'SCROLLLOCK': d.setRxPaused(on(arg)); return;
    case 'FONTSIZE': d.rxStyle({ fontSize: num !== null && num >= 6 && num <= 72 ? `${num}px` : '' }); return;
    case 'FACE': d.rxStyle({ fontFamily: arg.replace(/["<>;{}]/g, '').trim() }); return;
    case 'COLOR': {
      const hex = arg.trim().replace(/^#|^0x/i, '');
      d.rxStyle({ color: /^[0-9a-f]{6}$/i.test(hex) ? `#${hex}` : '' });
      return;
    }
    case 'BOLD': d.rxStyle({ fontWeight: on(arg) ? 'bold' : '' }); return;
    case 'ITALIC': d.rxStyle({ fontStyle: on(arg) ? 'italic' : '' }); return;
    case 'MARK': mark(d); return;
    case 'MARKERS': macroState.markers = on(arg); return;
    case 'BMERASE': d.removeRxLines((l) => l.startsWith(`[${MARK_PREFIX}`)); return;
    case 'ALERT': d.toast(arg, 'info'); beepOnce(); return;
    case 'PRINT': {
      const w = window.open('', '_blank');
      if (!w) return;
      const pre = w.document.createElement('pre');
      pre.textContent = d.rxText();
      w.document.body.appendChild(pre);
      w.print();
      return;
    }
    case 'ACTIVATE': window.focus(); return;
    case 'RXLOG': {
      if (arg.toUpperCase() === 'START') macroState.rxLog = { from: d.rxText().length };
      else if (macroState.rxLog) {
        const text = d.rxText().slice(macroState.rxLog.from);
        macroState.rxLog = null;
        const a = document.createElement('a');
        a.href = URL.createObjectURL(new Blob([text], { type: 'text/plain' }));
        a.download = `rx-${new Date().toISOString().slice(0, 16).replace(/[:T]/g, '')}.txt`;
        a.click();
        window.setTimeout(() => URL.revokeObjectURL(a.href), 2000);
      }
      return;
    }
    case 'STARTRECORD':
    case 'STARTRECORD2':
      void digiMacroRecord({ action: 'start', name: arg || undefined }).then((r) => d.toast(d.t('digi.run.recording', { file: r.file ?? '' }), 'info'), fail(d, tag));
      return;
    case 'STOPRECORD':
      void digiMacroRecord({ action: 'stop' }).then((r) => (r.file ? d.toast(d.t('digi.run.recSaved', { file: r.file }), 'ok') : undefined), fail(d, tag));
      return;
    case 'REPLAY': void digiMacroRecord({ action: 'replay', seconds: num ?? 10 }).catch(fail(d, tag)); return;
    case 'STARTPLAY': if (arg) void digiMacroPlay({ play: arg }).catch(fail(d, tag)); return;
    case 'WAVE': if (arg) void digiMacroPlay({ wave: arg }).catch(fail(d, tag)); return;
    case 'DTMF': if (arg) void digiMacroPlay({ dtmf: arg }).catch(fail(d, tag)); return;
    case 'STOPPLAY': void digiAbort(); return;
    case 'FILE': {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.txt,text/plain';
      input.onchange = () => {
        const f = input.files?.[0];
        if (f && f.size <= 65_536) void f.text().then((txt) => d.setTx(txt));
      };
      input.click();
      return;
    }
    case 'ONRX': {
      const cut = arg.lastIndexOf(',');
      if (arg.trim() === '' || cut < 0) {
        macroState.onRx = [];
        return;
      }
      const macro = arg.slice(cut + 1).trim();
      const text = arg.slice(0, cut);
      macroState.onRx = [...macroState.onRx.filter((w) => w.macro !== macro), { pattern: wildcard(text), macro }];
      return;
    }
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
