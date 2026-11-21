/**
 * Le stazioni che trasmettono cartine meteo in WEFAX (fax meteo, isobariche), con le frequenze assegnate in kHz.
 * Fonti: «Worldwide Marine Radiofacsimile Broadcast Schedules» del National Weather Service (NOAA), edizione del 7 marzo 2025
 * (https://weather.gov/media/marine/rfax.pdf), e per la Germania gli orari del Deutscher Wetterdienst (DDH3 3855, DDK3 7880, DDK6 13882,5 kHz,
 * https://www.dwd.de/EN/specialusers/shipping/broadcast_en/broadcast_node.html). Messe fuori di proposito: stazioni cessate (Canada 2019,
 * Wellington 2023), con dati del 1997-2014 o segnate con asterisco senza conferma (Pevek, Atene, Murmansk, Bangkok, Sydney NS, Guangzhou).
 * Le frequenze sono quelle ASSEGNATE: la portante, e quindi la frequenza della radio in USB, e' 1,9 kHz sotto.
 * I programmi e le frequenze cambiano: controllare di tanto in tanto l'edizione nuova (vedi `docs/da-fare.md`).
 */
export interface WefaxStation {
  /** Nominativo della stazione. */
  call: string;
  /** Localita' e paese. */
  place: string;
  /** Frequenze assegnate, in kHz. */
  khz: readonly number[];
  /** IOC e righe al minuto della stazione (colonna «RPM/IOC» del documento NOAA: 120/576 per tutte quelle dell'elenco; DWD: stessi valori). */
  ioc?: number;
  lpm?: number;
}

export const WEFAX_STATIONS: readonly WefaxStation[] = [
  { call: 'DDH3 · DDK3 · DDK6', place: 'Pinneberg (DE)', khz: [3855, 7880, 13882.5] },
  { call: 'GYA', place: 'Northwood (UK)', khz: [2618.5, 4610, 8040, 11086.5] },
  { call: 'JMH', place: 'Tokyo (JP)', khz: [3622.5, 7795, 13988.5] },
  { call: 'JFX', place: 'Kagoshima (JP)', khz: [4274, 8658, 13074, 16907.5, 22559.6] },
  { call: 'HLL2', place: 'Seoul (KR)', khz: [3585, 5857.5, 7433.5, 9165, 13570] },
  { call: 'CBV', place: 'Valparaiso (CL)', khz: [4228, 8677, 17146.4] },
  { call: 'CBM', place: 'Punta Arenas (CL)', khz: [4322, 8696] },
  { call: 'NOJ', place: 'Kodiak (US)', khz: [2054, 4298, 8459, 12412.5] },
  { call: 'NMC', place: 'Pt. Reyes (US)', khz: [4346, 8682, 12786, 17151.2, 22527] },
  { call: 'NMG', place: 'New Orleans (US)', khz: [4317.9, 8503.9, 12789.9, 17146.4] },
  { call: 'NMF', place: 'Boston (US)', khz: [4235, 6340.5, 9110, 12750] },
  { call: 'KVM70', place: 'Honolulu (US)', khz: [9982.5, 11090, 16135] },
  { call: 'VMC', place: 'Charleville (AU)', khz: [2628, 5100, 11030, 13920, 20469] },
  { call: 'VMW', place: 'Wiluna (AU)', khz: [5755, 7535, 10555, 15615, 18060] },
];

/** La portante e' 1,9 kHz sotto la frequenza assegnata: e' la frequenza della radio in USB. */
export const WEFAX_CARRIER_OFFSET_HZ = 1900;

export interface WefaxPreset {
  id: string;
  call: string;
  place: string;
  /** Frequenza assegnata, kHz. */
  khz: number;
  /** Frequenza della radio in USB, Hz. */
  dialHz: number;
  /** Banda: la parte intera dei MHz (4 = «4 MHz», da 4,0 a 4,99). */
  band: number;
  /** IOC e righe al minuto da impostare al ricevitore scegliendo questa frequenza. */
  ioc: number;
  lpm: number;
}

/** Valori di tutte le stazioni dell'elenco (NOAA, edizione del 7 marzo 2025). */
export const WEFAX_DEFAULT_IOC = 576;
export const WEFAX_DEFAULT_LPM = 120;

/** Tutte le frequenze, ordinate per banda e poi per frequenza. */
export function wefaxPresets(): WefaxPreset[] {
  const out: WefaxPreset[] = [];
  for (const s of WEFAX_STATIONS) {
    // con un nominativo per frequenza (Pinneberg: DDH3 3855, DDK3 7880, DDK6 13882,5) ogni riga mostra solo il suo
    const calls = s.call.split(' · ');
    for (const [i, khz] of s.khz.entries()) {
      const call = (calls.length === s.khz.length ? calls[i] : undefined) ?? s.call;
      out.push({ id: `${s.call}-${khz}`, call, place: s.place, khz, dialHz: Math.round(khz * 1000) - WEFAX_CARRIER_OFFSET_HZ, band: Math.floor(khz / 1000), ioc: s.ioc ?? WEFAX_DEFAULT_IOC, lpm: s.lpm ?? WEFAX_DEFAULT_LPM });
    }
  }
  return out.sort((a, b) => a.khz - b.khz || a.call.localeCompare(b.call));
}

/** Le frequenze raggruppate per banda (dalla piu' bassa), ognuna gia' ordinata per frequenza. */
export function wefaxGroups(): { band: number; presets: WefaxPreset[] }[] {
  const groups: { band: number; presets: WefaxPreset[] }[] = [];
  for (const p of wefaxPresets()) {
    const last = groups[groups.length - 1];
    if (last && last.band === p.band) last.presets.push(p);
    else groups.push({ band: p.band, presets: [p] });
  }
  return groups;
}

/** La frequenza come si legge: «3 855» kHz o «13 882,5» kHz (virgola decimale solo se serve). */
export function formatKhz(khz: number): string {
  const [int, dec] = khz.toString().split('.');
  const grouped = (int ?? '').replace(/\B(?=(\d{3})+(?!\d))/g, ' ');
  return dec ? `${grouped},${dec}` : grouped;
}

/** Il preset su cui e' la radio ora (entro 150 Hz), se c'e'. */
export function presetAt(dialHz: number): WefaxPreset | null {
  if (dialHz <= 0) return null;
  return wefaxPresets().find((p) => Math.abs(p.dialHz - dialHz) <= 150) ?? null;
}
