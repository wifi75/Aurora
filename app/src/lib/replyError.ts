import { ApiError } from '../api/client';
import type { T } from '../i18n';

/** Messaggio per chi tocca un comando e non parte (nella lingua scelta). */
export function replyErrorMessage(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'transmitting') return t('err.transmitting');
    if (error.code === 'rig_error') return t('err.rig');
    if (error.code === 'rig_unavailable') return t('err.noRig');
    if (error.code === 'bad_text') return t('err.badText');
    if (error.code === 'tx_disabled') return t('err.txDisabled');
    if (error.status === 403) return t('err.forbidden');
    if (error.status === 404) return t('err.gone');
    if (error.status === 401) return t('err.unpaired');
  }
  return t('err.generic');
}
