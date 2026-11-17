import { useEffect, useRef, type KeyboardEvent, type ReactNode } from 'react';
import { useT } from '../../../i18n';
import { multLabel, multName } from '../ContestStatsDialog';
import type { Qso } from '../DigiLog';
import { SaveFailAlert } from '../SaveFailAlert';
import { EXTRA_MAX, cleanExtra, extraLabelKey } from '../../../lib/contestSection';
import type { ContestEntryState } from './useContestEntry';

export interface EntryCardProps {
  /** Cio' che si sa del nominativo scritto (`useContestEntry`). */
  entry: ContestEntryState;
  qso: Qso;
  rsts: string;
  rstr: string;
  /** Lo scambio che parte da solo se `Nr inviato` non e' stato scritto (progressivo o scambio fisso del contest). */
  nrsAuto: string;
  /** Il numero del QSO in corso nel contest (QSO a log del contest + 1). */
  qsoNr: number;
  contestName: string | null;
  windowNotice: boolean;
  onEditWindow(): void;
  onQso(patch: Partial<Qso>): void;
  onRst(patch: { rsts?: string; rstr?: string }): void;
  /** Registra il QSO (e azzera i campi). */
  onRegister(): void;
  band: string;
  /** Perche' l'ultimo salvataggio e' stato rifiutato: l'avviso resta finche' non si cambia nominativo, si salva o si preme Esc. */
  saveError?: string | null;
  onDismissSaveError?(): void;
  /** Dentro una scheda spostabile: senza riquadro e titolo propri (li da' la scheda). */
  bare?: boolean;
  /** Il riquadro, per chi deve misurarlo (dove parte la finestra del rotore). */
  sectionRef?: React.RefObject<HTMLElement | null>;
}

/**
 * L'inserimento di un QSO (nominativo, numero inviato e ricevuto, casella extra dello scambio o nome e QTH senza contest, avvisi di doppio e
 * di moltiplicatore nuovo, suggerimenti, moltiplicatori e QSO precedenti del nominativo). Tab e Invio passano di campo, Invio nell'ultimo registra.
 */
export function EntryCard({ entry, qso, rsts, rstr, nrsAuto, qsoNr, contestName, windowNotice, onEditWindow, onQso, onRst, onRegister, band, saveError = null, onDismissSaveError, bare = false, sectionRef }: EntryCardProps) {
  const t = useT();
  const contestOn = contestName !== null;
  const callRef = useRef<HTMLInputElement>(null);
  const form = useRef<HTMLDivElement>(null);
  const { typed, known, c, showExtra, extraNow, suggestions, prevRows, callMults } = entry;
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
    <section className={`ssbc-entry${bare ? ' ssbc-entry--card' : ''}`} ref={sectionRef} aria-label={t('ssbc.enter')}>
      {bare ? null : (
        <h3>
          {t('ssbc.enter')}
          {contestName ? <b className="ssbc-contest-name"> · {contestName}</b> : null} <span className="ssbc-hint">· {t('ssbc.enterHint')}</span>
        </h3>
      )}
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
  );
}

