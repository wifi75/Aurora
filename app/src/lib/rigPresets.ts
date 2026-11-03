import type { CatConfig } from '../api/client';

/**
 * Impostazioni consigliate per le radio piu' comuni: cosa mettere in Aurora, cosa nel menu della radio e quale scheda audio scegliere.
 * Le voci di menu restano in inglese, come le scrive la radio. Si mostrano solo le voci certe; dove il nome esatto cambia
 * da modello a modello lo dice il commento e la scheda mostra l'avviso «controlla il manuale» (`generic`).
 */
export interface RigPreset {
  id: string;
  /** Riconosce il modello dal nome Hamlib («Icom IC-7300»), senza trattini e maiuscole. */
  match: RegExp;
  /** I valori da scrivere nella configurazione (solo questi: il resto, porta compresa, non si tocca). */
  aurora: Partial<Pick<CatConfig, 'baud' | 'ptt' | 'civ_address' | 'force_dtr' | 'force_rts'>>;
  /** Indirizzo CI-V di fabbrica in decimale (solo Icom): con 0 Hamlib usa questo. */
  civFactory?: number;
  /** Le voci del menu della radio, brevi. */
  radio: readonly string[];
  /** La scheda audio: quella USB interna della radio o un'interfaccia esterna. */
  audio: 'codec' | 'external';
  /** Nomi delle voci di menu non verificati su questo modello: si mostra l'invito a controllare il manuale. */
  generic?: boolean;
}

// Icom con USB: fonte docs/cat-radio.md (sezione «Icom IC-7300 e IC-7610») e manuali Icom, menu SET > Connectors.
// Indirizzi CI-V di fabbrica: 7300 94h, 7610 98h, 705 A4h, 9700 A2h.
const ICOM_MENU = [
  'SET > Connectors > CI-V USB Port: Unlink from [REMOTE]',
  'CI-V USB Baud Rate: 115200',
  'CI-V USB Echo Back: OFF',
  'CI-V Transceive: ON',
  'MOD Input > DATA MOD: USB',
  'USB MOD Level: ~50%',
  'Mode: USB-D (DATA)',
] as const;

// IC-705: la porta USB e' virtuale e la voce «CI-V USB Baud Rate» non e' verificata su questo modello: si omette.
const IC705_MENU = ICOM_MENU.filter((v) => !v.startsWith('CI-V USB Baud'));

// Yaesu recenti (FT-991A, FT-710, FTDX10, FTDX101): 38400 e CAT RTS OFF sono certi; il nome esatto delle voci (CAT RATE / CAT-1 RATE,
// sorgente audio dati e PTT dei dati) cambia tra i modelli e non e' verificato qui: indicazioni generiche e `generic`.
const YAESU_MENU = ['CAT RATE: 38400', 'CAT RTS: OFF', 'DATA (audio): USB', 'Mode: DATA-U'] as const;

// Kenwood TS-590SG e TS-890: 115200 sulla USB; DATA VOX spento, audio dati dalla USB. Nomi esatti dei menu da verificare (`generic`).
const KENWOOD_MENU = ['USB COM speed: 115200', 'DATA VOX: OFF', 'Data audio input: USB', 'Mode: USB-DATA'] as const;

export const RIG_PRESETS: readonly RigPreset[] = [
  { id: 'ic7300', match: /ic7300/, aurora: { baud: 115200, ptt: 'cat', civ_address: 0 }, civFactory: 148, radio: ICOM_MENU, audio: 'codec' },
  { id: 'ic7610', match: /ic7610/, aurora: { baud: 115200, ptt: 'cat', civ_address: 0 }, civFactory: 152, radio: ICOM_MENU, audio: 'codec' },
  { id: 'ic705', match: /ic705(?!\d)/, aurora: { baud: 115200, ptt: 'cat', civ_address: 0 }, civFactory: 164, radio: IC705_MENU, audio: 'codec' },
  { id: 'ic9700', match: /ic9700/, aurora: { baud: 115200, ptt: 'cat', civ_address: 0 }, civFactory: 162, radio: ICOM_MENU, audio: 'codec' },
  { id: 'ft991', match: /ft991/, aurora: { baud: 38400, ptt: 'cat' }, radio: YAESU_MENU, audio: 'codec', generic: true },
  { id: 'ft710', match: /ft710/, aurora: { baud: 38400, ptt: 'cat' }, radio: YAESU_MENU, audio: 'codec', generic: true },
  { id: 'ftdx101', match: /ftdx101/, aurora: { baud: 38400, ptt: 'cat' }, radio: YAESU_MENU, audio: 'codec', generic: true },
  { id: 'ftdx10', match: /ftdx10(?!\d)/, aurora: { baud: 38400, ptt: 'cat' }, radio: YAESU_MENU, audio: 'codec', generic: true },
  { id: 'ts590', match: /ts590/, aurora: { baud: 115200, ptt: 'cat' }, radio: KENWOOD_MENU, audio: 'codec', generic: true },
  { id: 'ts890', match: /ts890/, aurora: { baud: 115200, ptt: 'cat' }, radio: KENWOOD_MENU, audio: 'codec', generic: true },
  // FT-2000: la radio del proprietario, gia' provata dal vivo con 38400 e PTT su RTS della COM; nessuna USB audio interna.
  { id: 'ft2000', match: /ft2000/, aurora: { baud: 38400, ptt: 'rts' }, radio: ['CAT RATE: 38400'], audio: 'external' },
];

/** La radio nota che corrisponde al nome scelto, se c'e'. */
export function presetFor(rigName: string): RigPreset | undefined {
  const name = rigName.toLowerCase().replace(/[^a-z0-9]/g, '');
  if (name === '') return undefined;
  return RIG_PRESETS.find((p) => p.match.test(name));
}

/** La configurazione ha gia' tutti i valori consigliati. */
export function presetApplied(c: CatConfig, p: RigPreset): boolean {
  return (Object.keys(p.aurora) as (keyof RigPreset['aurora'])[]).every((k) => c[k] === p.aurora[k]);
}

/** Radio Icom con PTT dal CAT ma velocita' diversa da 115200: la causa piu' comune della trasmissione che non parte. */
export function icomBaudWarning(c: CatConfig): boolean {
  return c.backend === 'hamlib' && c.ptt === 'cat' && c.rig_name.toLowerCase().includes('icom') && c.baud !== 115200;
}
