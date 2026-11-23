import { useT } from '../../i18n';
import { useMeterBallistics } from '../../lib/meterBallistics';
import { displayModel, useRadioDisplay } from '../../lib/radioDisplay';
import { useStation } from '../../state/station';

/** Fondo scala della potenza: il primo valore tondo sopra la lettura (la radio non dice la sua potenza massima). */
export function powerCeiling(w: number): number {
  return [10, 25, 50, 100, 200, 500, 1500].find((c) => w <= c) ?? 1500;
}

/** Frazione di barra del ROS: 1,0 vuota, 3,0 piena. */
export function swrFraction(swr: number): number {
  return Math.min(1, Math.max(0, (swr - 1) / 2));
}

/** Sopra questo ROS la barra si fa rossa (lo stesso 2,0 dell'avviso di trasmissione). */
export const SWR_WARN = 2;
/** Sopra questo livello l'ALC e' alto: si abbassa il volume dell'audio. */
export const ALC_WARN = 0.6;
export function alcWarn(alc: number): boolean {
  return alc >= ALC_WARN;
}

function Tile({ name, value, fraction, bad }: { name: string; value: string; fraction: number; bad: boolean }) {
  return (
    <div className={`tx-big-tile${bad ? ' tx-big-tile--bad' : ''}`}>
      <small>{name}</small>
      <b className="data">{value}</b>
      <span className="tx-big-bar" aria-hidden="true">
        <i style={{ width: `${Math.round(Math.min(1, Math.max(0, fraction)) * 100)}%` }} />
      </span>
    </div>
  );
}

/**
 * Pannello fisso con potenza in uscita, ROS e ALC in grande, per leggerli da lontano mentre si regola l'audio. Compare solo se la radio
 * da almeno uno di questi strumenti (via CAT) ed e' stato scelto nelle Impostazioni.
 */
export function TxMetersBig() {
  const t = useT();
  const on = useRadioDisplay((s) => s.pref.bigMeters);
  const status = useStation((s) => s.status);
  const meters = useMeterBallistics(displayModel(status).meters);
  if (!on || !meters || meters.estimated) return null;
  const { power_w: po, swr, alc } = meters;
  if (po === undefined && swr === undefined && alc === undefined) return null;
  return (
    <div className="tx-big" role="group" aria-label={t('rd.big.label')}>
      {po !== undefined ? <Tile name={t('rd.big.po')} value={`${po < 10 ? po.toFixed(1) : Math.round(po)} W`} fraction={po / powerCeiling(po)} bad={false} /> : null}
      {swr !== undefined ? <Tile name={t('rd.big.swr')} value={swr.toFixed(1)} fraction={swrFraction(swr)} bad={swr >= SWR_WARN} /> : null}
      {alc !== undefined ? <Tile name={t('rd.big.alc')} value={`${Math.round(alc * 100)} %`} fraction={alc} bad={alcWarn(alc)} /> : null}
    </div>
  );
}
