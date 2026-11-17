import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState, type KeyboardEvent, type PointerEvent as RPointerEvent, type ReactNode } from 'react';
import { browserStorage } from '../../lib/theme';
import { useT } from '../../i18n';
import { getTextContestCheck, type TextContest, type TextContestScore } from '../../api/contestTesto';
import { ContestStatsCard, multLabel, multName } from './ContestStatsDialog';
import type { Qso } from './DigiLog';
import { getDxcc, getLog4om, getLogQsos, type Log4omQso } from '../../api/client';
import { autoExchange, callMultipliers, contestSuggestions, previousQsos, HF_BANDS } from '../../lib/contestEntry';
import { qsoTimeMs } from '../../lib/logWindow';
import { EXTRA_MAX, cleanExtra, extraLabelKey } from '../../lib/contestSection';
import { SaveFailAlert } from './SaveFailAlert';
import '../../styles/digi-ssb-contest.css';

interface Props {
  /** Il QSO in corso e i rapporti (gli stessi della pagina: il tasto «Registra» chiama lo stesso salvataggio dei modi testuali). */
  qso: Qso;
  rsts: string;
  rstr: string;
  /** Lo scambio che parte da solo se `Nr inviato` non e' stato scritto (progressivo o scambio fisso del contest). */
  nrsAuto: string;
  /** Il numero del QSO in corso nel contest (QSO a log del contest + 1, dalle statistiche del core). */
  qsoNr: number;
  /** Il contest attivo (per avanzamento e QSO per ora della scheda delle statistiche). */
  contest: TextContest | null;
  /** Il QSO del log e' del contest in corso? Il doppio si cerca solo fra questi. */
  inSet: (q: Log4omQso) => boolean;
  noBand: boolean;
  noMode: boolean;
  /** La finestra di date del contest non e' impostata o e' scaduta: il log mostra i QSO dopo l'attivazione e la vista lo dice. */
  windowNotice: boolean;
  /** Apre la modifica del contest attivo (le date). */
  onEditWindow(): void;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  /** Registra il QSO (e azzera i campi, RST a 59). */
  onRegister(): void;
  /** La banda con cui si registra (dalla radio o scelta a mano; vuota = sconosciuta). */
  band: string;
  /** Perche' l'ultimo salvataggio e' stato rifiutato: l'avviso resta finche' non si cambia nominativo, si salva o si preme Esc. */
  saveError?: string | null;
  onDismissSaveError?(): void;
  modeName: string;
  /** Il contest attivo (`null` = nessuno: la vista e' un log normale, senza Nr, punteggio e moltiplicatori). */
  contestName: string | null;
  score: TextContestScore | undefined;
  /** Colonna di sinistra (Log) e di destra (DX Cluster). */
  /** Vista › Log: spento, la colonna del Log (e la sua maniglia) sparisce e le altre si allargano. */
  showLog: boolean;
  log: ReactNode;
  cluster: ReactNode;
  /** La scheda del rotore (finestra mobile): parte sotto l'inserimento, nel punto dato. */
  rotor: (startAt: { x: number; y: number }) => ReactNode;
}

type Tr = ReturnType<typeof useT>;

const COLS_KEY = 'aurora.digi.ssbcols';
/** Minimi in px: il Log e il Cluster restano utilizzabili, il centro tiene leggibile la riga dell'inserimento. */
const MIN_SIDE = 220;
const MIN_CENTER = 460;
interface ColW {
  left: number;
  right: number;
}

function readCols(): ColW | null {
  try {
    const v = JSON.parse(browserStorage()?.getItem(COLS_KEY) ?? 'null') as Partial<ColW> | null;
    return v && Number.isFinite(v.left) && Number.isFinite(v.right) ? { left: Number(v.left), right: Number(v.right) } : null;
  } catch {
    return null;
  }
}
function writeCols(v: ColW | null): void {
  try {
    if (v) browserStorage()?.setItem(COLS_KEY, JSON.stringify(v));
    else browserStorage()?.removeItem(COLS_KEY);
  } catch {
    // le larghezze valgono comunque finche' la pagina e' aperta
  }
}

/** Il pannello dei moltiplicatori: per ogni tipo i valori lavorati evidenziati (con quelli che mancano se la lista e' chiusa) o la tabella per banda. */
function Mults({ s, pending, t }: { s: TextContestScore; pending: Set<string>; t: Tr }) {
  const types = Array.from(new Set([...s.griglia.map((g) => g.tipo), ...s.mancanti.map((m) => m.tipo)]));
  const fresh = new Set(s.recenti[0]?.nuovi ?? []);
  const bands = s.bande.map((b) => b.band);
  if (types.length === 0) return <p className="ssbc-muted">{t('ssbc.multsEmpty')}</p>;
  return (
    <>
      {types.map((tipo) => {
        const worked = s.griglia.filter((g) => g.tipo === tipo);
        const miss = s.mancanti.find((m) => m.tipo === tipo);
        const perBand = worked.some((g) => g.bande.length > 0) || miss?.per_banda === true;
        const key = (v: string) => `${tipo}:${v}`;
        return (
          <section key={tipo} className="ssbc-mult">
            <h4>
              {multName(t, tipo)} <span>{miss ? t('ssbc.multOf', { n: worked.length, tot: miss.totali }) : worked.length}</span>
            </h4>
            {perBand ? (
              <div className="ssbc-mult-wrap">
                <table className="ssbc-mult-table">
                  <thead>
                    <tr>
                      <th>{multName(t, tipo)}</th>
                      {bands.map((b) => (
                        <th key={b}>{b}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {worked.map((g) => (
                      <tr key={g.valore} className={fresh.has(key(g.valore)) ? 'is-new' : undefined}>
                        <td>{g.valore}</td>
                        {bands.map((b) => (
                          <td key={b}>{g.bande.includes(b) ? '✓' : ''}</td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="ssbc-chips">
                {(miss ? [...worked.map((g) => g.valore), ...miss.valori] : worked.map((g) => g.valore))
                  .filter((v, i, all) => all.indexOf(v) === i)
                  .sort((a, b) => a.localeCompare(b, undefined, { numeric: true }))
                  .map((v) => {
                    const done = worked.some((g) => g.valore === v);
                    const cls = done ? (fresh.has(key(v)) ? 'on new' : 'on') : pending.has(key(v)) ? 'pend' : '';
                    return (
                      <span key={v} className={`ssbc-chip ${cls}`}>
                        {v}
                      </span>
                    );
                  })}
              </div>
            )}
            {miss && miss.valori.length > 0 && miss.valori.length <= 12 ? <p className="ssbc-muted">{t('ssbc.missing', { list: miss.valori.join(' ') })}</p> : null}
          </section>
        );
      })}
    </>
  );
}

/**
 * Il contest in SSB: tre colonne a tutta altezza (Log, inserimento del QSO con punteggio e moltiplicatori, DX Cluster) al posto delle
 * finestre RX/TX, della cascata e della barra dei modi digitali.
 */
export function SsbContestView({ qso, rsts, rstr, nrsAuto, onQso, onRst, onRegister, qsoNr, contest, inSet, noBand, noMode, windowNotice, onEditWindow, band, saveError = null, onDismissSaveError, modeName, contestName, score, showLog, log, cluster, rotor }: Props) {
  const t = useT();
  const contestOn = contestName !== null;
  const callRef = useRef<HTMLInputElement>(null);
  const form = useRef<HTMLDivElement>(null);
  const colsBox = useRef<HTMLDivElement>(null);
  const entry = useRef<HTMLElement>(null);
  // dove parte la finestra del rotore: sotto il riquadro dell'inserimento (coordinate della pagina, senza lo zoom)
  const [rotorAt, setRotorAt] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    // dopo che la pagina ha preso le sue misure (colonne e riquadro al loro posto)
    const id = window.setTimeout(() => {
      const el = entry.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const scale = el.offsetWidth > 0 ? r.width / el.offsetWidth : 1;
      // di base in basso a destra della colonna centrale (sotto «Inserisci QSO»), mai sopra il blocco delle bande
      const c = colsBox.current?.querySelector('.ssbc-col--center')?.getBoundingClientRect();
      if (c) setRotorAt({ x: Math.max(0, Math.round((c.right - 250) / scale)), y: Math.max(0, Math.round((c.bottom - 205) / scale)) });
      else setRotorAt({ x: Math.round(r.left / scale), y: Math.round(r.bottom / scale) + 10 });
    }, 300);
    return () => window.clearTimeout(id);
  }, []);
  // larghezze delle colonne laterali (px); `null` = proporzioni di partenza
  const [cols, setCols] = useState<ColW | null>(readCols);
  // larghezza disponibile: con una finestra piu' stretta le colonne laterali salvate si accorciano, il centro tiene il suo minimo
  const [boxW, setBoxW] = useState(0);
  useEffect(() => {
    const el = colsBox.current;
    if (!el) return;
    const measure = () => setBoxW(el.clientWidth);
    measure();
    window.addEventListener('resize', measure);
    return () => window.removeEventListener('resize', measure);
  }, []);
  const shown: ColW | null = (() => {
    if (!cols) return null;
    const room = boxW - 16 - MIN_CENTER;
    if (boxW === 0 || cols.left + cols.right <= room) return cols;
    const k = Math.max(0.3, room / (cols.left + cols.right));
    return { left: Math.max(MIN_SIDE * 0.6, Math.round(cols.left * k)), right: Math.max(MIN_SIDE * 0.6, Math.round(cols.right * k)) };
  })();
  /** Trascina una maniglia: `side` e' la colonna laterale che cambia (il centro prende il resto). */
  const dragGrip = (side: 'left' | 'right') => (e: RPointerEvent<HTMLDivElement>) => {
    const box = colsBox.current;
    if (!box) return;
    e.preventDefault();
    const total = box.getBoundingClientRect().width - 12 - 4;
    const kids = box.children;
    const start: ColW = cols ?? { left: (kids[0] as HTMLElement).getBoundingClientRect().width, right: (kids[kids.length - 1] as HTMLElement).getBoundingClientRect().width };
    const x0 = e.clientX;
    const scale = box.offsetWidth > 0 ? box.getBoundingClientRect().width / box.offsetWidth : 1;
    let last = start;
    const move = (m: PointerEvent) => {
      const dx = (m.clientX - x0) / scale;
      if (side === 'left') {
        const left = Math.round(Math.min(Math.max(start.left + dx, MIN_SIDE), Math.max(MIN_SIDE, total - start.right - MIN_CENTER)));
        last = { left, right: start.right };
      } else {
        const right = Math.round(Math.min(Math.max(start.right - dx, MIN_SIDE), Math.max(MIN_SIDE, total - start.left - MIN_CENTER)));
        last = { left: start.left, right };
      }
      setCols(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      writeCols(last);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  const resetCols = () => {
    setCols(null);
    writeCols(null);
  };
  // il nominativo si controlla un attimo dopo l'ultimo tasto
  const call = qso.call.trim().toUpperCase();
  const [typed, setTyped] = useState('');
  useEffect(() => {
    const id = window.setTimeout(() => setTyped(call), 220);
    return () => window.clearTimeout(id);
  }, [call]);
  const nrr = (qso.nrr ?? '').trim();
  const check = useQuery({
    queryKey: ['ssb-contest-check', typed, band, nrr],
    queryFn: () => getTextContestCheck(typed, band, modeName, nrr),
    enabled: contestOn && typed.length >= 3 && call === typed && band !== '',
    retry: false,
    staleTime: 0,
    gcTime: 0,
    refetchOnWindowFocus: false,
  });
  // lo stesso log del pannello Log (stessa chiave, quindi la stessa lettura): il doppio si cerca solo nei QSO del contest
  const logQ = useQuery({ queryKey: ['log4om', 500], queryFn: () => getLog4om(500), retry: false, refetchInterval: 10_000 });
  const mine = (logQ.data?.qsos ?? []).find((q) => inSet(q) && q.call.toUpperCase() === call && (noBand || q.band === band) && (noMode || q.mode.toUpperCase() === modeName.toUpperCase()));
  const raw = call === typed && typed.length >= 3 ? check.data : undefined;
  const c = raw ? { ...raw, doppio: mine !== undefined, precedente: mine ? { time_ms: Date.parse(`${mine.date.replace(' ', 'T').slice(0, 19)}Z`), band: mine.band, mode: mine.mode, exch_rcvd: '', exch_sent: '' } : null } : undefined;
  const pending = new Set(c?.nuovi ?? []);
  // la casella del terzo elemento dello scambio (sezione, club, stato) compare solo se il core dice che questo nominativo la da' (stesse regole
  // del punteggio); la risposta si ricorda per il nominativo, cosi' non sparisce mentre si scrive il numero e la richiesta si rifa'
  const [exchFor, setExchFor] = useState<{ call: string; want: boolean }>({ call: '', want: false });
  const exchAtteso = raw ? raw.exch_atteso === true : null;
  useEffect(() => {
    if (exchAtteso !== null) setExchFor({ call: typed, want: exchAtteso });
  }, [exchAtteso, typed]);
  const showExtra = contestOn && exchFor.call === call && exchFor.want;
  const extraNow = qso.extra ?? '';
  useEffect(() => {
    // senza la casella il valore scritto non vale piu' (altro nominativo)
    if (!showExtra && extraNow !== '') onQsoRef.current({ extra: '' });
  }, [showExtra, extraNow]);

  // paese e zone del nominativo (database dei prefissi): suggerimenti, scambio ricevuto precompilato, moltiplicatori che porta
  const known = contestOn && call === typed && typed.length >= 3;
  const dxccQ = useQuery({ queryKey: ['ssb-dxcc', typed], queryFn: () => getDxcc(typed), enabled: known, retry: false, staleTime: 300_000, refetchOnWindowFocus: false });
  const info = known ? dxccQ.data : undefined;
  const suggestions = contest ? contestSuggestions(contest, info) : [];
  const auto = contest ? autoExchange(contest, info) : null;
  // lo scambio ricevuto si precompila solo se e' vuoto o e' ancora quello messo da qui: quello scritto a mano non si tocca
  const autoRef = useRef('');
  const nrrRef = useRef(nrr);
  nrrRef.current = nrr;
  const onQsoRef = useRef(onQso);
  onQsoRef.current = onQso;
  const dxccDone = dxccQ.isSuccess || dxccQ.isError;
  useEffect(() => {
    if (!contestOn) return;
    const cur = nrrRef.current;
    if (call.length < 3) {
      if (autoRef.current !== '' && cur === autoRef.current) onQsoRef.current({ nrr: '' });
      autoRef.current = '';
      return;
    }
    if (call !== typed || !dxccDone || (cur !== '' && cur !== autoRef.current)) return;
    if (auto !== null) {
      if (cur !== auto) onQsoRef.current({ nrr: auto });
      autoRef.current = auto;
    } else {
      if (cur !== '') onQsoRef.current({ nrr: '' });
      autoRef.current = '';
    }
  }, [contestOn, call, typed, auto, dxccDone]);

  // QSO precedenti con il nominativo, una riga per banda: dal log (con gli scambi nel commento), solo quelli del contest in corso
  const logAll = useQuery({ queryKey: ['log-qsos-contest', logQ.data?.total ?? 0], queryFn: getLogQsos, enabled: known, retry: false, staleTime: 5_000, refetchOnWindowFocus: false });
  const prevRows = known
    ? previousQsos(logAll.data?.qsos ?? [], typed, (q) => inSet({ ...q, date: `${new Date(qsoTimeMs(q)).toISOString().slice(0, 19).replace('T', ' ')}Z` }), HF_BANDS)
    : [];
  const callMults = known && score && c ? callMultipliers(score.tipi, info, c.nuovi, score.griglia, band) : [];

  useEffect(() => callRef.current?.focus(), []);

  const register = () => {
    onRegister();
    callRef.current?.focus();
  };
  const onKey = (e: KeyboardEvent<HTMLElement>) => {
    if (e.key === 'Escape' && saveError) onDismissSaveError?.();
    if (e.key !== 'Enter') return;
    e.preventDefault();
    const fields = Array.from(form.current?.querySelectorAll<HTMLInputElement>('input:not([readonly]):not([data-skip])') ?? []);
    const i = fields.indexOf(e.target as HTMLInputElement);
    // Invio nell'ultimo campo registra; negli altri passa al successivo
    if (i < 0 || i >= fields.length - 1) register();
    else {
      fields[i + 1]?.focus();
      fields[i + 1]?.select();
    }
  };

  let alert: ReactNode = null;
  if (c) {
    if (c.doppio) {
      const p = c.precedente;
      const at = p ? new Date(p.time_ms).toISOString().slice(11, 16) : '';
      alert = (
        <p className="ssbc-alert ssbc-alert--dupe" role="alert">
          {p ? t('ssbc.dupeAt', { time: at, band: p.band }) : t('ssbc.dupe')}
        </p>
      );
    } else if (!c.ammesso) {
      alert = (
        <p className="ssbc-alert ssbc-alert--warn" role="alert">
          {c.motivo === 'band' ? t('ssbc.notBand') : t('ssbc.notValid')}
        </p>
      );
    } else if (c.nuovi.length > 0) {
      alert = (
        <p className="ssbc-alert ssbc-alert--mult" role="status">
          {t('ssbc.newMult', { mult: c.nuovi.map((m) => multLabel(t, m)).join(', ') })}
        </p>
      );
    } else {
      alert = (
        <p className="ssbc-alert ssbc-alert--ok" role="status">
          {t('ssbc.newCall', { n: c.punti })}
        </p>
      );
    }
  }

  const field = (label: string, input: ReactNode, cls: string) => (
    <label className={`ssbc-field ${cls}`}>
      <span>{label}</span>
      {input}
    </label>
  );

  return (
    <div className="ssbc">
      <div className="ssbc-cols" ref={colsBox} style={showLog ? (shown ? { gridTemplateColumns: `${shown.left}px 6px minmax(0, 1fr) 6px ${shown.right}px` } : undefined) : { gridTemplateColumns: `minmax(0, 1fr) 6px ${shown?.right ?? 420}px` }}>
        {showLog ? <div className="ssbc-col ssbc-col--log">{log}</div> : null}
        {showLog ? <div className="digi-radargrip" role="separator" aria-orientation="vertical" title={t('ssbc.gripTip')} onPointerDown={dragGrip('left')} onDoubleClick={resetCols} /> : null}
        <div className="ssbc-col ssbc-col--center">
          <section className="ssbc-entry" ref={entry} aria-label={t('ssbc.enter')}>
            <h3>
              {t('ssbc.enter')}
              {contestName ? <b className="ssbc-contest-name"> · {contestName}</b> : null} <span className="ssbc-hint">· {t('ssbc.enterHint')}</span>
            </h3>
            <p className={`ssbc-bandnow${band === '' ? ' ssbc-bandnow--none' : ''}`}>{t('digi.log.bandNow', { band: band === '' ? '—' : band.replace(/^(\d+)m$/, '$1 m') })}</p>
            {contestOn && windowNotice ? (
              <p className="ssbc-notice" role="status">
                {t('ssbc.windowOff')}{' '}
                <button type="button" className="ssbc-link" onClick={onEditWindow}>
                  {t('ssbc.windowSet')}
                </button>
              </p>
            ) : null}
            <div className="ssbc-row" ref={form} onKeyDown={onKey}>
              {contestOn ? field(t('ssbc.qsoNr'), <input className="ssbc-in" value={String(qsoNr).padStart(3, '0')} readOnly tabIndex={-1} />, 'ssbc-field--qsonr') : null}
              {field(t('ssbc.call'), <input ref={callRef} className="ssbc-in ssbc-in--call" value={qso.call} maxLength={16} autoComplete="off" spellCheck={false} onChange={(e) => onQso({ call: e.target.value.toUpperCase() })} />, 'ssbc-field--call')}
              {contestOn ? (
                <>
                  {field(t('ssbc.serialSent'), <input className="ssbc-in" data-skip="1" tabIndex={-1} value={qso.nrs || nrsAuto} maxLength={12} autoComplete="off" onChange={(e) => onQso({ nrs: e.target.value.toUpperCase() })} />, 'ssbc-field--nr')}
                  {field(t('ssbc.serialRcvd'), <input className="ssbc-in" value={qso.nrr ?? ''} maxLength={12} autoComplete="off" onChange={(e) => onQso({ nrr: e.target.value.toUpperCase() })} />, 'ssbc-field--nrr')}
                  {showExtra ? field(t(extraLabelKey(contestName)), <input className="ssbc-in" value={extraNow} maxLength={EXTRA_MAX} autoComplete="off" spellCheck={false} onChange={(e) => onQso({ extra: cleanExtra(e.target.value) })} />, 'ssbc-field--nrr ssbc-field--extra') : null}
                </>
              ) : (
                <>
                  {field(t('ssbc.name'), <input className="ssbc-in" value={qso.name} maxLength={30} autoComplete="off" onChange={(e) => onQso({ name: e.target.value })} />, 'ssbc-field--nrr ssbc-field--name')}
                  {field(t('ssbc.qth'), <input className="ssbc-in" value={qso.qth} maxLength={40} autoComplete="off" onChange={(e) => onQso({ qth: e.target.value })} />, 'ssbc-field--nrr ssbc-field--qth')}
                  {field(t('ssbc.rsts'), <input className="ssbc-in" value={rsts} maxLength={4} inputMode="numeric" autoComplete="off" onChange={(e) => onRst({ rsts: e.target.value })} />, 'ssbc-field--rst')}
                  {field(t('ssbc.rstr'), <input className="ssbc-in" value={rstr} maxLength={4} inputMode="numeric" autoComplete="off" onChange={(e) => onRst({ rstr: e.target.value })} />, 'ssbc-field--rst')}
                </>
              )}
            </div>
            {suggestions.length > 0 ? (
              <p className="ssbc-suggest">
                <span>{t('ssbc.suggest')}:</span>
                {suggestions.map((x) => (
                  <button key={x.kind} type="button" tabIndex={-1} className="ssbc-chip on" onClick={() => onQso({ nrr: x.value })}>
                    {t(x.kind === 'cq' ? 'ctest.f.sCq' : x.kind === 'itu' ? 'ctest.f.sItu' : 'ctest.f.sDxcc')} {x.value}
                  </button>
                ))}
              </p>
            ) : null}
            <div className="ssbc-alertbox" aria-live="polite">
              {saveError ? <SaveFailAlert msg={saveError} /> : alert}
            </div>
            {callMults.length > 0 ? (
              <div className="ssbc-callmult" aria-label={t('ssbc.callMult', { call: typed })}>
                <span>{t('ssbc.callMult', { call: typed })}</span>
                {callMults.map((m) => (
                  <b key={`${m.tipo}:${m.value}`} className={m.fresh ? 'is-new' : undefined}>
                    {multName(t, m.tipo)} {m.value}: {m.fresh ? t('ssbc.cmNew') : t('ssbc.cmWorked')}
                  </b>
                ))}
              </div>
            ) : null}
            {known ? (
              <table className="ssbc-prev" aria-label={t('ssbc.prevTitle', { call: typed })}>
                <caption>{t('ssbc.prevTitle', { call: typed })}</caption>
                <thead>
                  <tr>
                    <th>{t('ctest.c.band')}</th>
                    <th>{t('ssbc.prevMode')}</th>
                    <th>{t('ssbc.prevTime')}</th>
                    <th>{t('ssbc.prevNr')}</th>
                  </tr>
                </thead>
                <tbody>
                  {prevRows.map((r) => (
                    <tr key={r.band} className={r.qso ? 'is-worked' : undefined}>
                      <td>{r.band.replace('m', '')}</td>
                      <td>{r.qso?.mode ?? '-'}</td>
                      <td title={r.qso ? new Date(r.qso.time_ms).toISOString().slice(0, 16).replace('T', ' ') : undefined}>{r.qso ? new Date(r.qso.time_ms).toISOString().slice(11, 16) : '-'}</td>
                      <td>{r.qso ? r.qso.nr || '-' : '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            ) : null}
          </section>
          {contestOn ? <div className="ssbc-lower-main">
          <section className="ssbc-score" aria-label={t('ssbc.points')}>
            {score && score.qso + score.doppi > 0 ? (
              <>
                <div className="ssbc-tiles">
                  <div>
                    <span>{t('ssbc.qso')}</span>
                    <b>{score.qso}</b>
                  </div>
                  <div>
                    <span>{t('ssbc.points')}</span>
                    <b>{score.punti}</b>
                  </div>
                  <div>
                    <span>{t('ssbc.mult')}</span>
                    <b>{score.molt}</b>
                  </div>
                  <div className="ssbc-tiles-total">
                    <span>{t('ssbc.total')}</span>
                    <b>{score.punteggio}</b>
                  </div>
                </div>
                <p className="ssbc-muted">{t('ssbc.rate', { r60: score.rate60, r10: score.rate10 })}</p>
              </>
            ) : (
              <p className="ssbc-muted">{t('ssbc.waiting')}</p>
            )}
          </section>
          <section className="ssbc-mults" aria-label={t('ssbc.mults')}>
            <h3>{t('ssbc.mults')}</h3>
            {score ? <Mults s={score} pending={pending} t={t} /> : <p className="ssbc-muted">{t('ssbc.multsEmpty')}</p>}
          </section>
          {score && contest ? (
            <section className="ssbc-stats" aria-label={t('ssbc.statsTitle')}>
              <h3>{t('ssbc.statsTitle')}</h3>
              <ContestStatsCard s={score} contest={contest} qsoTimes={(logQ.data?.qsos ?? []).filter(inSet).map((q) => Date.parse(`${q.date.replace(' ', 'T').slice(0, 19)}Z`))} />
            </section>
          ) : null}
          </div> : null}
        </div>
        <div className="digi-radargrip" role="separator" aria-orientation="vertical" title={t('ssbc.gripTip')} onPointerDown={dragGrip('right')} onDoubleClick={resetCols} />
        <div className="ssbc-col ssbc-col--cluster">{cluster}</div>
      </div>
      {rotorAt ? rotor(rotorAt) : null}
    </div>
  );
}
