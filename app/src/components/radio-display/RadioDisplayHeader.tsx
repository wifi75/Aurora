import { RadioExtras, useCatExtras } from './RadioExtras';
import { describeDisplay } from './describe';
import { Ft2000Display, Ic7610Display } from './RigFaceDisplays';
import { useT } from '../../i18n';
import { usePhone } from '../../lib/phone';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, isFaceStyle, useRadioDisplay, type DisplayModel } from '../../lib/radioDisplay';
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

  if (phone) {
    return (
      <div className="rd-header rd-header--phone">
        <div className="rd-trigger rd-trigger--static">
          <RadioDisplay model={model} style={pref.style} size="mini" />
        </div>
        <RadioExtras />
      </div>
    );
  }

  if (isFaceStyle(pref.style)) return <FaceHeader model={model} style={pref.style} />;

  // due blocchi uguali affiancati, oppure un blocco unico a doppio VFO (VFO A grande a sinistra, VFO B e indicatori a destra)
  return (
    <div className={`rd-header rd-duo rd-duo--${pref.layout}`}>
      <div className="rd-slot rd-trigger rd-trigger--static">
        <RadioDisplay model={model} style={pref.style} size="mini" />
      </div>
      <div className="rd-slot rd-slot--extras">
        <RadioExtras />
      </div>
    </div>
  );
}

/** Pannello unico in testata (FT-2000, IC-7610): stessa misura dei due blocchi, con gli indicatori cliccabili come nel display esteso. */
function FaceHeader({ model, style }: { model: DisplayModel; style: 'ft2000' | 'ic7610' }) {
  const t = useT();
  const { rig, waiting, click } = useCatExtras();
  const data = { extras: rig?.extras ?? {}, passbandHz: rig?.passband_hz ?? null, split: rig?.split ?? null };
  const props = { model, noRadio: t('rd.noRadio'), ariaLabel: describeDisplay(model, t), data, waiting, onAction: (item: Parameters<typeof click>[0]) => void click(item) };
  return <div className="rd-header rd-face-slot">{style === 'ft2000' ? <Ft2000Display {...props} /> : <Ic7610Display {...props} />}</div>;
}
