import { AutomationStateSchema, type AutomationRules, type AutomationState } from './automation';
import { HealthSchema, type Health } from './schemas';

const BASE = '/api/v1';

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly retryAfterSecs?: number,
  ) {
    super(`${status} ${code}`);
    this.name = 'ApiError';
  }
}

async function request(path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(BASE + path, { credentials: 'include', ...init });
  if (!response.ok) {
    let code = 'error';
    try {
      const body: unknown = await response.json();
      if (body && typeof body === 'object' && 'error' in body && typeof body.error === 'string') code = body.error;
    } catch {
      // corpo assente o non JSON: resta il codice generico
    }
    const retry = Number(response.headers.get('retry-after'));
    throw new ApiError(response.status, code, Number.isFinite(retry) && retry > 0 ? retry : undefined);
  }
  return response;
}

function jsonInit(body: unknown): RequestInit {
  return { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) };
}

export async function getHealth(): Promise<Health> {
  return HealthSchema.parse(await (await request('/health')).json());
}

/** Il dispositivo e' associato? Solo un 401 vuol dire "no"; ogni altro errore e' un problema di rete. */
export async function checkSession(): Promise<boolean> {
  const response = await fetch(`${BASE}/devices`, { credentials: 'include' });
  if (response.status === 401) return false;
  if (!response.ok) throw new ApiError(response.status, 'error');
  return true;
}

export async function postHalt(): Promise<void> {
  await request('/commands/halt-tx', { method: 'POST' });
}

export async function postReply(decodeId: number): Promise<void> {
  await request('/commands/reply', jsonInit({ decode_id: decodeId }));
}

export async function finishPairing(code: string, deviceName: string): Promise<void> {
  await request('/pair/finish', jsonInit({ code, device_name: deviceName }));
}

export async function getAutomation(): Promise<AutomationState> {
  return AutomationStateSchema.parse(await (await request('/automation')).json());
}

export interface AutomationChange {
  enabled?: boolean;
  mode?: 'dry_run' | 'live';
  rules?: AutomationRules;
  accept_license_notice?: boolean;
}

export async function putAutomation(change: AutomationChange): Promise<AutomationState> {
  const init = { ...jsonInit(change), method: 'PUT' };
  return AutomationStateSchema.parse(await (await request('/automation', init)).json());
}

export interface StationInfo {
  engine?: string;
  tx_allowed: boolean;
  rig_control?: boolean;
  udp_listen: string;
  udp_forward: string[];
  export_udp: string[];
  export_file: string | null;
  audio_device: string | null;
  cty_loaded: number;
  worked_qsos: number;
  limits: { max_consecutive_cycles: number; max_per_day: number; dead_man_secs: number; run_unattended: boolean };
}

export async function getStationInfo(): Promise<StationInfo> {
  return (await (await request('/station-info')).json()) as StationInfo;
}

export async function postFreeText(text: string, send: boolean): Promise<void> {
  await request('/commands/free-text', jsonInit({ text, send }));
}

export async function postCq(modifier: string): Promise<void> {
  await request('/commands/cq', jsonInit({ modifier }));
}

export async function postTxFrequency(hz: number): Promise<void> {
  await request('/commands/tx-frequency', jsonInit({ hz }));
}

export async function postBand(band: string): Promise<void> {
  await request('/commands/band', jsonInit({ band }));
}

export async function postMode(mode: string): Promise<void> {
  await request('/commands/mode', jsonInit({ mode }));
}

export async function postRxFrequency(hz: number): Promise<void> {
  await request('/commands/rx-frequency', jsonInit({ hz }));
}
