import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { getDigiBand } from '../../api/client';
import type { DigiSignal } from '../../api/schemas';
import { useT } from '../../i18n';
import { mainCall, splitUnsure } from '../../lib/digiBand';
import { flagCode } from '../../lib/flags';
import '../../styles/digi-band.css';

/** Caratteri dell'ultimo testo in una riga. */
const ROW_CHARS = 44;

interface Props {
  signals: DigiSignal[];
  /** CW: c'e' la colonna della velocita'. */
  showWpm: boolean;
  /** Frequenza della radio (Hz): con la radio nota la frequenza vera in kHz, se no il tono audio in Hz. */
  dialHz: number;
  /** La frequenza di ricezione adesso: la riga del segnale ascoltato si evidenzia. */
  rxHz: number;
  /** Testo da mostrare quando non c'e' nessun segnale. */
  emptyText: string;
  onTune(hz: number): void;
  onCall(call: string): void;
}

function freqLabel(hz: number, dialHz: number): string {
  return dialHz > 0 ? ((dialHz + hz) / 1000).toFixed(2) : `${Math.round(hz)} Hz`;
}

/** Il testo con i caratteri incerti in grigio. */
function TextWithUnsure({ text, unsure }: { text: string; unsure: number[] }) {
  return (
    <>
      {splitUnsure(text, unsure).map((p, i) =>
        p.unsure ? (
          <span key={i} className="digi-band-unsure">
            {p.text}
          </span>
        ) : (
          <span key={i}>{p.text}</span>
        ),
      )}
    </>
  );
}

/** Nominativo con bandiera e ruolo (CQ, DE). */
function CallCell({ s }: { s: DigiSignal }) {
  const t = useT();
  const c = mainCall(s);
  if (!c) return null;
  const code = flagCode(c.country);
  return (
    <span className={`digi-band-call${c.verified ? '' : ' digi-band-call--maybe'}`} title={`${c.country ? `${c.country} · ` : ''}${c.verified ? t('digi.band.verified') : t('digi.band.notVerified')}`}>
      {code ? <span className={`fi fi-${code} decode-flag`} aria-hidden="true" /> : null}
      <b>{c.call}</b>
      {c.role === 'cq' || s.cq ? <em className="digi-band-role digi-band-role--cq">CQ</em> : c.role === 'de' ? <em className="digi-band-role">DE</em> : null}
    </span>
  );
}

/**
 * Il pannello «Segnali»: una riga per ogni segnale della banda audio (frequenza, velocita', S/N, ultimo testo con i caratteri incerti in
 * grigio, nominativo con bandiera e ruolo). Clic: la ricezione va sul segnale; doppio clic: lo storico di 60 s.
 */
export function BandSignals({ signals, showWpm, dialHz, rxHz, emptyText, onTune, onCall }: Props) {
  const t = useT();
  const [historyId, setHistoryId] = useState<number | null>(null);
  // l'avviso dei CQ: quante stazioni chiamano adesso (le loro righe sono evidenziate)
  const cqs = signals.filter((s) => s.cq).length;
  return (
    <section className="digi-band" aria-label={t('digi.band.title')}>
      <header className="digi-band-head">
        <strong>{t('digi.band.title')}</strong>
        <small>{signals.length > 0 ? t('digi.band.count', { n: signals.length }) : null}</small>
        {cqs > 0 ? <small className="digi-band-cqcount">{t('digi.band.cqCount', { n: cqs })}</small> : null}
        <small className="digi-band-legend">{t('digi.band.unsureTip')}</small>
      </header>
      {signals.length === 0 ? (
        <p className="digi-band-empty">{emptyText}</p>
      ) : (
        <div className="digi-band-scroll">
          <table>
            <thead>
              <tr>
                <th>{t('digi.band.colFreq')}</th>
                {showWpm ? <th>WPM</th> : null}
                <th>S/N</th>
                <th>{t('digi.band.colCall')}</th>
                <th>{t('digi.band.colText')}</th>
              </tr>
            </thead>
            <tbody>
              {signals.map((s) => {
                const chars = Array.from(s.text);
                const from = Math.max(0, chars.length - ROW_CHARS);
                const tail = chars.slice(from).join('');
                const unsure = s.unsure.filter((i) => i >= from).map((i) => i - from);
                const tuned = Math.abs(s.freq_hz - rxHz) < 12;
                return (
                  <tr
                    key={s.id}
                    className={`${s.cq ? 'digi-band-cq' : ''}${tuned ? ' digi-band-on' : ''}`}
                    title={t('digi.band.rowTip')}
                    aria-selected={tuned}
                    onClick={() => onTune(s.freq_hz)}
                    onDoubleClick={() => setHistoryId(s.id)}
                  >
                    <td className="digi-band-num">{freqLabel(s.freq_hz, dialHz)}</td>
                    {showWpm ? <td className="digi-band-num">{s.wpm != null ? Math.round(s.wpm) : ''}</td> : null}
                    <td className="digi-band-num">{Math.round(s.snr_db)}</td>
                    <td>
                      <CallCell s={s} />
                    </td>
                    <td className="digi-band-text">
                      <TextWithUnsure text={tail} unsure={unsure} />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      {historyId !== null ? <History id={historyId} dialHz={dialHz} onTune={onTune} onCall={onCall} onClose={() => setHistoryId(null)} /> : null}
    </section>
  );
}

/** Lo storico di un segnale: tutto il testo degli ultimi 60 secondi, aggiornato mentre la finestra e' aperta. */
function History({ id, dialHz, onTune, onCall, onClose }: { id: number; dialHz: number; onTune(hz: number): void; onCall(call: string): void; onClose(): void }) {
  const t = useT();
  const band = useQuery({ queryKey: ['digi-band'], queryFn: getDigiBand, refetchInterval: 2_000, retry: false });
  const s = band.data?.find((x) => x.id === id) ?? null;
  const call = s ? mainCall(s) : null;
  return (
    <div className="digi-modal" role="dialog" aria-modal="true" aria-label={t('digi.band.history')} onKeyDown={(e) => (e.key === 'Escape' ? onClose() : undefined)}>
      <div className="digi-dialog digi-band-history">
        <header>
          <strong>
            {t('digi.band.history')}
            {s ? ` · ${freqLabel(s.freq_hz, dialHz)}` : ''}
            {s?.wpm != null ? ` · ${Math.round(s.wpm)} WPM` : ''}
            {s ? ` · S/N ${Math.round(s.snr_db)} dB` : ''}
          </strong>
          {s ? <CallCell s={s} /> : null}
        </header>
        {band.isSuccess && !s ? <p className="digi-band-empty">{t('digi.band.historyGone')}</p> : null}
        {s ? (
          <pre className="digi-band-full">
            <TextWithUnsure text={s.text} unsure={s.unsure} />
          </pre>
        ) : null}
        <footer>
          {s ? (
            <button type="button" className="glass sstv-btn sstv-btn--primary" onClick={() => onTune(s.freq_hz)}>
              {t('digi.band.tune')}
            </button>
          ) : null}
          {call ? (
            <button type="button" className="glass sstv-btn" onClick={() => onCall(call.call)}>
              {t('digi.band.toQso')}
            </button>
          ) : null}
          <button type="button" className="glass sstv-btn" autoFocus onClick={onClose}>
            {t('digi.band.close')}
          </button>
        </footer>
      </div>
    </div>
  );
}
