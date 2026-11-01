import { RadioExtras } from './RadioExtras';
import { usePhone } from '../../lib/phone';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, useRadioDisplay } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';
import { RadioDisplay } from './RadioDisplay';

/**
 * Il display in testata: piccolo, fisso, senza popup (passandoci sopra o cliccando non si apre niente).
 * Con "Nascondi il display" non c'e' affatto.
 */
export function RadioDisplayHeader() {
  const phone = usePhone();
  const pref = useRadioDisplay((s) => s.pref);
  const status = useStation((s) => s.status);
  const raw = displayModel(status);
  const meters = useMeterBallistics(raw.meters);
  if (pref.hidden) return null;
  const model = { ...raw, meters };

  return (
    <div className={phone ? 'rd-header rd-header--phone' : 'rd-header'}>
      <div className="rd-trigger rd-trigger--static">
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </div>
      <RadioExtras />
    </div>
  );
}
