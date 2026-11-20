import { bandOf } from '../../lib/format';
import { displayModel } from '../../lib/radioDisplay';
import { registerSpectrum } from '../../lib/spectrumFeed';
import { useStation } from '../../state/station';
import { CompactRadioDisplay } from '../radio-display/CompactRadioDisplay';
import { Waterfall } from '../Waterfall';

/**
 * Display della radio e piccola cascata sulla riga del selettore delle frequenze (a destra): si vede la frequenza e il segnale senza
 * abbassare la pagina. La cascata ha altezza e larghezza fisse e piccole; sotto i 760 px si nasconde (resta il display).
 */
export function WefaxMini() {
  const status = useStation((s) => s.status);
  const model = displayModel(status);
  return (
    <div className="wefax-mini">
      <CompactRadioDisplay model={model} />
      <div className="wefax-mini-wf" aria-label="Waterfall">
        <Waterfall register={registerSpectrum} mode={status?.mode} band={status ? (bandOf(status.dial_hz) ?? undefined) : undefined} />
      </div>
    </div>
  );
}
