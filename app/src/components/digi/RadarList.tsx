import { useCallback, useMemo, useState, useSyncExternalStore, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clearDigiBand, getClusterSpots, getDigi, getLogQsos } from '../../api/client';
import { useT } from '../../i18n';
import { clearBandSignals } from '../../lib/digiBand';
import { getPileup, onPileup, pileupTarget, setPileupDx } from '../../lib/dxPileup';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from '../MovableColumns';
import type { RadarStation } from '../../lib/digiBand';
import { flagCode } from '../../lib/flags';
import { bandOf } from '../../lib/format';
import { valuta } from '../../api/contest';
import { buildLogIndex, contestStatus, logStatus, type ContestMark, type LogStatus } from '../../lib/logStatus';
import { logWindowStart, qsoTimeMs } from '../../lib/logWindow';
import { browserStorage } from '../../lib/theme';
import { useContestLog } from '../../lib/useContestLog';
import { getActiveTextContest } from '../../api/contestTesto';
import { contestQsos, textContestStatus } from '../../lib/textContest';
import { spotBlocked } from '../../lib/spotPrefill';
import { toast } from '../../lib/toast';
import { PopupMenu } from '../PopupMenu';
import { SpotDialog } from './SpotDialog';
import '../../styles/digi-radar.css';

interface Props {
  /** Acceso (decodifica tutta la banda) e chi lo accende o spegne: spento resta solo la barra con l'interruttore. */
  on?: boolean;
  onToggle?(): void;
  /** Larghezza scelta con la maniglia (px). */
  width?: number;
  /** Altezza scelta con la maniglia del bordo inferiore (px): assente = alto quanto le finestre accanto. */
  height?: number;
  onHeightDrag?(e: ReactPointerEvent): void;
  onHeightReset?(): void;
  /** Anche chi risponde (dopo DE o TU), non solo chi chiama CQ; e chi la cambia. */
  answering?: boolean;
  onAnswering?(v: boolean): void;
  stations: RadarStation[];
  /** CW: colonna della velocita'. */
  showWpm: boolean;
  /** RTTY non misura il S/N per canale: la colonna si nasconde. */
  showSnr: boolean;
  /** Frequenza della radio (Hz): con la radio nota la frequenza vera in kHz, se no il tono audio in Hz. */
  dialHz: number;
  /** La frequenza di ricezione adesso: la riga della stazione ascoltata si evidenzia. */
  rxHz: number;
  modeName: string;
  onTune(hz: number): void;
  onCall(call: string): void;
  /** Il nominativo nel campo del QSO: si puo' scegliere come DX del pile-up. */
  qsoCall?: string;
  /** Modo DXpedition: la trasmissione va dove ascolta la DX (con Lock). */
  onPileupTx?(hz: number): void;
}

/** Le colonne del Radar: si spostano trascinando il titolo e si allargano dal bordo, come nel DX Cluster; ordine e larghezze restano ricordati. */
const RADAR_COLUMNS = ['call', 'freq', 'wpm', 'snr', 'cq'] as const;
type RadarCol = (typeof RADAR_COLUMNS)[number];
const RADAR_COL_EM: Record<RadarCol, number> = { call: 9, freq: 6.5, wpm: 3.6, snr: 3.6, cq: 3.2 };

type MarkTip = 'radar.logDupeTip' | 'radar.logWorkedTip' | 'radar.logMultTip' | 'radar.logDxccTip' | 'radar.logNewCallTip' | 'radar.logBandModeTip' | 'radar.logBandTip' | 'radar.logModeTip';

/** Il segno del log accanto al nominativo: etichetta breve e spiegazione (in gara il moltiplicatore e' «MULT», fuori gara il paese «DXCC»). */
function logMark(st: Exclude<LogStatus, null>, contest: boolean): { label: string; tip: MarkTip } {
  switch (st) {
    case 'dupe': return { label: 'DUPE', tip: 'radar.logDupeTip' };
    case 'worked': return { label: 'B4', tip: 'radar.logWorkedTip' };
    case 'mult': return contest ? { label: 'MULT', tip: 'radar.logMultTip' } : { label: 'DXCC', tip: 'radar.logDxccTip' };
    case 'new_call': return { label: 'NEW', tip: 'radar.logNewCallTip' };
    case 'new_band_mode': return { label: 'BM', tip: 'radar.logBandModeTip' };
    case 'new_band': return { label: 'B', tip: 'radar.logBandTip' };
    case 'new_mode': return { label: 'M', tip: 'radar.logModeTip' };
  }
}

/** I segni della legenda, nell'ordine di importanza. */
const LEGEND: Record<'contest' | 'log', Exclude<LogStatus, null>[]> = {
  contest: ['dupe', 'mult', 'new_call', 'new_band_mode', 'new_band', 'new_mode'],
  log: ['worked', 'mult', 'new_call', 'new_band_mode', 'new_band', 'new_mode'],
};

function freqLabel(hz: number, dialHz: number): string {
  return dialHz > 0 ? ((dialHz + hz) / 1000).toFixed(2) : `${Math.round(hz)} Hz`;
}

/**
 * Il Radar: le stazioni della banda decodificate con sicurezza, una riga per nominativo, in una griglia a colonne fisse. Ogni stazione ha
 * il suo colore. Clic: la ricezione va sulla stazione; doppio clic: ricezione sulla stazione e nominativo nel QSO. Accanto al nominativo il
 * segno del log locale: paese nuovo, gia' collegato su questa banda e modo, collegato altrove (colori dei tipi della pagina FT8).
 */
export function RadarList({ on = true, onToggle, width, height, onHeightDrag, onHeightReset, answering = false, onAnswering, stations, showWpm, showSnr, dialHz, rxHz, modeName, onTune, onCall, qsoCall = '', onPileupTx }: Props) {
  const t = useT();
  const pileup = useSyncExternalStore(onPileup, getPileup);
  const pileTarget = pileupTarget(pileup);
  const fmt = (hz: number) => (dialHz > 0 ? ((dialHz + hz) / 1000).toFixed(2) : `${Math.round(hz)} Hz`);
  const qsoDx = qsoCall.trim().toUpperCase();
  const hidden: RadarCol[] = [...(showWpm ? [] : (['wpm'] as const)), ...(showSnr ? [] : (['snr'] as const))];
  const cols = useMovableColumns({ storageKey: 'aurora.digi.radar.cols', columns: RADAR_COLUMNS, defaultEm: RADAR_COL_EM, elastic: 'call', hidden });
  const contest = useContestLog();
  const inContest = !!contest.attivo;
  const band = dialHz > 0 ? bandOf(dialHz) : null;
  const log = useQuery({ queryKey: ['log-qsos'], queryFn: getLogQsos, retry: false, refetchOnWindowFocus: false, enabled: !inContest });
  // il contest dei modi testuali (menu Contest): doppi e novita' dai QSO del log dentro la sua finestra di date e delle categorie valide
  const textContestData = useQuery({ queryKey: ['contest-testo-attivo'], queryFn: getActiveTextContest, retry: false, refetchOnWindowFocus: false, staleTime: 30_000, enabled: !inContest }).data;
  const textContest = inContest ? null : (textContestData?.contest ?? null);
  const minute = Math.floor(Date.now() / 60_000);
  const gara = inContest || !!textContest;
  const textQsos = useMemo(() => (textContest && log.data ? contestQsos(log.data.qsos, textContest, minute * 60_000) : null), [textContest, log.data, minute]);
  // fuori gara: i QSO dentro la finestra scelta con <IGNOREBEFORE> e <PERIOD> (al minuto, per non ricalcolare a ogni giro)
  const from = logWindowStart(browserStorage(), Math.floor(Date.now() / 60_000) * 60_000);
  const index = useMemo(() => {
    if (inContest) return contest.log ? buildLogIndex(contest.log.filter((q) => q.stato !== 'errore').map((q) => ({ ...q, country: '' }))) : null;
    if (!log.data) return null;
    return buildLogIndex(from == null ? log.data.qsos : log.data.qsos.filter((q) => !(qsoTimeMs(q) < from)));
  }, [inContest, contest.log, log.data, from]);
  // in gara il doppio e i moltiplicatori vengono dal contest: una richiesta quando cambia l'elenco dei nominativi o il log, non a ogni giro
  const calls = useMemo(() => Array.from(new Set(stations.map((s) => s.call.toUpperCase()))).sort(), [stations]);
  const valori = useQuery({
    queryKey: ['contest-valuta-radar', contest.attivo, contest.nQso, band, modeName, calls.join(',')],
    queryFn: () => valuta(calls.map((call) => ({ call, grid: '' })), band ?? undefined, modeName),
    enabled: inContest && on && calls.length > 0,
    retry: false,
    staleTime: 60_000,
    placeholderData: (prev) => prev,
  }).data;
  const marks = useMemo(() => (inContest && valori ? new Map<string, ContestMark>(valori.valori.map((v) => [v.call, v])) : null), [inContest, valori]);
  const source = inContest ? t('radar.srcContest', { name: contest.nome ?? '' }) : textContest ? t('radar.srcContest', { name: textContest.name }) : from != null ? t('radar.srcLogFrom', { t: new Date(from).toISOString().slice(0, 16).replace('T', ' ') }) : t('radar.srcLog');
  // menu del tasto destro su una riga: «Spot DX» (la finestra del DX Cluster gia' compilata) e «Copia nominativo»
  const [menu, setMenu] = useState<{ x: number; y: number; s: RadarStation } | null>(null);
  const [spot, setSpot] = useState<RadarStation | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);
  // la banda laterale e lo stato del cluster dalle stesse richieste della pagina e del pannello DX Cluster (nessuna richiesta in piu')
  const lsb = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false, enabled: menu !== null || spot !== null }).data?.rx.radio_lsb === true;
  const cluster = useQuery({ queryKey: ['dx-cluster'], queryFn: () => getClusterSpots(), retry: false, enabled: menu !== null }).data;
  const blocked = spotBlocked(cluster?.config.spot_via, cluster?.servers.filter((x) => x.state === 'connected').length ?? 0);
  const head: Record<RadarCol, string> = { call: t('radar.colCall'), freq: t('radar.colFreq'), wpm: 'WPM', snr: 'S/N', cq: 'CQ' };
  const headTip: Record<RadarCol, string> = { call: t('radar.callColTip'), freq: t('radar.freqTip'), wpm: t('radar.wpmTip'), snr: t('radar.snrTip'), cq: t('radar.cqTip') };
  const cell = (c: RadarCol, s: RadarStation) => {
    switch (c) {
      case 'call': {
        const code = flagCode(s.country);
        const st = inContest ? contestStatus(marks, index, s.call, band, modeName) : textContest && textQsos ? textContestStatus(textQsos, textContest, s.call, band, modeName) : logStatus(index, s.call, s.country, band, modeName);
        const mark = st ? logMark(st, gara) : null;
        return (
          <td key={c} className="radar-call">
            <span className="radar-dot" title={s.stale ? t('radar.dotStale') : t('radar.dotLive')} aria-label={s.stale ? t('radar.dotStale') : t('radar.dotLive')} />
            {code ? <span className={`fi fi-${code} decode-flag`} title={s.country ?? undefined} aria-hidden="true" /> : null}
            <b>{s.call}</b>
            {st && mark ? (
              <span className={`radar-log radar-log--${st.replaceAll('_', '-')}`} title={t(mark.tip)}>
                {mark.label}
              </span>
            ) : null}
          </td>
        );
      }
      case 'freq':
        return (
          <td key={c} className="radar-num" title={t('radar.freqTip')}>
            {freqLabel(s.freqHz, dialHz)}
          </td>
        );
      case 'wpm':
        return (
          <td key={c} className="radar-num" title={t('radar.wpmTip')}>
            {s.wpm != null ? Math.round(s.wpm) : ''}
          </td>
        );
      case 'snr':
        return (
          <td key={c} className="radar-num" title={t('radar.snrTip')}>
            {Math.round(s.snrDb)}
          </td>
        );
      case 'cq':
        return (
          <td key={c} className="radar-cq">
            {s.cq ? <em title={t('radar.cqTip')}>CQ</em> : ''}
          </td>
        );
    }
  };
  return (
    <div className={`radar-slot${on ? '' : ' radar-slot--off'}`} style={on ? ({ '--radar-w': width ? `${width}px` : undefined, '--radar-h': height ? `${height}px` : undefined } as CSSProperties) : undefined}>
    <section className={`radar${on ? '' : ' radar--off'}`} aria-label={`${t('radar.title')} ${modeName}`}>
      <header className="radar-head">
        {onToggle ? (
          <button type="button" role="switch" aria-checked={on} className={`radar-switch${on ? ' radar-switch--on' : ''}`} title={t('radar.tip')} onClick={onToggle}>
            {on ? 'ON' : 'OFF'}
          </button>
        ) : null}
        {on ? <small>{t('radar.count', { n: stations.length })}</small> : null}
        {on ? (
          <span className="radar-legend" title={[source, ...LEGEND[gara ? 'contest' : 'log'].map((k) => { const m = logMark(k, gara); return `${m.label}: ${t(m.tip)}`; })].join('\n')}>
            {LEGEND[gara ? 'contest' : 'log'].map((k) => (
              <span key={k} className={`radar-log radar-log--${k.replaceAll('_', '-')}`} title={`${logMark(k, gara).label}: ${t(logMark(k, gara).tip)}`}>
                {logMark(k, gara).label}
              </span>
            ))}
          </span>
        ) : null}
        {on && onAnswering ? (
          <button
            type="button"
            className={`radar-clear radar-clear--icon radar-answer${answering ? ' radar-answer--on' : ''}`}
            aria-pressed={answering}
            title={t('radar.answeringTip')}
            aria-label={t('radar.answering')}
            onClick={() => onAnswering(!answering)}
          >
            <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
              <path d="M4 8h12l-3-3M20 16H8l3 3" />
            </svg>
          </button>
        ) : null}
        {on ? (
        <button
          type="button"
          className="radar-clear radar-clear--icon"
          title={t('radar.clearTip')}
          aria-label={t('radar.clear')}
          onClick={() => {
            clearBandSignals();
            void clearDigiBand().catch(() => undefined);
          }}
        >
          <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M3 6h18M8 6V4h8v2M6 6l1 14h10l1-14M10 11v6M14 11v6" />
          </svg>
        </button>
        ) : null}
      </header>
      {on ? (
      <div className="radar-scroll">
        <table ref={cols.tableRef} className="radar-grid mv-grid" style={cols.tableStyle}>
          <MovableColGroup cols={cols} />
          <thead onContextMenu={cols.openMenu}>
            <tr>
              {cols.visible.map((c, i) => (
                <MovableTh key={c} cols={cols} col={c} index={i}>
                  <span title={headTip[c]}>{head[c]}</span>
                </MovableTh>
              ))}
            </tr>
          </thead>
          <tbody>
            {stations.length === 0 ? (
              <tr>
                <td className="radar-empty" colSpan={cols.visible.length}>
                  {t('radar.empty')}
                </td>
              </tr>
            ) : (
              stations.map((s) => {
                const tuned = Math.abs(s.freqHz - rxHz) < 12;
                return (
                  <tr
                    key={s.call}
                    className={`radar-c${s.color}${tuned ? ' radar-on' : ''}${s.stale ? ' radar-stale' : ''}`}
                    title={`${s.country ? `${s.country} · ` : ''}${t('radar.rowTip')}`}
                    aria-selected={tuned}
                    onClick={() => onTune(s.freqHz)}
                    onDoubleClick={() => {
                      onTune(s.freqHz);
                      onCall(s.call);
                    }}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      setMenu({ x: e.clientX, y: e.clientY, s });
                    }}
                  >
                    {cols.visible.map((c) => cell(c, s))}
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
      ) : null}
      {on && (pileup.dx || (qsoDx && onPileupTx)) ? (
        <div className="radar-pileup" title={pileup.dx ? [t('radar.pileupTip'), ...pileup.worked.map((w) => `${w.call} ${fmt(w.hz)} ${new Date(w.at).toISOString().slice(11, 19)}`)].join('\n') : undefined}>
          {pileup.dx ? (
            <>
              <span className="radar-pileup-text">
                {pileup.estimateHz !== null
                  ? t('radar.pileupListen', { call: pileup.dx, f: fmt(pileup.estimateHz) })
                  : pileup.hintHz !== null
                    ? t('radar.pileupHint', { call: pileup.dx, f: fmt(pileup.hintHz) })
                    : t('radar.pileupWait', { call: pileup.dx })}
                {pileup.drift !== 'none' ? <em> {t(pileup.drift === 'up' ? 'radar.pileupUp' : 'radar.pileupDown')}</em> : null}
              </span>
              {pileup.worked.length > 0 ? <small className="radar-pileup-last">{pileup.worked.slice(0, 4).map((w) => w.call).join(' · ')}</small> : null}
              {pileTarget !== null && onPileupTx ? (
                <button type="button" className="radar-clear" title={t('radar.pileupTxTip')} onClick={() => onPileupTx(pileTarget)}>
                  {t('radar.pileupTx')}
                </button>
              ) : null}
              <button type="button" className="radar-clear" title={t('radar.pileupOffTip')} onClick={() => setPileupDx(null)}>
                {t('radar.pileupOff')}
              </button>
            </>
          ) : (
            <button type="button" className="radar-clear" title={t('radar.pileupFromQsoTip')} onClick={() => setPileupDx(qsoDx)}>
              {t('radar.pileupFromQso', { call: qsoDx })}
            </button>
          )}
        </div>
      ) : null}
      <ColumnsMenu cols={cols} />
      <PopupMenu at={menu} onClose={closeMenu} className="call-menu" width={220} height={125} label={menu?.s.call}>
        {menu ? (
          <>
            <button
              type="button"
              role="menuitem"
              title={t('radar.pileupDxTip')}
              onClick={() => {
                setPileupDx(menu.s.call);
                setMenu(null);
              }}
            >
              {t('radar.pileupDx')}
            </button>
            <button
              type="button"
              role="menuitem"
              disabled={blocked}
              title={blocked ? t('radar.spotOffTip') : t('radar.spotTip')}
              onClick={() => {
                setSpot(menu.s);
                setMenu(null);
              }}
            >
              {t('radar.spot')}
            </button>
            <button
              type="button"
              role="menuitem"
              title={t('radar.copyTip')}
              onClick={() => {
                const call = menu.s.call;
                setMenu(null);
                void navigator.clipboard?.writeText(call).then(() => toast(t('radar.copied', { call }), 'ok'), () => undefined);
              }}
            >
              {t('radar.copyCall')}
            </button>
          </>
        ) : null}
      </PopupMenu>
      {spot ? <SpotDialog call={spot.call} dialHz={dialHz} audioHz={spot.freqHz} modeName={modeName} lsb={lsb} khzDecimals={2} onClose={() => setSpot(null)} /> : null}
      {on && onHeightDrag ? <div className="radar-hgrip" role="separator" aria-orientation="horizontal" title={t('radar.resizeH')} onPointerDown={onHeightDrag} onDoubleClick={onHeightReset} /> : null}
    </section>
    </div>
  );
}
