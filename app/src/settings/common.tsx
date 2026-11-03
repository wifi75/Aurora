import { useQuery } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { getStationInfo, type StationInfo } from '../api/client';
import { useT } from '../i18n';
import { Help as Tip } from './layout';

/** Come e' avviata la stazione (sola lettura), condiviso dalle schede che la mostrano. */
export function InfoTab({ render }: { render(info: StationInfo): ReactNode }) {
  const t = useT();
  const info = useQuery({ queryKey: ['station-info'], queryFn: getStationInfo, retry: false });
  if (info.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!info.data) return <p className="muted">{t('set.loading')}</p>;
  return <>{render(info.data)}</>;
}

/** Una riga "nome  valore". */
export function Row({ name, children, hint }: { name: string; children: ReactNode; hint?: string }) {
  return (
    <div className="sx-field sx-field--plain" title={name}>
      <span className="sx-label muted">
        {name}
        <Tip tip={hint} />
      </span>
      <span className="sx-control data">{children}</span>
    </div>
  );
}

/** Casella con etichetta e suggerimento. */
export function Check({ label, hint, checked, onChange }: { label: string; hint?: string; checked: boolean; onChange(value: boolean): void }) {
  return (
    <label className="set-check sx-field" title={label}>
      <span className="sx-label">
        {label}
        <Tip tip={hint} />
      </span>
      <span className="sx-control">
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      </span>
    </label>
  );
}

/** Spiegazioni lunghe sotto le voci: per scelta del proprietario non si mostrano piu' (restano i «?»). */
export function Help(_props: { children: ReactNode }) {
  return null;
}
