import type { EngineSettings } from '../api/schemas';
import { useT, type Key } from '../i18n';
import type { SettingRow } from '../lib/engineSettingsLayout';
import { SettingField, type Draft } from './NativeSettings';

/** «Fast/Normal/Deep» del menu Decode di WSJT-X (`NDepth & 7`). */
const DEPTH_OPTIONS: SettingRow['options'] = [
  { value: '1', label: 'dec.depth1' },
  { value: '2', label: 'dec.depth2' },
  { value: '3', label: 'dec.depth3' },
];

/** Modalita' «PC lento»: automatica, sempre accesa, mai (voce di Aurora `SlowPcMode`). */
const SLOW_PC_OPTIONS: SettingRow['options'] = [
  { value: 'auto', label: 'dec.slowPc.auto' },
  { value: 'always', label: 'dec.slowPc.always' },
  { value: 'never', label: 'dec.slowPc.never' },
];

/** I valori della tendina «F Tol» di WSJT-X: tutti i modi (`mainwindow.cpp:12296`) e MSK144 (`mainwindow.cpp:12114`). */
export const FTOL_VALUES = [1, 2, 5, 10, 20, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000] as const;
export const FTOL_MSK144_VALUES = [20, 50, 100, 150, 200, 250, 300, 350, 400, 450, 500] as const;
const hz = (values: readonly number[]): SettingRow['options'] => values.map((v) => ({ value: String(v), text: `${v} Hz` }));

/**
 * Le opzioni dei decodificatori di WSJT-X che il motore proprio usa, con i valori di fabbrica di WSJT-X 3.3: profondita', «Single decode»,
 * media e «Auto Clear Avg» di Q65, «Enable AP», ricerca profonda e livello aggressivo di JT65, F Tol per modo.
 */
export const DECODER_ROWS: readonly { row: SettingRow; label: Key }[] = [
  { row: { key: 'DecodeDepth', kind: 'choice', options: DEPTH_OPTIONS }, label: 'dec.depth' },
  { row: { key: 'SingleDecode', kind: 'bool' }, label: 'dec.singleDecode' },
  { row: { key: 'SlowPcMode', kind: 'choice', options: SLOW_PC_OPTIONS }, label: 'dec.slowPc' },
  { row: { key: 'IncludeAveraging', kind: 'bool' }, label: 'dec.includeAveraging' },
  { row: { key: 'AutoClearAvg', kind: 'bool' }, label: 'dec.autoClearAvg' },
  { row: { key: 'Ftol_Q65', kind: 'choice', options: hz(FTOL_VALUES) }, label: 'dec.ftolQ65' },
  { row: { key: 'JT65AP', kind: 'bool' }, label: 'dec.jt65Ap' },
  { row: { key: 'IncludeCorrelation', kind: 'bool' }, label: 'dec.correlation' },
  { row: { key: 'JT65Aggressive', kind: 'num', min: 0, max: 10 }, label: 'dec.aggressive' },
  { row: { key: 'Ftol_JT65', kind: 'choice', options: hz(FTOL_VALUES) }, label: 'dec.ftolJt65' },
  { row: { key: 'Ftol_JT9', kind: 'choice', options: hz(FTOL_VALUES) }, label: 'dec.ftolJt9' },
  { row: { key: 'Ftol_MSK144', kind: 'choice', options: hz(FTOL_MSK144_VALUES) }, label: 'dec.ftolMsk144' },
];

/** Quali opzioni stanno in ogni pagina dei modi: FT8 e FT4, JT65 e JT9, VHF ed EME (Q65 e MSK144). */
export type DecoderPart = 'all' | 'ft8' | 'jt' | 'vhf';
export const DECODER_PARTS: Readonly<Record<Exclude<DecoderPart, 'all'>, readonly string[]>> = {
  ft8: ['DecodeDepth', 'SingleDecode', 'SlowPcMode'],
  jt: ['JT65AP', 'IncludeCorrelation', 'JT65Aggressive', 'Ftol_JT65', 'Ftol_JT9'],
  vhf: ['IncludeAveraging', 'AutoClearAvg', 'Ftol_Q65', 'Ftol_MSK144'],
};

/** Le righe delle opzioni dei decodificatori (dentro il riquadro «Decodifica» della pagina del modo, con la stessa bozza e lo stesso Applica). */
export function DecoderOptionFields({ doc, draft, part = 'all' }: { doc: EngineSettings; draft: Draft; part?: DecoderPart }) {
  const t = useT();
  const rows = part === 'all' ? DECODER_ROWS : DECODER_ROWS.filter(({ row }) => DECODER_PARTS[part].includes(row.key));
  return (
    <>
      {rows.map(({ row, label }) => (
        <SettingField key={row.key} row={row} label={t(label)} doc={doc} draft={draft} />
      ))}
    </>
  );
}
