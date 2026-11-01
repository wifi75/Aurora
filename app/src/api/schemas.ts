import { z } from 'zod';

export const HealthSchema = z.object({
  status: z.literal('ok'),
  name: z.string(),
  version: z.string(),
  author: z.string(),
});
export type Health = z.infer<typeof HealthSchema>;

/** Strumenti letti dal CAT della radio: ognuno e' assente se la radio non lo da'. */
export const RadioMetersSchema = z.object({
  /** S-meter in punti S, 0-9. */
  s_points: z.number().optional(),
  /** Decibel sopra S9, 0-60. */
  s_over_db: z.number().optional(),
  power_w: z.number().optional(),
  swr: z.number().optional(),
  /** Livello ALC, 0-1. */
  alc: z.number().optional(),
  comp_db: z.number().optional(),
  id_a: z.number().optional(),
  vdd_v: z.number().optional(),
  /** `true`: s_points e s_over_db non vengono dalla radio ma sono stimati dal livello dell'audio in ingresso. */
  estimated: z.boolean().optional(),
});
export type RadioMeters = z.infer<typeof RadioMetersSchema>;

export const TxAlertSchema = z.object({
  kind: z.enum(['unrepresentable', 'high_swr', 'inhibited', 'hound_on']),
  detail: z.string().optional(),
  value: z.number().optional(),
  limit: z.number().optional(),
});
export type TxAlert = z.infer<typeof TxAlertSchema>;

export const StationStatusSchema = z.object({
  dial_hz: z.number(),
  mode: z.string(),
  tx_enabled: z.boolean(),
  transmitting: z.boolean(),
  decoding: z.boolean(),
  de_call: z.string(),
  de_grid: z.string(),
  dx_call: z.string().nullable().optional(),
  dx_grid: z.string().nullable().optional(),
  report: z.string().nullable().optional(),
  tx_df: z.number().nullable().optional(),
  rx_df: z.number().nullable().optional(),
  tx_message: z.string().nullable().optional(),
  tr_period: z.number().nullable().optional(),
  own_continent: z.string().nullable().optional(),
  meters: RadioMetersSchema.nullable().optional(),
  /** Le stazioni in coda per il prossimo QSO. */
  queue: z.array(z.string()).optional(),
  /** Avvisi di sicurezza della trasmissione: messaggio non rappresentabile, ROS alto, Tx bloccata da un controller UDP. */
  tx_alerts: z.array(TxAlertSchema).optional(),
  /** Hound: esito della verifica in linea del codice OTP del Fox. */
  fox_verification: z.object({ call: z.string(), verified: z.boolean() }).nullable().optional(),
});
export type StationStatus = z.infer<typeof StationStatusSchema>;

export const DecodeTagSchema = z.enum([
  'cq',
  'new_dxcc',
  'new_dxcc_band',
  'new_zone',
  'new_zone_band',
  'new_itu',
  'new_itu_band',
  'new_grid',
  'new_grid_band',
  'new_px',
  'new_px_band',
  'new_call',
  'new_call_band',
  'worked_call',
  'auto',
]);
export type DecodeTag = z.infer<typeof DecodeTagSchema>;

export const DecodedMessageSchema = z.object({
  id: z.number(),
  time_ms: z.number(),
  snr: z.number(),
  delta_time: z.number(),
  freq_hz: z.number(),
  text: z.string(),
  tag: DecodeTagSchema.nullable(),
  country: z.string().nullable().optional(),
  continent: z.string().nullable().optional(),
});
export type DecodedMessage = z.infer<typeof DecodedMessageSchema>;

export const QsoRecordSchema = z.object({
  call: z.string(),
  grid: z.string(),
  band: z.string(),
  mode: z.string(),
  freq_hz: z.number(),
  rst_sent: z.string(),
  rst_rcvd: z.string(),
  time_ms: z.number(),
});
export type QsoRecord = z.infer<typeof QsoRecordSchema>;

export const WantedAlertSchema = z.object({
  call: z.string(),
  reason: z.string(),
  source: z.string(),
  text: z.string(),
  freq_hz: z.number().nullable().optional(),
  band: z.string().nullable().optional(),
  snr: z.number().nullable().optional(),
  country: z.string().nullable().optional(),
  grid: z.string().nullable().optional(),
  time_ms: z.number(),
});
export type WantedAlert = z.infer<typeof WantedAlertSchema>;

export const UpdateNoticeSchema = z.object({ version: z.string(), tag: z.string(), notes: z.string(), url: z.string() });
export type UpdateNotice = z.infer<typeof UpdateNoticeSchema>;

export const AudioLevelSchema = z.object({
  peak: z.number(),
  rms: z.number(),
  clipped: z.number(),
  state: z.enum(['silent', 'low', 'good', 'high', 'overdrive']),
});
export type AudioLevel = z.infer<typeof AudioLevelSchema>;

/** La banda della radio e' cambiata: il motore ha gia' fermato la trasmissione e azzerato il DX; l'interfaccia svuota le liste. */
export const BandChangedSchema = z.object({ band: z.string().nullable(), dial_hz: z.number(), epoch: z.number() });
export type BandChanged = z.infer<typeof BandChangedSchema>;

/** Un programma collegato via UDP (Highlight Callsign) chiede colori per un nominativo: `#rrggbb`; entrambi `null` = togli. */
export const HighlightCallsignSchema = z.object({
  call: z.string(),
  background: z.string().nullable(),
  foreground: z.string().nullable(),
  /** Solo la riga piu' recente con quel nominativo. */
  last_only: z.boolean(),
});
export type HighlightCallsign = z.infer<typeof HighlightCallsignSchema>;

export const EventSchema = z.discriminatedUnion('type', [
  StationStatusSchema.extend({ type: z.literal('status') }),
  DecodedMessageSchema.extend({ type: z.literal('decode') }),
  QsoRecordSchema.extend({ type: z.literal('qso_logged') }),
  z.object({ type: z.literal('tx_halted') }),
  WantedAlertSchema.extend({ type: z.literal('wanted') }),
  UpdateNoticeSchema.extend({ type: z.literal('update_available') }),
  AudioLevelSchema.extend({ type: z.literal('audio_level') }),
  BandChangedSchema.extend({ type: z.literal('band_changed') }),
  HighlightCallsignSchema.extend({ type: z.literal('highlight_callsign') }),
  /** Un programma collegato via UDP (Clear) chiede di svuotare le liste, come «Cancella». */
  z.object({ type: z.literal('clear_decodes'), window: z.number() }),
]);
export type AuroraEvent = z.infer<typeof EventSchema>;

/** Impostazioni del motore proprio (`GET/PUT /api/v1/engine-settings`): valori e, per ogni voce, se e' viva o non disponibile (con il motivo). */
export const EngineSettingsItemSchema = z.object({
  state: z.enum(['live', 'unavailable']),
  /** `now`: ha effetto subito; `restart`: dal prossimo avvio di Aurora. */
  apply: z.enum(['now', 'restart']).optional(),
  reason: z.string().optional(),
  /** Una nota per le voci vive che in FT8 e FT4 non cambiano niente (come in JTDX). */
  note: z.string().optional(),
});
export type EngineSettingsItem = z.infer<typeof EngineSettingsItemSchema>;

export const EngineSettingsSchema = z.object({
  version: z.number(),
  file: z.string(),
  values: z.record(z.string(), z.string()),
  items: z.record(z.string(), EngineSettingsItemSchema),
  /** Segreti: solo se ci sono (mai il valore). */
  secrets: z.record(z.string(), z.boolean()).optional(),
  /** Esito dell'ultimo invio di ogni servizio. */
  status: z.record(z.string(), z.object({ ok: z.boolean().nullable(), at_ms: z.number(), message: z.string(), sent: z.number(), failed: z.number(), pending: z.number().optional() })).optional(),
});
export type EngineSettings = z.infer<typeof EngineSettingsSchema>;

/** Esito della prova di un servizio dell'invio diretto dei QSO (`POST /api/v1/qso-upload/test`). */
export const UploadTestSchema = z.object({ ok: z.boolean(), message: z.string() });

/** Esito dello spot su DXSummit (`POST /api/v1/spot/dxsummit`): chi, su che frequenza (Hz) e con che testo. */
export const SpotResultSchema = z.object({ ok: z.boolean(), dx_call: z.string(), frequency_hz: z.number(), info: z.string() });
export type SpotResult = z.infer<typeof SpotResultSchema>;
