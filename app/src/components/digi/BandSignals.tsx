import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useRef, useState } from 'react';
import { clearDigiBand, getDigiBand } from '../../api/client';
import type { DigiSignal } from '../../api/schemas';
import { useT } from '../../i18n';
import { clearBandSignals, mainCall, splitUnsure } from '../../lib/digiBand';
import { flagCode } from '../../lib/flags';
import { browserStorage } from '../../lib/theme';
import { PopupMenu } from '../PopupMenu';
import '../../styles/digi-band.css';

/** Caratteri dell'ultimo testo in una riga. */
const ROW_CHARS = 44;
/** I livelli di convalida dei nominativi. */
const VALIDATIONS = [
  ['minimal', 'digi.band.valMinimal'],
  ['normal', 'digi.band.valNormal'],
  ['strict', 'digi.band.valStrict'],
  ['maximum', 'digi.band.valMaximum'],
] as const;

/** Dove si ricorda l'altezza dell'elenco scelta trascinando la maniglia (px). */
export const BAND_HEIGHT_KEY = 'aurora.digi.bandh';
/** Misure di riserva di una riga e dell'intestazione della tabella (px), quando non si possono misurare. */
const ROW_PX = 21;
const HEAD_PX = 22;
/** Quanto aspetta la scelta della convalida che il salvataggio torni, prima di rimostrare il valore salvato (ms). */
const PICK_WAIT_MS = 4000;

/** I limiti dell'altezza dell'elenco: almeno l'intestazione e tre righe, al massimo meta' della colonna (senza misura: 400 px). */
export function bandHeightLimits(rowPx: number, headPx: number, columnPx: number): { min: number; max: number } {
  const min = Math.round((headPx > 0 ? headPx : HEAD_PX) + 3 * (rowPx > 0 ? rowPx : ROW_PX));
  const max = Math.max(min, Math.round(columnPx > 0 ? columnPx / 2 : 400));
  return { min, max };
}

function readHeight(): number | null {
  const n = Number(browserStorage()?.getItem(BAND_HEIGHT_KEY) ?? '');
  return Number.isFinite(n) && n >= 40 && n <= 2000 ? n : null;
}

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
  /** Il livello di convalida dei nominativi (`minimal`, `normal`, `strict`, `maximum`) e chi lo cambia. */
  validation: string;
  onValidation(v: string): void;
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
export function BandSignals({ signals, showWpm, dialHz, rxHz, emptyText, validation, onValidation, onTune, onCall }: Props) {
  const t = useT();
  const [historyId, setHistoryId] = useState<number | null>(null);

  // la convalida scelta si mostra subito e resta finche' il salvataggio non torna (o non riesce: dopo un po' torna il valore salvato)
  const [picked, setPicked] = useState<string | null>(null);
  const [valMenu, setValMenu] = useState<{ x: number; y: number } | null>(null);
  const closeValMenu = useCallback(() => setValMenu(null), []);
  useEffect(() => {
    if (picked === null) return;
    if (picked === validation) {
      setPicked(null);
      return;
    }
    const timer = window.setTimeout(() => setPicked(null), PICK_WAIT_MS);
    return () => window.clearTimeout(timer);
  }, [picked, validation]);
  const shown = picked ?? validation;
  const shownKey = VALIDATIONS.find(([id]) => id === shown)?.[1] ?? 'digi.band.valNormal';

  // l'altezza dell'elenco: si trascina la maniglia sul bordo alto del pannello; senza una scelta cresce da sola fino a 9,5 rem
  const panel = useRef<HTMLElement>(null);
  const scroll = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState<number | null>(readHeight);
  const startResize = (e: React.PointerEvent) => {
    e.preventDefault();
    const box = scroll.current;
    const row = box?.querySelector('tbody tr')?.getBoundingClientRect().height ?? 0;
    const head = box?.querySelector('thead')?.getBoundingClientRect().height ?? 0;
    const { min, max } = bandHeightLimits(row, head, panel.current?.parentElement?.clientHeight ?? 0);
    const startY = e.clientY;
    const startH = box?.offsetHeight ?? 0;
    let last: number | null = null;
    const move = (m: PointerEvent) => {
      last = Math.round(Math.min(max, Math.max(min, startH + (startY - m.clientY))));
      setHeight(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      if (last === null) return;
      try {
        browserStorage()?.setItem(BAND_HEIGHT_KEY, String(last));
      } catch {
        // l'altezza vale comunque finche' la pagina e' aperta
      }
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };

  // «Pulisci»: l'elenco si svuota subito qui e nel core i canali ripartono da zero (i segnali ancora presenti ricompaiono da soli)
  const clear = () => {
    setHistoryId(null);
    clearBandSignals();
    void clearDigiBand().catch(() => undefined);
  };
  // l'avviso dei CQ: quante stazioni chiamano adesso (le loro righe sono evidenziate)
  const cqs = signals.filter((s) => s.cq).length;
  return (
    <section className="digi-band" aria-label={t('digi.band.title')} ref={panel}>
      <div className="digi-band-grip" role="separator" aria-orientation="horizontal" title={t('digi.band.resize')} onPointerDown={startResize} />
      <header className="digi-band-head">
        <strong>{t('digi.band.title')}</strong>
        <small>{signals.length > 0 ? t('digi.band.count', { n: signals.length }) : null}</small>
        {cqs > 0 ? <small className="digi-band-cqcount">{t('digi.band.cqCount', { n: cqs })}</small> : null}
        <small className="digi-band-legend">{t('digi.band.unsureTip')}</small>
        <span className="digi-band-validation" title={t('digi.band.validationTip')}>
          <span>{t('digi.band.validation')}</span>
          <button
            type="button"
            className="digi-band-pick"
            aria-haspopup="menu"
            aria-expanded={valMenu !== null}
            aria-label={`${t('digi.band.validation')}: ${t(shownKey)}`}
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              setValMenu(valMenu ? null : { x: r.left, y: r.bottom + 2 });
            }}
          >
            {t(shownKey)} <span aria-hidden="true">▾</span>
          </button>
        </span>
        <button type="button" className="digi-band-clear" title={t('digi.band.clearTip')} onClick={clear}>
          {t('digi.band.clear')}
        </button>
        <PopupMenu at={valMenu} onClose={closeValMenu} className="call-menu digi-st-menu" width={170} height={130} label={t('digi.band.validation')}>
          {VALIDATIONS.map(([id, key]) => (
            <button
              key={id}
              type="button"
              role="menuitemradio"
              aria-checked={id === shown}
              onClick={() => {
                setValMenu(null);
                if (id === shown) return;
                setPicked(id);
                onValidation(id);
              }}
            >
              <span className="digi-st-menu-check" aria-hidden="true">
                {id === shown ? '✓' : ''}
              </span>
              {t(key)}
            </button>
          ))}
        </PopupMenu>
      </header>
      <div className="digi-band-scroll" ref={scroll} style={height !== null ? { height, maxHeight: 'none' } : undefined}>
        {signals.length === 0 ? (
          <p className="digi-band-empty">{emptyText}</p>
        ) : (
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
        )}
      </div>
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
