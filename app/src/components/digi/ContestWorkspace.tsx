import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { MENU_SLOT_ID } from './MenuBar';
import type { TextContest, TextContestScore } from '../../api/contestTesto';
import type { Log4omQso } from '../../api/client';
import { useT } from '../../i18n';
import { Icon } from '../Icon';
import { DashGrid, type DashWidget } from '../DashGrid';
import type { DashItem } from '../../lib/dashGrid';
import type { StoredLayout } from '../../lib/dashLayout';
import { contestDefaultWidths, defaultContestLayout, isMultCard, loadContestLayout, mergeContestLayout, minsOf, multCardId, multCardsOf, saveContestLayout, type ContestView } from '../../lib/contestLayout';
import { useNarrow } from '../../lib/useNarrow';
import type { Qso } from './DigiLog';
import { EntryCard } from './contest/EntryCard';
import { ClusterCard, LogCard } from './contest/LogCards';
import { MultsCard, MultsTitle } from './contest/MultsCard';
import { RateCard } from './contest/RateCard';
import { StatsCard } from './contest/StatsCard';
import { RxTextCard, TxTextCard, WaterfallCard } from './contest/TextCards';
import { useContestEntry } from './contest/useContestEntry';
import '../../styles/digi-ssb-contest.css';
import '../../styles/contest-workspace.css';

export interface ContestWorkspaceProps {
  /** `ssb` (inserimento, log, cluster, punteggio) oppure `text` (CW, RTTY, PSK: in piu' ricezione, trasmissione e cascata). */
  view: ContestView;
  /** Il contest attivo. */
  contest: TextContest;
  score: TextContestScore | undefined;
  qso: Qso;
  rsts: string;
  rstr: string;
  nrsAuto: string;
  qsoNr: number;
  inSet: (q: Log4omQso) => boolean;
  noBand: boolean;
  noMode: boolean;
  windowNotice: boolean;
  onEditWindow(): void;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  onRegister(): void;
  band: string;
  saveError?: string | null;
  onDismissSaveError?(): void;
  modeName: string;
  /** Vista › Log: spento, la scheda del Log non c'e'. */
  showLog: boolean;
  log: ReactNode;
  cluster: ReactNode;
  /** Solo nella vista `text`: le finestre di ricezione e di trasmissione e la cascata (gli stessi pezzi della pagina, senza contest). */
  rx?: ReactNode;
  tx?: ReactNode;
  /** La cascata alta quanto la scheda (`px` pixel). */
  waterfall?: (heightPx: number) => ReactNode;
  /** La scheda del rotore (finestra mobile): parte nel punto dato. */
  rotor: (startAt: { x: number; y: number }) => ReactNode;
}

/**
 * La vista del contest a schede spostabili e ridimensionabili (come nei Satelliti): Inserimento QSO, Log, DX Cluster, Punteggio e ritmo, Statistiche,
 * una scheda per tipo di moltiplicatore e, nei modi testuali, ricezione, trasmissione e cascata. La disposizione si salva per contest e per vista;
 * «Blocca» la ferma e «Ripristina disposizione» rimette quella di partenza.
 */
export function ContestWorkspace(props: ContestWorkspaceProps) {
  // un'altra disposizione per ogni contest e per ogni vista: si rilegge cambiando l'uno o l'altra
  return <Workspace key={`${props.contest.id}:${props.view}`} {...props} />;
}

function Workspace({ view, contest, score, qso, rsts, rstr, nrsAuto, qsoNr, inSet, noBand, noMode, windowNotice, onEditWindow, onQso, onRst, onRegister, band, saveError = null, onDismissSaveError, modeName, showLog, log, cluster, rx, tx, waterfall, rotor }: ContestWorkspaceProps) {
  const t = useT();
  const narrow = useNarrow();
  const box = useRef<HTMLDivElement>(null);
  // il segnaposto nella barra dei menu (c'e' solo quando la pagina ha la barra): i pulsanti della disposizione vanno li'
  const [slot, setSlot] = useState<HTMLElement | null>(null);
  useEffect(() => setSlot(document.getElementById(MENU_SLOT_ID)), []);
  const mults = useMemo(() => (score ? multCardsOf(score) : []), [score]);
  const [stored, setStored] = useState<StoredLayout>(() => loadContestLayout(contest.id, view, mults));
  // le schede dei moltiplicatori arrivano col punteggio: quelle senza posizione salvata prendono quella di partenza
  const layout = useMemo(() => mergeContestLayout(stored.items, view, mults), [stored.items, view, mults]);
  const setLayout = (next: StoredLayout) => {
    setStored(next);
    saveContestLayout(contest.id, view, next);
  };
  const defaultWidths = useMemo(() => contestDefaultWidths(view, mults), [view, mults]);

  const entry = useContestEntry({ qso, contest, inSet, noBand, noMode, band, modeName, contestName: contest.name, score, onQso });
  const qsoTimes = useMemo(
    () => (entry.logQ.data?.qsos ?? []).filter(inSet).map((q) => Date.parse(`${q.date.replace(' ', 'T').slice(0, 19)}Z`)),
    [entry.logQ.data, inSet],
  );

  // dove parte la finestra del rotore (se non ne ha una salvata): in alto a destra della vista
  const [rotorAt, setRotorAt] = useState<{ x: number; y: number } | null>(null);
  useEffect(() => {
    const id = window.setTimeout(() => {
      const el = box.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const scale = el.offsetWidth > 0 ? r.width / el.offsetWidth : 1;
      setRotorAt({ x: Math.max(0, Math.round((r.right - 260) / scale)), y: Math.max(0, Math.round((r.top + 48) / scale)) });
    }, 300);
    return () => window.clearTimeout(id);
  }, []);

  const mk = (id: string, title: ReactNode, children: ReactNode): DashWidget => {
    const m = minsOf(id);
    return { id, title, minW: m.minW, minH: m.minH, maxRows: 200, autoHeight: false, className: `ctw-card ctw-card--${isMultCard(id) ? 'mult' : id}`, children };
  };
  const widgets: DashWidget[] = [
    mk(
      'entry',
      <>
        {t('ssbc.enter')}
        <b className="ssbc-contest-name"> · {contest.name}</b>
      </>,
      <EntryCard
        bare
        entry={entry}
        qso={qso}
        rsts={rsts}
        rstr={rstr}
        nrsAuto={nrsAuto}
        qsoNr={qsoNr}
        contestName={contest.name}
        windowNotice={windowNotice}
        onEditWindow={onEditWindow}
        onQso={onQso}
        onRst={onRst}
        onRegister={onRegister}
        band={band}
        saveError={saveError}
        onDismissSaveError={onDismissSaveError}
      />,
    ),
    ...(showLog ? [mk('log', t('ctw.card.log'), <LogCard log={log} />)] : []),
    mk('cluster', t('ctw.card.cluster'), <ClusterCard cluster={cluster} />),
    mk('rate', t('ctw.card.rate'), <RateCard score={score} contest={contest} qsoTimes={qsoTimes} />),
    ...(score ? [mk('stats', t('ssbc.statsTitle'), <StatsCard score={score} />)] : []),
    ...(view === 'text'
      ? [
          ...(rx ? [mk('rx', t('ctw.card.rx'), <RxTextCard>{rx}</RxTextCard>)] : []),
          ...(tx ? [mk('tx', t('ctw.card.tx'), <TxTextCard>{tx}</TxTextCard>)] : []),
          ...(waterfall ? [mk('wf', t('ctw.card.wf'), <WaterfallCard render={waterfall} />)] : []),
        ]
      : []),
    ...(score ? mults.map((m) => mk(multCardId(m.tipo), <MultsTitle score={score} tipo={m.tipo} />, <MultsCard score={score} tipo={m.tipo} pending={entry.pending} />)) : []),
  ];

  const toggleLock = () => setLayout({ ...stored, items: layout, locked: !stored.locked });
  const reset = () => setLayout({ ...stored, items: defaultContestLayout(view, mults) });
  const onLayout = (items: DashItem[]) => setLayout({ ...stored, items });

  return (
    <div className="ssbc ssbc--ws" ref={box}>
      <div className="ctw-bar" role="toolbar" aria-label={t('ctw.bar')}>
        <strong className="ssbc-contest-name">{contest.name}</strong>
      </div>
      {narrow || !slot
        ? null
        : createPortal(
            <>
              <button type="button" className={`digi-menubar-icon${stored.locked ? ' on' : ''}`} aria-pressed={stored.locked} title={stored.locked ? t('sat.dash.unlock') : t('sat.dash.lock')} aria-label={stored.locked ? t('sat.dash.unlock') : t('sat.dash.lock')} onClick={toggleLock}>
                <Icon name={stored.locked ? 'lock' : 'unlock'} />
              </button>
              <button type="button" className="digi-menubar-icon" title={t('sat.dash.reset')} aria-label={t('sat.dash.reset')} onClick={reset}>
                <Icon name="layout" />
              </button>
            </>,
            slot,
          )}
      <DashGrid
        widgets={widgets}
        layout={layout}
        onLayout={onLayout}
        locked={stored.locked}
        narrow={narrow}
        defaultWidths={defaultWidths}
        moveLabel={t('sat.dash.move')}
        resizeLabel={t('sat.dash.resize')}
        widthLabel={t('sat.dash.width')}
        panelClass="ctw-panel"
      />
      {rotorAt ? rotor(rotorAt) : null}
    </div>
  );
}
