import { z } from 'zod';

export const HealthSchema = z.object({
  status: z.literal('ok'),
  name: z.string(),
  version: z.string(),
  author: z.string(),
  /** Il sistema del PC dove gira Aurora (`windows`, `linux`): OmniRig e il volume di Windows ci sono solo su Windows. */
  os: z.string().optional(),
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
  kind: z.enum(['unrepresentable', 'high_swr', 'inhibited', 'hound_on', 'fox_detected', 'hound_off']),
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
  /** Hound: in questi minuti si riceve un Fox UltraFOX/SuperFox (etichetta «Super Hound»). */
  super_hound: z.boolean().optional(),
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
  /** Data (AAAA-MM-GG) dell'ultimo invio a LoTW di chi trasmette, se e' nell'elenco pubblico dell'ARRL. */
  lotw_last_upload: z.string().nullable().optional(),
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

const FoxCallerSchema = z.object({ call: z.string(), grid: z.string(), snr: z.number(), freq_hz: z.number(), age: z.number(), order: z.number().optional().default(0) });
const FoxLogSchema = z.object({ call: z.string(), grid: z.string(), band: z.string(), mode: z.string(), freq_hz: z.number(), sent: z.string(), rcvd: z.string(), time_ms: z.number(), continent: z.string().optional() });
export type FoxLogEntry = z.infer<typeof FoxLogSchema>;
export const FoxSnapshotSchema = z.object({
  enabled: z.boolean(),
  slots: z.number(),
  more_cqs: z.boolean(),
  cq: z.string(),
  auto_queue: z.boolean(),
  ultra: z.boolean().optional().default(false),
  callers: z.array(FoxCallerSchema),
  queue: z.array(FoxCallerSchema),
  in_progress: z.array(z.object({ call: z.string(), sent: z.string(), rcvd: z.string().nullable().optional(), tries: z.number(), rr73_sent: z.boolean() })),
  tx: z.array(z.string()),
  periods: z.number(),
  logged: z.number(),
  otp: z.boolean().optional().default(false),
  otp_interval: z.number().optional().default(1),
  otp_code: z.string().nullable().optional(),
  guard: z.string().nullable().optional(),
  log: z.array(FoxLogSchema).optional().default([]),
  params: z.object({ strikes: z.number(), wait: z.number(), cq_every: z.number(), queue_max: z.number() }).optional().default({ strikes: 3, wait: 3, cq_every: 10, queue_max: 10 }),
});
export type FoxSnapshot = z.infer<typeof FoxSnapshotSchema>;

export const EventSchema = z.discriminatedUnion('type', [
  FoxSnapshotSchema.extend({ type: z.literal('fox') }),
  StationStatusSchema.extend({ type: z.literal('status') }),
  DecodedMessageSchema.extend({ type: z.literal('decode') }),
  QsoRecordSchema.extend({ type: z.literal('qso_logged') }),
  z.object({
    type: z.literal('digi'),
    kind: z.string(),
    mode: z.string().optional(),
    text: z.string().optional(),
    state: z.string().optional(),
    progress: z.number().optional(),
    level_db: z.number().optional(),
    freq_hz: z.number().optional(),
    listening: z.boolean().optional(),
    squelch_open: z.boolean().optional(),
    message: z.string().optional(),
  }),
  z.object({ type: z.literal('tx_halted') }),
  z.object({ type: z.literal('notice'), id: z.string(), detail: z.string() }),
  // SSTV: righe in arrivo, immagine finita, stato di ricezione e trasmissione (vedi docs/api.md)
  z.object({
    type: z.literal('sstv'),
    kind: z.string(),
    mode: z.string().optional(),
    width: z.number().optional(),
    height: z.number().optional(),
    first_row: z.number().optional(),
    rows: z.number().optional(),
    rgb: z.string().optional(),
    image_id: z.string().optional(),
    state: z.string().optional(),
    progress: z.number().optional(),
    sync: z.number().optional(),
    offset_hz: z.number().optional(),
    level_db: z.number().optional(),
    listening: z.boolean().optional(),
    message: z.string().optional(),
  }),
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
export const WavelogStationsSchema = z.object({ stations: z.array(z.object({ id: z.string(), name: z.string(), call: z.string() })) });
export type WavelogStation = z.infer<typeof WavelogStationsSchema>['stations'][number];

/** Esito dello spot su DXSummit (`POST /api/v1/spot/dxsummit`): chi, su che frequenza (Hz) e con che testo. */
export const SpotResultSchema = z.object({ ok: z.boolean(), dx_call: z.string(), frequency_hz: z.number(), info: z.string() });
export type SpotResult = z.infer<typeof SpotResultSchema>;
