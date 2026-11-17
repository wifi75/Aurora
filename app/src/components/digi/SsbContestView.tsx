import { useEffect, useRef, useState, type PointerEvent as RPointerEvent, type ReactNode } from 'react';
import { browserStorage } from '../../lib/theme';
import { useT } from '../../i18n';
import type { TextContest, TextContestScore } from '../../api/contestTesto';
import { ContestStatsCard, multName } from './ContestStatsDialog';
import type { Qso } from './DigiLog';
import type { Log4omQso } from '../../api/client';
import { EntryCard } from './contest/EntryCard';
import { useContestEntry } from './contest/useContestEntry';
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
  const entryState = useContestEntry({ qso, contest, inSet, noBand, noMode, band, modeName, contestName, score, onQso });
  const { pending, logQ } = entryState;

  return (
    <div className="ssbc">
      <div className="ssbc-cols" ref={colsBox} style={showLog ? (shown ? { gridTemplateColumns: `${shown.left}px 6px minmax(0, 1fr) 6px ${shown.right}px` } : undefined) : { gridTemplateColumns: `minmax(0, 1fr) 6px ${shown?.right ?? 420}px` }}>
        {showLog ? <div className="ssbc-col ssbc-col--log">{log}</div> : null}
        {showLog ? <div className="digi-radargrip" role="separator" aria-orientation="vertical" title={t('ssbc.gripTip')} onPointerDown={dragGrip('left')} onDoubleClick={resetCols} /> : null}
        <div className="ssbc-col ssbc-col--center">
          <EntryCard
            entry={entryState}
            qso={qso}
            rsts={rsts}
            rstr={rstr}
            nrsAuto={nrsAuto}
            qsoNr={qsoNr}
            contestName={contestName}
            windowNotice={windowNotice}
            onEditWindow={onEditWindow}
            onQso={onQso}
            onRst={onRst}
            onRegister={onRegister}
            band={band}
            saveError={saveError}
            onDismissSaveError={onDismissSaveError}
            sectionRef={entry}
          />
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
