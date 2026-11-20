export type DotState = 'ok' | 'bad' | 'unknown';

export interface TxChainInfo {
  peak_dbfs?: number | null;
  ptt_read?: boolean | null;
  audio_muted?: boolean;
  radio_mode?: string | null;
}

export interface ChainDot {
  key: 'mode' | 'audio' | 'ptt';
  state: DotState;
  /** Valore da mostrare nel suggerimento (modo letto, picco in dB). */
  value: string;
}

/** I modi della radio che non portano una trasmissione digitale: il PTT scatta ma non esce quello che serve. */
const NOT_FOR_DIGITAL = /^(CW|CWR|FM|WFM|AM|RTTY|RTTYR)$/i;

/** I tre pallini della catena: modo radio, audio, PTT. Senza rapporto (nessuna trasmissione ancora) non c'e' niente da mostrare. */
export function chainDots(chain: TxChainInfo | null | undefined): ChainDot[] {
  if (!chain) return [];
  const mode = chain.radio_mode?.trim() ?? '';
  const peak = chain.peak_dbfs;
  return [
    { key: 'mode', state: mode === '' ? 'unknown' : NOT_FOR_DIGITAL.test(mode) ? 'bad' : 'ok', value: mode },
    { key: 'audio', state: chain.audio_muted === true ? 'bad' : peak == null ? 'unknown' : 'ok', value: peak == null ? '' : `${peak.toFixed(1)} dBFS` },
    { key: 'ptt', state: chain.ptt_read === true ? 'ok' : chain.ptt_read === false ? 'bad' : 'unknown', value: '' },
  ];
}
