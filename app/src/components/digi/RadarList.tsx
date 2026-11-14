import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { clearDigiBand, getLogQsos } from '../../api/client';
import { useT } from '../../i18n';
import { clearBandSignals } from '../../lib/digiBand';
import { ColumnsMenu, MovableColGroup, MovableTh, useMovableColumns } from '../MovableColumns';
import type { RadarStation } from '../../lib/digiBand';
import { flagCode } from '../../lib/flags';
import { bandOf } from '../../lib/format';
import { buildLogIndex, logStatus, type LogStatus } from '../../lib/logStatus';
import '../../styles/digi-radar.css';

interface Props {
  /** Acceso (decodifica tutta la banda) e chi lo accende o spegne: spento resta solo la barra con l'interruttore. */
  on?: boolean;
  onToggle?(): void;
  /** Larghezza scelta con la maniglia (px). */
  width?: number;
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
}

/** Le colonne del Radar: si spostano trascinando il titolo e si allargano dal bordo, come nel DX Cluster; ordine e larghezze restano ricordati. */
const RADAR_COLUMNS = ['call', 'freq', 'wpm', 'snr', 'cq'] as const;
type RadarCol = (typeof RADAR_COLUMNS)[number];
const RADAR_COL_EM: Record<RadarCol, number> = { call: 9, freq: 6.5, wpm: 3.6, snr: 3.6, cq: 3.2 };

/** Il segno del log accanto al nominativo: paese nuovo, gia' collegato su banda e modo, collegato altrove. */
const LOG_MARK: Record<Exclude<LogStatus, null>, { label: string; tip: 'radar.logNewTip' | 'radar.logWorkedTip' | 'radar.logOtherTip' }> = {
  new_dxcc: { label: 'NEW', tip: 'radar.logNewTip' },
  worked: { label: 'B4', tip: 'radar.logWorkedTip' },
  worked_other: { label: 'B4', tip: 'radar.logOtherTip' },
};

function freqLabel(hz: number, dialHz: number): string {
  return dialHz > 0 ? ((dialHz + hz) / 1000).toFixed(2) : `${Math.round(hz)} Hz`;
}

/**
 * Il Radar: le stazioni della banda decodificate con sicurezza, una riga per nominativo, in una griglia a colonne fisse. Ogni stazione ha
 * il suo colore. Clic: la ricezione va sulla stazione; doppio clic: ricezione sulla stazione e nominativo nel QSO. Accanto al nominativo il
 * segno del log locale: paese nuovo, gia' collegato su questa banda e modo, collegato altrove (colori dei tipi della pagina FT8).
 */
export function RadarList({ on = true, onToggle, width, answering = false, onAnswering, stations, showWpm, showSnr, dialHz, rxHz, modeName, onTune, onCall }: Props) {
  const t = useT();
  const hidden: RadarCol[] = [...(showWpm ? [] : (['wpm'] as const)), ...(showSnr ? [] : (['snr'] as const))];
  const cols = useMovableColumns({ storageKey: 'aurora.digi.radar.cols', columns: RADAR_COLUMNS, defaultEm: RADAR_COL_EM, elastic: 'call', hidden });
  const log = useQuery({ queryKey: ['log-qsos'], queryFn: getLogQsos, retry: false, refetchOnWindowFocus: false });
  const index = useMemo(() => (log.data ? buildLogIndex(log.data.qsos) : null), [log.data]);
  const band = dialHz > 0 ? bandOf(dialHz) : null;
  const head: Record<RadarCol, string> = { call: t('radar.colCall'), freq: t('radar.colFreq'), wpm: 'WPM', snr: 'S/N', cq: 'CQ' };
  const cell = (c: RadarCol, s: RadarStation) => {
    switch (c) {
      case 'call': {
        const code = flagCode(s.country);
        const st = logStatus(index, s.call, s.country, band, modeName);
        const mark = st ? LOG_MARK[st] : null;
        return (
          <td key={c} className="radar-call">
            <span className="radar-dot" title={s.stale ? t('radar.dotStale') : t('radar.dotLive')} aria-label={s.stale ? t('radar.dotStale') : t('radar.dotLive')} />
            {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}
            <b>{s.call}</b>
            {st && mark ? (
              <span className={`radar-log radar-log--${st.replace('_', '-')}`} title={t(mark.tip)}>
                {mark.label}
              </span>
            ) : null}
          </td>
        );
      }
      case 'freq':
        return (
          <td key={c} className="radar-num">
            {freqLabel(s.freqHz, dialHz)}
          </td>
        );
      case 'wpm':
        return (
          <td key={c} className="radar-num">
            {s.wpm != null ? Math.round(s.wpm) : ''}
          </td>
        );
      case 'snr':
        return (
          <td key={c} className="radar-num">
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
    <section className={`radar${on ? '' : ' radar--off'}`} style={on && width ? { width } : undefined} aria-label={`${t('radar.title')} ${modeName}`}>
      <header className="radar-head">
        {onToggle ? (
          <button type="button" role="switch" aria-checked={on} className={`radar-switch${on ? ' radar-switch--on' : ''}`} title={t('radar.tip')} onClick={onToggle}>
            {on ? 'ON' : 'OFF'}
          </button>
        ) : null}
        <strong>
          {t('radar.title')} {modeName}
        </strong>
        {on ? <small>{t('radar.count', { n: stations.length })}</small> : null}
        {on ? (
          <span className="radar-legend" title={`NEW: ${t('radar.logNewTip')}
B4: ${t('radar.logWorkedTip')}
B4: ${t('radar.logOtherTip')}`}>
            <span className="radar-log radar-log--new-dxcc">NEW</span>
            <span className="radar-log radar-log--worked">B4</span>
            <span className="radar-log radar-log--worked-other">B4</span>
          </span>
        ) : null}
        {on && onAnswering ? (
          <label className="radar-answering" title={t('radar.answeringTip')}>
            <input type="checkbox" checked={answering} onChange={(e) => onAnswering(e.target.checked)} />
            {t('radar.answering')}
          </label>
        ) : null}
        {on ? (
        <button
          type="button"
          className="radar-clear"
          title={t('radar.clearTip')}
          onClick={() => {
            clearBandSignals();
            void clearDigiBand().catch(() => undefined);
          }}
        >
          {t('radar.clear')}
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
                  {head[c]}
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
      <ColumnsMenu cols={cols} />
    </section>
  );
}
