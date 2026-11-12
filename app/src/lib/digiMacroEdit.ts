import type { DigiMacro, DigiSettings, DigiSnapshot } from '../api/client';
import type { Macro } from './macroLang';

/** Dove vale una macro: per tutti i modi o solo per il modo in uso. */
export type MacroScope = 'all' | 'mode';

/**
 * Le macro dopo una modifica fatta con l'editor (pagina PSK·RTTY·CW o Impostazioni › Modi › PSK · RTTY · CW › Macro): `original` e' il
 * tasto da cui si e' partiti (se lo si cambia, quel tasto si libera), `where` l'insieme in cui la macro finisce. Una macro vuota toglie il
 * tasto; vuota «solo per questo modo» lascia il tasto libero anche se la macro di base esiste.
 */
export function editMacros(s: Pick<DigiSettings, 'macros' | 'mode_macros' | 'mode'>, original: string, m: Macro, where: MacroScope): Pick<DigiSettings, 'macros' | 'mode_macros'> {
  const blank = m.label.trim() === '' && m.text.trim() === '';
  const strip = (list: DigiMacro[], name: string) => list.filter((x) => x.name !== name);
  const entry: DigiMacro = { name: m.name, label: m.label, text: m.text, color: m.color };
  let all = [...s.macros];
  const modeSets = { ...s.mode_macros };
  let forMode = [...(modeSets[s.mode] ?? [])];
  // il tasto di partenza si libera dall'insieme che si sta modificando se e' stato cambiato
  if (original !== m.name) {
    all = strip(all, original);
    forMode = strip(forMode, original);
  }
  if (where === 'all') {
    all = strip(all, m.name);
    forMode = strip(forMode, m.name);
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
    mode: raw.mode ?? 'psk31',
    rx_freq_hz: raw.rx_freq_hz ?? 1000,
    tx_follow_rx: raw.tx_follow_rx ?? true,
    tx_freq_hz: raw.tx_freq_hz ?? 1000,
    cw_wpm: raw.cw_wpm ?? 20,
    rtty_shift_hz: raw.rtty_shift_hz ?? 170,
    rtty_baud: raw.rtty_baud ?? 45.45,
    rtty_reverse: raw.rtty_reverse ?? false,
    rtty_uos: raw.rtty_uos ?? true,
    rtty_squelch_snr_db: raw.rtty_squelch_snr_db ?? 6,
    rtty_receiver: raw.rtty_receiver ?? 'auto',
    afc_on: raw.afc_on ?? true,
    squelch_on: raw.squelch_on ?? false,
    squelch_db: raw.squelch_db ?? -45,
    snap_on: raw.snap_on ?? true,
    auto_receive: raw.auto_receive ?? false,
    macros: (raw.macros ?? []).filter((m) => typeof m.name === 'string' && m.name !== ''),
    mode_macros: raw.mode_macros ?? {},
    merge_macros: raw.merge_macros ?? {},
  };
  return { ...snap, settings, rx: { ...snap.rx, freq_hz: snap.rx.freq_hz ?? 0, squelch_open: snap.rx.squelch_open ?? true } };
}
