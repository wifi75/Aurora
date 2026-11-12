import type { Key } from '../i18n';

/**
 * L'albero delle Impostazioni: famiglie colorate, poi sottofamiglie (solo in «Modi»), poi pagine. Qui c'e' solo la struttura (id, nomi,
 * colori); cosa disegna ogni pagina lo dice `pages/SettingsWindow.tsx`. Gli id delle pagine sono minuscole e cifre, come gli indirizzi
 * `#/impostazioni/<id>`; i vecchi id delle schede piatte portano alla pagina nuova con `LEGACY_IDS`.
 */

export type FamilyId = 'station' | 'modes' | 'log' | 'look' | 'access';

export interface TreePage {
  kind: 'page';
  id: string;
  label: Key;
}

export interface TreeSub {
  kind: 'sub';
  id: string;
  label: Key;
  pages: readonly TreePage[];
}

export interface TreeFamily {
  id: FamilyId;
  label: Key;
  /** Il colore della barretta accanto al nome. */
  color: string;
  items: readonly (TreePage | TreeSub)[];
}

/** La pagina Informazioni, staccata in fondo all'albero: e' anche quella che si vede aprendo senza una scheda richiesta. */
export const INFO_ID = 'info';

const page = (id: string, label: Key): TreePage => ({ kind: 'page', id, label });
const sub = (id: string, label: Key, pages: readonly TreePage[]): TreeSub => ({ kind: 'sub', id, label, pages });

export const SETTINGS_TREE: readonly TreeFamily[] = [
  {
    id: 'station',
    label: 'sw.sec.station',
    color: '#2dd4bf',
    items: [
      page('personal', 'sw.pg.personal'),
      page('radio', 'sw.pg.radio'),
      page('profiles', 'sw.pg.profiles'),
      page('audio', 'sw.tab.audio'),
      page('rotator', 'sw.tab.rotator'),
      page('sdr', 'sw.pg.sdr'),
      page('clock', 'tab.clock'),
    ],
  },
  {
    id: 'modes',
    label: 'sw.fam.modes',
    color: '#8e7bff',
    items: [
      sub('modecommon', 'sw.sub.common', [page('freq', 'sw.tab.freq'), page('qso', 'sw.pg.qso'), page('autocq', 'sw.tab.autocq')]),
      sub('modeft8', 'sw.sub.ft8', [
        page('ft8decode', 'fil.decode'),
        page('sequence', 'tab.sequence'),
        page('macros', 'sw.tab.macros'),
        page('automation', 'nav.automation'),
        page('messages', 'sw.pg.messages'),
        page('contest', 'sw.sub.contest'),
      ]),
      page('jt', 'sw.sub.jt'),
      sub('modevhf', 'sw.sub.vhf', [page('vhf', 'sw.pg.vhfModes'), page('contestvhf', 'sw.pg.contest')]),
      page('wspr', 'sw.sub.wspr'),
      sub('modedigi', 'sw.sub.digi', [page('digi', 'sw.pg.digiRxTx'), page('digimacros', 'sw.pg.digiMacros'), page('digiscan', 'sw.pg.digiScan'), page('digicluster', 'cop.title')]),
      sub('modesstv', 'sw.sec.sstv', [page('sstv', 'sw.pg.rx'), page('sstvtx', 'sw.pg.tx'), page('sstvimages', 'sw.pg.images')]),
      sub('modesat', 'nav.sat', [page('satorbits', 'sw.pg.orbits'), page('satmap', 'sat.set.catMap'), page('satradio', 'sw.pg.doppler')]),
    ],
  },
  {
    id: 'log',
    label: 'sw.fam.log',
    color: '#f5a524',
    items: [
      page('logbooks', 'sw.sec.log'),
      page('files', 'sw.pg.files'),
      page('reporting', 'sw.pg.reporting'),
      page('cluster', 'tab.cluster'),
      page('online', 'tab.online'),
      page('wishlist', 'tab.wishlist'),
      page('dxcc', 'tab.dxcc'),
    ],
  },
  {
    id: 'look',
    label: 'sw.sec.look',
    color: '#4cc3ff',
    items: [
      page('aspetto', 'sw.tab.appearance'),
      page('colors', 'sw.tab.colors'),
      page('alerts', 'sw.pg.alerts'),
      page('waterfall', 'tab.waterfall'),
      page('windows', 'tab.windows'),
      page('bands', 'sw.pg.bands'),
    ],
  },
  {
    id: 'access',
    label: 'sw.sec.access',
    color: '#f472b6',
    items: [page('phone', 'sw.tab.phone'), page('agent', 'agent.tab'), page('updates', 'tab.updates'), page('support', 'tab.support')],
  },
];

/** Le schede di prima (e i collegamenti `#/impostazioni/<id>` sparsi nel programma) che oggi hanno un altro nome. Le altre hanno tenuto il loro id. */
export const LEGACY_IDS: Readonly<Record<string, string>> = {
  general: 'personal',
  filters: 'messages',
  cqnews: 'messages',
};

/** Gli id delle 34 schede piatte di prima: devono tutti portare a una pagina (lo controlla una prova). */
export const OLD_TAB_IDS = [
  'general', 'radio', 'profiles', 'rotator', 'sdr', 'audio', 'freq', 'bands', 'macros', 'sequence', 'autocq', 'automation', 'filters', 'cqnews', 'wishlist', 'dxcc',
  'aspetto', 'colors', 'alerts', 'waterfall', 'windows', 'clock', 'logbooks', 'files', 'reporting', 'online', 'cluster', 'sstv', 'sstvtx', 'sstvimages', 'phone',
  'agent', 'support', 'info',
] as const;

/** Dove sta un nodo: la famiglia e la sottofamiglia da aprire e la pagina da mostrare. */
export interface NodePath {
  family: TreeFamily | null;
  sub: TreeSub | null;
  page: string;
}

const INFO_PATH: NodePath = { family: null, sub: null, page: INFO_ID };

/** Tutte le pagine dell'albero, ognuna con la sua famiglia e la sua sottofamiglia. */
export function allPages(): Array<{ page: TreePage; family: TreeFamily; sub: TreeSub | null }> {
  const out: Array<{ page: TreePage; family: TreeFamily; sub: TreeSub | null }> = [];
  for (const family of SETTINGS_TREE) {
    for (const item of family.items) {
      if (item.kind === 'page') out.push({ page: item, family, sub: null });
      else for (const p of item.pages) out.push({ page: p, family, sub: item });
    }
  }
  return out;
}

/**
 * Il percorso di un id: una pagina (anche col nome vecchio), una sottofamiglia o una famiglia (si apre la loro prima pagina).
 * Senza id, o con un id sconosciuto, la pagina Informazioni con tutto chiuso.
 */
export function resolveNode(id: string | null | undefined): NodePath {
  if (!id) return INFO_PATH;
  const want = LEGACY_IDS[id] ?? id;
  if (want === INFO_ID) return INFO_PATH;
  for (const family of SETTINGS_TREE) {
    if (family.id === want) {
      const first = family.items[0];
      if (!first) return INFO_PATH;
      return first.kind === 'page' ? { family, sub: null, page: first.id } : { family, sub: first, page: first.pages[0]?.id ?? INFO_ID };
    }
    for (const item of family.items) {
      if (item.kind === 'page') {
        if (item.id === want) return { family, sub: null, page: item.id };
      } else {
        if (item.id === want) return { family, sub: item, page: item.pages[0]?.id ?? INFO_ID };
        const hit = item.pages.find((p) => p.id === want);
        if (hit) return { family, sub: item, page: hit.id };
      }
    }
  }
  return INFO_PATH;
}

/** Vero se l'id porta da qualche parte (pagina, sottofamiglia, famiglia o nome vecchio). */
export function isKnownNode(id: string | null | undefined): boolean {
  if (!id) return false;
  const want = LEGACY_IDS[id] ?? id;
  return want === INFO_ID || resolveNode(id).page !== INFO_ID;
}

/**
 * Il nodo delle Impostazioni di un modo, per l'ingranaggio del menu in alto: la pagina in cui si e' e il modo in uso.
 * `undefined` per le pagine comuni (Log, Mappa DX...), che aprono le Impostazioni sulla pagina Informazioni.
 */
export function settingsNodeFor(route: string, mode: string): string | undefined {
  switch (route) {
    case 'digi':
      return 'modedigi';
    case 'sstv':
      return 'modesstv';
    case 'sat':
      return 'modesat';
    // il contest nel ramo del suo modo: VHF · EME per MSK144 e Q65, altrimenti FT8 · FT4 (con il Fox/Hound)
    case 'contest':
      return modeNode(mode) === 'vhf' ? 'contestvhf' : 'contest';
    case 'fox':
      return 'contest';
    case 'rotator':
      return 'rotator';
    case 'live':
    case 'qmap':
    case 'stats':
    case 'settings':
      return modeNode(mode);
    default:
      return undefined;
  }
}

/** Il nodo di un modo del motore: FT8 e FT4, JT65, JT9 e FST4, i modi VHF ed EME, WSPR e FST4W. */
export function modeNode(mode: string): string {
  const up = mode.trim().toUpperCase();
  if (up.startsWith('WSPR') || up.startsWith('FST4W')) return 'wspr';
  if (up.startsWith('JT65') || up.startsWith('JT9') || up.startsWith('FST4') || up === 'JT') return 'jt';
  if (up.startsWith('Q65') || up.startsWith('MSK') || up.startsWith('JT4')) return 'vhf';
  return 'modeft8';
}
