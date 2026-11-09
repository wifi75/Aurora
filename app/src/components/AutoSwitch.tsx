import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, putAutomation } from '../api/client';
import { useT } from '../i18n';
import { toast } from '../lib/toast';
import { openSettingsWindow } from '../lib/settingsWindow';

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
  const toggle = () => {
    // si accende nel modo scelto nelle Impostazioni (prova a vuoto o reale): passare al modo reale chiede la conferma dell'avviso, che si da'
    // in Impostazioni → Automazioni. Prima il pulsante chiedeva sempre il modo reale senza conferma: il core rifiutava e non succedeva nulla
    const change = a.enabled ? { enabled: false } : { enabled: true };
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
        onClick={toggle}
      >
        {live ? t('autoSwitch.live') : dry ? t('autoSwitch.dry') : t('autoSwitch.off')}
      </button>
    </div>
  );
}
