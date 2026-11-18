import { useT } from '../../i18n';
import { formatRadioFrequency, hasFrequency, mainMeter, meterReadouts, type DisplayModel } from '../../lib/radioDisplay';
import { useCatExtras, useTuning } from './RadioExtras';

/**
 * Il display compatto per gli schermi stretti (sotto 1500 punti): niente display grande, che in testata non entrerebbe, ma una riga sola con
 * la frequenza della radio, la lettura principale (S-meter in ricezione, potenza in trasmissione), la potenza e il ROS, e il tasto TUNE
 * dell'accordatore della radio (solo se la radio ce l'ha). Sostituisce lo stile scelto: la scelta resta salvata e torna su uno schermo largo.
 */
export function CompactRadioDisplay({ model }: { model: DisplayModel }) {
  const t = useT();
  const { cat, rig, waiting, click } = useCatExtras();
  const tuning = useTuning();
  const main = mainMeter(model);
  const readouts = meterReadouts(model.meters);
  const power = readouts.find((r) => r.key === 'PO')?.text ?? '—';
  const swr = readouts.find((r) => r.key === 'SWR')?.text ?? '—';
  // come sotto il display grande: la radio dice di avere l'accordatore, oppure non si leggono i dati in piu' e non si sa
  const hasTuner = rig?.extras?.tuner != null || (cat.data !== undefined && cat.data.config.read_extras !== true && cat.data.config.backend !== 'none');
  const freq = hasFrequency(model.dialHz) ? formatRadioFrequency(model.dialHz) : '--.---.--';
  return (
    <div className={`rd-compact${model.transmitting ? ' rd-compact--tx' : ''}`} role="group" aria-label={t('rd.compactLabel')}>
      <span className="rd-compact-freq data" title={t('rd.compactFreq')}>
        {freq}
      </span>
      <span className="rd-compact-mode">{model.mode}</span>
      <span className="rd-compact-read" title={main.kind === 'S' ? 'S-meter' : 'PO'}>
        <small>{main.kind === 'S' ? 'S' : 'PO'}</small> {main.label ?? '—'}
      </span>
      <span className="rd-compact-read" title="PO">
        <small>PO</small> {power}
      </span>
      <span className="rd-compact-read" title="SWR">
        <small>SWR</small> {swr}
      </span>
      {hasTuner ? (
        <button
          type="button"
          className={`rd-compact-tune${tuning ? ' rd-compact-tune--on' : ''}`}
          title={t('rx.act.tune')}
          aria-label={t('rx.act.tune')}
          aria-pressed={tuning}
          disabled={waiting !== null}
          onClick={() => void click('tune')}
        >
          TUNE
        </button>
      ) : null}
    </div>
  );
}
