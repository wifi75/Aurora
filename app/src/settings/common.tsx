import { useQuery } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { getStationInfo, type StationInfo } from '../api/client';
import { useT } from '../i18n';

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
    <div className="info-row" title={hint}>
      <span className="muted">{name}</span>
      <span className="data">{children}</span>
    </div>
  );
}

/** Casella con etichetta e suggerimento. */
export function Check({ label, hint, checked, onChange }: { label: string; hint?: string; checked: boolean; onChange(value: boolean): void }) {
  return (
    <label className="set-check" title={hint}>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  );
}

export function Help({ children }: { children: ReactNode }) {
  return <p className="muted settings-hint">{children}</p>;
}
