import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react';
import { bearingDeg } from '../lib/geo';
import { gridToLatLon } from '../lib/maidenhead';
import { FldigiImportDialog } from '../components/FldigiImportDialog';
import { applyImport, buildImport, parseFldigiFile, type ImportReport } from '../lib/fldigiMacros';
import { addLogQso, rotatorGoto, ApiError, digiAbort, digiMacroPlay, digiTextFile, getLogQsos, digiRx, digiRxPage, postFrequency, digiTransmit, digiTransmitRestart, digiTxClose, digiTxOpen, getDigi, getExtraRx, putExtraRx, type ExtraRx, getDigiNotches, importMacros, putDigi, putDigiNotches, type ClusterSpot, type DigiNotch, type DigiMacro, type DigiSettings, type DigiSnapshot, type Log4omQso, type OliviaSettings } from '../api/client';
import { ControlBar } from '../components/digi/ControlBar';
import { FreqPicker } from '../components/FreqPicker';
import { bandPlanRows } from '../lib/bandplan';
import { effectiveTxHz, isOliviaMode, OLIVIA_BANDWIDTHS, OLIVIA_TONES, oliviaLabel, oliviaModePatch, oliviaOf, oliviaPatch, oliviaProblem } from '../lib/olivia';
import { useBandFilter, visibleBands } from '../lib/bandFilter';
import { RotorCard, useRotorCardVisible } from '../components/rotor/RotorCard';
import { usePublishRotorDx } from '../lib/rotorDx';
import { DigiLog, type LogMarks, type Qso } from '../components/digi/DigiLog';
import { applyAvailability, MODI_IN_ARRIVO } from '../lib/menuModi';
import { SsbContestView } from '../components/digi/SsbContestView';
import { ContestWorkspace } from '../components/digi/ContestWorkspace';
import { composeReceived, composeSent, readSection } from '../lib/contestSection';
import { commentWithExchange, contestNextSerial, exchangeSent, formatSerial, readSerial, writeSerial } from '../lib/contestSerial';
import { writeIgnoreBefore, writePeriod } from '../lib/logWindow';
import { useContestLog } from '../lib/useContestLog';
import { getActiveTextContest, getTextContestCheck, type TextContest } from '../api/contestTesto';
import { bandeAmmesse, haSezionePropria, modiDichiarati } from '../lib/contestVoci';
import { categoryValid, contestWindow, contestWindowStatus, grabExchange, startSerial, withContestMacros } from '../lib/textContest';
import { ContestEditDialog, ContestListDialog, useSelectTextContest } from '../components/digi/ContestListDialog';
import { ContestStatsDialog, TEXT_CONTEST_SCORE_KEY, useExportTextCabrillo, useTextContestScore } from '../components/digi/ContestStatsDialog';
import { QtcDialog, type QtcDir } from '../components/digi/QtcDialog';
import { RxWordMenu, wordAtPoint, type WordAt, type WordTarget } from '../components/digi/RxWordMenu';
import { filledFrom, isCallsign, useCallLookup } from '../components/digi/callLookup';
import { knownCallsign } from '../components/CallsignTip';
import { useCallsignAutofill } from '../lib/useCallsignAutofill';
import { grabCall } from '../lib/grabCall';
import { DigiWaterfall } from '../components/digi/DigiWaterfall';
import { BandSignals } from '../components/digi/BandSignals';
import { RadarList } from '../components/digi/RadarList';
import { RbnList } from '../components/digi/RbnList';
import { rbnLabels, useRbnSpots } from '../lib/rbnSpots';
import { ExtraRxWindow } from '../components/digi/ExtraRxWindow';
import { PopupMenu } from '../components/PopupMenu';
import { addNotch } from '../lib/digiNotches';
import { clearExtraText, getExtraText, onExtraText } from '../lib/digiBus';
import { takePendingSpot, textModeOfSpot } from '../lib/spotMode';
import { getBandMode, getBandSignals, onBandSignals, cqCallOf, radarColor, radarStations, waterfallLabels } from '../lib/digiBand';
import { MacroEditor } from '../components/digi/MacroEditor';
import { MacroGuide } from '../components/digi/MacroGuide';
import { digiWithDefaults as withDefaults, editMacros, swapMacros, type MacroScope } from '../lib/digiMacroEdit';
import { MenuBar, type MenuDef } from '../components/digi/MenuBar';
import { CW_BANDWIDTHS, CW_WIDE_HZ, ModeSettings, RTTY_BAUDS, RTTY_SHIFTS, roundBaud, withCurrent } from '../components/digi/ModeSettings';
import { usePersonal } from '../lib/personal';
import { openSettingsWindow } from '../lib/settingsWindow';
import { readMemories, readScan, SCAN_KEY, writeMemories, type DigiMemory, type ScanProps } from '../lib/digiScan';
import { DigiStatusBar, followRxPatch, followRxTarget, wpmStepPatch } from '../components/digi/StatusBar';
import { useT, type T } from '../i18n';
import { ListenGate } from '../components/ListenGate';
import { REMOTE_HOST } from '../lib/phone';
import { appendDigiNote, clearDigiText, getDigiText, onDigi, onDigiText, removeDigiLines } from '../lib/digiBus';
import { DigiRxLive } from '../components/DigiRxText';
import { nextHit } from '../lib/rxSearch';
import { txeStep } from '../lib/txeWait';
import { DigiTxBox } from '../components/DigiTxBox';
import { RttySettingsDialog } from '../components/digi/RttySettingsDialog';
import { useDigiTxLive } from '../lib/digiTxLive';
import { createLatestSequencer, restartBox } from '../lib/macroRestart';
import { TxMenu } from '../components/digi/TxMenu';
import { RowAverager, seekSignal, snapToPair, snapToPeak, snapToPsk, type SpectrumRow } from '../lib/digiSpectrum';
import { guessMode, type ModeGuessResult } from '../lib/modeGuess';
import { bandOf } from '../lib/format';
import { logBand, saveErrorText } from '../lib/logSaveError';
import { effectiveMacros, macroNameForKey, macroNeeds, runMacro, type Macro, type MacroAction, type MacroContext, type PrevQso } from '../lib/macroLang';
import { checkOnRx, macroState, mark, runCommand, sendCat, type MacroDeps } from '../lib/macroRuntime';
import { planMacro, splitAutoCq } from '../lib/macroPlan';
import { useListenWhileOpen } from '../lib/listenWhileOpen';
import { parseSpotSplit } from '../lib/spotSplit';
import { browserStorage } from '../lib/theme';
import { setContestSince } from '../lib/contestSince';
import { clearSize, readSize, writeSize } from '../lib/modeSizes';

/** Altezza minima del pannello BIGScan (px) con la maniglia del bordo inferiore. */
const RADAR_H_MIN = 160;
import { toast, toastAction } from '../lib/toast';
import { useAudioInputMenu } from '../components/digi/useAudioInputMenu';
import { useStation, useStatusPick } from '../state/station';
import '../styles/digi.css';
import '../styles/digi-dark.css';
import '../styles/digi-log-corta.css';
import '../styles/digi-band.css';

const VIEWS_KEY = 'aurora.digi.viste';
const MODE_NAMES: Record<DigiSettings['mode'], string> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY', cw: 'CW', ssb: 'SSB', olivia: 'OLIVIA', contestia: 'CONTESTIA' };
/** Secondi fra un CQ automatico e il successivo. */
const AUTOCQ_DELAY_S = 8;
const ACQ_DELAY_KEY = 'aurora-digi-acq-delay';
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
  /** Il testo trasmesso compare anche nella finestra di ricezione (di base no: resta nella finestra di trasmissione). */
  echoRx?: boolean;
  capsOnly: boolean;
  slashedZero: boolean;
  flashTx: boolean;
  hairlines: boolean;
  defaultRst: boolean;
  /** Cascata larga (0-5 kHz) o stretta (0-3,5 kHz). */
  wideWaterfall: boolean;
  /** L'orologio UTC nella barra di stato (menu del tasto destro della barra). */
  utc?: boolean;
  /** Il pannello «Segnali» di tutta la banda. */
  signals: boolean;
  /** I nominativi di tutta la banda scritti sulla cascata. */
  wfCalls: boolean;
  /** «Testo sulla cascata»: le ultime parole di ogni canale sotto il nominativo (assente nelle viste salvate prima: si'). */
  wfWords?: boolean;
  /** Stazioni della rete RBN sulla cascata e sotto Aurora BIGScan (assente = acceso; servono gli spot RBN accesi nelle impostazioni). */
  rbn?: boolean;
  /** Aurora BIGScan: anche chi risponde (dopo DE o TU), non solo chi chiama CQ. */
  radarAnswering: boolean;
}

const DEFAULT_VIEWS: Views = { controlBar: true, extended: true, log: true, waterfall: true, statusBar: true, underlineSent: true, capsOnly: false, slashedZero: true, flashTx: true, hairlines: false, defaultRst: true, wideWaterfall: true, signals: true, wfCalls: true, radarAnswering: false };

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
    case 'SSB':
    case 'USB':
    case 'LSB':
    case 'PHONE': return 'ssb';
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
  const status = useStatusPick(['dial_hz', 'de_call', 'de_grid']);
  // il selettore delle frequenze mostra solo le bande accese nelle Impostazioni (piu' quella in uso), come nella barra dell'FT8
  const chosenBands = useBandFilter((x) => x.bands);
  // le misure dei pannelli (cascata, RX/TX, Radar, macro, log) sono ricordate per modo: RTTY, CW e PSK hanno ciascuno le sue
  const sizeMode = rawSnap.settings.mode;
  const rxBox = useRef<HTMLPreElement>(null);
  const txBox = useRef<HTMLTextAreaElement>(null);
  const [tx, setTx] = useState('');
  // trasmissione a flusso: quanto del testo della finestra e' in coda e quanto e' gia' partito
  // quello che e' gia' partito via radio: resta nella finestra di trasmissione, sopra la casella (al piu' gli ultimi 4000 caratteri)
  const [txSent, setTxSent] = useState('');
  const txSentBox = useRef<HTMLDivElement>(null);
  const addSent = useCallback((sent: string) => {
    const line = sent.trim();
    if (line !== '') setTxSent((h) => (h === '' ? line : `${h}\n${line}`).slice(-4000));
  }, []);
  const txLive = useDigiTxLive(tx, setTx, addSent);
  // pulire la finestra di trasmissione (menu e macro <CLEARTXWINDOW>, <CLEARWINDOW>, <RXANDCLEAR>): la casella e lo storico di quello gia' partito
  const clearTxWindow = useCallback(() => {
    setTx('');
    setTxSent('');
  }, []);
  useEffect(() => {
    if (txSentBox.current) txSentBox.current.scrollTop = txSentBox.current.scrollHeight;
  }, [txSent]);
  const txLiveRef = useRef({ tx, txLive });
  txLiveRef.current = { tx, txLive };
  // larghezza del log di lato (si allarga trascinando il bordo), ricordata fra un avvio e l'altro
  const [logW, setLogW] = useState<number>(() => {
    const n = readSize('aurora.digi.logw', sizeMode);
    return Number.isFinite(n) && n >= 300 ? n : 432;
  });
  const [qso, setQso] = useState<Qso>(EMPTY_QSO);
  // il numero progressivo del contest (<QSONR>): avanza a ogni QSO salvato, ricordato nelle preferenze
  const [serial, setSerialState] = useState<number>(() => readSerial(browserStorage()));
  const setSerial = (n: number) => setSerialState(writeSerial(browserStorage(), n));
  // in gara <QSONR> viene dal log del contest (ultimo numero mandato + 1, 001 all'inizio); fuori gara il contatore della pagina
  const contest = useContestLog();
  const inContest = !!contest.attivo;
  const qsoSerial = inContest && contest.log ? contestNextSerial(contest.log) : serial;
  // il contest dei modi testuali scelto dal menu Contest (vale solo fuori dalla gara della pagina Contest): macro, numero da mandare, cattura
  const textContestQ = useQuery({ queryKey: ['contest-testo-attivo'], queryFn: getActiveTextContest, retry: false, refetchOnWindowFocus: false, staleTime: 30_000 });
  const textContest = inContest ? null : (textContestQ.data ?? null);
  // senza incremento automatico <NRS> manda lo scambio fisso del contest (zona, provincia)
  const fixedNrsRef = useRef('');
  fixedNrsRef.current = textContest && !textContest.contest.auto_inc ? textContest.contest.nrs.trim() : '';
  // la propria sezione / stato / club (per contest, impostata nelle impostazioni del contest) si manda dopo il numero solo se alla propria
  // stazione tocca per le regole del contest: lo dice il core con la stessa domanda dei nominativi (`exch_atteso`) fatta sul proprio nominativo
  const myCallNow = status?.de_call ?? '';
  const ownExchQ = useQuery({
    queryKey: ['contest-exch-own', textContest?.contest.id ?? '', myCallNow],
    queryFn: () => getTextContestCheck(myCallNow, '', ''),
    enabled: textContest !== null && myCallNow.trim().length >= 3,
    retry: false,
    staleTime: 60_000,
    refetchOnWindowFocus: false,
  });
  const sentSectionRef = useRef({ id: null as string | null, applies: false });
  sentSectionRef.current = { id: textContest?.contest.id ?? null, // il contest chiede la propria sezione se il punteggio ha un moltiplicatore sullo scambio (`exch_atteso`) o se la cartella dichiara un campo di testo da
  // mandare (zona di Volta, sezione del Field Day, riferimento IOTA, nome del SARTG New Year): senza questo `<MYSECT>` e il log restavano vuoti
  applies: textContest !== null && (ownExchQ.data?.exch_atteso === true || haSezionePropria(textContest.contest)) };
  const [contestList, setContestList] = useState(false);
  const [contestStats, setContestStats] = useState(false);
  /** La finestra dei QTC del WAE (mandare o ricevere), con il contest attivo che li prevede. */
  const [qtcOpen, setQtcOpen] = useState<QtcDir | null>(null);
  const textScore = useTextContestScore(!!textContest).data;
  /** SSB: la pagina cambia disposizione (Log, inserimento del QSO, DX Cluster) e non mostra RX/TX, cascata e modi digitali; con un contest attivo compaiono anche Nr, punteggio, DUPE e moltiplicatori. */
  const ssbContest = snap.settings.mode === 'ssb';
  // il tratto di banda a cui porta un blocchetto: fonia in SSB, il piano IARU del modo negli altri (CW, RTTY, PSK)
  const contestPlan = snap.settings.mode === 'ssb' ? 'SSB' : snap.settings.mode === 'cw' ? 'CW' : snap.settings.mode === 'rtty' ? 'RTTY' : snap.settings.mode === 'psk63' ? 'PSK63' : isOliviaMode(snap.settings.mode) ? 'OLIVIA' : 'PSK31';
  const contestModeIds = (textContest ? modiDichiarati(textContest.contest) : []).map((k) => (k === 'cw' ? 'cw' : k === 'phone' ? 'ssb' : 'rtty'));
  // si disegna al momento del render (usa `save` e `sending`, definiti piu' sotto)
  const contestModeButtons = () =>
    contestModeIds.length > 1 ? (
      <div className="ssbc-modes" role="group" aria-label={t('digi.menu.mode')}>
        {contestModeIds.map((id) => (
          <button key={id} type="button" className={`ssbc-mode${snap.settings.mode === id ? ' on' : ''}`} aria-pressed={snap.settings.mode === id} disabled={sending} onClick={() => save({ mode: id as DigiSettings['mode'] })}>
            {id.toUpperCase()}
          </button>
        ))}
      </div>
    ) : null;
  /** Con un contest attivo la pagina (SSB, CW, RTTY, PSK) e' a schede spostabili: inserimento, log, DX Cluster, punteggio, moltiplicatori e, nei modi testuali, ricezione, trasmissione e cascata. Senza contest resta quella di sempre. */
  const contestSheets = textContest !== null;
  const exportCabrillo = useExportTextCabrillo();
  const [contestEdit, setContestEdit] = useState<TextContest | null>(null);
  const selectTextContest = useSelectTextContest();
  // scegliendo un contest con l'incremento automatico il progressivo riparte dal suo primo numero (001 se vuoto)
  const prevContestId = useRef<string | null | undefined>(undefined);
  useEffect(() => {
    if (!textContestQ.isSuccess) return;
    const c = textContestQ.data?.contest ?? null;
    const id = c?.id ?? null;
    if (prevContestId.current !== undefined && prevContestId.current !== id && c?.auto_inc) setSerialState(writeSerial(browserStorage(), startSerial(c.nrs)));
    prevContestId.current = id;
  }, [textContestQ.isSuccess, textContestQ.data]);
  // il menu del tasto destro su una parola della finestra di ricezione
  const [wordMenu, setWordMenu] = useState<WordAt | null>(null);
  // il nominativo preso dalla ricezione va nel QSO in corso e si cercano nome, QTH, locatore e paese
  const takeCall = useCallLookup(setQso);
  // il nominativo scritto a mano riempie da solo nome, QTH e locatore (se la ricerca e' configurata); nel contest SSB la riga non li ha
  useCallsignAutofill(qso.call, { name: qso.name, qth: qso.qth, wwloc: qso.wwloc }, (patch) => setQso((q) => ({ ...q, ...patch })), !contestSheets);
  const [rotorShown, setRotorShown] = useRotorCardVisible('digi');
  // il DX del QSO anche per il rotore staccato in una finestra a parte
  usePublishRotorDx(qso.call, qso.wwloc);
  // la larghezza della scheda delle macro: si regola dal bordo destro (doppio clic: quella di partenza)
  const macroBox = useRef<HTMLDivElement>(null);
  const [macroW, setMacroW] = useState<number | null>(() => {
    const v = readSize('aurora.digi.macrow', sizeMode);
    return Number.isFinite(v) && v >= 360 ? v : null;
  });
  // le righe di macro mostrate (menu Vista › Righe delle macro), ricordate per modo; 0 = di base (tutte e 4), 1-4 a scelta, 5 = automatico (solo le righe con macro)
  const [macroRows, setMacroRowsState] = useState<number>(() => {
    const v = readSize('aurora.digi.macrorows', sizeMode);
    return Number.isFinite(v) && v >= 1 && v <= 5 ? v : 0;
  });
  const setMacroRows = (n: number) => {
    setMacroRowsState(n);
    try {
      if (n === 0) clearSize('aurora.digi.macrorows', sizeMode);
      else writeSize('aurora.digi.macrorows', sizeMode, n);
    } catch {
      // la scelta vale comunque finche' la pagina e' aperta
    }
  };
  // i dati che non cambiano da un QSO all'altro (nome, QTH, RST): gli stessi di Impostazioni › Stazione › Dati personali
  const [saved, setSavedField] = usePersonal();
  const [startAt, setStartAt] = useState<Date | null>(null);
  // la banda scelta a mano con i pulsanti delle bande: serve a registrare il QSO quando la radio non da la frequenza (niente CAT)
  const [manualBand, setManualBand] = useState('');
  // il motivo per cui l'ultimo salvataggio e' stato rifiutato, col nominativo a cui si riferisce (sparisce se il nominativo cambia)
  const [saveError, setSaveError] = useState<{ msg: string; call: string } | null>(null);
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
  // la «Guida alle macro» (menu Aiuto)
  const [macroGuide, setMacroGuide] = useState(false);
  // la finestra «Impostazioni RTTY» (clic sul modo nella barra in basso o sulla voce RTTY del menu Modo)
  const [rttyOpen, setRttyOpen] = useState(false);
  const [autoCq, setAutoCq] = useState(false);
  // doppio Esc: l'Auto CQ resta acceso ma in pausa; riparte quando si ripreme (macro <AUTOCQ>)
  const [autoCqPaused, setAutoCqPaused] = useState(false);
  // la pausa fra due chiamate (<ACQDELAY:s>) e la chiamata data da <AUTOCQ>testo (se no AutoCqText o F2)
  // la pausa scelta dal menu resta salvata (localStorage), come nei programmi di contest
  const [autoCqDelay, setAutoCqDelayState] = useState(() => {
    try {
      const v = Number(browserStorage()?.getItem(ACQ_DELAY_KEY));
      return Number.isFinite(v) && v >= 1 && v <= 600 ? v : AUTOCQ_DELAY_S;
    } catch {
      return AUTOCQ_DELAY_S;
    }
  });
  const setAutoCqDelay = (sec: number) => {
    const v = Math.max(1, Math.min(600, Math.round(sec)));
    setAutoCqDelayState(v);
    try {
      browserStorage()?.setItem(ACQ_DELAY_KEY, String(v));
    } catch {
      // senza archivio vale fino alla chiusura
    }
  };
  const [acqDelayOpen, setAcqDelayOpen] = useState(false);
  const autoCqScript = useRef<string | null>(null);
  const lastEsc = useRef(0);
  // <TXE>: l'attesa che cada il segnale ricevuto (Esc la annulla)
  const txeTimer = useRef<number | null>(null);
  const stopTxe = () => {
    if (txeTimer.current === null) return false;
    window.clearInterval(txeTimer.current);
    txeTimer.current = null;
    return true;
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => () => {
    stopTxe();
    if (clearAfterTimer.current !== null) window.clearInterval(clearAfterTimer.current);
  }, []);
  // barra sopra la finestra RX: vista ferma (Pausa) e ricerca di un nominativo
  const [rxPaused, setRxPaused] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [search, setSearch] = useState('');
  const [hit, setHit] = useState({ at: -1, count: 0 });
  const searchBox = useRef<HTMLInputElement>(null);
  const hitMarks = () => (rxBox.current ? [...rxBox.current.querySelectorAll<HTMLElement>('mark.digi-hit[data-first]')] : []);
  const jumpHit = (back: boolean) => {
    const marks = hitMarks();
    const at = nextHit(hit.at < marks.length ? hit.at : -1, marks.length, back);
    rxBox.current?.querySelectorAll('mark.digi-hit--now').forEach((m) => m.classList.remove('digi-hit--now'));
    setHit({ at, count: marks.length });
    const m = at >= 0 ? marks[at] : undefined;
    if (!m) return;
    m.classList.add('digi-hit--now');
    // per leggere il risultato la finestra si ferma (Pausa premuto)
    setRxPaused(true);
    m.scrollIntoView({ block: 'nearest' });
  };
  const closeSearch = () => {
    setSearchOpen(false);
    setSearch('');
    setHit({ at: -1, count: 0 });
  };
  useEffect(() => {
    if (searchOpen) searchBox.current?.focus();
  }, [searchOpen]);
  useEffect(() => {
    // il conto dei risultati dopo che la finestra si e' ridisegnata
    const id = window.requestAnimationFrame(() => setHit({ at: -1, count: hitMarks().length }));
    return () => window.cancelAnimationFrame(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search]);
  const [scanning, setScanning] = useState(false);
  const s = snap.settings;
  const sending = snap.tx.state === 'sending';
  // <RXANDCLEAR>: pulizia della finestra di trasmissione rimandata alla fine della trasmissione
  const clearAfterTx = useRef(false);
  const clearAfterTimer = useRef<number | null>(null);
  // il menu del tasto destro nella finestra di trasmissione
  const [txMenu, setTxMenu] = useState<{ x: number; y: number } | null>(null);
  // il testo nuovo della finestra di trasmissione (tastiera e menu): mentre si trasmette parte subito, e il Backspace toglie solo quello
  // che non e' ancora partito
  const txEdit = (v: string) => {
    const next = views.capsOnly ? v.toUpperCase() : v;
    if (sending || txLive.progress.queued > 0) txLive.edit(next);
    else setTx(next);
  };
  // macro ripremute ancora in viaggio verso il motore: finche' ce n'e' una, la fine della trasmissione di prima non pulisce
  const macroInflight = useRef(0);
  const wasSending = useRef(false);
  useEffect(() => {
    if (sending) wasSending.current = true;
    else if (wasSending.current) {
      wasSending.current = false;
      if (clearAfterTx.current && macroInflight.current === 0) {
        clearAfterTx.current = false;
        // il testo appena partito passa nello storico a fine trasmissione: si pulisce subito dopo, cosi' non ricompare
        clearTxWindow();
        window.setTimeout(clearTxWindow, 600);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sending]);
  const lock = !s.tx_follow_rx;
  const rxHz = snap.rx.freq_hz > 0 ? snap.rx.freq_hz : s.rx_freq_hz;

  // i notch della ricezione: stanno nel motore (non si salvano: all'avvio e al cambio di frequenza della radio sono vuoti)
  const notchQuery = useQuery({ queryKey: ['digi-notches'], queryFn: getDigiNotches, retry: false });
  const notches = notchQuery.data ?? [];
  useEffect(() => onDigi((ev) => (ev.kind === 'notches' ? void client.invalidateQueries({ queryKey: ['digi-notches'] }) : undefined)), [client]);
  const setNotches = useCallback(
    (list: DigiNotch[]) => {
      client.setQueryData(['digi-notches'], list);
      void putDigiNotches(list).then(
        (next) => client.setQueryData(['digi-notches'], next),
        () => void client.invalidateQueries({ queryKey: ['digi-notches'] }),
      );
    },
    [client],
  );

  // finestre RX in piu': stanno nel motore (non si salvano), ognuna con la sua finestra piccola e il suo indicatore sulla cascata
  const extraQuery = useQuery({ queryKey: ['digi-extra-rx'], queryFn: getExtraRx, retry: false });
  const extras = extraQuery.data ?? [];
  useEffect(() => onDigi((ev) => (ev.kind === 'extra_rx' ? void client.invalidateQueries({ queryKey: ['digi-extra-rx'] }) : undefined)), [client]);
  const setExtras = useCallback(
    (list: ExtraRx[]) => {
      client.setQueryData(['digi-extra-rx'], list);
      void putExtraRx(list).then(
        (next) => client.setQueryData(['digi-extra-rx'], next),
        () => void client.invalidateQueries({ queryKey: ['digi-extra-rx'] }),
      );
    },
    [client],
  );
  const addExtra = (hz: number) => {
    const used = new Set(extras.map((x) => x.id));
    const id = [1, 2, 3, 4].find((n) => !used.has(n));
    if (id === undefined) return;
    clearExtraText(id);
    setExtras([...extras, { id, freq_hz: hz }]);
  };
  const closeExtra = (id: number) => {
    clearExtraText(id);
    setExtras(extras.filter((x) => x.id !== id));
  };
  // la finestra RX in piu' e la ricezione principale si scambiano la frequenza
  const swapExtra = (id: number) => {
    const x = extras.find((e) => e.id === id);
    if (!x) return;
    const main = rxHz;
    pickFreq(x.freq_hz, false);
    clearExtraText(id);
    setExtras(extras.map((e) => (e.id === id ? { ...e, freq_hz: main } : e)));
  };
  // il menu del tasto destro sulla cascata: dove e su che frequenza
  const [wfMenu, setWfMenu] = useState<{ x: number; y: number; hz: number } | null>(null);
  const closeWfMenu = useCallback(() => setWfMenu(null), []);
  const nearestExtra = (hz: number) => extras.reduce<ExtraRx | null>((best, x) => (best === null || Math.abs(x.freq_hz - hz) < Math.abs(best.freq_hz - hz) ? x : best), null);

  // tutta la banda: i segnali del modo in uso (CW; PSK31 e PSK63, il browser), con il nominativo sopra la traccia nella cascata
  const bandAll = useSyncExternalStore(onBandSignals, getBandSignals);
  const isPsk = s.mode === 'psk31' || s.mode === 'psk63';
  // Radar: CW e RTTY (tutte le stazioni sicure della banda accanto alla cascata); il PSK ha il pannello «Segnali»
  const radarMode = s.mode === 'cw' || s.mode === 'rtty';
  const radarOn = (s.mode === 'cw' && s.band_cw !== false) || (s.mode === 'rtty' && s.band_rtty !== false);
  const bandOn = radarOn || (isPsk && s.band_psk !== false);
  const band = useMemo(() => (bandOn && getBandMode() === s.mode ? bandAll : []), [bandAll, bandOn, s.mode]);
  // la stazione che chiama CQ nella finestra RX principale resta sempre in BIGScan, anche se la ricerca di tutta la banda la perde
  // (lo stesso per le finestre RX in piu': ognuna manda il suo CQ alla sua frequenza)
  const [rxCq, setRxCq] = useState<Record<string, { call: string; hz: number; at: number }>>({});
  const extrasRef = useRef(extras);
  extrasRef.current = extras;
  useEffect(() => {
    const seen = (key: string, text: string, hz: number) => {
      const call = cqCallOf(text.slice(-800));
      if (!call) return;
      setRxCq((all) => {
        const o = all[key];
        return o && o.call === call && Date.now() - o.at < 2000 ? all : { ...all, [key]: { call, hz, at: Date.now() } };
      });
    };
    const offMain = onDigiText(() => seen('main', getDigiText(), live.current.rxHz));
    const offExtra = onExtraText(() => {
      for (const x of extrasRef.current) seen(`x${x.id}`, getExtraText(x.id), x.freq_hz);
    });
    return () => {
      offMain();
      offExtra();
    };
  }, []);
  const radar = useMemo(() => {
    if (!radarMode) return [];
    const list = radarStations(band, views.radarAnswering);
    for (const c of Object.values(rxCq)) {
      if (Date.now() - c.at >= 75_000 || list.some((r) => r.call === c.call)) continue;
      list.push({ call: c.call, freqHz: c.hz, wpm: null, snrDb: 0, cq: true, country: null, stale: false, color: radarColor(c.call) });
    }
    return list.sort((a, b) => a.freqHz - b.freqHz);
  }, [band, radarMode, views.radarAnswering, rxCq]);
  // sulla cascata del Radar solo i nominativi sicuri (verificati)
  // sulla cascata di Aurora BIGScan gli stessi nominativi dell'elenco (di base solo chi chiama CQ)
  const bandLabels = useMemo(() => {
    if (!radarMode) return waterfallLabels(band);
    const shown = new Set(radar.map((r) => r.call));
    return waterfallLabels(band)
      .filter((l) => l.sure && shown.has(l.text))
      .map((l) => ({ ...l, color: radarColor(l.text) }));
  }, [band, radarMode, radar]);
  // Reverse Beacon Network: chi sentono i ricevitori RBN nel tratto della cascata e Aurora non ha decodificato
  const radarCalls = useMemo(() => new Set(radar.map((r) => r.call.toUpperCase())), [radar]);
  const rbnSpots = useRbnSpots(radarMode && views.rbn !== false, status?.dial_hz ?? 0, s.mode === 'cw' ? 'CW' : 'RTTY', WF_MIN, views.wideWaterfall ? WF_MAX_WIDE : WF_MAX_NARROW, radarCalls);
  const wfLabels = useMemo(() => [...bandLabels, ...rbnLabels(rbnSpots, status?.dial_hz ?? 0)], [bandLabels, rbnSpots, status?.dial_hz]);
  const toggleRadar = () => (s.mode === 'cw' ? save({ band_cw: s.band_cw === false }) : save({ band_rtty: s.band_rtty === false }));

  // <MODEMACROS:modo>: le macro di un altro modo senza cambiare modo
  const [macroMode, setMacroMode] = useState<string | null>(null);
  const mm = macroMode ?? s.mode;
  const macros = useMemo(() => withContestMacros(effectiveMacros(s.macros.map(toMacro), s.mode_macros[mm]?.map(toMacro), s.merge_macros[mm] !== false), textContest?.macros, textContest?.contest.merge_macros !== false), [s.macros, s.mode_macros, s.merge_macros, mm, textContest]);

  // lo spettro: la media delle righe recenti serve a Snap, Seek e Scan
  const avg = useRef(new RowAverager());
  const lastRow = useRef<SpectrumRow | null>(null);
  // le righe degli ultimi 30 s circa (una su tre, copiate) per il suggerimento del modo dal tasto destro sulla cascata
  const guessRows = useRef<{ row: SpectrumRow; at: number }[]>([]);
  const rowTick = useRef(0);
  const onRow = useCallback((row: SpectrumRow) => {
    lastRow.current = row;
    avg.current.push(row);
    if (rowTick.current++ % 3 === 0) {
      const h = guessRows.current;
      h.push({ row: { bins: row.bins.slice(), fMinHz: row.fMinHz, fMaxHz: row.fMaxHz }, at: performance.now() });
      if (h.length > 150) h.shift();
    }
  }, []);
  const [guess, setGuess] = useState<{ x: number; y: number; hz: number; result: ModeGuessResult } | null>(null);
  const closeGuess = useCallback(() => setGuess(null), []);
  const showGuess = useCallback((x: number, y: number, hz: number) => {
    const h = guessRows.current;
    const first = h[0];
    const lastOne = h[h.length - 1];
    const spr = first && lastOne && h.length > 1 ? (lastOne.at - first.at) / 1000 / (h.length - 1) : 0.2;
    setGuess({ x, y, hz, result: guessMode(h.map((e) => e.row), hz, spr) });
  }, []);

  // l'ultimo stato, per i comandi dei tasti, la scansione e il CQ automatico (che non devono usare valori vecchi)
  const live = useRef({ snap, tx, qso, saved, startAt, manualBand, status, macros, scanProps, rxHz, serial: qsoSerial, inContest, views, rotorShown });
  live.current = { snap, tx, qso, saved, startAt, manualBand, status, macros, scanProps, rxHz, serial: qsoSerial, inContest, views, rotorShown };
  // contest dei modi testuali: il numero o il dominio ricevuti dopo il nominativo del QSO vanno da soli nel numero ricevuto, se e' vuoto
  useEffect(() => {
    const c = textContest;
    if (!c || !(c.contest.grab_number || c.contest.grab_state)) return;
    return onDigiText(() => {
      const q = live.current.qso;
      if (q.call.trim() === '' || (q.nrr ?? '').trim() !== '') return;
      // le righe «» …» (il nostro testo e l'eco della trasmissione) grabExchange le salta: il numero dopo il nominativo li' e' il nostro
      const v = grabExchange(getDigiText().slice(-300), q.call, { number: c.contest.grab_number, domain: c.contest.grab_state ? c.domain : null });
      if (v) setQso((x) => (x.call === q.call && (x.nrr ?? '').trim() === '' ? { ...x, nrr: v } : x));
    });
  }, [textContest]);

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

  // Olivia e Contestia: toni o banda dal menu o dalla barra; se non entrano nella banda audio lo si dice subito (il motore li salva ma non
  // trasmette e non riceve)
  const saveOlivia = (part: Partial<OliviaSettings>) => {
    const cur = live.current.snap.settings;
    if (!isOliviaMode(cur.mode)) return;
    const patch = oliviaPatch(cur, cur.mode, part);
    save(patch);
    const next = { ...cur, ...patch };
    const o = oliviaOf(next, cur.mode);
    const bad = oliviaProblem(next);
    if (bad) toast(t(bad === 'tx' ? 'digi.olivia.noFit' : 'digi.olivia.noFitRx', { tones: o.tones, bw: o.bandwidth_hz, hz: Math.round(bad === 'tx' ? effectiveTxHz(next) : next.rx_freq_hz) }), 'error');
  };

  // il divisore fra ricezione e trasmissione si trascina: la parte alta della colonna (0,2-0,8), ricordata
  // l'altezza della cascata si regola dalla barra sul suo bordo alto (da 50 a 400 px), ricordata
  const [wfHeight, setWfHeight] = useState<number>(() => {
    const n = readSize('aurora.digi.wfh2', sizeMode);
    return Number.isFinite(n) && n >= 50 && n <= 400 ? n : 80;
  });
  const changeWfHeight = (px: number) => {
    const h = Math.round(Math.min(400, Math.max(50, px)));
    setWfHeight(h);
    try {
      writeSize('aurora.digi.wfh2', sizeMode, h);
    } catch {
      // l'altezza vale comunque finche' la pagina e' aperta
    }
  };
  // larghezza della cascata (maniglia sul bordo destro; null = tutta la colonna), ricordata per modo come le altre misure
  const [wfWidth, setWfWidth] = useState<number | null>(() => {
    const n = readSize('aurora.digi.wfw', sizeMode);
    return Number.isFinite(n) && n >= 240 ? n : null;
  });
  const changeWfWidth = (px: number | null) => {
    setWfWidth(px);
    if (px === null) clearSize('aurora.digi.wfw', sizeMode);
    else writeSize('aurora.digi.wfw', sizeMode, px);
  };
  const windowsRef = useRef<HTMLDivElement>(null);
  // larghezza delle finestre RX e TX scelta con la maniglia sul loro bordo destro (null = tutta la colonna): log e DX Cluster restano fermi
  // dove sono, lo spazio che si libera resta vuoto (richiesta del proprietario, per esempio per tenere accanto la scheda del rotore)
  // larghezza del Radar (px), scelta trascinando la maniglia fra le finestre RX/TX e il Radar; resta ricordata
  const [radarW, setRadarW] = useState<number>(() => {
    const n = readSize('aurora.digi.radarw', sizeMode);
    return Number.isFinite(n) && n >= 220 && n <= 900 ? n : 330;
  });
  // altezza del pannello (px), scelta dalla maniglia sul bordo inferiore; null = quella di partenza (alto quanto le finestre RX/TX)
  const [radarH, setRadarH] = useState<number | null>(() => {
    const n = readSize('aurora.digi.radarh', sizeMode);
    return Number.isFinite(n) && n >= RADAR_H_MIN ? n : null;
  });
  const dragRadarH = (e: React.PointerEvent) => {
    const box = (e.currentTarget as HTMLElement).closest('.radar') as HTMLElement | null;
    if (!box) return;
    e.preventDefault();
    const startY = e.clientY;
    const startH = box.offsetHeight;
    const scale = box.offsetHeight > 0 ? box.getBoundingClientRect().height / box.offsetHeight : 1;
    let last = startH;
    const move = (m: PointerEvent) => {
      last = Math.round(Math.min(Math.max(RADAR_H_MIN, startH + (m.clientY - startY) / scale), Math.max(RADAR_H_MIN, window.innerHeight / scale - 160)));
      setRadarH(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      writeSize('aurora.digi.radarh', sizeMode, last);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const resetRadarH = () => {
    setRadarH(null);
    clearSize('aurora.digi.radarh', sizeMode);
  };
  const dragRadarW = (e: React.PointerEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = radarW;
    let last = startW;
    const move = (m: PointerEvent) => {
      last = Math.round(Math.min(900, Math.max(220, startW - (m.clientX - startX))));
      setRadarW(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      try {
        writeSize('aurora.digi.radarw', sizeMode, last);
      } catch {
        // la larghezza vale comunque finche' la pagina e' aperta
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const [winW, setWinW] = useState<number | null>(() => {
    const n = readSize('aurora.digi.winw', sizeMode);
    return Number.isFinite(n) && n >= 240 ? n : null;
  });
  const dragWinW = (e: React.PointerEvent) => {
    e.preventDefault();
    const box = windowsRef.current?.getBoundingClientRect();
    const col = windowsRef.current?.parentElement?.getBoundingClientRect();
    if (!box || !col) return;
    let last: number | null = null;
    const move = (m: PointerEvent) => {
      last = Math.round(Math.min(col.width, Math.max(240, m.clientX - box.left)));
      setWinW(last >= col.width - 4 ? null : last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      try {
        if (last === null) return;
        if (last >= col.width - 4) clearSize('aurora.digi.winw', sizeMode);
        else writeSize('aurora.digi.winw', sizeMode, last);
      } catch {
        // la larghezza vale comunque finche' la pagina e' aperta
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const resetWinW = () => {
    setWinW(null);
    try {
      clearSize('aurora.digi.winw', sizeMode);
    } catch {
      // niente da togliere
    }
  };
  const [rxFrac, setRxFrac] = useState<number>(() => {
    const n = readSize('aurora.digi.rxfrac', sizeMode);
    return Number.isFinite(n) && n >= 0.25 && n <= 0.8 ? n : 0.55;
  });
  // cambiando modo (RTTY, CW, PSK) si riprendono le misure di quel modo
  const firstSizeMode = useRef(sizeMode);
  useEffect(() => {
    if (firstSizeMode.current === sizeMode) return;
    firstSizeMode.current = sizeMode;
    const ok = (n: number, min: number, max = Infinity) => Number.isFinite(n) && n >= min && n <= max;
    const logw = readSize('aurora.digi.logw', sizeMode);
    setLogW(ok(logw, 300) ? logw : 432);
    const macrow = readSize('aurora.digi.macrow', sizeMode);
    setMacroW(ok(macrow, 360) ? macrow : null);
    const wfh = readSize('aurora.digi.wfh2', sizeMode);
    setWfHeight(ok(wfh, 50, 400) ? wfh : 80);
    const radarw = readSize('aurora.digi.radarw', sizeMode);
    setRadarW(ok(radarw, 220, 900) ? radarw : 330);
    const winw = readSize('aurora.digi.winw', sizeMode);
    setWinW(ok(winw, 240) ? winw : null);
    const frac = readSize('aurora.digi.rxfrac', sizeMode);
    setRxFrac(ok(frac, 0.25, 0.8) ? frac : 0.55);
    const wfw = readSize('aurora.digi.wfw', sizeMode);
    setWfWidth(ok(wfw, 240) ? wfw : null);
  }, [sizeMode]);
  const dragSplit = (e: React.PointerEvent) => {
    e.preventDefault();
    const box = windowsRef.current?.getBoundingClientRect();
    if (!box) return;
    const move = (m: PointerEvent) => setRxFrac(Math.min(0.8, Math.max(0.25, (m.clientY - box.top) / box.height)));
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setRxFrac((f) => {
        try {
          writeSize('aurora.digi.rxfrac', sizeMode, f);
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
  // sul client remoto (telefono, portatile in rete) aprire la pagina non accende il motore: si ascolta solo dopo «Ascolta»
  const [armed, setArmed] = useState(!REMOTE_HOST);
  useListenWhileOpen(digiRx, () => live.current.snap.settings.auto_receive, onChanged, digiRxPage, armed);

  const caps = useRef(false);
  caps.current = views.capsOnly;

  // «Use default RST»: dopo ogni QSO (azzerato o salvato) i rapporti tornano 599 (59 in fonia)
  const resetRst = () => {
    const rst = live.current.snap.settings.mode === 'ssb' ? '59' : '599';
    if (views.defaultRst) setSavedField({ rsts: rst, rstr: rst });
  };

  // il testo parte e resta nella finestra di trasmissione, dove si vede a che punto e' arrivato (l'eco nella ricezione arriva dal motore)
  const send = useCallback(
    (body: string, fromBox = false): Promise<boolean> => {
      const { txLive: tl } = txLiveRef.current;
      const queued = tl.progress.queued;
      // dalla finestra parte solo quello che non e' gia' in coda (prima si capiva confrontando il testo con la casella: una macro
      // uguale al testo gia' nella casella, come la stessa macro ripremuta, risultava «dalla casella» e non partiva niente)
      const upper = caps.current ? body.toUpperCase() : body;
      const message = fromBox ? upper.slice(queued) : upper.trim();
      if (message.trim() === '') return Promise.resolve(false);
      setProblem('');
      return digiTransmit(message).then(
        () => {
          if (!fromBox) setTx((v) => v.slice(0, queued) + message + v.slice(queued));
          tl.queue(message.length);
          onChanged();
          return true;
        },
        (e: unknown) => {
          const code = e instanceof ApiError ? e.code : '';
          setProblem(txProblem(code, live.current.snap.settings, t));
          return false;
        },
      );
    },
    [onChanged, t],
  );

  // le macro premute mentre si trasmette: in fila verso il motore, l'ultima vince (vedi macroRestart)
  const macroSeq = useRef(createLatestSequencer());
  const restartTx = useCallback(
    (body: string, open: boolean): Promise<{ ok: boolean; latest: boolean }> => {
      const message = (caps.current ? body.toUpperCase() : body).trim();
      setProblem('');
      macroInflight.current++;
      const r = macroSeq.current.run(() => digiTransmitRestart(message, open));
      void r.done.then(
        () => macroInflight.current--,
        () => macroInflight.current--,
      );
      return r.done.then(
        (kept) => {
          const { txLive: tl } = txLiveRef.current;
          // nessuna trasmissione prima: quello che era scritto nella casella resta dopo la macro, come prima
          const fresh = tl.progress.queued === 0 && kept === 0;
          setTx((v) => (fresh ? message + v : restartBox(v, kept, message)));
          tl.restart(kept, message.length);
          onChanged();
          return { ok: true, latest: macroSeq.current.isLatest(r.gen) };
        },
        (e: unknown) => {
          const code = e instanceof ApiError ? e.code : '';
          setProblem(txProblem(code, live.current.snap.settings, t));
          return { ok: false, latest: macroSeq.current.isLatest(r.gen) };
        },
      );
    },
    [onChanged, t],
  );

  // la levetta RX/TX (e il tasto Pausa): apre la trasmissione, che resta accesa e manda quello che si scrive; di nuovo la chiude a fine
  // testo. Esc la ferma subito.
  const toggleTx = () => {
    if (live.current.snap.tx.state === 'sending') {
      void digiTxClose().then(onChanged, onChanged);
      return;
    }
    const box = caps.current ? live.current.tx.toUpperCase() : live.current.tx;
    setProblem('');
    digiTxOpen(box).then(
      () => {
        setTx(box);
        txLiveRef.current.txLive.queue(box.length);
        onChanged();
        // dopo il clic sul pulsante RX/TX il fuoco resta sul pulsante: i tasti non arriverebbero alla casella
        txBox.current?.focus();
      },
      (e: unknown) => {
        const code = e instanceof ApiError ? e.code : '';
        setProblem(txProblem(code, live.current.snap.settings, t));
      },
    );
  };

  // ---- sintonia: clic sulla cascata (con Snap), Seek, Scan ----
  const pickFreq = (hz: number, viaClick: boolean, ctrl = false, snapIt = viaClick) => {
    // <LOCKRX>: la ricezione resta dov'e'
    if (viaClick && macroState.rxLocked) return;
    const l = live.current;
    // in RTTY la frequenza e' il centro fra i due toni: Snap cerca la coppia (un tono solo andrebbe sul mark o sullo space)
    const rttyShift = l.snap.settings.mode === 'rtty' ? l.snap.settings.rtty_shift_hz : 0;
    // in PSK la portante e' al centro del segnale: la riga piu' forte e' una delle due a mezzo baud, fuori di quanto l'AFC recupera
    const pskHalf = l.snap.settings.mode === 'psk31' ? 20 : l.snap.settings.mode === 'psk63' ? 40 : 0;
    // Olivia e Contestia: il segnale e' largo quanto la banda e la frequenza e' il centro, Snap non aggancia un tono (resta il clic)
    const snapped =
      snapIt && l.snap.settings.snap_on && avg.current.ready && lastRow.current && !isOliviaMode(l.snap.settings.mode)
        ? rttyShift > 0
          ? snapToPair(avg.current.values(), lastRow.current, hz, rttyShift)
          : pskHalf > 0
            ? snapToPsk(avg.current.values(), lastRow.current, hz, pskHalf)
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
    // il modo: prima il piano di banda, poi quello scritto dal cluster (girare la manopola della radio non cambia mai il modo)
    const want = textModeOfSpot(spot);
    const cur = l.snap.settings.mode;
    const patch: Partial<DigiSettings> = {};
    if (want !== null && want !== cur) patch.mode = want;
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
  // uno spot RTTY, CW o PSK cliccato in un'altra pagina: appena aperta questa, lo si sintonizza nel modo giusto (una volta sola)
  const tuneToSpotRef = useRef(tuneToSpot);
  tuneToSpotRef.current = tuneToSpot;
  useEffect(() => {
    const spot = takePendingSpot();
    if (spot) void tuneToSpotRef.current(spot, false).catch(() => toast(t('digi.spot.failed'), 'info'));
  }, [t]);

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
      nrr: l.qso.nrr ?? '',
      nrs: exchangeSent(l.qso.nrs || fixedNrsRef.current, l.serial),
      // come lo scambio che va nel log (`composeSent`): solo quando il contest chiede la propria sezione, cosi' macro e log dicono la stessa cosa
      mysect: sentSectionRef.current.applies ? readSection(browserStorage(), sentSectionRef.current.id) : '',
      startTime: l.startAt,
      now: new Date(),
      dialHz: dial,
      band: dial > 0 ? (bandOf(dial) ?? '') : '',
      modeName: MODE_NAMES[l.snap.settings.mode],
      qsoNumber: l.serial,
      macros: new Map([...l.macros.values()].filter((m) => m.text.trim() !== '').map((m) => [m.name.toUpperCase(), m.text] as const)),
      lang: forcedLang.current,
      txStart: txStartRef.current,
      rxHz: l.rxHz,
      snr: l.snap.rx.snr_db ?? null,
      imd: l.snap.rx.imd_db ?? null,
      rate: l.snap.settings.mode === 'cw' ? String(Math.round(l.snap.rx.wpm ?? l.snap.settings.cw_wpm)) : l.snap.settings.mode === 'rtty' ? String(l.snap.settings.rtty_baud) : l.snap.settings.mode === 'psk63' ? '62.5' : isOliviaMode(l.snap.settings.mode) ? oliviaLabel(oliviaOf(l.snap.settings, l.snap.settings.mode)) : '31.25',
      inContest: l.inContest || textContest !== null,
    };
  };

  // ---- comandi delle macro eseguiti dalla pagina (macroRuntime) ----
  const forcedLang = useRef('');
  const txStartRef = useRef<Date | null>(null);
  const [rxStyle, setRxStyle] = useState<Record<string, string>>({});
  useEffect(() => {
    const el = rxBox.current;
    if (!el) return;
    for (const k of ['fontSize', 'fontFamily', 'color', 'fontWeight', 'fontStyle'] as const) el.style[k] = rxStyle[k] ?? '';
  }, [rxStyle]);
  const macroDeps = (): MacroDeps => ({
    t: (key, vars) => t(key as Parameters<typeof t>[0], vars),
    toast,
    settings: () => live.current.snap.settings,
    save,
    rxHz: () => live.current.rxHz,
    dialHz: () => live.current.status?.dial_hz ?? 0,
    pickFreq: (hz, snapIt) => pickFreq(hz, false, false, snapIt),
    setDial: (hz) => {
      useStation.getState().patchStatus({ dial_hz: hz });
      void postFrequency(hz).then(onChanged, onChanged);
    },
    clearRx: clearDigiText,
    clearTx: clearTxWindow,
    setTx: (text) => setTx(text),
    selection: () => window.getSelection()?.toString() ?? '',
    rxText: getDigiText,
    appendRx: appendDigiNote,
    removeRxLines: removeDigiLines,
    setStart: (d) => setStartAt(d),
    qsoCall: () => live.current.qso.call,
    qsoGrid: () => live.current.qso.wwloc,
    macroNames: () => [...live.current.macros.keys()],
    runMacro: (name) => runEvent.current(name),
    view: (name, how) => {
      const keys: Record<string, keyof Views> = { CONTROLBAR: 'controlBar', LOGBAR: 'log', STATUSBAR: 'statusBar', TUNINGBAR: 'waterfall', WATERFALL: 'waterfall', SIGNALS: 'signals', wideWaterfall: 'wideWaterfall' };
      const key = keys[name];
      if (key) {
        const cur = live.current.views[key] === true;
        setView({ [key]: how === 'toggle' ? !cur : how === 'show' });
        return true;
      }
      if (name === 'ROTOR') {
        setRotorShown(how === 'toggle' ? !live.current.rotorShown : how === 'show');
        return true;
      }
      if (name === 'CONTESTSTATS' || name === 'QSOSTATISTIC' || name === 'QSOMETER') {
        setContestStats(how === 'toggle' ? (v) => !v : how === 'show');
        return true;
      }
      if ((name === 'WORLDMAP' || name === 'DXCLUSTER') && how !== 'hide') {
        window.location.hash = name === 'WORLDMAP' ? '#/mappa' : '#/mappa-dx';
        return true;
      }
      if (name === 'restartRx') {
        void digiRx('stop').then(() => digiRx('start')).then(onChanged, onChanged);
        return true;
      }
      return false;
    },
    extras: {
      add: () => addExtra(live.current.rxHz),
      closeAll: () => {
        for (const x of extras) clearExtraText(x.id);
        setExtras([]);
      },
      swapFirst: () => {
        const first = extras[0];
        if (first) swapExtra(first.id);
      },
    },
    setRxPaused: (p) => setRxPaused(p),
    rxStyle: (patch) => setRxStyle((cur) => ({ ...cur, ...patch })),
    setMacroMode: (m) => setMacroMode(m),
    openSettings: (tab) => openSettingsWindow(tab),
    setCapsOnly: (on) => setView({ capsOnly: on }),
    invalidateLog: () => {
      void client.invalidateQueries({ queryKey: ['log-qsos'] });
      void client.invalidateQueries({ queryKey: ['contest'] });
      void client.invalidateQueries({ queryKey: ['log4om'] });
    },
    ignoreBeforeErase: () => writeIgnoreBefore(browserStorage(), 'erase'),
  });
  const macroDepsRef = useRef(macroDeps);
  macroDepsRef.current = macroDeps;
  // <OnRX:testo,macro>: il testo nuovo della ricezione si confronta con le attese
  useEffect(() => {
    let seenLen = getDigiText().length;
    return onDigiText(() => {
      const text = getDigiText();
      const added = text.length >= seenLen ? text.slice(seenLen) : text;
      seenLen = text.length;
      if (macroState.onRx.length > 0) checkOnRx(macroDepsRef.current(), added);
    });
  }, []);

  // cambiando nominativo l'avviso del salvataggio rifiutato non vale piu'
  useEffect(() => {
    if (saveError && qso.call.trim().toUpperCase() !== saveError.call) setSaveError(null);
  }, [qso.call, saveError]);
  // con la frequenza della radio nota la banda scelta a mano non serve piu' (e non deve restare per quando il CAT si perde)
  const dialKnown = (status?.dial_hz ?? 0) > 0;
  useEffect(() => {
    if (dialKnown) setManualBand('');
  }, [dialKnown]);
  const saveQso = () => saveQsoThen();
  /** Registra il QSO; `after` gira solo a QSO salvato (la vista del contest SSB azzera i campi). */
  const saveQsoThen = (after?: () => void, failed?: () => void) => {
    const l = live.current;
    const call = l.qso.call.trim().toUpperCase();
    if (call === '') {
      toast(t('digi.run.noCall'), 'error');
      return;
    }
    setSaveError(null);
    const dial = l.status?.dial_hz ?? 0;
    const at = l.startAt ?? new Date();
    const sect = sentSectionRef.current;
    const nrs = composeSent(exchangeSent(l.qso.nrs || fixedNrsRef.current, l.serial), readSection(browserStorage(), sect.id), sect.applies && sect.id !== null);
    const rcvd = composeReceived(l.qso.nrr, l.qso.extra);
    // la riga del contest non ha Nome e QTH: se la ricerca del nominativo e' gia' in memoria, il QSO li porta lo stesso
    const known = l.qso.name === '' || l.qso.qth === '' ? knownCallsign(call) : undefined;
    const looked = known ? filledFrom(known) : null;
    void addLogQso({
      call,
      date: at.toISOString().slice(0, 10),
      time: at.toISOString().slice(11, 16),
      band: logBand(dial, l.manualBand),
      // la frequenza vera del segnale (radio + tono di ricezione, in USB), non solo la radio
      freq_hz: dial > 0 ? dial + Math.round(l.rxHz) : 0,
      mode: MODE_NAMES[l.snap.settings.mode],
      rst_sent: l.saved.rsts,
      rst_rcvd: l.saved.rstr,
      grid: l.qso.wwloc,
      name: l.qso.name || (looked?.name ?? ''),
      qth: l.qso.qth || (looked?.qth ?? ''),
      // il log non ha campi per gli scambi del contest: vanno in coda alle note (e a parte per il log del contest)
      comment: commentWithExchange(l.qso.notes, nrs, rcvd),
      exch_sent: nrs,
      exch_rcvd: rcvd,
    }, l.inContest).then(
      () => {
        // il QSO e' a log: il numero progressivo avanza (in gara lo da' il log del contest) e gli scambi si azzerano
        if (l.inContest) void client.invalidateQueries({ queryKey: ['contest'] });
        else setSerialState((n) => writeSerial(browserStorage(), n + 1));
        setQso((q) => ({ ...q, nrs: '', nrr: '', extra: '' }));
        setSaveError(null);
        toast(t('digi.run.saved', { call }), 'ok');
        void client.invalidateQueries({ queryKey: ['log-qsos'] });
        // il log di lato (Log4OM/Aurora) e la vista SSB lo rileggono subito, non dopo 10 s
        void client.invalidateQueries({ queryKey: ['log4om'] });
        void client.invalidateQueries({ queryKey: [...TEXT_CONTEST_SCORE_KEY] });
        after?.();
      },
      (e: unknown) => {
        // il motivo resta in vista accanto al nominativo (barra del log o riga del contest), oltre al messaggio che passa
        const msg = saveErrorText(e, t);
        setSaveError({ msg, call });
        toast(msg, 'error');
        failed?.();
      },
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
  const execute = (full: string) => {
    // <AUTOCQ>testo: il testo diventa la chiamata dell'Auto CQ, che si accende; si esegue subito solo quello che sta prima
    const { before: autoBefore, cq } = splitAutoCq(full);
    // <GRABCALL>: l'ultimo nominativo ricevuto va subito nel QSO, cosi' il <CALL> della stessa macro lo usa gia'; il comando resta nella
    // macro (con il nominativo trovato) per rimetterlo nel QSO al suo posto, anche dopo un <CLEARQSO>
    let script = autoBefore;
    let grabbed = '';
    if (macroNeeds(script, context().macros).grab) {
      grabbed = grabCall(getDigiText(), live.current.status?.de_call ?? '');
      if (grabbed !== '') live.current = { ...live.current, qso: { ...live.current.qso, call: grabbed } };
      script = script.replace(/<GRABCALL>/gi, `<GRABCALL:${grabbed}>`);
    }
    if (cq !== null) {
      autoCqScript.current = cq;
      setAutoCqPaused(false);
      setAutoCq(true);
      if (script.trim() === '') return;
    }
    const needs = macroNeeds(script, context().macros);
    // <CALL?>, <RSTS?>, <NRS?>...: i valori vuoti si chiedono prima di eseguire
    for (const f of needs.ask) {
      const l = live.current;
      const cur = { CALL: l.qso.call, RSTS: l.saved.rsts, RSTR: l.saved.rstr, NRS: l.qso.nrs ?? '', NRR: l.qso.nrr ?? '' }[f];
      if (cur.trim() !== '') continue;
      const v = window.prompt(t('digi.run.ask', { field: f }), f === 'RSTS' ? (l.snap.settings.mode === 'ssb' ? '59' : '599') : f === 'NRS' ? '001' : '');
      if (v === null) return;
      const value = v.trim();
      if (f === 'CALL') {
        live.current = { ...l, qso: { ...l.qso, call: value.toUpperCase() } };
        takeCall(value.toUpperCase());
      } else if (f === 'RSTS' || f === 'RSTR') {
        live.current = { ...l, saved: { ...l.saved, [f.toLowerCase()]: value } };
        setSavedField({ [f.toLowerCase()]: value });
      } else {
        live.current = { ...l, qso: { ...l.qso, [f.toLowerCase()]: value } };
        setQso((q) => ({ ...q, [f.toLowerCase()]: value }));
      }
    }
    const selection = needs.selection ? (window.getSelection()?.toString() ?? '') : '';
    if (needs.files.length === 0 && !needs.log && !needs.contest) {
      run(script, { selection, grabbed });
      return;
    }
    const call = live.current.qso.call.trim().toUpperCase();
    const dialNow = live.current.status?.dial_hz ?? 0;
    const bandNow = dialNow > 0 ? (bandOf(dialNow) ?? '') : '';
    const modeNow = MODE_NAMES[live.current.snap.settings.mode];
    void Promise.all([
      Promise.all(needs.files.map((n) => digiTextFile(n).then((text) => [n, text] as const, () => [n, ''] as const))),
      needs.log ? getLogQsos().then((r) => r.qsos, () => []) : Promise.resolve([]),
      needs.contest ? getTextContestCheck(call || '-', bandNow, modeNow).then((r) => r, () => null) : Promise.resolve(null),
    ]).then(([read, qsos, check]) => {
      const byTime = [...qsos].sort((a, b) => `${b.date}${b.time}`.localeCompare(`${a.date}${a.time}`));
      const mine = call !== '' ? byTime.filter((q) => q.call.toUpperCase() === call) : [];
      const same = mine.find((q) => q.band === bandNow && q.mode.toUpperCase() === modeNow);
      const msOf = (q: { date: string; time: string }) => Date.parse(`${q.date}T${q.time.length === 4 ? `${q.time.slice(0, 2)}:${q.time.slice(2)}` : q.time}Z`);
      const last = byTime[0];
      const exch = (c: string | undefined, k: 'NRS' | 'NRR') => new RegExp(`${k} (\\S+)`).exec(c ?? '')?.[1] ?? '';
      const prevQso: PrevQso | null = last ? { call: last.call, name: last.name ?? '', qth: last.qth ?? '', rsts: last.rst_sent, rstr: last.rst_rcvd, nrs: exch(last.comment, 'NRS'), nrr: exch(last.comment, 'NRR'), ms: msOf(last) } : null;
      const contestPrev = check ? { lastSent: check.ultimo_mandato ?? '', beforeMs: call !== '' ? (check.precedente?.time_ms ?? null) : null, lastCall: check.ultimo?.call ?? '', lastRst: check.ultimo?.rst_sent ?? '' } : null;
      run(script, {
        files: Object.fromEntries(read),
        lastQso: mine[0] ? { date: mine[0].date, band: mine[0].band, mode: mine[0].mode } : null,
        contestPrev,
        prevQso,
        before: same ? { ms: msOf(same), band: same.band, mode: same.mode } : null,
        selection,
        grabbed,
      });
    });
  };

  /**
   * Una macro nell'ordine dello script (vedi `planMacro`): i comandi subito, poi il testo prima di `<RX>` in trasmissione (o nella
   * casella), il testo dopo `<RX>` resta nella finestra, e a fine trasmissione il ritorno in ricezione e la pulizia.
   */
  const run = (script: string, extra: Pick<MacroContext, 'files' | 'lastQso' | 'contestPrev' | 'prevQso' | 'before' | 'selection' | 'grabbed'>) => {
    const l = live.current;
    const result = runMacro(script, { ...context(), ...extra });
    // in trasmissione anche se lo stato letto ogni secondo non lo dice ancora: una macro appena partita o testo ancora in coda.
    // Cosi' una macro premuta mentre trasmette butta subito il testo non partito e riparte da capo, invece di accodarsi.
    const busy = l.snap.tx.state === 'sending' || macroInflight.current > 0 || txLiveRef.current.txLive.progress.queued > 0;
    const plan = planMacro(result.steps, busy);
    const deps = macroDeps();
    const apply = (a: MacroAction) => {
      switch (a.type) {
        case 'clearRx': clearDigiText(); break;
        case 'saveQso': saveQso(); break;
        case 'qtc':
          if (textContest && textContest.contest.qtc > 0) setQtcOpen(a.dir);
          else toast(t('qtc.notWae'), 'info');
          break;
        case 'clearQso': setQso(EMPTY_QSO); setStartAt(null); resetRst(); break;
        case 'grabCall': if (a.call !== '') takeCall(a.call); break;
        case 'fixStartTime': setStartAt(new Date()); break;
        case 'mode': {
          if (macroState.modeLocked) {
            toast(t('digi.run.modeLocked'), 'info');
            break;
          }
          const named = oliviaModePatch(l.snap.settings, a.mode);
          const m = modeFromName(a.mode);
          if (named) save(named);
          else if (m) save({ mode: m });
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
          if (l.snap.settings.mode !== 'cw') save({ mode: 'cw', cw_wpm: next, cw_wpm_follow_rx: false });
          else save({ cw_wpm: next, cw_wpm_follow_rx: false });
          break;
        }
        case 'audioFq': if (!macroState.rxLocked) pickFreq(Math.max(200, Math.min(3500, a.relative ? l.rxHz + a.hz : a.hz)), false); break;
        case 'cursor': if (a.target === 'TX') txBox.current?.focus(); break;
        case 'beep': beep(); break;
        case 'autoCq':
          if (a.how === 'start' || (a.how === 'resume' && !autoCq)) {
            setAutoCqPaused(false);
            setAutoCq(true);
          } else if (autoCqPaused && autoCq) {
            setAutoCqPaused(false);
            toast(t('digi.run.autoCqOn', { s: autoCqDelay }), 'info');
          } else if (a.how === 'toggle') {
            setAutoCqPaused(false);
            setAutoCq((on) => !on);
          }
          break;
        case 'acqDelay': setAutoCqDelay(a.seconds); break;
        case 'jumpToTx': if (!macroState.rxLocked) pickFreq(l.snap.settings.tx_freq_hz, false); break;
        case 'align': if (!macroState.rxLocked) pickFreq(Math.max(200, Math.round(l.rxHz / a.hz) * a.hz), false); break;
        case 'seek': if (!macroState.rxLocked) seek(a.direction); break;
        case 'toggle': toggleSetting(a.what, a.value); break;
        case 'scan': setScanning(a.run); break;
        case 'memory': memory(a.write, a.slot); break;
        // la finestra dei segni del log di BIGScan fuori gara: BIGScan la rilegge al prossimo giro
        case 'ignoreBefore': writeIgnoreBefore(browserStorage(), a.at); break;
        case 'period': writePeriod(browserStorage(), a.minutes); break;
        case 'modeSettings':
        case 'threshold': setSettingsOpen(true); break;
        // i comandi CAT vanno in coda: <CATWAIT> li distanzia
        case 'cat': sendCat(deps, a.cmd); break;
        // la portante per accordare sulla frequenza di trasmissione (di base 10 s)
        case 'tune':
          void digiMacroPlay({ tune_s: a.seconds !== null && a.seconds > 0 ? a.seconds : 10 }).catch((e: unknown) => toast(t('digi.run.cmdFail', { tag: 'TUNE', why: e instanceof ApiError ? (e.message ?? e.code) : String(e) }), 'error'));
          break;
        case 'bookmark': mark(deps); break;
        // <MACROS:file>: Aurora tiene le macro nelle impostazioni; si ricorda una volta dove importarle
        case 'macros':
          if (!macroState.told.has('MACROS')) {
            macroState.told.add('MACROS');
            toast(t('digi.run.notInAurora', { tag: 'MACROS', what: t('digi.mi.macrosDefault') }), 'info');
          }
          break;
        case 'setField': {
          const f = a.field;
          if (f === 'call') takeCall(a.value);
          else if (f === 'rsts' || f === 'rstr') setSavedField({ [f]: a.value });
          else setQso((q) => ({ ...q, [f]: a.value }));
          break;
        }
        case 'cmd':
          if (a.tag === 'FORCELANGUAGE') forcedLang.current = a.arg;
          runCommand(a.tag, a.arg, deps);
          break;
        // EXEC ignorato in silenzio: Aurora non avvia programmi esterni (l'avviso a ogni macro disturbava, scelta del proprietario 2026-11-15)
        case 'exec': break;
        case 'unknown': break;
        // TX, RX e la pulizia della finestra di trasmissione li ordina planMacro: qui non arrivano
        case 'tx':
        case 'txe':
        case 'txToggle':
        case 'rx':
        case 'rxe':
        case 'clearTx':
        case 'clearWindow':
          break;
        default: {
          const never: never = a;
          return never;
        }
      }
    };
    plan.actions.forEach(apply);
    // <CATENABLEVFOSYNC>: a ogni macro il segnale torna al centro
    if (macroState.vfoSync) runCommand('CATVFOSYNCONCE', '', deps);
    if (plan.clearBox) clearTxWindow();
    const body = plan.text;
    // il testo dopo <RX>: resta nella finestra di trasmissione, dopo quello che parte
    const keepAfter = () => {
      if (plan.after !== '') setTx((v) => (v.trim() === '' ? plan.after : `${v.replace(/\s+$/, '')} ${plan.after}`));
    };
    const toTx = plan.tx === 'start';
    // una macro che trasmette, o premuta mentre si trasmette, ricomincia da capo: il testo non ancora partito si butta e parte questa
    // dall'inizio senza togliere il PTT; vince l'ultima pressione, e solo lei decide <RX> e la pulizia (vedi macroRestart)
    const transmitting = toTx || busy || plan.tx === 'stop';
    if (transmitting) {
      // un comando <TX> da solo trasmette quello che c'e' nella casella (se non e' stata appena pulita); mai una trasmissione vuota
      const what = body !== '' ? body : toTx && !plan.clearBox ? l.tx : '';
      // senza <RX> finito il testo si resta in trasmissione (si continua a scrivere); <TXTOGGLE> mentre si trasmette chiude
      // <BROADCASTMODE:ON>: anche con <RX> si resta in trasmissione finche' non si preme RX
      const open = plan.tx !== 'stop' && (!plan.rxAfter || macroState.broadcast);
      clearAfterTx.current = plan.clearAfter;
      const fire = () => {
        if (what.trim() === '') {
          if (busy && !open) {
            const r = macroSeq.current.run(() => digiTxClose());
            void r.done.then(onChanged, onChanged);
          }
          keepAfter();
          return;
        }
        void restartTx(what, open).then(({ ok, latest }) => {
          if (!latest) return;
          if (!ok) clearAfterTx.current = false;
          keepAfter();
        });
      };
      // <TXE>: si aspetta che cada il segnale sulla frequenza di ricezione (al massimo 30 s, poi si trasmette comunque)
      stopTxe();
      if (plan.waitCarrier && txeStep(l.snap.rx.carrier, 0, 0) === 'wait') {
        const startedAt = Date.now();
        toast(t('digi.run.txeWait'), 'info');
        txeTimer.current = window.setInterval(() => {
          if (txeStep(live.current.snap.rx.carrier, startedAt, Date.now()) === 'send') {
            stopTxe();
            fire();
          }
        }, 200);
      } else fire();
    } else {
      if (body !== '') setTx((v) => (plan.clearBox || v.trim() === '' ? body : `${v.replace(/\s+$/, '')} ${body}`));
      keepAfter();
    }
    // <RXC>: la finestra di trasmissione si pulisce quando il testo e' partito e si e' tornati in ricezione
    if (plan.clearAfter) {
      if (transmitting) {
        // riserva: se la pagina non vede mai «in trasmissione» (trasmissione cortissima), pulisce appena il core dice che non trasmette piu'
        const t0 = Date.now();
        // un solo intervallo alla volta: macro di fila non ne accumulano, e chiudendo la pagina si ferma (cleanup sotto)
        if (clearAfterTimer.current !== null) window.clearInterval(clearAfterTimer.current);
        const id = window.setInterval(() => {
          if (!clearAfterTx.current || Date.now() - t0 > 600_000) {
            window.clearInterval(id);
            if (clearAfterTimer.current === id) clearAfterTimer.current = null;
            return;
          }
          if (txeTimer.current !== null || macroInflight.current > 0 || wasSending.current || Date.now() - t0 < 2000) return;
          if (live.current.snap.tx.state !== 'sending') {
            clearAfterTx.current = false;
            window.clearInterval(id);
            if (clearAfterTimer.current === id) clearAfterTimer.current = null;
            clearTxWindow();
            window.setTimeout(clearTxWindow, 600);
          }
        }, 500);
        clearAfterTimer.current = id;
      } else clearTxWindow();
    }
  };

  const runMacroByName = (name: string): boolean => {
    const m = live.current.macros.get(name);
    if (!m || (m.label.trim() === '' && m.text.trim() === '')) return false;
    execute(m.text);
    return true;
  };
  // le macro evento: OnStart all'apertura della pagina, OnEnd alla chiusura, OnTX quando parte una trasmissione, OnCR con Invio nella
  // casella di trasmissione, OnClickAny quando una parola della ricezione va in un campo del log (dopo che il campo e' aggiornato)
  const runEvent = useRef(runMacroByName);
  runEvent.current = runMacroByName;
  const runEventSoon = (name: string) => window.setTimeout(() => runEvent.current(name), 50);
  useEffect(() => {
    runEvent.current('OnStart');
    return () => void runEvent.current('OnEnd');
  }, []);
  const wasSendingEv = useRef(sending);
  useEffect(() => {
    if (sending && !wasSendingEv.current) {
      txStartRef.current = new Date();
      runEvent.current('OnTX');
    }
    wasSendingEv.current = sending;
  }, [sending]);

  // i tasti F1-F12 con Ctrl e Shift, il «+» del tastierino (macro Gray+), Pausa (RX/TX) e Esc che ferma la trasmissione
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (editing !== null || settingsOpen || rttyOpen || macroGuide) return;
      if (e.key === 'Escape') {
        // Esc una volta ferma la trasmissione; due volte di seguito (entro mezzo secondo) spegne anche il CQ automatico (scelta del
        // proprietario: non resta in pausa, per ripartire si ripreme Auto CQ)
        const now = Date.now();
        const twice = now - lastEsc.current < 600;
        lastEsc.current = twice ? 0 : now;
        if (twice && autoCq) {
          setAutoCq(false);
          setAutoCqPaused(false);
          toast(t('digi.run.autoCqPaused'), 'info');
        }
        if (stopTxe()) toast(t('digi.run.txeCancel'), 'info');
        macroSeq.current.cancel();
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
    const fromMacro = live.current.macros.get('AutoCqText') ?? live.current.macros.get('F2');
    const m = autoCqScript.current !== null ? { text: autoCqScript.current } : fromMacro;
    if (!m || m.text.trim() === '') {
      toast(t('digi.run.autoCqNoMacro'), 'info');
      setAutoCq(false);
      return;
    }
    const timer = window.setTimeout(() => execute(m.text), autoCqDelay * 1000);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoCq, autoCqPaused, autoCqDelay, sending, snap.can_transmit, snap.tx.state]);

  // l'avviso del CQ automatico solo quando cambia, non all'apertura della pagina
  const autoCqFirst = useRef(true);
  useEffect(() => {
    if (autoCqFirst.current) {
      autoCqFirst.current = false;
      return;
    }
    // spento l'Auto CQ, la chiamata data da <AUTOCQ>testo si dimentica
    if (!autoCq) autoCqScript.current = null;
    toast(autoCq ? t('digi.run.autoCqOn', { s: autoCqDelay }) : t('digi.run.autoCqOff'), 'info');
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

  // trascinamento sulla barra: i due tasti si scambiano le macro in ogni insieme (vedi swapMacros); «Annulla» rimette com'era
  const moveMacro = (from: string, to: string) => {
    const before = { macros: s.macros, mode_macros: s.mode_macros };
    save(swapMacros(s, from, to));
    toastAction(t('digi.bar.moved'), t('digi.bar.undo'), () => save(before));
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

  // «Importa le macro di fldigi…»: si sceglie il file `.mdf`, si legge e si chiede dove metterle (niente cambia finche' non si conferma)
  const fldigiFile = useRef<HTMLInputElement | null>(null);
  const [fldigi, setFldigi] = useState<ImportReport | null>(null);
  const readFldigi = (file: File | undefined) => {
    if (!file) return;
    void file.text().then(
      (content) => {
        const parsed = parseFldigiFile(content);
        if (!parsed) {
          toast(t('fldigi.notFile'), 'error');
          return;
        }
        const report = buildImport(parsed);
        if (report.macros.length === 0) {
          toast(t('fldigi.empty'), 'info');
          return;
        }
        setFldigi(report);
      },
      () => toast(t('fldigi.notFile'), 'error'),
    );
  };
  const confirmFldigi = (where: 'all' | 'mode', how: 'add' | 'replace') => {
    if (!fldigi) return;
    const before = { macros: s.macros, mode_macros: s.mode_macros };
    save(applyImport(s, fldigi.macros, where, how));
    toastAction(t('fldigi.done', { n: String(fldigi.macros.length) }), t('digi.bar.undo'), () => save(before));
    setFldigi(null);
  };

  // la finestra di ricezione in un file di testo (cartella dei download)
  const saveRxText = () => {
    const url = URL.createObjectURL(new Blob([getDigiText().replace(/\r?\n/g, '\r\n')], { type: 'text/plain;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = `aurora-rx-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}.txt`;
    a.click();
    window.setTimeout(() => URL.revokeObjectURL(url), 1000);
  };

  // la parola scelta col tasto destro va nel campo del log del QSO in corso (il rapporto e' quello ricevuto) o negli appunti
  const pickWord = (target: WordTarget, word: string) => {
    if (target !== 'copy' && target !== 'clear') runEventSoon('OnClickAny');
    if (target === 'copy') {
      // il testo selezionato nella finestra di ricezione, altrimenti la parola cliccata
      const sel = window.getSelection();
      const picked = sel && rxBox.current && sel.anchorNode && rxBox.current.contains(sel.anchorNode) ? sel.toString() : '';
      const text = picked !== '' ? picked : word;
      if (text !== '') void navigator.clipboard?.writeText(text).catch(() => undefined);
    } else if (target === 'clear') clearDigiText();
    else if (target === 'rst') setSavedField({ rstr: word.toUpperCase() });
    else if (target === 'call') takeCall(word);
    else if (target === 'nrr') setQso((q) => ({ ...q, nrr: word.toUpperCase() }));
    else if (target === 'grid') setQso((q) => ({ ...q, wwloc: word.toUpperCase() }));
    else setQso((q) => ({ ...q, [target]: word }));
  };

  const copyRx = () => {
    void navigator.clipboard?.writeText(getDigiText()).catch(() => undefined);
  };

  // CW: la velocita' capita in ricezione (WPM interi, solo con l'ascolto acceso) e il comando che la usa per trasmettere
  const rxWpm = snap.rx.listening && snap.rx.wpm != null && Number.isFinite(snap.rx.wpm) ? Math.round(Math.min(60, Math.max(5, snap.rx.wpm))) : null;
  const txAtRx = () => {
    if (rxWpm != null) save({ cw_wpm: rxWpm });
  };
  // interruttore «TX = RX» acceso: a ogni nuova misura la velocita' di trasmissione la segue (se la misura manca resta l'ultima)
  const followWpm = s.mode === 'cw' ? followRxTarget(s.cw_wpm_follow_rx, rxWpm, s.cw_wpm) : null;
  useEffect(() => {
    if (followWpm != null) save({ cw_wpm: followWpm });
  }, [followWpm, save]);

  // ---- i menu ----
  // il menu Contest (dalla Vista): elenco dei contest, modifica di quello attivo, «Nessun contest» e i dati di quello attivo
  const contestMenu = (): MenuDef => {
    const c = textContest?.contest;
    const w = c ? contestWindow(c, Date.now()) : null;
    const fmt = (ms: number | null) => (ms == null ? '…' : new Date(ms).toISOString().slice(0, 16).replace('T', ' '));
    return {
      label: t('ctest.menu'),
      items: [
        { label: t('ctest.list'), onSelect: () => setContestList(true) },
        { label: t('ctest.editActive'), disabled: !c, onSelect: () => (c ? setContestEdit(c) : undefined) },
        { label: t('ctest.none'), checked: !c, onSelect: () => void selectTextContest(null) },
        'sep',
        { label: c ? t('ctest.infoActive', { name: c.name }) : t('ctest.infoNone'), disabled: true },
        ...(c
          ? [
              { label: w ? t('ctest.infoWindow', { from: fmt(w.from), to: fmt(w.to) }) : t('ctest.infoNoWindow'), disabled: true },
              { label: t('ctest.infoSerial', { nrs: c.auto_inc ? String(qsoSerial).padStart(3, '0') : c.nrs || '—' }), disabled: true },
              { label: t('ctest.infoMacros', { n: textContest?.macros.length ?? 0 }), disabled: true },
              ...(textScore ? [{ label: t('ctest.infoScore', { qso: textScore.qso, points: textScore.punti, mult: textScore.molt, score: textScore.punteggio }), disabled: true }] : []),
              'sep' as const,
              { label: t('ctest.stats'), onSelect: () => setContestStats(true) },
              ...(c.qtc > 0 ? [{ label: t('qtc.menu'), onSelect: () => setQtcOpen('send') }] : []),
              { label: t('ctest.cabrillo'), onSelect: () => void exportCabrillo() },
            ]
          : []),
      ],
    };
  };
  const slots = Array.from({ length: 20 }, (_, i) => i + 1);
  const audioInputMenu = useAudioInputMenu();
  const menus: MenuDef[] = [
    {
      label: t('digi.menu.file'),
      items: [
        { label: t('digi.mi.macrosMerge'), checked: s.merge_macros[s.mode] !== false, onSelect: () => save({ merge_macros: { ...s.merge_macros, [s.mode]: s.merge_macros[s.mode] === false } }) },
        'sep',
        { label: t('digi.mi.macrosFldigi'), onSelect: () => fldigiFile.current?.click() },
        { label: t('digi.mi.macrosReset'), onSelect: resetMacros },
      ],
    },
    {
      label: t('digi.menu.edit'),
      items: [
        { label: t('digi.mi.clearRx'), onSelect: () => clearDigiText() },
        { label: t('digi.mi.clearTx'), onSelect: clearTxWindow },
        'sep',
        { label: t('digi.mi.copyRx'), onSelect: copyRx },
      ],
    },
    {
      label: t('digi.menu.mode'),
      items: [
        ...snap.modes.map((m) => ({
          label: m.name,
          checked: s.mode === m.id,
          onSelect: () => {
            // la finestra «Impostazioni RTTY» si apre solo dalla scritta RTTY nella barra in basso, non scegliendo il modo
            if (s.mode !== m.id) save({ mode: m.id as DigiSettings['mode'] });
          },
        })),
        'sep' as const,
        { label: t('nav.sstv'), onSelect: () => { window.location.hash = '#/sstv'; } },
        { label: t('nav.wefax'), onSelect: () => { window.location.hash = '#/wefax'; } },
        // RTTY: spostamento e velocita' Baudot scelti dal menu (e nelle Impostazioni del modo)
        ...(s.mode === 'rtty'
          ? [
              'sep' as const,
              { label: `${t('digi.rtty.shift')}: ${s.rtty_shift_hz}`, items: withCurrent(RTTY_SHIFTS, s.rtty_shift_hz).map((v) => ({ label: `${v} Hz`, checked: v === s.rtty_shift_hz, onSelect: () => save({ rtty_shift_hz: v }) })) },
              { label: `${t('digi.rtty.baud')}: ${roundBaud(s.rtty_baud)}`, items: withCurrent(RTTY_BAUDS, roundBaud(s.rtty_baud)).map((v) => ({ label: `${v} baud`, checked: v === roundBaud(s.rtty_baud), onSelect: () => save({ rtty_baud: v }) })) },
              { label: t('digi.rtty.reverse'), checked: s.rtty_reverse, onSelect: () => save({ rtty_reverse: !s.rtty_reverse }) },
              { label: t('digi.rtty.uos'), checked: s.rtty_uos, onSelect: () => save({ rtty_uos: !s.rtty_uos }) },
              { label: t('digi.rtty.diddle'), checked: s.rtty_diddle, onSelect: () => save({ rtty_diddle: !s.rtty_diddle }) },
            ]
          : []),
        // CW: banda del filtro scelta al volo (anche larga, per la panoramica) e trasmissione alla velocita' ricevuta
        ...(s.mode === 'cw'
          ? [
              'sep' as const,
              { label: `${t('digi.cw.bandwidth')}: ${s.cw_bandwidth_hz} Hz`, items: withCurrent(CW_BANDWIDTHS, s.cw_bandwidth_hz).map((v) => ({ label: v >= CW_WIDE_HZ ? `${v} Hz, ${t('digi.cw.wide')}` : `${v} Hz`, checked: v === s.cw_bandwidth_hz, onSelect: () => save({ cw_bandwidth_hz: v }) })) },
              { label: rxWpm != null ? t('digi.mi.txAtRx', { wpm: rxWpm }) : t('digi.mi.txAtRxNone'), title: t('digi.mi.txAtRxTip'), disabled: rxWpm == null, onSelect: txAtRx },
            ]
          : []),
        // Olivia e Contestia: toni e banda scelti dal menu (e nelle Impostazioni del modo)
        ...(isOliviaMode(s.mode)
          ? (() => {
              const o = oliviaOf(s, s.mode);
              return [
                'sep' as const,
                { label: `${t('digi.olivia.tones')}: ${o.tones}`, title: t('digi.olivia.tonesTip'), items: OLIVIA_TONES.map((v) => ({ label: String(v), checked: v === o.tones, onSelect: () => saveOlivia({ tones: v }) })) },
                { label: `${t('digi.olivia.bandwidth')}: ${o.bandwidth_hz} Hz`, title: t('digi.olivia.bandwidthTip'), items: OLIVIA_BANDWIDTHS.map((v) => ({ label: `${v} Hz`, checked: v === o.bandwidth_hz, onSelect: () => saveOlivia({ bandwidth_hz: v }) })) },
              ];
            })()
          : []),
        'sep' as const,
        ...MODI_IN_ARRIVO.map((n) => ({ label: `${n} ${t('digi.mi.soon')}`, disabled: true })),
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
        { label: t('digi.mi.passband'), checked: s.passband_on === true, onSelect: () => save({ passband_on: s.passband_on !== true }) },
        { label: t('digi.notch.clear'), disabled: notches.length === 0, onSelect: () => setNotches([]) },
        { label: t('digi.mi.lock'), checked: lock, onSelect: () => toggleSetting('lock', 'toggle') },
        { label: t('digi.mi.snap'), checked: s.snap_on, onSelect: () => toggleSetting('snap', 'toggle') },
        { label: t('digi.mi.squelch'), checked: s.squelch_on, onSelect: () => toggleSetting('squelch', 'toggle') },
        { label: t('digi.mi.threshold'), onSelect: () => setSettingsOpen(true) },
        // tutta la banda (CW e PSK): acceso di base, e gli spot locali verso il pannello del DX Cluster di Aurora
        ...(radarMode || isPsk
          ? [
              'sep' as const,
              radarMode
                ? { label: t('radar.mi'), checked: radarOn, onSelect: toggleRadar }
                : { label: t('digi.band.miPsk'), checked: s.band_psk !== false, onSelect: () => save({ band_psk: s.band_psk === false }) },
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
        // Auto CQ come nei programmi di contest: avvia, ferma, testo della chiamata (macro AutoCqText) e pausa fra due chiamate
        {
          label: t('digi.mi.autoCq'),
          items: [
            { label: t('digi.mi.autoCqStart'), disabled: autoCq && !autoCqPaused, onSelect: () => { setAutoCqPaused(false); setAutoCq(true); } },
            { label: t('digi.mi.autoCqStop'), disabled: !autoCq, onSelect: () => { setAutoCq(false); setAutoCqPaused(false); } },
            { label: t('digi.mi.autoCqText'), onSelect: () => setEditing({ name: 'AutoCqText', scope: (s.mode_macros[s.mode] ?? []).some((m) => m.name === 'AutoCqText') ? 'mode' : 'all' }) },
            { label: t('digi.mi.autoCqDelay'), onSelect: () => setAcqDelayOpen(true) },
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
        { label: t('digi.mi.log'), checked: views.log, onSelect: () => setView({ log: !views.log }) },
        { label: t('digi.mi.rotor'), checked: rotorShown, onSelect: () => setRotorShown(!rotorShown) },
        { label: t('digi.mi.waterfall'), checked: views.waterfall, onSelect: () => setView({ waterfall: !views.waterfall }) },
        ...(radarMode ? [{ label: t('radar.mi'), checked: radarOn, onSelect: toggleRadar }] : []),
        { label: t('digi.band.view'), checked: views.signals, onSelect: () => setView({ signals: !views.signals }) },
        { label: t('digi.band.wfCalls'), checked: views.wfCalls, onSelect: () => setView({ wfCalls: !views.wfCalls }) },
        { label: t('digi.band.wfWords'), checked: views.wfWords !== false, onSelect: () => setView({ wfWords: views.wfWords === false }) },
        ...(radarMode ? [{ label: t('rbn.show'), checked: views.rbn !== false, onSelect: () => setView({ rbn: views.rbn === false }) }] : []),
        { label: t('digi.mi.statusBar'), checked: views.statusBar, onSelect: () => setView({ statusBar: !views.statusBar }) },
        'sep' as const,
        { label: t('digi.mi.echoRx'), checked: views.echoRx === true, onSelect: () => setView({ echoRx: !(views.echoRx === true) }) },
        { label: t('digi.mi.underlineSent'), checked: views.underlineSent, onSelect: () => setView({ underlineSent: !views.underlineSent }) },
        { label: t('digi.mi.capsOnly'), checked: views.capsOnly, onSelect: () => setView({ capsOnly: !views.capsOnly }) },
        { label: t('digi.mi.slashedZero'), checked: views.slashedZero, onSelect: () => setView({ slashedZero: !views.slashedZero }) },
        { label: t('digi.mi.flashTx'), checked: views.flashTx, onSelect: () => setView({ flashTx: !views.flashTx }) },
        { label: t('digi.mi.hairlines'), checked: views.hairlines, onSelect: () => setView({ hairlines: !views.hairlines }) },
        { label: t('digi.mi.wideWaterfall'), checked: views.wideWaterfall, onSelect: () => setView({ wideWaterfall: !views.wideWaterfall }) },
        { label: t('digi.mi.anticlick'), title: t('digi.mi.anticlickTip'), checked: s.wf_anticlick, onSelect: () => save({ wf_anticlick: !s.wf_anticlick }) },
        {
          label: t('digi.mi.macroRows'),
          items: [
            ...[1, 2, 3, 4].map((n) => ({ label: String(n), checked: macroRows === n || (macroRows === 0 && n === 4), onSelect: () => { setMacroRows(n); setView({ extended: n > 1 }); } })),
            { label: t('digi.mi.macroRowsAuto'), title: t('digi.mi.macroRowsAutoTip'), checked: macroRows === 5, onSelect: () => { setMacroRows(5); setView({ extended: true }); } },
          ],
        },
        { label: t('digi.mi.defaultRst'), checked: views.defaultRst, onSelect: () => setView({ defaultRst: !views.defaultRst }) },
      ],
    },
    contestMenu(),
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
        'sep',
        audioInputMenu,
      ],
    },
    {
      label: t('digi.menu.help'),
      items: [
        { label: t('digi.mi.guide'), onSelect: () => { location.hash = '#/guida'; } },
        { label: t('digi.mi.macroGuide'), onSelect: () => setMacroGuide(true) },
      ],
    },
  ];

  // le misure del segnale per la barra: S/N in tutti i modi, IMD nel PSK (durante le inversioni), velocita' capita nel CW
  const measures = [
    isOliviaMode(s.mode)
      ? snap.rx.snr_db != null ? t('digi.olivia.syncSnr', { db: Math.round(snap.rx.snr_db) }) : t('digi.olivia.syncSnrNone')
      : snap.rx.snr_db != null ? `S/N ${Math.round(snap.rx.snr_db)} dB` : 'S/N —',
    s.mode === 'psk31' || s.mode === 'psk63' ? (snap.rx.imd_db != null ? `IMD ${Math.round(snap.rx.imd_db)} dB` : 'IMD —') : null,
    s.mode === 'cw' && snap.rx.wpm != null ? `${Math.round(snap.rx.wpm)} WPM` : null,
  ]
    .filter((v) => v !== null)
    .join(' · ');
  const modeData = s.mode === 'rtty' ? `${Math.round(s.rtty_baud)} bd, ${Math.round(s.rtty_shift_hz)} Hz` : s.mode === 'cw' ? `${Math.round(s.cw_wpm)} WPM` : isOliviaMode(s.mode) ? `${oliviaLabel(oliviaOf(s, s.mode))} Hz` : s.mode === 'psk63' ? '63 bd' : '31 bd';

  // la colonna DUPE / Mult del log nel contest SSB: doppi dal log, moltiplicatori nuovi dalle statistiche del contest
  // entrando in SSB i rapporti di partenza sono 59 (non 599 dei modi digitali)
  useEffect(() => {
    if (ssbContest && (live.current.saved.rsts === '599' || live.current.saved.rstr === '599')) setSavedField({ rsts: live.current.saved.rsts === '599' ? '59' : live.current.saved.rsts, rstr: live.current.saved.rstr === '599' ? '59' : live.current.saved.rstr });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ssbContest]);
  // in un contest con CW e fonia (ARRL 10) passando a CW il rapporto di partenza torna 599: la riga del contest non mostra il campo RST
  useEffect(() => {
    if (!contestSheets || ssbContest) return;
    const sv = live.current.saved;
    if (sv.rsts === '59' || sv.rstr === '59') setSavedField({ rsts: sv.rsts === '59' ? '599' : sv.rsts, rstr: sv.rstr === '59' ? '599' : sv.rstr });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [contestSheets, ssbContest]);
  // il log del contest parte da zero: contano i QSO nella finestra di date del contest (se c'e' ed e' valida), altrimenti quelli dopo l'attivazione,
  // sempre della categoria giusta; doppi, moltiplicatori e numerazione si calcolano solo su questi
  const sinceKey = textContest ? `aurora.ssb.contest.since.${textContest.contest.id}` : null;
  const [since, setSince] = useState<number | null>(null);
  useEffect(() => {
    if (!sinceKey) {
      setSince(null);
      setContestSince(null);
      return;
    }
    let v = Number(browserStorage()?.getItem(sinceKey) ?? '');
    if (!Number.isFinite(v) || v <= 0) {
      v = Date.now();
      try {
        browserStorage()?.setItem(sinceKey, String(v));
      } catch {
        // l'attivazione vale comunque finche' la pagina e' aperta
      }
    }
    setSince(v);
    setContestSince(v);
    void client.invalidateQueries({ queryKey: [...TEXT_CONTEST_SCORE_KEY] });
  }, [sinceKey]);
  // lo stato della finestra (prima, dentro, scaduta) si riguarda ogni mezzo minuto
  const [winTick, setWinTick] = useState(0);
  useEffect(() => {
    if (!contestSheets) return;
    const id = window.setInterval(() => setWinTick((n) => n + 1), 30_000);
    return () => window.clearInterval(id);
  }, [contestSheets]);
  const winStatus = useMemo(() => (textContest ? contestWindowStatus(textContest.contest, Date.now()) : null), [textContest, winTick]);
  /** La finestra di date c'e' e vale adesso: il log si filtra per date. Prima dell'inizio, scaduta o senza date si usa l'attivazione (e la vista lo avvisa). */
  const windowUsable = winStatus !== null && winStatus.status === 'active';
  const [logOnly, setLogOnlyState] = useState<boolean>(() => browserStorage()?.getItem('aurora.ssb.logonly') !== '0');
  const toggleLogOnly = () => {
    setLogOnlyState((v) => {
      try {
        browserStorage()?.setItem('aurora.ssb.logonly', v ? '0' : '1');
      } catch {
        // la scelta vale comunque finche' la pagina e' aperta
      }
      return !v;
    });
  };
  const inContestSet = useMemo(() => {
    const c = textContest?.contest;
    if (!c) return () => false;
    const w = windowUsable ? (winStatus?.window ?? null) : null;
    return (q: Log4omQso) => {
      const ms = Date.parse(`${q.date.replace(' ', 'T').slice(0, 19)}Z`);
      if (!Number.isFinite(ms) || !categoryValid(c, q.mode)) return false;
      if (w) return (w.from == null || ms >= w.from) && (w.to == null || ms <= w.to);
      return since !== null && ms >= since;
    };
  }, [textContest, windowUsable, winStatus, since]);
  const logMarks: LogMarks | undefined = textContest ? { noBand: textContest.contest.dupe_no_band, noMode: textContest.contest.dupe_no_mode, recenti: textScore?.recenti ?? [], inSet: inContestSet, only: logOnly, onToggle: toggleLogOnly } : undefined;
  /** Il log di lato (o, nel contest SSB, solo il Log o solo il DX Cluster). */
  const digiLog = (view?: 'log' | 'cluster') => (
    <DigiLog
      view={view}
      marks={view === 'log' ? logMarks : undefined}
      width={logW}
      onWidth={(px) => {
        const w = Math.round(Math.min(Math.max(px, 300), Math.max(320, window.innerWidth - 320)));
        setLogW(w);
        try {
          writeSize('aurora.digi.logw', sizeMode, w);
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
      saveError={saveError?.msg ?? null}
      onDismissSaveError={() => setSaveError(null)}
      serial={qsoSerial}
      onSerial={setSerial}
      contestOn={inContest || textContest !== null}
      onClear={() => {
        setQso(EMPTY_QSO);
        setStartAt(null);
        resetRst();
      }}
    />
  );

  /** «Registra» della riga del contest (SSB, CW, RTTY, PSK): campi azzerati subito e rapporti di partenza (59 in SSB, 599 negli altri modi); se il log non salva si rimettono. */
  const registerContestQso = () => {
    if (qso.call.trim() === '') return saveQso();
    const prev = { qso, startAt, rsts: saved.rsts, rstr: saved.rstr };
    saveQsoThen(() => undefined, () => {
      setQso(prev.qso);
      setStartAt(prev.startAt);
      setSavedField({ rsts: prev.rsts, rstr: prev.rstr });
    });
    setQso(EMPTY_QSO);
    setStartAt(null);
    const rst = ssbContest ? '59' : '599';
    setSavedField({ rsts: rst, rstr: rst });
  };
  // i tre pezzi della pagina dei modi testuali (ricezione, trasmissione, cascata): stessi elementi nella pagina di sempre e nelle schede del contest
  const rxPart = (
    <>
    {/* barra sopra la finestra RX: posto per i pulsantini */}
    <div className="digi-rxbar" role="toolbar" aria-label={t('digi.rx.title')}>
      <button type="button" className="digi-rxbar-btn" title={t('digi.rxbar.copy')} aria-label={t('digi.rxbar.copy')} onClick={() => void navigator.clipboard?.writeText(getDigiText()).catch(() => undefined)}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M9 9h11v11H9zM5 15H4V4h11v1" /></svg>
      </button>
      <button type="button" className="digi-rxbar-btn" title={t('digi.rxbar.save')} aria-label={t('digi.rxbar.save')} onClick={saveRxText}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 3v12M7 10l5 5 5-5M5 21h14" /></svg>
      </button>
      <button type="button" className="digi-rxbar-btn" title={t('digi.mi.clearRx')} aria-label={t('digi.mi.clearRx')} onClick={() => clearDigiText()}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14M10 11v6M14 11v6" />
        </svg>
      </button>
      <button type="button" className="digi-rxbar-btn" title={t('digi.rxbar.pause')} aria-label={t('digi.rxbar.pause')} aria-pressed={rxPaused} onClick={() => setRxPaused((v) => !v)}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M9 5v14M15 5v14" /></svg>
      </button>
      <button type="button" className="digi-rxbar-btn" title={t('digi.rxbar.search')} aria-label={t('digi.rxbar.search')} aria-pressed={searchOpen} onClick={() => (searchOpen ? closeSearch() : setSearchOpen(true))}>
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6" /><path d="M15 15l5 5" /></svg>
      </button>
      {searchOpen ? (
        <>
          <input
            ref={searchBox}
            type="search"
            className="digi-rxbar-search"
            value={search}
            placeholder={t('digi.rxbar.searchHint')}
            aria-label={t('digi.rxbar.searchHint')}
            title={t('digi.rxbar.searchTip')}
            spellCheck={false}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => {
              // i tasti restano alla casella: Esc chiude la ricerca e non ferma la trasmissione
              e.stopPropagation();
              if (e.key === 'Enter') {
                e.preventDefault();
                jumpHit(e.shiftKey);
              } else if (e.key === 'Escape') {
                e.preventDefault();
                closeSearch();
              }
            }}
          />
          {search.trim() !== '' ? <span className="digi-rxbar-count" aria-live="polite">{hit.count === 0 ? '0' : `${hit.at >= 0 ? hit.at + 1 : '-'}/${hit.count}`}</span> : null}
        </>
      ) : null}
    </div>
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
        if (word && isCallsign(word)) {
          takeCall(word);
          runEventSoon('OnClickAny');
        }
      }}>
      <DigiRxLive box={rxBox} underlineSent={views.underlineSent} showSent={views.echoRx === true} paused={rxPaused} search={searchOpen ? search : ''} />
    </pre>
    </>
  );
  const txPart = (
    <div
      className="digi-txwrap"
      data-label={t('digi.tx.title')}
      style={{ flex: `${1 - rxFrac} 1 0` }}
      onContextMenu={(e) => {
        // tasto destro nella finestra di trasmissione: Pulisci tutto, Taglia, Copia, Incolla
        e.preventDefault();
        setTxMenu({ x: e.clientX, y: e.clientY });
      }}
    >
      {txMenu ? <TxMenu at={txMenu} box={txBox.current} text={tx} onText={txEdit} onClearAll={clearTxWindow} onClose={() => setTxMenu(null)} /> : null}
      {txSent !== '' ? (
        <div ref={txSentBox} className={`digi-txhist${views.slashedZero ? ' digi-zero' : ''}`} aria-label={t('digi.tx.sentTitle')} title={t('digi.tx.sentTitle')}>
          {txSent}
        </div>
      ) : null}
      <DigiTxBox
        boxRef={txBox}
        className={`digi-txwin${views.slashedZero ? ' digi-zero' : ''}${views.capsOnly ? ' digi-caps' : ''}`}
        value={tx}
        progress={txLive.progress}
        sending={sending}
        label={t('digi.tx.title')}
        placeholder={snap.tx.open ? t('digi.tx.openHint') : s.mode === 'ssb' ? t('digi.tx.ssb') : undefined}
        onText={txEdit}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !sending) send(tx, true);
          else if (e.key === 'Enter' && !e.shiftKey && !e.altKey) runEventSoon('OnCR');
        }}
      />
      {!snap.can_transmit ? <p className="notice">{t('digi.tx.notReady')}</p> : null}
      {problem ? (
        <p className="notice sstv-bad" role="alert">
          {problem}
        </p>
      ) : null}
    </div>
  );
  /** La cascata; con `cardHeight` (dentro una scheda) prende quell'altezza e niente maniglie di misura: la misura la da' la scheda. */
  const waterfallFor = (cardHeight?: number) => (
    <DigiWaterfall height={cardHeight ?? wfHeight} onHeight={cardHeight === undefined ? changeWfHeight : undefined} widthPx={cardHeight === undefined ? wfWidth : null} onWidth={cardHeight === undefined ? changeWfWidth : undefined} minHz={WF_MIN} maxHz={views.wideWaterfall ? WF_MAX_WIDE : WF_MAX_NARROW} rxHz={rxHz} txHz={lock ? s.tx_freq_hz : null} dialHz={status?.dial_hz ?? 0} bandwidthHz={s.mode === 'rtty' ? s.rtty_shift_hz : s.mode === 'cw' ? s.cw_bandwidth_hz : 0} passbandHz={s.passband_on === true ? passbandWidth(s) : 0} split={lock} hairlines={views.hairlines} transmitting={sending} onRow={onRow} onPick={(hz, ctrl, commit = true) => pickFreq(hz, true, ctrl, commit)} labels={views.wfCalls ? wfLabels : []} showWords={views.wfWords !== false} notches={notches} onNotches={setNotches}
      extraRx={extras}
      onExtraMove={(id, hz, commit) => {
        const next = extras.map((x) => (x.id === id ? { ...x, freq_hz: hz } : x));
        if (commit) {
          clearExtraText(id);
          setExtras(next);
        } else client.setQueryData(['digi-extra-rx'], next);
      }}
      onMenu={(hz, x, y) => setWfMenu({ x, y, hz })}
    />
  );

  return (
    <>
      <MenuBar menus={applyAvailability(menus, s.mode, t as never, t('digi.mi.onlyDigital'))} />
      <input
        ref={fldigiFile}
        type="file"
        hidden
        accept=".mdf,.txt,text/plain"
        onChange={(e) => {
          readFldigi(e.target.files?.[0]);
          e.target.value = '';
        }}
      />
      {fldigi ? <FldigiImportDialog report={fldigi} modeLabel={s.mode.toUpperCase()} onConfirm={confirmFldigi} onCancel={() => setFldigi(null)} /> : null}
      {armed ? null : <ListenGate onListen={() => setArmed(true)} />}

      {/* macro a sinistra (colonne fisse e piu' corte), a fianco il selettore di banda con le frequenze del piano IARU */}
      <div className={`digi-topline${ssbContest ? ' digi-topline--ssb' : ''}${ssbContest || textContest !== null ? ' digi-topline--contest' : ''}`}>
      {views.controlBar ? (
        <div className={`digi-macrowrap${macroW ? ' digi-macrowrap--sized' : ''}`} style={macroW ? { width: macroW } : undefined} ref={macroBox}>
          <ControlBar
            macros={macros}
            extended={views.extended && macroRows !== 1}
            onSetExtended={(multi) => {
              // «Barra multipla» si apre subito a 4 righe; «Barra singola» ne mostra una
              setView({ extended: multi });
              if (multi) setMacroRows(4);
              else if (macroRows !== 1) setMacroRows(0);
            }}
            rows={macroRows === 5 ? 'auto' : macroRows > 0 ? macroRows : 4}
            onRun={(m) => execute(m.text)}
            onMove={moveMacro}
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
                clearSize('aurora.digi.macrow', sizeMode);
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
                  writeSize('aurora.digi.macrow', sizeMode, last);
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
      {/* con un contest che ha PIU' modi (per esempio CW e SSB) i pulsanti diretti dei modi stanno sotto le bande; con un modo solo non compaiono */}
      {/* la radio si porta sulla frequenza scelta (non in SSB: il piano dei modi testuali non vale per la fonia) */}
      {/* contest in SSB: a destra delle macro un blocchetto per banda (le bande scelte nelle Impostazioni, hook useBandFilter), un clic porta la radio all'inizio del tratto fonia */}
      {ssbContest || textContest !== null ? (
        <div className="ssbc-bands" role="group">
          {(() => {
            // le bande del contest (dalle sue impostazioni: `[bande]` della cartella o `bands=` delle regole) e quelle scelte nelle Impostazioni della stazione:
            // si mostrano le scelte che il contest ammette; se non ce n'e' nessuna, quelle del contest
            const ammesse = textContest ? (bandeAmmesse(textContest.contest) ?? []).map((b) => b.replace(/m$/, '')) : [];
            const inContest = (band: string) => ammesse.length === 0 || ammesse.includes(band.replace(/m$/, ''));
            const scelte = visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null).flatMap((b) => {
              const row = bandPlanRows(contestPlan).find((r) => r.band === b);
              return row && inContest(row.band) ? [row] : [];
            });
            const rows = scelte.length > 0 || ammesse.length === 0 ? scelte : bandPlanRows(contestPlan).filter((r) => inContest(r.band));
            if (rows.length === 0)
              return (
                <p className="ssbc-bands-none">
                  {t('ssbc.noBands')}{' '}
                  <a href="#/impostazioni" onClick={(e) => { e.preventDefault(); openSettingsWindow('station'); }}>
                    {t('ssbc.noBandsLink')}
                  </a>
                </p>
              );
            return rows.map((row) => (
              <button
                key={row.band}
                type="button"
                className={`ssbc-band${status && status.dial_hz > 0 && bandOf(status.dial_hz) === row.band ? ' on' : ''}`}
                aria-pressed={!!(status && status.dial_hz > 0 && bandOf(status.dial_hz) === row.band)}
                disabled={sending}
                title={`${row.band} · ${(row.hz / 1e6).toFixed(3).replace('.', ',')} MHz`}
                onClick={() => {
                  setManualBand(row.band);
                  void postFrequency(row.hz).then(onChanged, onChanged);
                }}
              >
                <b>{row.band.replace(/m$/, '')}</b>
                <small>m</small>
              </button>
            ));
          })()}
          {contestModeButtons()}
        </div>
      ) : null}
      {s.mode !== 'ssb' && textContest === null && (
        <div className="digi-bandbar">
          <FreqPicker
            bands={visibleBands(chosenBands, status && status.dial_hz > 0 ? bandOf(status.dial_hz) : null)}
            dialHz={status?.dial_hz ?? 0}
            mode={s.mode.toUpperCase()}
            rows={bandPlanRows(s.mode === 'rtty' ? 'RTTY' : s.mode === 'cw' ? 'CW' : s.mode === 'psk63' ? 'PSK63' : isOliviaMode(s.mode) ? 'OLIVIA' : 'PSK31')}
            locked={sending}
            onPick={(hz) => {
              setManualBand(bandOf(hz) ?? '');
              void postFrequency(hz).then(onChanged, onChanged);
            }}
          />
        </div>
      )}
      {sending ? (
        <div className={`digi-txflag${views.flashTx ? ' digi-txflag--flash' : ''}`} role="status" aria-label="TX">
          TX
        </div>
      ) : null}
      </div>
      {textContest !== null ? (
        <ContestWorkspace
          view={ssbContest ? 'ssb' : 'text'}
          contest={textContest.contest}
          score={textScore}
          qso={qso}
          rsts={saved.rsts}
          rstr={saved.rstr}
          qsoNr={(textScore?.qso ?? 0) + (textScore?.doppi ?? 0) + 1}
          inSet={inContestSet}
          noBand={textContest.contest.dupe_no_band}
          noMode={textContest.contest.dupe_no_mode}
          windowNotice={!windowUsable}
          onEditWindow={() => setContestEdit(textContest.contest)}
          nrsAuto={fixedNrsRef.current !== '' ? fixedNrsRef.current : formatSerial(qsoSerial)}
          onQso={(patch) => setQso({ ...qso, ...patch })}
          onRst={(patch) => setSavedField(patch)}
          onRegister={registerContestQso}
          band={logBand(status?.dial_hz ?? 0, manualBand)}
          saveError={saveError?.msg ?? null}
          onDismissSaveError={() => setSaveError(null)}
          modeName={MODE_NAMES[s.mode]}
          showLog={views.log}
          log={digiLog('log')}
          cluster={digiLog('cluster')}
          rx={ssbContest ? undefined : rxPart}
          tx={ssbContest ? undefined : txPart}
          // la cascata e' una scheda come le altre e segue la scelta del menu Vista (in SSB non c'e')
          waterfall={ssbContest || !views.waterfall ? undefined : waterfallFor}
          bigscan={radarMode && radarOn ? (<RadarList on={radarOn} onToggle={toggleRadar} answering={views.radarAnswering} onAnswering={(v) => setView({ radarAnswering: v })} stations={radar} showWpm={s.mode === 'cw'} showSnr={s.mode === 'cw'} dialHz={status?.dial_hz ?? 0} rxHz={rxHz} modeName={s.mode === 'cw' ? 'CW' : 'RTTY'} onTune={(hz) => pickFreq(hz, false)} onCall={takeCall} qsoCall={qso.call} onPileupTx={(hz) => save({ tx_follow_rx: false, tx_freq_hz: Math.max(200, Math.min(3500, Math.round(hz))) })} />) : undefined}
          rotor={(at) => (rotorShown ? <RotorCard call={qso.call} grid={qso.wwloc} place="digi" startAt={at} /> : null)}
        />
      ) : ssbContest ? (
        <SsbContestView
          qso={qso}
          rsts={saved.rsts}
          rstr={saved.rstr}
          qsoNr={(textScore?.qso ?? 0) + (textScore?.doppi ?? 0) + 1}
          contest={null}
          inSet={inContestSet}
          noBand={false}
          noMode={false}
          windowNotice={false}
          onEditWindow={() => undefined}
          nrsAuto={fixedNrsRef.current !== '' ? fixedNrsRef.current : formatSerial(qsoSerial)}
          onQso={(patch) => setQso({ ...qso, ...patch })}
          onRst={(patch) => setSavedField(patch)}
          onRegister={registerContestQso}
          band={logBand(status?.dial_hz ?? 0, manualBand)}
          saveError={saveError?.msg ?? null}
          onDismissSaveError={() => setSaveError(null)}
          modeName={MODE_NAMES[s.mode]}
          contestName={null}
          score={undefined}
          showLog={views.log}
          log={digiLog('log')}
          cluster={digiLog('cluster')}
          rotor={(at) => (rotorShown ? <RotorCard call={qso.call} grid={qso.wwloc} place="digi" startAt={at} /> : null)}
        />
      ) : (
      <div className={`digi-main${views.log ? '' : ' digi-main--nolog'}`} style={views.log ? { gridTemplateColumns: `minmax(0, 1fr) ${logW}px` } : undefined}>
        <div className="digi-leftcol">
        {/* Radar: a destra delle finestre RX e TX, alto quanto loro; la cascata sotto resta larga */}
        <div className={`digi-winrow${radarMode && radarOn ? ' digi-winrow--radar' : ''}`}>
        <div className="digi-windows" ref={windowsRef} style={winW !== null && !(radarMode && radarOn) ? { width: winW, maxWidth: '100%' } : undefined}>
          <div className="digi-wingrip" role="separator" aria-orientation="vertical" title={t('digi.winw.tip')} onPointerDown={dragWinW} onDoubleClick={resetWinW} />
          {rxPart}
          <div
            className="digi-split"
            role="separator"
            aria-orientation="horizontal"
            title={t('digi.split.tip')}
            onPointerDown={dragSplit}
            onDoubleClick={() => {
              setRxFrac(0.55);
              try {
                clearSize('aurora.digi.rxfrac', sizeMode);
              } catch {
                // la proporzione vale comunque finche' la pagina e' aperta
              }
            }}
          />
          {txPart}
        </div>
        {radarMode && radarOn ? (
          <div className="digi-radargrip" role="separator" aria-orientation="vertical" title={t('radar.resize')} onPointerDown={dragRadarW} onDoubleClick={() => {
            setRadarW(330);
            try {
              clearSize('aurora.digi.radarw', sizeMode);
            } catch {
              // niente da fare
            }
          }} />
        ) : null}
        {radarMode ? (
          <RadarList on={radarOn} onToggle={toggleRadar} width={radarW} height={radarH ?? undefined} onHeightDrag={dragRadarH} onHeightReset={resetRadarH} answering={views.radarAnswering} onAnswering={(v) => setView({ radarAnswering: v })} stations={radar} showWpm={s.mode === 'cw'} showSnr={s.mode === 'cw'} dialHz={status?.dial_hz ?? 0} rxHz={rxHz} modeName={s.mode === 'cw' ? 'CW' : 'RTTY'} onTune={(hz) => pickFreq(hz, false)} onCall={takeCall} qsoCall={qso.call} onPileupTx={(hz) => save({ tx_follow_rx: false, tx_freq_hz: Math.max(200, Math.min(3500, Math.round(hz))) })} />
        ) : null}
        </div>
        {bandOn && !radarMode && views.signals ? (
          <BandSignals signals={band} showWpm={s.mode === 'cw'} dialHz={status?.dial_hz ?? 0} rxHz={rxHz} emptyText={isPsk ? t('digi.band.emptyPsk') : t('digi.band.empty')} validation={s.band_validation ?? 'normal'} onValidation={(v) => save({ band_validation: v })} onTune={(hz) => pickFreq(hz, false)} onCall={takeCall} />
        ) : null}
        {radarMode && views.rbn !== false ? <RbnList spots={rbnSpots} dialHz={status?.dial_hz ?? 0} onTune={(hz) => pickFreq(hz, false)} /> : null}
        {views.waterfall ? waterfallFor() : null}
        </div>
        {/* rotore e log nella stessa colonna di destra: prima la scheda del rotore prendeva la colonna del log e lo spingeva sotto, fuori vista */}
        <div className="digi-rightcol">
        <RotorCard call={qso.call} grid={qso.wwloc} place="digi" />
        {views.log ? digiLog() : null}
        </div>
      </div>
      )}


      {/* la barra di stato in basso (RX/TX, Sq, AFC...) segue la scelta del menu Vista; in SSB non c'e' */}
      {views.statusBar && !ssbContest ? (
        <DigiStatusBar
          transmitting={sending}
          flashTx={views.flashTx}
          squelchOn={s.squelch_on}
          squelchOpen={snap.rx.squelch_open}
          afc={s.afc_on}
          passband={s.passband_on === true}
          onTogglePassband={() => save({ passband_on: s.passband_on !== true })}
          notchCount={notches.length}
          onClearNotches={() => setNotches([])}
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
          olivia={
            isOliviaMode(s.mode)
              ? { tones: oliviaOf(s, s.mode).tones, bandwidthHz: oliviaOf(s, s.mode).bandwidth_hz, onTones: (n) => saveOlivia({ tones: n }), onBandwidth: (hz) => saveOlivia({ bandwidth_hz: hz }) }
              : undefined
          }
          cw={s.mode === 'cw' ? { bandwidthHz: s.cw_bandwidth_hz, onBandwidth: (hz) => save({ cw_bandwidth_hz: hz }), rxWpm, followRx: s.cw_wpm_follow_rx === true, onFollowRx: (on) => save(followRxPatch(on, rxWpm)), txWpm: s.cw_wpm, onTxWpm: (wpm) => save(wpmStepPatch(wpm)) } : undefined}
          modeName={MODE_NAMES[s.mode]}
          onModeClick={s.mode === 'rtty' ? () => setRttyOpen(true) : undefined}
          modes={snap.modes.map((m) => ({ id: m.id, name: m.name }))}
          modeId={s.mode}
          onPickMode={(id) => save({ mode: id as DigiSettings['mode'] })}
          contestName={textContest ? (textScore ? `${textContest.contest.name} · ${textScore.qso} QSO · ${textScore.punteggio}` : textContest.contest.name) : null}
          onContestClick={() => setContestList(true)}
          onToggleTx={() => {
            // il tasto RX/TX della barra in basso premuto mentre si trasmette ferma subito la trasmissione (come Esc); da fermi la apre
            macroSeq.current.cancel();
            if (live.current.snap.tx.state === 'sending') void digiAbort().then(onChanged, onChanged);
            else toggleTx();
          }}
          onToggleSquelch={() => toggleSetting('squelch', 'toggle')}
          onToggleAfc={() => toggleSetting('afc', 'toggle')}
          onToggleLock={() => toggleSetting('lock', 'toggle')}
          onToggleSnap={() => toggleSetting('snap', 'toggle')}
          onFreq={(hz) => pickFreq(hz, false)}
          onModeSettings={() => setSettingsOpen(true)}
          onThreshold={() => (s.mode === 'rtty' ? setRttyOpen(true) : setSettingsOpen(true))}
          showUtc={views.utc === true}
          onToggleUtc={() => setView({ utc: views.utc !== true })}
          onToggleFlashTx={() => setView({ flashTx: !views.flashTx })}
        />
      ) : null}

      {acqDelayOpen ? <AutoCqDelayDialog seconds={autoCqDelay} onSave={(v) => { setAutoCqDelay(v); setAcqDelayOpen(false); }} onClose={() => setAcqDelayOpen(false)} /> : null}
      {editing !== null && editedMacro ? (
        <MacroEditor key={editing.name} macro={editedMacro} modeName={MODE_NAMES[s.mode]} scope={editing.scope} onSave={saveMacro} onCancel={() => setEditing(null)} />
      ) : null}

      {macroGuide ? <MacroGuide onClose={() => setMacroGuide(false)} /> : null}
      {wordMenu ? <RxWordMenu at={wordMenu} onPick={pickWord} onClose={() => setWordMenu(null)} /> : null}
      {rttyOpen ? <RttySettingsDialog settings={s} onApply={save} onClose={() => setRttyOpen(false)} /> : null}
      {contestList ? <ContestListDialog onClose={() => setContestList(false)} /> : null}
      {contestStats && textContest ? <ContestStatsDialog name={textContest.contest.name} onClose={() => setContestStats(false)} /> : null}
      {qtcOpen && textContest ? (
        <QtcDialog
          dir={qtcOpen}
          call={live.current.qso.call}
          mycall={live.current.status?.de_call ?? ''}
          band={context().band}
          mode={MODE_NAMES[s.mode]}
          dialHz={live.current.status?.dial_hz ?? 0}
          rxText={getDigiText}
          onRun={execute}
          onClose={() => setQtcOpen(null)}
        />
      ) : null}
      {contestEdit ? <ContestEditDialog contest={contestEdit} onClose={() => setContestEdit(null)} onSaved={() => setContestEdit(null)} /> : null}
      {extras.map((x, i) => (
        <ExtraRxWindow key={x.id} id={x.id} freqHz={x.freq_hz} modeName={MODE_NAMES[s.mode]} index={i} afc={x.afc === true} onAfc={(on) => setExtras(extras.map((e) => (e.id === x.id ? { ...e, afc: on } : e)))} onClose={() => closeExtra(x.id)} onWord={takeCall} />
      ))}
      <PopupMenu at={wfMenu} onClose={closeWfMenu} className="call-menu digi-st-menu" width={250} height={300} label={t('wfm.title')}>
        {wfMenu ? (
          <>
            <button type="button" role="menuitemcheckbox" aria-checked={views.wfCalls} onClick={() => { setView({ wfCalls: !views.wfCalls }); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true">{views.wfCalls ? '✓' : ''}</span>
              {t('wfm.calls')}
            </button>
            <button type="button" role="menuitemcheckbox" aria-checked={views.wfWords !== false} onClick={() => { setView({ wfWords: views.wfWords === false }); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true">{views.wfWords !== false ? '✓' : ''}</span>
              {t('digi.band.wfWords')}
            </button>
            <button type="button" role="menuitemcheckbox" aria-checked={views.wideWaterfall} onClick={() => { setView({ wideWaterfall: !views.wideWaterfall }); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true">{views.wideWaterfall ? '✓' : ''}</span>
              {t('wfm.wide')}
            </button>
            <hr />
            <button type="button" role="menuitem" onClick={() => { setNotches(addNotch(notches, wfMenu.hz)); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true" />
              {t('wfm.notchAdd', { hz: wfMenu.hz })}
            </button>
            <button type="button" role="menuitem" disabled={notches.length === 0} onClick={() => { setNotches([]); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true" />
              {t('wfm.notchClear')}
            </button>
            <hr />
            <button type="button" role="menuitem" disabled={extras.length >= 4} title={extras.length >= 4 ? t('wfm.rxFull') : undefined} onClick={() => { addExtra(wfMenu.hz); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true" />
              {t('wfm.rxNew', { hz: wfMenu.hz })}
            </button>
            {(() => {
              const near = nearestExtra(wfMenu.hz);
              return (
                <>
                  <button type="button" role="menuitem" disabled={near === null} onClick={() => { if (near) closeExtra(near.id); setWfMenu(null); }}>
                    <span className="digi-st-menu-check" aria-hidden="true" />
                    {near ? t('wfm.rxClose', { n: near.id }) : t('wfm.rxCloseNone')}
                  </button>
                  <button type="button" role="menuitem" disabled={near === null} onClick={() => { if (near) swapExtra(near.id); setWfMenu(null); }}>
                    <span className="digi-st-menu-check" aria-hidden="true" />
                    {near ? t('wfm.rxSwap', { n: near.id }) : t('wfm.rxSwapNone')}
                  </button>
                </>
              );
            })()}
            <hr />
            <button type="button" role="menuitem" onClick={() => { showGuess(wfMenu.x, wfMenu.y, wfMenu.hz); setWfMenu(null); }}>
              <span className="digi-st-menu-check" aria-hidden="true" />
              {t('wfm.guess')}
            </button>
          </>
        ) : null}
      </PopupMenu>
      <PopupMenu at={guess} onClose={closeGuess} className="call-menu wf-guess" width={340} height={320} label={guess ? t('wfm.guessTitle', { hz: guess.hz }) : undefined}>
        {guess ? (
          <>
            <p className="wf-guess-title">{t('wfm.guessTitle', { hz: guess.hz })}</p>
            {guess.result.guesses.length === 0 ? <p className="sstv-hint">{t('wfm.guessNone')}</p> : null}
            {guess.result.guesses.map((g) => (
              <p key={g.kind} className="wf-guess-item">
                <b>{t(g.confidence === 'likely' ? 'wfm.guessLikely' : 'wfm.guessPossible')}</b> — {t(`wfm.guess.${g.kind}`, { shift: g.shiftHz ?? 170 })}
              </p>
            ))}
            {guess.result.widthHz > 0 ? <p className="sstv-hint">{t('wfm.guessMeasure', { w: guess.result.widthHz, db: guess.result.peakDb })}</p> : null}
            <p className="sstv-hint">{t('wfm.guessNote')}</p>
          </>
        ) : null}
      </PopupMenu>
      {settingsOpen ? <ModeSettings settings={s} personal={saved} levelDb={snap.rx.level_db} snrDb={snap.rx.snr_db ?? null} quality={snap.rx.quality ?? null} onSave={save} onPersonal={setSavedField} onClose={() => setSettingsOpen(false)} keyer={snap.keyer ?? null} /> : null}

    </>
  );
}

/** Il messaggio di una trasmissione rifiutata; in Olivia e Contestia `bad_value` vuol dire toni e banda fuori dalla banda audio. */
function txProblem(code: string, s: DigiSettings, t: T): string {
  if (code === 'bad_value' && isOliviaMode(s.mode)) {
    const o = oliviaOf(s, s.mode);
    return t('digi.olivia.noFit', { tones: o.tones, bw: o.bandwidth_hz, hz: Math.round(effectiveTxHz(s)) });
  }
  return code === 'tx_disabled' ? t('digi.tx.disabled') : code === 'busy' ? t('digi.tx.busy') : code === 'no_transmitter' ? t('digi.tx.notReady') : code === 'no_text_mode' ? t('digi.tx.ssb') : t('digi.tx.error');
}

/** La larghezza del filtro BPF (Hz), come la calcola il core: la sua se impostata, altrimenti secondo il modo. */
function passbandWidth(s: DigiSettings): number {
  if ((s.passband_hz ?? 0) > 0) return s.passband_hz ?? 0;
  if (s.mode === 'psk31') return 3 * 31.25;
  if (s.mode === 'psk63') return 3 * 62.5;
  if (s.mode === 'rtty') return s.rtty_shift_hz + 2 * s.rtty_baud;
  if (s.mode === 'ssb') return 2400;
  // Olivia e Contestia: la banda del modo con un po' di margine per il sincronismo, come nel core
  if (isOliviaMode(s.mode)) return oliviaOf(s, s.mode).bandwidth_hz * 1.2;
  return 80;
}

/** Pausa dell'Auto CQ fra una chiamata e la successiva, in secondi (1-600). */
function AutoCqDelayDialog({ seconds, onSave, onClose }: { seconds: number; onSave(v: number): void; onClose(): void }) {
  const t = useT();
  const [v, setV] = useState(String(seconds));
  const n = Number(v);
  const ok = Number.isFinite(n) && n >= 1 && n <= 600;
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.mi.autoCqDelayTitle')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)} onClick={(e) => (e.target === e.currentTarget ? onClose() : undefined)}>
      <form className="digi-dialog acq-delay" onSubmit={(e) => { e.preventDefault(); if (ok) onSave(n); }}>
        <header className="cop-head">
          <strong>{t('digi.mi.autoCqDelayTitle')}</strong>
          <button type="button" className="digi-tool" onClick={onClose} aria-label={t('sat.set.close')}>✕</button>
        </header>
        <label className="acq-delay-row">
          {t('digi.mi.autoCqDelaySec')}
          <input type="number" min={1} max={600} value={v} autoFocus onChange={(e) => setV(e.target.value)} />
        </label>
        <div className="acq-delay-actions">
          <button type="button" className="digi-tool" onClick={onClose}>{t('digi.mi.cancel')}</button>
          <button type="submit" className="digi-tool" disabled={!ok}>OK</button>
        </div>
      </form>
    </div>
  );
}
