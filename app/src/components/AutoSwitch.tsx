import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation } from '../api/client';
import { useT } from '../i18n';
import { toast } from '../lib/toast';
import { openSettingsWindow } from '../lib/settingsWindow';
import { askConfirm } from './ConfirmHost';

/**
 * Le Automazioni a portata di mano nella barra in alto (richiesta di ON7KEC): un clic le accende in modo reale o le spegne, senza aprire
 * le Impostazioni. Acceso = verde; in prova a vuoto = arancio. Se il modo reale non e' consentito (trasmissione chiusa) resta la prova a vuoto.
 */
export function AutoSwitch() {
  const t = useT();
  const qc = useQueryClient();
  const auto = useQuery({ queryKey: ['automation'], queryFn: getAutomation, refetchInterval: 5000, retry: false });
  const a = auto.data;
  if (!a) return null;
  const live = a.enabled && a.mode === 'live';
  const dry = a.enabled && a.mode !== 'live';
  const toggle = async () => {
    // acceso: si spegne subito. Spento in prova a vuoto: la stessa domanda di sicurezza delle Impostazioni direttamente qui, «Si'» accende in
    // modo reale, altrimenti resta la prova a vuoto; data la conferma, i clic dopo accendono e spengono il modo reale senza domande
    // (richiesta di Luc ON7KEC: non passare dal menu). Senza trasmissione consentita resta la prova a vuoto.
    let change: Parameters<typeof putAutomation>[0] = { enabled: !a.enabled };
    if (!a.enabled && a.mode !== 'live' && a.live_allowed) {
      change = (await askConfirm(t('auto.liveNotice'))) ? { enabled: true, mode: 'live', accept_license_notice: true } : { enabled: true };
    }
    void putAutomation(change)
      .then((next) => qc.setQueryData(['automation'], next))
      .catch(() => {
        toast(t('autoSwitch.failed'), 'error');
        openSettingsWindow('automation');
      });
  };
  return (
    <div className="rx-seg rx-seg--btns" role="group">
      <button
        type="button"
        role="switch"
        aria-checked={a.enabled}
        className={`rx-seg-item auto-switch${live ? ' auto-switch--live' : dry ? ' auto-switch--dry' : ''}`}
        title={t('tip.autoSwitch')}
        onClick={() => void toggle()}
      >
        {live ? t('autoSwitch.live') : dry ? t('autoSwitch.dry') : t('autoSwitch.off')}
      </button>
    </div>
  );
}
