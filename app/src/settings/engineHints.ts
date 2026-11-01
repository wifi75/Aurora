import { STRINGS, type Key, type T } from '../i18n';

/** La spiegazione di una voce del motore (`jxh.<chiave>`), se c'e'. */
export function engineHint(key: string, t: T): string | undefined {
  const k = `jxh.${key}`;
  return k in STRINGS ? t(k as Key) : undefined;
}
