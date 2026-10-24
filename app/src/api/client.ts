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

export async function postHalt(): Promise<void> {
  await request('/commands/halt-tx', { method: 'POST' });
}

export async function postReply(decodeId: number): Promise<void> {
  await request('/commands/reply', jsonInit({ decode_id: decodeId }));
}

export async function finishPairing(code: string, deviceName: string): Promise<void> {
  await request('/pair/finish', jsonInit({ code, device_name: deviceName }));
}
