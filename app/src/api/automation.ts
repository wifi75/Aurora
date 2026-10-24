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
  }),
  limits: z.object({
    max_consecutive_cycles: z.number(),
    max_per_day: z.number(),
    dead_man_secs: z.number(),
    run_unattended: z.boolean(),
  }),
  replies_today: z.number(),
  consecutive: z.number(),
  log: z.array(
    z.object({
      at: z.number(),
      kind: z.enum(['reply', 'would_reply', 'skipped', 'blocked', 'halted', 'paused', 'info']),
      call: z.string().nullable(),
      decode_id: z.number().nullable(),
      reason: z.string(),
    }),
  ),
});
export type AutomationState = z.infer<typeof AutomationStateSchema>;
export type AutomationRules = AutomationState['rules'];
