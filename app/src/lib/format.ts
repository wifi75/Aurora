export function formatMhz(hz: number): string {
  return `${(hz / 1e6).toFixed(3)} MHz`;
}

/** Millisecondi dalla mezzanotte UTC come HH:MM:SS; oltre le 24 ore riparte da zero. */
export function formatClock(ms: number): string {
  const total = ((Math.floor(ms / 1000) % 86_400) + 86_400) % 86_400;
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(Math.floor(total / 3600))}:${pad(Math.floor(total / 60) % 60)}:${pad(total % 60)}`;
}

// [nome, da (kHz), a (kHz)]
const BANDS: ReadonlyArray<readonly [string, number, number]> = [
  ['160m', 1800, 2000],
  ['80m', 3500, 4000],
  ['60m', 5250, 5450],
  ['40m', 7000, 7300],
  ['30m', 10_100, 10_150],
  ['20m', 14_000, 14_350],
  ['17m', 18_068, 18_168],
  ['15m', 21_000, 21_450],
  ['12m', 24_890, 24_990],
  ['10m', 28_000, 29_700],
  ['6m', 50_000, 54_000],
  ['2m', 144_000, 148_000],
];

export function bandOf(hz: number): string | null {
  const khz = hz / 1000;
  return BANDS.find(([, from, to]) => khz >= from && khz <= to)?.[0] ?? null;
}
