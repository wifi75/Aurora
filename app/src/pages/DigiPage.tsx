import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { addLogQso, ApiError, digiAbort, digiRx, postFrequency, digiTransmit, getDigi, importMacros, putDigi, type ClusterSpot, type DigiMacro, type DigiSettings, type DigiSnapshot } from '../api/client';
import { ControlBar } from '../components/digi/ControlBar';
import { FreqPicker } from '../components/FreqPicker';
import { bandPlanRows } from '../lib/bandplan';
import { DigiLog, type Qso } from '../components/digi/DigiLog';
import { DigiWaterfall } from '../components/digi/DigiWaterfall';
import { MacroEditor, type MacroScope } from '../components/digi/MacroEditor';
import { MenuBar, type MenuDef } from '../components/digi/MenuBar';
import { ModeSettings, type Personal } from '../components/digi/ModeSettings';
import { DigiStatusBar } from '../components/digi/StatusBar';
import { useT } from '../i18n';
import { appendDigiNote, appendDigiTx, clearDigiText, getDigiText, onDigi, onDigiText } from '../lib/digiBus';
import { RowAverager, seekSignal, snapToPeak, type SpectrumRow } from '../lib/digiSpectrum';
import { bandOf } from '../lib/format';
import { effectiveMacros, macroNameForKey, runMacro, type Macro, type MacroAction, type MacroContext } from '../lib/macroLang';
import { formatDuration } from '../lib/sstv';
import { browserStorage } from '../lib/theme';
import { toast } from '../lib/toast';
import { useStation } from '../state/station';
import '../styles/digi.css';
import '../styles/digi-dark.css';
import '../styles/digi-log-corta.css';

const FIELDS_KEY = 'aurora.digi.campi';
const VIEWS_KEY = 'aurora.digi.viste';
const SCAN_KEY = 'aurora.digi.scansione';
const MEM_KEY = 'aurora.digi.memorie';
const MODE_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY' };
/** Secondi fra un CQ automatico e il successivo. */
const AUTOCQ_DELAY_S = 8;
/** Il tratto di audio che si ascolta e si mostra (Hz). */
const WF_MIN = 0;
/**: la cascata arriva fino al massimo che il motore calcola (5 kHz), cosi' ogni kHz e' piu' stretto e si vede una banda piu' larga. */
const WF_MAX = 5000;
const SEEK_LIMITS = { min: 300, max: 3300 };

interface Views {
  controlBar: boolean;
  extended: boolean;
  log: boolean;
  waterfall: boolean;
  statusBar: boolean;
  /** Opzioni della Vista. */
  underlineSent: boolean;
  capsOnly: boolean;
  slashedZero: boolean;
  flashTx: boolean;
  hairlines: boolean;
  defaultRst: boolean;
}

interface ScanProps {
  dwell: number;
  once: boolean;
  timeMarks: boolean;
  freqMarks: boolean;
}

const DEFAULT_VIEWS: Views = { controlBar: true, extended: true, log: true, waterfall: true, statusBar: true, underlineSent: true, capsOnly: false, slashedZero: true, flashTx: true, hairlines: false, defaultRst: true };
const DEFAULT_SCAN: ScanProps = { dwell: 10, once: false, timeMarks: true, freqMarks: true };

/** Le righe trasmesse (iniziano con «» ») sottolineate nella finestra di ricezione. */
function withSentUnderlined(text: string): React.ReactNode {
  return text.split('\n').map((line, i, all) => {
    const nl = i < all.length - 1 ? '\n' : '';
    return line.startsWith('» ') ? (
      <u key={i} className="digi-sent">
        {line}
        {nl}
      </u>
    ) : (
      <span key={i}>
        {line}
        {nl}
      </span>
    );
  });
}

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = browserStorage()?.getItem(key);
    if (raw) return { ...fallback, ...(JSON.parse(raw) as Partial<T>) };
  } catch {
    // senza archivio si riparte dai valori di base
  }
  return fallback;
}

function writeJson(key: string, value: unknown): void {
  try {
    browserStorage()?.setItem(key, JSON.stringify(value));
  } catch {
    // senza archivio la scelta vale finche' la pagina e' aperta
  }
}

/** I dati della stazione che non cambiano da un QSO all'altro: restano salvati. */
interface Saved extends Personal {
  rsts: string;
  rstr: string;
}

const EMPTY_QSO: Qso = { call: '', name: '', qth: '', wwloc: '', notes: '' };
const toMacro = (m: DigiMacro): Macro => ({ name: m.name, label: m.label, text: m.text, color: m.color });

/** Il modo del programma per il nome che si usa nelle macro (BPSK31, RTTY...), o `null` se non lo conosce. */
function modeFromName(name: string): DigiSettings['mode'] | null {
  switch (name.toUpperCase()) {
    case 'PSK31':
    case 'BPSK31': return 'psk31';
    case 'PSK63':
    case 'BPSK63': return 'psk63';
    case 'RTTY': return 'rtty';
    default: return null;
  }
}

/** La larghezza del segnale (Hz): le freccette ai lati della bandierina nella cascata. */
/** La pagina dei modi testuali, nella disposizione: menu, barra dei comandi, finestre RX e TX, log di lato, cascata e barra di stato. */
export function DigiPage() {
  const t = useT();
  const client = useQueryClient();
  const snap = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false, refetchInterval: 1000 });
  const data = snap.data;
  const refresh = useCallback(() => void client.invalidateQueries({ queryKey: ['digi'] }), [client]);

  // un evento di stato dal flusso aggiorna subito la pagina
  useEffect(() => onDigi((ev) => (ev.kind === 'rx_status' || ev.kind === 'tx' ? refresh() : undefined)), [refresh]);

  return (
    <div className="digi-page digi-classica">
      {snap.isError ? <p className="notice">{t('set.loadError')}</p> : null}
      {data ? <DigiBody snap={data} onChanged={refresh} /> : null}
    </div>
  );
}

/** Un core piu' vecchio puo' mandare meno campi: i mancanti si riempiono con i valori di base (macro senza tasto = nessuna). */
function withDefaults(snap: DigiSnapshot): DigiSnapshot {
  const raw = snap.settings as Partial<DigiSettings>;
  const settings: DigiSettings = {
    mode: raw.mode ?? 'psk31',
    rx_freq_hz: raw.rx_freq_hz ?? 1000,
    tx_follow_rx: raw.tx_follow_rx ?? true,
    tx_freq_hz: raw.tx_freq_hz ?? 1000,
    rtty_shift_hz: raw.rtty_shift_hz ?? 170,
    rtty_baud: raw.rtty_baud ?? 45.45,
    afc_on: raw.afc_on ?? true,
    squelch_on: raw.squelch_on ?? false,
    squelch_db: raw.squelch_db ?? -45,
    snap_on: raw.snap_on ?? true,
    auto_receive: raw.auto_receive ?? false,
    macros: (raw.macros ?? []).filter((m) => typeof m.name === 'string' && m.name !== ''),
    mode_macros: raw.mode_macros ?? {},
  };
  return { ...snap, settings, rx: { ...snap.rx, freq_hz: snap.rx.freq_hz ?? 0, squelch_open: snap.rx.squelch_open ?? true } };
}

function DigiBody({ snap: rawSnap, onChanged }: { snap: DigiSnapshot; onChanged: () => void }) {
  const snap = useMemo(() => withDefaults(rawSnap), [rawSnap]);
  const t = useT();
  const client = useQueryClient();
  const status = useStation((s) => s.status);
  const text = useSyncExternalStore(onDigiText, getDigiText);
  const rxBox = useRef<HTMLPreElement>(null);
  const txBox = useRef<HTMLTextAreaElement>(null);
  const [tx, setTx] = useState('');
  // larghezza del log di lato (si allarga trascinando il bordo), ricordata fra un avvio e l'altro
  const [logW, setLogW] = useState<number>(() => {
    const n = Number(browserStorage()?.getItem('aurora.digi.logw') ?? '');
    return Number.isFinite(n) && n >= 300 ? n : 432;
  });
  const [qso, setQso] = useState<Qso>(EMPTY_QSO);
  const [saved, setSaved] = useState<Saved>(() => readJson<Saved>(FIELDS_KEY, { myname: '', myqth: '', rsts: '599', rstr: '599' }));
  const [startAt, setStartAt] = useState<Date | null>(null);
  const [views, setViews] = useState<Views>(() => ({ ...DEFAULT_VIEWS, ...readJson<Partial<Views>>(VIEWS_KEY, {}) }));
  const [scanProps, setScanProps] = useState<ScanProps>(() => readJson(SCAN_KEY, DEFAULT_SCAN));
  const [problem, setProblem] = useState('');
  const [editing, setEditing] = useState<{ name: string; scope: MacroScope } | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [scanDialog, setScanDialog] = useState(false);
  const [autoCq, setAutoCq] = useState(false);
  const [scanning, setScanning] = useState(false);
  const s = snap.settings;
  const sending = snap.tx.state === 'sending';
  const lock = !s.tx_follow_rx;
  const rxHz = snap.rx.freq_hz > 0 ? snap.rx.freq_hz : s.rx_freq_hz;

  const macros = useMemo(() => effectiveMacros(s.macros.map(toMacro), s.mode_macros[s.mode]?.map(toMacro)), [s.macros, s.mode_macros, s.mode]);

  // lo spettro: la media delle righe recenti serve a Snap, Seek e Scan
  const avg = useRef(new RowAverager());
  const lastRow = useRef<SpectrumRow | null>(null);
  const onRow = useCallback((row: SpectrumRow) => {
    lastRow.current = row;
    avg.current.push(row);
  }, []);

  // l'ultimo stato, per i comandi dei tasti, la scansione e il CQ automatico (che non devono usare valori vecchi)
  const live = useRef({ snap, tx, qso, saved, startAt, status, macros, scanProps, rxHz });
  live.current = { snap, tx, qso, saved, startAt, status, macros, scanProps, rxHz };

  useEffect(() => {
    const el = rxBox.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [text]);

  const setView = (patch: Partial<Views>) => {
    const next = { ...views, ...patch };
    setViews(next);
    writeJson(VIEWS_KEY, next);
  };

  const save = useCallback(
    (patch: Partial<DigiSettings>) => {
      void putDigi({ ...live.current.snap.settings, ...patch }).then(
        (next) => {
          client.setQueryData(['digi'], next);
          onChanged();
        },
        () => setProblem(t('digi.saveError')),
      );
    },
    [client, onChanged, t],
  );

  const setSavedField = (patch: Partial<Saved>) => {
    const next = { ...saved, ...patch };
    setSaved(next);
    writeJson(FIELDS_KEY, next);
  };

  // il divisore fra ricezione e trasmissione si trascina: la parte alta della colonna (0,2-0,8), ricordata
  // l'altezza della cascata si regola dalla barra sul suo bordo alto (da 70 a 400 px), ricordata
  const [wfHeight, setWfHeight] = useState<number>(() => {
    const n = Number(browserStorage()?.getItem('aurora.digi.wfh') ?? '');
    return Number.isFinite(n) && n >= 70 && n <= 400 ? n : 120;
  });
  const changeWfHeight = (px: number) => {
    const h = Math.round(Math.min(400, Math.max(70, px)));
    setWfHeight(h);
    try {
      browserStorage()?.setItem('aurora.digi.wfh', String(h));
    } catch {
      // l'altezza vale comunque finche' la pagina e' aperta
    }
  };
  const windowsRef = useRef<HTMLDivElement>(null);
  const [rxFrac, setRxFrac] = useState<number>(() => {
    const n = Number(browserStorage()?.getItem('aurora.digi.rxfrac') ?? '');
    return Number.isFinite(n) && n >= 0.2 && n <= 0.8 ? n : 0.55;
  });
  const dragSplit = (e: React.PointerEvent) => {
    e.preventDefault();
    const box = windowsRef.current?.getBoundingClientRect();
    if (!box) return;
    const move = (m: PointerEvent) => setRxFrac(Math.min(0.8, Math.max(0.2, (m.clientY - box.top) / box.height)));
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setRxFrac((f) => {
        try {
          browserStorage()?.setItem('aurora.digi.rxfrac', String(f));
        } catch {
          // la proporzione vale comunque finche' la pagina e' aperta
        }
        return f;
      });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };

  const caps = useRef(false);
  caps.current = views.capsOnly;

  // «Use default RST»: dopo ogni QSO (azzerato o salvato) i rapporti tornano 599
  const resetRst = () => {
    if (views.defaultRst) setSavedField({ rsts: '599', rstr: '599' });
  };

  const send = useCallback(
    (body: string) => {
      const message = (caps.current ? body.toUpperCase() : body).trim();
      if (message === '') return;
      setProblem('');
      digiTransmit(message).then(
        () => {
          appendDigiTx(message);
          setTx('');
          onChanged();
        },
        (e: unknown) => {
          const code = e instanceof ApiError ? e.code : '';
          setProblem(code === 'tx_disabled' ? t('digi.tx.disabled') : code === 'busy' ? t('digi.tx.busy') : code === 'no_transmitter' ? t('digi.tx.notReady') : t('digi.tx.error'));
        },
      );
    },
    [onChanged, t],
  );

  const toggleTx = () => {
    if (live.current.snap.tx.state === 'sending') void digiAbort().then(onChanged, onChanged);
    else send(live.current.tx);
  };

  // ---- sintonia: clic sulla cascata (con Snap), Seek, Scan ----
  const pickFreq = (hz: number, viaClick: boolean, ctrl = false, snapIt = viaClick) => {
    const l = live.current;
    const snapped = snapIt && l.snap.settings.snap_on && avg.current.ready && lastRow.current ? snapToPeak(avg.current.values(), lastRow.current, hz) : hz;
    const f = Math.max(200, Math.min(3500, snapped));
    // in split (Lock) la ricezione resta ferma e il clic sposta la trasmissione; con Ctrl si sposta invece la ricezione
    if (viaClick && !l.snap.settings.tx_follow_rx && !ctrl) {
      save({ tx_freq_hz: f });
      return;
    }
    // con Lock la trasmissione resta dov'e': si sposta solo l'ascolto
    save({ rx_freq_hz: f, ...(l.snap.settings.tx_follow_rx ? { tx_freq_hz: f } : {}) });
  };

  // clic su uno spot del DX Cluster: la radio si porta in modo che il segnale cada sul tono di ricezione; con «up N» nel commento la
  // trasmissione si sposta di N kHz (split dentro l'audio, fino a 3,5 kHz); il nominativo va nel QSO
  const tuneToSpot = async (spot: ClusterSpot) => {
    const l = live.current;
    const want = (spot.mode ?? '').toUpperCase();
    const cur = l.snap.settings.mode;
    if (want === 'RTTY' && cur !== 'rtty') save({ mode: 'rtty' });
    else if (want.startsWith('PSK') && cur === 'rtty') save({ mode: 'psk31' });
    const signalHz = Math.round(spot.freq_khz * 1000);
    const dial = signalHz - Math.round(l.rxHz);
    useStation.getState().patchStatus({ dial_hz: dial });
    await postFrequency(dial);
    setQso({ ...l.qso, call: spot.dx_call });
    const up = /\bup\s*(\d+(?:[.,]\d+)?)/i.exec(spot.comment ?? '');
    if (up) {
      const khz = Number((up[1] ?? '').replace(',', '.'));
      const txAudio = Math.round(l.rxHz + khz * 1000);
      if (khz > 0 && txAudio <= 3500) {
        save({ tx_follow_rx: false, tx_freq_hz: txAudio });
        toast(t('digi.spot.splitUp', { khz: String(khz) }), 'info');
      } else {
        toast(t('digi.spot.splitFar', { khz: String(khz) }), 'info');
      }
    }
  };

  const seek = (direction: 'left' | 'right', quiet = false): number | null => {
    const row = lastRow.current;
    if (!row || !avg.current.ready) {
      if (!quiet) toast(t('digi.run.noSpectrum'), 'info');
      return null;
    }
    const found = seekSignal(avg.current.values(), row, live.current.rxHz, direction, SEEK_LIMITS);
    if (found === null) {
      if (!quiet) toast(t('digi.run.seekNone'), 'info');
      return null;
    }
    pickFreq(found, false);
    return found;
  };

  // la scansione: a ogni sosta si passa al segnale successivo a destra, e dalla fine si riparte dall'inizio
  useEffect(() => {
    if (!scanning) return;
    const timer = window.setInterval(() => {
      const l = live.current;
      const row = lastRow.current;
      if (!row || !avg.current.ready) return;
      let found = seekSignal(avg.current.values(), row, l.rxHz, 'right', SEEK_LIMITS);
      if (found === null) found = seekSignal(avg.current.values(), row, SEEK_LIMITS.min - 40, 'right', SEEK_LIMITS);
      if (found === null) return;
      pickFreq(found, false);
      const now = new Date();
      const marks = [l.scanProps.timeMarks ? `${now.toISOString().slice(11, 19)}z` : '', l.scanProps.freqMarks ? `${Math.round(found)} Hz` : ''].filter(Boolean);
      if (marks.length > 0) appendDigiNote(marks.join(' '));
      if (l.scanProps.once) setScanning(false);
    }, Math.max(2, scanProps.dwell) * 1000);
    return () => window.clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scanning, scanProps.dwell]);

  useEffect(() => {
    if (scanning) toast(t('digi.run.scanOn'), 'info');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scanning]);

  // ---- memorie delle impostazioni (1-20) ----
  const memories = (): Record<string, Partial<DigiSettings>> => readJson<Record<string, Partial<DigiSettings>>>(MEM_KEY, {});
  const memory = (write: boolean, slot: number) => {
    const all = memories();
    if (write) {
      const c = live.current.snap.settings;
      all[String(slot)] = { mode: c.mode, rx_freq_hz: c.rx_freq_hz, tx_freq_hz: c.tx_freq_hz, tx_follow_rx: c.tx_follow_rx, rtty_shift_hz: c.rtty_shift_hz, rtty_baud: c.rtty_baud, afc_on: c.afc_on, snap_on: c.snap_on, squelch_on: c.squelch_on, squelch_db: c.squelch_db };
      writeJson(MEM_KEY, all);
      toast(t('digi.run.memSaved', { n: slot }), 'ok');
    } else if (all[String(slot)]) {
      save(all[String(slot)] as Partial<DigiSettings>);
      toast(t('digi.run.memLoaded', { n: slot }), 'ok');
    } else {
      toast(t('digi.run.memEmpty', { n: slot }), 'info');
    }
  };

  // ---- macro ----
  const context = (): MacroContext => {
    const l = live.current;
    const dial = l.status?.dial_hz ?? 0;
    return {
      mycall: l.status?.de_call ?? '',
      myname: l.saved.myname,
      myqth: l.saved.myqth,
      mywwloc: l.status?.de_grid ?? '',
      call: l.qso.call,
      name: l.qso.name,
      qth: l.qso.qth,
      wwloc: l.qso.wwloc,
      rstr: l.saved.rstr,
      rsts: l.saved.rsts,
      notes: l.qso.notes,
      nrr: '',
      nrs: '',
      startTime: l.startAt,
      now: new Date(),
      dialHz: dial,
      band: dial > 0 ? (bandOf(dial) ?? '') : '',
      modeName: MODE_NAMES[l.snap.settings.mode],
      qsoNumber: 1,
    };
  };

  const saveQso = () => {
    const l = live.current;
    const call = l.qso.call.trim().toUpperCase();
    if (call === '') {
      toast(t('digi.run.noCall'), 'error');
      return;
    }
    const dial = l.status?.dial_hz ?? 0;
    const at = l.startAt ?? new Date();
    void addLogQso({
      call,
      date: at.toISOString().slice(0, 10),
      time: at.toISOString().slice(11, 16),
      band: dial > 0 ? (bandOf(dial) ?? '') : '',
      freq_hz: dial,
      mode: MODE_NAMES[l.snap.settings.mode],
      rst_sent: l.saved.rsts,
      rst_rcvd: l.saved.rstr,
      grid: l.qso.wwloc,
      name: l.qso.name,
      qth: l.qso.qth,
      comment: l.qso.notes,
    }).then(
      () => {
        toast(t('digi.run.saved', { call }), 'ok');
        void client.invalidateQueries({ queryKey: ['log-qsos'] });
      },
      () => toast(t('digi.tx.error'), 'error'),
    );
  };

  const beep = () => {
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      osc.frequency.value = 880;
      osc.connect(ctx.destination);
      osc.start();
      osc.stop(ctx.currentTime + 0.18);
      window.setTimeout(() => void ctx.close(), 400);
    } catch {
      // senza audio nel browser il comando non fa niente
    }
  };

  const toggleSetting = (what: 'afc' | 'snap' | 'squelch' | 'lock', value: boolean | 'toggle') => {
    const c = live.current.snap.settings;
    const cur = { afc: c.afc_on, snap: c.snap_on, squelch: c.squelch_on, lock: !c.tx_follow_rx }[what];
    const on = value === 'toggle' ? !cur : value;
    if (what === 'afc') save({ afc_on: on });
    else if (what === 'snap') save({ snap_on: on });
    else if (what === 'squelch') save({ squelch_on: on });
    // il Lock fissa la trasmissione dove si e' adesso
    else save({ tx_follow_rx: !on, tx_freq_hz: on ? live.current.rxHz : c.tx_freq_hz });
  };

  /** Una macro: i comandi di programma nell'ordine, il testo in trasmissione o nella casella, poi il salvataggio e la pulizia. */
  const execute = (script: string) => {
    const l = live.current;
    const result = runMacro(script, context());
    let toTx = false;
    let rxClear = false;
    let clearTx = false;
    let wantSave = false;
    const apply = (a: MacroAction) => {
      switch (a.type) {
        case 'tx':
        case 'txToggle': toTx = true; break;
        case 'rx': rxClear = rxClear || a.clear; break;
        case 'clearTx': clearTx = true; break;
        case 'clearRx': clearDigiText(); break;
        case 'clearWindow': clearDigiText(); clearTx = true; break;
        case 'clearQso': setQso(EMPTY_QSO); setStartAt(null); resetRst(); break;
        case 'fixStartTime': setStartAt(new Date()); break;
        case 'saveQso': wantSave = true; break;
        case 'mode': {
          const m = modeFromName(a.mode);
          if (m) save({ mode: m });
          else toast(t('digi.run.modeUnsupported', { mode: a.mode }), 'info');
          break;
        }
        case 'baud': {
          const cur = l.snap.settings.mode;
          if (cur !== 'rtty' && Math.abs(a.baud - 31.25) < 0.5) save({ mode: 'psk31' });
          else if (cur !== 'rtty' && Math.abs(a.baud - 62.5) < 0.5) save({ mode: 'psk63' });
          else if (cur === 'rtty' && a.baud >= 30 && a.baud <= 300) save({ rtty_baud: a.baud });
          else toast(t('digi.run.baudUnsupported', { baud: a.baud }), 'info');
          break;
        }
        case 'shift': save({ rtty_shift_hz: a.hz }); break;
        case 'audioFq': pickFreq(Math.max(200, Math.min(3500, a.relative ? l.rxHz + a.hz : a.hz)), false); break;
        case 'cursor': if (a.target === 'TX') txBox.current?.focus(); break;
        case 'beep': beep(); break;
        case 'autoCq': setAutoCq((on) => !on); break;
        case 'seek': seek(a.direction); break;
        case 'toggle': toggleSetting(a.what, a.value); break;
        case 'scan': setScanning(a.run); break;
        case 'memory': memory(a.write, a.slot); break;
        case 'modeSettings':
        case 'threshold': setSettingsOpen(true); break;
        case 'txe':
        case 'rxe': break;
        case 'unknown': toast(t('digi.run.unknown', { tag: a.tag }), 'info'); break;
        default: toast(t('digi.run.unknown', { tag: a.type.toUpperCase() }), 'info');
      }
    };
    result.actions.forEach(apply);
    const body = result.text.replace(/^[\r\n\t ]+|[\r\n\t ]+$/g, '');
    if (toTx) {
      // un comando <TX> da solo trasmette quello che c'e' nella casella
      send(body !== '' ? body : l.tx);
    } else if (body !== '') {
      setTx((v) => (clearTx || v === '' ? body : `${v} ${body}`));
    } else if (clearTx) {
      setTx('');
    }
    if (clearTx && toTx) setTx('');
    if (wantSave) saveQso();
    if (rxClear && !toTx) setTx('');
  };

  const runMacroByName = (name: string): boolean => {
    const m = live.current.macros.get(name);
    if (!m || (m.label.trim() === '' && m.text.trim() === '')) return false;
    execute(m.text);
    return true;
  };

  // i tasti F1-F12 con Ctrl e Shift, il «+» del tastierino (macro Gray+), Pausa (RX/TX) e Esc che ferma la trasmissione
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (editing !== null || settingsOpen || scanDialog) return;
      if (e.key === 'Escape' && live.current.snap.tx.state === 'sending') {
        void digiAbort().then(onChanged, onChanged);
        return;
      }
      if (e.key === 'Pause') {
        e.preventDefault();
        toggleTx();
        return;
      }
      const name = e.code === 'NumpadAdd' ? 'Gray+' : macroNameForKey(e);
      if (name && runMacroByName(name)) e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  // CQ automatico: ripete la macro «CQ» finche' non si spegne, con una pausa fra una chiamata e l'altra
  useEffect(() => {
    if (!autoCq || sending || !snap.can_transmit) return;
    const m = live.current.macros.get('AutoCqText') ?? live.current.macros.get('F2');
    if (!m || m.text.trim() === '') {
      toast(t('digi.run.autoCqNoMacro'), 'info');
      setAutoCq(false);
      return;
    }
    const timer = window.setTimeout(() => execute(m.text), AUTOCQ_DELAY_S * 1000);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq, sending, snap.can_transmit, snap.tx.state]);

  // l'avviso del CQ automatico solo quando cambia, non all'apertura della pagina
  const autoCqFirst = useRef(true);
  useEffect(() => {
    if (autoCqFirst.current) {
      autoCqFirst.current = false;
      return;
    }
    toast(autoCq ? t('digi.run.autoCqOn', { s: AUTOCQ_DELAY_S }) : t('digi.run.autoCqOff'), 'info');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq]);

  const editedMacro: Macro | null = useMemo(() => {
    if (editing === null) return null;
    return macros.get(editing.name) ?? { name: editing.name, label: '', text: '', color: null };
  }, [editing, macros]);

  const saveMacro = (m: Macro, where: MacroScope) => {
    const original = editing?.name ?? m.name;
    const blank = m.label.trim() === '' && m.text.trim() === '';
    const strip = (list: DigiMacro[], name: string) => list.filter((x) => x.name !== name);
    const entry: DigiMacro = { name: m.name, label: m.label, text: m.text, color: m.color };
    let all = [...s.macros];
    const modeSets = { ...s.mode_macros };
    let forMode = [...(modeSets[s.mode] ?? [])];
    // il tasto di partenza si libera dall'insieme che si sta modificando se e' stato cambiato
    if (original !== m.name) {
      all = strip(all, original);
      forMode = strip(forMode, original);
    }
    if (where === 'all') {
      all = strip(all, m.name);
      forMode = strip(forMode, m.name);
      if (!blank) all.push(entry);
    } else {
      forMode = strip(forMode, m.name);
      // vuota per questo modo = il tasto resta libero anche se la macro di base esiste
      if (!blank || all.some((x) => x.name === m.name)) forMode.push(blank ? { name: m.name, label: '', text: '', color: null } : entry);
    }
    modeSets[s.mode] = forMode;
    save({ macros: all, mode_macros: modeSets });
    setEditing(null);
  };

  const importFrom = () => {
    importMacros().then(
      (r) => {
        client.setQueryData(['digi'], r.snapshot);
        onChanged();
        const modes = Object.values(r.imported.modes).reduce((a, b) => a + b, 0);
        toast(t('digi.mi.importDone', { base: r.imported.default_count, modes }), 'ok');
      },
      (e: unknown) => {
        const code = e instanceof ApiError ? e.code : '';
        toast(code === 'local_only' ? t('digi.mi.importLocal') : t('digi.mi.importErr', { why: e instanceof ApiError ? (e.message ?? code) : String(e) }), 'error');
      },
    );
  };

  const copyRx = () => {
    void navigator.clipboard?.writeText(getDigiText()).catch(() => undefined);
  };

  // ---- i menu, come quelli ----
  const slots = Array.from({ length: 20 }, (_, i) => i + 1);
  const menus: MenuDef[] = [
    {
      label: t('digi.menu.file'),
      items: [{ label: t('digi.mi.import'), onSelect: importFrom }],
    },
    {
      label: t('digi.menu.edit'),
      items: [
        { label: t('digi.mi.clearRx'), onSelect: () => clearDigiText() },
        { label: t('digi.mi.clearTx'), onSelect: () => setTx('') },
        'sep',
        { label: t('digi.mi.copyRx'), onSelect: copyRx },
      ],
    },
    {
      label: t('digi.menu.mode'),
      items: [
        ...snap.modes.map((m) => ({ label: m.name, checked: s.mode === m.id, onSelect: () => (s.mode === m.id ? undefined : save({ mode: m.id as DigiSettings['mode'] })) })),
        'sep' as const,
        { label: t('nav.sstv'), onSelect: () => { window.location.hash = '#/sstv'; } },
        ...['CW', 'QPSK31', 'FSK31', 'MFSK', 'Olivia', 'Contestia', 'MT63', 'Throb', 'Hellschreiber', 'Packet', 'Pactor', 'Amtor'].map((n) => ({ label: `${n} ${t('digi.mi.soon')}`, disabled: true })),
        'sep' as const,
        { label: t('digi.mi.modeSettings'), onSelect: () => setSettingsOpen(true) },
        { label: t('digi.mi.threshold'), onSelect: () => setSettingsOpen(true) },
      ],
    },
    {
      label: t('digi.menu.options'),
      items: [
        { label: t('digi.mi.afc'), checked: s.afc_on, onSelect: () => toggleSetting('afc', 'toggle') },
        { label: t('digi.mi.lock'), checked: lock, onSelect: () => toggleSetting('lock', 'toggle') },
        { label: t('digi.mi.snap'), checked: s.snap_on, onSelect: () => toggleSetting('snap', 'toggle') },
        { label: t('digi.mi.squelch'), checked: s.squelch_on, onSelect: () => toggleSetting('squelch', 'toggle') },
        { label: t('digi.mi.threshold'), onSelect: () => setSettingsOpen(true) },
        'sep',
        { label: t('digi.mi.rx'), checked: !sending, onSelect: () => (sending ? toggleTx() : undefined) },
        { label: t('digi.mi.tx'), checked: sending, shortcut: 'Pausa', onSelect: () => (sending ? undefined : toggleTx()) },
        'sep',
        { label: t('digi.mi.seekLeft'), onSelect: () => seek('left') },
        { label: t('digi.mi.seekRight'), onSelect: () => seek('right') },
        {
          label: t('digi.mi.scan'),
          items: [
            { label: t('digi.mi.scanStart'), disabled: scanning, onSelect: () => setScanning(true) },
            { label: t('digi.mi.scanStop'), disabled: !scanning, onSelect: () => { setScanning(false); toast(t('digi.run.scanOff'), 'info'); } },
            { label: t('digi.mi.scanProps'), onSelect: () => setScanDialog(true) },
          ],
        },
        'sep',
        {
          label: t('digi.mi.memory'),
          items: [
            { label: t('digi.mi.memWrite'), items: slots.map((n) => ({ label: t('digi.mi.slot', { n }), onSelect: () => memory(true, n) })) },
            { label: t('digi.mi.memRead'), items: slots.map((n) => ({ label: t('digi.mi.slot', { n }), onSelect: () => memory(false, n) })) },
          ],
        },
      ],
    },
    {
      label: t('digi.menu.view'),
      items: [
        { label: t('digi.mi.controlBar'), checked: views.controlBar, onSelect: () => setView({ controlBar: !views.controlBar }) },
        { label: t('digi.bar.extended'), checked: views.extended, onSelect: () => setView({ extended: !views.extended }) },
        { label: t('digi.mi.log'), checked: views.log, onSelect: () => setView({ log: !views.log }) },
        { label: t('digi.mi.waterfall'), checked: views.waterfall, onSelect: () => setView({ waterfall: !views.waterfall }) },
        { label: t('digi.mi.statusBar'), checked: views.statusBar, onSelect: () => setView({ statusBar: !views.statusBar }) },
        'sep' as const,
        { label: t('digi.mi.underlineSent'), checked: views.underlineSent, onSelect: () => setView({ underlineSent: !views.underlineSent }) },
        { label: t('digi.mi.capsOnly'), checked: views.capsOnly, onSelect: () => setView({ capsOnly: !views.capsOnly }) },
        { label: t('digi.mi.slashedZero'), checked: views.slashedZero, onSelect: () => setView({ slashedZero: !views.slashedZero }) },
        { label: t('digi.mi.flashTx'), checked: views.flashTx, onSelect: () => setView({ flashTx: !views.flashTx }) },
        { label: t('digi.mi.hairlines'), checked: views.hairlines, onSelect: () => setView({ hairlines: !views.hairlines }) },
        { label: t('digi.mi.defaultRst'), checked: views.defaultRst, onSelect: () => setView({ defaultRst: !views.defaultRst }) },
      ],
    },
    {
      label: t('digi.menu.configure'),
      items: [
        { label: t('digi.mi.macrosMode'), onSelect: () => setEditing({ name: 'F1', scope: 'mode' }) },
        { label: t('digi.mi.macrosDefault'), onSelect: () => setEditing({ name: 'F1', scope: 'all' }) },
        'sep',
        { label: t('digi.mi.modeSettings'), onSelect: () => setSettingsOpen(true) },
        { label: t('digi.mi.personal'), onSelect: () => setSettingsOpen(true) },
      ],
    },
    {
      label: t('digi.menu.help'),
      items: [{ label: t('digi.mi.guide'), onSelect: () => { location.hash = '#/guida'; } }],
    },
  ];

  const modeData = s.mode === 'rtty' ? `${Math.round(s.rtty_baud)} bd, ${Math.round(s.rtty_shift_hz)} Hz` : s.mode === 'psk63' ? '63 bd' : '31 bd';

  return (
    <>
      <MenuBar menus={menus} />

      {/* macro a sinistra (colonne fisse e piu' corte), a fianco il selettore di banda con le frequenze del piano IARU */}
      <div className="digi-topline">
      {views.controlBar ? (
        <ControlBar
          macros={macros}
          extended={views.extended}
          onToggleExtended={() => setView({ extended: !views.extended })}
          onRun={(m) => execute(m.text)}
          onEdit={(name) => setEditing({ name, scope: (s.mode_macros[s.mode] ?? []).some((m) => m.name === name) ? 'mode' : 'all' })}
        />
      ) : null}
      {/* la radio si porta sulla frequenza scelta */}
      <div className="digi-bandbar">
        <FreqPicker
          dialHz={status?.dial_hz ?? 0}
          mode={s.mode.toUpperCase()}
          rows={bandPlanRows(s.mode.toUpperCase() === 'RTTY' ? 'RTTY' : s.mode.toUpperCase() === 'PSK63' ? 'PSK63' : 'PSK31')}
          locked={sending}
          onPick={(hz) => void postFrequency(hz).then(onChanged, onChanged)}
        />
      </div>
      </div>

      <div className={`digi-main${views.log ? '' : ' digi-main--nolog'}`} style={views.log ? { gridTemplateColumns: `minmax(0, 1fr) ${logW}px` } : undefined}>
        <div className="digi-windows" ref={windowsRef}>
          <pre className={`digi-rxwin${views.slashedZero ? ' digi-zero' : ''}`} style={{ flex: `${rxFrac} 1 0` }} ref={rxBox} aria-live="off" aria-label={t('digi.rx.title')}>
            {text === '' ? <span className="digi-empty">{t('digi.rx.empty')}</span> : views.underlineSent ? withSentUnderlined(text) : text}
          </pre>
          <div className="digi-split" role="separator" aria-orientation="horizontal" title={t('digi.split.tip')} onPointerDown={dragSplit} />
          <div className="digi-txwrap" style={{ flex: `${1 - rxFrac} 1 0` }}>
            <textarea
              ref={txBox}
              className={`digi-txwin${views.slashedZero ? ' digi-zero' : ''}${views.capsOnly ? ' digi-caps' : ''}`}
              value={tx}
              aria-label={t('digi.tx.title')}
              placeholder={t('digi.tx.placeholder')}
              onChange={(e) => setTx(views.capsOnly ? e.target.value.toUpperCase() : e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !sending) send(tx);
              }}
            />
            <div className="digi-txbtns">
              {/* il PTT parte da solo con la trasmissione (metodo scelto in Impostazioni > Radio): qui si avvia e si ferma */}
              <button type="button" className="glass sstv-btn sstv-btn--primary" disabled={sending || !snap.can_transmit || tx.trim() === ''} title={t('digi.tx.sendTip')} onClick={() => send(tx)}>
                {t('digi.tx.send')}
              </button>
              <button type="button" className="glass sstv-btn" disabled={!sending} onClick={() => void digiAbort().then(onChanged, onChanged)}>
                {t('digi.tx.stop')}
              </button>
              <small className="muted">{t('digi.tx.sendHelp')}</small>
            </div>
            {sending ? (
              <div className="sstv-progress digi-txprogress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(snap.tx.progress * 100)}>
                <span style={{ width: `${snap.tx.progress * 100}%` }} />
                <em>{t('digi.tx.progress', { done: formatDuration(snap.tx.elapsed_s), total: formatDuration(snap.tx.total_s) })}</em>
              </div>
            ) : null}
            {!snap.can_transmit ? <p className="notice">{t('digi.tx.notReady')}</p> : null}
            {problem ? (
              <p className="notice sstv-bad" role="alert">
                {problem}
              </p>
            ) : null}
          </div>
        </div>
        {views.log ? (
          <DigiLog
            width={logW}
            onWidth={(px) => {
              const w = Math.round(Math.min(Math.max(px, 300), Math.max(320, window.innerWidth - 320)));
              setLogW(w);
              try {
                browserStorage()?.setItem('aurora.digi.logw', String(w));
              } catch {
                // la larghezza vale comunque finche' la pagina e' aperta
              }
            }}
            qso={qso}
            rsts={saved.rsts}
            rstr={saved.rstr}
            modeName={MODE_NAMES[s.mode]}
            dialHz={status?.dial_hz ?? 0}
            audioHz={rxHz}
            onSpot={tuneToSpot}
            onQso={(patch) => setQso({ ...qso, ...patch })}
            onRst={(patch) => setSavedField(patch)}
            onSave={saveQso}
            onClear={() => {
              setQso(EMPTY_QSO);
              setStartAt(null);
              resetRst();
            }}
          />
        ) : null}
      </div>

      {views.waterfall ? (
        <DigiWaterfall height={wfHeight} onHeight={changeWfHeight} minHz={WF_MIN} maxHz={WF_MAX} rxHz={rxHz} txHz={lock ? s.tx_freq_hz : null} dialHz={status?.dial_hz ?? 0} bandwidthHz={s.mode === 'rtty' ? s.rtty_shift_hz : 0} split={lock} hairlines={views.hairlines} transmitting={sending} onRow={onRow} onPick={(hz, ctrl, commit = true) => pickFreq(hz, true, ctrl, commit)} />
      ) : null}

      {views.statusBar ? (
        <DigiStatusBar
          transmitting={sending}
          flashTx={views.flashTx}
          squelchOn={s.squelch_on}
          squelchOpen={snap.rx.squelch_open}
          afc={s.afc_on}
          lock={lock}
          snap={s.snap_on}
          freqHz={rxHz}
          modeData={modeData}
          modeName={MODE_NAMES[s.mode]}
          onToggleTx={toggleTx}
          onToggleSquelch={() => toggleSetting('squelch', 'toggle')}
          onToggleAfc={() => toggleSetting('afc', 'toggle')}
          onToggleLock={() => toggleSetting('lock', 'toggle')}
          onToggleSnap={() => toggleSetting('snap', 'toggle')}
          onFreq={(hz) => pickFreq(hz, false)}
          onModeSettings={() => setSettingsOpen(true)}
          onThreshold={() => setSettingsOpen(true)}
        />
      ) : null}

      <div className="digi-rxctl">
        <button type="button" className="glass sstv-btn" onClick={() => void (snap.rx.listening ? digiRx('stop') : digiRx('start')).then(onChanged, onChanged)}>
          {snap.rx.listening ? t('digi.rx.stop') : t('digi.rx.start')}
        </button>
        <span className="sstv-hint digi-hint">{t('digi.macros.hint')}</span>
      </div>

      {editing !== null && editedMacro ? (
        <MacroEditor key={editing.name} macro={editedMacro} modeName={MODE_NAMES[s.mode]} scope={editing.scope} onSave={saveMacro} onCancel={() => setEditing(null)} />
      ) : null}

      {settingsOpen ? <ModeSettings settings={s} personal={saved} levelDb={snap.rx.level_db} onSave={save} onPersonal={setSavedField} onClose={() => setSettingsOpen(false)} /> : null}

      {scanDialog ? (
        <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.scan.title')}>
          <div className="digi-dialog digi-dialog--small">
            <header>
              <strong>{t('digi.scan.title')}</strong>
            </header>
            <label>
              <span>{t('digi.scan.dwell')}</span>
              <input type="number" min={2} max={600} value={scanProps.dwell} onChange={(e) => { const next = { ...scanProps, dwell: Math.max(2, Number(e.target.value) || 10) }; setScanProps(next); writeJson(SCAN_KEY, next); }} />
            </label>
            {(['once', 'timeMarks', 'freqMarks'] as const).map((k) => (
              <label key={k} className="sstv-check">
                <input type="checkbox" checked={scanProps[k]} onChange={(e) => { const next = { ...scanProps, [k]: e.target.checked }; setScanProps(next); writeJson(SCAN_KEY, next); }} />
                {t(`digi.scan.${k}` as 'digi.scan.once')}
              </label>
            ))}
            <footer>
              <span style={{ flex: 1 }} />
              <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={() => setScanDialog(false)}>
                {t('digi.ms.close')}
              </button>
            </footer>
          </div>
        </div>
      ) : null}
    </>
  );
}
