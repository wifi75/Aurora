import { useT } from '../i18n';
import { attach, attachAll, DASHBOARD, detach, openGroupWindow, useGroups, type ItemId } from '../lib/detached';
import { ALL_PANELS, readLayout, saveLayout, toggleHidden, type PanelId } from '../lib/layout';
import { browserStorage } from '../lib/theme';
import { ITEM_TITLE } from '../components/SoloWindow';
import { useState } from 'react';
import { Actions, Group } from './layout';

const ITEMS: readonly ItemId[] = [...ALL_PANELS, 'dxmap', 'propagation', 'display'];

/**
 * Dove sta ogni elemento (Impostazioni › Finestre): nella finestra principale, in una finestra a parte (nuova o accanto a un'altra) o
 * nascosto. Da qui si ritrova sempre tutto, anche se una finestra a parte e' stata chiusa o e' su uno schermo che non c'e' piu'.
 */
export function WhereGroups() {
  const t = useT();
  const groups = useGroups();
  const [layout, setLayout] = useState(() => readLayout(browserStorage()));
  const hidden = layout.hidden ?? [];
  const open = groups.filter((g) => g.open);
  const placeOf = (item: ItemId): string => {
    const g = open.find((x) => x.items.includes(item));
    if (g) return `in:${g.id}`;
    if (hidden.includes(item as PanelId)) return 'hidden';
    return 'main';
  };
  const setHidden = (item: ItemId, want: boolean) => {
    if (item === 'map' || item === 'dxmap' || item === 'bands' || item === 'propagation' || item === 'display') return;
    if (hidden.includes(item) !== want) {
      const next = toggleHidden(layout, item);
      saveLayout(browserStorage(), next);
      setLayout(next);
    }
  };
  const choose = (item: ItemId, value: string) => {
    if (value === 'main') {
      attach(item);
      setHidden(item, false);
    } else if (value === 'hidden') {
      attach(item);
      setHidden(item, true);
    } else if (value === 'new') {
      setHidden(item, false);
      detach(item);
    } else if (value.startsWith('in:')) {
      setHidden(item, false);
      detach(item, value.slice(3));
    }
  };
  const label = (items: readonly ItemId[]) => items.map((i) => t(ITEM_TITLE[i])).join(' · ');
  return (
    <>
      <Group title={t('win.where')}>
        <div className="where-list">
          {ITEMS.map((item) => {
            const place = placeOf(item);
            const page = item === 'map' || item === 'dxmap' || item === 'propagation' || item === 'display';
            return (
              <label key={item} className="where-row" data-place={place === 'main' ? 'main' : place === 'hidden' ? 'hidden' : 'away'}>
                <span>{t(ITEM_TITLE[item])}</span>
                <select value={place} onChange={(e) => choose(item, e.target.value)}>
                  <option value="main">{t('win.place.main')}</option>
                  {place.startsWith('in:') ? null : <option value="new">{t('win.new')}</option>}
                  {open.some((g) => g.id === DASHBOARD) ? null : <option value={`in:${DASHBOARD}`}>{t('win.dash')}</option>}
                  {open.map((g) => (
                    <option key={g.id} value={`in:${g.id}`}>
                      {g.id === DASHBOARD ? t('win.dash') : g.items.includes(item) ? t('win.place.in', { what: label(g.items) }) : t('win.beside', { what: label(g.items) })}
                    </option>
                  ))}
                  {page ? null : <option value="hidden">{t('win.place.hidden')}</option>}
                </select>
              </label>
            );
          })}
        </div>
      </Group>
      <Group title={t('win.open')}>
        {open.length === 0 ? (
          <p className="muted">{t('win.none')}</p>
        ) : (
          <ul className="where-groups">
            {open.map((g) => (
              <li key={g.id}>
                <span>{g.id === DASHBOARD ? `${t('win.dashName')}: ${label(g.items)}` : label(g.items)}</span>
                <button type="button" className="glass chip" onClick={() => openGroupWindow(g.id)}>
                  {t('win.show')}
                </button>
                <button type="button" className="glass chip" onClick={() => g.items.forEach((i) => attach(i))}>
                  {t('win.attach')}
                </button>
              </li>
            ))}
          </ul>
        )}
        <Actions>
          <button type="button" className="glass" disabled={groups.length === 0} onClick={attachAll}>
            {t('win.attachAll')}
          </button>
        </Actions>
      </Group>
    </>
  );
}

