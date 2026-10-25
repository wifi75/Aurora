import { create } from 'zustand';
import { browserStorage } from './theme';

/**
 * Come si disegnano i messaggi gia' lavorati e i marcatori (scheda Notifiche, come nel motore). Sono scelte dell'interfaccia: valgono subito
 * e, come le altre preferenze, si copiano sul server (per dispositivo).
 *
 * Nomi nel `JTDX.ini`: `workedColor`, `workedStriked`, `workedUnderlined`, `workedDontShow`, `txtColor`, `OtherStandardMessagesMarker`,
 * `redMarker`, `blueMarker`, `hidehintMarker`, `differentBackground` (`cyanBackground` / `yellowBackground`), `highlightDXCall`, `highlightDXGrid`, `BeepOnFirstMsg`.
 */
export interface NotifyView {
  /** «Evidenzia gia' lavorati»: il messaggio gia' lavorato ha il suo colore (di base spento: solo barrato). */
  workedColor: boolean;
  /** Barra il messaggio gia' lavorato (di base acceso). Esclude «sottolinea», come nel motore. */
  workedStriked: boolean;
  /** Sottolinea il messaggio gia' lavorato. Esclude «barra». */
  workedUnderlined: boolean;
  /** «Non mostrare»: i messaggi gia' lavorati si nascondono (non quelli rivolti a me ne' le stazioni desiderate). */
  workedDontShow: boolean;
  /** «Inverti colore sfondo/testo»: il colore del tipo di messaggio va sul testo invece che sullo sfondo. */
  invert: boolean;
  /** «Altri messaggi predefiniti»: marcatore sui messaggi tra altre stazioni con una novita'. */
  otherMarker: boolean;
  /** «Rosso - il mio nominativo nel messaggio»: marcatore sui messaggi rivolti a me. */
  redMarker: boolean;
  /** «Blu - il mio nominativo e' nel QSO»: il marcatore diventa blu se il messaggio e' della stazione che sto chiamando. Vale solo con il rosso. */
  blueMarker: boolean;
  /** «Nascondendo suggerimento»: nasconde il marcatore dei suggerimenti (il decodificatore di Aurora non ne produce). */
  hideHint: boolean;
  /** «Usa un colore di sfondo diverso per il mio nominativo nel messaggio» (`differentBackground`): i messaggi che contengono il mio nominativo hanno lo sfondo ciano o giallo. */
  differentBackground: boolean;
  /** Il colore di quello sfondo: `false` ciano (`cyanBackground`), `true` giallo (`yellowBackground`). */
  diffYellow: boolean;
  /** «Evidenzia i messaggi con il nominativo DX» (`highlightDXCall`): il nominativo della stazione che sto chiamando si evidenzia nei messaggi. Aurora l'ha sempre fatto: di base acceso. */
  highlightDxCall: boolean;
  /** «Evidenzia i messaggi con il locatore DX» (`highlightDXGrid`). */
  highlightDxGrid: boolean;
  /** «Suono al primo messaggio decodificato» (`BeepOnFirstMsg`): suona all'arrivo dei messaggi di un nuovo periodo. */
  beepFirst: boolean;
  /** «Play alert sound» (`AlertOnMyCall`, `AlertOnDXCall`, `AlertOnNewCQZ/ITUZ/DXCC/Grid/Px`): l'avviso con il suono scelto. */
  alertMyCall: boolean;
  alertDxCall: boolean;
  alertCqz: boolean;
  alertItu: boolean;
  alertDxcc: boolean;
  alertGrid: boolean;
  alertPx: boolean;
  /** «per banda» (`AlertOnNew*OB`): l'avviso scatta anche per il «nuovo su banda/modo» di quel tipo. */
  alertCqzOb: boolean;
  alertItuOb: boolean;
  alertDxccOb: boolean;
  alertGridOb: boolean;
  alertPxOb: boolean;
}

/** Le chiavi degli avvisi, nell'ordine della schermata. */
export const ALERT_KEYS = ['alertMyCall', 'alertDxCall', 'alertCqz', 'alertItu', 'alertDxcc', 'alertGrid', 'alertPx', 'alertCqzOb', 'alertItuOb', 'alertDxccOb', 'alertGridOb', 'alertPxOb'] as const satisfies readonly (keyof NotifyView)[];
export type AlertKey = (typeof ALERT_KEYS)[number];

const KEY = 'aurora-notify-view';

type ReadableStorage = Pick<Storage, 'getItem'> | null;
type WritableStorage = Pick<Storage, 'setItem'> | null;

export function defaultNotifyView(): NotifyView {
  return { workedColor: false, workedStriked: true, workedUnderlined: false, workedDontShow: false, invert: false, otherMarker: true, redMarker: true, blueMarker: false, hideHint: false, differentBackground: false, diffYellow: false, highlightDxCall: true, highlightDxGrid: false, beepFirst: false, alertMyCall: true, alertDxCall: false, alertCqz: false, alertItu: false, alertDxcc: false, alertGrid: false, alertPx: false, alertCqzOb: false, alertItuOb: false, alertDxccOb: false, alertGridOb: false, alertPxOb: false };
}

/** Le dipendenze del motore: barra e sottolinea si escludono (vince la barra), il blu vale solo con il rosso. */
export function normalizeView(view: NotifyView): NotifyView {
  const next = { ...view };
  if (next.workedStriked && next.workedUnderlined) next.workedUnderlined = false;
  if (!next.redMarker) next.blueMarker = false;
  // «per banda» vale solo con l'avviso del suo tipo
  if (!next.alertCqz) next.alertCqzOb = false;
  if (!next.alertItu) next.alertItuOb = false;
  if (!next.alertDxcc) next.alertDxccOb = false;
  if (!next.alertGrid) next.alertGridOb = false;
  if (!next.alertPx) next.alertPxOb = false;
  return next;
}

export function readNotifyView(storage: ReadableStorage): NotifyView {
  const view = defaultNotifyView();
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return view;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    for (const key of Object.keys(view) as (keyof NotifyView)[]) {
      if (typeof parsed[key] === 'boolean') view[key] = parsed[key];
    }
  } catch {
    return defaultNotifyView();
  }
  return normalizeView(view);
}

export function saveNotifyView(storage: WritableStorage, view: NotifyView): void {
  try {
    storage?.setItem(KEY, JSON.stringify(view));
  } catch {
    // senza archivio le scelte non vengono ricordate, ma l'interfaccia funziona lo stesso
  }
}

/** Il disegno di un messaggio gia' lavorato: la cosa da fare al testo e il colore. */
export interface WorkedLook {
  strike: boolean;
  underline: boolean;
  color: boolean;
}

export function workedLook(view: NotifyView): WorkedLook {
  const v = normalizeView(view);
  return { strike: v.workedStriked, underline: v.workedUnderlined, color: v.workedColor };
}

interface NotifyViewState {
  view: NotifyView;
  set(change: Partial<NotifyView>): void;
  reset(): void;
}

export const useNotifyView = create<NotifyViewState>((set, get) => ({
  view: readNotifyView(browserStorage()),
  set(change) {
    const next = normalizeView({ ...get().view, ...change });
    saveNotifyView(browserStorage(), next);
    set({ view: next });
  },
  reset() {
    saveNotifyView(browserStorage(), defaultNotifyView());
    set({ view: defaultNotifyView() });
  },
}));
