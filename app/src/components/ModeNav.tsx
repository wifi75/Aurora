import { useEffect, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getDigi, postMode, putDigi, type DigiSettings } from '../api/client';
import { useT } from '../i18n';
import type { Route } from '../lib/route';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';

/** I modi in alto: ognuno ha le sue pagine; i gruppi FT8 e VHF dividono i modi del motore. Gli altri modi arrivano uno alla volta. */
export type ModeGroup = 'ft8' | 'vhf';

/** Il gruppo del modo in uso, come lo riporta il motore (MSK144, Q65, JT65 e JT4 sono VHF; FT8, FT4 e JT9 sono HF). */
export function groupOf(mode: string): ModeGroup {
  const up = mode.toUpperCase();
  if (up === 'MSK144' || up.startsWith('Q65') || up.startsWith('JT4') || up.startsWith('JT65') || up === 'JT') return 'vhf';
  return 'ft8';
}

const DEFAULT_MODE: Record<ModeGroup, string> = { ft8: 'FT8', vhf: 'MSK144' };
const KEY = 'aurora.modenav.ultimo';

/** L'ultimo modo usato in ogni gruppo: tornando al gruppo si riprende da quello. */
function lastMode(group: ModeGroup): string {
  try {
    const raw = browserStorage()?.getItem(`${KEY}.${group}`);
    if (raw && groupOf(raw) === group) return raw;
  } catch {
    // senza archivio si parte dal modo di base
  }
  return DEFAULT_MODE[group];
}

const MODE_ROUTES: readonly Route['name'][] = ['live', 'fox', 'contest', 'qmap', 'stats'];
const MORE_ROUTES: readonly Route['name'][] = ['settings', 'guide', 'news'];

interface Props {
  route: Route;
  showQmap: boolean;
  /** Vista dal browser remoto: senza Impostazioni. */
  remote: boolean;
}

const current = (on: boolean) => ({ className: `glass${on ? ' glass--on' : ''}`, 'aria-current': on ? ('page' as const) : undefined });

/** Il menu nuovo: i modi come pillole, a destra le pagine del modo scelto, le pagine comuni e il menu delle impostazioni (Impostazioni, Guida, Novita). */
export function ModeNav({ route, showQmap, remote }: Props) {
  const t = useT();
  const mode = useStation((s) => s.status?.mode ?? '');
  const transmitting = useStation((s) => s.status?.transmitting ?? false);
  const group = groupOf(mode);
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement>(null);
  const client = useQueryClient();
  // il modo testuale scelto (PSK31, PSK63 o RTTY), per accendere la pillola giusta
  const digi = useQuery({ queryKey: ['digi'], queryFn: getDigi, retry: false, enabled: route.name === 'digi', staleTime: 5000 });
  const digiMode = digi.data?.settings.mode ?? null;

  // si ricorda l'ultimo modo di ogni gruppo
  useEffect(() => {
    if (!mode) return;
    try {
      browserStorage()?.setItem(`${KEY}.${groupOf(mode)}`, mode);
    } catch {
      // niente archivio: si riparte dal modo di base
    }
  }, [mode]);

  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) setOpen(false);
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', esc);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', esc);
    };
  }, [open]);

  // il pulsante del gruppo accende subito il modo; se il comando fallisce torna com'era
  const pickGroup = (target: ModeGroup) => {
    if (target === group || transmitting) return;
    const next = lastMode(target);
    const store = useStation.getState();
    const before = store.status?.mode;
    store.patchStatus({ mode: next });
    void postMode(next).catch(() => {
      if (before !== undefined) useStation.getState().revertStatus(['mode'], { mode: before });
    });
  };

  const onModePage = MODE_ROUTES.includes(route.name);
  const pill = (target: ModeGroup, label: string) => (
    <a
      key={target}
      {...current(onModePage && group === target)}
      data-mode={target}
      href="#/"
      title={transmitting && group !== target ? t('mode.locked') : undefined}
      onClick={() => pickGroup(target)}
    >
      {label}
    </a>
  );
  // PSK e RTTY stanno nella stessa pagina: la pillola sceglie il modo e apre la pagina
  const pickText = (family: 'psk' | 'rtty') => {
    void getDigi()
      .then((snap) => {
        const current = snap.settings.mode;
        if ((family === 'rtty') === (current === 'rtty')) return undefined;
        const mode: DigiSettings['mode'] = family === 'rtty' ? 'rtty' : 'psk31';
        return putDigi({ ...snap.settings, mode }).then((next) => client.setQueryData(['digi'], next));
      })
      .catch(() => undefined);
  };
  const textPill = (family: 'psk' | 'rtty', label: string) => (
    <a
      key={family}
      {...current(route.name === 'digi' && (family === 'rtty' ? digiMode === 'rtty' : digiMode !== null && digiMode !== 'rtty'))}
      data-mode={family}
      href="#/testo"
      title={t('tip.nav.digi')}
      onClick={() => pickText(family)}
    >
      {label}
    </a>
  );
  const soon = (id: string, label: string) => (
    <span key={id} className="nav-soon" data-mode={id} title={t('nav.modeSoon')}>
      {label}
    </span>
  );
  const page = (name: Route['name'], href: string, label: string, tip?: string) => (
    <a key={name} {...current(route.name === name)} data-pagina="modo" href={href} title={tip}>
      {label}
    </a>
  );
  const common = (name: Route['name'], href: string, label: string, tip?: string) => (
    <a key={name} {...current(route.name === name)} data-pagina="comune" href={href} title={tip}>
      {label}
    </a>
  );

  return (
    <>
      <span className="nav-modes">
        {pill('ft8', 'FT8')}
        {pill('vhf', 'VHF')}
        {soon('cw', 'CW')}
        {textPill('rtty', 'RTTY')}
        {textPill('psk', 'PSK')}
        <a {...current(route.name === 'sstv')} data-mode="sstv" href="#/sstv" title={t('tip.nav.sstv')}>
          {t('nav.sstv')}
        </a>
        {soon('sat', t('nav.sat'))}
      </span>
      <span className="nav-pages">
        {/* le pagine del modo scelto */}
        {route.name === 'sstv' || route.name === 'digi' ? null : (
          <>
            {group === 'ft8' ? page('fox', '#/fox', t('nav.fox'), t('tip.nav.fox')) : null}
            {page('contest', '#/contest', t('nav.contest'), t('tip.nav.contest'))}
            {group === 'vhf' && (showQmap || route.name === 'qmap') ? page('qmap', '#/qmap', t('nav.qmap'), t('tip.nav.qmap')) : null}
            {page('stats', '#/statistiche', t('nav.stats'))}
          </>
        )}
        {/* le pagine di tutti i modi, divise da quelle del modo come nel prototipo */}
        {route.name === 'sstv' || route.name === 'digi' ? null : <span className="nav-sep" aria-hidden="true" />}
        {common('export', '#/esporta', t('nav.log'))}
        {common('dxmap', '#/mappa-dx', t('nav.dxmap'))}
        {common('propagation', '#/propagazione', t('nav.propagation'))}
        {common('rotator', '#/rotore', t('nav.rotator'), t('tip.nav.rotator'))}
        <span className="nav-more" ref={box}>
          <button
            type="button"
            className={`glass nav-more-btn${MORE_ROUTES.includes(route.name) ? ' glass--on' : ''}`}
            aria-haspopup="menu"
            aria-expanded={open}
            aria-label={t('nav.more')}
            title={t('nav.more')}
            onClick={() => setOpen((o) => !o)}
          >
            ⚙ <span aria-hidden="true">▾</span>
          </button>
          {open ? (
            <span className="nav-more-menu" role="menu">
              {remote ? null : (
                <a role="menuitem" href="#/impostazioni" onClick={() => setOpen(false)}>
                  {t('nav.settings')}
                </a>
              )}
              <a role="menuitem" href="#/guida" onClick={() => setOpen(false)}>
                {t('nav.wiki')}
              </a>
              <a role="menuitem" href="#/novita" onClick={() => setOpen(false)}>
                {t('nav.news')}
              </a>
            </span>
          ) : null}
        </span>
      </span>
    </>
  );
}
