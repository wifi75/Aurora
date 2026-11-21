import { z } from 'zod';

/** Lo stato del ricevitore dei QSO dagli altri log (`qso_export::inbound::InboundStats`). */
export const LogInStatusSchema = z.object({
  available: z.boolean().default(false),
  enabled: z.boolean().default(false),
  listening: z.string().nullable().default(null),
  port: z.number().default(0),
  /** `busy` (porta occupata), `own` (porta usata da Aurora per l'invio) o il testo dell'errore del sistema. */
  error: z.string().nullable().default(null),
  received: z.number().default(0),
  duplicates: z.number().default(0),
  discarded: z.number().default(0),
  last: z.string().nullable().default(null),
});
export type LogInStatus = z.infer<typeof LogInStatusSchema>;

export async function getLogInStatus(): Promise<LogInStatus> {
  const response = await fetch('/api/v1/log-in/status', { credentials: 'include' });
  if (!response.ok) throw new Error(`${response.status}`);
  return LogInStatusSchema.parse(await response.json());
}
