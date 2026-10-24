import { z } from 'zod';

export const HealthSchema = z.object({
  status: z.literal('ok'),
  name: z.string(),
  version: z.string(),
  author: z.string(),
});
export type Health = z.infer<typeof HealthSchema>;

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

export const EventSchema = z.discriminatedUnion('type', [
  StationStatusSchema.extend({ type: z.literal('status') }),
  DecodedMessageSchema.extend({ type: z.literal('decode') }),
  QsoRecordSchema.extend({ type: z.literal('qso_logged') }),
  z.object({ type: z.literal('tx_halted') }),
]);
export type AuroraEvent = z.infer<typeof EventSchema>;
