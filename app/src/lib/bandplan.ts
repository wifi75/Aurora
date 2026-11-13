import type { FrequencyEntry } from '../api/client';

/**
 * Frequenze di chiamata dei modi testuali e dell'SSTV dai piani di banda IARU (dal 6 m in su quelli del Band Plan ARI, https://www.ari.it/bandplan.html) (frequenza della manopola in USB, cioe' dove si sente
 * il segnale con la portante audio a circa 1000 Hz). Sono indicative: ogni regione ha le sue sezioni e i QSO si spostano dove c'e' posto.
 * Regione: `ALL` valida ovunque, `R1`, `R2` o `R3` solo li' (come nella tabella dei modi dell'FT8).
 */
export type TextMode = 'PSK31' | 'PSK63' | 'RTTY' | 'SSTV' | 'CW';

type Row = readonly [band: string, hz: number, region: 'ALL' | 'R1' | 'R2' | 'R3', preferred: boolean];

const PSK: readonly Row[] = [
  ['160m', 1_838_000, 'ALL', true],
  ['80m', 3_580_000, 'ALL', true],
  ['40m', 7_040_000, 'R1', true],
  ['40m', 7_070_000, 'R2', true],
  ['40m', 7_070_000, 'R3', true],
  ['30m', 10_142_000, 'ALL', true],
  ['20m', 14_070_000, 'ALL', true],
  ['17m', 18_100_000, 'ALL', true],
  ['15m', 21_070_000, 'ALL', true],
  ['12m', 24_920_000, 'ALL', true],
  ['10m', 28_120_000, 'ALL', true],
  ['6m', 50_305_000, 'ALL', true],
  ['70cm', 432_088_000, 'ALL', true],
  ['23cm', 1_296_138_000, 'ALL', true],
  ['13cm', 2_320_138_000, 'ALL', true],
];

const RTTY: readonly Row[] = [
  ['160m', 1_838_000, 'ALL', false],
  ['80m', 3_590_000, 'ALL', true],
  ['40m', 7_040_000, 'R1', true],
  ['40m', 7_080_000, 'R2', true],
  ['40m', 7_040_000, 'R3', true],
  ['30m', 10_140_000, 'ALL', true],
  ['20m', 14_080_000, 'ALL', true],
  ['17m', 18_100_000, 'ALL', true],
  ['15m', 21_080_000, 'ALL', true],
  ['12m', 24_920_000, 'ALL', true],
  ['10m', 28_080_000, 'ALL', true],
  ['6m', 50_620_000, 'ALL', true],
  ['2m', 144_600_000, 'ALL', true],
  ['23cm', 1_296_600_000, 'ALL', true],
];

const SSTV: readonly Row[] = [
  ['80m', 3_735_000, 'R1', true],
  ['80m', 3_845_000, 'R2', true],
  ['80m', 3_735_000, 'R3', true],
  ['40m', 7_165_000, 'R1', true],
  ['40m', 7_171_000, 'R2', true],
  ['40m', 7_033_000, 'R3', true],
  ['20m', 14_230_000, 'ALL', true],
  ['15m', 21_340_000, 'ALL', true],
  ['10m', 28_680_000, 'ALL', true],
  ['6m', 50_510_000, 'ALL', true],
  ['2m', 144_500_000, 'ALL', true],
  ['70cm', 433_400_000, 'ALL', true],
  ['23cm', 1_296_500_000, 'ALL', true],
];

// CW dai piani di banda dell'ARI: HF dal piano IARU Regione 1 (https://www.ari.it/band-plan-hf-iaru-reg-1.html), dal 6 m in su dal piano
// ARI (https://www.ari.it/bandplan.html). In HF non ci sono frequenze di chiamata CW: una frequenza dentro il tratto CW (preferita), poi i
// centri di attivita' QRS e QRP e, in 80 m, il tratto per i collegamenti intercontinentali; dal 6 m in su i centri di attivita' CW, l'EME e
// il meteor scatter.
const CW: readonly Row[] = [
  ['160m', 1_825_000, 'ALL', true],
  ['160m', 1_836_000, 'ALL', false],
  ['80m', 3_505_000, 'ALL', false],
  ['80m', 3_530_000, 'ALL', true],
  ['80m', 3_555_000, 'ALL', false],
  ['80m', 3_560_000, 'ALL', false],
  ['60m', 5_352_500, 'ALL', true],
  ['40m', 7_015_000, 'ALL', true],
  ['40m', 7_030_000, 'ALL', false],
  ['30m', 10_110_000, 'ALL', true],
  ['30m', 10_116_000, 'ALL', false],
  ['20m', 14_025_000, 'ALL', true],
  ['20m', 14_055_000, 'ALL', false],
  ['20m', 14_060_000, 'ALL', false],
  ['17m', 18_080_000, 'ALL', true],
  ['17m', 18_086_000, 'ALL', false],
  ['15m', 21_025_000, 'ALL', true],
  ['15m', 21_055_000, 'ALL', false],
  ['15m', 21_060_000, 'ALL', false],
  ['12m', 24_900_000, 'ALL', true],
  ['12m', 24_906_000, 'ALL', false],
  ['10m', 28_025_000, 'ALL', true],
  ['10m', 28_055_000, 'ALL', false],
  ['10m', 28_060_000, 'ALL', false],
  ['6m', 50_050_000, 'ALL', true],
  ['6m', 50_090_000, 'ALL', false],
  ['2m', 144_025_000, 'ALL', false],
  ['2m', 144_050_000, 'ALL', true],
  ['2m', 144_100_000, 'ALL', false],
  ['70cm', 432_010_000, 'ALL', false],
  ['70cm', 432_050_000, 'ALL', true],
  ['23cm', 1_296_010_000, 'ALL', false],
  ['23cm', 1_296_200_000, 'ALL', true],
  ['13cm', 2_320_200_000, 'ALL', true],
  ['6cm', 5_760_200_000, 'ALL', true],
  ['3cm', 10_368_200_000, 'ALL', true],
  ['1.2cm', 24_048_200_000, 'ALL', true],
];

const PLAN: Record<TextMode, readonly Row[]> = { PSK31: PSK, PSK63: PSK, RTTY, SSTV, CW };

/** Le righe del piano di banda di un modo, nella forma della tabella delle frequenze (cosi' le mostra lo stesso selettore dell'FT8). */
export function bandPlanRows(mode: TextMode): FrequencyEntry[] {
  return PLAN[mode].map(([band, hz, region, preferred]) => ({ band, mode, hz, region, preferred }));
}
