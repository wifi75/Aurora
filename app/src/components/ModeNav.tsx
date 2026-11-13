import { Icon } from './Icon';
import { HealthLight } from './HealthLight';
import { useEffect, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { deleteSplitDx, getDigi, getFrequencies, postMode } from '../api/client';
import { toast } from '../lib/toast';
import type { TextMode } from '../lib/bandplan';
import { useT } from '../i18n';
import type { Route } from '../lib/route';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';
import { rememberDial, retuneForMode, retuneForText } from '../lib/modeRetune';
import { modeBase } from '../lib/period';

/** I modi in alto: ognuno ha le sue pagine; i gruppi FT8 e VHF dividono i modi del motore. Gli altri modi arrivano uno alla volta. */
export type ModeGroup = 'ft8' | 'vhf';

/** Il gruppo del modo in uso, come lo riporta il motore (MSK144, Q65, JT65 e JT4 sono VHF; FT8, FT4 e JT9 sono HF). */
export function groupOf(mode: string): ModeGroup {
  const up = mode.toUpperCase();
  if (up === 'MSK144' || up.startsWith('Q65') || up.startsWith('JT4') || up.startsWith('JT65') || up === 'JT') return 'vhf';
  return 'ft8';
}

/** JT65 sta in tutti e due i gruppi (HF nel gruppo FT8, EME nel gruppo VHF): da solo il modo non dice in quale gruppo si e'. */
export function isSharedMode(mode: string): boolean {
  const up = mode.toUpperCase();
  return up.startsWith('JT65') || up === 'JT';
}

const GROUP_KEY = 'aurora.modenav.gruppo';
const GROUP_EVENT = 'aurora-modenav-gruppo';

/** L'ultimo gruppo scelto (pulsanti in alto o un modo che sta in un gruppo solo). */
function readPickedGroup(): ModeGroup | null {
  try {
    const raw = browserStorage()?.getItem(GROUP_KEY);
    return raw === 'ft8' || raw === 'vhf' ? raw : null;
  } catch {
    return null;
  }
}

function rememberGroup(group: ModeGroup): void {
  if (readPickedGroup() === group) return;
  try {
    browserStorage()?.setItem(GROUP_KEY, group);
  } catch {
    // senza archivio JT65 torna al suo gruppo di base (VHF)
  }
  try {
    window.dispatchEvent(new Event(GROUP_EVENT));
  } catch {
    // fuori dal browser (prove) non c'e' nessuno da avvisare
  }
}

/** Il gruppo del modo in uso tenendo conto della scelta: JT65 scelto dal gruppo FT8 resta nel gruppo FT8. */
export function groupFor(mode: string, picked: ModeGroup | null): ModeGroup {
  return isSharedMode(mode) && picked ? picked : groupOf(mode);
}

/** Il gruppo in uso, aggiornato quando lo si cambia (anche da un'altra finestra); un modo di un gruppo solo lo fissa. */
export function useModeGroup(mode: string): ModeGroup {
  const [picked, setPicked] = useState(readPickedGroup);
  useEffect(() => {
    const sync = () => setPicked(readPickedGroup());
    window.addEventListener(GROUP_EVENT, sync);
    window.addEventListener('storage', sync);
    return () => {
      window.removeEventListener(GROUP_EVENT, sync);
      window.removeEventListener('storage', sync);
    };
  }, []);
  useEffect(() => {
    if (mode && !isSharedMode(mode)) rememberGroup(groupOf(mode));
  }, [mode]);
  return groupFor(mode || 'FT8', picked);
}

const DEFAULT_MODE: Record<ModeGroup, string> = { ft8: 'FT8', vhf: 'MSK144' };
const KEY = 'aurora.modenav.ultimo';

/** L'ultimo modo usato in ogni gruppo: tornando al gruppo si riprende da quello. */
function lastMode(group: ModeGroup): string {
  try {
    const raw = browserStorage()?.getItem(`${KEY}.${group}`);
    if (raw && (groupOf(raw) === group || isSharedMode(raw))) return raw;
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
  const group = useModeGroup(mode);
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLSpanElement>(null);
  const client = useQueryClient();
  // si ricorda l'ultima frequenza di ogni gruppo (solo se e' una frequenza del modo in uso)
  const dialHz = useStation((s) => s.status?.dial_hz ?? 0);
  const freqTable = useQuery({ queryKey: ['frequencies'], queryFn: getFrequencies, staleTime: 60_000, retry: false });
  useEffect(() => {
    if (!mode || dialHz <= 0) return;
    // FST4 e FST4W: lo stato porta il periodo («FST4-60»), la tabella il nome nudo
    const base = modeBase(mode).toUpperCase();
    const known = (freqTable.data ?? []).some((r) => r.mode.toUpperCase() === base && r.hz === dialHz);
    if (!known) return;
    rememberDial(group, dialHz);
  }, [mode, group, dialHz, freqTable.data]);

  // si ricorda l'ultimo modo di ogni gruppo (JT65 in quello in cui lo si e' scelto)
  useEffect(() => {
    if (!mode) return;
    try {
      browserStorage()?.setItem(`${KEY}.${group}`, mode);
    } catch {
      // niente archivio: si riparte dal modo di base
    }
  }, [mode, group]);

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
  const onModePage = MODE_ROUTES.includes(route.name);
  /** La radio resta dov'era: se il modo non ha frequenze sulla banda di adesso va alla prima banda dove esiste; un errore si dice, non toglie il modo. */
  const retune = (run: () => Promise<number | null>, name: string) => {
    void run().catch(() => toast(t('mode.retuneFail', { mode: name }), 'error'));
  };
  const pickGroup = (target: ModeGroup) => {
    if (transmitting) return;
    rememberGroup(target);
    if (target === group) {
      // si torna da PSK, RTTY, SSTV o Satelliti allo stesso gruppo di prima: il modo c'e' gia', ma la radio e' rimasta sulla frequenza dell'altro modo
      // lo Split DX di uno spot («UP 5») acceso nella pagina PSK·RTTY·CW non vale nei modi del motore: si spegne anche se la radio e' gia'
      // su una frequenza del modo (nessun cambio di frequenza che lo spenga) o se il CAT in quel momento non risponde
      if (!onModePage && mode) {
        retune(async () => {
          await deleteSplitDx().catch(() => undefined);
          return retuneForMode(client, mode);
        }, mode);
      }
      return;
    }
    const next = lastMode(target);
    const store = useStation.getState();
    const before = store.status?.mode;
    store.patchStatus({ mode: next });
    void postMode(next).then(
      () => retune(() => retuneForMode(client, next), next),
      () => {
        if (before !== undefined) useStation.getState().revertStatus(['mode'], { mode: before });
      },
    );
  };
  const TEXT_MODES: Record<string, TextMode> = { psk31: 'PSK31', psk63: 'PSK63', rtty: 'RTTY', cw: 'CW' };
  /** Entrando in PSK, RTTY, CW o SSTV da un'altra pagina la radio va sulla banda del loro piano IARU (se quella di adesso non lo ha, alla prima). */
  const enterText = () => {
    if (transmitting || route.name === 'digi' || route.name === 'sstv') return;
    void (async () => {
      const digi = await client.fetchQuery({ queryKey: ['digi'], queryFn: getDigi, staleTime: 5_000 });
      await retuneForText(client, TEXT_MODES[digi.settings.mode] ?? 'PSK31');
    })().catch(() => toast(t('mode.retuneFail', { mode: 'PSK' }), 'error'));
  };
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

  // la voce «Impostazioni» del menu apre sempre la pagina Informazioni (richiesta del proprietario); i rami dei modi si aprono
  // dagli ingranaggi delle singole pagine
  const settingsHref = '#/impostazioni';

  return (
    <>
      <span className="nav-modes">
        {pill('ft8', 'FT8')}
        {pill('vhf', 'VHF')}
        {/* PSK, RTTY, CW e SSTV stanno insieme: il modo si sceglie dal menu «Modo» della loro pagina */}
        <a {...current(route.name === 'digi' || route.name === 'sstv')} data-mode="psk" href={route.name === 'sstv' ? '#/sstv' : '#/testo'} title={t('tip.nav.digi')} onClick={enterText}>
          {t('nav.textModes')}
        </a>
        <a {...current(route.name === 'sat')} data-mode="sat" href="#/satelliti" title={t('tip.nav.sat')}>
          {t('nav.sat')}
        </a>
      </span>
      <span className="nav-pages">
        {/* le pagine del modo scelto */}
        {route.name === 'sstv' || route.name === 'digi' || route.name === 'sat' ? null : (
          <>
            {group === 'ft8' ? page('fox', '#/fox', t('nav.fox'), t('tip.nav.fox')) : null}
            {page('contest', '#/contest', t('nav.contest'), t('tip.nav.contest'))}
            {group === 'vhf' && (showQmap || route.name === 'qmap') ? page('qmap', '#/qmap', t('nav.qmap'), t('tip.nav.qmap')) : null}
            {page('stats', '#/statistiche', t('nav.stats'))}
          </>
        )}
        {/* le pagine di tutti i modi, divise da quelle del modo come nel prototipo */}
        {route.name === 'sstv' || route.name === 'digi' || route.name === 'sat' ? null : <span className="nav-sep" aria-hidden="true" />}
        {common('export', '#/esporta', t('nav.log'))}
        {common('dxmap', '#/mappa-dx', t('nav.dxmap'))}
        {common('propagation', '#/propagazione', t('nav.propagation'))}
        {common('rotator', '#/rotore', t('nav.rotator'), t('tip.nav.rotator'))}
        {/* spia di salute della stazione, subito prima dell'ingranaggio */}
        <HealthLight />
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
            <Icon name="gear" size="1.15em" />
            <Icon name="chevron-down" size="0.8em" />
          </button>
          {open ? (
            <span className="nav-more-menu" role="menu">
              {remote ? null : (
                // le Impostazioni si aprono sul nodo della pagina e del modo in uso (Modi › FT8 · FT4, PSK · RTTY · CW, Satelliti...)
                <a role="menuitem" href={settingsHref} onClick={() => setOpen(false)}>
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
