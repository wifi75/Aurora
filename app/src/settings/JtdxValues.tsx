import { useQuery } from '@tanstack/react-query';
import { getJtdxConfig } from '../api/client';
import { useT, type Key, type T } from '../i18n';
import { JTDX_GROUPS, type JtdxGroup, type JtdxKind } from '../lib/jtdxGroups';
import { Help, Row } from './common';

/** Il valore di una voce di JTDX come testo leggibile. */
export function formatJtdxValue(kind: JtdxKind, raw: string | undefined, t: T): string {
  if (raw === undefined || raw === '') return t('jx.empty');
  if (kind === 'bool') return raw === 'true' || raw === '1' ? t('jx.yes') : t('jx.no');
  if (kind.startsWith('enum:')) {
    const key = `jx.${kind.slice(5)}.${raw}` as Key;
    const text = t(key);
    return text === key ? raw : text;
  }
  return raw;
}

/** Le impostazioni di JTDX di un gruppo, lette dal suo file (sola consultazione). */
export function JtdxValues({ group, title }: { group: JtdxGroup; title?: string }) {
  const t = useT();
  const config = useQuery({ queryKey: ['jtdx-config'], queryFn: getJtdxConfig, retry: false, refetchInterval: 15_000 });
  return (
    <section aria-label={title ?? t('jx.title')}>
      <h3 className="set-sub">{title ?? t('jx.title')}</h3>
      {config.isError ? (
        <p className="muted">{t('jx.unavailable')}</p>
      ) : !config.data ? (
        <p className="muted">{t('set.loading')}</p>
      ) : (
        <>
          {JTDX_GROUPS[group].map((item) => (
            <Row key={item.key} name={t(`jx.${item.key}` as Key)} hint={t('tip.jx')}>
              {formatJtdxValue(item.kind, config.data[item.key], t)}
            </Row>
          ))}
          <Help>{t('jx.note')}</Help>
        </>
      )}
    </section>
  );
}
