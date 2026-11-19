import { useState, useSyncExternalStore } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { ApiError, getCat, postCatExtra, type CatExtraItem } from '../../api/client';
import { useT } from '../../i18n';
import { coreText } from '../../lib/coreText';
import { toast } from '../../lib/toast';
import { formatVfo } from '../../lib/radioExtras';
import { displayModel } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';

/** Cosa fa il clic su ogni indicatore (suggerimento ed etichetta per gli screen reader). */
const ACTION_TIP = {
  preamp: 'rx.act.preamp',
  att: 'rx.act.att',
  agc: 'rx.act.agc',
  nb: 'rx.act.nb',
  nr: 'rx.act.nr',
  notch: 'rx.act.notch',
  atu: 'rx.act.atu',
  tune: 'rx.act.tune',
  pwr_up: 'rx.act.pwrUp',
  pwr_down: 'rx.act.pwrDown',
  vfo: 'rx.act.vfo',
  rit: 'rx.act.rit',
  xit: 'rx.act.xit',
} as const;

/**
 * Lettura dei dati in piu' della radio e clic sugli indicatori, condivisi dal display esteso e dai display a pannello unico
 * (FT-2000, IC-7610): l'indicatore appena cliccato resta in attesa finche' la radio non ha risposto.
 */
/** Quanto resta accesa la scritta TX dopo «TUNE»: la radio accorda da sola (qualche secondo di portante) e il CAT non lo dice. */
export const TUNE_TX_MS = 6000;
const tuneTx = { until: 0, subs: new Set<() => void>() };

/** Segna l'accordatura appena avviata: il display accende TX per {@link TUNE_TX_MS}. */
export function markTuning(now = Date.now()) {
  tuneTx.until = now + TUNE_TX_MS;
  tuneTx.subs.forEach((f) => f());
  setTimeout(() => tuneTx.subs.forEach((f) => f()), TUNE_TX_MS + 50);
}

/** La radio sta accordando dopo un «TUNE» del display. */
export function useTuning(): boolean {
  return useSyncExternalStore(
    (f) => {
      tuneTx.subs.add(f);
      return () => tuneTx.subs.delete(f);
    },
    () => Date.now() < tuneTx.until,
  );
}

export function useCatExtras() {
  const t = useT();
  const cat = useQuery({ queryKey: ['cat'], queryFn: getCat, retry: false, refetchInterval: 3000 });
  const queryClient = useQueryClient();
  const [waiting, setWaiting] = useState<CatExtraItem | null>(null);
  const click = async (item: CatExtraItem) => {
    if (waiting !== null) return;
    setWaiting(item);
    try {
      await postCatExtra(item);
      // l'accordatore della radio trasmette da solo: TX acceso sul display mentre accorda
      if (item === 'tune' && cat.data?.config.backend !== 'tci') markTuning();
    } catch (e) {
      const code = e instanceof ApiError ? e.code : '';
      const detail = e instanceof ApiError && e.detail ? coreText(e.detail, t) : undefined;
      if (code === 'local_only') toast(t('rx.err.local'), 'error');
      else if (code === 'transmitting') toast(t('rx.err.tx'), 'error');
      else if (code === 'unsupported') toast(detail ?? t('rx.err.unsupported'), 'error');
      else toast(detail ?? t('rx.err.generic'), 'error');
    } finally {
      setWaiting(null);
      await queryClient.invalidateQueries({ queryKey: ['cat'] });
    }
  };
  return { cat, rig: cat.data?.status.rig, waiting, click };
}

/** Suggerimento del clic su un indicatore. */
export function actionTip(item: CatExtraItem) {
  return ACTION_TIP[item];
}

/**
 * Sotto il display resta poco: la frequenza del secondo VFO (se la radio la da') e il tasto TUNE. Preamplificatore, ATT, AGC, NB/NR, guadagni, RIT/XIT, VFO,
 * alimentazione, ALC e compressione non si mostrano piu' (scelta del proprietario, 2026-11-18): meno richieste alla radio e niente «clic» dei misuratori.
 */
export function RadioExtras({ hidePwr = false, hideVfos = false }: { hidePwr?: boolean; hideVfos?: boolean } = {}) {
  const t = useT();
  const { cat, rig, waiting, click } = useCatExtras();
  const tuning = useTuning();
  const x = rig?.extras ?? {};
  const dialHz = displayModel(useStation((s) => s.status)).dialHz;
  void hidePwr;
  const vfoB = !hideVfos && x.vfo_b_hz ? `${formatVfo(x.vfo_b_hz)}${x.vfo_b_mode ? ` ${x.vfo_b_mode}` : ''}` : null;
  // TUNE: la radio dice di avere l'accordatore, oppure non si leggono i dati in piu' e non si sa (se non lo ha, il clic spiega perche')
  const showTune = x.tuner != null || (cat.data !== undefined && cat.data.config.read_extras !== true && cat.data.config.backend !== 'none');
  if (!vfoB && !showTune) return null;
  return (
    <div className="rx-extras" aria-label={t('rx.grp')}>
      {vfoB ? (
        <div className="rx-vfos" title={t('tip.rx.vfoB')}>
          {dialHz > 0 ? (
            <div className="rx-vfob rx-vfob--a">
              <span className="rx-vfob-tag">VFOA</span>
              <span className="rx-vfob-freq">{formatVfo(dialHz)}</span>
            </div>
          ) : null}
          <div className="rx-vfob">
            <span className="rx-vfob-tag">VFOB</span>
            <span className="rx-vfob-freq">{vfoB}</span>
          </div>
        </div>
      ) : null}
      {showTune ? (
        <div className="rx-chips">
          <button
            type="button"
            className={`rx-chip rx-chip--btn${tuning ? ' rx-chip--on' : ''}${waiting === 'tune' ? ' rx-chip--wait' : ''}`}
            title={t(ACTION_TIP.tune)}
            aria-label={`TUNE: ${t(ACTION_TIP.tune)}`}
            aria-busy={waiting === 'tune'}
            disabled={waiting !== null}
            onClick={() => void click('tune')}
          >
            TUNE
          </button>
        </div>
      ) : null}
    </div>
  );
}
