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

/** Risponde con la frequenza impostata (`dial_hz`), cosi' la pagina la mostra subito. */
export async function postBand(band: string): Promise<{ dial_hz?: number }> {
  return (await (await request('/commands/band', jsonInit({ band }))).json()) as { dial_hz?: number };
}

export async function postMode(mode: string): Promise<void> {
  await request('/commands/mode', jsonInit({ mode }));
}

export async function postRxFrequency(hz: number): Promise<void> {
  await request('/commands/rx-frequency', jsonInit({ hz }));
}

export interface FrequencyEntry {
  band: string;
  mode: string;
  hz: number;
}

export async function getFrequencies(): Promise<FrequencyEntry[]> {
  return (await (await request('/frequencies')).json()) as FrequencyEntry[];
}

export async function putFrequencies(entries: FrequencyEntry[]): Promise<void> {
  await request('/frequencies', { ...jsonInit(entries), method: 'PUT' });
}

export interface ScheduleEntry {
  hour: number;
  minute: number;
  band: string;
}

export interface ScheduleState {
  enabled: boolean;
  entries: ScheduleEntry[];
}

export async function getSchedule(): Promise<ScheduleState> {
  return (await (await request('/schedule')).json()) as ScheduleState;
}

export async function putSchedule(state: ScheduleState): Promise<void> {
  await request('/schedule', { ...jsonInit(state), method: 'PUT' });
}

export async function getAccess(): Promise<{ open: boolean }> {
  return (await (await request('/access')).json()) as { open: boolean };
}

export async function putAccess(open: boolean): Promise<void> {
  await request('/access', { ...jsonInit({ open }), method: 'PUT' });
}

export interface PairingSettings {
  code_ttl_secs: number;
  min_secs: number;
  max_secs: number;
}

export async function getPairingSettings(): Promise<PairingSettings> {
  return (await (await request('/pairing-settings')).json()) as PairingSettings;
}

export async function putPairingSettings(codeTtlSecs: number): Promise<void> {
  await request('/pairing-settings', { ...jsonInit({ code_ttl_secs: codeTtlSecs }), method: 'PUT' });
}

export interface PairedDevice {
  id: number;
  name: string;
  created_at: number;
}

export async function getDevices(): Promise<PairedDevice[]> {
  return (await (await request('/devices')).json()) as PairedDevice[];
}

export async function revokeDevice(id: number): Promise<void> {
  await request(`/devices/${id}`, { method: 'DELETE' });
}

export interface PairingOffer {
  code: string;
  expires_in: number;
  qr_svg: string;
}

/** Un codice nuovo con il QR; il link punta all'indirizzo da cui si sta guardando la pagina. */
export async function startPairing(baseUrl: string): Promise<PairingOffer> {
  return (await (await request('/pair/start', jsonInit({ base_url: baseUrl }))).json()) as PairingOffer;
}

export interface OutputLevelState {
  attenuation_db: number;
  min_db: number;
  max_db: number;
}

export async function getOutputLevel(): Promise<OutputLevelState> {
  return (await (await request('/output-level')).json()) as OutputLevelState;
}

export async function putOutputLevel(attenuationDb: number): Promise<void> {
  await request('/output-level', { ...jsonInit({ attenuation_db: attenuationDb }), method: 'PUT' });
}

export interface AwardStats {
  qsos: number;
  dxcc_worked: number;
  dxcc_total: number;
  cq_zones_worked: number;
  cq_zones_total: number;
  itu_zones_worked: number;
  itu_zones_total: number;
  grids: number;
  prefixes: number;
  calls: number;
  bands: Array<{ band: string; dxcc: number; qsos: number }>;
}

export interface StatsResponse {
  stats: AwardStats;
  missing_dxcc: string[];
}

export interface ClusterServer {
  name: string;
  host: string;
  port: number;
  enabled: boolean;
}

export interface ClusterConfig {
  enabled: boolean;
  callsign: string;
  servers: ClusterServer[];
  login_commands: string[];
  max_spots: number;
  max_age_min: number;
}

export interface ClusterSpot {
  id: number;
  received_ms: number;
  server: string;
  spotter: string;
  freq_khz: number;
  dx_call: string;
  comment: string;
  time_utc: string;
  grid: string | null;
  band: string | null;
  mode: string | null;
  country: string | null;
  continent: string | null;
}

export interface ClusterServerStatus {
  key: string;
  name: string;
  state: 'connecting' | 'connected' | 'error';
  detail: string | null;
  spots: number;
  since_ms: number;
}

export interface ClusterCatalogEntry {
  name: string;
  host: string;
  port: number;
  software: string;
  place: string;
  note: string;
}

/** Impostazioni, stato dei collegamenti e spot recenti del DX Cluster (`limit` spot al massimo). */
export async function getClusterSpots(limit = 200): Promise<{ config: ClusterConfig; servers: ClusterServerStatus[]; spots: ClusterSpot[] }> {
  return (await (await request(`/dx-cluster?limit=${limit}`)).json()) as { config: ClusterConfig; servers: ClusterServerStatus[]; spots: ClusterSpot[] };
}

export async function putCluster(config: ClusterConfig): Promise<void> {
  await request('/dx-cluster', { ...jsonInit(config), method: 'PUT' });
}

/** Paese e zone di un nominativo, dal file dei prefissi (cty.dat); 404 se non lo conosce. */
export async function getDxcc(call: string): Promise<{ country: string; continent: string; prefix: string; cq_zone: number; itu_zone: number }> {
  return (await (await request(`/dxcc/${encodeURIComponent(call)}`)).json()) as { country: string; continent: string; prefix: string; cq_zone: number; itu_zone: number };
}

/** Prova un server: si collega e legge il benvenuto, senza dire chi siamo. */
export async function testClusterServer(host: string, port: number): Promise<{ ok: boolean; ms: number | null; banner: string | null; error: string | null }> {
  return (await (await request('/dx-cluster/test', jsonInit({ host, port }))).json()) as { ok: boolean; ms: number | null; banner: string | null; error: string | null };
}

/** I server DX Cluster pubblici noti. */
export async function getClusterCatalog(): Promise<ClusterCatalogEntry[]> {
  return (await (await request('/dx-cluster/catalog')).json()) as ClusterCatalogEntry[];
}

/** Manda un comando di consultazione a un cluster collegato (mai spot o messaggi). */
export async function sendClusterCommand(server: string, text: string): Promise<void> {
  await request('/dx-cluster/command', jsonInit({ server, text }));
}

export interface ClockConfig {
  enabled: boolean;
  servers: string[];
  interval_secs: number;
  warn_ms: number;
  auto_correct: boolean;
}

export interface ClockMeasurement {
  at_ms: number;
  server: string | null;
  offset_ms: number | null;
  delay_ms: number | null;
  servers_ok: number;
  servers_tried: number;
  error: string | null;
  corrected: boolean;
}

/** Client NTP: impostazioni e ultima misura dell'orologio del PC. */
export async function getClock(): Promise<{ config: ClockConfig; last: ClockMeasurement | null }> {
  return (await (await request('/clock')).json()) as { config: ClockConfig; last: ClockMeasurement | null };
}

export async function putClock(config: ClockConfig): Promise<void> {
  await request('/clock', { ...jsonInit(config), method: 'PUT' });
}

/** Misura subito lo scarto dell'orologio. */
export async function checkClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/check', { method: 'POST' })).json()) as ClockMeasurement;
}

/** Sposta l'ora di Windows dello scarto misurato (serve l'amministratore, solo dal PC di casa). */
export async function correctClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/correct', { method: 'POST' })).json()) as ClockMeasurement;
}

/** Le impostazioni di JTDX, per nome, come testo (dal suo file di configurazione). */
export async function getJtdxConfig(): Promise<Record<string, string>> {
  return (await (await request('/jtdx-config')).json()) as Record<string, string>;
}

export interface Log4omQso {
  date: string;
  call: string;
  band: string;
  mode: string;
  rst_sent: string;
  rst_rcvd: string;
  country: string;
  grid: string;
}

export interface Log4omView {
  total: number;
  qsos: Log4omQso[];
}

export async function getLog4om(limit: number): Promise<Log4omView> {
  return (await (await request(`/log4om?limit=${limit}`)).json()) as Log4omView;
}

export async function getStats(): Promise<StatsResponse> {
  return (await (await request('/stats')).json()) as StatsResponse;
}

export type ControlId =
  | 'autoseq' | 'tx_period' | 'single_qso' | 'answer_b4' | 'bypass' | 'skip_tx1' | 'split' | 'show_wanted'
  | 'auto_tx' | 'enable_tx' | 'hound' | 'jt' | 'monitor' | 'swl' | 'sync' | 'agc' | 'filter' | 'decode' | 'erase' | 'stop_rx' | 'clear_dx' | 'gen_msgs' | 'rx_to_tx' | 'tx_to_rx';

/** Stato degli interruttori di JTDX (AutoSeq, 1 QSO, Abilita Tx...). */
export async function getControls(): Promise<Partial<Record<ControlId, boolean>>> {
  const body = (await (await request('/controls')).json()) as { controls?: Partial<Record<ControlId, boolean>> };
  return body.controls ?? {};
}

/** Porta un interruttore allo stato voluto, o preme un pulsante semplice (senza `on`). */
export async function postControl(id: ControlId, on?: boolean): Promise<void> {
  await request('/commands/control', jsonInit(on === undefined ? { id } : { id, on }));
}
