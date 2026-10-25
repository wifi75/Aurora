import type { T } from '../../i18n';
import { alcFraction, formatRadioFrequency, hasFrequency, meterReadouts, sLabel, type DisplayModel } from '../../lib/radioDisplay';

/** Testo alternativo del display (per chi non lo vede): frequenza, modo, RX o TX e gli strumenti che la radio da'. */
export function describeDisplay(model: DisplayModel, t: T): string {
  if (!hasFrequency(model.dialHz)) return t('rd.noRadio');
  const parts: string[] = [t('rd.alt.freq', { freq: formatRadioFrequency(model.dialHz) })];
  if (model.mode) parts.push(t('rd.alt.mode', { mode: model.mode }));
  parts.push(model.transmitting ? t('rd.alt.tx') : t('rd.alt.rx'));
  const s = sLabel(model.meters);
  if (s) parts.push(t('rd.alt.s', { value: s }));
  for (const r of meterReadouts(model.meters)) {
    if (r.key === 'PO') parts.push(t('rd.alt.po', { value: r.text.replace(' W', '') }));
    else if (r.key === 'SWR') parts.push(t('rd.alt.swr', { value: r.text }));
    else if (r.key === 'ALC') parts.push(t('rd.alt.alc', { value: String(Math.round((alcFraction(model.meters) ?? 0) * 100)) }));
    else if (r.key === 'COMP') parts.push(t('rd.alt.comp', { value: r.text.replace(' dB', '') }));
  }
  if (!s && meterReadouts(model.meters).length === 0) parts.push(t('rd.alt.noMeters'));
  return parts.join(', ');
}
