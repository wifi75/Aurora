import { z } from 'zod';
import { DecodeTagSchema } from './schemas';

export const AutomationStateSchema = z.object({
  enabled: z.boolean(),
  mode: z.enum(['dry_run', 'live']),
  live_allowed: z.boolean(),
  rules: z.object({
    reply_to: z.array(DecodeTagSchema),
    min_snr: z.number(),
    excluded_calls: z.array(z.string()),
    max_attempts_per_call: z.number(),
    /** Risponde anche ai CQ della lista dei desiderati (un core piu' vecchio non lo manda: spento). */
    reply_to_wanted: z.boolean().default(false),
    /** Con piu' CQ interessanti nello stesso ciclo: la novita' piu' rara o il segnale piu' forte (un core piu' vecchio non lo manda: rarita'). */
    prefer: z.enum(['rarity', 'snr', 'score']).default('rarity'),
    cooldown_secs: z.number().default(900),
    respect_cq_direction: z.boolean().default(true),
    /** Le regole nuove: un core piu' vecchio non le manda e valgono i valori prudenti (tutto spento, nessun limite orario). */
    reply_to_new_mode: z.boolean().default(false),
    max_qso_per_hour: z.number().default(0),
    window_enabled: z.boolean().default(false),
    window_from_min: z.number().default(480),
    window_to_min: z.number().default(1320),
    excluded_countries: z.array(z.string()).default([]),
    continents: z.array(z.string()).default([]),
    auto_cq_after_min: z.number().default(0),
  }),
  limits: z.object({
    max_consecutive_cycles: z.number(),
    max_per_day: z.number(),
    dead_man_secs: z.number(),
    run_unattended: z.boolean(),
  }),
  replies_today: z.number(),
  consecutive: z.number(),
  would_replies_today: z.number().default(0),
  qsos_today: z.number().default(0),
  qsos_last_hour: z.number().default(0),
  in_window: z.boolean().default(true),
  log: z.array(
    z.object({
      at: z.number(),
      kind: z.enum(['reply', 'would_reply', 'skipped', 'blocked', 'halted', 'paused', 'info', 'call_cq', 'would_call_cq']),
      call: z.string().nullable(),
      decode_id: z.number().nullable(),
      reason: z.string(),
    }),
  ),
});
export type AutomationState = z.infer<typeof AutomationStateSchema>;
export type AutomationRules = AutomationState['rules'];
export type DecisionKind = AutomationState['log'][number]['kind'];

/** I paesi di cty.dat, per scegliere quelli esclusi. */
export const AutomationCountriesSchema = z.object({ countries: z.array(z.string()).default([]) });
