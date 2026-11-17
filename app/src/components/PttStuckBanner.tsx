import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { postHalt } from '../api/client';
import type { StationStatus, TxAlert } from '../api/schemas';
import { useT } from '../i18n';
import { playSound } from '../lib/soundBank';
import { useStation } from '../state/station';

/** Ogni quanti ms il suono di avviso si ripete finche' l'avviso e' sullo schermo e non e' stato ignorato. */
export const PTT_STUCK_SOUND_MS = 8000;
const PTT_STUCK_SOUND = 'alarm';
const PTT_STUCK_VOLUME = 0.8;

/** L'avviso «PTT non spento» dello stato della stazione, se c'e': il motore lo tiene finche' il PTT non risulta spento davvero. */
export function pttStuckAlert(status: StationStatus | null): TxAlert | null {
  return status?.tx_alerts?.find((a) => a.kind === 'ptt_stuck') ?? null;
}

/**
 * Avviso rosso, fisso, su tutte le pagine: il PTT non si e' spento (radio scollegata, cavo USB, CAT caduto) e la radio potrebbe essere in
 * trasmissione senza che l'operatore se ne accorga. Suona (anche con gli avvisi sonori spenti: e' un avviso di sicurezza) e si ripete.
 * «Riprova a spegnere» chiede subito un tentativo al programma (e ferma ogni trasmissione); «Ignora» lo nasconde fino al prossimo guasto.
 * Il blocco della trasmissione non si ignora: dura finche' il PTT non e' confermato spento.
 */
export function PttStuckBanner() {
  const t = useT();
  const alert = useStation((s) => pttStuckAlert(s.status));
  const [ignored, setIgnored] = useState(false);
  const [busy, setBusy] = useState(false);
  const active = alert !== null;
  // appena il PTT torna spento l'avviso sparisce, e il prossimo guasto si mostra di nuovo anche se questo era stato ignorato
  useEffect(() => {
    if (!active) setIgnored(false);
  }, [active]);
  const sounding = active && !ignored;
  useEffect(() => {
    if (!sounding) return undefined;
    const ring = () => void playSound(PTT_STUCK_SOUND, PTT_STUCK_VOLUME);
    ring();
    const id = window.setInterval(ring, PTT_STUCK_SOUND_MS);
    return () => window.clearInterval(id);
  }, [sounding]);
  if (!alert || ignored) return null;
  const retry = () => {
    setBusy(true);
    postHalt()
      .catch(() => undefined)
      .finally(() => setBusy(false));
  };
  // fuori da `.app-shell`: con le Impostazioni aperte la pagina sotto e' disattivata (inert) e i pulsanti non si potrebbero premere
  return createPortal(
    <div className="ptt-stuck" role="alert" aria-live="assertive">
      <p className="ptt-stuck-title">{t('tx.pttStuck.title')}</p>
      <p className="ptt-stuck-detail">{t('tx.pttStuck.detail', { reason: alert.detail ?? '', attempts: Math.round(alert.value ?? 0), secs: Math.round(alert.limit ?? 0) })}</p>
      <div className="ptt-stuck-actions">
        <button type="button" className="ptt-stuck-retry" disabled={busy} onClick={retry}>
          {t('tx.pttStuck.retry')}
        </button>
        <button type="button" className="ptt-stuck-ignore" onClick={() => setIgnored(true)}>
          {t('tx.pttStuck.ignore')}
        </button>
      </div>
    </div>,
    document.body,
  );
}

/**
 * Il PTT di una radio che non c'e' piu' (cavo tolto, profilo cambiato) non e' stato confermato spento e Aurora ha smesso di provarci. Avviso
 * rosso, persistente nello stato (non si perde ricaricando la pagina, ogni abbandono ne fa uno), con un suono breve quando compare. Si
 * chiude con un clic; il motore lo toglie alla prossima trasmissione conclusa con il PTT spento. La trasmissione con la radio nuova e' libera.
 */
export function PttAbandonedNotice() {
  const t = useT();
  const alert = useStation((s) => s.status?.tx_alerts?.find((a) => a.kind === 'ptt_abandoned') ?? null);
  const [closed, setClosed] = useState(false);
  const active = alert !== null;
  // finche' l'avviso e' nello stato la chiusura vale; tolto e ricomparso (un altro abbandono) si mostra di nuovo
  useEffect(() => {
    if (!active) setClosed(false);
  }, [active]);
  useEffect(() => {
    if (active) void playSound(PTT_STUCK_SOUND, PTT_STUCK_VOLUME);
  }, [active]);
  if (!alert || closed) return null;
  return createPortal(
    <div className="ptt-stuck ptt-stuck--once" role="alert" aria-live="assertive">
      <p className="ptt-stuck-title">{t('tx.pttAbandoned.title')}</p>
      <div className="ptt-stuck-actions">
        <button type="button" className="ptt-stuck-retry" onClick={() => setClosed(true)}>
          {t('diag.close')}
        </button>
      </div>
    </div>,
    document.body,
  );
}
