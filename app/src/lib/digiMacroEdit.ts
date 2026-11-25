import type { DigiMacro, DigiSettings, DigiSnapshot } from '../api/client';
import type { Macro } from './macroLang';

/** Dove vale una macro: per tutti i modi o solo per il modo in uso. */
export type MacroScope = 'all' | 'mode';

/**
 * Le macro dopo una modifica fatta con l'editor (pagina PSK·RTTY·CW o Impostazioni › Modi › PSK · RTTY · CW › Macro): `original` e' il
 * tasto da cui si e' partiti (se lo si cambia, quel tasto si libera), `where` l'insieme in cui la macro finisce. Una macro vuota toglie il
 * tasto; vuota «solo per questo modo» lascia il tasto libero anche se la macro di base esiste.
 * `onlyGroup` (schede «Comuni» e «Solo per questo modo» delle Impostazioni): si tocca solo il gruppo `where`, l'altro resta com'e'.
 */
export function editMacros(s: Pick<DigiSettings, 'macros' | 'mode_macros' | 'mode'>, original: string, m: Macro, where: MacroScope, onlyGroup = false): Pick<DigiSettings, 'macros' | 'mode_macros'> {
  const blank = m.label.trim() === '' && m.text.trim() === '';
  const strip = (list: DigiMacro[], name: string) => list.filter((x) => x.name !== name);
  const entry: DigiMacro = { name: m.name, label: m.label, text: m.text, color: m.color };
  let all = [...s.macros];
  const modeSets = { ...s.mode_macros };
  let forMode = [...(modeSets[s.mode] ?? [])];
  // il tasto di partenza si libera dall'insieme che si sta modificando se e' stato cambiato
  if (original !== m.name) {
    if (!onlyGroup || where === 'all') all = strip(all, original);
    if (!onlyGroup || where === 'mode') forMode = strip(forMode, original);
  }
  if (where === 'all') {
    all = strip(all, m.name);
    if (!onlyGroup) forMode = strip(forMode, m.name);
    if (!blank) all.push(entry);
  } else {
    forMode = strip(forMode, m.name);
    if (!blank || all.some((x) => x.name === m.name)) forMode.push(blank ? { name: m.name, label: '', text: '', color: null } : entry);
  }
  modeSets[s.mode] = forMode;
  return { macros: all, mode_macros: modeSets };
}

/** Un core piu' vecchio puo' mandare meno campi: i mancanti si riempiono con i valori di base (macro senza tasto = nessuna). */
export function digiWithDefaults(snap: DigiSnapshot): DigiSnapshot {
  const raw = snap.settings as Partial<DigiSettings>;
  const settings: DigiSettings = {
    // prima tutto quello che manda il core: i campi senza un valore di base qui sotto (tutta la banda: `band_cw`, `band_psk`, `local_spots`,
    // `band_validation`...) devono restare, se no la pagina non li vede e ogni salvataggio li rimanda al core vuoti (tornano ai valori di base)
    ...raw,
    mode: raw.mode ?? 'psk31',
    rx_freq_hz: raw.rx_freq_hz ?? 1000,
    tx_follow_rx: raw.tx_follow_rx ?? true,
    tx_freq_hz: raw.tx_freq_hz ?? 1000,
    cw_wpm: raw.cw_wpm ?? 20,
    cw_wpm_follow_rx: raw.cw_wpm_follow_rx ?? false,
    cw_bandwidth_hz: raw.cw_bandwidth_hz ?? 150,
    cw_squelch_snr_db: raw.cw_squelch_snr_db ?? -15,
    cw_som: raw.cw_som ?? false,
    cw_receiver: raw.cw_receiver ?? 'fast',
    wf_anticlick: raw.wf_anticlick ?? true,
    psk_squelch: raw.psk_squelch ?? 15,
    rtty_shift_hz: raw.rtty_shift_hz ?? 170,
    rtty_baud: raw.rtty_baud ?? 45.45,
    rtty_reverse: raw.rtty_reverse ?? false,
    rtty_uos: raw.rtty_uos ?? true,
    rtty_squelch_snr_db: raw.rtty_squelch_snr_db ?? 0,
    rtty_receiver: raw.rtty_receiver ?? 'auto',
    rtty_charset: raw.rtty_charset ?? 'ita2',
    rtty_diddle: raw.rtty_diddle ?? true,
    rtty_profile: raw.rtty_profile ?? 0,
    rtty_profiles: raw.rtty_profiles ?? [],
    afc_on: raw.afc_on ?? true,
    squelch_on: raw.squelch_on ?? false,
    snap_on: raw.snap_on ?? true,
    auto_receive: raw.auto_receive ?? false,
    macros: (raw.macros ?? []).filter((m) => typeof m.name === 'string' && m.name !== ''),
    mode_macros: raw.mode_macros ?? {},
    merge_macros: raw.merge_macros ?? {},
  };
  return { ...snap, settings, rx: { ...snap.rx, freq_hz: snap.rx.freq_hz ?? 0, squelch_open: snap.rx.squelch_open ?? true } };
}

/**
 * Due tasti della barra si scambiano le macro (trascinamento o Alt+freccia): la macro prende il tasto della nuova posizione.
 * Regola: lo scambio si fa in OGNI insieme (quello di base e quello del modo in uso), cosi' ogni macro resta nel suo insieme
 * (per tutti i modi o solo per questo modo) e la barra che si vede dopo e' esattamente quella di prima con i due tasti scambiati.
 * Un tasto vuoto riceve la macro e lascia libero quello di partenza. Gli altri modi non cambiano.
 */
export function swapMacros(s: Pick<DigiSettings, 'macros' | 'mode_macros' | 'mode'>, a: string, b: string): Pick<DigiSettings, 'macros' | 'mode_macros'> {
  const swap = (list: readonly DigiMacro[]) =>
    list.map((m) => (m.name === a ? { ...m, name: b } : m.name === b ? { ...m, name: a } : m));
  const modeSets = { ...s.mode_macros };
  if (a !== b && modeSets[s.mode]) modeSets[s.mode] = swap(modeSets[s.mode] ?? []);
  return { macros: a === b ? [...s.macros] : swap(s.macros), mode_macros: modeSets };
}
