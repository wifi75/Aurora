import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { bearingDeg } from '../lib/geo';
import { gridToLatLon } from '../lib/maidenhead';
import { addLogQso, rotatorGoto, ApiError, digiAbort, digiCat, digiTextFile, getLogQsos, digiRx, postFrequency, digiTransmit, getDigi, importMacros, putDigi, type ClusterSpot, type DigiMacro, type DigiSettings, type DigiSnapshot } from '../api/client';
import { ControlBar } from '../components/digi/ControlBar';
import { FreqPicker } from '../components/FreqPicker';
import { bandPlanRows } from '../lib/bandplan';
import { RotorCard, useRotorCardVisible } from '../components/rotor/RotorCard';
import { usePublishRotorDx } from '../lib/rotorDx';
import { DigiLog, type Qso } from '../components/digi/DigiLog';
import { RxWordMenu, wordAtPoint, type WordAt, type WordTarget } from '../components/digi/RxWordMenu';
import { isCallsign, useCallLookup } from '../components/digi/callLookup';
import { DigiWaterfall } from '../components/digi/DigiWaterfall';
import { BandSignals } from '../components/digi/BandSignals';
import { getBandMode, getBandSignals, mainCall, onBandSignals } from '../lib/digiBand';
import { MacroEditor } from '../components/digi/MacroEditor';
import { digiWithDefaults as withDefaults, editMacros, type MacroScope } from '../lib/digiMacroEdit';
import { MenuBar, type MenuDef } from '../components/digi/MenuBar';
import { CW_BANDWIDTHS, CW_WIDE_HZ, ModeSettings, RTTY_BAUDS, RTTY_SHIFTS, roundBaud, withCurrent } from '../components/digi/ModeSettings';
import { usePersonal } from '../lib/personal';
import { openSettingsWindow } from '../lib/settingsWindow';
import { readMemories, readScan, SCAN_KEY, writeMemories, type DigiMemory, type ScanProps } from '../lib/digiScan';
import { DigiStatusBar } from '../components/digi/StatusBar';
import { useT } from '../i18n';
import { appendDigiNote, appendDigiTx, clearDigiText, getDigiText, onDigi, onDigiText } from '../lib/digiBus';
import { RowAverager, seekSignal, snapToPair, snapToPeak, type SpectrumRow } from '../lib/digiSpectrum';
import { bandOf } from '../lib/format';
import { effectiveMacros, macroNameForKey, runMacro, type Macro, type MacroAction, type MacroContext } from '../lib/macroLang';
import { useListenWhileOpen } from '../lib/listenWhileOpen';
import { parseSpotSplit } from '../lib/spotSplit';
import { browserStorage } from '../lib/theme';
import { toast } from '../lib/toast';
import { useStation } from '../state/station';
import '../styles/digi.css';
import '../styles/digi-dark.css';
import '../styles/digi-log-corta.css';
import '../styles/digi-band.css';

const VIEWS_KEY = 'aurora.digi.viste';
const MODE_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY', cw: 'CW' };
/** Secondi fra un CQ automatico e il successivo. */
const AUTOCQ_DELAY_S = 8;
/** Il tratto di audio che si ascolta e si mostra (Hz). */
const WF_MIN = 0;
/** Il tratto di audio della cascata: di base 0-5 kHz (il massimo del motore), oppure 0-3,5 kHz dal menu Vista. */
const WF_MAX_WIDE = 5000;
const WF_MAX_NARROW = 3500;
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
  /** Cascata larga (0-5 kHz) o stretta (0-3,5 kHz). */
  wideWaterfall: boolean;
  /** Il pannello «Segnali» di tutta la banda. */
  signals: boolean;
}

const DEFAULT_VIEWS: Views = { controlBar: true, extended: true, log: true, waterfall: true, statusBar: true, underlineSent: true, capsOnly: false, slashedZero: true, flashTx: true, hairlines: false, defaultRst: true, wideWaterfall: true, signals: true };

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
    case 'CW': return 'cw';
    default: return null;
  }
}

/** La larghezza del segnale (Hz): le freccette ai lati della bandierina nella cascata. */
/** La pagina dei modi testuali: menu, barra dei comandi, finestre RX e TX, log di lato, cascata e barra di stato. */
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
  // il menu del tasto destro su una parola della finestra di ricezione
  const [wordMenu, setWordMenu] = useState<WordAt | null>(null);
  // il nominativo preso dalla ricezione va nel QSO in corso e si cercano nome, QTH, locatore e paese
  const takeCall = useCallLookup(setQso);
  const [rotorShown, setRotorShown] = useRotorCardVisible('digi');
  // il DX del QSO anche per il rotore staccato in una finestra a parte
  usePublishRotorDx(qso.call, qso.wwloc);
  // la larghezza della scheda delle macro: si regola dal bordo destro (doppio clic: quella di partenza)
  const macroBox = useRef<HTMLDivElement>(null);
  const [macroW, setMacroW] = useState<number | null>(() => {
    const v = Number(browserStorage()?.getItem('aurora.digi.macrow'));
    return Number.isFinite(v) && v >= 360 ? v : null;
  });
  // i dati che non cambiano da un QSO all'altro (nome, QTH, RST): gli stessi di Impostazioni › Stazione › Dati personali
  const [saved, setSavedField] = usePersonal();
  const [startAt, setStartAt] = useState<Date | null>(null);
  const [views, setViews] = useState<Views>(() => ({ ...DEFAULT_VIEWS, ...readJson<Partial<Views>>(VIEWS_KEY, {}) }));
  const [scanProps, setScanProps] = useState<ScanProps>(() => readScan(browserStorage()));
  // la scansione si regola anche dalle Impostazioni (finestra a parte): qui si segue la modifica
  useEffect(() => {
    const onStorage = (e: StorageEvent) => {
      if (e.key === null || e.key === SCAN_KEY) setScanProps(readScan(browserStorage()));
    };
    window.addEventListener('storage', onStorage);
    return () => window.removeEventListener('storage', onStorage);
  }, []);
  const [problem, setProblem] = useState('');
  const [editing, setEditing] = useState<{ name: string; scope: MacroScope } | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [autoCq, setAutoCq] = useState(false);
  // doppio Esc: l'Auto CQ resta acceso ma in pausa; riparte quando si ripreme (macro <AUTOCQ>)
  const [autoCqPaused, setAutoCqPaused] = useState(false);
  const lastEsc = useRef(0);
  const [scanning, setScanning] = useState(false);
  const s = snap.settings;
  const sending = snap.tx.state === 'sending';
  const lock = !s.tx_follow_rx;
  const rxHz = snap.rx.freq_hz > 0 ? snap.rx.freq_hz : s.rx_freq_hz;

  // tutta la banda: i segnali del modo in uso (CW), con il nominativo sopra la traccia nella cascata
  const bandAll = useSyncExternalStore(onBandSignals, getBandSignals);
  const bandOn = s.mode === 'cw' && s.band_cw !== false;
  const band = useMemo(() => (bandOn && getBandMode() === s.mode ? bandAll : []), [bandAll, bandOn, s.mode]);
  const bandLabels = useMemo(
    () =>
      band.flatMap((sig) => {
        const c = mainCall(sig);
        return c ? [{ hz: sig.freq_hz, text: c.call, cq: sig.cq, sure: c.verified }] : [];
      }),
    [band],
  );

  const macros = useMemo(() => effectiveMacros(s.macros.map(toMacro), s.mode_macros[s.mode]?.map(toMacro), s.merge_macros[s.mode] !== false), [s.macros, s.mode_macros, s.merge_macros, s.mode]);

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


  // il divisore fra ricezione e trasmissione si trascina: la parte alta della colonna (0,2-0,8), ricordata
  // l'altezza della cascata si regola dalla barra sul suo bordo alto (da 50 a 400 px), ricordata
  const [wfHeight, setWfHeight] = useState<number>(() => {
    const n = Number(browserStorage()?.getItem('aurora.digi.wfh2') ?? '');
    return Number.isFinite(n) && n >= 50 && n <= 400 ? n : 80;
  });
  const changeWfHeight = (px: number) => {
    const h = Math.round(Math.min(400, Math.max(50, px)));
    setWfHeight(h);
    try {
      browserStorage()?.setItem('aurora.digi.wfh2', String(h));
    } catch {
      // l'altezza vale comunque finche' la pagina e' aperta
    }
  };
  const windowsRef = useRef<HTMLDivElement>(null);
  const [rxFrac, setRxFrac] = useState<number>(() => {
    const n = Number(browserStorage()?.getItem('aurora.digi.rxfrac') ?? '');
    return Number.isFinite(n) && n >= 0.1 && n <= 0.9 ? n : 0.55;
  });
  const dragSplit = (e: React.PointerEvent) => {
    e.preventDefault();
    const box = windowsRef.current?.getBoundingClientRect();
    if (!box) return;
    const move = (m: PointerEvent) => setRxFrac(Math.min(0.9, Math.max(0.1, (m.clientY - box.top) / box.height)));
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

  // entrando nella pagina l'ascolto parte da solo (si ferma e riparte dal menu Opzioni); lasciandola (per esempio tornando all'FT8)
  // si spegne, cosi' il motore non consuma in secondo piano, salvo «Ascolto acceso all'avvio di Aurora»
  useListenWhileOpen(digiRx, () => live.current.snap.settings.auto_receive, onChanged);

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
    // in RTTY la frequenza e' il centro fra i due toni: Snap cerca la coppia (un tono solo andrebbe sul mark o sullo space)
    const rttyShift = l.snap.settings.mode === 'rtty' ? l.snap.settings.rtty_shift_hz : 0;
    const snapped =
      snapIt && l.snap.settings.snap_on && avg.current.ready && lastRow.current
        ? rttyShift > 0
          ? snapToPair(avg.current.values(), lastRow.current, hz, rttyShift)
          : snapToPeak(avg.current.values(), lastRow.current, hz)
        : hz;
    const f = Math.max(200, Math.min(3500, snapped));
    // in split (Lock) la ricezione resta ferma e il clic sposta la trasmissione; con Ctrl si sposta invece la ricezione
    if (viaClick && !l.snap.settings.tx_follow_rx && !ctrl) {
      save({ tx_freq_hz: f });
      return;
    }
    // con Lock la trasmissione resta dov'e': si sposta solo l'ascolto
    save({ rx_freq_hz: f, ...(l.snap.settings.tx_follow_rx ? { tx_freq_hz: f } : {}) });
  };

  // clic su uno spot del DX Cluster: la radio si porta in modo che il segnale cada sul tono di ricezione; con «UP N» nel commento (letto con
  // le stesse regole del cluster) la trasmissione si sposta di N kHz (split dentro l'audio, fino a 3,5 kHz); il nominativo va nel QSO.
  // `splitDx`: lo Split DX vero, la radio va subito dopo in split (VFO B spostato): il tono di trasmissione torna quello di ricezione e Lock si
  // spegne (la trasmissione segue la ricezione), se no lo split nell'audio si sommerebbe a quello della radio. Una sola scrittura delle
  // impostazioni: due di fila partirebbero entrambe dalle impostazioni di prima e la seconda cancellerebbe la prima (per esempio il modo).
  const tuneToSpot = async (spot: ClusterSpot, splitDx: boolean) => {
    const l = live.current;
    const want = (spot.mode ?? '').toUpperCase();
    const cur = l.snap.settings.mode;
    const patch: Partial<DigiSettings> = {};
    if (want === 'RTTY' && cur !== 'rtty') patch.mode = 'rtty';
    else if (want === 'CW' && cur !== 'cw') patch.mode = 'cw';
    else if (want.startsWith('PSK') && (cur === 'rtty' || cur === 'cw')) patch.mode = 'psk31';
    const signalHz = Math.round(spot.freq_khz * 1000);
    const rxTone = Math.round(l.rxHz);
    const dial = signalHz - rxTone;
    useStation.getState().patchStatus({ dial_hz: dial });
    await postFrequency(dial);
    setQso({ ...l.qso, call: spot.dx_call });
    let note: string | null = null;
    if (splitDx) {
      // il segnale trasmesso cade sul VFO B piu' lo stesso tono: dove ascolta la DXpedition
      if (Math.round(l.snap.settings.tx_freq_hz) !== rxTone) patch.tx_freq_hz = rxTone;
      if (!l.snap.settings.tx_follow_rx) patch.tx_follow_rx = true;
    } else {
      const split = parseSpotSplit(spot.comment, spot.freq_khz, spot.mode);
      // solo «UP N» (anche «N UP», «UP 1-3»): sotto o con la QSX il tono andrebbe sotto la ricezione, e gli avvisi parlano di «up»
      if (split?.kind === 'up') {
        const khz = String(split.offsetKhz);
        const txAudio = Math.round(rxTone + split.offsetKhz * 1000);
        if (txAudio <= 3500) {
          patch.tx_follow_rx = false;
          patch.tx_freq_hz = txAudio;
          note = t('digi.spot.splitUp', { khz });
        } else {
          note = t('digi.spot.splitFar', { khz });
        }
      }
    }
    if (Object.keys(patch).length > 0) save(patch);
    if (note) toast(note, 'info');
  };

  // Seek e Scan trovano un picco: in RTTY e' uno dei due toni, e la sintonia va al centro della coppia
  const centreOfPair = (tone: number, row: SpectrumRow) => {
    const c = live.current.snap.settings;
    return c.mode === 'rtty' ? snapToPair(avg.current.values(), row, tone, c.rtty_shift_hz) : tone;
  };

  const seek = (direction: 'left' | 'right', quiet = false): number | null => {
    const row = lastRow.current;
    if (!row || !avg.current.ready) {
      if (!quiet) toast(t('digi.run.noSpectrum'), 'info');
      return null;
    }
    const tone = seekSignal(avg.current.values(), row, live.current.rxHz, direction, SEEK_LIMITS);
    if (tone === null) {
      if (!quiet) toast(t('digi.run.seekNone'), 'info');
      return null;
    }
    const found = centreOfPair(tone, row);
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
      let tone = seekSignal(avg.current.values(), row, l.rxHz, 'right', SEEK_LIMITS);
      if (tone === null) tone = seekSignal(avg.current.values(), row, SEEK_LIMITS.min - 40, 'right', SEEK_LIMITS);
      if (tone === null) return;
      const found = centreOfPair(tone, row);
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
  const memory = (write: boolean, slot: number) => {
    const all = readMemories(browserStorage());
    if (write) {
      const c = live.current.snap.settings;
      all[String(slot)] = { mode: c.mode, rx_freq_hz: c.rx_freq_hz, tx_freq_hz: c.tx_freq_hz, tx_follow_rx: c.tx_follow_rx, rtty_shift_hz: c.rtty_shift_hz, rtty_baud: c.rtty_baud, rtty_reverse: c.rtty_reverse, afc_on: c.afc_on, snap_on: c.snap_on, squelch_on: c.squelch_on, psk_squelch: c.psk_squelch, cw_squelch_snr_db: c.cw_squelch_snr_db, rtty_squelch_snr_db: c.rtty_squelch_snr_db };
      writeMemories(browserStorage(), all);
      toast(t('digi.run.memSaved', { n: slot }), 'ok');
    } else if (all[String(slot)]) {
      save(all[String(slot)] as DigiMemory);
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
      // la frequenza vera del segnale (radio + tono di ricezione, in USB), non solo la radio
      freq_hz: dial > 0 ? dial + Math.round(l.rxHz) : 0,
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

  /** Una macro. Se chiede file di testo (`<TEXTFILE>`) o l'ultimo QSO (`<ANYQSOBEFORE...>`) prima si leggono, poi si esegue. */
  const execute = (script: string) => {
    const files = [...script.matchAll(/<TEXTFILE:([^<>]*)>/g)].map((m) => (m[1] ?? '').trim()).filter((n) => n !== '');
    const wantsLast = /<ANYQSOBEFORE(?:DATE|BAND|MODE)>/.test(script);
    if (files.length === 0 && !wantsLast) {
      run(script, {});
      return;
    }
    const call = live.current.qso.call.trim().toUpperCase();
    void Promise.all([
      Promise.all(files.map((n) => digiTextFile(n).then((text) => [n, text] as const, () => [n, ''] as const))),
      wantsLast && call !== '' ? getLogQsos().then((r) => r.qsos, () => []) : Promise.resolve([]),
    ]).then(([read, qsos]) => {
      const mine = qsos.filter((q) => q.call.toUpperCase() === call).sort((a, b) => `${b.date}${b.time}`.localeCompare(`${a.date}${a.time}`))[0];
      run(script, { files: Object.fromEntries(read), lastQso: mine ? { date: mine.date, band: mine.band, mode: mine.mode } : null });
    });
  };

  /** I comandi di programma nell'ordine, il testo in trasmissione o nella casella, poi il salvataggio e la pulizia. */
  const run = (script: string, extra: Pick<MacroContext, 'files' | 'lastQso'>) => {
    const l = live.current;
    const result = runMacro(script, { ...context(), ...extra });
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
        case 'fq': {
          // la frequenza del segnale: la radio si porta in modo che cada sul tono di ricezione
          const dial = Math.round(a.khz * 1000) - Math.round(l.rxHz);
          useStation.getState().patchStatus({ dial_hz: dial });
          void postFrequency(dial).then(onChanged, onChanged);
          break;
        }
        case 'rotor': {
          const goto = (az: number) => void rotatorGoto(az).catch(() => toast(t('digi.run.rotorFail'), 'error'));
          if (typeof a.az === 'number') goto(a.az);
          else {
            const from = gridToLatLon(l.status?.de_grid ?? '');
            const to = gridToLatLon(l.qso.wwloc);
            if (!from || !to) toast(t('digi.run.rotorNoGrid'), 'info');
            else goto(a.az === 'LP' ? (bearingDeg(from, to) + 180) % 360 : bearingDeg(from, to));
          }
          break;
        }
        case 'editMacro': setEditing({ name: a.name, scope: (s.mode_macros[s.mode] ?? []).some((m) => m.name === a.name) ? 'mode' : 'all' }); break;
        case 'alarm': toast(a.text, 'info'); break;
        case 'wpm': {
          const next = Math.round(Math.min(60, Math.max(5, a.relative ? l.snap.settings.cw_wpm + a.wpm : a.wpm)));
          if (l.snap.settings.mode !== 'cw') save({ mode: 'cw', cw_wpm: next });
          else save({ cw_wpm: next });
          break;
        }
        case 'audioFq': pickFreq(Math.max(200, Math.min(3500, a.relative ? l.rxHz + a.hz : a.hz)), false); break;
        case 'cursor': if (a.target === 'TX') txBox.current?.focus(); break;
        case 'beep': beep(); break;
        case 'autoCq':
          if (autoCqPaused && autoCq) {
            setAutoCqPaused(false);
            toast(t('digi.run.autoCqOn', { s: AUTOCQ_DELAY_S }), 'info');
          } else {
            setAutoCqPaused(false);
            setAutoCq((on) => !on);
          }
          break;
        case 'seek': seek(a.direction); break;
        case 'toggle': toggleSetting(a.what, a.value); break;
        case 'scan': setScanning(a.run); break;
        case 'memory': memory(a.write, a.slot); break;
        case 'modeSettings':
        case 'threshold': setSettingsOpen(true); break;
        case 'txe':
        case 'rxe': break;
        case 'cat':
          void digiCat(a.cmd).catch((e: unknown) => toast(t('digi.run.catFail', { why: e instanceof ApiError ? (e.message ?? e.code) : String(e) }), 'error'));
          break;
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
      if (editing !== null || settingsOpen) return;
      if (e.key === 'Escape') {
        // Esc una volta ferma la trasmissione; due volte di seguito (entro mezzo secondo) mette anche in pausa il CQ automatico, che resta
        // acceso e riparte quando lo si ripreme
        const now = Date.now();
        const twice = now - lastEsc.current < 600;
        lastEsc.current = twice ? 0 : now;
        if (twice && autoCq && !autoCqPaused) {
          setAutoCqPaused(true);
          toast(t('digi.run.autoCqPaused'), 'info');
        }
        if (live.current.snap.tx.state === 'sending') void digiAbort().then(onChanged, onChanged);
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
    if (!autoCq || autoCqPaused || sending || !snap.can_transmit) return;
    const m = live.current.macros.get('AutoCqText') ?? live.current.macros.get('F2');
    if (!m || m.text.trim() === '') {
      toast(t('digi.run.autoCqNoMacro'), 'info');
      setAutoCq(false);
      return;
    }
    const timer = window.setTimeout(() => execute(m.text), AUTOCQ_DELAY_S * 1000);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq, autoCqPaused, sending, snap.can_transmit, snap.tx.state]);

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
    save(editMacros(s, editing?.name ?? m.name, m, where));
    setEditing(null);
  };

  // le macro di fabbrica di Aurora (l'importazione da altri programmi e' stata tolta dal menu, scelta del proprietario 2026-11-11)
  const resetMacros = () => {
    importMacros('defaults').then(
      (r) => {
        client.setQueryData(['digi'], r.snapshot);
        onChanged();
        toast(t('digi.mi.macrosRestored'), 'ok');
      },
      (e: unknown) => {
        const code = e instanceof ApiError ? e.code : '';
        toast(code === 'local_only' ? t('digi.mi.importLocal') : t('digi.mi.importErr', { why: e instanceof ApiError ? (e.message ?? code) : String(e) }), 'error');
      },
    );
  };

  // la parola scelta col tasto destro va nel campo del log del QSO in corso (il rapporto e' quello ricevuto) o negli appunti
  const pickWord = (target: WordTarget, word: string) => {
    if (target === 'copy') {
      // il testo selezionato nella finestra di ricezione, altrimenti la parola cliccata
      const sel = window.getSelection();
      const picked = sel && rxBox.current && sel.anchorNode && rxBox.current.contains(sel.anchorNode) ? sel.toString() : '';
      const text = picked !== '' ? picked : word;
      if (text !== '') void navigator.clipboard?.writeText(text).catch(() => undefined);
    } else if (target === 'clear') clearDigiText();
    else if (target === 'rst') setSavedField({ rstr: word.toUpperCase() });
    else if (target === 'call') takeCall(word);
    else if (target === 'grid') setQso((q) => ({ ...q, wwloc: word.toUpperCase() }));
    else setQso((q) => ({ ...q, [target]: word }));
  };

  const copyRx = () => {
    void navigator.clipboard?.writeText(getDigiText()).catch(() => undefined);
  };

  // ---- i menu ----
  const slots = Array.from({ length: 20 }, (_, i) => i + 1);
  const menus: MenuDef[] = [
    {
      label: t('digi.menu.file'),
      items: [
        { label: t('digi.mi.macrosMerge'), checked: s.merge_macros[s.mode] !== false, onSelect: () => save({ merge_macros: { ...s.merge_macros, [s.mode]: s.merge_macros[s.mode] === false } }) },
        'sep',
        { label: t('digi.mi.macrosReset'), onSelect: resetMacros },
      ],
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
        // RTTY: spostamento e velocita' Baudot scelti dal menu (e nelle Impostazioni del modo)
        ...(s.mode === 'rtty'
          ? [
              'sep' as const,
              { label: `${t('digi.rtty.shift')}: ${s.rtty_shift_hz}`, items: withCurrent(RTTY_SHIFTS, s.rtty_shift_hz).map((v) => ({ label: `${v} Hz`, checked: v === s.rtty_shift_hz, onSelect: () => save({ rtty_shift_hz: v }) })) },
              { label: `${t('digi.rtty.baud')}: ${roundBaud(s.rtty_baud)}`, items: withCurrent(RTTY_BAUDS, roundBaud(s.rtty_baud)).map((v) => ({ label: `${v} baud`, checked: v === roundBaud(s.rtty_baud), onSelect: () => save({ rtty_baud: v }) })) },
              { label: t('digi.rtty.reverse'), checked: s.rtty_reverse, onSelect: () => save({ rtty_reverse: !s.rtty_reverse }) },
              { label: t('digi.rtty.uos'), checked: s.rtty_uos, onSelect: () => save({ rtty_uos: !s.rtty_uos }) },
            ]
          : []),
        // CW: banda del filtro scelta al volo (anche larga, per la panoramica)
        ...(s.mode === 'cw'
          ? ['sep' as const, { label: `${t('digi.cw.bandwidth')}: ${s.cw_bandwidth_hz} Hz`, items: withCurrent(CW_BANDWIDTHS, s.cw_bandwidth_hz).map((v) => ({ label: v >= CW_WIDE_HZ ? `${v} Hz, ${t('digi.cw.wide')}` : `${v} Hz`, checked: v === s.cw_bandwidth_hz, onSelect: () => save({ cw_bandwidth_hz: v }) })) }]
          : []),
        'sep' as const,
        ...['QPSK31', 'FSK31', 'MFSK', 'Olivia', 'Contestia', 'MT63', 'Throb', 'Hellschreiber', 'Packet', 'Pactor', 'Amtor'].map((n) => ({ label: `${n} ${t('digi.mi.soon')}`, disabled: true })),
        'sep' as const,
        { label: t('digi.mi.modeSettings'), onSelect: () => openSettingsWindow('digi') },
        { label: t('digi.mi.threshold'), onSelect: () => setSettingsOpen(true) },
      ],
    },
    {
      label: t('digi.menu.options'),
      items: [
        { label: t('digi.mi.listen'), checked: snap.rx.listening, onSelect: () => void (snap.rx.listening ? digiRx('stop') : digiRx('start')).then(onChanged, onChanged) },
        'sep' as const,
        { label: t('digi.mi.afc'), checked: s.afc_on, onSelect: () => toggleSetting('afc', 'toggle') },
        { label: t('digi.mi.lock'), checked: lock, onSelect: () => toggleSetting('lock', 'toggle') },
        { label: t('digi.mi.snap'), checked: s.snap_on, onSelect: () => toggleSetting('snap', 'toggle') },
        { label: t('digi.mi.squelch'), checked: s.squelch_on, onSelect: () => toggleSetting('squelch', 'toggle') },
        { label: t('digi.mi.threshold'), onSelect: () => setSettingsOpen(true) },
        // tutta la banda (CW): acceso di base, e gli spot locali verso il pannello del DX Cluster di Aurora
        ...(s.mode === 'cw'
          ? [
              'sep' as const,
              { label: t('digi.band.mi'), checked: s.band_cw !== false, onSelect: () => save({ band_cw: s.band_cw === false }) },
              { label: t('digi.band.spots'), checked: s.band_spots !== false, onSelect: () => save({ band_spots: s.band_spots === false }) },
            ]
          : []),
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
            { label: t('digi.mi.scanProps'), onSelect: () => openSettingsWindow('digiscan') },
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
        { label: t('digi.mi.rotor'), checked: rotorShown, onSelect: () => setRotorShown(!rotorShown) },
        { label: t('digi.mi.waterfall'), checked: views.waterfall, onSelect: () => setView({ waterfall: !views.waterfall }) },
        { label: t('digi.band.view'), checked: views.signals, onSelect: () => setView({ signals: !views.signals }) },
        { label: t('digi.mi.statusBar'), checked: views.statusBar, onSelect: () => setView({ statusBar: !views.statusBar }) },
        'sep' as const,
        { label: t('digi.mi.underlineSent'), checked: views.underlineSent, onSelect: () => setView({ underlineSent: !views.underlineSent }) },
        { label: t('digi.mi.capsOnly'), checked: views.capsOnly, onSelect: () => setView({ capsOnly: !views.capsOnly }) },
        { label: t('digi.mi.slashedZero'), checked: views.slashedZero, onSelect: () => setView({ slashedZero: !views.slashedZero }) },
        { label: t('digi.mi.flashTx'), checked: views.flashTx, onSelect: () => setView({ flashTx: !views.flashTx }) },
        { label: t('digi.mi.hairlines'), checked: views.hairlines, onSelect: () => setView({ hairlines: !views.hairlines }) },
        { label: t('digi.mi.wideWaterfall'), checked: views.wideWaterfall, onSelect: () => setView({ wideWaterfall: !views.wideWaterfall }) },
        { label: t('digi.mi.defaultRst'), checked: views.defaultRst, onSelect: () => setView({ defaultRst: !views.defaultRst }) },
      ],
    },
    {
      label: t('digi.menu.configure'),
      items: [
        // le macro, le impostazioni del modo e i dati personali stanno nelle Impostazioni (Modi › PSK · RTTY · CW e Stazione › Dati personali);
        // il clic destro su un tasto macro e la soglia dello squelch restano scorciatoie della pagina
        { label: t('digi.mi.macrosMode'), onSelect: () => openSettingsWindow('digimacros') },
        { label: t('digi.mi.macrosDefault'), onSelect: () => openSettingsWindow('digimacros') },
        'sep',
        { label: t('digi.mi.modeSettings'), onSelect: () => openSettingsWindow('digi') },
        { label: t('digi.mi.personal'), onSelect: () => openSettingsWindow('personal') },
      ],
    },
    {
      label: t('digi.menu.help'),
      items: [{ label: t('digi.mi.guide'), onSelect: () => { location.hash = '#/guida'; } }],
    },
  ];

  // le misure del segnale per la barra: S/N in tutti i modi, IMD nel PSK (durante le inversioni), velocita' capita nel CW
  const measures = [
    snap.rx.snr_db != null ? `S/N ${Math.round(snap.rx.snr_db)} dB` : 'S/N —',
    s.mode === 'psk31' || s.mode === 'psk63' ? (snap.rx.imd_db != null ? `IMD ${Math.round(snap.rx.imd_db)} dB` : 'IMD —') : null,
    s.mode === 'cw' && snap.rx.wpm != null ? `${Math.round(snap.rx.wpm)} WPM` : null,
  ]
    .filter((v) => v !== null)
    .join(' · ');
  const modeData = s.mode === 'rtty' ? `${Math.round(s.rtty_baud)} bd, ${Math.round(s.rtty_shift_hz)} Hz` : s.mode === 'cw' ? `${Math.round(s.cw_wpm)} WPM` : s.mode === 'psk63' ? '63 bd' : '31 bd';

  return (
    <>
      <MenuBar menus={menus} />

      {/* macro a sinistra (colonne fisse e piu' corte), a fianco il selettore di banda con le frequenze del piano IARU */}
      <div className="digi-topline">
      {views.controlBar ? (
        <div className={`digi-macrowrap${macroW ? ' digi-macrowrap--sized' : ''}`} style={macroW ? { width: macroW } : undefined} ref={macroBox}>
          <ControlBar
            macros={macros}
            extended={views.extended}
            onToggleExtended={() => setView({ extended: !views.extended })}
            onRun={(m) => execute(m.text)}
            autoCq={autoCq ? (autoCqPaused ? 'paused' : 'on') : 'off'}
            onEdit={(name) => setEditing({ name, scope: (s.mode_macros[s.mode] ?? []).some((m) => m.name === name) ? 'mode' : 'all' })}
          />
          <div
            className="digi-macrogrip"
            role="separator"
            aria-orientation="vertical"
            title={t('digi.bar.resize')}
            onDoubleClick={() => {
              setMacroW(null);
              try {
                browserStorage()?.removeItem('aurora.digi.macrow');
              } catch {
                // niente da fare
              }
            }}
            onPointerDown={(e) => {
              const el = macroBox.current;
              if (!el) return;
              e.preventDefault();
              const startX = e.clientX;
              const startW = el.offsetWidth;
              const scale = el.offsetWidth > 0 ? el.getBoundingClientRect().width / el.offsetWidth : 1;
              let last = startW;
              const move = (m: PointerEvent) => {
                last = Math.round(Math.min(Math.max(360, startW + (m.clientX - startX) / scale), Math.max(360, window.innerWidth / scale - 40)));
                setMacroW(last);
              };
              const up = () => {
                window.removeEventListener('pointermove', move);
                window.removeEventListener('pointerup', up);
                try {
                  browserStorage()?.setItem('aurora.digi.macrow', String(last));
                } catch {
                  // la misura vale comunque finche' la pagina e' aperta
                }
              };
              window.addEventListener('pointermove', move);
              window.addEventListener('pointerup', up);
            }}
          />
        </div>
      ) : null}
      {/* la radio si porta sulla frequenza scelta */}
      <div className="digi-bandbar">
        <FreqPicker
          dialHz={status?.dial_hz ?? 0}
          mode={s.mode.toUpperCase()}
          rows={bandPlanRows(s.mode === 'rtty' ? 'RTTY' : s.mode === 'cw' ? 'CW' : s.mode === 'psk63' ? 'PSK63' : 'PSK31')}
          locked={sending}
          onPick={(hz) => void postFrequency(hz).then(onChanged, onChanged)}
        />
      </div>
      {sending ? (
        <div className={`digi-txflag${views.flashTx ? ' digi-txflag--flash' : ''}`} role="status" aria-label="TX">
          TX
        </div>
      ) : null}
      </div>

      <div className={`digi-main${views.log ? '' : ' digi-main--nolog'}`} style={views.log ? { gridTemplateColumns: `minmax(0, 1fr) ${logW}px` } : undefined}>
        <div className="digi-leftcol">
        <div className="digi-windows" ref={windowsRef}>
          <pre className={`digi-rxwin${views.slashedZero ? ' digi-zero' : ''}`} style={{ flex: `${rxFrac} 1 0` }} ref={rxBox} aria-live="off" aria-label={t('digi.rx.title')}
            onContextMenu={(e) => {
              // il nostro menu al posto di quello del sistema: sulle parole anche le voci del log, fuori solo Copia e Cancella tutto
              e.preventDefault();
              const word = rxBox.current ? wordAtPoint(e.clientX, e.clientY, rxBox.current) : null;
              setWordMenu({ word, x: e.clientX, y: e.clientY });
            }}
            onDoubleClick={(e) => {
              // doppio clic su un nominativo: nel QSO in corso (non salvato) con la ricerca
              const word = rxBox.current ? wordAtPoint(e.clientX, e.clientY, rxBox.current) : null;
              if (word && isCallsign(word)) takeCall(word);
            }}>
            {text === '' ? null : views.underlineSent ? withSentUnderlined(text) : text}
          </pre>
          <div className="digi-split" role="separator" aria-orientation="horizontal" title={t('digi.split.tip')} onPointerDown={dragSplit} />
          <div className="digi-txwrap" style={{ flex: `${1 - rxFrac} 1 0` }}>
            <textarea
              ref={txBox}
              className={`digi-txwin${views.slashedZero ? ' digi-zero' : ''}${views.capsOnly ? ' digi-caps' : ''}`}
              value={tx}
              aria-label={t('digi.tx.title')}
              onChange={(e) => setTx(views.capsOnly ? e.target.value.toUpperCase() : e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !sending) send(tx);
              }}
            />
            {!snap.can_transmit ? <p className="notice">{t('digi.tx.notReady')}</p> : null}
            {problem ? (
              <p className="notice sstv-bad" role="alert">
                {problem}
              </p>
            ) : null}
          </div>
        </div>
        {bandOn && views.signals ? (
          <BandSignals signals={band} showWpm={s.mode === 'cw'} dialHz={status?.dial_hz ?? 0} rxHz={rxHz} emptyText={t('digi.band.empty')} onTune={(hz) => pickFreq(hz, false)} onCall={takeCall} />
        ) : null}
        {views.waterfall ? (
          <DigiWaterfall height={wfHeight} onHeight={changeWfHeight} minHz={WF_MIN} maxHz={views.wideWaterfall ? WF_MAX_WIDE : WF_MAX_NARROW} rxHz={rxHz} txHz={lock ? s.tx_freq_hz : null} dialHz={status?.dial_hz ?? 0} bandwidthHz={s.mode === 'rtty' ? s.rtty_shift_hz : 0} split={lock} hairlines={views.hairlines} transmitting={sending} onRow={onRow} onPick={(hz, ctrl, commit = true) => pickFreq(hz, true, ctrl, commit)} labels={bandLabels} />
        ) : null}
        </div>
        <RotorCard call={qso.call} grid={qso.wwloc} place="digi" />
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
          measures={snap.rx.listening ? measures : ''}
          rtty={
            s.mode === 'rtty'
              ? {
                  shiftHz: s.rtty_shift_hz,
                  baud: s.rtty_baud,
                  reverse: s.rtty_reverse,
                  radioLsb: snap.rx.radio_lsb === true,
                  onShift: (hz) => save({ rtty_shift_hz: hz }),
                  onBaud: (baud) => save({ rtty_baud: baud }),
                  onReverse: () => save({ rtty_reverse: !s.rtty_reverse }),
                }
              : undefined
          }
          cw={s.mode === 'cw' ? { bandwidthHz: s.cw_bandwidth_hz, onBandwidth: (hz) => save({ cw_bandwidth_hz: hz }) } : undefined}
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

      {editing !== null && editedMacro ? (
        <MacroEditor key={editing.name} macro={editedMacro} modeName={MODE_NAMES[s.mode]} scope={editing.scope} onSave={saveMacro} onCancel={() => setEditing(null)} />
      ) : null}

      {wordMenu ? <RxWordMenu at={wordMenu} onPick={pickWord} onClose={() => setWordMenu(null)} /> : null}

      {settingsOpen ? <ModeSettings settings={s} personal={saved} levelDb={snap.rx.level_db} snrDb={snap.rx.snr_db ?? null} quality={snap.rx.quality ?? null} onSave={save} onPersonal={setSavedField} onClose={() => setSettingsOpen(false)} /> : null}

    </>
  );
}
