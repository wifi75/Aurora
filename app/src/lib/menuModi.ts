import type { MenuDef, MenuItem } from '../components/digi/MenuBar';

/** I modi della pagina digitale. */
export type DigiMode = 'psk31' | 'psk63' | 'rtty' | 'cw' | 'ssb' | 'olivia' | 'contestia';

/** I modi a testo (ricezione e trasmissione di testo, cascata): tutti tranne la fonia. */
const TEXT: readonly DigiMode[] = ['psk31', 'psk63', 'rtty', 'cw', 'olivia', 'contestia'];

/** I modi che Aurora non ha ancora: nel menu Modo sono voci grigie «presto». */
export const MODI_IN_ARRIVO: readonly string[] = ['QPSK31', 'FSK31', 'MFSK', 'MT63', 'Throb', 'Hellschreiber', 'Packet', 'Pactor', 'Amtor'];

/**
 * In quali modi e' attiva ogni voce dei menu della pagina digitale (un solo posto). Una voce che non e' qui vale in tutti i modi.
 * Le voci proprie di un solo modo (baud e shift dell'RTTY, banda del filtro e WPM del CW, tutta la banda del PSK) sono gia' messe nel menu
 * solo in quel modo da `DigiPage`: qui restano le voci comuni ai modi a testo, che la fonia (SSB) non usa.
 */
export const DISPONIBILE_IN: Readonly<Record<string, readonly DigiMode[]>> = {
  // Modifica: finestre RX/TX
  'digi.mi.clearRx': TEXT,
  'digi.mi.clearTx': TEXT,
  'digi.mi.copyRx': TEXT,
  // Modo
  'digi.mi.modeSettings': TEXT,
  'digi.mi.threshold': TEXT,
  // Vista: cascata, pannello Segnali, testo RX/TX
  'digi.mi.waterfall': TEXT,
  'digi.band.view': TEXT,
  'digi.band.wfCalls': TEXT,
  'digi.band.wfWords': TEXT,
  'rbn.show': TEXT,
  'digi.mi.statusBar': TEXT,
  'digi.mi.echoRx': TEXT,
  'digi.mi.underlineSent': TEXT,
  'digi.mi.capsOnly': TEXT,
  'digi.mi.slashedZero': TEXT,
  'digi.mi.flashTx': TEXT,
  'digi.mi.hairlines': TEXT,
  'digi.mi.wideWaterfall': TEXT,
  'digi.mi.anticlick': TEXT,
  // Opzioni (tutte): ascolto, AFC, filtro, notch, Lock, Snap, squelch, RX/TX, cerca, scansione, Auto CQ, memorie
  'digi.mi.listen': TEXT,
  'digi.mi.afc': TEXT,
  'digi.mi.passband': TEXT,
  'digi.notch.clear': TEXT,
  'digi.mi.lock': TEXT,
  'digi.mi.snap': TEXT,
  'digi.mi.squelch': TEXT,
  'digi.mi.rx': TEXT,
  'digi.mi.tx': TEXT,
  'digi.mi.seekLeft': TEXT,
  'digi.mi.seekRight': TEXT,
  'digi.mi.scan': TEXT,
  'digi.mi.autoCq': TEXT,
  'digi.mi.memory': TEXT,
};

export const disponibileIn = (key: string, mode: DigiMode): boolean => (DISPONIBILE_IN[key] ?? null)?.includes(mode) ?? true;

/**
 * Le voci non disponibili nel modo diventano grigie e senza spunta (il valore salvato resta com'e'), con il suggerimento che dice perche'.
 * `t` traduce una chiave; `tip` e' il testo del suggerimento.
 */
export function applyAvailability(menus: MenuDef[], mode: DigiMode, t: (key: never) => string, tip: string): MenuDef[] {
  const tr = t as unknown as (key: string) => string;
  const byLabel = new Map<string, readonly DigiMode[]>();
  for (const [key, modes] of Object.entries(DISPONIBILE_IN)) byLabel.set(tr(key), modes);
  const options = tr('digi.menu.options');
  const walk = (items: MenuItem[], whole: boolean): MenuItem[] =>
    items.map((it): MenuItem => {
      if (it === 'sep') return it;
      const rule = byLabel.get(it.label);
      const off = (whole || (rule !== undefined && !rule.includes(mode))) && mode === 'ssb';
      if (off) return { ...it, checked: undefined, disabled: true, title: tip, onSelect: undefined, items: undefined };
      return it.items ? { ...it, items: walk(it.items, false) } : it;
    });
  return menus.map((m) => ({ ...m, items: walk(m.items, m.label === options) }));
}
